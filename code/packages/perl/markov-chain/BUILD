#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

# Install the directed-graph dependency first (leaf before root).
(cd ../../perl/directed-graph && cpanm --installdeps --quiet .)

# Install our own dependencies (Test::More etc.) and run tests.
cpanm --installdeps --quiet .
prove -l -v t/
