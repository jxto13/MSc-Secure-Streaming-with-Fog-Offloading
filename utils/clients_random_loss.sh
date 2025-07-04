#!/bin/bash

# Configuration
PREFIX="iris-client"
TCGUI_URL="http://localhost:5000"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SETTINGS_FILE="$SCRIPT_DIR/clients_random_loss_settings.json"

# Handle stop command early
if [[ "$1" == "stop" ]]; then
  echo "Stopping and removing settings file..."
  rm -f "$SETTINGS_FILE"
  exit 0
fi

# Parse arguments: MIN_LOSS and MAX_LOSS
MIN_LOSS=${1:-0}
MAX_LOSS=${2:-40}

echo "Scanning for containers named '${PREFIX}-*'..."

# Function to list all running container names
get_container_names() {
  docker container ls --format '{{ .Names }}'
}

# Get the interface index inside the container (eth0)
get_container_iflink() {
  docker exec "$1" cat /sys/class/net/eth0/iflink 2>/dev/null
}

# Find all veth interfaces (excluding veth0)
get_veth_interfaces() {
  find /sys/class/net -type l -name 'veth*' 2>/dev/null | grep -v '/veth0'
}

# Get interface index for a host veth interface
get_interface_index() {
  cat "$1/ifindex"
}

# Match container eth0 iflink to host veth interface index
find_veth_interface_with_matching_index() {
  for interface in $(get_veth_interfaces); do
    if [ "$(get_interface_index "$interface")" = "$1" ]; then
      echo "${interface##*/}"
      break
    fi
  done
}

# Prepare the JSON settings file
> "$SETTINGS_FILE"
echo '[' > "$SETTINGS_FILE"
FIRST=1

# Process containers
for container in $(get_container_names); do
  if [[ ! "$container" =~ ^${PREFIX}-[0-9]+$ ]]; then
    continue
  fi

  echo "Processing container: $container"

  iflink=$(get_container_iflink "$container")
  if [ -z "$iflink" ]; then
    echo "  Failed to get iflink for $container, skipping..."
    continue
  fi

  network=$(find_veth_interface_with_matching_index "$iflink")
  if [ -z "$network" ]; then
    echo "  No matching veth interface found for $container (iflink=$iflink)"
    continue
  fi

  echo "  Found host interface: $network"

  # Generate a random loss value
  LOSS=$(( RANDOM % (MAX_LOSS - MIN_LOSS + 1) + MIN_LOSS ))
  echo "  Applying $LOSS% loss"

  # Apply the loss using tcgui
  RESPONSE_CODE=$(curl -s -L -o /dev/null -w "%{http_code}" -X POST "$TCGUI_URL/new_rule/$network" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    --data-raw "Rate=&rate_unit=mbit&Delay=&DelayVariance=&Loss=${LOSS}&LossCorrelation=&Duplicate=&Reorder=&ReorderCorrelation=&Corrupt=&Limit=")

  if [ "$RESPONSE_CODE" != "200" ]; then
    echo "  Failed to apply loss to $network (HTTP $RESPONSE_CODE)"
    continue
  fi

  echo "  Successfully applied ${LOSS}% loss to $network"

  # Write entry to JSON file
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
done

echo ']' >> "$SETTINGS_FILE"

echo "Done. Settings saved to $SETTINGS_FILE"
