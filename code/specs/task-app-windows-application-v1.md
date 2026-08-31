# TaskApp portable Windows application v1

Issue: [#13613](https://github.com/adhithyan15/coding-adventures/issues/13613)

## Decision

The product-scoped release lane publishes the strict XAML build as a directly
runnable Windows 10 x64 folder. The ZIP is a **portable application**, not an
installer or MSIX. It carries self-contained .NET and Windows App SDK runtimes so
the user extracts the whole folder and launches `Trestle.exe` without separately
installing either framework.

The generated XAML project remains a separate source artifact. Code signing,
MSIX identity, Store distribution, and an update channel require future
credentials and release engineering and are not implied here.

## Stable release metadata

The publish build sets:

- product name and visible release window title `Trestle`;
- the exact TaskApp SemVer as the assembly product version;
- numeric SemVer core as assembly/file version;
- a generated multi-resolution Trestle application icon;
- release metadata identity `org.codingadventures.trestle`; and
- Mosaic application/persistence identity `task-app`.

Because this is unpackaged, `org.codingadventures.trestle` is release provenance,
not a claimed Windows package identity. The stable live state path is
`%LOCALAPPDATA%\task-app\mosaic-state.v1.json`.

Upgrade, backup, restore, uninstall/purge, and corrupt-state recovery are pinned
by the shared [local-data operations contract](task-app-local-data-operations-v1.md)
and shipped offline as `LOCAL-DATA.txt` at the portable bundle root.

## Payload contract

`task-app-xaml-windows-bundle-v<VERSION>.zip` has one
`Trestle-windows-x64-v<VERSION>` root. The self-contained `dotnet publish` output
is kept intact, with its entry apphost exposed as `Trestle.exe`. The selected Rust
library is installed beside it under the standard binding name `mosaic_app.dll`.
The emitted project explicitly carries `Trestle.pri` plus the generated XBF files
from the build output into `PublishDir`; WinUI needs those application resources
at startup even though the stock publish target omits them.

The root also contains:

- `Trestle.ico` — the same stable multi-resolution release mark embedded in the
  executable;
- `SOURCE_COMMIT` — exact source provenance;
- `BUNDLE.json` — identity, version, runtime, state, signing, and MSIX claims; and
- `INSTALL.txt` — extraction, launch, system, and SmartScreen guidance.

The archive builder rejects paths escaping the publish root, nonstandard
executable/runtime names, and a runtime whose bytes differ from the selected Rust
release artifact.

## Verification

The Windows release job must:

1. build `task-mosaic-app` and generate XAML under `native-complete`;
2. apply release-only Trestle title, product, version, and icon metadata;
3. publish `win-x64` with both .NET and Windows App SDK self-contained;
4. require `Trestle.pri` and every generated application XBF in the publish
   directory;
5. install and compare `mosaic_app.dll` byte-for-byte with
   `task_mosaic_app.dll`;
6. archive and extract independent original/replacement directories;
7. use UI Automation to drive the real todo and Rust scheduling lifecycle from
   the first executable, restart through the replacement executable, and restore
   the standard LocalApplicationData snapshot without `MOSAIC_APP_LIBRARY`; and
8. retain hosted-runner console conformance against the replacement runtime.

Before that lifecycle, the gate seeds the committed v0.1.0 fixture at the normal
LocalApplicationData path and requires the extracted executable to accept it
without relocation or quarantine. It separately launches against invalid bytes
and requires byte-preserving `.corrupt` quarantine.

The hosted Windows UI Automation gate does not require a foreground desktop, but
it launches the real WinUI executable and inspects native controls. This is a
stronger claim than a compile-only or ABI-only package check while remaining
honest about absent signing and MSIX installation.
