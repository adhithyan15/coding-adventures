### Fixed -- `emit_styled_box` never applied `border-radius` (#15225)

This builder assembled its `BoxDecoration` from background, border and
elevation and simply never looked at the radius, so a styled box came out
SQUARE on Flutter however it was authored -- a plain `border-radius: 8` was
dropped here just as surely as a percentage one.

The same property visibly works elsewhere in the same file because
`emit_container` does read it. Two writers, and only one of them had it.

**This is not scoped to the percentage case.** Measured across the products:
engram-app gains 24 radii it was silently losing, and task-app 17, none of
which involve a percentage at all. Every changed line is the same
`BoxDecoration` with a radius inserted -- verified by stripping the
insertion and asserting the line is then byte-identical to before.

An unreadable radius is dropped rather than coerced to `0`, since a zero
radius is a square.

**The radius is never paired with a per-edge border.** Flutter's
`Border.paint` throws *"A borderRadius can only be given on borders with
uniform colors"* the moment a non-uniform `Border(...)` carries one, and
asserts separately on a hairline side -- taking out the widget and
everything above it in any debug or profile build. Before this change the
builder emitted no radius at all, so the pairing was unreachable; adding one
without the gate turns an authored per-edge border plus a radius into a
runtime crash.

That was not hypothetical. **Three widgets in task-app** author exactly that
combination, and an ungated version emitted all three. Found in security
review, before it shipped.

