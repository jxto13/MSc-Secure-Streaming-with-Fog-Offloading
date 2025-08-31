#!/usr/bin/env python3
import json
import os
import re
import subprocess
import sys
from pathlib import Path

CLIENT_REGEX = re.compile(r"^iris-client", re.I)

def sh(*args) -> str:
    out = subprocess.check_output(args, text=True)
    return out.strip()

def docker_names_and_ids():
    out = sh("docker", "ps", "--format", "{{.Names}} {{.ID}}")
    lines = [l.strip() for l in out.splitlines() if l.strip()]
    for line in lines:
        name, cid = line.rsplit(" ", 1)
        if CLIENT_REGEX.match(name):
            yield name, cid

def container_pid(container_id: str) -> int:
    pid = sh("docker", "inspect", "-f", "{{.State.Pid}}", container_id)
    return int(pid)

def container_eth0_iflink(pid: int) -> int:
    path = Path(f"/proc/{pid}/root/sys/class/net/eth0/iflink")
    with open(path) as f:
        return int(f.read().strip())

def host_veth_from_ifindex(ifindex: int) -> str | None:
    sys_net = Path("/sys/class/net")
    for iface in sys_net.iterdir():
        try:
            with open(iface / "ifindex") as f:
                idx = int(f.read().strip())
            if idx == ifindex:
                return iface.name
        except Exception:
            continue
    return None

def apply_tc(veth: str, delay_ms: float | int | None, loss_pct: float | int | None):
    # Normalize
    d = float(delay_ms) if delay_ms is not None else 0.0
    l = float(loss_pct) if loss_pct is not None else 0.0

    if d <= 0.0 and l <= 0.0:
        # Clear any existing qdisc
        subprocess.run(["tc", "qdisc", "del", "dev", veth, "root"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        return

    # Build netem args
    args = ["tc", "qdisc", "replace", "dev", veth, "root", "netem"]
    if d > 0.0:
        args += ["delay", f"{d}ms"]
    if l > 0.0:
        args += ["loss", f"{l}%"]
    subprocess.check_call(args)

def load_policy(json_path: str) -> dict:
    with open(json_path) as f:
        data = json.load(f)
    if not isinstance(data, dict):
        raise ValueError("Top-level JSON must be an object.")
    return data

def policy_for(name: str, policy: dict) -> dict:
    # Exact name match first, then regex keys, then optional "default"
    if name in policy:
        return policy[name]
    # Regex keys if provided, marked by leading "~"
    for k, v in policy.items():
        if isinstance(k, str) and k.startswith("~"):
            if re.search(k[1:], name):
                return v
    return policy.get("default", {})

def main():
    if os.geteuid() != 0:
        print("Run as root. tc and /proc access require it.", file=sys.stderr)
        sys.exit(1)
    if len(sys.argv) != 2:
        print(f"Usage: {sys.argv[0]} policy.json", file=sys.stderr)
        sys.exit(1)

    policy = load_policy(sys.argv[1])

    results = []
    for name, cid in docker_names_and_ids():
        try:
            pid = container_pid(cid)
            iflink = container_eth0_iflink(pid)
            veth = host_veth_from_ifindex(iflink)
            if not veth:
                results.append((name, "veth not found"))
                continue
            cfg = policy_for(name, policy)
            delay = cfg.get("delay_ms", 0)
            loss = cfg.get("loss_pct", 0)
            apply_tc(veth, delay, loss)
            results.append((name, f"{veth}: delay={delay}ms loss={loss}%"))
        except subprocess.CalledProcessError as e:
            results.append((name, f"error: {e}"))
        except Exception as e:
            results.append((name, f"error: {e}"))

    for name, msg in results:
        print(f"{name}: {msg}")

if __name__ == "__main__":
    main()
