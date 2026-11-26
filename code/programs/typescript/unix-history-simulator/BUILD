#!/usr/bin/env bash
set -euo pipefail

# The browser app consumes the TypeScript library, whose tests require the SAT
# solver and fuzz packages. Materialize the complete local file: dependency
# chain in leaf-to-root order before installing the program.
(cd ../../../packages/typescript/fuzz && npm ci --quiet)
(cd ../../../packages/typescript/sat-solver && npm ci --quiet)
(cd ../../../packages/typescript/sat-solver && npm run build)
(cd ../../../packages/typescript/unix-history-simulator && npm ci --quiet)
npm ci --quiet
npm run build
npm test
