### Fixed — a Row-child `Text` could not shrink, and unboundedness was mis-detected (#14857)

A Flutter `Row` measures a non-flexible child with an **unbounded** max width,
so a `Text` never wraps — it extends the row until the row throws
`A RenderFlex overflowed by N pixels`. UI59 §11 records the same rule on
Compose, where it starves a sibling to zero instead; Flutter is the louder of
the two.

A `Text` that is a direct Row child now gets `Flexible` (default loose fit): it
may take up to its share and measures to its content when smaller, so a short
`Text` is unaffected and a long one wraps rather than overrunning. Only a
`Text`, because a Text is the child that can reflow.

**And a second, load-bearing bug found while doing it.** The existing
`direct_row_accepts_flex` asked only `!ctx.direct_row_child` — "my parent is
not a Row" — and so treated one level of indirection as bounded. Trestle's
`Row [subline]` sits at `Row [topbar] > Column [title-block] > Row [subline]`,
and was judged flex-accepting while measuring unbounded. Putting a flexible
child in it throws
`RenderFlex children have non-zero flex but incoming width constraints are unbounded`
— an assertion, not a layout quirk. Unboundedness now propagates through
intermediate containers as `width_bounded`.

The component root had to be marked bounded explicitly: it is built with
`..TableCtx::default()`, and `bool::default()` is `false`, which suppressed
flex everywhere.

Emitted wraps: TaskApp 18, Engram 11, VisiCalc 6. `flutter analyze` reports no
issues and TaskApp's real conformance test passes.

**This does not unblock #14851.** Reviving the closed `max-width` change on
top still throws at exactly 316px, because the task row's chain is judged
unbounded and its `Text` gets no `Flexible` — the `ConstrainedBox` the cap adds
re-bounds the width at runtime, and the static analysis does not know. The next
step is for `part_max_width` to mark its subtree bounded before emitting it.

