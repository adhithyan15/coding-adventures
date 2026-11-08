#!/usr/bin/env bash
# build-tool: deps=typescript/cli-builder
set -euo pipefail
cd ../directed-graph && npm ci --quiet && npx tsc
cd ../state-machine && npm ci --quiet && npx tsc
cd ../cli-builder && npm ci --quiet && npx tsc
cd ../test-bootstrap
npm ci --quiet
npx tsc
node --experimental-strip-types --test --experimental-test-coverage tests/matchers.test.ts tests/conformance.test.ts tests/sabotage.test.ts tests/cli.test.ts
node bin/cli.mjs --help >/dev/null
node bin/cli.mjs --version >/dev/null
node bin/cli.mjs --self-corpus
# Sabotage check: pipeline status is grep's (TF01 §6)
(set +o pipefail; node bin/cli.mjs --sabotage always-pass 2>/dev/null | grep -q '^not ok ')
