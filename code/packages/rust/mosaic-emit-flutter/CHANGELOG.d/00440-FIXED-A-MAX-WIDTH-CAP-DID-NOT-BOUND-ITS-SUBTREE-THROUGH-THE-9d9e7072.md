### Fixed — a `max-width` cap did not bound its subtree through the Row-child reset (#14902)

A `ConstrainedBox(maxWidth: ..)` bounds its subtree at runtime, but the
emitter's boundedness analysis did not treat the cap as a bound. A `Row` inside
a capped subtree therefore judged itself unbounded and refused to hand its
children `Flexible`.

The cap is now a **source** of boundedness rather than an inherited flag:

```rust
let width_bounded = width.is_some()
    || part_max_width(node, part_styles).is_some()   // <- new
    || part_flex_grow(node, part_styles).is_some()
    || (!ctx.direct_row_child && ctx.width_bounded);
```

That last clause is why the obvious version of this fix does not work, and it
was tried first: threading `width_bounded: true` down from `part_max_width`
before emitting the subtree (dropped in #14901). A capped node is itself usually
a *direct Row child*, so the reset discarded the flag the moment it arrived. The
reset is correct for an inherited flag — a direct Row child genuinely is
unbounded in the general case, which
`nested_if_inside_row_branch_column_resets_direct_row_context` protects. An
explicit cap is different in kind: the `ConstrainedBox` is emitted a few lines
later, unconditionally, so the bound is a fact about the output rather than an
assumption about the context.

**No shipped product output changes.** Trestle's one cap and Engram's seven all
sit below their Rows rather than above them, and the generated Dart for both is
byte-identical with and without this change — verified rather than assumed,
because that is exactly the check the inert version of the fix also passed. The
difference is that this version changes the output for the shape it targets:
`a_max_width_cap_bounds_its_subtree_through_the_row_child_reset` fails without
it, and the fixture deliberately puts the capped node where boundedness would
otherwise be lost.

