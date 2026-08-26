# Changelog — mosaic-emit-xaml

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

    TaskApp â€” Mosaic â†’ XAML demo

The em dash and arrow were double-encoded **in the emitter's own source
literal** — UTF-8 bytes reinterpreted as Latin-1 and re-encoded — so the
corruption shipped to every consumer. The status text carried the same defect
in its ellipsis (`waiting for dispatchâ€¦`).

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

### The guard

`every_xbind_emission_site_declares_its_mode` scans **this file's own source**
and fails on any emission site whose `x:Bind` carries no `Mode`, naming each
offender by line.

Scanning source rather than emitted output is deliberate, and was learned the
hard way. The first version of this test compiled a fixture and scanned the
resulting XAML — but a fixture only reaches the sites it happens to exercise.
It used a `Text` and an `If`, both already correct, so it passed while roughly
twenty sites were still emitting mode-less bindings. A fixture-based test would
also have gone stale the moment someone added a site it did not cover, which is
exactly how the original defect arose.

It handles both emission forms — the doubled-brace `format!` form and the
literal single-brace form used by `inject_attr_into_first_element` — and
brace-matches rather than stopping at the first `}}`, so a binding carrying a
nested `Converter={StaticResource …}` parses correctly. It scans only up to
`#[cfg(test)]`, since tests legitimately pass mode-less bindings as *input*.

Verified by regression: reverting a single site to the mode-less form makes the
test fail and name that binding.

## The actual deliverable

`every_emitted_xbind_declares_its_mode` scans emitted XAML and fails on any
`{x:Bind …}` without an explicit `Mode`, naming the offenders. It brace-matches
rather than scanning to the first `}`, so a binding carrying a nested
`Converter={StaticResource …}` is parsed correctly.

Verified by regression: reverting a single site to the mode-less form makes the
test fail and name that binding. Without that check the test would be
decoration — a per-site fix list goes stale the moment a new emission site is
added, which is exactly how this defect arose.

## [Unreleased] — accessible HostSlider names

Literal and slot-backed `HostSlider.a11y-label` values now lower to
`AutomationProperties.Name` on the native WinUI slider while retaining its
RangeValue automation pattern.

## [Unreleased] — native HostSlider

`HostSlider` now lowers to a component-scoped WinUI `Slider`, retaining native
range-value UI Automation, touch/pointer input, keyboard controls, platform
theming, and high-contrast behavior. Generated lifecycle tracking dispatches
continuous user changes and exact pointer/key/blur commits without app-owned C#
glue. Positive steps use native snapping; `step: 0` uses sub-pixel pointer
granularity with a practical keyboard increment.

## [Unreleased] — portable Text accessibility

`Text` now emits UI Automation names, heading levels, and raw-view hiding from
Mosaic accessibility metadata, including live `x:Bind` accessible-name slots.

## [Unreleased] — native WinUI drag and drop

`HostDraggable` and `HostDropTarget` now lower to component-scoped WinUI
controls backed by native pointer/touch drag events and equivalent keyboard
operation. Generated code applies authored acceptance and disabled-state rules,
keeps repeated/nested component scopes isolated, sends every lifecycle event
through the existing MIL dispatcher, and exposes focus, names, help, and live
announcements through UI Automation.

## [Unreleased] — native UI Automation table semantics

Canonical dynamic UI31 tables now emit component-scoped WinUI table, header,
and cell controls. Their automation peers implement the native UIA Table/Grid
and TableItem/GridItem provider patterns, publish dimensions and column-header
associations, preserve the authored interactive cell subtree, and support
arrow-key movement between realized cells. Unsupported or ambiguous table
shapes keep the existing structural Grid rendering instead of overstating their
accessibility contract.

## [Unreleased] — native-complete runtime-required shell

`EmitOptions::require_runtime` now emits a direct standard-runtime WinUI host.
It loads Rust before window activation, validates required MIL props before the
component is shown, sends every Mosaic event to Rust, and omits reflection-host,
sample-prop, and app-owned dispatch fallbacks. The default permissive shell is
unchanged.

## [Unreleased] — VC2-xaml Grid: WinUI value translation + nested-For + per-column widths

### Fixed - Serviced Windows App Runtime

Generated WinUI projects now pin Windows App SDK `1.8.260710003`, its required
Windows SDK BuildTools `10.0.26100.4654`, and the matching 1.8 framework
libraries. The Windows App SDK is bundled self-contained, removing the
system-wide runtime installation prerequisite and insulating generated apps
from machine runtime registration failures.

### Fixed - Typed-template helper ownership

Generated `For` row view models now retain their owning component and expose
Mosaic expressions as ordinary computed row properties. Those properties
delegate to assembly-local component helpers in C#, so WinUI's compiled
binding engine never has to resolve a page method from a typed DataTemplate.
Nested loops are projected by their enclosing row VM and capture outer element
and index bindings, while component-slot bindings route through the retained
owner. This keeps flat-list filters, nested grid cells, and editor state in the
correct typed scope without application-specific XAML glue.
Generated nested grid projections also zip authored `column-widths` into each
cell VM and invalidate outer projections when nested sources or widths change,
preserving both visible geometry and live runtime updates.
The shared visibility converter now applies Mosaic truthiness to booleans,
numbers, text, and collections, so string-backed list fields drive `If` blocks
the same way they do on the other native backends.

### Fixed - Valid styled text composition

Text primitives now split MSL box paint from typography. Backgrounds, borders,
corner radii, padding, and sizing render on a native `Border`, while foreground
and font properties remain on the nested `TextBlock`. This preserves pill/chip
styling without sending unsupported `CornerRadius` or `Background` attributes
to WinUI's `TextBlock` markup compiler.

MSL `text-align` on `HostButton` now maps to WinUI's native
`HorizontalContentAlignment` property instead of the unsupported
`Button.TextAlignment` attribute.

### Fixed - Collision-safe repeated-loop projections

Each `For` loop now receives a distinct generated row-view-model and projection
name when a package-expanded component reuses an `as:` alias. The first alias
keeps its stable historical name and later loops receive numeric suffixes, so
TaskApp's sheet and task-list `row` loops no longer share the wrong collection.

### Fixed - Native input commit payloads

`HostInput.onCommit` and `onCancel` handlers now inspect the authored MIL emit
schema. Void events remain parameterless, while single text, number, or boolean
events receive the native `TextBox` value with the required conversion. Complete
TaskApp generation therefore constructs `SheetEditCommit(value: text)` correctly
instead of emitting code-behind that cannot compile.

### Fixed - Generated WinUI SDK selection

Complete project emission now includes a `global.json` that selects the .NET 9
SDK family targeted by the generated project. The generated build script also
builds from the project directory, so machines with .NET 10 installed globally
do not accidentally run the Windows App SDK 1.7 XAML compiler under an
unsupported newer SDK toolchain.

### Fixed - Native multiline Input compatibility

The still-supported UI25 `Input` primitive now shares the complete native
`TextBox` lowering, including `AcceptsReturn`, text wrapping, dispatch, and
automation identity. Trestle Notes can therefore compile to WinUI without
losing its multiline body editor.

### Fixed - Live native disabled state

Slot-backed `disabled` properties now lower to one-way WinUI `IsEnabled`
bindings. Generated buttons and inputs therefore observe runtime Mosaic state
changes instead of retaining the value captured when their XAML first loads.

### Added - optional generated-shell interaction acceptance

Generated WinUI applications now invoke an optional package-host interaction
hook after wiring the Mosaic component's dispatch event. Package owners can
exercise emitted native controls and shared dispatch in direct launch
acceptance without adding application-specific behavior to the shell.

### Added - Native automation identifiers for authored controls

`HostInput` and `HostButton` now preserve their MLL part names as WinUI
`AutomationProperties.AutomationId` values. Generated applications can locate
the same Mosaic-authored control deterministically for accessibility and direct
native interaction acceptance without adding a parallel Win32 control tree.

### Added - HostSurface native composition

WinUI output now lowers Mosaic `HostSurface ( content: slot: ... )` to a
`ContentPresenter` bound to the host-supplied `UIElement`, wrapped by the
shared MSL-styled `Border`. This gives Direct2D and other native renderers a
typed mount point inside Mosaic-authored application chrome.

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

Documents file roles, self-contained Windows App SDK deployment, the expected
cosmetic MSB4062 error, and the build/run commands.

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
- `<WindowsAppSDKSelfContained>true</WindowsAppSDKSelfContained>` bundles the
  pinned 1.8 WinUI framework libraries so generated hosts do not depend on a
  separately registered machine runtime. The .NET runtime remains
  framework-dependent.

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
