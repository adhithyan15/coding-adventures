# HOS03a1 — Affected-only routing for architecture fixtures

## Purpose

The TypeScript architecture simulator's normal BUILD runs two external fixture
verifiers: `verify_profiles.mjs` reads `profiles.json`, and `verify_boards.mjs`
reads `boards.json`. These files are canonical build inputs, but they are
outside the package tree and create no package dependency edge. Register their
exact paths so a fixture or verifier edit selects and invalidates only the
TypeScript architecture package.

## Exact input contract

The sole consumer is `typescript/os-architecture-simulator`. Its registered
repository inputs are exactly:

| Input | BUILD consumer |
|---|---|
| `code/specs/fixtures/os-architecture-v1/profiles.json` | shared profile verifier |
| `code/specs/fixtures/os-architecture-v1/verify_profiles.mjs` | shared profile verifier source |
| `code/specs/fixtures/os-machine-v1/boards.json` | synthetic board verifier |
| `code/specs/fixtures/os-machine-v1/verify_boards.mjs` | synthetic board verifier source |

The selector and fallback cache hash must use the same four-file list. A change
to any listed file selects this package; a TypeScript-only plan selects only
this package for those changes. No directory-prefix match is allowed. Neighbor
README files, similarly named paths, other fixtures, and unrelated package
tests do not select it. Git change lists include the old path for a deletion or
rename, so removing or renaming a canonical input still selects the consumer.

Fixture reads remain in the BUILD-only verifier scripts outside the package.
The reusable package keeps its empty OS-capability profile. Future language
consumers must register their own exact rows when their BUILD actually reads
these fixtures; this contract does not broaden fixture routing to other lanes.

## Evidence

Build-tool tests must prove exact selection for each of the four paths, language
filtering, exclusion of nearby lookalikes, rename-path behavior, and that
changing each listed file changes the TypeScript package's fallback hash. They
must also assert that the package hash input list equals this table exactly.
