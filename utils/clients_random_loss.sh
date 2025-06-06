#!/bin/bash

PREFIX="iris-client"
TCGUI_URL="http://localhost:5000"

echo "Scanning for containers named '${PREFIX}-*'..."

# # Get all matching running container names
# CONTAINERS=$(docker ps --format '{{.Names}}' | grep "^${PREFIX}-")

# if [[ -z "$CONTAINERS" ]]; then
#     echo "No matching containers found."
#     exit 1
# fi

# for CONTAINER in $CONTAINERS; do
#     PID=$(docker inspect -f '{{.State.Pid}}' "$CONTAINER" 2>/dev/null)
#     if [[ -z "$PID" || "$PID" == "0" ]]; then
#         echo "$CONTAINER: not running or doesn't exist"
#         continue
#     fi

#     CONTAINER_IFINDEX=$(nsenter -t "$PID" -n ip link show eth0 | grep -oP '(?<=if)[0-9]+')
#     HOST_VETH=$(ip -o link | grep "if${CONTAINER_IFINDEX}@" | awk -F: '{print $2}' | tr -d ' ')

#     if [[ -z "$HOST_VETH" ]]; then
#         echo "$CONTAINER: could not determine host veth"
#         continue
#     fi

#     # Generate random loss between 0 and 40
#     LOSS=$((RANDOM % 41))

#     echo "Applying $LOSS% loss to $CONTAINER (interface: $HOST_VETH)"

#     curl -s -X POST "$TCGUI_URL/new_rule/$HOST_VETH" \
#         -H 'Content-Type: application/x-www-form-urlencoded' \
#         --data-raw "Rate=&rate_unit=mbit&Delay=&DelayVariance=&Loss=${LOSS}&LossCorrelation=&Duplicate=&Reorder=&ReorderCorrelation=&Corrupt=&Limit=" > /dev/null

#     echo "$CONTAINER: applied ${LOSS}% loss to $HOST_VETH"
# done

get_container_names() {
  docker container ls --format '{{ .Names }}'
}

get_container_iflink() {
  docker exec "$1" cat /sys/class/net/eth0/iflink
}

get_veth_interfaces() {
  find /sys/class/net -type l -name 'veth*'
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

for container in $(get_container_names); do
  if [[ ! "$container" =~ ^iris-client-[0-9]+$ ]]; then
    continue
  fi

  iflink=$(get_container_iflink "$container")
  network=$(find_veth_interface_with_matching_index "$iflink")

  if [ "$network" ]; then
    echo "$container - $network"

    # Generate random loss between 0 and 40
    LOSS=$((RANDOM % 41))

    echo "Applying $LOSS% loss to $container (interface: $network)"

    curl -s -X POST "$TCGUI_URL/new_rule/$network" \
        -H 'Content-Type: application/x-www-form-urlencoded' \
        --data-raw "Rate=&rate_unit=mbit&Delay=&DelayVariance=&Loss=${LOSS}&LossCorrelation=&Duplicate=&Reorder=&ReorderCorrelation=&Corrupt=&Limit=" > /dev/null

    echo "$container: applied ${LOSS}% loss to $network"
  fi
done
