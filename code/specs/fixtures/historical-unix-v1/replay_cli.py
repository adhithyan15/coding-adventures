"""Replay the Unix traces through a port's JSON command-line adapter."""

import json
import subprocess
import sys
from pathlib import Path


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: replay_cli.py EXECUTABLE [ARG ...]", file=sys.stderr)
        return 2
    fixture_path = Path(__file__).with_name("file-read.json")
    cases = json.loads(fixture_path.read_text(encoding="utf-8"))
    for case in cases:
        command = case["command"]
        result = subprocess.run(
            [*sys.argv[1:], command],
            check=True,
            stdout=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            timeout=30,
        )
        actual = json.loads(result.stdout)
        expected = {"output": case["output"], "events": case["events"]}
        if actual != expected:
            print(f"{command!r}: expected {expected!r}, got {actual!r}", file=sys.stderr)
            return 1
    print(f"Replayed {len(cases)} historical Unix traces")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
