#!/usr/bin/env bash

set -euo pipefail

script_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$script_root"

# Keep the package contract tests and the generated cross-platform shell matrix
# behind the same authoritative entry point used by repository CI.
cargo test
./scripts/build-all.sh
