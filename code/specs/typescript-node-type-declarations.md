# TypeScript Node Type Declaration Ownership

## Status

This contract owns build-script TypeScript projects whose compiler inputs use
Node.js APIs. It extends the repository's executable TypeScript build audit and
keeps type-only dependencies separate from runtime package dependencies.

## Problem

TypeScript does not provide declarations for Node built-in modules or globals.
A package can pass Vitest because the test runner brings ambient declarations
into its own process while `npm run build` still fails with TS2307 or TS2580.
The failure became visible in the Algol lexer/parser chain and the browser
extension toolkit after their unrelated TS6059 input leaks were repaired.

The syntax-aware merged-main audit finds exactly 93 build-script projects with
compiler-owned Node API use. Thirty-one already own a direct provider, 62 do
not, one of the 31 provider-owning projects lacks synchronized lock metadata,
and one has the reviewed native N-API lock exception below. The earlier textual
prioritization ceiling of 74 missing providers correctly overestimated the
exact missing corpus by 12.

## Required Behavior

1. The repository audit MUST inspect every TypeScript package and program with
   a build script and `tsconfig.json`.
2. The audit MUST lex Git-visible compiler-owned `.ts`, `.tsx`, `.mts`, and
   `.cts` inputs without requiring `node_modules`.
3. Real imports, re-exports, dynamic imports, and `require` calls for Node
   built-in modules MUST require a direct type provider. The same applies to
   unqualified uses of `process`, `Buffer`, `NodeJS`, `__dirname`, and
   `__filename`.
4. Comments, ordinary string contents, and member properties such as
   `worker.process()` MUST NOT count as Node API use.
5. A provider MUST be owned directly in `devDependencies`; a transitive lock
   entry or runtime dependency is not sufficient.
6. `package-lock.json` MUST agree with the root package's declaration and MUST
   contain a resolved `node_modules/@types/node` entry unless the project is the
   explicit reviewed exception below.
7. The selected repair MUST use the repository's Node 22 declaration baseline
   for newly owned providers, preserve existing dependency ranges, and make no
   runtime source changes.

## Executable Validation

Synthetic tests MUST cover static imports, re-exports, dynamic imports,
`require`, Node globals, template expressions, false-positive prose/property
cases, missing direct ownership, stale lock metadata, and the reviewed lock
exception. The repository test MUST lock the exact red corpus of 93 Node API
projects, 62 missing direct providers, one stale provider lock, and one reviewed
lock exception before metadata repair. After the repair all 93 MUST own direct
providers with zero missing or stale declarations and exactly one exception.

## Reviewed Lock Exception

`code/packages/typescript/matrix-rust-napi` deliberately regenerates
`package-lock.json` for the platform-specific native N-API workspace and tracks
an exact ignore rule with that rationale. The audit accepts only this named
project, only while its tracked `.gitignore` contains the exact lock entry, and
does not generalize the exception to other packages.

Representative packages from lexer/parser, CLI/tooling, server/runtime, and
browser-scaffold families MUST run their real Windows front doors, coverage,
and `npm run build` from declared dependencies. Production audits, the full
TypeScript build-file validator, Go build-tool test/vet/build/module gates,
collision-checked package inventory, committed diff plan, diff, formatting,
and added-line secret checks remain required.
