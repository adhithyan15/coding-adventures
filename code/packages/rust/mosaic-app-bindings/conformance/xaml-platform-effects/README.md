# XAML platform library conformance (UI87 §7.6)

This console harness drives the XAML platform library
(`MosaicPlatformEffects.cs`: `files.open` / `files.save` and the router that
sends each effect to the app's handler or to the library by kind) with a fake
host and a fake picker, so the real open/save logic, limits and routing run
without a display, WinUI, or a Rust runtime. It checks the same cases as the
Compose library's `MosaicPlatformEffectsTest.kt` and the SwiftUI harness's
`PlatformEffectsChecks.swift`:

- routing: a claimed kind to the app (even a standard one), an unclaimed
  standard kind to the library, a custom kind to the app when it claimed
  nothing and to nobody when it did; a second install changes nothing;
- `files.save`: the bytes under the chosen name, the name (never a path)
  returned, replace-in-place with no temporary left behind, cancel, every
  refused name (paths, `:`, dot-files, RLO and other format characters,
  padding, trailing dot, lone surrogates, private use, invisible characters),
  a name that does not match the accepted type, executable extensions when no
  type is named, bad base64, oversized payloads, and -- on Unix -- that a
  replaced file keeps its permission bits (never setuid) and a new one is 0600;
- `files.open`: name, MIME type and bytes, the accept list mapped to
  extensions in order, cancel, a directory or missing file refused, a chosen
  symlink (Unix), exactly 50 MiB accepted and one byte more refused;
- the router's threading: deferred before any picker, answered from the UI
  queue, one operation at a time, a request that outlives its JSON document,
  a notify ignored, a refused deferral leaving it free, and every way the
  work can fail after deferral (a refused or throwing queue, a throwing
  picker, a host that refuses the answer) still ending in an answer or, for
  a closed host, in nothing escaping.

The `.csproj` defines `MOSAIC_HEADLESS_TEST`, which compiles the library
without its WinUI fence (the pickers, the window handle and the
`DispatcherQueue`). `MosaicRuntimeHost.cs` is compiled too, so the adapter the
library installs through is type-checked against the real generated host;
`WindowsColorStub.cs` stands in for the one Windows App SDK type it names.

Run it against a generated project's files, which are not duplicated here:

```sh
cp <generated>/xaml/MosaicPlatformEffects.cs <generated>/xaml/MosaicRuntimeHost.cs .
dotnet run -c Release --project XamlPlatformEffectsConformance.csproj
```

CI does this in the Windows XAML lane, against TaskApp's generated project,
so the save's move-into-place runs on NTFS -- the file system the real app
writes to.
