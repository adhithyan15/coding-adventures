#!/usr/bin/env bash
# BUILD — @coding-adventures/markov-chain
# =========================================
#
# Install transitive dependencies leaf-to-root before testing.
# The dependency chain is: graph → directed-graph → markov-chain
#
# We must install in this order so that `npm ci` in each package
# can resolve its local `file:` dependencies correctly.
set -euo pipefail
cd "$(dirname "$0")"
(cd ../graph && mise exec -- npm ci --quiet)
(cd ../directed-graph && mise exec -- npm ci --quiet)
mise exec -- npm ci --quiet
mise exec -- npm run test:coverage
