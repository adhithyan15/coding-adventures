# SPICE Tool Compatibility Contract

This is the post-Berkeley v1 compatibility register. It is deliberately a
contract and a backlog, not a claim that the simulator already replaces
Berkeley SPICE3, ngspice, or LTspice.

The machine-readable source is
`code/grammars/spice/tool-compatibility-v1.json`. All three parser suites load
it directly. A change that adds a profile or feature but leaves a missing
family, profile mapping, state, or evidence entry fails in Python, Rust, and
TypeScript.

## Drop-in Definition

A target profile becomes drop-in eligible only when all of its reference-manual
features have an explicit disposition, and every feature marked `supported` is
backed by shared executable corpus cases across Python, Rust, and TypeScript.
The corpus must check accepted/rejected decks, normalized output, diagnostics,
and numerical tolerances where the feature executes a circuit. A parser-only
acceptance test is insufficient.

The current register identifies three profiles:

- Berkeley SPICE3f3 is the completed core baseline; v1 remains the published
  compatibility boundary.
- ngspice 47 is the first post-v1 open-source profile.
- LTspice 24.1 is the vendor-workflow profile, including its netlisting,
  stepping, raw-data, and model-library behavior.

The ngspice manual is actively maintained, so its version is intentionally
pinned. LTspice's published technical material and help are likewise versioned
reference inputs rather than moving compatibility assumptions.

## Inventory Rules

Feature IDs are stable, family-scoped names. A feature may be split only when
its syntax, semantics, output, diagnostics, or numerical behavior requires a
separate corpus oracle. The initial registry covers every product surface that
can prevent replacement: deck input, preprocessing, elements and models,
analyses, controls, numerical behavior, output, raw interchange, extensions,
and user workflows.

The entries currently marked `partial`, `planned`, or `out-of-scope` are not
drop-in compatible. In particular, binary raw files, arbitrary `.control`
programs, external code-model APIs, compact-model breadth, and encrypted
vendor libraries remain explicit gaps. `out-of-scope` means the feature needs a
separate product decision before it can be described as supported.

## Execution Order

1. Expand each profile's manual sections into feature-level rows and attach
   reference-section identifiers.
2. Batch corpus work by coherent feature family. The first LTspice stepping
   phase supports one `.step param` card with `LIST` or linear-range values,
   static numeric `.param` substitution, repeated analyses, and per-result
   `stepValues` metadata. It intentionally excludes multiple `.step` cards,
   Cartesian products, `table()` and expression evaluation, temperature/corner
   sweeps, and ngspice `.control` loops. Those are separate register rows so a
   passing simple step deck cannot be mistaken for broad sweep compatibility.
3. Complete Cartesian/table parameter sweeps, then ngspice control-loop
   semantics, before temperature, corner, and Monte Carlo contracts.
4. Add execution and numerical parity cases across all three ports for each
   feature moved to `supported`.
5. Add CLI, interactive, model-library, and UI workflow parity after the
   underlying parser and engine contracts are green.
6. Mark a profile drop-in eligible only after the register and its corpus gates
   are complete; do not infer the claim from a broad test count.
