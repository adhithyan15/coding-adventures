# Changelog — mosaic-emit-flutter

All notable changes to `mosaic-emit-flutter` will be documented in
this file.

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
