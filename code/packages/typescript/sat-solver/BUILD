#!/usr/bin/env bash
set -euo pipefail

npm ci --quiet
npm ci --prefix ../fuzz --quiet
npm run build

node --experimental-strip-types --test tests/sat.test.ts
