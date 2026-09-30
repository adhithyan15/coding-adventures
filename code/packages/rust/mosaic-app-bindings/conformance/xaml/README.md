# XAML runtime conformance

This console harness compiles the exact `MosaicRuntimeHost.cs` emitted into a
generated WinUI project, loads `mosaic-app-conformance.dll`, applies initial
props to a typed C# object, dispatches an event, and verifies the revised props.

CI copies the generated binding into this directory in a temporary workspace;
the binding itself is not duplicated here.

The preceding CI step compiles that binding against the real Windows App SDK as
part of the complete TaskApp. This console harness uses a source-compatible
`Windows.UI.Color` value stub for the binding's optional color projection so the
runtime-only executable cannot bootstrap WinUI on GitHub's non-interactive
Windows worker. It still executes the unchanged generated binding and production
Rust C ABI, and the CI step has a ten-minute hard timeout.

That lane also relaunches against persisted and incompatible state, covering
restore-before-render, atomic dispatch persistence, and recoverable quarantine.

It also checks UI48 ENV4 (environment reporting, spec §7.7) against the
conformance runtime, which ignores `environmentChanged`: the six reported
values and thresholds, that an ignored report re-applies nothing and keeps the
props, that an invalid report is refused and not remembered, and -- with
`MOSAIC_APP_STATE_PATH` set, since every dispatch rewrites the state file --
that an unchanged report is not sent and a changed one is.
