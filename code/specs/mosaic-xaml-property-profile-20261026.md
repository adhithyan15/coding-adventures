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

Specification only. Implementation, executable tests and native receipt pending.
