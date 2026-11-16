#!/usr/bin/env bash
# build-tool: deps=typescript/test-bootstrap
set -euo pipefail
npm ci --quiet
[ -f ../test-bootstrap/dist/index.js ] || (cd ../test-bootstrap && npx tsc)
npx tsc
node --experimental-strip-types --test tests/analyzer.test.ts tests/pbt.test.ts tests/hostile.test.ts tests/cli.test.ts
node bin/cli.mjs --help >/dev/null
node bin/cli.mjs --version >/dev/null
