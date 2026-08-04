# TypeScript Standalone Config Output Isolation

## Status

This contract owns TypeScript package and program configs that run a compiler
build without extending `code/packages/typescript/tsconfig.base.json`.
Shared-base path portability is specified separately in
`typescript-shared-tsconfig-portability.md`.

## Problem

TypeScript emits next to each input when an emit-capable project declares no
`outDir`. That behavior is unsafe for a repository build front door: a clean
build mutates tracked `src`, `tests`, and root config trees with generated
JavaScript and declarations.

The current repository audit finds exactly two standalone in-place emitters:

- `code/packages/typescript/window-core/tsconfig.json`; and
- `code/packages/typescript/window-canvas/tsconfig.json`.

Before this repair, each package's `npm run build` creates `.js` and `.d.ts`
siblings for `src/index.ts`, its test module, and `vitest.config.ts`.

## Required Behavior

Every TypeScript package or program that has a sibling `tsconfig.json` and a
`package.json` build script MUST satisfy one of these output policies:

1. extend the shared TypeScript base, whose output path is governed by the
   shared portability contract;
2. set `compilerOptions.noEmit` to the JSON boolean `true`; or
3. declare a non-empty string `compilerOptions.outDir`.

`window-core` and `window-canvas` MUST use `outDir: "dist"`. Their existing
input set remains authoritative, so the compiler preserves the package-root
relative `src`, `tests`, and `vitest.config` layout under `dist`. This tranche
does not change runtime APIs, dependency declarations, or source entry points.

## Executable Audit

The repository TypeScript config auditor MUST reject an emit-capable
standalone project with:

- code `STANDALONE_OUTPUT_NOT_ISOLATED`;
- the repository-relative `tsconfig.json` path; and
- a stable explanation that `noEmit: true` or a non-empty `outDir` is required.

The audit summary MUST report the number of standalone emit-capable projects
and how many declare isolated output. Empty or non-string `outDir` values are
invalid rather than accepted as isolation.

## Validation

Tests MUST cover:

- an emitting standalone project without `outDir`;
- `noEmit: true` as a valid type-check-only project;
- a non-empty standalone `outDir` as a valid emitting project;
- shared-base consumers remaining owned by the shared contract; and
- the real repository, with both current standalone emitters isolated.

Real package validation MUST install `window-core` before `window-canvas`, run
both build and coverage front doors, and prove that generated `.js`, `.d.ts`,
and source maps exist only under each package's ignored `dist` directory. A
post-build Git-visible file scan MUST find no generated siblings in tracked
source, test, or root config trees.

