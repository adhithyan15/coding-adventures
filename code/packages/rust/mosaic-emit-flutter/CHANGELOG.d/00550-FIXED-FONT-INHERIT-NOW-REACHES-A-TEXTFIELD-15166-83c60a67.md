### Fixed -- `font: inherit` now reaches a `TextField` (#15166)

A Flutter `TextField` does **not** read the enclosing `DefaultTextStyle` the
way a `Text` does -- it falls back to the Material theme. So an input
authoring `font: inherit` had no way to follow the text around it, and the
grid's inline editor rendered at the theme's font while the cell beside it
rendered at the authored one.

Measured in a real `flutter test` with a real font, inside the cell's own
text context:

| editor | height | delta |
| --- | --- | --- |
| display `Text` | 19.0 | — |
| without the inherited font | 24.0 | **+5** |
| with it | 20.0 | **+1** |

The residual 1px is the editable's own line box and is not reachable from a
stylesheet.

`DefaultTextStyle.of(context)` cannot substitute for this: the `context` in
scope at the input is the widget's own, which sits **above** the merge the
emitter just wrote. So the style is computed before the child is emitted and
threaded down on `TableCtx`, beside the `sheet_text_*` fields that already
work this way.

Three things security review found, all inside this change and all inert for
the products:

- **The threaded copy is validated separately from the merge copy.** The
  merge reads `font-size` through `parse_pixel_value`, whose contract is "0
  on anything unreadable" -- harmless for a `Text`, which the theme still
  sizes, but on a `TextField` `fontSize: 0` is an editor the user cannot see
  or place a cursor in. Threading the merge copy verbatim walked around the
  invariant the explicit path already pins.
- **A `For` resets it.** The threaded style is TEXT, and a loop rebinds the
  identifiers inside it -- so carrying it across would change the loop's
  shape (the emitters pick `(_)` vs `(item)` by scanning the emitted body
  for those identifiers) and make the input's copy evaluate a predicate
  against the loop binding while the enclosing merge evaluates it against
  the outer one. `emit_host_table` resets it for the same reason.
- **The test was vacuous.** It asserted over the whole output, and the
  `DefaultTextStyle.merge` line has always contained the same byte
  sequence -- so it passed with the feature entirely reverted. It asserts on
  the `TextField`'s own slice now.

**Only an explicit `inherit` counts.** CSS does not give a text input the
surrounding font by default -- which is exactly why `font: inherit` is a
standard reset -- so applying the enclosing style to every unstyled input
would diverge from the web backends rather than agree with them. Emitted
output gains exactly one `style:` argument, on the inline grid editor.

