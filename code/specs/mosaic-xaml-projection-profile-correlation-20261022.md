# XAML projection profiling correlation

## Scope

For #16865, correlate generated XAML interaction-handler and interaction-next-render JSONL samples with the component's cumulative MosaicProjectionCounts snapshot. Pass the component to the shared runtime profiler; components without this optional diagnostic property retain the original three-field record. No generated output is edited.

## Contract

BeginProfile accepts an optional component. Only when profiling is enabled and capacity has been reserved, resolve the public, non-indexed diagnostic property with the exact four-long ValueTuple type. Read a start and end snapshot on the calling UI thread. Export per-scope projection deltas (reads, created, reused, removed) under a projection object. Missing, throwing, or malformed diagnostics omit the object and must not interrupt app execution. Keep the 4096 reservation bound and idempotent disposal. No workbook values, component names or formulas are logged. Disk export remains at normal process exit.

Overlapping frame scopes can include the same work; these are per-interval deltas, not additive event totals or native realization counts. Existing timing/allocation fields remain; reading diagnostics adds profiling overhead. This does not establish native performance acceptance.

## Validation

Execute C# probes for enabled deltas, absent diagnostics, throwing diagnostics, disabled getter avoidance, idempotence and bounded pending scopes. Check emitted host calls pass the component. Run bindings acceptance and emitter tests, then independent security review before publication. Native warm workload measurement follows separately.
