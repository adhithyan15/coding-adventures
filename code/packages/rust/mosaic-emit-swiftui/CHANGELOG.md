# Changelog

## 2026-09-13

- Project numeric typography through a stable font modifier on Text, HostInput, HostButton and HostTable. Invalid live values keep the inherited/authored font; monospaced design flows through resolved descendants. Add a macOS test that type-checks two generated components together.

All notable changes to this package will be documented in this file.

## [Unreleased]

### Fixed -- an empty run-time accessible name made VoiceOver read just "Button" (#15427)

An accessible name known only at run time can be empty, and an empty override is not the same as no override. `.accessibilityLabel(Text(""))` *replaces* the
button's label. Slot, keyword and expression names now go through a
generated `_mosaicA11yName(name, fallback: label)`, which speaks the visible
label when the name is empty (the same idea the Icon lowering already used).
An empty literal is treated as no name.

### Fixed -- the CSS `border` shorthand was dropped whole (#15272)

SwiftUI read `border-width`, `border-color` and `border-style`, and read
`border` **zero times**. An authored `border: 1px solid #32463b` was
therefore a total loss: no width, no colour, no border.

VisiCalc is the one product that authors the shorthand, and the loss was
visible in the emitted Swift. Measured with a single command per package,
`mosaic-compile pkg --backend swiftui --profile permissive`:

| package         | `.overlay(` before | after |
| --------------- | -----------------: | ----: |
| visicalc        |                  0 |    12 |
| task-app        |                104 |   104 |
| engram-app      |                240 |   240 |
| venture-browser |                 16 |    16 |

VisiCalc rendered on SwiftUI with no cell borders at all. The other three
products' generated Swift is **byte-identical** before and after
(`diff -rq`), so this reaches only the shorthand it set out to fix.

`expand_border_shorthand` (ported from the Qt fix, #15255) desugars the
shorthand into longhands in `build_part_style_map`, before any lowering
runs. The drop reporter walks the desugared props too, so the report and
the emitter keep reading one value and cannot drift.

Every new border carries its authored colour -- `#32463b` lowers to
`Color(red: 0.196, green: 0.275, blue: 0.231)` -- with no occurrences of
the `Color.gray` fallback among them.

Expansion is **all-or-nothing**: a token this emitter cannot place as a
width, a stroke keyword or a colour `swiftui_color_value` accepts
abandons the expansion and leaves the shorthand alone, so the loss stays
reported. Security review of the first version caught the alternative --
expanding what parsed and discarding the rest turned one loud loss into
two silent ones, since `border: 1px solid notacolour` would have rendered
a border in fallback grey *and* vanished from the drop report.

**What this does not do.** It does not add the `border-style` longhand:
`dashed` and `dotted` are still unrendered and still reported. It does
not touch the per-edge shorthands (`border-top` and friends), which
remain unread. And it does not change how any style drop is reported for
the five product programs -- their degradation reports carry no
`style.property-dropped` entries on this path at all.

### Fixed -- an unresolvable colour was painted invisible instead of dropped (#15141)

`swiftui_color_value` was total, falling back to `Color.clear`.

That answer compiles, renders, and what it renders is **invisible**, so the
mistake surfaces as "the text disappeared" a long way from the authored
value that caused it. Two ways in, both real:

- **`color: inherit`.** A CSS-wide keyword and a reasonable thing to
  author. It produced a `Color.clear` foreground. Found by reading emitted source while
  fixing #15048 -- no test failed and no degradation was reported, because
  as far as the emitter was concerned it had produced a valid colour.
- **Any unrecognised colour name.** `rebeccapurple`, a typo, a design-token
  name that did not resolve -- all silently invisible.

`swiftui_color_value` now returns `None` for a value it cannot resolve, and each caller
keeps whatever it already had. For text that means the inherited style:
unstyled rather than invisible. This matches how `px_or_none` has always
handled lengths in this file. `transparent` stays a real answer -- an author
asking for nothing painted still gets nothing painted; only the catch-all
is gone.

**Product impact, measured by diffing emitted output before and after
across task-app, visicalc and engram-app.** Only task-app changes, and only by dropping `_mosaicBackground(.., Color.clear)` calls that painted nothing. They came from `background: "currentColor"` on the status dot, which SwiftUI has no lowering for -- invisible before, invisible after. Identical rendering, honest source. The underlying product defect is filed separately.

A unit test asserted the old behaviour directly (`swiftui_color_value("rebeccapurple") == "Color.clear"`); it now pins the opposite, because that fallback was the bug rather than a feature.

### Fixed — `gap` was reported as dropped while the container applied it

`dropped_style_properties` derived its answer by running the modifier chain and
collecting what no match arm handled. That is the right method for almost every
property, and wrong for `gap`: SwiftUI takes spacing at view-**construction**
time — `HStack(spacing:)`, `VStack(spacing:)` — and `container_spacing` has been
reading it from the part's own style for some time. The modifier scan cannot see
that, so it called every applied gap a drop.

**22 of the 40 SwiftUI style drops reported for Engram were this**, and all 22
were false. `$style.app-shell` was reported to drop `gap: 18` while the emitted
Swift opened `VStack(spacing: 18)` on that very part. After the fix Engram
reports 18 drops and no `gap` at all.

`dropped_style_properties` now takes the layout, the same way the Compose
reporter does for `justify-content`/`align-items` (#14834), and asks
`container_spacing` rather than re-deriving the rule. `gap_consuming_parts`
mirrors the emitter's tag-to-view mapping:

- `Column` → `VStack` and `Row` → `HStack` consult `container_spacing`, so their
  gap lands and is no longer reported.
- `Box` → `Group`, `Stack` → `ZStack` and `HostScroll` → `ScrollView` do not.
  `container_spacing` refuses them by name, deliberately: a `ZStack` overlays
  along the depth axis and a `ScrollView` delegates layout to its content. Those
  drops are still reported.
- A `Row` **inside** a `HostTable` lowers through `container_table_row` to
  `HStack(spacing: 0)`, pinning spacing to zero to match
  `border-collapse: collapse`, so its authored gap really is discarded and stays
  reported. That exception is what makes the table walk necessary rather than
  decorative.
- A value `container_spacing` cannot parse is still a drop, because
  `container_spacing` decides rather than this reporter assuming
  "`Column` means applied".

**EVERY occurrence of a part has to consume the gap, not merely one.** The first
version of `gap_consuming_parts` used `any`, which is the too-wide direction its
own doc warns about: a part name can be bound to more than one node — package
resolution substitutes a `pkg::` reference with the resolved sub-tree — and if
one is a `Column` and the other a `Box`, the gap really is lost on the `Box`.
Suppressing the report because some *other* node applied it hides a real loss.

That is the `all` the Compose reporter already uses, for the same reason its
`container_argument_covers` gives: a part shared between a `Row` and a `Text` is
genuinely dropped on the `Text`. No layout in the repo triggers it today, which
is precisely why it would have gone unnoticed; caught in security review, below
its reporting bar, by comparison with that sibling.

Five tests cover the five cases; four of them are the ones that would catch a
fix widened too far. The multi-node test is mutation-tested against a faithful
`any` — exactly that one test fails, and the other 216 stay green, so the test
isolates the distinction rather than merely reacting to suppression breaking.

A false drop is not a harmless extra line. It gets carried in
`ALLOWED_STYLE_DROPS`, where it reads as a standing licence for a gap that
genuinely stops being applied — the report is the evidence the release gate
consults, so one that overstates cannot be acted on.

### Added — a border has edges (UI79, #14835)

`border-{top,right,bottom,left}-{width,color}` now lowers. SwiftUI's `.border`
strokes all four sides and has no per-edge form, so each authored edge is
**drawn**: a `Rectangle` constrained on one axis and pinned to that side by
`.overlay(alignment:)`. The overlay sits on the view's own bounds — the border
box, outside padding, where CSS puts it.

A part authoring no edge keeps the byte-identical `.border(..)` it emits today.
Unauthored edge colours fall back to the `border-color` shorthand, the CSS
cascade answer.

Trestle emits **17** of these — 3 top, 6 trailing, 3 bottom, 5 leading — and the
repo's own compile gate, `complete_task_app_generated_swift_typechecks`, builds
the real generated Trestle SwiftUI through SwiftPM with them in place. That is a
`swift build`, not a grep.

#### A known divergence, recorded rather than worked around

`.leading`/`.trailing` are SwiftUI's only horizontal alignments and they **flip
under right-to-left layout**, whereas CSS `border-left` is physically left. UI79
puts RTL out of scope, so `border-left` maps to `.leading` and that is wrong in
an RTL locale. Naming it here because it is invisible in every LTR test.

#### A test this change had silently disabled

Inserting the new tests above `part_style_border_width_and_color_emit_border_modifier`
took its `#[test]` attribute, leaving that function dead and unrun while the
suite still reported green. Caught by clippy's `dead_code` and
`duplicate_macro_attributes`, not by the test count. Restored, and it runs
again.

### Added — `HostScroll` honours its axis (UI61, #14854)

`vertical` stays a bare `ScrollView` — SwiftUI's own default and byte-identical
to what every existing layout emits. `horizontal` and `both` name their axes.

### Fixed — `min-height` reached nothing (#14837)

The same gap Compose had, found because Engram is gated on **both**. Authoring
`min-height` on Engram's app shell made its `native_complete_gate` fail naming
SwiftUI — which is the gate working: fixing one backend and leaving the other
is how a property lands half-way (#14786).

| authored | SwiftUI |
| --- | --- |
| `min-height: 100vh` / `100%` | `.frame(maxHeight: .infinity)` |
| `min-height: N` / `Npx` | `.frame(minHeight: N)` |
| `min-height: 0` | nothing — a zero floor constrains nothing |

Its own chained `.frame`, beside the `max-width` ceiling and for the same
reason: chaining is how SwiftUI composes these, so a floor constrains an
authored height rather than replacing it. SwiftUI has no viewport unit either;
`maxHeight: .infinity` is its idiom for "take all the vertical space offered".

Verified with `swift build` on the generated Engram project, not by reading the
emitted Swift.

### Fixed — `gap` was dropped (#14804)

`gap` reached the lattice IR and died at this emitter. SwiftUI emitted only
`spacing: 0`, twice, against authored values of 2, 3, 5, 6, 7, 8, 10 and 22 —
and the strict `native-complete` profile reported zero degradations throughout,
because the degradation analyzer does not know the property exists. 178
declarations across 27 stylesheets.

`VStack` and `HStack` now take the authored `spacing:`. TaskApp emits **55**
spacing arguments where it previously emitted 2.

`ZStack` overlays its children along the depth axis and `ScrollView` delegates
layout to its content, so a gap on either is meaningless rather than merely
unsupported — dropped deliberately rather than guessed at, with a test for it.

Verified by generating TaskApp's SwiftUI sources and running `swiftc -parse`
over the result.

### Added — `HostInput.disabled` (#14786)

SwiftUI has no read-only `TextField`, so this emitter already approximated
`read-only` with `.disabled(...)` — over-restricting, but the only option.
Adding a real `disabled` prop means a control that *means* disabled now says
so directly. `disabled` is checked first, leaving only the genuinely
read-only case approximating (#14772).

Spec: `code/specs/UI58-hostinput-disabled.md` (#14786). Landed on all eight
backends in one change — a partly-landed prop would make a disabled input
*less* restricted on whichever backend lagged.

### Added — `border-radius` and `max-width` lowering (#12022, #14728)

Both were dropped entirely. `border-radius` is the most-authored property this
emitter was losing — 254 occurrences in Engram alone — so every rounded surface
rendered square on macOS.

`.cornerRadius` is emitted between `.background` and the border, because the
order is load-bearing: applied before the background it rounds an unfilled view
and leaves square fill, applied after the border it clips the stroke instead of
curving it. A part with both a radius and a border now strokes an overlaid
`RoundedRectangle` rather than calling `.border`, which always draws a
rectangle — otherwise the corners were round and the outline square.

`max-width` emits its own chained `.frame(maxWidth:)`, which SwiftUI composes
with the sizing frame, and correctly overrides the `.infinity` stretch the
alignment path emits when there is no width.

Both were found by the drop reporting added in the same change: making the
losses visible immediately showed which were one match arm away.

### Added — report style properties SwiftUI cannot lower (#12022)

`dropped_style_properties` returns every mosstyle property, per part, that this
emitter had no expressible SwiftUI output for. `mosaic-package-artifact-builder`
surfaces them in `mosaic-degradations.json` under `styleDegradations`, the same
channel XAML has used since #12022 opened.

SwiftUI is the second backend of eight to report. Until now the emitter's own
comment said the quiet part outright — "border-style, border-collapse, outline,
etc. — silently skipped" — and a package could carry `nativeComplete: true`
with `styleDegradations: []` while losing most of its styling.

The drops are collected from the **same `match` that does the lowering**, via a
`_with_drops` variant, not from a parallel list of supported names. A parallel
list goes stale silently the moment a property stops being lowered, which is
the exact failure mode being reported on.

Each drop carries a reason a reader can act on rather than a generic
"unsupported", because these are genuinely different problems: `flex-direction`
and `justify-content` cannot be applied to an already-built view at all
(SwiftUI chooses HStack/VStack/Spacer at construction time), while `box-shadow`
simply needs `.shadow` with different arguments.

`text-align` inside a state layer is excluded. It is deliberately base-only,
not unsupported, and reporting a documented decision as a loss would train
readers to ignore the list.

### Added — UI49 slot-owned style states

`one-of` slot values now activate their matching `.msl` state blocks in
generated SwiftUI. Enum axes follow `.mil` slot declaration order, then
existing automatic and explicit structural/interaction states take
precedence. Generic views and specialized host controls use the same modifier
lowering.

Tracked by [#14330](https://github.com/adhithyan15/coding-adventures/issues/14330).

### Fixed — radio groups now get real mutual exclusion (#13007)

A `HostRadio` lowered to a `Toggle`, and N independent Toggles have no mutual
exclusion: nothing stopped two being on at once except the host echoing back
consistent state. Qt has `ButtonGroup`, Compose `selectableGroup`, Flutter a
shared group value; SwiftUI has no container that groups Toggles *after the
fact*, which is why this stayed degraded after the other three were fixed.

It does have `Picker`, whose single selection is exclusive by construction. A
run of contiguous sibling radios sharing a literal `group:` now becomes one
Picker — a sibling-level transform rather than a per-node one.

The selection binding is **derived** from the members' own `checked:` slots
rather than held in `@State`. Local state would be a second source of truth
that drifts the moment the host changes the selection itself.

Deliberately strict about when it applies. A `slot:`-bound group, duplicate
values, members dispatching different events, an explicit `disabled:`, or a
lone radio all fall back to the previous Toggle emission and keep reporting
`property.radio-group-ignored` — each is a case where a Picker would change
behaviour rather than preserve it. `radio_groups_with_native_semantics` is the
same predicate the emitter uses, so what is reported cannot drift from what
was emitted.

The radio *appearance* is macOS-only, so `.pickerStyle(.radioGroup)` sits
behind `#if os(macOS)`; exclusivity itself is cross-platform.

### Fixed — a `list<list<text>>` slot never read the host

A slot typed as a list of rows fell through the host-binding match to the
*sample* value — a constant — while every neighbouring prop read from the host
correctly. The generated shell therefore compiled, ran, and showed an **empty
table** no matter what the host sent.

Rows are how every table in Mosaic is modelled: Engram's deck list, TaskApp's
project nav. So this was not an exotic corner, it was the one slot shape whose
whole purpose is to carry data, silently bound to nothing.

Adds a nested-list reader (`mosaicStringListList` / `MosaicHostValue.stringListList`)
and the match arm that reaches it.

### Fixed - preserve dynamic HostButton accessible names (#13754)

SwiftUI buttons now lower literal, slot-bound, keyword-bound, and
expression-bound `HostButton.a11y-label` values through
`.accessibilityLabel(...)`, including repeated rows.

### Fixed - preserve HostInput accessible names (#13717)

SwiftUI text fields now retain literal and slot-backed `HostInput.a11y-label`
values through `.accessibilityLabel(...)`.

### Added - native indeterminate checkbox state (#13006)

`emit_host_checkbox` previously had no code path for `indeterminate:`
at all — SwiftUI's `Toggle` has no tri-state visual, so the value was
silently dropped (documented as a known gap, not implemented). When
`indeterminate:` is authored as anything other than a literal
`Keyword("false")`, the emitter now swaps `Toggle` for a manually
composed mixed-checkbox: a `Button` wrapping an SF Symbol
(`minus.square.fill` / `checkmark.square.fill` / `square`) plus the
label, with `.buttonStyle(.plain)` — unlike a bare `Image` +
`.onTapGesture`, `Button` correctly respects a trailing `.disabled(...)`
modifier. Tapping always resolves *out of* mixed (mixed → checked,
checked → unchecked, unchecked → checked), matching the "toggle away
from indeterminate" rule `mosaic-emit-compose`'s `TriStateCheckbox`
lowering uses for the same prop.

A `slot:`/expression-valued `indeterminate` can't be evaluated at
compile time, so — mirroring XAML's `IsThreeState` treatment — its mere
*presence* unconditionally routes to the mixed-capable primitive; the
runtime value decides whether the mixed glyph actually renders.

New `pub fn host_checkbox_has_native_semantics` lets
`mosaic-package-artifact-builder`'s degradation analyzer stop reporting
`property.checkbox-indeterminate-ignored` for SwiftUI wherever this
lowering actually applies.

Verified with a real `swiftc -parse` of the generated `Button`/`HStack`/
`Image(systemName:)` shape.

### Fixed - HostLink.href now supports a slot:-bound value (#13110)

`emit_host_link` only ever matched a literal `String` href
(`find_string_prop` was the sole source) — a `slot:`-bound href
silently fell back to the `"#"` placeholder with no diagnostic, the
epic's own recurring "silent drop" failure class (#12017). Every other
native backend (XAML, Qt, Compose, Flutter) already supported this.

Added `find_slot_ref_prop(node, "href")` handling, introducing a small
`HostLinkHref` enum (`Literal`/`Slot`) so the same value threads
correctly through both consumption sites: the `Link(destination:)` URL
and the `onActivate` dispatch payload (previously always emitted a
hardcoded quoted string for the payload — for a slot-bound href it now
emits the bare property reference instead, so the host receives the
slot's live value, not a fixed placeholder).

A slot-bound href is an unknown runtime value, so it can't be scheme-
validated at compile time the way a literal is (#13052) — and
`URL(string:)` returns `nil` for malformed input, so the previous
bare `!` force-unwrap pattern would have crashed the app on tap for
any invalid runtime value. Both are handled by a small inline
runtime-validated closure: `guard let u = URL(string: <slot>), ["http",
"https", "mailto"].contains(u.scheme?.lowercased() ?? "") else {
return URL(string: "about:blank")! }; return u`. A malformed or
disallowed-scheme value falls back to a fixed, always-valid, inert
URL rather than crashing or navigating — the same "no navigation
target" outcome the XAML/Compose backends settled on for their own
runtime guards.

Verified with a real `swiftc` compile-and-run of the exact generated
closure expression (not just Rust-level string assertions): an
allowed scheme passes through unchanged, a disallowed scheme
(`javascript:`) and a malformed string both safely produce
`about:blank`.

### Security - validate literal HostLink.href's URI scheme (#13052)

Follow-up to #12038 (the identical XAML gap). `Link(destination: URL(string:
href)!)` hands the href straight to the OS on tap with no scheme check.
Added `has_disallowed_uri_scheme` (new `PipelineEmitError::UnsafeUriScheme`
variant), checked when `external` is not `false` — `external: false` routes
through a Button + dispatch instead, never constructing a `URL` at all, so
a routing placeholder like `href: "#"` stays valid there (mirrors the
Qt/Compose backends' identical scoping).

Audit finding, not fixed here: this backend has no `slot:`-bound href
support at all today — `find_string_prop` only matches a literal `String`,
so a `SlotRef` href silently falls back to `"#"` with no diagnostic. That's
a pre-existing silent-drop bug (the epic's own recurring failure class,
#12017), not a #13052 scheme-validation gap; filed separately as its own
issue rather than folded into this security fix.

### Added - accessible HostSlider names

Literal and slot-backed `HostSlider.a11y-label` values now lower to a native
SwiftUI accessibility label without replacing the slider's adjustable role.

Slot-bound or expression-backed slider steps now flow into the generated
native Slider wrapper instead of falling back to a hard-coded increment.

### Added - native adjustable slider

`HostSlider` now lowers to SwiftUI's native `Slider`, including controlled
range, discrete or continuous movement, disabled state, continuous change
dispatch, and exact release-value commit dispatch. Strict generated macOS and
iOS artifacts compile in required CI.

### Added - portable Text accessibility

`Text` now lowers literal or slot-backed accessible names, heading traits, and
intentional accessibility hiding to native SwiftUI modifiers. Mosaic-authored
headings no longer lose their semantic level when emitted outside the web
backend.

### Added - native accessible dynamic tables

Canonical UI31/Grid `HostTable` trees now lower to SwiftUI's native `Table`
with runtime-sized `TableColumnForEach` definitions, stable row identity,
bounds-safe cell and width lookup, and the authored interactive Cell subtree.
Generated macOS 13 and iOS 16 packages use a native `List`/`Section` fallback
before the dynamic-column API's macOS 14.4 / iOS 17.4 availability. Unsupported
HostTable structures retain the visual fallback and remain explicit
native-complete degradations.

### Added - native accessible drag and drop

`HostDraggable` and `HostDropTarget` now lower to SwiftUI's native drag/drop
system with component-local payload isolation, accepts and disabled filtering,
before/into/after hover proposals, accepted-only drag completion, and an
equivalent keyboard workflow with platform accessibility announcements. The
complete generated TaskApp is compiled through SwiftPM as the regression gate.

### Fixed - empty-event project shells

Components with no authored events keep their uninhabited event enum but now
emit the standard wire helpers with exhaustive empty switches. Generated host
state can therefore type-check its unreachable dispatch seam, allowing minimal
Rust-driven SwiftUI packages to compile through SwiftPM.

### Added - native icons and progress

`Icon` now lowers semantic glyph names to SF Symbols with authored or default
accessibility labels. The semantic `spinner` glyph becomes SwiftUI's native
indeterminate `ProgressView`. Indexed text expressions also use the enclosing
`ForEach` integer shadow, allowing Accordion's `bodies[i]` projection to remain
type-correct alongside number-typed loop comparisons.

### Added - native-complete runtime-required shell

`EmitOptions::require_runtime` now generates a SwiftUI shell that requires the
standard Mosaic Rust runtime, applies its initial props before the first view,
and validates required, optional, and defaulted slots without preview values,
reflection hosts, or event-print fallbacks. Permissive output is unchanged.

### Fixed - Complete Task App SwiftUI compilation

Package-expanded applications now lower Mosaic value conditions through
type-correct truthiness conversion, keep generated helper names collision-safe,
and introduce concrete `AnyView` and local-function boundaries around native
controls, state wrappers, and modifier chains. This keeps Swift's constraint
solver within budget without replacing SwiftUI controls or authored MSL styles.
`HostInput` commit dispatch also follows the MIL event declaration, carrying the
current text for a one-text-parameter event while preserving void commits. A
macOS regression gate builds the complete generated Task App through SwiftPM
rather than type-checking only focused fixtures.

### Fixed - Native multiline Input compatibility

The still-supported UI25 `Input` primitive now lowers to an accessible SwiftUI
`TextEditor` when `multiline: true`, including visible placeholder text,
dispatch-driven editing, maximum-length enforcement, and the authored MLL part
identifier. Trestle Notes can therefore compile without collapsing its body
editor into a single-line field.

### Added - optional generated-shell interaction acceptance

Generated SwiftUI applications now call an optional package-host interaction
hook after their Mosaic root appears. Package owners can exercise the emitted
native controls and shared dispatch path in direct launch acceptance without
adding application-specific behavior to the generated shell.

### Added - Native automation identifiers for authored controls

`HostInput` and `HostButton` now preserve their MLL part names as SwiftUI
`accessibilityIdentifier` values. Generated applications can locate the same
Mosaic-authored control deterministically for accessibility and direct native
interaction acceptance without adding a parallel AppKit control tree.

### Added - host-driven prop refresh

Generated SwiftUI project shells now let optional native `MosaicHost` adapters
register a props-changed handler. Host-surface interactions can request a fresh
slot projection on the main queue without duplicating Mosaic component behavior
in AppKit or UIKit.

### Added - project-shell native node-slot bridge

Generated SwiftUI project shells now resolve `node` and component slots through
an optional `MosaicHostBridgeObject.node(named:)` hook. macOS `NSView` and iOS
`UIView` objects are wrapped with `NSViewRepresentable` / `UIViewRepresentable`
and passed to the generated component as `AnyView`; absent or mistyped nodes
retain the empty-view fallback. A Venture acceptance test builds the generated
SwiftPM app with a real host-provided `NSView` content surface.

### Added - HostSurface native composition

SwiftUI output now lowers Mosaic `HostSurface ( content: slot: ... )` to the
host-supplied `AnyView` node slot. Generated SwiftPM previews retain an
`AnyView(EmptyView())` fallback, while application hosts can mount a native
Metal or other platform renderer inside Mosaic-authored chrome.

### Added - Native activation for MSL pressed states

SwiftUI output now connects UI15's built-in `state pressed` blocks on
`HostButton`, `HostCheckbox`, `HostRadio`, and `HostLink` to a generated local
`@GestureState`. Pointer and touch presses activate the shared MSL properties
and transitions without replacing the control's native action, including
independent instances inside `ForEach`. Explicit `state-when-pressed`
predicates remain author-controlled. A Task App acceptance gate proves its
Mosaic-authored add-task button feedback reaches generated, type-checked
SwiftUI without handwritten AppKit UI.

### Added - Native activation for MSL focused states

SwiftUI output now connects UI15's built-in `state focused` blocks on native
focus-capable host controls to a generated local `@FocusState`. The same shared
MSL properties and transitions activate when `TextField`, `Button`, `Toggle`,
or `Link` receives native keyboard or pointer focus, including independent
instances inside `ForEach`. Explicit `state-when-focused` predicates remain
author-controlled. A Task App acceptance gate proves its Mosaic-authored
project-composer focus ring reaches generated SwiftUI without handwritten
AppKit UI.

### Added - Native activation for MSL hover states

SwiftUI output now activates UI15's built-in `state hover` blocks without
requiring authors to repeat the interaction as a `state-when-hover` layout
predicate. The emitter generates a small native SwiftUI hover wrapper only
when the compiled component uses a hover state. Every wrapper owns its own
`@State`, including wrappers inside `ForEach`, so hovering one repeated row
does not restyle the whole list. Existing explicit `state-when-hover`
predicates remain author-controlled and do not install pointer tracking.

### Added - Native SwiftUI lowering for MSL transitions

Part-level and state-local transitions from `mosstyle-compiler` now lower to
property-scoped SwiftUI `.animation(_:value:)` modifiers. Resolved millisecond
and second durations, standard ease curves, and cubic Bézier timing curves map
to native `Animation` values. State-local transitions apply while entering the
matching `state-when-*` condition and fall back to the part transition, or no
exit animation when no part transition exists. The emitter also now lowers the
MSL `opacity` property so common fade transitions work natively.

### Added - Mosaic event envelopes for SwiftUI hosts

Generated non-empty `{Component}Event` enums now include `mosaicName`,
`mosaicPayload`, and `mosaicEnvelope` helpers. `Sources/App/App.swift` uses the
envelope in its sample dispatch closure, giving SwiftUI native hosts a stable
wire shape to JSON-encode into shared Mosaic/Engram business logic.

### Fixed - `--emit-project` SwiftPM shell supplies view inputs

`Sources/App/App.swift` now mounts `{Component}View(...)` with deterministic
sample values for every declared slot plus a dispatch closure. Previously the
project shell emitted `{Component}View()` even though generated SwiftUI views
store each slot and `dispatch` as required initializer inputs, so any component
with slots (including EngramApp) produced a SwiftPM shell that was not
compile-shaped.

### Fixed — editable `HostInput` (the formula-bar issue)

A `HostInput` with an `onChange` handler and a bound `value` slot now lowers to
a **writable** `TextField` binding — `Binding(get: { value }, set: { dispatch(.onChange(value: $0)) })`
— instead of the read-only `text: .constant(value)`. Previously the generated
`TextField` could not be typed into at all (the constant binding discarded
every keystroke, and the separate `.onChange(of:)` modifier only fired when the
*prop* changed). The setter now dispatches the change per keystroke, so hosts
get a genuinely editable field; the redundant `.onChange(of:)` modifier is no
longer emitted for editable inputs (emitting both would feed back: setter →
host updates slot → `.onChange` fires → dispatches again). Inputs without an
`onChange` handler keep the read-only `.constant(...)` form (label-like
display). This is what makes the VisiCalc SwiftUI demo's formula bar actually
editable.

### Added — UI32-K-swiftui — `--emit-project` SwiftPM macOS shell

L7 of UI32 ([spec PR #4286](https://github.com/adhithyan15/coding-adventures/pull/4286); L2-L6: #4297, #4309, #4315, #4319, #4325). `mosaic-compile --backend swiftui --emit-project` now produces a SwiftPM scaffold:

- `Package.swift` — pinned `swift-tools-version: 5.10` + `platforms: [.macOS(.v13)]` per UI32 §3.6.3. Single executable target `App` at `Sources/App/`.
- `Sources/App/App.swift` — SwiftUI `@main App` + `WindowGroup` mounting `{Component}View()` (matches the emitter's `{name}View` struct convention).
- `README.md` — `swift run` recipe + file map. Notes the user must move `{Component}.swift` into `Sources/App/` for SwiftPM to compile it (v1 layout doesn't auto-place the component file).

New public API (matches L2-L6 pattern):

- `pub struct EmitOptions` — `emit_project`, `pinned_swift_tools`, `pinned_macos_min`.
- `pub struct ProjectFiles` — `package_swift`, `app_swift`, `readme`.
- `pub enum ProjectShellError` — `SwiftKeywordCollision(String)` surfaced through `PipelineEmitError::UnsafeSlotName`.
- `pub struct PipelineEmitResultWithProject`.
- `pub fn from_pipeline_with_options(...)`. Existing `from_pipeline(...)` unchanged.

UI32 §3.6.2 SwiftUI row contract: Swift reserved keywords (`Class`, `Protocol`, `Actor`, `Self`, `Any`, `Type`, etc. — PascalCase subset) MUST be rejected to avoid backtick-quoting in identifier positions. `SWIFT_RESERVED_KEYWORDS` reject-list enforces this; collision → fail-loud via `ProjectShellError::SwiftKeywordCollision`.

10 new tests cover the spec §3 gates plus a Swift-keyword truth table (10 accept/reject vectors) and an App.swift structural test (@main + WindowGroup + `<Component>View()` mount). Total tests: 81 (was 71, +10).

### Added — UI31-K-swiftui — `HostTable` RTL contract

The SwiftUI `HostTable` lowering (which produces a structural
`VStack(alignment: .leading, spacing: 0)` of `HStack` rows) now
honours the UI31 §3.2 RTL contract via SwiftUI's `Environment`
key-path knob `\.layoutDirection`:

- `dir: rtl` → `.environment(\.layoutDirection, .rightToLeft)`
  modifier attached to the VStack; flips horizontal layout
  direction for the whole table.
- `dir: ltr` → `.environment(\.layoutDirection, .leftToRight)` —
  explicit-LTR, useful for tables that should stay LTR inside an
  RTL window (e.g. data-heavy spreadsheets).
- `dir: auto` → no modifier; the spec-mandated "let the host
  decide" semantic is the SwiftUI default — the ambient
  `Environment(\.layoutDirection)` flows through from the system
  locale → app → ancestor view cascade.
- `dir: slot: layout-direction` →
  `.environment(\.layoutDirection, layoutDirection)`, where the
  slot must evaluate to a `LayoutDirection`. The slot name passes
  through `is_safe_swift_identifier` so it can't smuggle malicious
  Swift through the modifier's expression position.
- Unknown keywords drop silently — the allow-list is the security
  gate. Test #6 feeds the literal payload
  `".rightToLeft).onAppear { pwn() }"` (specifically shaped to
  break out of the modifier-call argument list) and asserts `pwn()`
  never reaches the output.

7 new tests cover the a11y gate (VStack + HStack structure
preserved — not a flat ZStack or Group), the three allow-listed
keywords (incl. the SwiftUI-unique no-emit for `auto` and the
explicit `.leftToRight` for cross-locale tables), the slot-ref
binding, the silent-drop with an injection-shaped payload, and a
no-`dir` regression guard. Total tests: 71 (was 64).

### Added — UI29-4 `HostLink` + `HostTooltip` + `HostNumberInput` (U29-4-K-swiftui)

Three new UI29-4 kernel primitives lower to native SwiftUI views:

- **`HostLink` → `Link(label, destination: URL(string: href)!)`**
  (iOS 14+/macOS 11+). OS-managed URL open by default. When
  `external: false` + `onActivate` are bound, the lowering swaps
  to a `Button(action: { dispatch(.x(href: "...")) }) { Text(label) }`
  so the host's in-app router takes over instead of opening
  externally. When `external != false` but `onActivate` is bound,
  the v1 emitter currently drops the dispatch (SwiftUI's `Link`
  has no click-hook closure); documented as a v2 follow-up.
- **`HostTooltip` → `VStack { child(ren) }.help("text")`** (macOS
  / iOS 16+). Hovering (macOS) or long-pressing (iOS) the wrapped
  view shows the tooltip; screen readers read it via
  `accessibilityHint`.
- **`HostNumberInput` → `TextField(placeholder, value: .constant(slot),
  format: .number)`** (iOS 15+/macOS 12+). `disabled` adds a
  trailing `.disabled(...)` modifier; `onChange` adds an
  `.onChange(of: slot) { dispatch(.x(value: slot)) }` modifier
  (pre-iOS-17 closure shape — host can adapt to the new
  `(old, new)` shape if needed).

5 new tests cover: bare HostLink with Link + URL, the
external-false + onActivate Button swap, HostTooltip's VStack +
.help wrapper, HostNumberInput's TextField + .number format, and
the .onChange modifier wiring.

### Added — UI29-2 `HostCheckbox` + `HostRadio` kernel primitives (U29-2-K-swiftui)

Both new primitives lower to SwiftUI `Toggle` with the platform's
default toggle style. The semantic distinction is in the dispatched
payload:

- `HostCheckbox` dispatches `checked: Bool` on every flip via a
  `Binding(get:set:)` whose setter calls `dispatch(.x(checked:
  newValue))`. Without an `onToggle` emit the binding degrades to
  `.constant(checked)` (read-only but type-checks).
- `HostRadio` dispatches `value: String` only on positive transition
  via a `Binding(get:set:)` whose setter wraps the call in `if
  newValue { dispatch(.x(value: …)) }` — flips to `false` (a sibling
  radio caused this one to deselect) are silently dropped to match
  the kernel-canonical `onSelect = "this radio was chosen"`
  semantics.
- The `group:` prop on `HostRadio` is preserved as a `// group: …`
  Swift comment ahead of the `Toggle`. SwiftUI has no implicit radio
  grouping; the comment keeps the metadata visible for a future
  structural pass that synthesises a `Picker` from sibling radios
  sharing a `group:`.
- `label:` becomes the first positional `Toggle(...)` argument
  (string literal or slot identifier); `disabled:` becomes a trailing
  `.disabled(...)` modifier.

Deferred to a follow-up:

- `HostCheckbox.indeterminate` slot. SwiftUI's `Toggle` has no
  tri-state visual; rendering a "mixed" state needs a custom
  `ToggleStyle` or an `Image` of `checkmark.square.fill`.
- `.toggleStyle(.checkbox)` for an actual checkbox look on macOS.
  That style is macOS-only and breaks iOS compilation; a follow-up
  can add platform-conditional emission or move the choice to a
  userland modifier.

9 new tests cover the bare-toggle shape, slot-driven `.constant(…)`
binding, the `Binding(get:set:)` setter for `onToggle`, string label,
`.disabled(…)` modifier, the radio's `// group:` comment, the
positive-transition setter for `onSelect`, and the slot-typed
`value:` flowing into the dispatch payload.

## [0.5.0] - 2026-05-21

### Added — UI29-1 `HostDialog` kernel primitive (U29-1-K-swiftui)

`HostDialog` is now recognised by the SwiftUI emitter and lowers to an
invisible `Color.clear.frame(width: 0, height: 0)` anchor view carrying
a `.sheet(...)` (modal=true, default) or `.popover(...)` (modal=false)
view modifier. SwiftUI exposes dialogs as view modifiers, not
standalone views; anchoring on `Color.clear` lets `HostDialog` remain
a single tree-walker node in the kernel emitter.

Prop mapping:

- `open: slot: x` → `isPresented: .constant(x)` (immutable-slot pattern;
  same `.constant(...)` choice `HostInput` uses).
- `modal: true` (default) → `.sheet(...)`.
- `modal: false` → `.popover(...)`. SwiftUI's `.popover` does NOT
  accept an `onDismiss:` argument, so when modal=false the emitter
  silently drops the `onClose` wiring — the host should observe its
  own `open` slot change and dispatch the close event itself.
- `title: "..."` / `title: slot: x` → `.navigationTitle(...)` inside
  the content closure.
- `dismiss-on-backdrop: false` → `.interactiveDismissDisabled(true)`
  inside the content closure.
- `onClose: emit: onX` → `onDismiss: { dispatch(.x) }` (`.sheet` only).

The dialog's children render inside the content closure's `VStack`,
walked via `emit_children` so nested kernel primitives lower the same
way they do anywhere else.

### Added — tests

- 8 new tests covering: empty HostDialog (Color.clear + .sheet),
  `open` slot → `.constant(x)`, `modal: true` → `.sheet`, `modal: false`
  → `.popover` (and `onClose` NOT wired), children render inside content
  VStack with correct order, `onClose` → `onDismiss` callback,
  `title` slot → `.navigationTitle` (plus string-literal sanity), and
  `dismiss-on-backdrop: false` → `.interactiveDismissDisabled(true)`
  (plus negative case for the default).

### Changed

- Recognised-vs-deferred matrix test now lists `HostDialog` as
  recognised alongside `HostTable` / `HostInput` / etc.
- Crate version bumped `0.4.0` → `0.5.0`.

### Spec

- `code/specs/UI29-1-host-dialog.md` ships in the same commit (specs
  must precede implementation per repo workflow §8). The SwiftUI
  lowering section pins exactly what this PR implements.

## [0.4.0] - 2026-05-20

### Added — UI29 `For` / `If` / `Else` meta-primitives (U29-K-swiftui)

The two UI29 meta-primitives now have SwiftUI lowerings, completing
the kernel surface for this backend.

`For (each: <slot-or-expr>, as: <name>, index: <name>?) { ... }` lowers
to SwiftUI's `ForEach`. The id keypath switches based on whether
`index:` is bound:

- `as:` only           → `ForEach(<coll>, id: \.self) { <as> in <body> }`
- `as:` + `index:`     → `ForEach(Array(<coll>.enumerated()), id: \.offset) { (<idx>, <as>) in <body> }`

`If (when: <slot-or-expr>) { <then> } Else { <else>? }` lowers to a
Swift view-builder `if`/`else`. The `Else` is paired with its
preceding `If` sibling in a peek-and-consume walk inside container
bodies, so `If`+`Else` always emit as a single `if cond { ... } else
{ ... }` block rather than two stray nodes.

For both primitives, `SlotRef` props are camelCased into Swift
identifiers and `Expr` props pass through verbatim (the moslayout
parser hands them in as the reconstructed source substring).

Orphan `Else` (an `Else` not preceded by an `If`) is rejected by the
moslayout analyzer; the emitter is defensive and renders a Swift
comment `// orphan Else — ignored` so any escapee still produces a
compilable file.

### Added — tests

12 new tests cover the For/If/Else surface: SlotRef vs Expr each, the
index-on/off id-keypath switch, body-uses-as-binding, nested For,
if-only, if/else paired, expr-condition, orphan-Else comment, the
combined expr+pair case, two adjacent If-without-Else siblings, and a
For body whose children include an If/Else pair.

## [0.3.0] - 2026-05-19

### Added — UI29 `HostTable` kernel primitive

`HostTable` is now recognised by the SwiftUI emitter and lowers to a
`VStack(alignment: .leading, spacing: 0)` of `HStack` rows rather than
SwiftUI's data-driven `Table` view. SwiftUI's `Table` needs a
`[RowType]` collection plus per-column `TableColumn(key:)` declarations
that don't naturally fall out of the structural "compose from children"
emitter — full `SwiftUI.Table` integration waits on `For`-inside-table
in a follow-up PR.

Sub-tag handling:

- `HostTableHead` → `HStack` rows whose `Text(...)` children carry
  `.bold()` modifiers.
- `HostTableBody` → `HStack` rows, plain.
- `HostTableFoot` → preceded by `Divider()`, then `HStack` rows.
- `HostTableColGroup` → emits a Swift comment
  `// HostTableColGroup ignored in SwiftUI` (no SwiftUI analog).

When a `HostTableHead` is followed by any non-head section, a
`Divider()` is auto-inserted between them so the visual head/body
separation matches the HTML `<thead>` / `<tbody>` convention.

Orphan sub-tags (`HostTableHead` / `Body` / `Foot` / `ColGroup` used
outside a `HostTable` parent) emit a self-documenting Swift comment
rather than erroring; comments are statement-level no-ops, so the
generated file still type-checks.

`part_name` on a `HostTable` is currently surfaced as a Swift comment
`// part: <name>` directly before the VStack opener. SwiftUI has no
native equivalent of CSS `part`; a future style-inlining PR can swap
this for a real modifier.

### Added — tests

- 8 new tests covering: empty HostTable, head-only (bold), body-only,
  foot preceded by divider, head+body ordering with auto-divider,
  ColGroup-emits-comment, orphan sub-tag handling, and `part_name`
  emission. The recognised-vs-deferred matrix test now lists
  `HostTable` as recognised; only `If` and `For` remain deferred.

### Crate version

- Bumped from `0.2.0` → `0.3.0`.

## [0.2.0] - 2026-05-19

### Added — UI29 kernel primitives (partial)

Extends the SwiftUI backend skeleton (v0.1.0) with four of UI29's kernel
primitives. The remaining three (`If`, `For`, `HostTable`) wait on the
moslayout grammar additions (U29-G3) and a `HostTable` spec; they still
return `UnknownPrimitive` so authors who reach for them get a clear
"not yet supported" diagnostic.

- `Stack` lowers to `ZStack { ... }` — the z-axis / overlay container.
- `HostScroll` lowers to `ScrollView { ... }` — SwiftUI's built-in
  scrollable region. Implicit scroll-state and viewport handling means
  no offset/extent slots need to be threaded through the lowering.
- `HostInput` lowers to `TextField(placeholder, text: .constant(value))`
  with these prop bindings:
  - `placeholder: "..."` → the first arg of `TextField("...", text: ...)`
  - `value: slot: x` → `text: .constant(x)` (see binding nuance below)
  - `read-only: slot: x` / `read-only: true` / `read-only: false` →
    `.disabled(...)` modifier
  - `onChange: emit: onE` → `.onChange(of: value) { dispatch(.e(value: value)) }`
  - `onCommit: emit: onE` → `.onSubmit { dispatch(.e(value: value)) }`
  - `onCancel: emit: onE` → `.onExitCommand { dispatch(.e) }` (macOS-only;
    documented as a known limitation for iOS / iPadOS).
- `HostButton` lowers to `Button(action: { dispatch(.tap) }) { Text(label) }`
  with these prop bindings:
  - `label: "..."` → `Text("...")` inside the label closure
  - `label: slot: x` → `Text(x)` inside the label closure
  - `disabled: slot: x` / `disabled: true` / `disabled: false` →
    `.disabled(...)` modifier
  - `onTap: emit: onE` → `action: { dispatch(.e) }`

### Binding nuance — `.constant(value)`

SwiftUI `TextField` requires a `Binding<String>`, not a plain `String`.
Mosaic components receive slots as immutable `let`s, so we have two
reasonable options:

1. **`.constant(value)` wrapper** — emit `.constant(value)` and rely on
   `dispatch(.commit(value: ...))` for updates. Inline typing does NOT
   echo back into the bound slot; only `onSubmit` (Enter) carries new
   text. Matches UI24's dispatch-driven flux pattern.
2. **Local `@State` proxy** — wrap the body in a `@State` buffer that
   initializes from the slot and dispatches `onChange` per keystroke.
   More complex generated code; deferred to a future PR.

This release ships option (1) and documents the choice in code comments,
the README, and the per-function doc comment on `emit_host_input`.

### Added — tests

- 8 new tests covering: `Stack → ZStack` empty + with-children, HostInput
  `.constant(value)` binding shape, HostInput `read-only` keyword + slot
  forms, HostInput `onCommit` → `.onSubmit` dispatch, HostButton
  `action: + label` structure, HostScroll → `ScrollView`, and a
  recognised-vs-deferred matrix that pins `If` / `For` / `HostTable` as
  still returning `UnknownPrimitive`.
- The crate-version pin test now expects `0.2.0`.

### Crate version

- Bumped from `0.1.0` → `0.2.0`.

## [0.1.0] - 2026-05-19

### Added — initial skeleton (UI28 WB4)

- New crate `mosaic-emit-swiftui` providing a `from_pipeline(interface,
  layout, style) -> Result<PipelineEmitResult, PipelineEmitError>` entry
  point that consumes the three-file Mosaic pipeline (`.mil` / `.mll` /
  `.msl`) and emits a `.swift` source file containing a SwiftUI `View`
  struct.
- Primitive lowering for `Box` → `Group`, `Row` → `HStack`, `Column` →
  `VStack`, `Text` → `Text("...")` / `Text(slotName)`, `Spacer` →
  `Spacer()`, `Image` → `Image(systemName: "...")` placeholder, and
  `Divider` → `Divider()`. Other primitives (`Scroll`, `Stack`, `Icon`,
  `Grid`, `Input`) return `UnknownPrimitive` errors and land in
  follow-up PRs.
- Slot lowering: `text` → `String`, `number` → `Double`, `bool` →
  `Bool`, `image` → `String`, `color` → `String`, `node` → `AnyView`,
  `list<T>` → `[<inner Swift>]`. Slots become `let` stored properties on
  the View struct; `dispatch: (NameEvent) -> Void` is appended last.
- Event lowering: one Swift `enum case` per declared emit, with the `on`
  prefix stripped and the rest lower-camelCased (mirrors the React
  backend's union-variant `type` literal). Payload parameters become
  named associated values (`case navigate(row: Double, col: Double)`).
  Empty emit lists produce an uninhabitable `enum NameEvent {}` — the
  SwiftUI analog of TypeScript's `type NameEvent = never`.
- 13 unit tests covering: empty layout, slot lowering for every
  primitive type, empty / non-empty event enum, primitive surface views,
  Text-literal escaping, Text-slot-ref camelCase conversion, Image
  source / fallback, kebab→camelCase across slot+emit+param names,
  component name mismatch error, UnknownPrimitive error, full smoke
  test for a multi-slot Row+Text+Spacer+Text component, and a version
  pin.
- `PipelineEmitError` variants: `ComponentNameMismatch`,
  `UnsafeSlotName`, `UnsafeEmitName`, `UnknownPrimitive` (mirrors React
  backend; no React-only reserved-name variant because Swift has no
  equivalent prop-name conflict surface).

### Known limitations (deferred to follow-up PRs)

- **UI28 §2 Cell / Column-as-metadata / Grid v3** — this PR keeps the
  legacy UI14 `Column → VStack` lowering so existing demos still
  compile. The UI28 SwiftUI lowering (`Grid → SwiftUI.Table {
  TableColumn(...) }` per spec §4.4) is a separate follow-up.
- **`connects` wiring** — emit refs on layout nodes are not yet
  attached to SwiftUI gesture modifiers (`.onTapGesture { dispatch(.tap) }`).
- **`mosstyle::StyleDef` inlining** — the argument is accepted to lock
  the signature, but `View` modifier chains (`.background(...)`,
  `.padding(...)`) are not yet emitted.
- **`Scroll`, `Stack`, `Icon`, `Grid` (v2), `Input` primitives** — each
  returns `UnknownPrimitive` and lands in its own follow-up.
