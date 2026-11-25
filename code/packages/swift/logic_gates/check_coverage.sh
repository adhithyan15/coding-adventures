#!/bin/sh
# Line-coverage gate for Sources/LogicGates (>= 95%, per repo standard).
# Usage: sh check_coverage.sh   (run after `swift test --enable-code-coverage`)
# Fails closed: any missing step or missing JSON is a non-zero exit.
set -e
SWIFT="swift"
if command -v xcrun >/dev/null 2>&1; then SWIFT="xcrun swift"; fi
# `swift test --show-codecov-path` is the canonical location, but on Linux
# SwiftPM 6.4 it can report the path for a different build system than the
# one that ran the tests. Fall back to the newest codecov JSON the test run
# actually wrote under .build/. Still fails closed if neither exists.
out=$($SWIFT test --show-codecov-path 2>/dev/null || true)
JSON=$(printf '%s\n' "$out" | tail -1)
if [ ! -f "$JSON" ]; then
  JSON=$(find .build -type f -path '*/codecov/*.json' -newer Package.swift 2>/dev/null \
         | xargs ls -t 2>/dev/null | head -1)
fi
[ -n "$JSON" ] && [ -f "$JSON" ] || { echo "coverage JSON missing" >&2; exit 1; }
echo "coverage JSON: $JSON"
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
