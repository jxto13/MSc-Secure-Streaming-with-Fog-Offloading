#!/bin/bash

PREFIX="iris-client"
TCGUI_URL="http://localhost:5000"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SETTINGS_FILE="$SCRIPT_DIR/clients_random_loss_settings.json"

# Usage: ./clients_random_loss.sh [MIN_LOSS] [MAX_LOSS]
MIN_LOSS=${1:-0}
MAX_LOSS=${2:-40}

echo "Scanning for containers named '${PREFIX}-*'..."

get_container_names() {
  docker container ls --format '{{ .Names }}'
}

get_container_iflink() {
  docker exec "$1" cat /sys/class/net/eth0/iflink
}

get_veth_interfaces() {
  # Only include veth interfaces that match the new pattern and exclude veth0
  find /sys/class/net -type l -name 'veth*-br' -o -name 'veth[1-9]*-br' 2>/dev/null | grep -v '/veth0'
}

get_interface_index() {
  cat "$1/ifindex"
}

find_veth_interface_with_matching_index() {
  for interface in $(get_veth_interfaces); do
    if [ "$(get_interface_index "$interface")" = "$1" ]; then
      echo "${interface##*/}"
      break
    fi
  done
}

# Clear or create the JSON file
> "$SETTINGS_FILE"
echo '[' > "$SETTINGS_FILE"
FIRST=1

for container in $(get_container_names); do
  if [[ ! "$container" =~ ^iris-client-[0-9]+$ ]]; then
    continue
  fi

  iflink=$(get_container_iflink "$container")
  network=$(find_veth_interface_with_matching_index "$iflink")

  # Exclude veth0 explicitly
  if [ "$network" ] && [ "$network" != "veth0" ]; then
    echo "$container - $network"

    # Generate random loss between MIN_LOSS and MAX_LOSS
    LOSS=$(( RANDOM % (MAX_LOSS - MIN_LOSS + 1) + MIN_LOSS ))

    echo "Applying $LOSS% loss to $container (interface: $network)"

    curl -s -X POST "$TCGUI_URL/new_rule/$network" \
        -H 'Content-Type: application/x-www-form-urlencoded' \
        --data-raw "Rate=&rate_unit=mbit&Delay=&DelayVariance=&Loss=${LOSS}&LossCorrelation=&Duplicate=&Reorder=&ReorderCorrelation=&Corrupt=&Limit=" > /dev/null

    echo "$container: applied ${LOSS}% loss to $network"

    # Write to JSON file
    if [ $FIRST -eq 1 ]; then
      FIRST=0
    else
      echo ',' >> "$SETTINGS_FILE"
    fi
    echo -n '{' >> "$SETTINGS_FILE"
    echo -n "\"container\": \"$container\", " >> "$SETTINGS_FILE"
    echo -n "\"interface\": \"$network\", " >> "$SETTINGS_FILE"
    echo -n "\"loss\": $LOSS" >> "$SETTINGS_FILE"
    echo -n '}' >> "$SETTINGS_FILE"
  fi
done
echo ']' >> "$SETTINGS_FILE"

# Add stop logic to delete the JSON file
if [[ "$1" == "stop" ]]; then
  rm -f "$SETTINGS_FILE"
fi
