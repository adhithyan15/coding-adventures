#!/usr/bin/env bash
set -euo pipefail
npm ci --quiet
npm run build
node --experimental-strip-types --test tests/core.test.ts
node --experimental-strip-types ../../../../code/specs/fixtures/os-architecture-v1/verify_profiles.mjs
