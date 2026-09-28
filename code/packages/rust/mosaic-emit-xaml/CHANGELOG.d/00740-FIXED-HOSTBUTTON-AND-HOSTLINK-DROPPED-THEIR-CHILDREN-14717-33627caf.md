### Fixed — `HostButton` and `HostLink` dropped their children (#14717)

Both emitted self-closing elements unconditionally, discarding any nested
subtree with no error, no warning, and no degradation entry. A probe wrapping
the real `mosaic-pkg-card` Card produced a 10-line file containing a lone
`<Button/>` and zero occurrences of the card's title, body, or footer.

`Button` and `HyperlinkButton` are both `ContentControl`s, so the subtree is
expressible as the control's content — this was a missing case rather than a
platform limit. A `label:` still wins, since it has already been lowered into a
`Content="..."` attribute and a control cannot carry both.

`HostLink` needed a second fix beyond rendering children. With no label it
falls back to showing the raw `href` as visible text, which is reasonable for a
bare link but wrong for one wrapping a subtree: it hid the subtree *and*
displayed a routing path as body text. The fallback now applies only when there
are no children.

Found by testing the composition pattern `Card.mil` documents for interaction.
It worked on seven backends and produced an empty button on this one, which
made every product following that advice broken on Windows only.

## [Unreleased] — bind native XAML waveform line coordinates

`Path kind: line` now accepts literal numbers, slot references, and numeric
expressions for `x1`, `y1`, `x2`, and `y2`. Numeric index expressions such as
the SPICE workbench's `segment[0]` generate `double` helpers; inside a typed
`For` template those helpers are exposed as local row-view-model properties so
WinUI's XAML compiler never has to resolve an `Owner.Helper(...)` binding.

`circle` and `curve` coordinates intentionally remain literal-only. Their
current lowering respectively needs derived margins and `Point` construction,
which need a broader geometry-binding contract rather than an unsafe partial
extension of line bindings. The new workbench acceptance test verifies all four
adjacent-point segment coordinates through the actual package pipeline.

## [Unreleased] — activate UI49 one-of slot states (#14359)

Mosstyle states owned by `.mil` `one-of` slots now activate from the generated
WinUI dependency properties. Property-scoped `VisualStateGroup`s compare each
closed-set slot value through a generated ordinal string converter, preserve
model slot order when multiple axes apply, and keep explicit structural and
interaction predicates at higher precedence. The lowering covers native Host
controls plus paint-bearing `Box`, `Row`, `Column`, and `Stack` containers,
including state-only styles with no base paint.

A generated two-axis WinUI project was compiled with .NET 9 and the pinned
Windows App SDK after the emitter and package-builder suites passed.

## [Unreleased] — preserve dynamic HostButton accessible names (#13754)

WinUI buttons now lower literal, slot-bound, keyword-bound, and bindable
expression `HostButton.a11y-label` values to `AutomationProperties.Name`,
including generated row-view-model bindings.

## [Unreleased] — preserve HostInput accessible names (#13717)

WinUI text boxes now lower literal, slot-backed, and bindable expression
`HostInput.a11y-label` values to `AutomationProperties.Name`.

## [Unreleased] — preserve WinUI application resources in publish output (#13658)

Emitted unpackaged WinUI projects now copy their application PRI and generated
XBF files from the build output into `PublishDir`. `dotnet publish` previously
omitted those app-owned resources even though `dotnet build` produced them, so a
self-contained publish built cleanly and then exited during XAML startup with
stowed exception `0xC000027B`. The PRI lookup follows `AssemblyName`, preserving
product overrides such as Trestle, while the XBF glob is restricted to the
generated build-output directory.

## [Unreleased] — retain typed row payloads for native button events (#13573)

`HostButton` and internal `HostLink` controls inside `For` templates now bind
their generated row VM into `FrameworkElement.Tag`. Click handlers recover index,
text, number, and boolean payloads from that stable binding instead of assuming
WinUI's `ItemsRepeater` populated `DataContext`. This fixes visibly inert TaskApp
completion and delete controls that previously dispatched the fallback index `-1`.

## [Unreleased] — drive `ThemeShadow` from the new `elevation` property (UI41, #12028 item 1)

Second PR of the elevation-tokens cascade (mosstyle contract → **XAML**
→ Compose → Qt → Flutter). Replaces `part_wants_theme_shadow`'s
box-shadow-value-sniffing heuristic with a direct read of the new
`elevation` prop (`part_elevation_tier`, returning a new `ElevationTier`
enum instead of `bool`). `raised` keeps the already-shipped
`Translation="0,0,4"` + `<Tag.Shadow><ThemeShadow/></Tag.Shadow>`
treatment; `overlay` gets a deeper `Translation="0,0,16"` — no real
caller yet (UI41 §3), wired up for future floating UI.

This is a real behavior change from the prior heuristic: `box-shadow`
alone no longer implies elevation — `elevation` does. Every real part
in this repo already declares both together (mosstyle-compiler's own
PR migrated all 8 real `.msl` files), so this only affects a
hypothetical author who writes `box-shadow` without `elevation` — that
case now correctly surfaces as a real, reported drop in
`dropped_style_properties` instead of silently matching the old
heuristic.

**A real gap found via full-package verification, not just unit
tests**: compiling the actual TaskApp package and counting rendered
shadows (9 of 13 real `elevation: raised` parts) revealed that
`emit_flex_grid` (`Row`/`Column`) and `emit_host_draggable`
(`HostDraggable`) never called the shadow helper at all — only
`emit_container` (`Box`/`Stack`) and the `HostButton` emitter did.
Fixed both:

- `emit_flex_grid`: applies `Translation`/`.Shadow` directly to the
  unwrapped `<Grid>` when the part has no other container style, or to
  the wrapping `<Border>` when it does (`Grid` is a `UIElement` like
  `Border`, so no forced wrapping is needed just for elevation).
- `emit_host_draggable`: **cannot** use the same `Translation`/
  `.Shadow` XAML attribute/property-element syntax at all — a real
  `dotnet build` probe found that combination fails XamlCompiler (exit
  code 1, no diagnostic) specifically when applied to a *custom*
  `ContentControl` subclass like `{component}MosaicDragSource` (a
  plain built-in `ContentControl` or `Border` both accept `Translation`
  as a XAML attribute fine — verified as the sanity baseline). Instead,
  a new `ElevationZ` string `DependencyProperty` was added to the
  `{component}MosaicDragSource` class template (the same shape
  `DragKey`/`DragLabel` etc. already use successfully on this exact
  class), whose property-changed callback applies `Translation`/
  `Shadow` from C# — proven to work via the same probe once the
  attribute/property-element XAML syntax was replaced with plain C#
  assignment.

Real usage now fully covered: `brand-mark` (`Stack`), six segmented-
control tab buttons (`HostButton`), `composer`/`label-composer`
(`Row`), `task-card`/`timeline-card` (`Column`), and
`board-card`/`board-card-crit` (`HostDraggable`) — TaskApp's real
project-nav "pill" toggle and Notes' selected-row button pull in the
same treatment from their own packages.

Verified against the real toolchain in two passes, matching this
cascade's established discipline: a real `mosaic-compile pkg --backend
xaml --profile native-complete` build against the real TaskApp package
confirmed `nativeComplete: true` with only the expected `inset`
box-shadow drop (the unrelated moon-icon shape hack) in
`styleDegradations`, and zero `elevation`/non-inset-`box-shadow`
degradations; then a real `mosaic-compile pkg --backend xaml
--emit-project` build of the actual TaskApp WinUI project (all 13 real
`elevation` parts, across every emitter path) ran through a real
`dotnet build` with **zero errors**.

## [Unreleased] — lower `HostProgressRing` to native `ProgressRing` (#13176)

XAML is the first backend to render the new kernel primitive
(`HostProgressRing`, registered in the prior kernel-contract PR). Lowers
to WinUI 3's native `ProgressRing` in its determinate mode
(`IsActive="True" IsIndeterminate="False" Minimum="0" Maximum="100"
Value="..."`) — the real accessible `progressbar` role, announced
value, and native visuals a `Box`+`Box` CSS conic-gradient trick can't
provide.

New `emit_host_progress_ring`, dispatched from the primitive-tag match.
Unlike `HostSlider` (which needs a component-scoped `local:
{component}MosaicSlider` subclass for its change/commit events),
`HostProgressRing` is display-only and lowers directly to the plain
control — no subclass, no generated event handlers. Reuses
`host_slider_number_attr_value` for the `value` prop's `Number`/
`SlotRef`/`Expr` handling (the same three-way binding `HostSlider`
already has), and the same `a11y-label` → `AutomationProperties.Name`
pattern. `value` is required — a missing prop is a clear compile error
(`PipelineEmitError::UnsupportedPrimitive`), not a silent default.

Narrows `mosaic-package-artifact-builder`'s `("HostProgressRing", ...)`
degradation arm to exclude `Backend::Xaml`.

Verified against the real toolchain before writing the emitter: a
`dotnet build` probe with both a literal `Value="42"` and an
`x:Bind`-bound `Value`/`AutomationProperties.Name` compiled cleanly.
After implementation, a real `mosaic-compile pkg --backend xaml
--profile native-complete` build of a `HostProgressRing` bound to a
`ring-percent-value` slot produces `nativeComplete: true` with zero
degradations, and the generated `Ring` UserControl — mounted in a real
WinUI window with `RingPercentValue` set to 42 — compiles and launches
cleanly with no crash.

## [Unreleased] — lower `Path` to real WinUI vector geometry (#12028 item 3, UI39)

XAML is the first backend to render the new kernel drawing primitive
(`Path`, registered in the prior PR). `circle`/`line`/`curve` are
implemented; `arc` is a stretch goal not included here — a real build
using `kind: arc` still hard-errors with a named "not yet supported"
message, same posture any other unimplemented shape kind gets.

New `emit_path`, dispatched from the primitive-tag match alongside every
other leaf primitive:
- `circle` → `<Ellipse Width="{2r}" Height="{2r}" Margin="{cx-r},{cy-r},0,0" HorizontalAlignment="Left" VerticalAlignment="Top" .../>`.
- `line` → `<Line X1="{x1}" Y1="{y1}" X2="{x2}" Y2="{y2}" .../>`.
- `curve` → `<Path><Path.Data><PathGeometry><PathFigure StartPoint="{x1},{y1}"><QuadraticBezierSegment Point1="{cx},{cy}" Point2="{x2},{y2}"/></PathFigure></PathGeometry></Path.Data></Path>`.

Positioning uses `Margin`/`HorizontalAlignment="Left"`/
`VerticalAlignment="Top"`, not `Canvas.Left`/`Canvas.Top` — `Stack`
lowers to `<Grid>` (`emit_stack`), not `<Canvas>`, and `Canvas.*`
attached properties are inert outside an actual `Canvas` parent. This
is the same mechanism `absolute_position_style_attrs` (#12028 item 4)
already established for `position: absolute`, and was verified
empirically via a real `dotnet build` probe project — confirmed
overlapping `Ellipse`s inside a plain `<Grid>` render as the intended
crescent-moon shape — before writing `emit_path`, catching that the
plan's original `Canvas.Left`/`Canvas.Top` assumption was wrong before
any code was written on it.

New `path_paint_attr`: `Fill`/`Stroke`/`StrokeThickness`, remapped from
the same `background`/`border-color`/`border-width` every other
primitive already authors (UI39 §3.2's zero-new-style-properties design).
Deliberately does NOT reuse `part_style_attr` — that splices
`Background`/`BorderBrush`/`BorderThickness` verbatim (real dependency
properties on `Border`/`Panel`/`Control`, but `Ellipse`/`Line`/`Path`
are `Shape`-derived and have none of the three; XamlCompiler would
reject the attribute). Mirrors `content_control_style_attr`'s
selective-remap pattern instead.

Coordinate props (`cx`/`cy`/`r`/`x1`/`y1`/etc.) accept only a literal
`Number` for now — `SlotRef`/`Expr` produce a clear compile error
naming the unsupported prop, not a silent 0 or a dropped binding.
Kernel-level bindability (UI39 §3.1) is real, but wiring it to actual
`x:Bind` XAML is XAML's own not-yet-landed UI36 (data-driven sizing)
work, tracked separately — implementing a one-off binding path just for
`Path` ahead of that would duplicate work UI36 already owns.

`mosaic-package-artifact-builder`'s `("Path", ...)` degradation arm
narrowed to exclude `Backend::Xaml` (`HostSlider`'s per-backend
narrowing pattern). SwiftUI/Qt/Flutter/Compose remain fully degraded.

Verified against the real toolchain: the exact XAML syntax above was
confirmed via a throwaway `dotnet build` probe project *before* writing
`emit_path` (catching the `Canvas.Left` mistake, see above); a real
`mosaic-compile pkg --backend xaml` build of a package authoring the
actual crescent-moon shape (two overlapping `Path` circles) produces
`nativeComplete: true, degradations: []`; a real `--emit-project` +
`dotnet build` + launch of that same package succeeds cleanly (0
errors, 0 warnings) and the window stays running. (Along the way, found
and filed #13184 — the `--emit-project` scaffold breaks for a component
with zero declared `emit`s, unrelated to `Path`, sidestepped in this
verification by adding one unused `emit`.)

## [Unreleased] — lower `box-shadow` to a `ThemeShadow` elevation token (#12028 item 1)

30 `box-shadow` declarations across `task-app`'s stylesheet — its
entire elevation system — were silently dropped, the single largest
contributor to the flat, unfinished look the epic's original writeup
flagged.

New `part_wants_theme_shadow`: a non-`inset` `box-shadow` (any value)
now gets `Translation="0,0,4"` plus a `<{Element}.Shadow><ThemeShadow/>
</{Element}.Shadow>` child — WinUI's own consistent elevation
treatment. Deliberately does NOT attempt to reproduce the authored
blur/spread/color/opacity: `ThemeShadow` is a fixed, system-composited
shadow with no CSS-shaped parameters, so every qualifying value
collapses to the same one Z-depth (Microsoft's own "Cards: 4–8px"
guidance) — a value an author might reasonably expect to vary in
*intensity* can't, on this platform, regardless of what this emitter
does. Applies uniformly to `emit_container`'s `Border`/`Grid`/
`StackPanel` shapes and `emit_host_button`'s `<Button>` (which
self-closes unless a `Shadow` child forces the open/close form).

An `inset` value (the crescent-moon/status-dot drawing hack, item 3 —
a shape cutout, not elevation) is a fundamentally different technique
and stays a genuine, reported drop.

Verified empirically against the real WinUI3 `dotnet build` toolchain
before implementing (confirming `<Border.Shadow>`/`Translation="0,0,N"`
both compile as direct XAML, despite `Translation` not being settable
via `<Setter>`), then against a real regenerated `task-app` XAML
project: 9 real elements (the brand-mark icon, every "active" pill/
segmented-switch button) now carry `ThemeShadow`, the degradation
report's only remaining `box-shadow` entry is the one genuinely-
unsupported `inset` moon hack, and `dotnet build` succeeds with 0
errors (same 98 pre-existing, unrelated warnings).

## [Unreleased] — recognize mosstyle's `align: "center-vertical"` (#13164)

Found while auditing TaskApp's remaining `styleDegradations` after
#13160: `align: "center-vertical"` (and one `align: "center"`) is
authored 30+ times across real `.msl` files (TaskApp, VentureChrome,
Calendar, ProjectNav) — always on a `Row`, always meaning "center the
cross axis" — but nothing lowered it, so every one of those rows
rendered top-aligned instead of vertically centered.

`align` is not a real mosstyle property (per `UI15-mosstyle.md`
§1/§11, alignment belongs in `.mll`; mosstyle's grammar has no property
whitelist to reject it — a separate gap, tracked in #13164 rather than
fixed here). Given every real occurrence is semantically identical to
`align-items: center`, `extract_flex_hints` now also recognizes
`align: "center-vertical"` / `align: "center"`, mapping to the exact
same `align_items = Some("center")` FlexHints slot — reusing 100% of
the existing, tested per-child `VerticalAlignment`/`HorizontalAlignment`
injection, no new mechanism.

Verified against a real regenerated `task-app` XAML project: 60
`VerticalAlignment="Center"` attributes now appear (versus rows
rendering top-aligned before), the degradation report no longer lists
`align` for any part, and `dotnet build` succeeds with 0 errors (same
98 pre-existing unrelated warnings).

## [Unreleased] — lower `position: "absolute"` to a pinned Margin (#12028 item 4)

`position: "absolute"`/`top`/`left` had no XAML setter at all and were
silently dropped — real impact: `mosaic/programs/task-app`'s bridge-arc
brand mark (three `Box`es absolutely positioned inside one `Stack`)
rendered as a plain amber square, since `Stack`'s `<Grid>` lowering
stacks all children at the same z-axis cell with zero offset, and
several inline icon compositions elsewhere in the app had the same gap.

New `absolute_position_style_attrs`: when `position: "absolute"` is
authored, `top`/`left` (each defaulting to `0` if absent) become
`Margin="{left},{top},0,0"` plus `HorizontalAlignment="Left"
VerticalAlignment="Top"`, pinned to the top-left corner of the existing
`Grid` cell — reproducing the CSS offset with no need to restructure
the parent into a `<Canvas>` (WinUI's actual `Canvas.Left`/`Canvas.Top`
attached properties only take effect inside a literal `Canvas` panel,
which every enclosing container up to the nearest common ancestor would
also need to become). Applied after the normal per-property loop, so it
overrides whatever `Margin`/alignment another authored property already
produced — `position: absolute` takes full control of placement in CSS
too. `position`/`top`/`left` are excluded from `dropped_style_properties`
(#12022) only when consumed this way; authored without `position:
"absolute"` they're meaningless in CSS too and stay reported exactly as
before.

Verified against a real regenerated `task-app` XAML project: the three
brand-mark parts (and the other icon compositions this also fixed) now
carry the expected `Margin`/alignment attrs, `dotnet build` — 0 errors
(98 pre-existing, unrelated `WMC1506` binding warnings), and the
`mosaic-degradations.json` `styleDegradations` list no longer reports
`position`/`top`/`left` for any part.

## [Unreleased] — confirm HostDialog's open-host-required degradation is permanent, not a to-do (#13008)

Investigated whether XAML's `HostDialog` could drop its "host code-behind
must call `ShowAsync()`/`Hide()`" requirement, matching SwiftUI/Qt/Compose's
dialog lowerings — none of which report an equivalent degradation, since
`.sheet(isPresented:)`, QML `Popup.visible`, and Compose's conditional
`Dialog { }` composition are all natively declarative: the `open:` slot
drives visibility directly, with no imperative call needed.

Confirmed (against current WinUI3/WinAppSDK docs, plus the multiple
open community discussions/issues asking Microsoft for exactly this)
that this is a genuine, permanent WinUI3 API gap, not something Mosaic's
architecture is missing: `ContentDialog` exposes only imperative
`ShowAsync()`/`Hide()` and has no bindable `IsOpen`-style dependency
property, unlike `Popup`/`Flyout`/`TeachingTip`, all of which do.
Lowering `HostDialog` to `Flyout` instead (which does have `IsOpen`)
was considered and rejected — `Flyout` isn't a true modal dialog (no
dimmed overlay, different dismiss semantics), so it would trade this
degradation for a wrong-primitive one rather than closing it.

No functional change. Updated the doc comment above
`build_host_dialog_attrs` and the `property.dialog-open-host-required`
degradation site in `mosaic-package-artifact-builder` to record the
finding, so it reads as a confirmed, permanent decision rather than an
open TODO. Closes #13008.

## [Unreleased] — close the remaining find_prop_value catch-all sites (#13040)

#12126 fixed seven sites where a bare `_ => {}` silently dropped a row
`Expr` value; it deliberately left ~15 more sites with the identical
shape out of scope to keep that PR reviewable. This closes all of them,
grouped by how deep the fix goes:

- **Full `Expr` support, same `lower_expr_for_xbind` pattern as #12126**
  (13 sites): `disabled:` → `IsEnabled` (negated) on `HostButton`,
  `HostCheckbox`, `HostRadio`, `HostNumberInput`, `HostSlider`;
  `checked:` → `IsChecked` on `HostCheckbox`/`HostRadio`; `read-only:` →
  `IsReadOnly` on `HostInput`; `indeterminate:` → `IsThreeState` on
  `HostCheckbox`; `HostSurface.content` → `Content` (which also had no
  `String` arm at all — an audit finding, not just the `Expr` gap);
  `Icon.glyph`/`.name` → `Glyph`; `HostRadio.group` → `GroupName`;
  `HostTable.dir` → `FlowDirection` on both the native-shape and
  non-native fallback lowering paths.

  The five negated `disabled:` sites route through a new
  `disabled_expr_xbind_path`, the `Expr` analogue of the existing
  `disabled_slot_xbind_path` — it composes a lowered expression with
  the same shared `Not(bool)` C# helper `SlotRef` already used, wrapped
  in `x:Bind Not(...), Mode=OneWay`. Inside a `For` template scope this
  returns `Unsupported` rather than guessing: WinUI's typed
  `DataTemplate` compiler rejects a function binding rooted through
  another property, and `disabled_slot_xbind_path`'s own for-scope
  branch works around that for a plain slot by projecting a new row-VM
  computed property — composing that projection with an arbitrary
  lowered expression isn't implemented or verified anywhere, so an
  `Expr`-valued `disabled:` inside a `For` gets a clear diagnostic
  instead of risking subtly wrong generated C#/XAML.

  `HostTable.dir`'s existing keyword allow-list (`rtl`/`ltr` only,
  everything else silently dropped) is a security gate against an
  *unrecognized static keyword string* reaching the XAML attribute —
  not a reason to reject `Expr` wholesale. `lower_expr_for_xbind` never
  splices attacker-influenced text directly; it always resolves to a
  compiler-generated path or helper call, so `Expr` gets the same real
  `FlowDirection` binding every other XAML-attribute site in this fix
  gets, while the keyword allow-list's own narrowness is untouched.

- **Exhaustive but `Expr` deliberately deferred, with a clear
  diagnostic instead of a silent empty payload** (2 sites):
  `host_link_href_payload_expr` (used both for `HostLink`'s in-app
  `Click` payload and `HostRadio`'s `onSelect` `value:` payload). These
  build a bare C# expression spliced into a code-behind
  `Dispatch?.Invoke(...)` call fired once at click time — a materially
  different code shape from every XAML-attribute-bind site above.
  `SlotRef` resolves via `this.<Pascal>` (component-level, no
  for-scope awareness at all); real per-row `Expr` support here would
  need the same sender-`DataContext`-cast-to-row-VM-type codegen
  `host_button_click_payload_expr` already does for its own params — a
  materially different, more novel code shape with no existing
  precedent for an arbitrary expression, and a real feature addition
  rather than a "make the match exhaustive" fix. Both functions are now
  exhaustive over every `LayoutPropValue` variant (no `_`); `Expr`
  explicitly returns `PipelineEmitError::UnsupportedExpression` rather
  than silently falling back to `""` the way it did before — an author
  who wrote an expression here deserves to know it wasn't honoured, not
  have it silently become an empty click payload.

Verified with 8 new tests plus a real `dotnet build` against a probe
project exercising the two least mechanically-obvious translations: the
`disabled:` → `Not(bool)`-helper composition (genuinely new C# codegen,
not just a new attribute) and `HostTable.dir`'s `FlowDirection` `Expr`
path. Both compiled clean, 0 warnings, 0 errors, and the emitted XAML
matched what the unit tests already asserted
(`IsEnabled="{x:Bind Not(Editable), Mode=OneWay}"` and
`FlowDirection="{x:Bind Dir, Mode=OneWay}"`).

## [Unreleased] — security: narrow CopyMosaicNativeHostLibraries from a *.dll glob to the known runtime filename (#12026)

The generated `.csproj`'s `CopyMosaicNativeHostLibraries` MSBuild target
globbed `$(MSBuildProjectDirectory)\*.dll` and copied every match next
to the built executable — a DLL-planting primitive if anything can drop
a file into the generated project directory, since the app already
loads `mosaic_app.dll` from its own directory by convention (a planted
DLL with that name would be loaded directly). Flagged during #12015's
review as adjacent to that diff, not introduced by it.

`mosaic-package-artifact-builder::install_xaml_runtime_library` is the
only code path that ever legitimately places a DLL there for this
mechanism's purpose — it `unreachable!()`s unless the file is validated
to be named exactly `mosaic_app.dll` before ever being written. Narrowed
the glob to that one exact, known filename; matches 100% of legitimate
usage with nothing left to catch. Since bundling a runtime library is
optional even with project emission on, `mosaic_app.dll` may legitimately
not exist — a literal (non-wildcard) MSBuild `Include`, unlike a glob,
makes `<Copy>` error at build time on a missing source file, so the
`<Target>` now also carries `Condition="Exists('$(MSBuildProjectDirectory)\mosaic_app.dll')"`
to skip cleanly instead.

Verified against a real `dotnet build`, three cases: (1) no
`mosaic_app.dll` present — build succeeds, target skipped, no error on
the missing source file; (2) `mosaic_app.dll` present — build succeeds
and the file is still copied to `$(OutDir)` exactly as before, no
regression to the legitimate case; (3) a differently-named planted DLL
(`evil_planted.dll`) present instead — build succeeds and the planted
file is confirmed absent from `$(OutDir)`, the concrete proof the
vulnerability is closed rather than just narrowed.

One bug caught by that same empirical verification, not by the unit
tests: the new doc comment's first draft used a literal `--` inside the
generated XML `<!-- -->` comment (illegal — XML comments cannot contain
`--` anywhere in their body), which broke MSBuild project-file parsing
entirely (`MSB4025`). Rewritten to avoid the sequence; a reminder that
generated-XML-comment content needs the same scrutiny as any other
emitted markup, not just the code around it.

Closes #12026.

## [Unreleased] — security: validate URI schemes on HostLink's NavigateUri (#12038)

`emit_host_link`'s `NavigateUri` binding had no scheme validation on
either arm: the literal `href` arm only applied `escape_xaml_attr` (XML
escaping, irrelevant to scheme safety), and the slot-bound arm bound the
runtime value directly. WinUI hands `NavigateUri` to the OS shell
launcher, so a `file:`, UNC, or registered custom-protocol target would
launch rather than open as a web link. Layout/style source is a trust
boundary (a third-party Mosaic package), so this was reachable. Reject
rather than escape, per the issue's own framing — there is no escaping
that makes `file:` safe in this position.

- **Literal `href` — rejected at compile time.** A new
  `has_allowed_uri_scheme` checks the RFC 3986 §3.1 scheme token against
  an allowlist (`http`, `https`, `mailto`); anything outside it — or a
  string with no scheme at all (a relative reference) — returns the new
  `PipelineEmitError::UnsafeUriScheme` instead of emitting `NavigateUri`.
  Confirmed via `grep` that every current `href` usage in the repo
  (`Breadcrumb`/`Nav`/`Navbar`/`Pagination` in both `mosaic-pkg-toolkit`
  and `toolkit-xaml-showcase`) is `href: "#"` paired with
  `external: false`, which routes through the *other* branch of
  `emit_host_link` and never reaches `NavigateUri` at all — this cannot
  regress any currently-shipping package.
- **Slot-bound `href` — validated host-side via a generated helper**,
  since the value isn't known until runtime. A shared `SafeNavigateUri`
  C# helper (registered once per component via the existing
  `ctx.add_helper` dedup mechanism — the same shape
  `disabled_slot_xbind_path`'s `Not(b)` helper already uses) parses the
  bound string with `Uri.TryCreate` and checks the scheme against the
  same allowlist, returning `null` for anything that fails either check.
  `NavigateUri` bound to `null` means the button simply doesn't navigate
  on click — no new `Click` handler, no manual `Launcher.LaunchUriAsync`
  reimplementation, smallest change that closes the runtime-bound gap.
  Verified against a real `dotnet build` (not just Rust-level string
  assertions): the generated single-expression method body uses an
  inline `out var` inside a boolean condition (`Uri.TryCreate(raw,
  UriKind.Absolute, out var u) && (u.Scheme == "http" || ...) ? u :
  null`), required because `HelperMethod` bodies are emitted as C#
  expression-bodied methods, not statement blocks — build succeeded, 0
  warnings, 0 errors.
- **Literal-arm hardening from the security review of this fix itself.**
  The review flagged that checking only the scheme token leaves the
  literal arm weaker than the slot-bound arm: the same string is parsed
  *twice*, independently, by two different parsers — this hand-rolled
  Rust check at compile time, and .NET's own `Uri`/`UriTypeConverter`
  again at XAML-load time. A string like `href: "http:evil"` has an
  allowed scheme but isn't a real hierarchical URI, so a scheme-only
  check would let it through here only for .NET's independent parse of
  the same string to throw `UriFormatException` when the app loads —
  trading "rejected at compile time" for "crashes on click". Since
  `http`/`https` are hierarchical schemes (RFC 3986 §3 — a scheme with an
  authority always has a `//`-prefixed `hier-part`, and the authority
  can't be empty), `has_allowed_uri_scheme` now also requires a
  `//`-prefixed, *non-empty* authority token immediately after the
  scheme name for those two; `mailto` (RFC 6068's non-hierarchical
  `mailto:mailbox` form, never `//`-prefixed) is exempt from this check.
  A second review round on this exact hardening caught that the first
  version (`//` present, but not checking for an empty authority) still
  let `http://`, `https://` and `http:///path` through — `new
  Uri("http://")` throws in .NET the same way `new Uri("http:evil")`
  does, so those needed the same fix. No bypass of the scheme allowlist
  itself was found by either review round for either arm — this narrows
  a real "malformed input crashes the app instead of failing the build"
  gap; it deliberately doesn't chase full parity with .NET's `Uri`
  grammar (an authority token that's present but itself malformed, e.g.
  a bare space, can still reach .NET's independent parse unrejected).

Checked the other 7 backend emitter crates
(`mosaic-emit-{compose,flutter,html,qt,react,swiftui,webcomponent}`) —
all of them handle `href` in their own `HostLink`-equivalent lowering,
and none validate the scheme either. Filed as a follow-up
([#13052](https://github.com/adhithyan15/coding-adventures/issues/13052))
rather than fixed here — each backend's navigation API has a different
codegen shape, so bundling all 7 would have made this PR much harder to
review as a single, contained fix.

Three new tests: a disallowed-scheme table (`file:///...`,
`ms-appx-web:///...`, a UNC path, `javascript:...`, a scheme-less
string, `http:evil`/`https:not-a-real-authority` from round one of the
hardening, and `http://`/`https://`/`http:///path`/`http://?x`/
`http://#frag` from round two) all reject with `UnsafeUriScheme`; an
allowed-scheme table
(`http`, `https`, `mailto`, plus a case-insensitive `HTTPS://` variant)
all still emit `NavigateUri` unchanged; a slot-bound href binds through
`SafeNavigateUri` and the helper itself is present in the generated
code-behind. All 241 pre-existing tests pass unchanged (the one
pre-existing `NavigateUri="https://example.com"` test is unaffected —
`https` is on the allowlist).

Closes #12038.

## [Unreleased] — fix seven more silent Expr-drop sites, make their matches exhaustive (#12126)

Continuation of the `label`-match `Expr`-drop bug fixed twice already
(#12045 on `HostButton`, #12121 on `HostCheckbox`/`HostRadio`/`HostLink`):
a `find_prop_value(node, "...")` match ending in a bare `_ => {}` silently
swallows `LayoutPropValue::Expr` (e.g. `text: ( row[1] )`), so a
row-expression-valued prop emits no attribute at all and the control
renders with that attribute simply absent — invisible rather than
obviously broken.

Seven more sites had the identical shape:

- `HostTooltip.text` (`ToolTipService.ToolTip`)
- `HostInput.value` (`Text`, was `Mode=TwoWay` for `SlotRef` — the new
  `Expr` arm is `Mode=OneWay`; see below)
- `HostInput.placeholder` (`PlaceholderText`) — this one was an `if let`
  handling only `String`, not even a `match`; converting it to an
  exhaustive `match` for the `Expr` fix also surfaced and fixed a missing
  `SlotRef` arm found along the way (a slot-valued placeholder silently
  emitted nothing either, same failure mode, different variant)
- `HostNumberInput.value` (`Value`, same `TwoWay`→`OneWay` reasoning)
- `HostDialog.title` (`Title`, shared by both `emit_host_dialog` and
  `emit_host_dialog_as_root`)
- `a11y-label` on `Text` and `HostSlider` (`AutomationProperties.Name`)
- `Image.src` (`Source`)

All seven now route through the same `lower_expr_for_xbind` helper the
already-fixed `label` sites use, and every one of the seven matches is
now **exhaustive** over `LayoutPropValue`'s six variants (no `_`
anywhere) — mirroring `emit_text`'s `content` match, the one site in the
file already written this way. A future 7th `LayoutPropValue` variant is
now a compiler error at these sites, not silent runtime blankness.

**`TwoWay` → `OneWay` for the `Expr` arm at the two two-way sites.**
`Mode=TwoWay` needs an assignable target for user-edit writeback. A
`row[1]`-style indexer lowers to `ExprLowering::Helper`, a C# **method
call** — not an lvalue — so `x:Bind Expr_xxx(Row), Mode=TwoWay` would not
compile. Every other `Expr`-arm precedent in the file already uses
`OneWay` for the same reason, regardless of what the target property
allows for a plain `SlotRef`.

**`HostNumberInput.value` (`double`) and `Image.src` (`ImageSource`)
bind a `string`-returning helper — verified against a real build, not
assumed.** `lower_expr_for_xbind`'s `Helper` case always returns a C#
`string` (indexing into a `list<list<text>>` row yields a `string` cell).
Compiled a probe `.mil`/`.mll`/`.msl` component through the real
`mosaic-compile --backend xaml --emit-project` → `dotnet build` pipeline
with both an `Expr`-valued `HostNumberInput.value` and `Image.src` inside
a `For`: **build succeeded**, with only the expected `WMC1506` warning
(the same "OneWay binding step can't itself raise notifications" warning
every other `Expr`-arm site already produces, benign since row-VM
rebuilds re-evaluate it regardless — see the existing `Text.content` Expr
arm's own comment). Confirms WinUI's compiled-binding implicit
`string`→`double` and `string`→`ImageSource` conversions apply to a
`Helper`-lowered method-call binding target, not just a plain property
path — no special-cased numeric-returning helper variant was needed.

Two new tests: `row_expression_valued_props_bind_at_every_remaining_drop_site`
covers all seven sites in one layout (a `For` over `list<list<text>>`
rows, each control's target prop set to `( row[1] )`), asserting each
attribute lowers to an `x:Bind` rather than being omitted, plus a count
assertion that both `a11y-label` sites (`Text` and `HostSlider`) fired
independently since they share a target attribute name.
`host_input_placeholder_binds_slot_ref` covers the incidental `SlotRef`
gap found while fixing the `Expr` gap at the same site. All 236
pre-existing tests still pass.

Filed a follow-up issue (#13040) for ~15 more `find_prop_value` sites with
the same bare-catch-all shape found by the survey that scoped this fix,
deliberately left out of this PR — most aren't specifically about `Expr`
(`disabled`/`checked`/`indeterminate` need boolean-`Keyword` handling,
`group` needs its own semantic decision, table `dir` is an intentional
allow-list), so bundling them would have tripled this diff without adding
review value.

Closes #12126.

## [Unreleased] — security: XML-escape style-fragment values at every attribute sink (#12025)

`build_style_fragment_with_drops` "escaped" mosstyle values with C-string
escaping (`\`→`\\`, `"`→`\"`) — not XML escaping. `parse_style_fragment`,
the sole reader used by every downstream consumer, then **stripped that
backslash-escaping back out** before handing values to callers, so by the
time a value reached a real `key="value"` XAML attribute it was completely
unescaped. `mosstyle-compiler`'s token validation only rejects
`{ } ; NUL CR LF` — `"`, `<`, `>`, `&`, `=` all pass through, so a hostile
package token (`x" Foo="bar`) could inject attributes into generated XAML.
Found during #12015's review; disclosed there since practical
exploitability is low today (no third-party Mosaic package registry
exists yet — no distribution channel).

Fixed at the single production write path rather than at each of the 8+
downstream call sites the issue's own writeup names: `build_style_fragment_with_drops`
now calls the already-correct `escape_xaml_attr` (the same helper the
`<Setter Value="...">` path has always used) instead of the C-string
escape, and `parse_style_fragment`'s now-vestigial backslash-unescaping
branch is removed — a value can no longer contain a literal `"` at all
once it's escaped at the source, so every consumer (the five
`parse_style_fragment` callers, and `part_style_attr`'s whole-fragment raw
splice used by 17 more call sites) is correct with zero further changes.
Same end state as the issue's prescribed per-sink fix, less code, and
structurally impossible to miss a site since there's only one producer.

New tests cover all six architecturally distinct consumption paths with a
value carrying all of `" < > &`: a plain `Box` (`partition_box_style`), a
`Row` (`partition_flex_grid_style`, #12021's `<Grid>` lowering), a
`HostButton` (`content_control_style_attr`), `Image`
(`part_style_attr`'s raw-splice path), and direct unit tests for
`drag_control_style_attr` (`HostDraggable`/`HostDropTarget`) and
`partition_stack_panel_style` (still used by `HostTable`'s row-section
emitter) — each asserts the escaped form appears and the raw hostile value
never does. A seventh test confirms a literal backslash (not
XML-significant) still round-trips unchanged now
that the backslash-specific escape/unescape scheme is gone. All 229
pre-existing tests pass unchanged, since escaping ordinary values (none of
which contain `"<>&`) is a no-op.

`normalize_xaml_color_value`'s narrower #12015 guard (drops any color
value containing `"<>&` outright) is unchanged — now redundant-but-harmless
defense-in-depth specifically for colors, comment updated to say so.

## [Unreleased] — `dropped_style_properties`: make silently-discarded style properties visible (#12022)

`build_style_fragment` has two points where a mosstyle property produces no
XAML output at all — an unrecognised property name, or a value that can't
be translated (a non-100% percentage, an unsupported CSS unit, …) — and
both were a bare `continue` with zero record. New public
`pub fn dropped_style_properties(style: &StyleDef) -> Vec<DroppedStyleProperty>`
(`mosaic-emit-xaml::pipeline`) makes both visible: one entry per dropped
`(part, property, value, reason)`, read by `mosaic-package-artifact-builder`'s
degradation analyzer (issue #12022) the same way it already reads
`host_table_has_native_semantics` and friends for capability-level gaps.

Pure reporting — no change to what XAML is emitted. `build_style_fragment`
itself is now a thin wrapper over `build_style_fragment_with_drops`, so its
existing signature and ~15 test call sites are untouched.

Excludes `align-items`/`justify-content` when their value is one `FlexHints`
(#12980) already consumes through its own side channel outside this
function (`"center"` / `"space-between"` respectively — any other value IS
still reported, since nothing consumes it), and excludes `flex-grow`
unconditionally (fully boolean-handled today, nothing recognisable as
"lost"). Verified this doesn't false-positive by regenerating the
package-expanded TaskApp: `align-items: center` and
`justify-content: space-between` (both authored) produce zero drop entries;
166 *other* properties do (see `mosaic-package-artifact-builder`'s
CHANGELOG for the fuller picture — this crate only supplies the detector,
the builder crate decides what to do with it).

New tests: a genuinely-dropped property (`box-shadow`) is reported with a
specific reason; the two flex exclusions are covered both ways (recognised
value → not reported, unrecognised value → reported); a non-100%
percentage width (the issue's own motivating example) is reported; an
unknown/typo'd property name falls through to a generic reason rather than
being silently ignored.

## [Unreleased] — lower `Row`/`Column` to `Grid` with flex sizing (#12021)

`StackPanel` sizes to content and has no concept of distributing free space.
Because `Row`/`Column` lowered to it, the generated app occupied roughly the
top-left third of the window with the rest empty, and `flex-grow`,
`justify-content`, `align-items`, and a main-axis `width: 100%` were all
silently dropped — none of them are `StackPanel` limitations, `Grid` with
row/column definitions, star sizing, and child alignment does all of it.

`Row` now lowers to `<Grid>` + one `ColumnDefinition` per child slot;
`Column` lowers to `<Grid>` + one `RowDefinition`. Each child gets a matching
`Grid.Column`/`Grid.Row` attached property. An `If`/`Else` pair is one
logical slot even though it emits two sibling `<ContentControl>`s (§6.2) —
both now carry the same index, since only one is ever visible at a time.

Scoped to what's actually authored anywhere in the repo today rather than a
general CSS flexbox engine (see `mosaic-emit-xaml.md` §3.1 for the full
writeup and what's deliberately deferred):

- `flex-grow` (only `1` is authored, 12 sites) and a main-axis `width`/
  `height: 100%` (flexbox's own "claim the remaining space", treated the
  same) → that child's definition becomes `"*"` instead of `Auto`.
- `align-items: center` (2 sites) → `VerticalAlignment`/`HorizontalAlignment`
  = `Center` on every child, injected the same way as the Grid position.
- `justify-content: space-between` (1 site) → a `"*"` spacer definition
  between each pair of children (N children, N−1 spacers).
- `gap` keeps mapping to the same `Spacing`-shaped value, but the *attribute
  name* becomes `Grid.ColumnSpacing`/`Grid.RowSpacing` (`<Grid>` has no
  `Spacing` property; both were added to `Grid` in Windows App SDK 1.3+,
  inside this backend's pinned 1.5 floor).
- Everything else (`flex-wrap` — no WinUI 3 `WrapPanel` — weighted
  `flex-grow`, `align-items: flex-end`/`baseline`, other `justify-content`
  values) is unchanged/dropped, not guessed at.

Verified live: generated the TaskApp, built and launched it, and confirmed
via `PrintWindow` capture that content now fills the 1920×1015 window with
the rail/topbar/view-switcher taking proportional widths instead of
overlapping in the top-left corner. The functional UI-Automation smoke test
(`code/scripts/taskapp-xaml-smoke.ps1`) still passes: the app launches, a
dispatched "add task" event updates the rendered summary, and the new row's
name renders. `mosaic-degradations.json` stays `nativeComplete: true` with
zero degradations.

Wide golden-string churn as expected (the issue's own estimate): 6 existing
tests asserted literal `<StackPanel Orientation=...>` output for `Row`/
`Column` and needed updating to the new `<Grid>` shape; none were logic
bugs. 4 new tests cover the flex-grow star column, the main-axis 100% case,
`justify-content: space-between` spacer insertion, and the If/Else
shared-index case.

## [Unreleased] — emit Content for Checkbox, Radio and Link expression labels

#12045 fixed a missing `LayoutPropValue::Expr` arm in `emit_host_button`'s
`label` match, where the trailing `_ => {}` silently swallowed an expression
label and the button emitted no `Content` attribute at all. It noted, without
fixing, that the sibling match in the `HostCheckbox` / `HostRadio` lowering had
the same shape.

It does, and it reproduces. A `HostCheckbox` and a `HostRadio` inside a `For`,
labelled `label: ( row[1] )`, emitted:

    <CheckBox x:Name="HostCheckbox_2"/>
    <RadioButton x:Name="HostRadio_3"/>

— no `Content`, so both render blank, while the already-fixed `Button` beside
them emitted its binding correctly. Both now route through
`lower_expr_for_xbind`, exactly as `emit_host_button` does.

Auditing the rest of the file for the same shape turned up a third instance:
`emit_host_link`'s `label`. Its symptom differed only because its catch-all is
not empty — it falls back to `href`, so an expression label rendered the raw
URL with a string `href` and rendered blank with a slot or expression `href`.
An explicit label now wins. Fixed here because it is the same attribute, the
same binding mode and the same helper; no judgement call was involved.

`host_button_with_row_expression_label_emits_content` is generalised (and
renamed `labelled_hosts_with_row_expression_label_emit_content`) rather than
duplicated. Its useful half — that no emitted content control lacks `Content` —
now covers `CheckBox`, `RadioButton` and `HyperlinkButton` alongside `Button`,
so the next element to grow a label match without an `Expr` arm trips it.

No golden churn: no current layout binds these labels to an expression, so
emitted output for every existing fixture is byte-identical. This is a latent
bug closed before it shipped, not a live one repaired.

Left open deliberately (recorded on #12047): seven further `find_prop_value`
matches in this file end in a bare `_ => {}` that swallows `Expr` the same way
— `HostTooltip.text`, `HostInput.value` and `.placeholder`,
`HostNumberInput.value`, `HostDialog.title`, `a11y-label` on `Text` and
`HostSlider`, and `Image.src`. All were confirmed to drop the expression, but
unlike the label sites they are not mechanically identical: they need per-site
decisions about the target attribute, the binding mode (`value` wants TwoWay,
not OneWay) and type conversion (`Image.src` needs a `string`→`ImageSource`
step). They are not guessed at here.

The structural cause is worth naming: the `Text` `content` match handles the
same ten variants but ends `Some(LayoutPropValue::EmitRef(_)) | None => …`,
which is exhaustive. Adding a variant to `LayoutPropValue` would be a compile
error there and silence at every `_ => {}` site. That is why this class of bug
keeps recurring in exactly these matches and never in that one.

## [Unreleased] — fix double-encoded characters in the generated host

Every generated WinUI app displayed a mojibake title bar:

    TaskApp — Mosaic → XAML demo

The em dash and arrow were double-encoded **in the emitter's own source
literal** — UTF-8 bytes reinterpreted as Latin-1 and re-encoded — so the
corruption shipped to every consumer. The status text carried the same defect
in its ellipsis (`waiting for dispatch…`).

Four literals fixed across both `RootShape` variants. Guarded by an assertion
on the generated `MainWindow.xaml` that it contains real U+2014/U+2192 and no
U+00E2 — the tell-tale of a Latin-1 round trip.

Verified on the running app: the live window title's non-ASCII code points are
now exactly `U+2014` and `U+2192`.

Worth noting for anyone re-checking this: `Get-Content` and many terminals
default to ANSI and will *display* correct UTF-8 as mojibake. Both the earlier
misdiagnosis and the verification here needed a byte-level or code-point-level
check, not a visual one.
## [Unreleased] — reject CSS units XAML cannot parse

The length path stripped `px` and rejected `%`, but every other CSS unit fell
straight through into the emitted attribute. The generated TaskApp shipped

    <StackPanel Orientation="Horizontal" MinHeight="100vh">

`vh` is a CSS viewport unit; WinUI lengths are `Double`, so that value is
unparseable. It was silent at build time.

Two changes:

- `100vh` / `100vw` on a size setter now lower to `VerticalAlignment="Stretch"`
  / `HorizontalAlignment="Stretch"`. In a desktop app the window is the
  viewport, so "fill the viewport" and "fill the parent" coincide.
- Any other unparseable unit (`em`, `rem`, `ch`, `pt`, fractional `vh`) is
  refused rather than emitted, so an element is sized by its parent instead of
  carrying an attribute the runtime cannot read.

**This does not fix the app's layout.** Verified by screenshot before and
after: identical. XAML was evidently already discarding the bad value, so
removing it changed nothing visible. It is a correctness fix — no invalid
attribute in generated output — not the fix for the window-filling symptom,
which remains open.

## [Unreleased] — width/height 100% become stretch alignments

`width: 100%` is a *sizing* property in CSS but an *alignment* in XAML: WinUI's
`Width` is an absolute `Double` with no percentage form, and the way an element
fills its parent's cross axis is `HorizontalAlignment="Stretch"`.

Value translation alone could not express that, so the property was dropped
outright and the element fell back to sizing itself to its content.

Deliberately narrow: only `100%` maps this cleanly. Other percentages need
proportional (star) sizing, which is a `Grid` change of a different magnitude —
those still drop rather than being approximated.

In the generated TaskApp this turns 0 stretch alignments into 14. Visible
effect is real but modest — the project rail rows now fill their column instead
of collapsing to a sliver. It does **not** fix the app hugging the top-left of
an empty window; that needs the flex→Grid lowering.

## [Unreleased] — HostButton labels from row expressions

A `HostButton` whose `label` is a row expression emitted **no `Content`
attribute at all**, so the button rendered blank.

The `label` match handled `SlotRef`, `String` and `Keyword`; `label: ( row[1] )`
parses as `LayoutPropValue::Expr`, which had no arm, and the trailing `_ => {}`
swallowed it silently. Adding the `Expr` arm routes it through
`lower_expr_for_xbind` — the same helper the `Text` lowering already used
successfully two elements away inside the same template.

This was not a binding-mode problem. There was no attribute for a mode to
apply to.

**Scope.** Every `HostButton` inside a `For`. In the generated TaskApp that
meant the task name, the completion toggle, every project-rail row and every
notes row rendered as empty buttons. It read as a styling gap for weeks
because an empty button is invisible rather than obviously broken.

Verified live: the UI Automation tree went from listing only `Delete` for a
task row to listing `[○]`, `[Ship the XAML fix]` and `[Delete]`, and the
project rail from a blank row to `[Inbox]`.

`host_button_with_row_expression_label_emits_content` guards it, asserting
both that the binding is emitted and — more generally — that no emitted
`<Button>` lacks `Content`.

## [Unreleased] — every x:Bind declares its mode

`x:Bind` defaults to **OneTime** in WinUI, and each emission site chose its
binding mode by hand, so coverage drifted. In the generated TaskApp that left
118 of 153 bindings frozen after first render: every event reached the Rust
engine and the engine computed correctly, but none of it reached the screen.

A previous change fixed `Text=` and `AutomationProperties.Name=`. Every
remaining site now emits `Mode=OneWay`:

- **`Visibility=`** on the `If` lowering — the one that mattered most, since it
  pinned every conditional surface to its first-render value and made view
  switching a no-op
- `Content=` (7 sites, including the two literal `Content="{x:Bind Index}"`
  forms), `Source=`, `Glyph=`, `IsReadOnly=`, `GroupName=`, `NavigateUri=`,
  `ToolTipService.ToolTip=`
- the UI31 table cell/header attributes (`Row=`, `Column=`, `Header=`,
  `Value=`, `AutomationProperties.Name=`)
- the GROUP C `Width=` injection, whose own doc comment already claimed a mode
  it did not emit
- `ItemsRepeater.ItemsSource`, where the mode was conditional on there being a
  projection property — so a repeater bound directly to a slot got the
  OneTime default and never re-rendered when its list changed

