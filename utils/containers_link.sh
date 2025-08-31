#!/usr/bin/env bash
set -euo pipefail

# iris-linker.sh
# Create or tear down veth links between Docker containers with name prefix iris-client-
# Topology JSON schema:
# {
#   "links": [
#     {
#       "a": "iris-client-1",
#       "b": "iris-client-2",
#       "a_ip": "10.200.1.1/30",
#       "b_ip": "10.200.1.2/30",
#       "mtu": 1500,             # optional
#       "a_ifname": "vab1",      # optional, <=15 chars
#       "b_ifname": "vab2"       # optional, <=15 chars
#     },
#     ...
#   ]
# }

PREFIX="iris-client-"

need() { command -v "$1" >/dev/null 2>&1 || { echo "missing dependency: $1" >&2; exit 1; }; }
need docker
need jq
need ip
need nsenter
need md5sum || need md5

usage() {
  cat <<EOF
Usage:
  $0 apply   <topology.json>   # create/ensure links
  $0 delete  <topology.json>   # delete links
  $0 verify  <topology.json>   # print planned operations
EOF
}

[[ $# -eq 2 ]] || { usage; exit 1; }
ACTION="$1"
JSON="$2"
[[ -f "$JSON" ]] || { echo "no such file: $JSON" >&2; exit 1; }

# Helpers
docker_pid() {
  local name="$1"
  docker inspect -f '{{.State.Pid}}' "$name" 2>/dev/null || echo 0
}

short_hash() {
  # stable 8 chars for pair directionless
  local a="$1" b="$2"
  local key
  if [[ "$a" < "$b" ]]; then key="${a}|${b}"; else key="${b}|${a}"; fi
  printf "%s" "$key" | md5sum | awk '{print $1}' | cut -c1-8
}

inside() {
  local pid="$1"; shift
  nsenter --target "$pid" --net -- "$@"
}

ensure_prefix() {
  local name="$1"
  [[ "$name" == $PREFIX* ]] || { echo "container '$name' does not start with prefix '$PREFIX'"; return 1; }
}

link_names() {
  # Compute safe host veth names and in-namespace names (<=15 chars)
  local a="$1" b="$2" h
  h=$(short_hash "$a" "$b")
  # host names (exist only briefly)
  echo "v${h}a v${h}b l${h}a l${h}b"
}

link_exists_in_ns() {
  local pid="$1" ifname="$2"
  inside "$pid" ip link show "$ifname" >/dev/null 2>&1
}

delete_link_pair() {
  local a="$1" b="$2"
  read -r vA vB lA lB < <(link_names "$a" "$b")
  # Try to delete by host names if present, else by inside names
  if ip link show "$vA" >/dev/null 2>&1; then ip link del "$vA"; return; fi
  if ip link show "$vB" >/dev/null 2>&1; then ip link del "$vB"; return; fi
  # Attempt delete from either namespace if one end is visible
  local pidA pidB
  pidA=$(docker_pid "$a"); pidB=$(docker_pid "$b")
  if [[ "$pidA" -ne 0 ]] && link_exists_in_ns "$pidA" "$lA"; then inside "$pidA" ip link del "$lA" || true; fi
  if [[ "$pidB" -ne 0 ]] && link_exists_in_ns "$pidB" "$lB"; then inside "$pidB" ip link del "$lB" || true; fi
}

apply_link() {
  local a="$1" b="$2" a_ip="$3" b_ip="$4" mtu="$5" a_if="$6" b_if="$7"

  ensure_prefix "$a"; ensure_prefix "$b"

  local pidA pidB
  pidA=$(docker_pid "$a"); pidB=$(docker_pid "$b")
  [[ "$pidA" -gt 0 && "$pidB" -gt 0 ]] || { echo "container not running: $a or $b" >&2; return 1; }

  read -r vA vB lA lB < <(link_names "$a" "$b")
  [[ -n "$a_if" ]] && lA="$a_if"
  [[ -n "$b_if" ]] && lB="$b_if"

  # Idempotency: if both ends exist, set IPs and return
  if link_exists_in_ns "$pidA" "$lA" && link_exists_in_ns "$pidB" "$lB"; then
    [[ -n "$a_ip" ]] && inside "$pidA" sh -c "ip addr show dev $lA | grep -q \"$a_ip\" || ip addr add $a_ip dev $lA"
    [[ -n "$b_ip" ]] && inside "$pidB" sh -c "ip addr show dev $lB | grep -q \"$b_ip\" || ip addr add $b_ip dev $lB"
    inside "$pidA" ip link set "$lA" up
    inside "$pidB" ip link set "$lB" up
    return 0
  fi

  # Clean any leftover host ends
  ip link show "$vA" >/dev/null 2>&1 && ip link del "$vA" || true
  ip link show "$vB" >/dev/null 2>&1 && ip link del "$vB" || true

  # Create veth pair
  ip link add "$vA" type veth peer name "$vB"
  [[ -n "$mtu" ]] && ip link set mtu "$mtu" dev "$vA" && ip link set mtu "$mtu" dev "$vB"

  # Move into namespaces
  ip link set "$vA" netns "$pidA"
  ip link set "$vB" netns "$pidB"

  # Rename inside, address, up
  inside "$pidA" ip link set "$vA" name "$lA"
  inside "$pidB" ip link set "$vB" name "$lB"

  [[ -n "$a_ip" ]] && inside "$pidA" ip addr add "$a_ip" dev "$lA"
  [[ -n "$b_ip" ]] && inside "$pidB" ip addr add "$b_ip" dev "$lB"

  inside "$pidA" ip link set "$lA" up
  inside "$pidB" ip link set "$lB" up
}

verify_link() {
  local a="$1" b="$2" a_ip="$3" b_ip="$4" mtu="$5" a_if="$6" b_if="$7"
  read -r vA vB lA lB < <(link_names "$a" "$b")
  [[ -n "$a_if" ]] && lA="$a_if"
  [[ -n "$b_if" ]] && lB="$b_if"
  echo "link $a($lA,$a_ip) <-> $b($lB,$b_ip) mtu=${mtu:-default}"
}

# Iterate JSON
mapfile -t indices < <(jq -r '.links | to_entries | .[].key' "$JSON")
[[ ${#indices[@]} -gt 0 ]] || { echo "no links found in JSON" >&2; exit 1; }

for i in "${indices[@]}"; do
  a=$(jq -r ".links[$i].a" "$JSON")
  b=$(jq -r ".links[$i].b" "$JSON")
  a_ip=$(jq -r ".links[$i].a_ip // empty" "$JSON")
  b_ip=$(jq -r ".links[$i].b_ip // empty" "$JSON")
  mtu=$(jq -r ".links[$i].mtu // empty" "$JSON")
  a_if=$(jq -r ".links[$i].a_ifname // empty" "$JSON")
  b_if=$(jq -r ".links[$i].b_ifname // empty" "$JSON")

  case "$ACTION" in
    verify) verify_link "$a" "$b" "$a_ip" "$b_ip" "$mtu" "$a_if" "$b_if" ;;
    delete) delete_link_pair "$a" "$b" ;;
    apply)  apply_link  "$a" "$b" "$a_ip" "$b_ip" "$mtu" "$a_if" "$b_if" ;;
    *) usage; exit 1 ;;
  esac
done

echo "done: $ACTION"
