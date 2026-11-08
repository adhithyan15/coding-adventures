#!/usr/bin/env bash
set -euo pipefail
npx tsc
node --experimental-strip-types --test --experimental-test-coverage tests/matchers.test.ts tests/conformance.test.ts tests/sabotage.test.ts
node dist/cli.js
# Sabotage check: pipeline status is grep's (TF01 §6)
(set +o pipefail; node dist/cli.js --sabotage always-pass 2>/dev/null | grep -q '^not ok ')
