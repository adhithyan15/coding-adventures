#!/usr/bin/env bash
# needs-toolchain: python
set -euo pipefail
npm ci --quiet
npx tsc
node --experimental-strip-types --test --experimental-test-coverage tests/matchers.test.ts tests/conformance.test.ts tests/sabotage.test.ts tests/cli.test.ts tests/pbt.test.ts tests/fuzz.test.ts tests/warrant.test.ts tests/sat.test.ts
node bin/cli.mjs --help >/dev/null
node bin/cli.mjs --version >/dev/null
node bin/cli.mjs --self-corpus
# Sabotage check (TF01 §6): the shared checker (WD-HARN1c) runs the sabotaged harness with no
# shell and passes only on a non-zero exit, one well-formed TAP plan and a real `not ok`.
python3 ../../../scripts/warrant_negctl.py -- node bin/cli.mjs --sabotage always-pass
