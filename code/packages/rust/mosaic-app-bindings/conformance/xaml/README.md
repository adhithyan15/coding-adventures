# XAML runtime conformance

This console harness compiles the exact `MosaicRuntimeHost.cs` emitted into a
generated WinUI project, loads `mosaic-app-conformance.dll`, applies initial
props to a typed C# object, dispatches an event, and verifies the revised props.

CI copies the generated binding into this directory in a temporary workspace;
the binding itself is not duplicated here.
