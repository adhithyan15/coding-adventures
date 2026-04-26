set -e

# BUILD — conduit-hello (TypeScript demo program)
#
# Depends on the conduit package being built first.  The build tool handles
# dependency ordering via the directed-graph affected_nodes() traversal.
#
# Steps:
#   1. Install npm dependencies (tsx + vitest devDeps)
#   2. Run the test suite (imports conduit package src/ directly)

mise exec -- npm ci --quiet
mise exec -- npx vitest run
