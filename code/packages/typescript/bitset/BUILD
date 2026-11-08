#!/usr/bin/env bash
# build-tool: deps=typescript/test-bootstrap
set -euo pipefail
[ -f ../test-bootstrap/dist/index.js ] || (cd ../test-bootstrap && npx tsc)
npx tsc
node --experimental-strip-types --test tests/bitset.test.ts
