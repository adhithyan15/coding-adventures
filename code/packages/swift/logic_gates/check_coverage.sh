#!/bin/sh
# Line-coverage gate for Sources/LogicGates (>= 95%, per repo standard).
# Usage: sh check_coverage.sh   (run after `swift test --enable-code-coverage`)
# Fails closed: any missing step or missing JSON is a non-zero exit.
set -e
SWIFT="swift"
if command -v xcrun >/dev/null 2>&1; then SWIFT="xcrun swift"; fi
out=$($SWIFT test --show-codecov-path)
JSON=$(printf '%s\n' "$out" | tail -1)
[ -f "$JSON" ] || { echo "coverage JSON missing" >&2; exit 1; }
python3 -I - "$JSON" <<'PY'
import json, sys
data = json.load(open(sys.argv[1]))["data"][0]["files"]
files = [f for f in data if "/Sources/LogicGates/" in f["filename"]]
if not files:
    print("no LogicGates files in coverage data", file=sys.stderr)
    sys.exit(1)
cov = sum(f["summary"]["lines"]["covered"] for f in files)
tot = sum(f["summary"]["lines"]["count"] for f in files)
pct = 100.0 * cov / tot
print("LogicGates line coverage: %.2f%% (%d/%d)" % (pct, cov, tot))
sys.exit(0 if pct >= 95.0 else 1)
PY
