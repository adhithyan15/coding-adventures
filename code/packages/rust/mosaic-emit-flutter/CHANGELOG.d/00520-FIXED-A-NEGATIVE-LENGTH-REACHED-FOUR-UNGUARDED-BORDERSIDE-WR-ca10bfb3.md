### Fixed -- a negative length reached four unguarded `BorderSide` writers (#15160)

Flutter's `BorderSide` constructor is `assert(width >= 0.0)`, so a negative
authored width produces Dart that type-checks and then **throws when the
widget builds**, taking out that widget and everything above it in the
tree. It is a runtime crash reachable from any stylesheet, not a rendering
glitch.

`per_edge_border_expr` had guarded its own path for a while. Four other
writers took their width straight from `parse_pixel_value`, which happily
returned `-5`:

| site | construct |
| --- | --- |
| styled container | `border: Border.all(color: .., width: {w})` |
| `emit_styled_box` | `border: Border.all(color: .., width: {w})` |
| button shape | `side: BorderSide(width: {w})` |
| `path_paint` | `Border.all(color: {stroke}, width: {w})` |

`Border.all` builds a `BorderSide`, so all four hit the same assert.

The guard is central now: `parse_pixel_value` rejects negatives and falls
back to `0`, the same answer it already gave for anything unreadable. That
also covers `EdgeInsets`, `SizedBox` and every other length sink at once.

**Audited before centralising**, because a shared helper is the wrong place
for a rule that does not hold everywhere. Every property reaching it is
non-negative geometry -- gap, padding, width, height, min-height,
border-width, border-radius, font-size, stroke width -- and CSS forbids a
negative for each. No margin, inset, offset or letter-spacing is lowered
through it, so nothing that legitimately admits a negative loses one.
(`top`/`left`/`right`/`bottom` appear nearby only as per-edge border names.)

**A dead check the central fix would have created.** `per_edge_border_expr`
tested `w.starts_with('-')` on the PARSED value. Once `parse_pixel_value`
never returns a leading `-`, that check can never fire, and the edge would
have been silently emitted at width 0 instead of skipped -- a different
answer, since skipping lets the shorthand cascade in. It now reads the
authored text. An existing test caught this, which is the only reason it is
not in this release.

**Two corrections from security review, both in this change.**

A central rule is only safe where it holds everywhere, and it did not quite:

- `fixed_pixel_length` delegates here, and it exists *because* the `0`
  fallback collapses a subtree into a zero-width box -- its own doc records
  catching that in a real `flutter test` render. Centralising the guard
  made `width: -5px` on a `Row` part go from `SizedBox(width: -5)`, which
  trips Flutter's `debugAssertIsValid` **loudly**, to `SizedBox(width: 0)`,
  which **silently** eats the subtree. Trading a loud failure for a silent
  one is the wrong direction, so that site now drops a negative outright,
  the same way it already drops a `%`.
- IEEE `-0.0 >= 0.0` is true and Rust prints it `-0`, so `border-width:
  -0px` emitted `width: -0` from a legal input. Not a crash -- Dart reads
  it as `-0.0`, which satisfies the assert -- but it falsified the very
  invariant these tests assert. The sign of zero is normalised.

A second review round found two more of the same shape, both now closed:

- `strict_pixel_length` -- the sibling helper feeding `BorderSide` and
  `TextStyle` on the `HostInput` path -- had the identical negative-zero
  leak, so `border: -0px solid #ff0000` emitted `width: -0` and
  `font-size: -0px` emitted `fontSize: -0`.
- `part_max_width` **validated the parse and emitted the authored text**,
  so its guard proved nothing about what shipped. Rust's float grammar
  accepts a leading `+` and Dart has no unary `+` on a literal, so
  `max-width: +760px` passed validation and emitted `maxWidth: +760` -- a
  hard compile error from one authored value. A 22-digit literal got
  through the same way. It now emits the parsed value, with the same
  magnitude cap as every other length path.

And one of the new tests was **vacuous**: it asserted
`!out.contains("EdgeInsets.all(-")` on a fixture whose path emits
`EdgeInsets.symmetric`, so it could never fail and read as coverage it did
not provide. It now asserts what that path actually emits.

**No product change.** Zero of the 3,536 length declarations in the
authored `.msl` corpus is negative, and emitted Flutter output for
task-app, visicalc and engram-app is byte-identical. This closes a latent
trap; it does not fix a live defect.

