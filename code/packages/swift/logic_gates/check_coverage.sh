#!/bin/sh
# Line-coverage gate for Sources/LogicGates (>= 95%, per repo standard).
# Usage: sh check_coverage.sh   (run after `swift test --enable-code-coverage`)
set -e
SWIFT="swift"; if command -v xcrun >/dev/null 2>&1; then SWIFT="xcrun swift"; fi
JSON=$($SWIFT test --show-codecov-path | tail -1)
python3 -I - "$JSON" <<PY
import json, sys
data = json.load(open(sys.argv[1]))["data"][0]["files"]
files = [f for f in data if "/Sources/LogicGates/" in f["filename"]]
cov = sum(f["summary"]["lines"]["covered"] for f in files)
tot = sum(f["summary"]["lines"]["count"] for f in files)
pct = 100.0 * cov / tot
print("LogicGates line coverage: %.2f%% (%d/%d)" % (pct, cov, tot))
sys.exit(0 if pct >= 95.0 else 1)
PY
