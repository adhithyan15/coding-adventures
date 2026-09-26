### Fixed — HostDialog now actually renders on WinUI 3

Discovered while making the first end-to-end Mosaic → XAML → on-screen
dialog demo run (see `code/programs/csharp/hello-dialog-xaml/ISSUES.md`). Five
generator bugs that each blocked the dialog from displaying.

**A1 — HostDialog at the moslayout root now hoists to a
`<ContentDialog>` XAML root.** Previously the emitter always wrote a
`<UserControl>` root containing a `<ContentDialog>`. WinUI 3's
`ContentDialog` is a top-layer popup that can't be embedded as a
UserControl child and then shown via `ShowAsync()` — the parented
child can't be re-parented. The crash ranged from
`ArgumentException` to a native `0xc000027b` in `CoreMessagingXP.dll`.
The fix introduces `RootShape` ({`UserControl`, `ContentDialog`}),
picks it based on the moslayout root (`HostDialog` → ContentDialog,
everything else → UserControl), and propagates it to:
  - `emit_xaml` (the XAML root tag + closing tag)
  - `emit_code_behind` (the partial class's `: BaseClass`)
  - `emit_host_dialog_as_root` (a new path that writes the dialog's
    attributes onto the outer ContentDialog and its children
    directly — no inner wrapping)

`modal: false` at the moslayout root still uses `<ContentDialog>` (a
Flyout cannot be a XAML root either). Nested HostDialog `modal: false`
still produces `<Flyout>` — see `nested_host_dialog_modal_false_uses_flyout`.

**A2 — Dropped the `mos:Dialog.IsOpen` attribute entirely.** The
attribute was emitted but the `mos:` xmlns was never declared, so
XAML loading failed at runtime with the opaque "could not be started"
dialog. Per the existing documented contract ("host code-behind owns
the lifecycle"), the comment stub is sufficient — authors get a clear
`<!-- HostDialog #N open-state: bind 'Show'; host code-behind watches
this DP and calls ShowAsync()/Hide() accordingly. -->` in the
generated XAML.

**A3 — `Title=` binding now uses `{x:Bind ..., Mode=OneWay}` instead
of `{Binding ...}`.** Every other emitter in the crate uses
`{x:Bind}` because the generator never sets DataContext. The
HostDialog emitter regressed to `{Binding}`, which silently failed
(empty Title). Fixed to match the rest of the crate.

**A4 — Slot DPs whose PascalCased name collides with a property on
the chosen base class are renamed to `<BaseName>{Slot}`.** A `slot
title : text` on a ContentDialog-rooted component now generates a DP
named `DialogTitle` (avoiding shadowing `ContentDialog.Title`). The
`{x:Bind}` paths in the generated XAML route through
`EmitContext::slot_xbind_path` to use the alias. The set of
collidable inherited properties lives on `RootShape::inherited_properties()`
and currently lists `Title`, `PrimaryButtonText`, `SecondaryButtonText`,
`CloseButtonText`, plus three IsXxxEnabled / DefaultButton — expand
as new collisions emerge.

**A5 — `BoolToVisibilityConverter.cs` is now auto-emitted alongside
the component triple whenever `ctx.needs_bool_to_vis` is set.** The
3-line `IValueConverter` implementation supports
`ConverterParameter="invert"` (matches the `If`/`Else` lowering in
§6.2). Lands as an `EmittedFile` in `XamlEmitResult::if_helpers` —
the field PR-2 left empty per its deviation note. Helpers from PR-2
(method-style) continue to inline into the code-behind; the
converter is a separate type and ships as a sibling file.

