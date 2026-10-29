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
2. Batch corpus work by coherent feature family. The LTspice stepping phase
   supports `LIST` and linear-range `.step param` cards, source-order Cartesian
   products across multiple cards, static numeric `.param` substitution, and
   exact numeric `table(selector, key, value, ...)` mappings driven by a
   stepped selector. Repeated analyses and all CLI artifacts retain the full
   per-run `stepValues` map. Table interpolation, file/string tables,
   logarithmic stepping, and general expression evaluation remain separate
   compatibility work, as do temperature/corner sweeps and ngspice `.control`
   loops. A passing parameter-table deck must not be mistaken for full LTspice
   expression compatibility.
3. The bounded ngspice `.control` parameter-run form accepts numeric `foreach`
   lists that feed one top-level `.param` through `alterparam`, `reset`, and the
   existing `run` command. It emits the same source-ordered `stepValues` maps as
   deck-owned parameter stepping. `while`, element `alter`, nested loops,
   expressions, scripting, and interactive commands remain unsupported.
4. Complete temperature, corner, and Monte Carlo contracts before broadening the
   LTspice table/expression row.
5. Add execution and numerical parity cases across all three ports for each
   feature moved to `supported`.
5. Add CLI, interactive, model-library, and UI workflow parity after the
   underlying parser and engine contracts are green.
6. Mark a profile drop-in eligible only after the register and its corpus gates
   are complete; do not infer the claim from a broad test count.
