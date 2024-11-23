#!/bin/bash

# Name of the veth interfaces, namespace, and bridge
VETH_HOST="veth0"
VETH_NS="veth1"
NS="myns"
IP_HOST="192.168.15.1/24"
IP_NS="192.168.15.2/24"
BRIDGE="br0"

function start_veth_namespace() {
    echo "Setting up veth pair, namespace, and bridge..."

    # Create veth pair
    sudo ip link add ${VETH_HOST} type veth peer name ${VETH_NS}

    # Create the namespace
    sudo ip netns add ${NS}

    # Move one side of veth to the namespace
    sudo ip link set ${VETH_NS} netns ${NS}

    # Bring up the bridge and the interfaces
    sudo ip addr add ${IP_HOST} dev ${VETH_HOST}  # Assign IP to bridge
    sudo ip link set ${VETH_HOST} up

    # Configure the namespace veth interface and loopback
    sudo ip netns exec ${NS} ip addr add ${IP_NS} dev ${VETH_NS}
    sudo ip netns exec ${NS} ip link set ${VETH_NS} up
    sudo ip netns exec ${NS} ip link set lo up
    sudo ip netns exec ${NS} route add -net 224.0.0.0 netmask 240.0.0.0 dev ${VETH_NS}
    # route add -net 224.0.0.0 netmask 240.0.0.0 dev lo

    echo "veth pair, namespace, setup complete."
    echo "Host IP: ${IP_HOST}, Namespace IP: ${IP_NS}"
}

function stop_veth_namespace() {
    echo "Tearing down veth pair, namespace, and bridge..."

    # Bring down and delete the veth interface on the host
    sudo ip link delete ${VETH_HOST}

    # Delete the namespace
    sudo ip netns delete ${NS}

    echo "veth pair, namespace, teardown complete."
}

function open_namespace_shell() {
    echo "Opening shell in namespace ${NS}..."

    # Run a shell inside the network namespace
    sudo ip netns exec ${NS} bash
}

# Check input argument for start/stop/open
case "$1" in
    start)
        start_veth_namespace
        ;;
    stop)
        stop_veth_namespace
        ;;
    open)
        open_namespace_shell
        ;;
    *)
        echo "Usage: $0 {start|stop|open}"
        exit 1
        ;;
esac
