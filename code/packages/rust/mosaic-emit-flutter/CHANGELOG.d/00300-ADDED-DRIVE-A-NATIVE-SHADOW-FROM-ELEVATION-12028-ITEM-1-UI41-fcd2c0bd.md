### Added - drive a native shadow from `elevation` (#12028 item 1, UI41)

Fifth and final backend in the elevation-tokens cascade (mosstyle-compiler
contract → XAML → Compose → Qt → **Flutter**; SwiftUI stays out of scope,
tracked separately in #13206). A part declaring `elevation: raised;` or
`elevation: overlay;` now gets a real native shadow instead of the shadow
silently vanishing.

Unlike Compose (one shared `compose_box_style`) but similar to Qt (three
separate paint-line builders), this crate has **two** distinct styling
mechanisms with no shared function between them:

- `Box` → `Container`, via `emit_styled_box`'s `BoxDecoration` builder —
  elevation adds a `boxShadow: [BoxShadow(...)]` entry alongside the
  existing `color`/`border`. Also updated `part_has_decoration` to route a
  part with ONLY `elevation` set (no background/border) through the
  decoration path, since `boxShadow` can only live inside `BoxDecoration`,
  not the lighter `style_to_container_args` fast path.
- `Row`/`Column`/`Stack` have **no** decoration mechanism of their own —
  today they lower straight to their native Flutter widget with no
  `Container` wrapper at all, so a part with ONLY `elevation` (matching
  real `task-card`, a `Column`-tagged part) would have nothing to attach a
  shadow to. Elevation now force-wraps them in a `Container(decoration:
  BoxDecoration(boxShadow: [...]))` around the original widget — narrowly
  scoped to elevation, not a general fix for `Row`/`Column`'s missing
  background/border support (a separate, larger pre-existing gap, correctly
  out of scope here).
- `HostButton` uses Flutter's own first-class Material `ButtonStyle.
  elevation` (a real, depth-scaling shadow) instead of `BoxShadow` — a
  materially better fit than the `BoxShadow` hack every other primitive
  needs, verified via a real widget-test golden render showing the shadow
  visibly deepen at `elevation: 0.0 → default → 8.0 → 24.0` (the harder,
  more solid-edged look at higher values, compared to `BoxShadow`'s softer
  blur, is a known characteristic of `flutter test`'s software Skia
  rasterizer for `Material`/`PhysicalModel` shadows — real GPU-rendered
  apps render the same elevation value as the intended soft drop shadow).
- `HostDraggable` delegates its whole body straight to
  `emit_container(node, "Container", ...)`, reading `node.part_name`
  directly — elevation flows through for free with **no**
  `HostDraggable`-specific code, unlike XAML, which hit a real
  XamlCompiler bug on this exact primitive and needed a workaround.
- `HostSurface` was left out of scope: it has no `part_styles` parameter
  in its signature at all today (a separate, pre-existing "no styling
  support whatsoever" gap, not just missing `elevation`), and no real
  component declares `elevation` on a `HostSurface` part.

Shadow constants: `boxShadow` uses `color: 0x40000000`, `blurRadius: 8`
(raised) / `24` (overlay), `offset: Offset(0, 4)` / `Offset(0, 16)` — the
4/16 vertical-offset numbers match `mosaic-emit-xaml`'s
`ElevationTier::translation_z`, `mosaic-emit-compose`'s `dp()`, and
`mosaic-emit-qt`'s `shadowVerticalOffset`, so a part reads the same "how
far off the surface" intent on every backend; `blurRadius` was tuned
independently via a real `flutter test` golden render — an initial
`overlay` attempt at `blurRadius: 16` rendered as a flat, solid grey block
rather than a soft shadow (the blur radius was too small relative to the
16px offset for a wide/tall card to feather anywhere but the edges), fixed
by bumping to `blurRadius: 24`. `HostButton`'s native `ButtonStyle.
elevation` uses the plain Material dp values `4.0`/`16.0`.

Verified against the real toolchain: `flutter analyze` on a real scratch
Flutter project (Flutter 3.47.0 / Dart 3.13.0) confirmed
`BoxShadow`/`BoxDecoration`/`ButtonStyle.elevation`/`WidgetStatePropertyAll`
all compile cleanly; a real `flutter test --update-goldens` render (not
just static analysis) confirmed both `BoxShadow` and `ButtonStyle.
elevation` produce a real, visible shadow — this is also where the
`overlay`-tier `blurRadius` bug above was caught and fixed. All four real
components (`TaskApp`, `ProjectNav`, `Notes`, `Calendar`) build clean via
`mosaic-compile pkg --backend flutter --profile native-complete` — zero
degradations; the real generated `TaskApp.dart` produces a shadow
(`boxShadow` or `ButtonStyle.elevation`) for all 13 real
`elevation`-declaring parts (16 total instances — matching Compose's and
Qt's identical counts against the same source, a third independent
cross-backend consistency check). `flutter analyze` on the real generated
`TaskApp` `--emit-project` scaffold (with `flutter pub get` run for real)
is clean, zero issues.

A full **live widget mount** of the complete real `TaskApp` (via
`flutter test` against the real `--emit-project` output) was attempted but
hit a genuine, pre-existing runtime layout crash ("RenderFlex children
have non-zero flex but incoming width constraints are unbounded") deep in
`TaskApp`'s own `Row`/`Column` structure — confirmed via a `git stash`
comparison to be present on the pre-elevation baseline too, i.e.
unrelated to this change and out of scope for this PR. Filed as
[#13429](https://github.com/adhithyan15/coding-adventures/issues/13429)
rather than silently worked around. Verification for elevation
specifically relied instead on the isolated widget-test golden renders
above (which mount real `BoxShadow`/`ButtonStyle.elevation` widgets
directly, sidestepping the unrelated crash) plus `flutter analyze` and
the grep-based full-package coverage check.

