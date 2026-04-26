set -e

# BUILD — conduit-hello (TypeScript demo program)
#
# Depends on the conduit package being built first.  The build tool reads
# `file:` deps from package.json to construct its build graph, and the
# BUILD validator requires that prerequisite packages are referenced via
# relative-path commands in this BUILD file (so standalone builds can
# install the dep before running tests).
#
# Tools (npm, npx, node) are expected on PATH (mise shims locally; CI installs
# directly via actions/setup-node).
#
# Each line runs in its own shell starting from the package directory, so
# `cd ../../../packages/typescript/conduit` is a transient cd that doesn't
# persist into the next line.

cd ../../../packages/typescript/conduit && npm ci --quiet
npm ci --quiet
npx vitest run
