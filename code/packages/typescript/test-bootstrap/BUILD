#!/usr/bin/env bash
set -euo pipefail
npm ci --quiet
npx tsc
node --experimental-strip-types --test --experimental-test-coverage tests/matchers.test.ts tests/conformance.test.ts tests/sabotage.test.ts tests/cli.test.ts tests/pbt.test.ts
node bin/cli.mjs --help >/dev/null
node bin/cli.mjs --version >/dev/null
node bin/cli.mjs --self-corpus
# Sabotage check: pipeline status is grep's (TF01 §6)
(set +o pipefail; node bin/cli.mjs --sabotage always-pass 2>/dev/null | grep -q '^not ok ')
