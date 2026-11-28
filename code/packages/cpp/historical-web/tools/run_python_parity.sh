#!/bin/sh
# Optional local reference audit. Kept out of BUILD because the Python source
# tree is not part of all package projections, and its syntax requires Python 3.12+.
set -eu
SELF=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF"
ROOT="$SELF"
while [ "$ROOT" != "/" ] && [ ! -d "$ROOT/code/packages/python/historical-web/src" ]; do ROOT=$(dirname "$ROOT"); done
if [ ! -d "$ROOT/code/packages/python/historical-web/src" ]; then
    echo "historical-web: Python reference source is not present in this checkout" >&2; exit 1
fi
mkdir -p _build
${CXX:-c++} -std=c++17 -pedantic-errors -Wall -Wextra -Werror -Iinclude \
    tests/python_parity_adapter.cpp src/historical_web.cpp -o _build/python-parity-adapter
PYTHON=${PYTHON:-python3}
"$PYTHON" tests/python_parity.py _build/python-parity-adapter 5000
