# XAML interaction-only profiling

## Problem and scope

Native VisiCalc smoke evidence for #16865 records about70 internal profile records per interaction. The existing global4096 reservation cap can truncate collection before100 warm interactions. Raising the cap would increase retained memory and pending render subscriptions. This change instead adds a reusable host option, not an app or generated-output patch.

## Contract

With MOSAIC_PROFILE_PATH enabled, MOSAIC_PROFILE_INTERACTIONS_ONLY=1 selects only the exact ordinal phases interaction-handler and interaction-next-render. Absent or any other value preserves existing full-phase profiling. The setting is read once at process initialization. Filtering happens before reserving capacity, constructing a scope, or reading component diagnostics. The existing shared4096 cap still bounds admitted pending and completed scopes; disposal remains idempotent and normal process exit remains the only export path.

The JSON schema is unchanged. No configuration text or application data is added to output. Projection counters still describe overlapping intervals, not native realization. This option enables longer workloads; it does not establish performance acceptance or change app behavior.

## Validation

Execute the emitted C# host in the existing native-driver acceptance harness. More than4096 excluded requests must consume no reservations and read no projection getter. Then admit at least100 paired interaction scopes, verify both phase names in exported JSON, and verify the same4096 pending-scope cap. Existing disabled/full-phase and failed-export tests remain passing. Document that filtered measurements omit internal dispatch/persistence breakdowns and cannot be compared as equivalent instrumentation to full-phase baselines.
