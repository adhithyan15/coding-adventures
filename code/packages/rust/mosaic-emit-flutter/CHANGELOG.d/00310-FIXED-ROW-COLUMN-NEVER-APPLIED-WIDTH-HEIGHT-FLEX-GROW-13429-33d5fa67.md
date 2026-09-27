### Fixed - `Row`/`Column` never applied `width`/`height`/`flex-grow` (#13429)

A real crash: `mosaic-compile pkg --backend flutter --emit-project` on the
real `task-app` package produced a `TaskApp.dart` that hard-crashed on
mount with `RenderFlex children have non-zero flex but incoming width
constraints are unbounded`. Root cause was a pre-existing architectural
gap this session's elevation-tokens work had already partially chipped
away at (see the `elevation`-only `Container`-wrap entry below, from the
same PR sequence): `Row`/`Column`/`Stack` lowered straight to their bare
Flutter widget with **no** style application at all beyond that one
`elevation` case — a part's `width:`, `height:`, and `flex-grow:` were all
silently dropped.

The app-shell's `rail` sidebar (`width: 236px`) is a `Column`, so it lost
its width entirely; a `Row`'s non-`Expanded` children get UNBOUNDED width
by Flutter's own design (so they can shrink-wrap to content), and an
unsized `Column` propagates that same unboundedness straight down through
its own descendants — so the first `Expanded` anywhere further down that
subtree (a very common real shape: `Row(children: [Expanded(...), ...])`)
crashes outright.

Fix, generalizing `emit_container`'s existing elevation-only wrap:

- A `Row`/`Column`/`Stack` part declaring `width`/`height` (and/or
  `elevation`, unchanged) now gets wrapped in a `SizedBox` carrying those
  properties — `Container` only when `elevation` is also present, since
  `SizedBox` has no `decoration:` slot for the shadow and `flutter
  analyze`'s `sized_box_for_whitespace` lint prefers `SizedBox` when
  there's nothing to decorate.
- A percentage `width`/`height` (e.g. TaskApp's real `title-block` part,
  `width: 100%`) is left unapplied rather than parsed as `0` — a real
  regression caught by re-verifying against the actual generated file: a
  `%` value has no fixed-pixel translation inside a `Row`/`Column`
  wrapper, and naively zeroing it collapsed `title-block` into a
  `SizedBox(width: 0, ...)` that then overflowed its own content.
- A part declaring `flex-grow` that is a **direct `Row` child** (`Expanded`
  is a compile error anywhere else) is wrapped in `Expanded(flex: N, ...)`
  — both for a plain child (`emit_paired_children`) and for the single
  child of an `If`/`Else` ternary (`emit_if_dart`/`render_branch`, added
  after Notes' real `notes-editor`/`notes-empty` toggle — "show the editor
  when a note is selected, else an empty-state message" — turned out to
  declare the identical `flex-grow: 1` on *both* branches, a shape the
  first pass's plain-child-only wrap didn't reach).

Verified against the real toolchain, not just unit tests: regenerated the
real `task-app`, `mosaic-pkg-notes`, and `mosaic-pkg-calendar` packages
via `mosaic-compile pkg --backend flutter --emit-project`, confirmed
`flutter analyze` clean (no issues, including no `sized_box_for_whitespace`
regressions) and `flutter test` mounting the real generated app shell with
zero exceptions on all three — TaskApp specifically re-checked at a
realistic desktop window size (`1600×1000`, not `flutter_test`'s narrow
default), since the original crash only reproduces once the widget tree is
laid out at a size resembling how a desktop task app is actually used.
That same real-toolchain pass surfaced a second, unrelated, previously
unreachable issue — TaskApp's topbar content is too wide to fit below
roughly 1670px once `main` has a real bounded width for the first time —
filed separately as
[#13465](https://github.com/adhithyan15/coding-adventures/issues/13465)
rather than folded into this fix, since it's a content/design question,
not an emitter defect.

