### Changed -- the two box writers now share one decoration builder

`emit_container` and `flutter_box_style` each built their own
`BoxDecoration(..)`. Every time the two drifted, a product lost something:

- **#15225** -- `border-radius` was taught to one writer and not the other.
- **#15246** -- the gap that survived it: containers rendered square, and a
  comment in the *fixed* writer asserted the other one handled the radius.
- **#15249** -- padding, wrong in two different ways at once: `EdgeInsets.all`
  collapsed it in one writer while `symmetric(horizontal:)` dropped the
  vertical axis in the other.

Three defects, one cause. Qt, XAML, SwiftUI and React each lower a part's box
in one place and none of them has had this class of bug, so the decoration now
has one writer too: a property taught to `flutter_decoration_parts` is taught
to both callers at once.

**No emitted output changes.** Flutter artifacts for all four Mosaic product
packages are byte-identical against `origin/main` -- 496,013 bytes, zero diff
lines -- and the comparison was falsified by planting a marker inside the new
builder, which moved 453 lines across 112 sites.

The background is deliberately **not** folded in. The two callers genuinely
disagree about it: `emit_container` omits `color:` entirely when nothing
authored a background and defaults to `Colors.transparent`, while
`flutter_box_style` always emits it and defaults to `null`. Unifying that
would change what is emitted, which a refactor must not do -- so the caller
computes the expression and only the argument ORDER is shared.

The border condition is now a single `flutter_has_border` predicate, asked by
both the wrapper gate in `emit_container` and the border arm of the shared
builder. Those were two textually identical copies of one question -- which is
exactly the shape that produced the three defects above -- so proving they
matched today would not have stopped them drifting tomorrow.

The elevation tier is likewise **passed in** rather than recomputed, for the
same reason: `emit_container`'s wrapper gate already tests its own `elevation`
binding, and a second evaluation inside the builder would be two answers to
one question.

Note this is the *decoration*; #15249 already gave the two writers a shared
padding resolver. What remains separate is each writer's sizing and its
wrapper decision, which are genuinely different jobs.

