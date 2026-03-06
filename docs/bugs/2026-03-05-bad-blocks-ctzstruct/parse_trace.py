#!/usr/bin/env python3
"""Parse trace logs for CTZSTRUCT commit events.

Extracts commitattr CTZSTRUCT, traverse ProcessTag CTZSTRUCT, and traverse
PopAndProcess CTZSTRUCT events. Flags when id=2 (pacman) gets wrong (head, size).

Usage: python parse_trace.py rust-trace.log
"""
import re
import sys
from dataclasses import dataclass
from typing import Optional


@dataclass
class CtzCommitEvent:
    """A commitattr CTZSTRUCT event."""
    id: int
    from_disk: bool
    head: Optional[int] = None
    size: Optional[int] = None
    disk_block: Optional[int] = None
    disk_off: Optional[int] = None
    commit_block: int = 0
    commit_off: int = 0
    line: str = ""


def parse_rust_log(path: str) -> dict:
    result = {
        "compact": [],
        "commitattr_ctz": [],
        "traverse_process_ctz": [],
        "traverse_pop_ctz": [],
    }
    with open(path) as f:
        for line in f:
            line = line.strip()
            if "lfs_dir_compact: traverse" in line:
                m = re.search(
                    r"source=\[(\d+),(\d+)\].*begin=(\d+).*end=(\d+).*dir\.pair=\[(\d+),(\d+)\]",
                    line,
                )
                if m:
                    result["compact"].append({
                        "source": (int(m.group(1)), int(m.group(2))),
                        "begin": int(m.group(3)),
                        "end": int(m.group(4)),
                        "dir_pair": (int(m.group(5)), int(m.group(6))),
                    })
            elif "commitattr CTZSTRUCT:" in line:
                evt = CtzCommitEvent(id=0, from_disk=False, line=line)
                m = re.search(r"id=(\d+)", line)
                if m:
                    evt.id = int(m.group(1))
                if "from_disk=true" in line:
                    evt.from_disk = True
                    m = re.search(r"disk\.block=(\d+).*disk\.off=(\d+)", line)
                    if m:
                        evt.disk_block = int(m.group(1))
                        evt.disk_off = int(m.group(2))
                else:
                    m = re.search(r"head=(\d+).*size=(\d+)", line)
                    if m:
                        evt.head = int(m.group(1))
                        evt.size = int(m.group(2))
                m = re.search(r"commit\.block=(\d+).*commit\.off=(\d+)", line)
                if m:
                    evt.commit_block = int(m.group(1))
                    evt.commit_off = int(m.group(2))
                result["commitattr_ctz"].append(evt)
            elif "traverse ProcessTag CTZSTRUCT:" in line:
                m = re.search(r"id=(\d+).*disk_override=(\w+).*buffer=0x([0-9a-f]+)", line)
                if m:
                    result["traverse_process_ctz"].append({
                        "id": int(m.group(1)),
                        "disk_override": m.group(2) == "true",
                        "buffer": m.group(3),
                    })
            elif "traverse PopAndProcess CTZSTRUCT:" in line:
                m = re.search(
                    r"frame\.tag=0x([0-9a-f]+).*id=(\d+).*frame\.disk=\((\d+),(\d+)\).*"
                    r"frame\.buffer=0x([0-9a-f]+).*disk_override=(\w+)",
                    line,
                )
                if m:
                    result["traverse_pop_ctz"].append({
                        "frame_tag": m.group(1),
                        "id": int(m.group(2)),
                        "disk": (int(m.group(3)), int(m.group(4))),
                        "buffer": m.group(5),
                        "disk_override": m.group(6) == "true",
                    })
    return result


def analyze(data: dict) -> str:
    lines = []
    lines.append("=== CTZSTRUCT Trace Analysis ===")
    lines.append(f"compact traverse calls: {len(data['compact'])}")
    lines.append(f"commitattr CTZSTRUCT: {len(data['commitattr_ctz'])}")
    lines.append(f"traverse ProcessTag CTZSTRUCT: {len(data['traverse_process_ctz'])}")
    lines.append(f"traverse PopAndProcess CTZSTRUCT: {len(data['traverse_pop_ctz'])}")

    # CTZSTRUCT commit sequence: id=2 is pacman, id=1 is ghost
    # Bug: pacman (id=2) should have head=6, size=504; corruption gives head=4, size=0
    lines.append("")
    lines.append("--- commitattr CTZSTRUCT sequence ---")
    for i, evt in enumerate(data["commitattr_ctz"]):
        if evt.from_disk:
            lines.append(
                f"  [{i}] id={evt.id} from_disk disk=({evt.disk_block},{evt.disk_off}) "
                f"commit=({evt.commit_block},{evt.commit_off})"
            )
        else:
            lines.append(
                f"  [{i}] id={evt.id} from_mem head={evt.head} size={evt.size} "
                f"commit=({evt.commit_block},{evt.commit_off})"
            )
        if evt.id == 2 and evt.head is not None and (evt.head != 6 or evt.size != 504):
            lines.append(f"       *** BUG: pacman (id=2) has wrong head={evt.head} size={evt.size} (expected 6, 504)")

    # Check for disk_override usage
    lines.append("")
    lines.append("--- PopAndProcess with disk_override ---")
    for p in data["traverse_pop_ctz"]:
        if p["disk_override"]:
            lines.append(f"  id={p['id']} disk={p['disk']} disk_override=TRUE")

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
