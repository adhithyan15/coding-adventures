# Issues found making the Mosaic → XAML → on-screen dialog actually work

> **All eleven issues are now closed.** This file remains as the
> historical chronicle of the first time we made Mosaic → XAML run
> end-to-end. The summary table at the bottom links each entry to the
> PR that closed it.

This document chronicles every concrete obstacle hit while taking the
generated XAML output and turning it into a running Windows app that
displays the authored dialog. Each issue is logged with: what we saw,
what's actually wrong, where to fix it, and the workaround we used in
the meantime.

The original intent was that each item here would become a follow-up
PR. They all landed across `#3910` (the A1–A5 generator fixes) and
`#3917` (the B1–B3 project-shell + build infrastructure). The C-series
items are environmental (missing VS Build Tools, runtime install
requirement) and remain documented in lessons.md + the emitted
README rather than fixed in code.

The screenshot proving the end-state works is
[`dialog-rendered.png`](./dialog-rendered.png) — the "Hello
from Mosaic" dialog rendering inside the WinUI 3 host window, with the
Close button and the bound Title and Message. Getting there required
hand-patches in five places and one missing piece of infrastructure.

---

## A — Generator bugs (mosaic-emit-xaml)

### A1. `HostDialog` lowered inside `<UserControl>` (root issue)

**What we saw.** The first `await dialog.ShowAsync()` call threw
`ArgumentException: Value does not fall within the expected range.`
even with a correctly populated `XamlRoot`. With slight changes the
crash dropped into `CoreMessagingXP.dll` with exit code `0xc000027b`
(STATUS_RUNTIME_RAISED_EXCEPTION).

**Why.** Looking at `mosaic-emit-xaml`'s `emit_xaml`:

```rust
writeln!(out, "<UserControl").unwrap();
writeln!(out, "    x:Class=\"{ns}.{name}\"").unwrap();
// ...
let body = emit_xaml_node(root, 4, part_styles, ctx)?;
out.push_str(&body);
// ...
writeln!(out, "</UserControl>").unwrap();
```

The XAML root is *always* `<UserControl>`. When the moslayout root is
`HostDialog`, this produces:

```xml
<UserControl ...>
  <ContentDialog ...> ... </ContentDialog>
</UserControl>
```

That isn't a valid WinUI 3 pattern. `ContentDialog` is a top-layer
primitive that becomes the visual root when shown; embedding it inside
a `UserControl` means it's "already parented", and the dialog presenter
can't re-parent it for the modal popup. WinUI 3 doesn't surface a
clean error — the failure modes range from `ArgumentException` to a
native heap-corruption crash, depending on timing.

**Where to fix.**
`code/packages/rust/mosaic-emit-xaml/src/pipeline.rs::emit_xaml`.

The root-element decision needs to inspect the moslayout root:
- If the layout root is `HostDialog`, emit a `<ContentDialog>` root and
  make the partial class `: ContentDialog`.
- Otherwise (the common case), emit the existing `<UserControl>` root
  and `: UserControl`.

Spec impact: `code/specs/mosaic-emit-xaml.md` §4 (the Host* primitives)
should document this — HostDialog is the one primitive where the
XAML root *follows the moslayout root*, not the convention.

**Workaround used.**
Hand-patched [`winui/HelloDialog.xaml`](./winui/HelloDialog.xaml) and
[`winui/HelloDialog.xaml.cs`](./winui/HelloDialog.xaml.cs) to make
`<ContentDialog>` the root and the partial class extend
`ContentDialog`.

---

### A2. `mos:Dialog.IsOpen` uses an undeclared namespace

**What we saw.** The XAML compiler accepted the source but the runtime
loader couldn't resolve `mos:` — early launches showed a XAML parse
failure ahead of any other error.

**Why.** `emit_host_dialog` writes:

```rust
attrs.push_str(&format!(" mos:Dialog.IsOpen=\"{{Binding {pascal}}}\""));
```

…but `emit_xaml`'s `<UserControl>` header only declares `local:` and
`x:`. The `mos:` prefix has no `xmlns` definition.

The intent was for the host project to define a `Dialog` static class
with an `IsOpen` attached property. Even with that infrastructure in
place, the XAML can't compile because no `xmlns:mos="..."` was emitted.

**Where to fix.**
`code/packages/rust/mosaic-emit-xaml/src/pipeline.rs::emit_host_dialog`
should EITHER:
- (a) Declare `xmlns:mos="using:Mosaic.Generated"` on the
  `<UserControl>` (similar to PR-5's `used_xmlns` mechanism for
  component references) AND emit a `Dialog` attached-property class
  alongside the code-behind. OR
- (b) Drop the `mos:Dialog.IsOpen` attribute entirely. The existing
  documented contract is that the *host code-behind* calls
  `ShowAsync()`/`Hide()` when the slot flips. The attached-property
  is a redundant convenience.

Recommend (b) — it removes a runtime moving part and keeps the spec's
"host owns the lifecycle" contract clean.

**Workaround used.**
Removed the `mos:Dialog.IsOpen` attribute from
[`winui/HelloDialog.xaml`](./winui/HelloDialog.xaml).

---

### A3. Inconsistent binding form: `{Binding}` vs `{x:Bind}`

**What we saw.** The dialog's `Title` rendered as empty even though we
set the `Title` DP at construction; the rest of the slot bindings
worked.

**Why.** `emit_host_dialog` emits:

```rust
attrs.push_str(&format!(" Title=\"{{Binding {pascal}}}\""));
```

…but every other slot in the emitter uses `{x:Bind ...}`:

```rust
attrs.push_str(&format!(" Text=\"{{x:Bind {pascal}}}\""));
```

`{Binding}` resolves via the element's `DataContext`. The emitter never
sets `DataContext`, so the Title binding silently fails. `{x:Bind}`
resolves at compile time against the partial class itself — that's
what the rest of the generated code relies on.

**Where to fix.**
`code/packages/rust/mosaic-emit-xaml/src/pipeline.rs::emit_host_dialog`:
change the two `{Binding ...}` emissions to `{x:Bind ..., Mode=OneWay}`.

Spec impact: `code/specs/mosaic-emit-xaml.md` §6.3 (the ExprLowerer
documentation) already documents `{x:Bind}` as the standard form; this
is just an internal inconsistency to clean up.

**Workaround used.**
Replaced `{Binding Title}` with `{x:Bind DialogTitle, Mode=OneWay}` in
the hand-patched XAML.

---

### A4. Slot named `title` collides with `ContentDialog.Title`

**What we saw.** Once the XAML root became `<ContentDialog>` (per A1),
declaring a `Title` DP on the partial class shadowed
`ContentDialog.Title` and the `{x:Bind Title}` binding became
ambiguous.

**Why.** `ContentDialog` already has a public `Title` property (the
dialog heading). The author's `slot title : text` produces a DP also
named `Title` on the same class.

**Where to fix.**
`code/packages/rust/mosaic-emit-xaml/src/pipeline.rs::emit_dependency_property`.
When the partial class extends `ContentDialog`, a slot named `title`
should generate a DP named `DialogTitle` (or some other safe alias) and
the XAML's ContentDialog `Title=...` binding should reference the
aliased name.

Generalised: whenever a slot's PascalCased name collides with a
property already on the chosen base class, rename to `<BaseName>{Slot}`
(e.g. `DialogTitle`, `WindowTitle`).

**Workaround used.**
Renamed the DP to `DialogTitle` in the hand-patched
[`winui/HelloDialog.xaml.cs`](./winui/HelloDialog.xaml.cs).

---

### A5. `BoolToVisibilityConverter` referenced but never emitted

**What we saw.** Not hit in this minimal demo (no `If` blocks), but
`emit_if` writes:

```xml
<ContentControl Visibility="{x:Bind X, Converter={StaticResource BoolToVisibilityConverter}}">
```

Without a corresponding C# class, any host using `If` lowering would
fail at XAML parse time.

**Why.** The converter has to exist as a `IValueConverter`
implementation in C#. The emitter assumes the host ships it.

**Where to fix.**
`code/packages/rust/mosaic-emit-xaml/src/pipeline.rs`: when
`ctx.needs_bool_to_vis` is set, emit a
`BoolToVisibilityConverter.cs` file alongside the per-component
triple. Three lines of C#:

```csharp
public sealed class BoolToVisibilityConverter : IValueConverter
{
    public object Convert(object v, Type t, object p, string l)
        => (v is bool b && (p?.ToString() == "invert" ? !b : b))
            ? Visibility.Visible : Visibility.Collapsed;
    public object ConvertBack(object v, Type t, object p, string l)
        => throw new NotImplementedException();
}
```

Spec impact: `code/specs/mosaic-emit-xaml.md` §6.2 mentions the
converter; add a note that the generator emits it automatically.

**Workaround used.**
Not hit in this demo. The fix is mandatory before any consumer uses
`If`.

---

## B — Generator missing infrastructure

### B1. No `--emit-project` flag implementation

**What we saw.** The spec describes `--emit-project` (`mosaic-emit-xaml.md`
§10) as producing a full WinUI 3 project: `.csproj`, `App.xaml(.cs)`,
`MainWindow.xaml(.cs)`, `app.manifest`. The flag exists on
`EmitOptions::emit_project` but is never read. PR-1's CHANGELOG even
notes "PR-1 ignores this flag — the project triple lands in PR-5",
but PR-5 went out without implementing it.

**Why.** The implementation got deferred. The pieces exist conceptually
but no code emits them.

**Where to fix.**
`code/packages/rust/mosaic-emit-xaml/src/pipeline.rs`: implement the
project emission. Roughly, when `options.emit_project` is true:

1. Populate `XamlEmitResult::project` (the field exists; it's `None`
   today).
2. Generate `.csproj` with the right WindowsAppSDK PackageReference,
   `<UseWinUI>`, `<WindowsPackageType>None</WindowsPackageType>`,
   `<WindowsAppSDKSelfContained>true</WindowsAppSDKSelfContained>`,
   and `RuntimeIdentifier=win-x64`.
3. Generate `App.xaml` + `App.xaml.cs`: standard WinUI 3 application
   shell that instantiates `MainWindow`.
4. Generate `MainWindow.xaml` + `MainWindow.xaml.cs`: a default host
   that instantiates the component, sets sensible DP values, and
   wires the `Dispatch` event to a stub event handler the user fills
   in. For HostDialog components, the MainWindow should automatically
   call `ShowAsync()` on the dialog after Activated.
5. Generate `app.manifest` with DPI awareness + supported-OS GUID.

CLI integration: `mosaic-compile --backend xaml --emit-project` should
write all of the above plus the existing component triple under a
single output directory.

**Workaround used.**
Hand-wrote everything in [`winui/`](./winui/).

---

### B2. Native runtime DLLs not auto-staged into the build output

**What we saw.** Even after a successful `dotnet build`,
`Microsoft.WindowsAppRuntime.Bootstrap.dll` (the native loader) sits
in `bin/.../runtimes/win-x64/native/` rather than next to the .exe.
Running the .exe without that copy yields the "could not be started"
runtime-missing dialog.

**Why.** `dotnet build` (as opposed to `dotnet publish`) doesn't
flatten the runtime-specific folders. With a `RuntimeIdentifier` set,
publish does the flatten; build doesn't.

**Where to fix.**
Two paths:
- (a) The emitted `.csproj` should add a post-build MSBuild target
  that copies `runtimes/win-x64/native/*.dll` next to the .exe.
- (b) The emitted build script (PR B3) should do the copy as a final
  step.

(a) is more portable. Either works.

**Workaround used.**
Manual `cp runtimes/win-x64/native/*.dll bin/Debug/.../` per build.

---

### B3. No build driver script

**What we saw.** There's no documented `build.ps1` or `build.cmd` that
glues `mosaic-compile --backend xaml` + `dotnet build` + the runtime
DLL copy into one command.

**Where to fix.**
The `--emit-project` flag (B1) should also emit a
`build.ps1` that:

1. Runs `mosaic-compile` over each `.mil/.mll/.msl` triple under
   `mosaic/` to produce/refresh the per-component triples under
   `winui/`.
2. Runs `dotnet build winui/<Project>.csproj -c Debug`.
3. Copies the native runtime DLLs (per B2(a)/(b)).
4. Optionally runs the .exe.

A `README.md` should accompany describing prerequisites (Windows App
Runtime install) and the build/run/clean commands.

---

## C — WinUI 3 SDK / MSBuild environment issues (not generator bugs but real consumer pain)

### C1. `MrtCore.PriGen.targets` requires VS-shipped MSBuild tasks

**What we saw.** Every `dotnet build` after the actual C# / XAML
compile step ends with:

```
MSB4062: The "Microsoft.Build.AppxPackage.RemovePayloadDuplicates"
task could not be loaded from the assembly
.../AppxPackage/Microsoft.Build.AppxPackage.dll. Could not load file
or assembly ... The system cannot find the path specified.
```

**Why.** WindowsAppSDK's `MrtCore.PriGen.targets` references MSBuild
tasks that ship only with Visual Studio's `AppxPackage` tooling. The
`dotnet` SDK alone doesn't include them. The packaging target
(`CopyLocalFilesOutputGroup` and its `RemovePayloadDuplicates` call)
runs even for `<WindowsPackageType>None</WindowsPackageType>` builds.

The .dll and .exe DO build correctly before the packaging target
fails, so the failure is effectively cosmetic for the unpackaged path.

**Where to fix.** Out of scope for this repo — this is a
WindowsAppSDK + .NET SDK story. Two mitigations:
- The emitted `.csproj` (B1) should override the offending target to a
  no-op. We tried this in the demo's `.csproj` but the package's
  transitive targets load AFTER our override, so it doesn't take
  effect via the project file alone.
- The README (B3) should document the expected error and that the
  output is functional anyway.

A proper fix is "install Visual Studio Build Tools with the WinUI
workload" — but that's a 6 GB download, so we don't require it.

**Workaround used.**
Ignore the error; copy outputs manually; trust that the .exe works.

---

### C2. Windows App Runtime must be installed system-wide for framework-dependent apps

**What we saw.** First launch of an unpackaged WinUI 3 app on a
machine without the Windows App Runtime shows a system error dialog:

> This application requires the Windows App Runtime
> Version 1.6
> (MSIX package version >= 6000.373.1641.0)

**Why.** The bootstrap DLL (`Microsoft.WindowsAppRuntime.Bootstrap.dll`)
queries the system for an installed runtime. If absent or
version-mismatched, it pops the dialog.

**Where to fix.** Out of scope — this is a Microsoft / WindowsAppSDK
deployment story. Options for the emitted project:
- Document `winget install Microsoft.WindowsAppRuntime.1.7` (or
  whichever the project targets) in the emitted README.
- Set `<WindowsAppSDKSelfContained>true</WindowsAppSDKSelfContained>`
  on the emitted `.csproj` to bundle the runtime, eliminating the
  install requirement. (This is what the demo's csproj does today.)

---

## D — Demo project gaps (host code we had to write by hand)

### D1. ContentDialog programmatic show with explicit XamlRoot

**What we saw.** Even with the right ContentDialog-as-root pattern, the
host's `await dlg.ShowAsync()` throws `ArgumentException` if the
ContentDialog's `XamlRoot` isn't set.

**Why.** WinUI 3 requires an explicit `XamlRoot` for a programmatically
created ContentDialog (it can't infer one from the calling thread).

**Where to fix.** The emitted MainWindow boilerplate (per B1) should
set `dlg.XamlRoot = SomeKnownElement.XamlRoot` before showing.

**Workaround used.**
The button's own `XamlRoot` (guaranteed valid at click time) is
assigned in
[`winui/MainWindow.xaml.cs`](./winui/MainWindow.xaml.cs).

---

# Summary table — all issues closed

| # | Layer | Status | Where fixed |
|---|---|---|---|
| A1 | Generator: HostDialog lowering | ✅ resolved | #3910 — `RootShape` enum + `emit_host_dialog_as_root` |
| A2 | Generator: undeclared `mos:` namespace | ✅ resolved | #3910 — dropped `mos:Dialog.IsOpen`, comment-stub only |
| A3 | Generator: `{Binding}` vs `{x:Bind}` | ✅ resolved | #3910 — `Title=` uses `{x:Bind ..., Mode=OneWay}` |
| A4 | Generator: `Title` slot collision | ✅ resolved | #3910 — `slot_aliases` + `RootShape::inherited_properties` |
| A5 | Generator: missing `BoolToVisibilityConverter.cs` | ✅ resolved | #3910 — auto-emitted into `XamlEmitResult::if_helpers` |
| B1 | Generator: missing `--emit-project` | ✅ resolved | #3917 — `build_project_files()` + CLI wiring |
| B2 | Generator: native DLLs not flattened | ✅ resolved | #3917 — `FlattenNativeRuntimeDlls` MSBuild target |
| B3 | Generator: no build driver script | ✅ resolved | #3917 — `build.ps1` template + README |
| C1 | WinUI SDK: missing AppxPackage MSBuild tasks | Doc-only | lessons.md + emitted README + `<AppxGeneratePriEnabled>false</AppxGeneratePriEnabled>` mitigation in the csproj |
| C2 | WinUI runtime: system-wide install required | Doc-only | `winget install Microsoft.WindowsAppRuntime.1.7` in emitted README |
| D1 | Host code: explicit XamlRoot on ContentDialog | ✅ resolved | #3917 — MainWindow template uses `(sender as FrameworkElement)?.XamlRoot` |

Generator output now matches the hand-patched reference: regenerating
this directory's `winui/` produces a project that builds, runs, and
displays the dialog with **only the host's business logic remaining
for the user to fill in.**

# Regenerating

Run from the repo root (no longer needs hand-patches):

```powershell
$compiler = ".\code\packages\rust\target\release\mosaic-compile.exe"
& $compiler `
    --interface demo\hello-dialog-xaml\mosaic\HelloDialog.mil `
    --layout    demo\hello-dialog-xaml\mosaic\HelloDialog.mll `
    --style     demo\hello-dialog-xaml\mosaic\HelloDialog.dark.msl `
    --backend   xaml `
    --emit-project `
    -o          demo\hello-dialog-xaml\winui\HelloDialog
```

All 11 files in `winui/` are produced by that single invocation.
