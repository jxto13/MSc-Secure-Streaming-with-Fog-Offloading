# ┌─────────────────────────────┐
# │          HOST               │
# │                             │
# │         br-host             │
# │           │                 │
# │           │                 │
# │           │                 │
# └───────────┴─────────────────┘
#             |
#         [brns namespace]
#             |
#             |
#         [br0 bridge]
#         /      \
#   veth1-br   veth2-br
#     |           |
#  [myns1]     [myns2]
#     |           |
#   veth1       veth2
#
#   veth-link1  <───── direct link ─────>  veth-link2
#
# Legend:
# - brns: bridge namespace (contains br0, br-host, veth1-br, veth2-br, ...)
# - host-br: veth in host, br-host: peer in brns namespace
# - veth1-br, veth2-br: veths in brns, veth1, veth2: peers in myns1, myns2
# - myns1, myns2: client namespaces
# - veth-link1/veth-link2: direct veth pair between myns1 and myns2
#
# All IPs and links are configured in the script below.


#!/bin/bash

# Interface and namespace names
VETH1="veth1"
VETH1_BR="veth1-br"
VETH2="veth2"
VETH2_BR="veth2-br"
LINK1="aware1"
LINK2="aware2"
NS1="myns1"
NS2="myns2"
BRIDGE="br0"
BRNS="brns"

# IP configuration
IP_HOST="192.168.15.1/24"
IP_NS1="192.168.15.2/24"
IP_NS2="192.168.15.3/24"


function start_network() {
    echo "[+] Creating namespaces, veth pairs, and bridge namespace..."

    # Create namespaces
    sudo ip netns add ${NS1}
    sudo ip netns add ${NS2}
    sudo ip netns add ${BRNS}

    # Create veth pair for host <-> brns
    sudo ip link add host-br type veth peer name br-host
    sudo ip addr add 192.168.15.100/24 dev host-br
    sudo ip link set host-br up
    sudo ip link set br-host netns ${BRNS}

    # Create veth pair for ns1 <-> brns
    sudo ip link add ${VETH1_BR} type veth peer name ${VETH1}
    sudo ip link set ${VETH1} netns ${NS1}
    sudo ip link set ${VETH1_BR} netns ${BRNS}

    # Create veth pair for ns2 <-> brns
    sudo ip link add ${VETH2_BR} type veth peer name ${VETH2}
    sudo ip link set ${VETH2} netns ${NS2}
    sudo ip link set ${VETH2_BR} netns ${BRNS}

    # Setup bridge in brns namespace
    sudo ip netns exec ${BRNS} ip link add ${BRIDGE} type bridge
    sudo ip netns exec ${BRNS} ip addr add ${IP_HOST} dev ${BRIDGE}
    sudo ip netns exec ${BRNS} ip link set ${BRIDGE} up

    # Attach br-host and vethX-br to bridge in brns
    sudo ip netns exec ${BRNS} ip link set br-host up
    sudo ip netns exec ${BRNS} ip link set br-host master ${BRIDGE}
    sudo ip netns exec ${BRNS} ip link set ${VETH1_BR} up
    sudo ip netns exec ${BRNS} ip link set ${VETH1_BR} master ${BRIDGE}
    sudo ip netns exec ${BRNS} ip link set ${VETH2_BR} up
    sudo ip netns exec ${BRNS} ip link set ${VETH2_BR} master ${BRIDGE}

    # Setup ns1
    sudo ip netns exec ${NS1} ip link set lo up
    sudo ip netns exec ${NS1} ip addr add ${IP_NS1} dev ${VETH1}
    sudo ip netns exec ${NS1} ip link set ${VETH1} up

    # Setup ns2
    sudo ip netns exec ${NS2} ip link set lo up
    sudo ip netns exec ${NS2} ip addr add ${IP_NS2} dev ${VETH2}
    sudo ip netns exec ${NS2} ip link set ${VETH2} up

    # Direct aware link is now created by a separate option.

    echo "[+] Setup complete."
    echo "  - Bridge namespace: ${BRNS}"
    echo "  - Host bridge IP (in brns): ${IP_HOST}"
    echo "  - Host veth IP: 192.168.15.100/24 (host-br)"

    # --- MULTICAST SETUP ---

    # Add multicast routes inside client namespaces for 224.0.0.0/4
    sudo ip netns exec ${NS1} ip route add 224.0.0.0/4 dev ${VETH1}
    sudo ip netns exec ${NS2} ip route add 224.0.0.0/4 dev ${VETH2}

    # Add multicast route in bridge namespace for 224.0.0.0/4
    sudo ip netns exec ${BRNS} ip route add 224.0.0.0/4 dev ${BRIDGE}

}

function add_aware_link() {
    echo "[+] Adding aware1 <-> aware2 veth link between namespaces..."
    sudo ip link add ${LINK1} type veth peer name ${LINK2}
    sudo ip link set ${LINK1} netns ${NS1}
    sudo ip link set ${LINK2} netns ${NS2}

    sudo ip netns exec ${NS1} ip link set ${LINK1} up
    sudo ip netns exec ${NS2} ip link set ${LINK2} up

    echo "[+] aware link created."
    echo "  - myns1: ${IP_NS1}"
    echo "  - myns2: ${IP_NS2}"
}


# Function to remove aware1/aware2 veth link between namespaces
function stop_aware_link() {
    echo "[+] Removing aware1 <-> aware2 veth link between namespaces..."
    sudo ip netns exec ${NS1} ip link delete aware1 2>/dev/null || true
    sudo ip netns exec ${NS2} ip link delete aware2 2>/dev/null || true
    echo "[+] aware link removed (if it existed)."
}

function stop_network() {
    echo "[+] Cleaning up..."

    sudo ip netns delete ${NS1} 2>/dev/null || true
    sudo ip netns delete ${NS2} 2>/dev/null || true
    sudo ip netns delete ${BRNS} 2>/dev/null || true
    sudo ip link delete host-br 2>/dev/null || true
    sudo ip link delete ${LINK1} 2>/dev/null || true

    echo "[+] Teardown complete."
}

function open_shell() {
    if [[ "$2" == "1" ]]; then
        sudo ip netns exec ${NS1} bash
    elif [[ "$2" == "2" ]]; then
        sudo ip netns exec ${NS2} bash
    elif [[ "$2" == "br" ]]; then
        sudo ip netns exec ${BRNS} bash
    else
        echo "Usage: $0 open {1|2|br}"
    fi
}

case "$1" in
    start)
        start_network
        ;;
    stop)
        stop_network
        ;;
    open)
        open_shell "$@"
        ;;
    link)
        add_aware_link
        ;;
    unlink)
        stop_aware_link
        ;;
    *)
        echo "Usage: $0 {start|stop|open 1|2|br|link}"
        exit 1
        ;;
esac
