### Added — Fix B2: native runtime DLL flattening via MSBuild post-build target

The emitted `.csproj` includes a `FlattenNativeRuntimeDlls` target
that copies `Microsoft.WindowsAppRuntime.Bootstrap.dll` from
`runtimes/win-x64/native/` to the output root next to the .exe.
`dotnet build` doesn't do this (only `dotnet publish` does); without
it the unpackaged bootstrap crashes on launch.

