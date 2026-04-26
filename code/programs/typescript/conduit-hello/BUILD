set -e

# BUILD — conduit-hello (TypeScript demo program)
#
# Depends on the conduit package being built first.  The build tool handles
# dependency ordering via the directed-graph affected_nodes() traversal.
#
# Tools (npm, npx, node) are expected on PATH (mise shims locally; CI installs
# directly via actions/setup-node).
#
# Steps:
#   1. Install npm dependencies (tsx + vitest devDeps)
#   2. Run the test suite (imports conduit package src/ directly)

npm ci --quiet
npx vitest run
