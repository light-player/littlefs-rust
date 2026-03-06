#!/usr/bin/env python3
"""Parse trace logs for dir_find/fetchmatch/dir_split events.
Detects cycles in tail chain and summarizes split behavior.
Usage: python parse_trace.py rust-trace.log
"""
import re
import sys
from collections import OrderedDict

def parse_rust_log(path: str) -> dict:
    result = {
        "dir_find": [],
        "fetchmatch_continue": [],
        "fetchmatch_found": [],
        "fetchmatch_noent": [],
        "dir_split": [],
        "splittingcompact": [],
    }
    with open(path) as f:
        for line in f:
            line = line.strip()
            if "dir_find:" in line:
                m = re.search(r"iter=(\d+).*tag=(-?\d+).*split=(\w+).*tail=\[(\d+),(\d+)\].*namelen=(\d+)", line)
                if m:
                    result["dir_find"].append({
                        "iter": int(m.group(1)),
                        "tag": int(m.group(2)),
                        "split": m.group(3) == "true",
                        "tail": (int(m.group(4)), int(m.group(5))),
                        "namelen": int(m.group(6)),
                    })
            elif "fetchmatch: CONTINUE" in line:
                m = re.search(r"pair=\[(\d+),(\d+)\].*count=(\d+).*split=(\w+).*tail=\[(\d+),(\d+)\]", line)
                if m:
                    result["fetchmatch_continue"].append({
                        "pair": (int(m.group(1)), int(m.group(2))),
                        "count": int(m.group(3)),
                        "split": m.group(4) == "true",
                        "tail": (int(m.group(5)), int(m.group(6))),
                    })
            elif "fetchmatch: FOUND" in line:
                m = re.search(r"besttag=0x([0-9a-f]+).*pair=\[(\d+),(\d+)\].*count=(\d+).*split=(\w+).*tail=\[(\d+),(\d+)\]", line)
                if m:
                    result["fetchmatch_found"].append({
                        "besttag": int(m.group(1), 16),
                        "pair": (int(m.group(2)), int(m.group(3))),
                        "count": int(m.group(4)),
                        "split": m.group(5) == "true",
                        "tail": (int(m.group(6)), int(m.group(7))),
                    })
            elif "fetchmatch: NOENT" in line:
                result["fetchmatch_noent"].append(line)
            elif "dir_split:" in line:
                m = re.search(r"split=(\d+).*end=(\d+).*new_tail=\[(\d+),(\d+)\].*dir\.pair=\[(\d+),(\d+)\].*dir\.tail=\[(\d+),(\d+)\]", line)
                if m:
                    result["dir_split"].append({
                        "split": int(m.group(1)),
                        "end": int(m.group(2)),
                        "new_tail": (int(m.group(3)), int(m.group(4))),
                        "dir_pair": (int(m.group(5)), int(m.group(6))),
                        "dir_tail": (int(m.group(7)), int(m.group(8))),
                    })
            elif "splittingcompact:" in line:
                result["splittingcompact"].append(line)
    return result


def analyze(data: dict) -> str:
    lines = []
    lines.append("=== Trace Analysis ===")
    lines.append(f"dir_find iterations: {len(data['dir_find'])}")
    lines.append(f"fetchmatch CONTINUE: {len(data['fetchmatch_continue'])}")
    lines.append(f"fetchmatch FOUND: {len(data['fetchmatch_found'])}")
    lines.append(f"dir_split calls: {len(data['dir_split'])}")

    # Check for empty-range splits (split >= end)
    empty_splits = [s for s in data["dir_split"] if s["split"] >= s["end"]]
    if empty_splits:
        lines.append(f"\n*** BUG: {len(empty_splits)} dir_split calls with empty range (split >= end)! ***")
        lines.append("First few:")
        for s in empty_splits[:5]:
            lines.append(f"  split={s['split']} end={s['end']} new_tail={s['new_tail']}")

    # Check for cycles in dir_find tail sequence
    seen = {}
    cycle_at = None
    for d in data["dir_find"]:
        t = d["tail"]
        if t in seen:
            cycle_at = (d["iter"], t)
            break
        seen[t] = d["iter"]
    if cycle_at:
        lines.append(f"\nCycle detected at iter {cycle_at[0]} tail={cycle_at[1]}")
    else:
        lines.append(f"\nNo cycle in dir_find tail sequence; {len(seen)} unique tails")

    return "\n".join(lines)


def main():
    if len(sys.argv) < 2:
        print("Usage: parse_trace.py <trace.log>")
        sys.exit(1)
    path = sys.argv[1]
    data = parse_rust_log(path)
    print(analyze(data))


if __name__ == "__main__":
    main()
