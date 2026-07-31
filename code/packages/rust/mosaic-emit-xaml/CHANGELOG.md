# Changelog — mosaic-emit-xaml

## [Unreleased] — VC2-xaml Grid: WinUI value translation + nested-For + per-column widths

### Added - Native activation for MSL pressed states

WinUI output now connects UI15's built-in `state pressed` blocks on
`HostButton`, `HostCheckbox`, `HostRadio`, and `HostLink` directly to
`ButtonBase.IsPressed`. DataTemplate instances remain row-local, pressed takes
precedence over simultaneous focused or hover states, and explicit
`state-when-pressed` predicates remain author-controlled. A Task App
acceptance gate proves its Mosaic-authored add-task button feedback reaches
generated XAML without handwritten Win32 UI.

### Added - Native activation for MSL focused states

WinUI output now connects UI15's built-in `state focused` blocks on native
focus-capable Host controls to `Control.FocusState` through a generated
`IValueConverter`. Pointer, keyboard, and programmatic focus activate the
shared MSL properties and transitions; DataTemplate instances remain
row-local, and explicit `state-when-focused` predicates remain
author-controlled. A Task App acceptance gate proves its Mosaic-authored
project-composer focus ring reaches the generated TextBox without handwritten
Windows UI.

### Added - Native activation for MSL hover states

WinUI output now activates UI15's built-in `state hover` blocks on Mosaic
controls that lower to the native ButtonBase family: `HostButton`,
`HostCheckbox`, `HostRadio`, and `HostLink`. The generated `StateTrigger`
binds directly to the control's native `IsPointerOver` dependency property.
Bindings inside a `For` remain in the DataTemplate namescope, so each repeated
row owns independent pointer state. Existing explicit `state-when-hover`
predicates remain author-controlled and do not install pointer tracking.

### Added - Native MSL states and transitions for Host controls

`HostInput`, `HostButton`, `HostCheckbox`, `HostRadio`, `HostLink`, and
`HostNumberInput` now consume structured MSL state and transition IR.
Top-level `state-when-*` predicates become one-way WinUI `StateTrigger`
bindings, state properties become `VisualState` setters, and MSL durations
and easing curves become native `VisualTransition` values.

Each transitioned property is emitted in a separate `VisualStateGroup`.
This preserves MSL's property-scoped motion contract instead of letting one
transition duration animate every property changed by a state. Part-level
transitions apply in both directions, while a state-local transition
overrides the curve on entry. Multiple active states retain React/SwiftUI
precedence: the last `state-when-*` declaration wins. Stateful components use
a transparent first-child `Grid`, which is the placement WinUI requires for
automatic `StateTrigger` evaluation.

Supported easing lowerings are `linear`, `ease`, `ease-in`, `ease-out`, and
`ease-in-out`. WinUI XAML has no arbitrary cubic-bezier
`EasingFunctionBase`, so `cubic-bezier(...)` currently uses the closest
native `CubicEase` curve; an exact Composition-API lowering remains a
follow-up. Template-local Host controls inside `For` keep their VisualStates
inside the DataTemplate namescope so their triggers and targets remain
row-local.

### Added - XAML host intent extension point

Generated WinUI project shells now preserve structured `HostIntent` values from
optional `MosaicHost.HandleEvent` results and can delegate them to an
app-provided asynchronous `MosaicHost.HandleHostIntent(Window, Component,
HostIntent)` method. This lets app packages implement native file pickers or
other platform-owned workflows without hand-patching generated `MainWindow`
code.

### Changed - XAML host build script reliability

Generated `build.ps1` drivers now resolve `dotnet` from PATH or the standard
`Program Files\dotnet\dotnet.exe` location before building, and fail with a
non-zero exit code when the tool is unavailable. The `-Run` path also reports a
missing executable or non-zero app exit instead of leaving the script looking
green after a failed launch.

The nested Windows Rust workspace config now uses the Rust-bundled `rust-lld`
linker, matching the repo root and avoiding accidental resolution of Git/MSYS
`link.exe` without requiring Visual Studio's `lld-link.exe` in local dev shells.

### Added - Mosaic event envelopes for WinUI hosts

Generated non-empty `{Component}.Event.cs` unions now expose `MosaicName`,
`MosaicPayload`, and `MosaicEnvelope` on the base event record, with each nested
record preserving its original Mosaic emit name and payload keys. WinUI hosts
can use the envelope as the JSON-shaped event bridge into shared business logic.

The VisiCalc `Grid` (from `mosaic-pkg-grid`, lowered through
`HostTable` + nested `For` + `Cell`) regenerated into XAML that the
WinUI 3 markup compiler would reject and that would block
`dotnet build`. Four groups of fixes make it valid and
spreadsheet-correct. The demo `code/programs/csharp/visicalc-xaml/` is rewired to
mount the generated `<gen:Grid>` instead of its hand-written
placeholder.

> Verified on macOS via `cargo test -p mosaic-emit-xaml --lib`
> (164 passing) + structural inspection of the generated XAML/C#.
> Runtime / `dotnet build` verification needs Windows.

### Group A — WinUI value translation (X5)

`build_style_fragment` gained a value-translation layer
(`translate_xaml_value`) below the X1 name-mapping and X4 color
PascalCasing. `css_property_to_xaml_setter` now returns
`Option<String>` so CSS-only properties can be dropped.

- **px-strip** — length setters (`FontSize`, `Height`, `Width`,
  `Padding`, `Margin`, `BorderThickness`, `CornerRadius`) emit bare
  numbers / `Thickness`: `12px`→`12`, `0,0,0,1px`→`0,0,0,1`.
- **drop CSS-only props** — `border-collapse`, `border-style`,
  `outline`, `text-decoration`, `box-shadow` return `None` (omitted,
  not emitted as invalid attrs / `<Setter>`s).
- **drop `Width="100%"`** — WinUI `Width` is a `Double`, not a
  percentage.
- **`text-align` → `TextAlignment`** with a PascalCase value
  (`center`→`Center`, `right`→`Right`, `left`→`Left`). The old output
  emitted `<Setter Property="TextAlign" Value="center"/>` — invalid
  on both the property name and the value.
- **`font-weight`** → WinUI `FontWeights` constant
  (`normal`→`Normal`, `bold`→`Bold`, `600`/`semibold`→`SemiBold`,
  `500`/`medium`→`Medium`).
- `{x:Bind …}` markup-extension values pass through unmangled (never
  px-stripped or case-mangled).

Tests: `x5_px_units_stripped_from_length_setters`,
`x5_css_only_properties_are_dropped`,
`x5_percentage_width_is_dropped`,
`x5_text_align_maps_to_textalignment_pascalcase`,
`x5_font_weight_maps_to_named_constant`,
`x5_binding_value_passes_through_unmangled`,
`x5_strip_px_units_preserves_thickness_shape`,
`group_a_cell_style_is_valid_winui`. Updated
`x4_non_color_setters_pass_through_unchanged` and
`box_partitions_style_between_border_and_textblock_resource` which
asserted the old (now-invalid) `FontWeight="normal"` / `"500"`.

### Group B — nested-For inner value type (compile gate)

The inner `For (each: row, as: v)` (UI29 §3.4, `each:` referencing
the outer For's `as:` binding) inferred the cell value type as
`IReadOnlyList<string>` instead of `string`, because `emit_for`'s
Keyword arm used the enclosing binding's `element_type` verbatim
(that is the type of `row` ITSELF). The cell then bound a `string`
`<TextBlock Text="{x:Bind V}"/>` to a list field — a `dotnet build`
blocker. Fixed by peeling exactly one `List<>` level
(`inner_type_of_list(outer_type)`), so the inner value VM
(`Grid_VVm`) types `V` as `string` while the outer `Grid_RowVm`
keeps `IReadOnlyList<string> Row`.

Test: `group_b_inner_value_vm_field_is_string_not_list`.

### Group C — per-column fixed widths

The per-column cell loop's value VM (`Grid_VVm`) now carries a
`double Width` field, and the generated cell element binds
`Width="{x:Bind Width}"` (injected via
`inject_attr_into_first_element`). The host-side VM-builder that
POPULATES the width (zipping cell value + column index → width) is
host code the emitter doesn't generate — a `<remarks>` doc comment
in the generated value-VM `.cs` tells the Windows dev exactly how
(`new Grid_VVm(value, col, ColumnWidths[col])`).

Tests: `group_c_value_vm_carries_width_and_cell_binds_it`.

### Group D — demo rewired (no hand-written placeholder)

`code/programs/csharp/visicalc-xaml/`: `scripts/build.sh` now runs a second
`mosaic-compile --backend xaml` for the Grid (with
`--package-search-path code/packages`); `MainWindow.xaml` mounts
`<gen:Grid>`; `MainWindow.xaml.cs` feeds the generated control's
dependency properties + a `Dispatch` handler; `VisiCalc.csproj`
compiles the generated Grid files. The per-cell VM projection and
the selected/editing background highlight remain for a Windows dev
(see the demo README + `MainWindow.xaml.cs` TODO).

## [Unreleased] — #4548 toolkit-demo regressions — three emitter gaps closed

Three mosaic-emit-xaml code-gen bugs surfaced when compiling
components from `mosaic-pkg-toolkit` (Button / Alert / Badge / Spinner
demo, PR #4548) through the XAML backend. None of the existing
demos (hello-dialog, mosaic-pkg-grid) exercised the affected style
or naming surface. Each fix is a localised change with regression
tests; the toolkit Button + Alert + Badge XAML now regenerates
cleanly and builds without hand-patches.

### X1 — `border-radius` lowered to invalid `BorderRadius`

`css_property_to_xaml_setter` had no entry for `border-radius`, so
the kebab-to-pascal fallback produced `BorderRadius` — which isn't
a real WinUI 3 property. The XAML markup compiler rejected it
silently (`XamlCompiler.exe` exits 1 with no diagnostic). Fixed
by adding the explicit `"border-radius" => "CornerRadius"` mapping
(`UIElement.CornerRadius` is the actual WinUI property).

Regression test: `border_radius_lowers_to_corner_radius`.

### X2 — `x:Name` collided with the enclosing class name

Components where the pascal-cased part name equals the component
name (e.g. `Button.mll`'s `HostButton [ button ]` inside the
`Button` component) produced `<Button x:Name="Button">`. WinUI's
XAML compiler auto-generates a `private Button Button;` field
that triggers C# error CS0542 ("member names cannot be the same
as their enclosing type"). Affected Button, Checkbox, Input,
Radio.

Fixed by detecting the collision in `host_x_name` and suffixing
`Element` to the identifier. Event-handler stems are derived from
`x_name` so both the XAML attribute (`Click="ButtonElement_Click"`)
and the code-behind method (`private void ButtonElement_Click`)
stay consistent automatically.

Regression tests: `x_name_avoids_component_class_name_collision`,
`x_name_unchanged_when_no_collision`.

### X3 — text-style props on `<Border>` rejected by WinUI

`<Border>` doesn't have `Foreground` / `FontSize` / `FontWeight` /
`FontFamily` — those belong on the text content inside. The emitter
was placing every part-style property on the wrapping `<Border>`
unconditionally, so styled toolkit components like Alert and Badge
emitted invalid markup that XamlCompiler silently rejected.

Fixed in `emit_container` by partitioning the part-style fragment:
container-paint props (Background, BorderBrush, BorderThickness,
CornerRadius, Padding, Margin, Width, Height, *Alignment) stay on
the opening tag; text-style props move into a scoped
`<Border.Resources>` block as a `<Style TargetType="TextBlock">`
implicit style. WinUI's implicit-style resolution then applies
them to every `TextBlock` descendant inside the container.

This change also applies to the other emit_container call sites
(`Stack` → `<Grid>`), which have the same constraint.

Regression tests:
`box_partitions_style_between_border_and_textblock_resource`,
`box_without_text_style_emits_no_resources_block`,
`parse_style_fragment_round_trips_build_style_fragment`.

## [Unreleased] — UI31-K-xaml — `HostTable` RTL contract

The WinUI `HostTable` lowering (which produces a structural `<Grid>`
with `<Grid.RowDefinitions>` per section) now honours the UI31 §3.2
RTL contract via WinUI's `FrameworkElement.FlowDirection`:

- `dir: rtl` → `FlowDirection="RightToLeft"` on the `<Grid>`; flips
  column ordering of all descendant rows automatically.
- `dir: ltr` → `FlowDirection="LeftToRight"` — explicit-LTR for
  tables that should stay LTR inside an ambient-RTL `Page` (e.g.
  number-heavy spreadsheets).
- `dir: auto` → no attribute (spec semantic "let the host decide" =
  WinUI default of inheriting from the `Page`'s `FlowDirection`,
  typically set from `CultureInfo`).
- `dir: slot: layout-direction` → `FlowDirection="{x:Bind LayoutDirection}"`.
  The slot must evaluate to a `FlowDirection`; the slot name passes
  through `kebab_to_pascal_case` + `is_safe_identifier` so it can't
  smuggle malicious XAML through the binding path.
- Unknown keywords drop silently — the allow-list is the security
  gate. Test #6 feeds the literal payload `"RightToLeft\" Tag=\"pwn\""`
  (specifically shaped to break out of the attribute-value quoting)
  and asserts `Tag="pwn"` never reaches the output.

7 new tests cover the a11y gate (structural `<Grid>` with
`<Grid.RowDefinitions>` preserved — not a flat `<StackPanel>` mess),
the three allow-listed keywords (incl. the no-emit `auto` case),
the slot-ref binding through `{x:Bind PascalCase}`, the silent-drop
with attribute-injection payload, and a no-`dir` regression guard.
Total tests: 141 (was 134).

## [Unreleased] — UI29-4 `HostLink` + `HostTooltip` + `HostNumberInput` (U29-4-K-xaml)

Three new UI29-4 kernel primitives lower to native WinUI 3 widgets:

- **`HostLink` → `<HyperlinkButton NavigateUri="..." Content="..."/>`**.
  WinUI 3 ships `HyperlinkButton` specifically for clickable
  hyperlinks (vs `<Hyperlink>` which is the inline-text-flow
  variant). When `external: false` + `onActivate` are both bound,
  the lowering swaps to a `<Button Click="X_Click"/>` with a
  code-behind handler that dispatches the named emit (`href` flows
  into the dispatch payload as a string literal or `this.<Pascal>`
  property reference) — host's in-app router takes over.
- **`HostTooltip` → `<Border ToolTipService.ToolTip="text">child</Border>`**.
  The attached property hooks the tooltip directly to the wrapped
  element with native a11y wiring. `Border` is a layout pass-
  through (no padding/margin/background by default).
- **`HostNumberInput` → `<NumberBox Value="{x:Bind V, Mode=TwoWay}"
  Minimum Maximum SmallChange PlaceholderText IsEnabled
  ValueChanged>`**. WinUI 3's NumberBox is the native numeric
  input with built-in ± stepper, min/max validation, and locale-
  aware decimal parsing. `onChange` registers a `ValueChanged`
  handler that dispatches `XEvent.X(args.NewValue)` — the standard
  WinUI NumberBox event-arg shape (`args.NewValue` is the
  validated `double`).

6 new tests cover: HyperlinkButton with NavigateUri+Content, the
external-false + onActivate Button swap with Click handler +
href-in-payload dispatch, HostTooltip's Border + ToolTipService
wrap, bare NumberBox emission, min/max/step → Minimum/Maximum/
SmallChange mapping, and the ValueChanged code-behind handler
emission.

## [Unreleased] — UI29-2 `HostCheckbox` + `HostRadio` (U29-2-K-xaml)

Both new UI29-2 primitives lower to native WinUI / WPF widgets:

- `HostCheckbox` → `<CheckBox>` with `IsChecked` / `IsEnabled` / `Content`
  / `IsThreeState` / `Checked` + `Unchecked` events.
- `HostRadio`    → `<RadioButton>` with `IsChecked` / `IsEnabled` /
  `Content` / `GroupName` / `Checked` event (only — `Unchecked` is
  silent per UI29-2 §2.2's "onSelect = this radio was chosen").

Detailed prop handling:

- `checked: slot: c` → `IsChecked="{x:Bind C, Mode=OneWay}"`.
- `checked: true|false` → `IsChecked="True"` / `IsChecked="False"`.
- `disabled: slot: d` → `IsEnabled="{x:Bind Not(D)}"` (reuses
  HostButton's shared `Not(bool)` helper).
- `disabled: true|false` → `IsEnabled="False"` / `IsEnabled="True"`.
- `label: str|slot` → `Content="..."` / `Content="{x:Bind Label}"`.
- `HostCheckbox.indeterminate: slot|true` → `IsThreeState="True"`.
  The actual `IsChecked = null` transition is the host's job (WinUI
  doesn't have a "show as indeterminate" attribute, only the
  three-state-enabled flag).
- `HostCheckbox.onToggle: emit: onX` → registers TWO code-behind
  handlers — `<x>_Checked` dispatches `XEvent.X(true)` and
  `<x>_Unchecked` dispatches `XEvent.X(false)`. WinUI has no
  combined "toggled" event; the pair satisfies the kernel-canonical
  `onToggle(checked: bool)` signature exactly.
- `HostRadio.group: str|slot` → `GroupName="..."` / `GroupName="{x:Bind G}"`.
  WinUI auto-deselects siblings sharing `GroupName` when one
  `IsChecked` goes true — true radio-group behavior at the XAML
  level, no userland RadioGroup needed for v1.
- `HostRadio.value: str|slot` → flows into the C# dispatch payload
  as a string literal (escaped) or `this.<Pascal>` property ref.
- `HostRadio.onSelect: emit: onX` → registers ONLY a `<x>_Checked`
  handler that dispatches `XEvent.X(<value>)`. The `Unchecked` event
  is intentionally not wired so sibling-caused deselects don't
  trigger `onSelect`.

10 new tests cover: bare CheckBox / RadioButton blocks, checked-slot
binding, string label → Content, disabled → Not(bool) helper,
onToggle's Checked + Unchecked pair with matching bool payloads,
indeterminate → IsThreeState, bare RadioButton, group → GroupName,
onSelect with string-literal value, onSelect with slot-typed value.

Internal: added `escape_csharp_string` helper for embedding string
literals inside C# code-behind handler bodies (separate from
`escape_xaml_attr`, which is for XML-attribute contexts).

## [Unreleased] — `--emit-project` (B1, B2, B3 from demo catalog)

### Added — full WinUI 3 host shell generation

`mosaic-compile --backend xaml --emit-project -o <BASE>` now produces a
buildable WinUI 3 project alongside the per-component triple. Output:

| File | Source |
|---|---|
| `<Component>.xaml` / `.xaml.cs` / `.Event.cs` | mosaic-emit-xaml (component triple) |
| `<Component>.csproj` | --emit-project |
| `App.xaml` / `App.xaml.cs` | --emit-project |
| `MainWindow.xaml` / `MainWindow.xaml.cs` | --emit-project |
| `app.manifest` | --emit-project |
| `build.ps1` | --emit-project |
| `README.md` | --emit-project |
| `BoolToVisibilityConverter.cs` (when `If` used) | A5 (PR-2) |
| `<Component>_<As>Vm.cs` (one per For block) | PR-2 |

The `MainWindow` shape depends on the component's `RootShape`:
- `ContentDialog`-rooted (HostDialog): host window has a "Show
  dialog" button which constructs the dialog, sets its `XamlRoot`
  from the button (Fix D1 from the demo catalog), wires the
  `Dispatch` event to a stub handler, and `ShowAsync`'s it.
- `UserControl`-rooted: host window's Grid hosts the component
  directly as its main content; component DPs are wired in the
  MainWindow constructor.

Slot DPs are pre-populated with sensible stubs (`"Sample <Slot>"`
for text, `0` for number, `false` for bool, `null!` for image/node,
empty list for `list<T>`). The user replaces them with real data.

The `Dispatch` event is wired to `OnComponentDispatch` which
pattern-matches the discriminated event union. Each arm has a
`// TODO: business logic for <EventName>` comment marking the
insertion point.

### Added — Fix B2: native runtime DLL flattening via MSBuild post-build target

The emitted `.csproj` includes a `FlattenNativeRuntimeDlls` target
that copies `Microsoft.WindowsAppRuntime.Bootstrap.dll` from
`runtimes/win-x64/native/` to the output root next to the .exe.
`dotnet build` doesn't do this (only `dotnet publish` does); without
it the unpackaged bootstrap crashes on launch.

### Added — Fix B3: `build.ps1` driver script

Cleans bin/obj with `-Clean`, builds with `dotnet build -c Debug
-p:Platform=x64 --nologo` (Platform=x64 required because
WindowsAppSDK refuses AnyCPU), and with `-Run` launches the .exe.

### Added — Per-project README

Documents file roles, the `winget install
Microsoft.WindowsAppRuntime.1.7` prerequisite, the expected cosmetic
MSB4062 error, and the build/run commands.

### CLI

- New `--emit-project` boolean flag in
  `code/specs/mosaic-compile.json`.
- `run_pipeline` threads it through to `EmitOptions::emit_project`.
- The xaml branch also writes any `if_helpers` side-files (the
  BoolToVisibilityConverter.cs from A5, when needed).

### MSBuild csproj details that required experimentation

- `<UseRidGraph>true</UseRidGraph>` — WindowsAppSDK uses legacy
  `win10-*` RIDs that .NET 8+ removed from the default graph.
- `<AppxGeneratePriEnabled>false</AppxGeneratePriEnabled>` +
  `<EnableDefaultPriItems>false</EnableDefaultPriItems>` — bypass
  most AppxPackage MSBuild plumbing that requires Visual Studio.
  One cosmetic MSB4062 still fires at the very end of build; the
  .exe + dependencies are produced first.
- `<WindowsAppSDKSelfContained>false</WindowsAppSDKSelfContained>`
  (NOT true). The 1.7 NuGet's bundled `Microsoft.UI.Xaml.dll`
  3.1.7.0 currently crashes with `0xc000027b` on initialization
  when self-contained — switching to framework-dependent (relying
  on system-installed runtime) is the only path that actually
  launches.

### End-to-end verification

1. Authored a minimal `HelloDialog.mil`/`.mll`/`.dark.msl` triple
   using HostDialog.
2. Ran `mosaic-compile --backend xaml --emit-project -o
   /tmp/proj-test/HelloDialog` — 11 files written.
3. Ran `powershell ./build.ps1`. Build emitted one cosmetic
   MSB4062 error (documented); the .exe was produced at
   `bin/x64/Debug/.../HelloDialog.exe`.
4. Launched the .exe. Window appeared with title "HelloDialog —
   Mosaic → XAML demo".
5. Clicked "Open the dialog" via UIAutomation. The ContentDialog
   appeared with the stub Message and a Close button.
6. Pressed Close. The dialog dismissed; the status bar updated to
   "Dispatch: Close" — proof the `HelloDialogEvent.Close` event
   round-tripped through the generated wiring to the host's
   `OnComponentDispatch` handler.

End-to-end Mosaic → XAML → on-screen dialog with **zero hand-patches**.

## [Unreleased] — HostDialog runnability fixes (A1–A5 from demo catalog)

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

### Tests

- 4 new tests cover the fixes:
  - `host_dialog_at_root_modal_false_still_uses_contentdialog_root` (A1)
  - `host_dialog_title_slot_emits_xbind_oneway_after_a3` (A3 + A4)
  - `host_dialog_title_slot_named_title_aliases_to_dialog_title` (A4 with collision)
  - `host_dialog_open_slot_emits_comment_stub_only_after_a2` (A2)
- 3 PR-1 tests updated to match the new contract (Flyout test moved
  to the nested path).
- Total: 115 unit tests + 5 integration tests pass.

### End-to-end verification

Regenerated the `code/programs/csharp/hello-dialog-xaml/` artifacts from the unedited
`.mil`/`.mll`/`.msl` triple via `mosaic-compile --backend xaml`. The
generated XAML, code-behind, and Event union are now byte-identical
to the working hand-patched files in
`code/programs/csharp/hello-dialog-xaml/winui/`. After PR-3 (`--emit-project`) and
PR-4 (regenerate the demo) land, the demo will need zero hand-patches.

## [Unreleased] — U29-1-K-xaml — HostDialog kernel primitive

### Added — `HostDialog` lowering (UI29-1 §3.6)

- `HostDialog` lowers to WinUI 3's `ContentDialog` (modal: true,
  the default) or `Flyout` (modal: false). Both are platform-level
  top-layer primitives — they provide modal blocking / focus
  trap / dismiss handling out of the box (per UI29-1 §1 these
  cannot be composed from `<Border>`/`<Grid>`).
- `modal: true` (keyword default) → `<ContentDialog>`.
- `modal: false` (keyword) → `<Flyout>` (popover form).
- `title: slot: x` → `Title="{Binding X}"` (matches the spec
  §3.6 sketch's Binding form so the host's DataContext drives
  the title text).
- `title: "literal"` → `Title="literal"` (XAML-escaped).
- `open: slot: x` → `mos:Dialog.IsOpen="{Binding X}"` plus a
  documented stub comment naming the binding so the host's
  code-behind can wire `ShowAsync()` / `Hide()` against the slot.
- `onClose: emit: onX` → `Closed="OnHostDialogClose_N"` plus a
  generated private `void OnHostDialogClose_N(object, object)`
  in the code-behind that dispatches the named emit case
  (matches the HostButton.Click handler pattern).
- `dismiss-on-backdrop: false` → comment stub (XAML's
  ContentDialog has no boolean equivalent — only the
  `LightDismissOverlayMode` enum on Flyout / `IsLightDismissEnabled`
  on a few other controls). Documented in the emitted XAML so the
  gap is visible in diffs.

### Why code-behind stubs and not full plumbing

ContentDialog is not driven by a simple `IsOpen` DP — the caller
must `await dialog.ShowAsync()` to present it. The lifecycle
plumbing lives on the host project's code-behind, the same shape
the HTML/React backends use for the equivalent dialog primitive.
This emitter writes the XAML element, the comment contract, and
the Closed event handler; the host writes the ShowAsync/Hide
side. A follow-up PR can lift this into an emitted attached
property + a small static helper class — leaves the spec-shape
intact today.

### Tests

- 9 new tests covering: empty HostDialog → ContentDialog; explicit
  `modal: true`; `modal: false` → Flyout; `title` slot binding;
  child rendering inside the body; `onClose` handler emission;
  `open` slot binding stub + comment; recognition (no
  UnsupportedPrimitive); `dismiss-on-backdrop: false` comment stub.
- Total: 113 tests (was 104).

### Drive-by

- Clippy clean-ups in pre-existing PR-1..PR-3 emitters
  (`write!` → `writeln!` for trailing newlines, manual
  `Option::filter`, identical-branch collapse in `emit_text`'s
  `Keyword` arm). Behavioural no-ops; existing tests cover.

## [Unreleased] — PR-6 — mosaic-pkg-grid through xaml + CLI wiring

### Added — `mosaic-compile --backend xaml` CLI wiring

- `mosaic-compile --interface X.mil --layout X.mll --style X.msl
  --backend xaml [-o BASE]` now compiles a three-file Mosaic
  pipeline triple to a WinUI 3 component triple.
- The `--backend` validation list grew `xaml`.
- `run_pipeline` branches on backend: `react` emits one `.tsx`
  file (unchanged from before); `xaml` emits the triple
  (`{base}.xaml`, `{base}.xaml.cs`, `{base}.Event.cs`) plus
  zero-or-more RowVm `.cs` files. `BASE` is treated as a file-name
  prefix; the default is the component name. A trailing `.xaml` in
  `BASE` is stripped so `Grid.xaml` produces three sensibly-named
  files instead of `Grid.xaml.xaml.cs` etc.
- Three new prints (`Written: ...`) per invocation in xaml mode.
- `mosaic-compile`'s `Cargo.toml` now depends on `mosaic-emit-xaml`.

### Added — End-to-end integration test against `mosaic-pkg-grid`

- New `tests/pkg_grid_compiles_to_xaml.rs` integration test.
- Resolves `mosaic-pkg-grid`'s source root relative to
  `CARGO_MANIFEST_DIR` (steps up four directory levels), then
  compiles each component (`Grid`, `Cell`, `Column`) through the
  three IR compilers and the XAML emitter.
- 5 tests cover: package source resolution; each component lowers
  through `from_pipeline` without error; Grid (the complex
  component using HostTable + For + Cell component reference)
  produces the expected XAML structure (UserControl root,
  ItemsRepeater for For, `<grid:Cell/>` reference, xmlns:grid
  declaration); Grid produces RowVm side-files.
- This is the spec §17 PR-6 capstone — the XAML emitter is
  "done" in the spec sense when `mosaic-pkg-grid` compiles cleanly
  end-to-end, which it now does.

### What's NOT in this PR (deferred to PR-7)

- **VisiCalc Windows demo** (`code/programs/typescript/visicalc/windows/xaml/`) — the
  full end-to-end app that consumes the compiled `mosaic-pkg-grid`
  package and a hand-written `FormulaBar` component. PR-7 lands
  this directory, the `windows/build.ps1` driver, and the
  hand-written C# host code (`State.cs` mirroring
  `src/app/state.ts`).
- **`dotnet build` smoke test** on Windows CI. Requires the
  Microsoft .NET SDK + Windows App SDK; will land alongside the
  demo so we have a real consumer to validate against.
- **Manifest-driven CLI** (`mosaic-compile pkg <path> --backend xaml`)
  that walks `mosaic-package.toml`, parses dependency manifests,
  and constructs the `ComponentRegistry`. The single-component
  invocation works today; the multi-component package invocation
  needs the resolver wired into `run_pkg`.

### Tests

- 5 new integration tests in `tests/pkg_grid_compiles_to_xaml.rs`.
- Unit tests unchanged: 104 still pass.
- Total across unit + integration: 109.

## [Unreleased] — PR-5 — Component reference resolution

### Added — `ComponentRegistry` public type

- New `ComponentRegistry` + `ComponentRef` types re-exported from
  the crate root. The registry maps PascalCase tag names →
  `(xmlns_prefix, xmlns_value, package_name)` and is the input the
  emitter consumes when resolving a non-kernel tag.
- The CLI (mosaic-compile) is responsible for populating the
  registry from parsed dependency manifests; the emitter takes the
  already-resolved data and emits the XAML reference.
- Tests use the registry directly — `ComponentRegistry::new()` +
  `.register("Grid", "grid", "using:Mosaic.Package.Grid", "mosaic-pkg-grid")`.

### Changed — `from_pipeline` signature

The fourth argument changed from `manifest: Option<&()>` (a stub from
PR-1) to `registry: Option<&ComponentRegistry>`. Callers that don't
need component references continue to pass `None`; the behaviour for
them is identical to PR-4.

### Added — Non-kernel tag → `<{prefix}:{Tag} ... />` reference

When a layout node's tag isn't in the UI29 kernel:

- **With a registry** AND the tag is registered → emits
  `<{prefix}:{Tag} ... />` with the registered xmlns prefix. The
  matching `xmlns:{prefix}="{value}"` declaration lands on the
  `<UserControl>` root tag.
- **With a registry** AND the tag is NOT registered →
  `PipelineEmitError::UnknownComponent(tag)` (the spec's intended
  error for missing manifest dependency).
- **Without a registry** → `PipelineEmitError::UnsupportedPrimitive(tag)`
  (preserves pre-PR-5 behaviour for callers that don't use packages).

Kernel primitives ALWAYS win over registry entries — if a registry
happens to define an entry for `Box` / `Text` / etc., the kernel
emitter is used and the registry entry is ignored. This protects
against accidental shadowing.

### Added — Component-reference prop resolution

The emitter walks the component-reference's `props` and produces
XAML attribute fragments:

- `slot ref` → `Attribute="{x:Bind Path}"` (PascalCased)
- `string literal` → `Attribute="literal"` (XAML-escaped)
- `number` → `Attribute="N"`
- `keyword (for-bound name)` → `Attribute="{x:Bind Name}"` (treated
  as a bound name when in scope)
- `keyword (other)` → `Attribute="literal"` (passes through)
- `expr` → routed through the PR-2 ExprLowerer (bindable path or
  helper call)
- `emit ref` → DEFERRED — surfaced as a XAML comment listing the
  skipped props so the gap is visible in diffs. Host-side handler-stub
  generation is PR-5+ work and lands in a follow-up.

### Added — xmlns deduplication

Two references to the same package produce ONE `xmlns:prefix="..."`
declaration on the `<UserControl>` root. The internal map is keyed
by xmlns prefix; `BTreeMap` storage gives deterministic alphabetical
output ordering.

### Tests

- 12 new tests cover: registry register/lookup round-trip, registry
  empty-lookup misses, no-registry → UnsupportedPrimitive, empty
  registry → UnknownComponent, prefixed XAML tag emission, xmlns
  declaration injection, slot-ref / string-literal / emit-ref prop
  mapping, multi-package xmlns emission, xmlns dedup for repeated
  package use, kernel-primitive shadowing protection.
- Total: 104 tests (was 92 in PR-4, +12).

### Known limitations carried to PR-6

- **CLI integration** (`mosaic-compile --backend xaml --package-mode`)
  still pending. The CLI needs to read each dependency's
  `mosaic-package.toml`, parse it via `mosaic-package-manifest`, and
  populate the `ComponentRegistry` before invoking `from_pipeline`.
  Same status as the swiftui/qt backends.
- **Emit-ref props on component references** are surfaced as a
  comment but not wired. The host-side handler stubs and the
  package's own `Dispatch` event subscription are PR-5+ work that
  lands either at the tail end of the xaml series or in a generic
  cross-backend PR.
- **`--use-community-datagrid` flag** still inert (PR-4 carryover).

## [Unreleased] — PR-4 — HostTable + section sub-tags

### Added — `HostTable` lowering (spec §5)

- `HostTable [name] { section sub-tags... }` lowers to a hand-rolled
  `<Grid>` with `Grid.RowDefinitions` driven by the present section
  sub-tags. Each section appears at most once per HostTable; duplicates
  produce a `DuplicateTableSection` error.

### Added — Section sub-tag handling

- **`HostTableColGroup`** — recognised but ignored in PR-4 (the
  column-widths layout question per spec §5.2 needs more design).
- **`HostTableHead`** — emits as `<StackPanel Grid.Row="N" Orientation="Vertical">`
  containing the header row(s). Auto-sized row.
- **`HostTableBody`** — emits inside `<ScrollViewer Grid.Row="N" VerticalScrollBarVisibility="Auto">`
  for vertical overflow. `*`-sized row (fills remaining space).
- **`HostTableFoot`** — same shape as Head but at the last Grid.Row.
  Auto-sized.

Each section's `Row` children become `<StackPanel Orientation="Horizontal">` (via the existing `emit_stack_panel` reused from PR-1).
Sections also accept `For` and `If` children so authors can iterate /
conditionally include rows. Any other child of a section is an
`UnsupportedPrimitive` error.

### Added — Empty HostTable case

An empty HostTable (no section sub-tags) lowers to a single `<Grid/>`
self-closing element, preserving any part-style attributes.

### Added — Section sub-tags as direct nodes error

`HostTableHead` / `HostTableBody` / `HostTableFoot` / `HostTableColGroup`
appearing outside a HostTable (i.e. as direct children of a non-table
container) surface as `UnsupportedPrimitive("HostTable<X> outside HostTable")`.

### Tests

- 11 new tests covering: head-only Grid shape; head+body two-row Grid;
  body-only ScrollViewer wrap; foot-only no-ScrollViewer; full quad
  ColGroup+Head+Body+Foot Grid.Row assignment; empty HostTable; duplicate-section
  error; unknown-child-of-HostTable error; non-Row-child-of-section
  error; `For` inside a section iterating over rows; orphan section
  sub-tag at top level; part-style application on the outer `<Grid>`.
- One PR-1 test (`host_table_errors_with_unsupported_primitive`)
  updated to verify the empty-Grid lowering.
- Total: 92 tests (was 81 in PR-3, +11).

### Known limitations carried to later PRs

- **HostTableColGroup column-widths layout** — the spec §5.2 caveat
  about WinUI 3's lack of a native semantic-table control means
  column widths need either explicit `Grid.ColumnDefinitions` or
  per-cell `Width` settings. PR-4 emits the StackPanel-per-row
  structure but doesn't yet propagate column widths; the
  ColGroup sub-tag is recognised and ignored. A follow-up tackles
  this together with the `--use-community-datagrid` flag.
- **`--use-community-datagrid` flag** — exists on `EmitOptions` but
  not yet acted on. When set, future PR will switch the lowering to
  `<controls:DataGrid>` from CommunityToolkit.WinUI for full UIA
  fidelity (spec §5.3 caveat).
- **Component references** still `UnsupportedPrimitive` pending PR-5.

## [Unreleased] — PR-3 — HostInput / HostButton / HostScroll

### Added — `HostInput` lowering (spec §4.1)

- `HostInput` lowers to `<TextBox>` with the spec's attribute mapping:
  - `value: slot: V` → `Text="{x:Bind V, Mode=TwoWay}"`
  - `value: "..."` → `Text="..."` literal
  - `read-only: slot: R` → `IsReadOnly="{x:Bind R}"`
  - `read-only: true` / `false` keyword → literal `IsReadOnly="True"` / `False`
  - `placeholder: "..."` → `PlaceholderText="..."`
  - `max-length: N` → `MaxLength="N"` (integer-cast from the f64 prop)
  - `multiline: true` → adds `AcceptsReturn="True" TextWrapping="Wrap"`
- Event wiring lands as private code-behind handlers:
  - `onChange: emit: X` → `TextChanged` handler dispatching
    `XEvent.{X}(textbox.Text)` (payload-carrying)
  - `onCommit: emit: X` + `onCancel: emit: Y` → merged `KeyDown`
    handler keyed on `VirtualKey.Enter` / `VirtualKey.Escape`
  - `onFocus: emit: X` → `GotFocus` handler

### Added — `HostButton` lowering (spec §4.2)

- `HostButton` lowers to `<Button>` with:
  - `label: slot: L` → `Content="{x:Bind L}"`
  - `label: "..."` → `Content="..."` literal
  - `disabled: slot: D` → `IsEnabled="{x:Bind Not(D)}"` plus a generated
    `private bool Not(bool b) => !b;` helper added once per component
  - `disabled: true` / `false` keyword → literal `IsEnabled="False"` / `True`
  - `onClick: emit: X` → `Click` handler dispatching `XEvent.{X}()`

### Added — `HostScroll` lowering (spec §4.3)

- `HostScroll` lowers to `<ScrollViewer>` wrapping its children.
  Direction keyword maps to scroll-bar visibility:
  - default (vertical): `VerticalScrollBarVisibility="Auto"` + `HorizontalScrollBarVisibility="Disabled"`
  - `direction: horizontal`: H=Auto, V=Disabled
  - `direction: both`: both Auto

### Added — `x:Name` allocation for Host* primitives

- When the node has a `part_name`, the `x:Name` is the part name
  PascalCased (`formula-field` → `FormulaField`). Matches the spec's
  examples and the convention React/SwiftUI use for code-behind refs.
- When the node lacks a `part_name`, the emitter allocates a
  monotonically-increasing per-component counter (`HostInput_1`,
  `HostInput_2`, ...). Stable across rebuilds.

### Added — `HostHandler` registration on `EmitContext`

- The Host* event handlers are accumulated on `EmitContext::host_handlers`
  during the walk and emitted inline in the code-behind partial class
  after the PR-2 helper methods. The dedup is by handler name, mirroring
  the helper-dedup pattern.

### Tests

- 19 new tests covering: each Host* primitive's attribute mappings;
  event-handler emission (TextChanged with payload, merged KeyDown for
  Commit+Cancel, Click); `x:Name` allocation with and without
  `part_name`; the `Not(bool)` helper generation for disabled
  polarity flip; multi-counter assignment across multiple unnamed
  HostInputs.
- Two PR-1 tests
  (`host_input_errors_with_unsupported_primitive`) updated to verify
  the new successful lowering shape instead of the previous error.
- Total: 81 tests (was 62 in PR-2, +19).

### Known limitations carried forward to later PRs

- **`HostTable`** + section sub-tags still `UnsupportedPrimitive`
  pending PR-4.
- **Component references** still `UnsupportedPrimitive` pending PR-5.
- **`BoolToVisibilityConverter` C# class** still references-only;
  hosts need to ship one. A follow-up emits the converter alongside
  the rest.
- **HostInput event payload** captures the *raw* `tb.Text` of the
  `TextBox` at dispatch time. A future PR may switch to two-way
  bindings for the slot in addition to the dispatch (mirroring
  the React emitter's `e.target.value` pattern).
- **HostButton accelerator-key wiring** (e.g. `accelerator: "Ctrl+S"`
  → `KeyboardAccelerator`) is out of scope for PR-3.

## [Unreleased] — PR-2 — If / Else / For + ExprLowerer

### Added — `For` lowering (spec §6.1)

- `For (each: <expr>, as: <name>, index: <name>?) { ... }` now lowers
  to `<ItemsRepeater ItemsSource="{x:Bind ...}">` with an
  `<ItemsRepeater.ItemTemplate>` containing a `<DataTemplate
  x:DataType="local:{Component}_{AsName}Vm">`.
- One `RowVm` C# record is generated per `For` block and surfaced as
  an `EmittedFile` in `XamlEmitResult::for_view_models`. The record
  shape is
  `public sealed record {Component}_{AsName}Vm(ElementType ElementProperty[, int Index]);`.
- RowVms dedupe within a component: two `For` blocks binding the same
  `as:` name share one generated record.
- The element type is derived from the iterated slot's declared
  mosmodel type (`list<text>` → `string`, `list<number>` → `double`,
  `list<bool>` → `bool`, etc.). Expressions like `row.cells` that
  don't resolve to a typed slot default to `object`.
- `For`'s bound name and optional index are pushed into the
  `EmitContext::for_scope` for the duration of the body walk; nested
  `For` blocks are supported, innermost shadowing outermost.

### Added — `If` / `Else` lowering (spec §6.2)

- `If (when: <expr>) { ... } [Else { ... }]` lowers to twin
  `<ContentControl>`s whose `Visibility` is bound to the expression
  and (for the `Else` branch) the negation via `ConverterParameter=invert`.
- A `BoolToVisibilityConverter` resource is added to
  `<UserControl.Resources>` exactly once per component when any `If`
  is emitted (the converter implementation itself is expected to ship
  with the host project or via a future PR; for now the emitter just
  references the `x:Key`).
- `Else` is paired with the preceding `If` by the new
  `emit_xaml_children` look-ahead. A standalone `Else` errors with
  `UnsupportedPrimitive("Else without preceding If")`.

### Added — `ExprLowerer` (spec §6.3)

- A small recursive-descent parser-and-lowerer over the UI29 §3.3
  expression grammar. Returns one of:
  - `Bindable(path)` — direct `{x:Bind X}` path. Covers bare slot ref
    (`slot: foo`), bare for-bound name (`row`), boolean literal
    (`true`/`false`), and dotted member access (`row.value.bg`).
  - `Helper(call)` — a registered helper-method call (e.g.
    `Expr_a3f24b6c(R, C)`). Covers indexers (`row[c]`), comparisons
    (`==`, `!=`, `<`, `<=`, `>`, `>=`), logical (`&&`, `||`), and
    unary `!`.
  - `Unsupported(reason)` — anything else gets a human-readable
    diagnostic via `PipelineEmitError::UnsupportedExpression`.
- Helper methods land inline in the code-behind partial class as
  `private <Type> Expr_<hash>(<params>) => <body>;`. The body is a
  direct transliteration of the moslayout expression to C# (operators
  carry through identically; `slot:` becomes `this.`; for-bound names
  become PascalCased parameters).
- Helpers dedupe by name (deterministic FNV-1a hash of the original
  expression source).

### Deviation from spec §13

The spec's `if_helpers` field on `XamlEmitResult` is intended to carry
helper sources as separate files. PR-2 inlines the helpers directly
into the code-behind `partial class` instead, leaving `if_helpers`
empty. The motivation is that one `.xaml.cs` file is simpler to
review and slot into a WinUI 3 project than a sibling `.cs` per
helper. If a reviewer prefers the separate-file shape, the inlining
can flip cheaply — the registration mechanism (`EmitContext::helpers`)
already shapes the data the right way for either output.

### Tests

- 21 new tests cover: For with slot ref / numeric list / index
  binding; RowVm record shape and dedup; If with slot ref / true
  keyword / paired Else; converter resource emission and uniqueness;
  ExprLowerer for each lowering category (bindable bare ref, bindable
  dotted, bindable literal, helper indexer, helper comparison, helper
  logical, helper unary not, helper dedup, helper with for-bound
  parameters); standalone-Else error; an end-to-end `For` + `If`/`Else`
  combination producing the expected ItemsRepeater + paired
  ContentControl nesting.
- Total test count: 62 (was 41 in PR-1).

### Public API changes

- `XamlEmitResult::for_view_models` now populated (was always empty
  in PR-1).
- `XamlEmitResult::if_helpers` remains an empty `Vec` (helpers inline
  into `code_behind`; see Deviation above).
- No breaking changes to consumers.

### Known limitations carried forward to later PRs

- The `BoolToVisibilityConverter` class itself is referenced by
  `x:Key` but not emitted. Hosts need to ship one (a 5-line C# class).
  A follow-up may bundle it as a fixed asset or emit it once per
  component.
- `HostInput`, `HostButton`, `HostScroll`, `HostTable` (+ section
  sub-tags), component references — all still `UnsupportedPrimitive`
  pending PR-3 / PR-4 / PR-5.

## [0.1.0] — Unreleased — PR-1 scaffold

### Added — initial crate

First implementation per `code/specs/mosaic-emit-xaml.md` §17 PR-1.

- Public API: `from_pipeline(interface, layout, style, manifest, options)
  -> Result<XamlEmitResult, XamlEmitError>`.
- `XamlEmitResult` carries three generated source strings: `xaml`,
  `code_behind`, `events` (per spec §2 output shape). The `project`,
  `for_view_models`, and `if_helpers` fields exist on the struct but are
  always empty / `None` in PR-1 — they fill in across PR-2..PR-6.
- The nine simple UI29 kernel primitives lower:
  - `Box` → `<Border>` (or `<ContentPresenter>` for the bare-container case)
  - `Row` → `<StackPanel Orientation="Horizontal">`
  - `Column` → `<StackPanel Orientation="Vertical">`
  - `Stack` → `<Grid>` (z-axis container)
  - `Text` → `<TextBlock>` with slot-binding or literal content
  - `Image` → `<Image Source="..."/>`
  - `Spacer` → `<Rectangle/>` flex glue
  - `Divider` → `<Border BorderThickness="..."/>`
  - `Icon` → `<FontIcon Glyph="..."/>`
- UI24 event-dispatch contract: emits a `partial class {Component}Event`
  with one nested `sealed record` per declared emit, plus a `public event
  EventHandler<{Component}Event>? Dispatch;` on the UserControl.
- Slot → `DependencyProperty` translation per spec §8. The mapping table
  covers `text` / `number` / `bool` / `color` / `image` / `node` /
  `list<T>` / `list<list<T>>` from mosmodel.
- Component-name mismatch validation across `.mil` / `.mll` (the `.msl`
  is allowed to disagree per UI23 §4).
- Errors: `ComponentNameMismatch`, `UnsupportedPrimitive`,
  `UnsupportedExpression`, `UnknownComponent`, `UnmappableSlotType`,
  `UnmappableStyleProperty`, `DuplicateTableSection`, `UnsafeSlotName`,
  `UnsafeEmitName`. The PR-1 emitter only fires the first three plus the
  identifier checks; the rest become reachable in PR-2..PR-5.

### Known limitations (deferred per the spec's PR sequence)

- **`If` / `Else` / `For` / `Expr`** — these surface as
  `UnsupportedPrimitive` / `UnsupportedExpression` in PR-1. The
  `ExprLowerer` plus the `<ContentControl>` / `<ItemsRepeater>` lowerings
  land in PR-2.
- **`HostInput` / `HostButton` / `HostScroll`** — same: `UnsupportedPrimitive`
  in PR-1, real lowering in PR-3.
- **`HostTable` + section sub-tags** — `UnsupportedPrimitive` in PR-1,
  real lowering in PR-4. The four sub-tags (`HostTableColGroup`,
  `HostTableHead`, `HostTableBody`, `HostTableFoot`) are recognised by
  name only as a "you need PR-4" diagnostic.
- **Component references (non-kernel tags)** — `UnsupportedPrimitive` in
  PR-1; the manifest-driven resolver lands in PR-5.
- **`mosstyle::StyleDef`** — accepted in the signature so consumers can
  build against the stable interface today; only base `part` blocks
  inline as a `<UserControl.Resources>` `<Style>` per part. State
  blocks (`state hover { ... }`) get a placeholder
  `<VisualStateGroup>`; the full `<VisualState>` setter wiring is a
  follow-up.
- **`<UserControl.Resources>` theming cascade** — host overrides land
  with the component-reference resolver in PR-5.
- **`--use-community-datagrid`** flag — placeholder on `EmitOptions`,
  has no effect until PR-4.
- **`--package-mode`** flag — placeholder on `EmitOptions`, has no
  effect until PR-5.
- **`dotnet build` Windows-only smoke test** — gated by
  `#[cfg(target_os = "windows")]`; PR-1 includes the test scaffold but
  the actual `dotnet` CLI invocation lands once we have a real WinUI 3
  consumer (a follow-up after PR-6 builds the VisiCalc demo).
- **`mosaic-compile --backend xaml` CLI wiring** — the CLI driver's
  `run_pipeline` currently only routes `--backend react`; the swiftui
  and qt backends also aren't wired today. A small follow-up PR will
  add the three new arms together (xaml/swiftui/qt) once the team
  agrees on the multi-file output convention (XAML emits three files
  per component; pure react/swift/qt emit one each).
