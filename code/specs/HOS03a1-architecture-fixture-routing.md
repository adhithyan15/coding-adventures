# HOS03a1 — Affected-only routing for architecture fixtures

## Purpose

The TypeScript architecture simulator's normal BUILD runs two external fixture
verifiers: `verify_profiles.mjs` reads `profiles.json`, and `verify_boards.mjs`
reads `boards.json`. These files are canonical build inputs, but they are
outside the package tree and create no package dependency edge. Register their
exact paths so a fixture or verifier edit selects and invalidates only the
TypeScript architecture package.

## Exact input contract

The sole consumer is `typescript/os-architecture-simulator`. The original
HOS03a1 route registered exactly these four repository inputs:

| Input | BUILD consumer |
|---|---|
| `code/specs/fixtures/os-architecture-v1/profiles.json` | shared profile verifier |
| `code/specs/fixtures/os-architecture-v1/verify_profiles.mjs` | shared profile verifier source |
| `code/specs/fixtures/os-machine-v1/boards.json` | synthetic board verifier |
| `code/specs/fixtures/os-machine-v1/verify_boards.mjs` | synthetic board verifier source |

HOS03b adds exactly two more inputs to this same selector and fallback-cache
row: `code/specs/fixtures/os-machine-v1/riscv.json` and
`code/specs/fixtures/os-machine-v1/verify_riscv.test.ts`. They are listed in
HOS03b because the RV32I verifier now reads and executes that fixture. The
original four above remain the HOS03a1 baseline.

The selector and fallback cache hash must use the same six-file current list.
A change to any listed file selects this package; a TypeScript-only plan
selects only this package for those changes. No directory-prefix match is
allowed. Neighbor README files, similarly named paths, other fixtures, and
unrelated package tests do not select it. Git change lists include the old
path for a deletion or rename, so removing or renaming a canonical input still
selects the consumer.

Fixture reads remain in the BUILD-only verifier scripts outside the package.
The reusable package keeps its empty OS-capability profile. Future language
consumers must register their own exact rows when their BUILD actually reads
these fixtures; this contract does not broaden fixture routing to other lanes.

## Evidence

Build-tool tests prove exact selection for all six current paths, language
filtering, exclusion of nearby lookalikes, rename-path behavior, and that
changing each listed file changes the TypeScript package's fallback hash. They
also assert that the package hash input list equals the six-file set above.
