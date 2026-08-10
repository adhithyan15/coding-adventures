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
