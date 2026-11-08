#!/usr/bin/env bash
set -euo pipefail

npm ci --quiet
npx tsc

node --experimental-strip-types --test tests/sat.test.ts
