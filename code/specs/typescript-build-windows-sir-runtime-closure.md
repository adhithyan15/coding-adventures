# TypeScript Windows SIR Runtime Prerequisite Closure

## Status

This contract owns the three remaining TypeScript `BUILD_windows`
standalone-prerequisite failures after the `cli-builder` closure merged. It is
limited to the SIR core, object, and symbolic runtime dependency families.

## Problem

On Windows, `BUILD_windows` replaces the generic `BUILD` front door. An npm
`file:` dependency does not recursively install that dependency's local
`file:` dependencies, so each standalone front door must materialize its full
local prerequisite closure before running the package's own install.

The canonical Go build validator reports exactly three TypeScript failures on
merged main:

- `sir-runtime-core` lacks `sir-runtime-exceptions` and `sir-runtime-pairs`;
- `sir-runtime-oop` lacks `sir-runtime-core`, `sir-runtime-exceptions`, and
  `sir-runtime-pairs`; and
- `sir-runtime-symbolic` lacks `cas-pattern-matching` and `symbolic-ir`.

Executing the repaired core and object front doors from clean dependency state
also exposes package-local compiler prerequisite gaps: the core runtime uses
`process.stdout` without declaring Node's type package, its dynamically
constructed builtin table does not contextually type its callback parameters,
and the object runtime compares nullable SIR block-result values without
narrowing them. The existing strict type-check therefore fails before coverage
can run.

## Required Behavior

For each selected package:

1. `BUILD_windows` MUST install every local prerequisite declared by the
   generic `BUILD` front door.
2. Prerequisites MUST retain the generic front door's dependency-safe order.
3. Core and oop MUST declare the Node type package needed while their compiler
   traverses the core runtime's `process.stdout` calls. Core builtin dispatch
   callbacks MUST explicitly accept SIR `Val` arguments, and oop's extremal
   block-key comparisons MUST narrow nullable `Val` results without changing
   runtime behavior.
4. The package's existing `npm ci`, type-check, test, and coverage commands
   MUST remain byte-for-byte unchanged after the prerequisite bootstrap.
5. No generic `BUILD`, compiler configuration, non-core runtime behavior, or
   non-SIR package may change in this tranche.

The exact required orders are:

- core: `sir-runtime-exceptions`, then `sir-runtime-pairs`;
- oop: `sir-runtime-core`, then `sir-runtime-exceptions`, then
  `sir-runtime-pairs`; and
- symbolic: `cas-pattern-matching`, then `symbolic-ir`.

## Executable Validation

Before the repair, the canonical Go validator MUST reproduce exactly the three
diagnostics above. After the repair, the same TypeScript validator MUST pass
with no missing-prerequisite diagnostic.

A focused validator regression MUST prove that a complete multi-level
TypeScript Windows closure is accepted while omission of a transitive local
prerequisite remains rejected. Real validation MUST run each selected package's
coverage front door and strict type-check from a clean dependency state. The
core package metadata MUST retain the Node type dependency, and production
dependency security audits MUST pass.

The full Go build-tool test, vet, build, module-verification, committed-diff
plan, collision-checked package inventory, diff, and secret gates remain
required.
