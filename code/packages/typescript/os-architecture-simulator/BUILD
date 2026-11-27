#!/usr/bin/env bash
set -eu
npm ci --prefix ../transistors --quiet
npm ci --prefix ../logic-gates --quiet
npm ci --prefix ../arithmetic --quiet
npm ci --prefix ../cpu-simulator --quiet
npm ci --prefix ../riscv-simulator --quiet
npm ci --quiet
npm run build --prefix ../cpu-simulator
npm run build --prefix ../riscv-simulator
npm run build
npm test
node --experimental-strip-types ../../../../code/specs/fixtures/os-architecture-v1/verify_profiles.mjs
node --experimental-strip-types ../../../../code/specs/fixtures/os-machine-v1/verify_boards.mjs
npx vitest run --globals --root ../../../../code/specs fixtures/os-machine-v1/verify_riscv.test.ts
