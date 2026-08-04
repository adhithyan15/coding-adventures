# TypeScript Build Input Boundaries

## Status

This contract owns the build-script TypeScript projects whose effective
compiler root is `src` but whose project config does not bound the compiler's
input set. It extends the executable shared-tsconfig portability audit; it does
not change package runtime behavior or test discovery.

## Problem

`compilerOptions.rootDir` describes the expected source-tree layout, but it
does not select compiler inputs. When a `tsconfig.json` omits top-level
`include`, `files`, and `exclude`, TypeScript applies its default include and
admits every supported source file below the project directory. A tracked test
or `vitest.config.ts` outside `src` then enters `npm run build` and triggers
TS6059 because it is outside the effective root.

The merged-main audit finds exactly 96 build-script projects with:

- an effective compiler root below the project's `src` directory;
- no explicit top-level input boundary; and
- 202 tracked `.ts`, `.tsx`, `.mts`, or `.cts` files outside that root.

`typescript/algol-lexer` is the representative failure: its unchanged build
admits `tests/tokenizer.test.ts` and fails with TS6059.

## Required Behavior

1. The repository audit MUST inspect every TypeScript package and program that
   declares a build script and a `tsconfig.json`.
2. For a project with an effective `rootDir`, the audit MUST identify tracked
   TypeScript inputs below the project but outside that root.
3. A project with such outside-root files MUST declare a top-level `include`,
   `files`, or `exclude` boundary. The canonical repair for the selected corpus
   is `"include": ["src"]`.
4. The repair MUST preserve every existing `extends` value and compiler option.
5. Vitest continues to discover tests through its own configuration; the build
   compiler emits or checks source files only.
6. Package manifests, lockfiles, BUILD front doors, runtime sources, and test
   sources MUST remain unchanged in this tranche.

## Executable Validation

Before the repair, the audit MUST report exactly 96
`INPUT_BOUNDARY_MISSING` diagnostics covering 202 outside-root tracked files.
After the repair, the same 458-project audit MUST report no issue and zero
unbounded rooted projects.

Synthetic tests MUST reject a rooted project whose default input set includes a
tracked test and accept the same project after `include: ["src"]` is declared.
Real validation MUST run coverage for representative lexer/parser,
hardware/runtime, and browser-toolkit projects while proving their tests remain
discoverable. Clean parser, hardware/runtime, and browser-capable
representatives MUST also build from their declared dependencies. A separate
package-manifest failure that becomes visible only after TS6059 is removed MUST
be logged as a separate backlog item instead of widening this configuration
tranche.

The full TypeScript build validator, Go build-tool test/vet/build/module gates,
collision-checked package inventory, committed diff plan, diff, formatting, and
secret gates remain required.
