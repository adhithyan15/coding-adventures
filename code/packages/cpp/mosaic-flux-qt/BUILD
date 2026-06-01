#!/usr/bin/env bash
# BUILD — build + test mosaic-flux-qt
set -euo pipefail
cd "$(dirname "$0")"
mise exec -- cmake -B build -S . -DCMAKE_BUILD_TYPE=Debug
mise exec -- cmake --build build
mise exec -- ctest --test-dir build --output-on-failure
