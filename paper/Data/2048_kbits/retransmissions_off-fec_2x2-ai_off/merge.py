#!/usr/bin/env python3
"""
Merge metric CSVs by 'Time' and harmonize per-client columns.

Features
- Detects per-client columns in two forms:
    1) Embedded labels like {... client_id="HEX64" ...}
    2) Bare 64-hex column headers (e.g., '206e9a...').
- Builds a stable client_id -> index map from the missed-deadlines table if present.
  Falls back to scanning any per-client tables.
- Normalizes single-series files to friendly names.
- Skips non-time tables like Clients.
- Renames before merging to avoid _x/_y suffixes.
- Outputs Excel.

Usage
    python merge.py --out merged_output.xlsx ./file1.csv ./file2.csv ...

"""

import argparse
import os
import re
from functools import reduce
from typing import Dict, List, Tuple

import pandas as pd

# ------------------------- Patterns -----------------------------------------

RE_EMBEDDED = re.compile(r'client_id="([0-9a-f]{64})"', re.IGNORECASE)
RE_HEX64 = re.compile(r'^[0-9a-f]{64}$', re.IGNORECASE)

# ------------------------- IO helpers ---------------------------------------

def read_any(path: str) -> pd.DataFrame:
    if path.lower().endswith((".xlsx", ".xls")):
        return pd.read_excel(path)
    return pd.read_csv(path)

def normalize_time_column(df: pd.DataFrame) -> pd.DataFrame:
    # Try common variants and standardize to "Time"
    for c in df.columns:
        lc = str(c).strip().lower()
        if lc in {"time", "timestamp", "date"} and c != "Time":
            return df.rename(columns={c: "Time"})
    return df

# ------------------------- Classification -----------------------------------

def classify_by_name(path: str) -> str:
    """
    Coarse file type based on filename.
    Returned tags drive renaming behavior.
    """
    s = os.path.basename(path).lower()
    if "missed" in s and ("deadline" in s or "dealine" in s or "dealines" in s):
        return "missed"
    if "packet loss rate" in s:
        return "packet"
    if "fec values" in s:
        return "fec"
    if "multicast retransmissions" in s and "rate" not in s:
        return "mretrans"
    if "unicast retransmissions" in s and "rate" not in s:
        return "uretrans"
    if "unicast rate per client" in s:
        return "urate"
    if "clients" in s:
        return "clients"
    if "multicast_retransmissions_rate" in s:
        return "mretrans_rate"
    if "total_retransmissions_rate" in s:
        return "total_retrans_rate"
    if "unicast_retransmissions_rate" in s:
        return "unicast_retrans_rate"
    if "multicast_protocol_overhead" in s:
        return "mp_overhead"
    if "multicast_sender_rate" in s:
        return "ms_rate"
    if "data_source_rate" in s:
        return "data_src_rate"
    if "protocol overhead percentage" in s:
        return "po_pct"
    return "other"

def reclassify_by_content(t: str, df: pd.DataFrame) -> str:
    """
    Content-based fallback to catch typos or ambiguous names.
    """
    cols = [str(c) for c in df.columns]
    if t == "other":
        if any('__name__="missed_deadlines_client"' in c for c in cols):
            return "missed"
        if set(map(str.lower, cols)) >= {"time", "fec_row", "fec_col"}:
            return "fec"
    return t

# ------------------------- Client mapping -----------------------------------

def extract_client_id(col_name: str) -> str | None:
    m = RE_EMBEDDED.search(col_name)
    if m:
        return m.group(1)
    if RE_HEX64.match(col_name):
        return col_name
    return None

def build_client_map(seed_df: pd.DataFrame | None) -> Dict[str, int]:
    """
    Seed the client map from a preferred per-client table (missed-deadlines).
    """
    client_map: Dict[str, int] = {}
    if seed_df is None:
        return client_map
    for col in seed_df.columns:
        if col == "Time":
            continue
        cid = extract_client_id(str(col))
        if cid and cid not in client_map:
            client_map[cid] = len(client_map) + 1
    return client_map

def rename_per_client(df: pd.DataFrame, prefix: str, client_map: Dict[str, int], next_idx_ref: List[int]) -> pd.DataFrame:
    new_cols = {}
    for col in df.columns:
        if col == "Time":
            new_cols[col] = col
            continue
        cid = extract_client_id(str(col))
        if not cid:
            new_cols[col] = col
            continue
        if cid not in client_map:
            client_map[cid] = next_idx_ref[0]
            next_idx_ref[0] += 1
        new_cols[col] = f"{prefix}_{client_map[cid]}"
    return df.rename(columns=new_cols)

def rename_single_series(df: pd.DataFrame, new_name: str) -> pd.DataFrame:
    data_cols = [c for c in df.columns if c != "Time"]
    if len(data_cols) == 1:
        return df.rename(columns={data_cols[0]: new_name})
    return df

# ------------------------- Merge core ---------------------------------------

def merge_metrics(paths: List[str]) -> pd.DataFrame:
    # Load frames and classify
    loaded: List[Tuple[str, str, pd.DataFrame]] = []  # (type, path, df)
    for p in paths:
        df = read_any(p)
        df = normalize_time_column(df)
        t = classify_by_name(p)
        t = reclassify_by_content(t, df)
        loaded.append((t, p, df))

    # Choose seed for client map
    missed_df = next((df for t, _, df in loaded if t == "missed"), None)
    client_map = build_client_map(missed_df)

    # If no seed, scan any per-client table to bootstrap
    if not client_map:
        for t, _, df in loaded:
            if t in {"packet", "mretrans", "uretrans", "urate"}:
                for col in df.columns:
                    if col == "Time":
                        continue
                    cid = extract_client_id(str(col))
                    if cid and cid not in client_map:
                        client_map[cid] = len(client_map) + 1

    next_idx_ref = [max(client_map.values(), default=0) + 1]

    prepared: List[pd.DataFrame] = []
    for t, path, df in loaded:
        # Skip non-time reference tables like Clients
        if "Time" not in df.columns:
            if t == "clients":
                continue
            # unknown non-time table: skip
            continue

        if t == "missed":
            prepared.append(rename_per_client(df, "missed_deadlines_client", client_map, next_idx_ref))
        elif t == "packet":
            prepared.append(rename_per_client(df, "packet_loss_client", client_map, next_idx_ref))
        elif t == "mretrans":
            prepared.append(rename_per_client(df, "multicast_retransmissions_client", client_map, next_idx_ref))
        elif t == "uretrans":
            prepared.append(rename_per_client(df, "unicast_retransmissions_client", client_map, next_idx_ref))
        elif t == "urate":
            prepared.append(rename_per_client(df, "unicast_rate_client", client_map, next_idx_ref))
        elif t == "fec":
            prepared.append(df.copy())  # keep fec_row and fec_col
        elif t == "mp_overhead":
            prepared.append(rename_single_series(df, "Multicast Protocol Overhead (kbit/s)"))
        elif t == "ms_rate":
            prepared.append(rename_single_series(df, "Multicast Sender Rate (kbit/s)"))
        elif t == "data_src_rate":
            prepared.append(rename_single_series(df, "Data Source Rate (kbit/s)"))
        elif t == "mretrans_rate":
            prepared.append(rename_single_series(df, "Multicast Retransmissions Rate (kbit/s)"))
        elif t == "total_retrans_rate":
            prepared.append(rename_single_series(df, "Total Retransmissions Rate (kbit/s)"))
        elif t == "unicast_retrans_rate":
            prepared.append(rename_single_series(df, "Unicast Retransmissions Rate (kbit/s)"))
        elif t == "po_pct":
            prepared.append(rename_single_series(df, "Protocol Overhead (%)"))
        else:
            # Unknown but time-indexed: include as is
            prepared.append(df.copy())

    if not prepared:
        raise ValueError("No time-indexed tables to merge.")

    merged = reduce(lambda l, r: pd.merge(l, r, on="Time", how="outer"), prepared)

    # Put Time first
    cols = ["Time"] + [c for c in merged.columns if c != "Time"]
    merged = merged.reindex(columns=cols)

    return merged

# --- Example runner ---------------------------------------------------------

if __name__ == "__main__":
    # Example: inject your file list here or parse from CLI.
    files = [
        "Missed Dealines-data-as-joinbyfield-2025-08-28 05_50_48.csv",
        "FEC Values-data-as-joinbyfield-2025-08-28 05_50_40.csv",
        "Packet Loss Rate-data-as-joinbyfield-2025-08-28 05_50_34.csv",
        "multicast_protocol_overhead (kbit_s)-data-2025-08-28 05_50_19.csv",
        "multicast_sender_rate (kbit_s)-data-2025-08-28 05_50_14.csv",
        "data_source_rate (kbit_s)-data-2025-08-28 05_50_09.csv",
        "Protocol Overhead Percentage-data-2025-08-28 05_50_02.csv"
    ]







    if files:
        out = merge_metrics(files)
        out.to_excel("merged_output.xlsx", index=False)
        print("Wrote merged_output.xlsx")
