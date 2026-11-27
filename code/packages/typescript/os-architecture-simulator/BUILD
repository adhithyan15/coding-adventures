#!/usr/bin/env bash
set -eu
npm ci --quiet
npm run build
node --experimental-strip-types --test tests/core.test.ts tests/machine.test.ts
node --experimental-strip-types ../../../../code/specs/fixtures/os-architecture-v1/verify_profiles.mjs
node --experimental-strip-types ../../../../code/specs/fixtures/os-machine-v1/verify_boards.mjs
