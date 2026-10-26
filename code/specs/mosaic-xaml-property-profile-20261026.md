# Attribute detailed XAML property application samples

## Evidence and scope

For #16865 and epic #14267. The native keyboard correctness receipt from #18348
contains an interaction-handler interval of 104.6552 ms and next-render interval
of 108.9595 ms. Its encompassing apply-props scope is 102.66 ms; two anonymous
apply-props-set samples are 65.09 and 32.46 ms. The log cannot identify which
generated properties incurred the work. A separate scroll interval is 10.4948 ms
in the handler and 165.2926 ms to the next-render callback, showing that setter
attribution cannot explain every tail.

Add optional generated property metadata to detailed conversion/setter samples
in the shared XAML runtime host. Never infer property identity from reflection
enumeration order, and never log workbook values or runtime JSON keys. This
change enables diagnosis; it does not claim to repair latency or satisfy the
performance acceptance issue.

## Contract

- Keep the public BeginProfile contract and existing phase names compatible.
- Only the reflected component property's metadata name may identify a detailed
  conversion/setter scope. Bound the label length to 128 characters.
- Serialize the optional property label as JSON data, never interpolate JSON.
- Do not add labels to interaction-only records, inspect property values for
  diagnostics, or invoke getters beyond existing semantic operations.
- Disabled profiling and interaction-only filtering happen before scope
  reservation or metadata-label work. Preserve the 4096-scope budget and deferred
  flush; no per-interaction file I/O.
- Preserve conversion, setter execution, equivalent-list reuse, exception
  propagation, notifications and durable snapshot behavior.

## Validation

Execute generated C# tests covering two distinguishable property setters,
conversion attribution, optional-field compatibility, no value leakage,
interaction-only filtering and the existing bounded/disabled profiler contracts.
Run the bindings package suite and security review before publication. Regenerate
the runtime host through its public generator, build the retained native candidate,
and perform an isolated editing/scrolling smoke run. Inspect actual emitted labels
and record source/runtime provenance. Retain #16865 for matched Release/Debug
distributions, broader viewport workloads, concrete repairs and durability.

## Status

Implemented in ce6a05d8df0 after specification commit 569f2d5888a.
Full `cargo test --manifest-path code/packages/rust/mosaic-app-bindings/Cargo.toml`
passed: 44 library tests and six integration tests, including executable C#
conversion/setter failures, distinct Count/AwaitedEffects/Status labels, unchanged
aggregate shape, disabled/filtering and 4096-budget tests. Security review passed.

## Native receipt (2026-10-26)

Generated the host using public `xaml_runtime_binding_for_application` from this
source. Built retained #18348 generated VisiCalc components as Debug x64, zero
errors and two existing CS0067 warnings. This is a host diagnostic candidate,
not a rebuilt full current-main runtime or release artifact.

- Managed DLL SHA256: 69AE11817148996858FA74AA377ECBB7E36F3983142333C4E5674B6446D8BF1B.
- Retained Rust DLL SHA256: A18198012F2985EC5CEA6E10A2151388D2E9093F567A59F46A72E45C28BC2124.
- Raw log: adjacent `mosaic-xaml-property-profile-20261026.jsonl` (353 records),
  SHA256 D8AB7A627D93C65D60BE4A054EDA51246D7F5E5E2DCA72D0F7F2FB8B23603EE4.
- Isolated state/profile paths; detailed profiling enabled, interactions-only off.
- Through native computer input at 1908x1019, focused the formula field, replaced
  A1 with `=20+22`, pressed Enter, observed A1=42 and dependent totals 65/66/196.
  Scrolled downward 600 at (600,600), observed rows 16-34 and onViewportShift.
  Closed normally to flush the buffer. No Save/Open picker acceptance claimed.

The raw records now identify EditContent setters at 13.3411/12.8208 ms,
RowHeaders at 11.0331 ms, and EditorLabel at 5.0498 ms. These include startup
and a tiny smoke workload; they are attribution evidence, not steady-state
percentiles or evidence that earlier anonymous tails involved those properties.
The log contains metadata labels only, not the entered formula or cell values.
Larger matched workloads and a concrete latency repair remain open in #16865.
