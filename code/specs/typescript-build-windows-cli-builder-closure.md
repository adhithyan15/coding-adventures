# TypeScript Windows `cli-builder` Prerequisite Closure

## Status

This contract owns the 55-package TypeScript `BUILD_windows` repair selected
from the broader standalone-build integrity backlog. The three SIR runtime
packages in the same validator snapshot are a separate dependency family.

## Problem

On Windows, `BUILD_windows` overrides the generic `BUILD` front door. A local
`file:` dependency installed by an npm package does not recursively materialize
that dependency's own local `file:` dependencies. Every standalone front door
must therefore install the complete local prerequisite closure before running
the package's own `npm ci`.

The canonical Go build validator reports 58 TypeScript Windows failures on
merged main. Fifty-four packages are missing only `typescript/cli-builder`.
`typescript/grammar-tools` is missing `cli-builder` plus its direct
`directed-graph` and `state-machine` prerequisites. The remaining three
failures belong to the separately tracked SIR runtime closure.

## Required Behavior

For each selected package:

1. `BUILD_windows` MUST install `../cli-builder` before any dependent package
   that reaches it through a local `file:` edge.
2. Existing prerequisite installs MUST retain the same dependency-safe order
   as the generic `BUILD` front door.
3. `grammar-tools/BUILD_windows` MUST materialize `directed-graph`,
   `state-machine`, and `cli-builder` in the order used by its generic `BUILD`.
4. The package's existing install, test, coverage, and build commands MUST remain
   unchanged.
5. No generic `BUILD`, package manifest, runtime source, or SIR package may be
   changed in this tranche.

## Executable Validation

Before the repair, the canonical Go validator MUST reproduce exactly 58
TypeScript missing-prerequisite diagnostics, partitioned as:

- 54 packages missing only `typescript/cli-builder`;
- `grammar-tools` missing `typescript/cli-builder`,
  `typescript/directed-graph`, and `typescript/state-machine`; and
- three SIR runtime diagnostics outside this contract.

After the repair, the same validator MUST report exactly the three SIR runtime
diagnostics and no selected-package diagnostic. A diff-driven Windows dry run
MUST select the repaired packages without introducing undeclared local refs.

Real validation MUST execute representative Windows front doors for the base
`grammar-tools` package and at least one downstream lexer/parser chain, run the
package coverage suites, and audit production dependencies. The full Go build
tool test, vet, build, module-verification, collision-checked parity inventory,
diff, and secret gates remain required.
