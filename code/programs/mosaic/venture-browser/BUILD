#!/bin/sh

set -eu

script_root="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
cd "$script_root"

# Keep the package contract tests and the generated cross-platform shell matrix
# behind the same authoritative entry point used by repository CI.
cargo test
./scripts/build-all.sh
