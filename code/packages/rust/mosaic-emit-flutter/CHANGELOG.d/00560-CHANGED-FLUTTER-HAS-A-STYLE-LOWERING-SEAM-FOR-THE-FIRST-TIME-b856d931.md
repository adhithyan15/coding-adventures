### Changed -- Flutter has a style-lowering seam for the first time

Everything `emit_styled_box` lowers from a part's own style props now lives
in a pure `flutter_box_style(base, layers, ctx) -> FlutterBoxStyle`. Not one
byte of emitted output changes.

The point is the shape, not the saving. `emit_styled_box` takes a
`LayoutNode`, the component name and the emit declarations and returns
generated Dart **text**, so the only way to ask "what did lowering do with
`border-radius`?" was to emit an entire widget subtree and read the answer
back out of a string. Compose can answer that question directly, because
`compose_box_style` is a plain function from style props to a style -- and
that, not any shortage of care, is why Compose has a dropped-property
reporter and Flutter does not (#12022).

**What this does not do.** It adds no reporting, records no dropped
property, changes no gate and is wired into nothing. Flutter's
`styleDegradations` is still empty, and still means "nobody looked" rather
than "nothing was lost". The accumulator and the reporter that reads it
belong in one change and will land together once every Flutter writer is
covered: there are three (`emit_container`, `emit_styled_box`, and the
host-widget helpers), and a reporter that saw only one would make the
report look populated while remaining blind to the rest.

The caller keeps what needs the tree. The child is emitted by
`emit_styled_box` and appended as the last `args` entry, because building it
needs the layout and can fail; `inherited_text_style` is returned rather
than applied, because it is threaded into the `TableCtx` the child is
emitted under (#15166).

Verified byte-for-byte rather than by argument: the Flutter artifacts for
all four Mosaic product packages -- VisiCalc, Venture, Engram and Trestle --
are identical before and after (27 files, 618,546 bytes, zero diff lines).
The comparison was itself falsified by planting a marker inside the new
function, which moved 337 lines across 83 sites; an equality that cannot
fail is not evidence.

