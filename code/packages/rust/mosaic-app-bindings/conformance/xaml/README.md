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
props, and that an invalid report is refused and the identical one held back.
With the conformance app's `failEnvironment` switch on, every report that
reaches the app fails, so a failure status proves a report was sent and null
that it was held back: the refused report and an unchanged one are not sent, a
changed one is, and one that failed for the app's own reasons is sent again
rather than held back. With `MOSAIC_APP_STATE_PATH` set it also checks that an
ignored report does not rewrite the state file, and -- with the state path made
a directory -- raises no persistence warning while an event that cannot persist
surfaces one at once, and that once the path is writable again the next
ignored report retries the failed save and reports the warning cleared.
