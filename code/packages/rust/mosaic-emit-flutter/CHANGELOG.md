# Changelog — mosaic-emit-flutter

All notable changes to `mosaic-emit-flutter` will be documented in
this file.

## [Unreleased]

### Fixed - native form state and generated-shell interaction acceptance

Slot-backed `HostButton.disabled` and `HostInput.read-only` values now reach
Flutter's native `onPressed` and `readOnly` contracts, while `HostInput`
`onCommit` dispatches from `TextField.onSubmitted`. Direct `Row` inputs are
wrapped in `Expanded` so generated toolbars have a finite width and can render.
Generated `MosaicApp` shells also accept an injected `MosaicHost`, allowing
widget tests to exercise initial props, native input, event envelopes, and
host-driven prop refresh without replacing the generated component.

### Fixed - project-shell widget-slot hydration

Generated Flutter shells now accept a host-provided `Widget` in the props map
for `node` slots and pass it to the component through `mosaicWidget`. Missing or
mistyped values retain the deterministic `SizedBox.shrink` fallback.

### Fixed - host-owned surface composition

`HostSurface ( content: slot: ... )` now mounts the supplied `Widget` instead
of silently emitting the unresolved-component `SizedBox` placeholder.
The package builder mirrors the component into `lib/` so Dart imports stay
inside the package boundary, and the generated README documents the one-time
`flutter create --platforms=... .` runner bootstrap. That exact flow now
produces a native macOS app from Venture's generated browser chrome.

### Added - Flutter Mosaic event envelopes

Generated Dart event classes now expose `mosaicName`, `mosaicPayload`, and
`mosaicEnvelope`, and the generated Flutter app shell logs the envelope. This
lets Flutter hosts forward the same event map used by HTML, Electron, SwiftUI,
XAML, and Qt shells.

### Fixed - `--emit-project` Flutter shell supplies constructor inputs

`lib/main.dart` now mounts the generated widget with deterministic sample
values for every declared slot plus a dispatch callback. Previously the
Flutter shell emitted `{Component}()` even though generated widgets always
require `dispatch` and may require slot constructor arguments.

### Added — UI29-FU-flutter / UI28-1 §6.2 — Flutter `For` / `If` / `Else` lowering

Replaces the v0.1 placeholder (`/* TODO: For not yet wired in the Flutter
emitter */ const SizedBox.shrink()`) with real Dart lowering for all
three control-flow primitives. Required by `mosaic-pkg-grid` v0.2.0 per
[UI28-1 §6.2](../../../specs/UI28-1-grid-v3-userland-revised.md).

- **`For`** lowers to a `Column(children: ...)` whose list is built
  by `.map(...).toList()` over the iterated collection:
  - `For ( each: slot: X , as: y )` → `Column(children: x.map((y) => <body>).toList())`
  - `For ( each: slot: X , as: y , index: i )` → `Column(children:
    x.asMap().entries.map((entry) { final i = entry.key; final y =
    entry.value; return KeyedSubtree(key: ValueKey(i), child:
    <body>); }).toList())` — the `KeyedSubtree(ValueKey(i))` wrapper
    gives Flutter's element tree the stable identity Flutter's diff
    needs when rows reorder (matches React `key={i}` and SwiftUI
    `id: \.offset`; UI28-1 §5 performance property).
  - `each:` accepting `SlotRef` or `Expr` — Expr text passes through
    verbatim (author-controlled, matches React/SwiftUI/Qt behaviour).
  - `as:` / `index:` kebab-case names lower via `to_camel_case_first_lower`
    so the emitted Dart bindings are valid identifiers.
- **`If`** lowers to a Dart ternary returning a Widget:
  - `If { then }` → `((cond) ? <then> : const SizedBox.shrink())`
  - `If { then } Else { e }` → `((cond) ? <then> : <else>)`
  - `when:` accepting `SlotRef` (lowered via camelCase) or `Expr`
    (verbatim). Empty branches collapse to `const SizedBox.shrink()`
    so the ternary always returns a concrete Widget.
- **Sibling pairing** — `emit_paired_children` walks each container's
  children with peek-ahead so an `If` followed by `Else` fuses into a
  single ternary. Orphan `Else` (analyzer should reject) renders a
  documenting comment instead of crashing. Pattern mirrors the SwiftUI
  backend's `emit_children`. `HostScroll` and `HostTooltip`'s
  multi-child paths also route through the paired walker.

8 new tests cover the lowering shapes: unindexed `For`, indexed `For`
with `KeyedSubtree`, `Expr`-as-each pass-through, kebab-case binding
camel-casing, standalone `If` with empty-else SizedBox fallback,
paired `If`/`Else` inside a Box (Cell.mll shape), `Expr` `when:`
(verifies the Cell.mll predicate `cellRow == editRow && cellCol ==
editCol` lowers without UI29 §3.4 since slots are in scope), empty
`For` body, and nested `For` with `Expr` inner each (the v0.2.0 Grid
composition shape). Total tests: 57 (was 49, +8).

### Added — UI32-K-flutter — `--emit-project` Flutter app shell

L5 of UI32 ([spec PR #4286](https://github.com/adhithyan15/coding-adventures/pull/4286); L2 React #4297, L3 HTML #4309, L4 WebComponent #4315). `mosaic-compile --backend flutter --emit-project` now produces a flutter-create-shaped scaffold alongside the component `.dart`:

- `pubspec.yaml` — pinned Flutter SDK `>=3.24.0 <4.0.0` + Dart `>=3.5.0 <4.0.0` per UI32 §3.6.3. Dart pub package name follows snake_case rules (§3.6.2 Flutter row) — auto-derived as `mosaic_{snake(name)}` (e.g., `ProfileCard` → `mosaic_profile_card`).
- `lib/main.dart` — `MaterialApp` shell that mounts the component as `Scaffold.body`'s `Center(child: <Component>())`. Imports the component package-locally from `lib/{Component}.dart`, which keeps the generated shell valid under Dart's package-boundary rules.
- `README.md` — `flutter pub get && flutter run` recipe + file map.

New public API (matches L2/L3/L4 pattern):

- `pub struct EmitOptions` — `emit_project`, `pinned_flutter_sdk`, `pinned_dart_sdk`, `package_name` override.
- `pub struct ProjectFiles` — `pubspec_yaml`, `main_dart`, `readme`.
- `pub enum ProjectShellError` — `InvalidDartPubName(String)` surfaced through `PipelineEmitError::UnsafeSlotName`.
- `pub struct PipelineEmitResultWithProject` — `output`, `component_name`, `project: Option<ProjectFiles>`.
- `pub fn from_pipeline_with_options(...)` — new entry. Existing `from_pipeline(...)` unchanged.

UI32 §3.6.2 Flutter row: Dart pub names MUST match `[a-z][a-z0-9_]*` (lowercase, digits, underscores; must start with letter; no leading underscore; no hyphens; no uppercase). `is_valid_dart_pub_name` enforces this; an explicit invalid `package_name` fails-loud via `ProjectShellError::InvalidDartPubName`.

11 new tests cover the spec §3 gates plus a Dart-pub-name truth table (8 accept/reject vectors) and a main.dart structural test (MaterialApp + Scaffold + Component() mount). Total tests: 48 (was 37, +11).

### Added

- **UI31-K-flutter** — RTL contract for `HostTable`. The lowering
  now wraps the emitted `DataTable` in `Directionality(textDirection:
  ..., child: ...)` when the layout author writes `dir:`:
  - `dir: rtl` → `Directionality(textDirection: TextDirection.rtl, child: DataTable(...))`
  - `dir: ltr` → `Directionality(textDirection: TextDirection.ltr, child: DataTable(...))`
  - `dir: auto` → no wrapper (Flutter has no `TextDirection.auto`
    enum; the ambient `Directionality` from `MaterialApp` flows
    through, which is the correct semantic for "let the host decide")
  - `dir: slot: layout-direction` → `Directionality(textDirection:
    layoutDirection, child: DataTable(...))`. The slot is expected to
    evaluate to a `TextDirection` Dart value; the slot name passes
    through `is_safe_dart_identifier` so it can't smuggle bad source
    into the format string.
  - Unknown keywords drop silently — the allow-list is the security
    gate against attacker-controlled keywords sneaking `, child:
    pwn(),` style payloads into the generated source. The bare
    `DataTable` still renders so the rest of the layout is intact.
  - 7 new tests cover the a11y gate (must lower to native
    `DataTable`, not a `Container`/`Row` substitute), the three
    allow-listed keywords (including the `auto` no-wrap case which
    is uniquely Flutter), the slot-ref interpolation, the silent-
    drop on an injection-style unknown keyword, and a bare-table
    regression guard. Total tests: 37 (was 30).
  - Full sub-tag (`HostTableHead` / `HostTableBody` / `HostTableFoot`
    / `HostTableColGroup`) walk is still a follow-up — the
    `DataTable` body remains a `columns: const [], rows: const []`
    placeholder, matching the existing UI29 §2.1 stub. The RTL
    contract is independent of the sub-tag walk and can ship now.

## [0.2.0] - 2026-05-23 — UI29-4 host primitives

### Added

- **`HostLink` (kernel primitive #19)** → `InkWell(onTap:, child:
  Text(...))`. Flutter has no built-in URL launcher, so the
  `onTap` body carries a `/* TODO: launchUrl(Uri.parse(href)) */`
  comment that hosts wire to the `url_launcher` package. The
  `external: false` keyword suppresses the `launchUrl` comment
  for in-app routing (host handles via the `onActivate`
  dispatch). The `target` keyword (`same`/`new-tab`/`parent`/
  `top`) is preserved in the comment as a hint to the host.
- **`HostTooltip` (kernel primitive #20)** → `Tooltip(message:,
  child:)`. Flutter's built-in tooltip handles hover (web /
  desktop) and long-press (mobile) automatically; the overlay
  layer escapes parent clipping by default. Single-child shape
  matches the spec exactly.
- **`HostNumberInput` (kernel primitive #21)** → `TextField`
  configured with `keyboardType: TextInputType.number`, which
  surfaces the numeric keypad on iOS/Android. `min`/`max`/`step`
  numeric literals are emitted as a `/* min: N, max: N, step: N
  */` range hint; full `inputFormatters` clamping is a follow-up.
  `onChange` wires `onSubmitted` (commit-on-Enter), matching the
  UI29-4 spec's rejection of per-keystroke dispatch for numeric
  fields.
- New `find_number_prop` helper for fishing `LayoutPropValue::
  Number(f64)` values out of a node's prop list. Used by
  HostNumberInput for `min`/`max`/`step`.
- 10 new tests covering all three primitives plus a security
  regression test for Dart-string injection through HostLink's
  `href` slot (`$`-interpolation and `"`-quote escaping).

### Security notes

- `href`, `label`, `text`, and `placeholder` slot/string values
  all flow through `escape_dart_string`, which handles `\`, `"`,
  `$` (critical: Dart interpolates `$ident` inside double-quoted
  strings), `\n`, and `\r`.
- `external`/`target` keywords are validated against allow-lists
  before splicing into block comments — a malicious keyword like
  `false*/dispatch(evil())/*` is impossible because the grammar
  layer guarantees keywords are bare identifiers and we further
  match against `same|new-tab|parent|top` / `false|true`.
- `min`/`max`/`step` are `LayoutPropValue::Number(f64)` from the
  IR, never strings — no injection possible by construction.

## [0.1.0] - 2026-05-23 — initial release

Brand-new backend bringing Flutter (Dart) as the seventh supported
target alongside React, SwiftUI, Qt, HTML, WebComponent, and XAML.

### Added

- `pipeline::from_pipeline(interface, layout, style)` — the standard
  three-IR entry point matching the other six backends' signatures
  so `mosaic-compile` and `mosaic-package-artifact-builder` can
  dispatch uniformly.
- `PipelineEmitResult` / `PipelineEmitError` mirror the cross-backend
  shape (output string + component name; same error variants:
  `ComponentNameMismatch`, `UnsafeSlotName`, `UnsafeEmitName`,
  `UnknownPrimitive`).
- Dart `StatelessWidget` output: one class per Mosaic component, plus
  a sealed `<Component>Event` base class and one subclass per
  declared emit. Constructor uses Dart's named-required parameter
  syntax so component-by-component prop changes don't break
  call-site order.
- **Slot → Dart field type map:**
  - `text` / `image` / `color` → `String`
  - `number` → `double`
  - `bool` → `bool`
  - `node` → `Widget`
  - `list<T>` → `List<dart-type-of-T>`, including the nested
    `list<list<text>>` case → `List<List<String>>` (motivated by
    VisiCalc's viewport-rows slot)
  - `Component(Name)` → `Name` (assumes the host imports that
    Dart class)
- **Kernel primitive lowerings (v0.1 coverage):**
  - `Box` → `Container` (with single-child / multi-child / styled
    variants)
  - `Row` / `Column` / `Stack` → Flutter's same-named widgets with
    a `children: [...]` list
  - `Text` → `Text("...")` or `Text(<slot>)`
  - `Image` → `Image.network(...)` for `http(s)://` sources,
    `Image.asset(...)` otherwise
  - `Spacer` → `SizedBox(width: 8, height: 8)`
  - `Divider` → `Divider()`
  - `Icon` → `Icon(Icons.<name>)` (`source:` keyword feeds the
    `Icons` constant lookup)
  - `HostInput` → `TextField` with `TextEditingController(text:
    <slot>)` and an `InputDecoration(hintText: ...)` when
    `placeholder` is bound
  - `HostButton` → `ElevatedButton(onPressed:, child: Text(...))`;
    `disabled: true` → `onPressed: null`
  - `HostCheckbox` → `Checkbox(value:, onChanged:)` with optional
    sibling `Text` label inside a `Row`
  - `HostRadio` → `Radio<String>(value:, groupValue:, onChanged:)`
    with optional sibling `Text` label
  - `HostScroll` → `SingleChildScrollView(child:)` (wraps multi-
    child case in a `Column`)
  - `HostDialog` → placeholder `SizedBox.shrink()` (full
    `showDialog` plumbing deferred to a follow-up — see
    "Deferred" below)
  - `HostTable` → placeholder `DataTable(columns: const [], rows:
    const [])` (full sub-tag walk deferred — see "Deferred")
- **Style → widget property pass:**
  - `padding: N` / `padding: Npx` → `padding: const EdgeInsets.all(N)`
  - `width: N` / `height: N` → matching Container properties
  - `background-color: #RRGGBB` → `color: const Color(0xFFRRGGBB)`
  - Unknown / unsupported style props are silently dropped (TODO:
    surface as Dart comments).
- **Dart-safety helpers:**
  - `escape_dart_string` handles `\\`, `\"`, `\$` (Dart double-
    quoted strings interpolate `$ident`), and `\n` / `\r`.
  - `is_safe_dart_identifier` checks the ascii identifier shape AND
    rejects Dart reserved keywords (`class`, `if`, `return`, etc.)
    so a kebab-case slot like `class` doesn't compile to broken
    Dart.
- **Test coverage:** 17 unit tests cover the smoke path, slot-type
  matrix (required vs optional, scalar vs list), event-union
  shape, container nesting, every wired host primitive, the
  component-name-mismatch error path, the dart-string escape
  regressions, the reserved-keyword rejection, and the style →
  Container args mapping.

### Deferred to follow-up PRs

- **HostDialog** full `showDialog` plumbing. Flutter's `showDialog`
  is imperative — it requires a `BuildContext` and is called from
  a callback, not declared in the widget tree. The cleanest shape
  uses either `flutter_hooks`' `useEffect` (third-party package)
  or a `StatefulWidget` wrapper with `WidgetsBinding.instance
  .addPostFrameCallback`. v1 emits a placeholder; the follow-up
  picks one of those two approaches.
- **HostTable** sub-tag walk (`HostTableHead` / `HostTableBody` /
  `HostTableFoot` / `HostTableColGroup`). Flutter's `DataTable` has
  a richer API than the other backends' table widgets (it carries
  per-column sort logic, per-row selection, etc.); a separate PR
  will design the lowering surface.
- **`For` / `If` / `Else`** meta-primitives. Flutter widget trees
  are expressions, not statements, so Dart's `if`/`for` need to be
  used in *collection-literal* contexts (`children: [for (var x
  in xs) Widget(x)]`). Wiring this through the recursive walker
  needs the same sibling-pair lookahead the other backends use;
  deferred so the v1 PR stays reviewable.
- **`onTap` / `onChange` / `onToggle` / `onSelect` dispatch wiring**
  currently writes `/* TODO: dispatch <name> */` comments inline.
  The full dispatch payload synthesis (mapping `event.target.value`
  / Checkbox's `bool?` callback into the right `<Component>Event<Case>`
  constructor invocation) needs the component-name to be threaded
  down to the per-primitive emitters. The plumbing is a small but
  cross-cutting refactor; deferred to the follow-up.
- **Theme integration.** Generated widgets ignore
  `Theme.of(context)`. Hosts that want themed colours should wrap
  the generated widget in a `Theme(...)` override for v1.
- **Component reference resolution.** PascalCase tags that aren't
  kernel primitives emit a labelled `SizedBox.shrink()` placeholder
  instead of importing + instantiating the referenced component.
  Package-resolver wiring follows the same pattern the other
  backends use; it'll land in the next iteration.
