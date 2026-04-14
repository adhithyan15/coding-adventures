#!/usr/bin/env bash
# Thin wrapper around the pure-Ruby grammar-tools generator.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

exec env LANG=en_US.UTF-8 mise exec -- ruby \
  "$REPO_ROOT/code/programs/ruby/grammar-tools/main.rb" \
  generate-compiled-grammars \
  "$@"
