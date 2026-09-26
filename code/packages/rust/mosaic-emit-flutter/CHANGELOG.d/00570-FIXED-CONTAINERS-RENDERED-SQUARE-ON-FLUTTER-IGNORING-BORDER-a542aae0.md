### Fixed -- containers rendered square on Flutter, ignoring `border-radius`

`emit_container` -- the writer that runs for a styled Row, Column or Box
wrapper -- read background, border and elevation and **never looked at the
radius**. Every container-shaped part rendered with square corners however it
was authored.

Trestle's `task-card` authors `border-radius: 13` and emitted:

```dart
decoration: BoxDecoration(
  color: const Color(0xFF252019),
  border: Border.all(color: const Color(0xFF352E25), width: 1),
  boxShadow: [BoxShadow(...)]),          // no borderRadius
```

**#15225 fixed this in the other writer only**, and left a comment there
stating that `emit_container` already handled the radius. That claim was
false, and it is corrected in this change -- it is exactly the sentence that
would stop the next reader from looking.

Measured in the emitted artifact rather than inferred: across Trestle's dark
theme, authored radii 9, 10 and 13 appeared **zero** times in the generated
Dart, and 20 appeared twice against eleven authored. Sixteen radii are
restored.

**Not every authored radius comes back, by design.** `board-card` and
`board-card-crit` author a 3px left accent alongside `border-radius: 9`, and
Flutter's `Border.paint` throws *"A borderRadius can only be given on borders
with uniform colors"* the moment a non-uniform `Border(...)` carries one --
taking out the widget and everything above it in any debug or profile build.
Those two stay square, gated exactly as the other writer gates them. Radius 9
is therefore still absent from the output, and that is correct.

Qt does not share this gap: it emits `radius: 13` six times for the same six
authored declarations. The defect was Flutter's container writer alone.

Nothing caught this. The suite passed identically before and after the fix,
because no test asserted on this writer's radius at all; two now do, and the
gate test carries the uniform case as its control so neither can pass
vacuously.

