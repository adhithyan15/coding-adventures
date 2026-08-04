# TypeScript Shared TSConfig Portability

## Status

This document defines the repository contract for
`code/packages/typescript/tsconfig.base.json` and every package or program that
extends it. It is a build-portability contract, not a change to any package's
runtime behavior, public API, source selection, module format, or dependency
graph.

## Problem

TypeScript resolves an ordinary relative path inherited from a base config
relative to the file that declared the path. The former shared config declared
`rootDir: "src"` and `outDir: "dist"`, so a derived package inherited the
shared directory's `src` and `dist` rather than its own. A clean build of
`typescript/transistors` therefore failed with TS6059 because
`transistors/src/index.ts` was outside `code/packages/typescript/src`.

The repository has 458 TypeScript package/program configs with build scripts.
Of those, 129 directly extend the shared base without overriding either path;
three more override only `rootDir` and still inherit `outDir`; 155 directly
extend it and override both paths; and the remaining configs do not consume
this shared path contract. The shared repair therefore affects 132 configs in
total. Local overrides remain authoritative.

## Required behavior

The shared compiler options MUST declare:

```json
{
  "rootDir": "${configDir}/src",
  "outDir": "${configDir}/dist"
}
```

TypeScript 5.5 introduced `${configDir}` for this exact shared-config use case.
At compilation time it denotes the directory containing the derived project
config, so an extending package receives its own `src` and `dist` paths. See
the official [TypeScript 5.5 release notes](https://www.typescriptlang.org/docs/handbook/release-notes/typescript-5-5.html#the-configdir-template-variable-for-configuration-files).

The contract has these invariants:

1. A derived config that does not override `rootDir` resolves it to that
   project's `src` directory.
2. A derived config that does not override `outDir` resolves it to that
   project's `dist` directory.
3. A derived config's explicit `rootDir` or `outDir` continues to override the
   shared default.
4. The shared config does not change `include`, `exclude`, or `files`; path
   portability must not broaden a compilation.
5. Every checked-in TypeScript lock that supplies the compiler for this shared
   contract must remain on TypeScript 5.5 or newer.
6. Clean builds must not emit JavaScript, declarations, or source maps into the
   shared `code/packages/typescript/src` or `code/packages/typescript/dist`
   directories.

## Conformance

Repository validation MUST check the exact shared path templates, audit every
tracked TypeScript package/program manifest with a build script and sibling
`tsconfig.json`, and reject a compiler lock below TypeScript 5.5. Real compiler
validation MUST cover:

- a minimal package that inherits both paths (`transistors`);
- a dependency consumer that inherits both paths (`logic-gates`);
- a package with explicit local overrides, proving they remain authoritative;
- the one program config that inherits the shared paths; and
- all 129 current `rootDir` inheritors and 132 `outDir` inheritors via
  `tsc --showConfig`, requiring package-local effective values.

Standalone configs that do not extend this base are outside this contract. Any
in-place emission gap they contain is separately owned and must not be repaired
by widening this shared-config change.
