#!/usr/bin/env bash
# BUILD — @coding-adventures/markov-chain
# =========================================
#
# Install transitive dependencies leaf-to-root before testing.
# The dependency chain is: directed-graph → markov-chain
#
# directed-graph contains its own graph.ts internally; it does not depend on
# the @coding-adventures/graph package. We only need to install directed-graph.
set -euo pipefail
cd "$(dirname "$0")"
(cd ../directed-graph && mise exec -- npm ci --quiet)
mise exec -- npm ci --quiet
mise exec -- npm run test:coverage
