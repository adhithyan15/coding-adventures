### Added - `Path` drawing primitive lowering (#12028 item 3, UI39)

`Path [name] (kind: circle|line|curve, ...)` now lowers to real Flutter
vector geometry instead of reporting `primitive.path-unimplemented` on
every build. `circle` reuses `Container(decoration: BoxDecoration(shape:
BoxShape.circle, ...))` directly — `BoxDecoration`'s native `color`/
`border` properties already match `Path`'s `background`/`border-color`+
`border-width` 1:1, the same reuse Qt's `Rectangle` lowering made for
the same shape. `line`/`curve` have no equivalent native declarative
widget, so they lower to `CustomPaint` backed by one of two small
painter classes (`_MosaicLinePainter`, `_MosaicCurvePainter`) emitted
once per file when `Path` is present anywhere in the tree — mirrors the
existing `emit_drag_helpers`/`emit_dialog_helper` "shared top-level
helper, emitted once" pattern. `arc` is a stretch goal not implemented
in this PR; it hard-errors with a named "not yet supported" message
rather than silently rendering nothing, matching the XAML and Qt
lowerings' posture for the same gap.

Only `circle` needs explicit placement: a `Container` always draws from
its own top-left corner, so two overlapping circles (the crescent moon)
need `Positioned(left: cx - r, top: cy - r, ...)` to land at their
authored centers instead of collapsing onto Flutter `Stack`'s default
top-left-aligned, un-positioned-child placement. Unlike WinUI's
`Margin` (valid in any panel) or QML's implicit-`Item` positioning
(valid in any non-Layout parent), `Positioned` only type-checks as a
*direct* `Stack` child in Flutter, so `emit_path` only wraps in
`Positioned` when a new `TableCtx.direct_stack_child` field (threaded
the same way `direct_row_child` already is, set in `emit_container`
when `widget == "Stack"`) confirms the immediate parent actually is
one — a standalone circle with no `Stack` parent renders unwrapped,
sized to its own `2r × 2r` box, since `Positioned` would otherwise
panic at runtime outside a `Stack`. `line`/`curve` never need this:
their own drawn geometry already encodes the correct offset from
`(0, 0)` (absolute, un-offset canvas coordinates, mirroring Qt's
`ShapePath`), and an un-positioned `Stack` child already renders at
`(0, 0)` by default (`Stack`'s own `alignment` default is top-left).

Coordinate props (`cx`/`cy`/`r`/`x1`/`y1`/`x2`/`y2`) accept a literal
`Number` only; a `SlotRef`/`Expr`-bound coordinate is a clear compile
error, not a silent 0 — full data-driven binding is future work, the
same not-yet-landed gap the XAML and Qt lowerings both note.

Verified against the real toolchain: a `mosaic-compile pkg --backend
flutter --profile native-complete` build of the crescent-moon shape
(two overlapping circles, a line, and a curve) produces
`nativeComplete: true` with zero degradations, and the generated
`Moon.dart` passes `dart analyze` (via a minimal one-time `pubspec.yaml`
scaffold pulling in the `flutter` SDK dependency) with "No issues
found!".

