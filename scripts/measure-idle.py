#!/usr/bin/env python3
"""Endpoint-only daemon CPU/memory/scheduler sample; keep the touchpad idle."""
import argparse
import json
import os
import pathlib
import subprocess
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--seconds", type=float, default=30)
parser.add_argument("--output", type=pathlib.Path)
args = parser.parse_args()
if not 1 <= args.seconds <= 3600:
    parser.error("seconds must be between 1 and 3600")
pid = int(subprocess.check_output(["systemctl", "--user", "show", "edgeslide.service", "-p", "MainPID", "--value"]))
if pid <= 0:
    parser.error("edgeslide.service is not running")
process = pathlib.Path("/proc") / str(pid)

def snapshot():
    stat = (process / "stat").read_text().split(") ", 1)[1].split()
    tasks = {}
    for task in (process / "task").iterdir():
        status = dict(line.split(":", 1) for line in (task / "status").read_text().splitlines() if ":" in line)
        tasks[task.name] = {
            "runtime_ns": int((task / "schedstat").read_text().split()[0]),
            "voluntary": int(status["voluntary_ctxt_switches"]),
            "involuntary": int(status["nonvoluntary_ctxt_switches"]),
            "wait": (task / "wchan").read_text(),
        }
    return {"ticks": int(stat[11]) + int(stat[12]), "tasks": tasks}

before = snapshot()
start = time.monotonic()
time.sleep(args.seconds)
elapsed = time.monotonic() - start
after = snapshot()
status = dict(line.split(":", 1) for line in (process / "status").read_text().splitlines() if ":" in line)
result = {
    "pid": pid,
    "seconds": elapsed,
    "clock_ticks_per_second": os.sysconf("SC_CLK_TCK"),
    "cpu_ticks_delta": after["ticks"] - before["ticks"],
    "cpu_percent_one_core": 100 * (after["ticks"] - before["ticks"]) / os.sysconf("SC_CLK_TCK") / elapsed,
    "rss": status["VmRSS"].strip(),
    "pss": next(line.strip() for line in (process / "smaps_rollup").read_text().splitlines() if line.startswith("Pss:")),
    "before": before,
    "after": after,
}
text = json.dumps(result, indent=2)
if args.output:
    args.output.write_text(text + "\n")
print(text)
