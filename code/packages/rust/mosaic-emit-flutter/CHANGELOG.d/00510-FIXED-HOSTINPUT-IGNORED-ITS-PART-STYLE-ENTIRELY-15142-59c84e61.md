### Fixed -- `HostInput` ignored its part style entirely (#15142)

`emit_host_input` took `_part_styles` and never read it, so no authored
style reached a text input on any Flutter build. That is a geometry bug,
not a cosmetic one. A bare `TextField` keeps Material's default decoration
-- an underline border and a 48dp minimum touch target -- and, because a
`TextField` does **not** inherit the enclosing `DefaultTextStyle` the way a
`Text` does, it also renders at the Material theme's font rather than the
one beside it.

Measured with a real `flutter test` and a real font loaded, a display
`Text` against an editor in the same cell:

| editor | height | delta |
| --- | --- | --- |
| bare `TextField` (before) | 48.0 | +28 |
| with the part's decoration | 24.0 | +4 |
| with its decoration and text style | 21.0 | +1 |

In a grid that made the editing row taller than every display row, which
is the Flutter half of #15048.

What now lowers, from the part named on the node:

| authored | Dart |
| --- | --- |
| `padding` | `isDense: true, contentPadding: EdgeInsets.all(N)` |
| `border: 0` / `none` | `border: InputBorder.none` |
| `border: W solid C` | `OutlineInputBorder(borderSide: BorderSide(..))` |
| `background` | `filled: true, fillColor: ..` |
| `color`, `font-size`, `font-family`, `font-weight` | `style: TextStyle(..)` |

`isDense` matters as much as the padding: without it Material enforces the
48dp minimum that no `contentPadding` can undercut, and it is most of the
+28.

**What this does not do.** The residual +1 is the editable's own line box
and is not reachable from a stylesheet. A part that authors `font: inherit`
rather than an explicit size -- `mosaic-pkg-grid`'s `cell-editor`, for
instance -- still lands at +4, because honouring `inherit` needs the style
from a context *below* the generated `DefaultTextStyle.merge`, which a
`Builder` would have to supply; filed separately. Unrecognised properties
are still dropped silently, because this emitter has no style-drop
reporting at all (#12022).

Two build-breaking value ranges were closed while adding these sinks, both
found in security review:

- A **negative** border width reached `BorderSide`, whose constructor is
  `assert(width >= 0.0)` -- so `border: -5px solid #ff0000` type-checked and
  then threw when the widget built, taking out the input and everything
  above it in the tree. `per_edge_border_expr` had guarded this for a while
  and the new parse reintroduced it; it now rejects negatives and falls back
  to Material's default.
- `parse_pixel_value` filtered only `is_finite`, but Rust's `Display` for
  `f64` never uses exponent notation, so a finite `1e300` expanded to a
  **301-digit** bare literal. Dart rejects that outright
  (`integer_literal_imprecise_as_double`), so one authored value stopped the
  whole generated app compiling. Values beyond the exactly-representable
  integer range now fall back to `0` like any other unreadable input. This
  helper is shared, so the fix reaches container padding and sizing too.

A third, subtler one: `font-size` was read through `parse_pixel_value`,
whose "0 on anything unreadable" fallback turned `font-size: inherit`,
`90%` or `0.9rem` into `fontSize: 0` -- Dart that compiles and then renders
the input's text at zero size, invisible. Unreadable lengths on this path
are now dropped so the theme's size survives. Both new length sinks share
one `strict_pixel_length`, which rejects unreadable, negative and absurd
values alike.

The CSS-wide keywords are dropped rather than guessed at: `css_color_to_dart`
returns `None` for `inherit` and `transparent`, so neither invents a brush.
That is deliberate -- inventing one is exactly how Compose and SwiftUI paint
invisible text (#15141).

`decoration:` now has exactly ONE producer. The hint text used to be written
on its own, and a second `decoration:` argument is a Dart compile error; a
test pins the two sharing one. The Qt emitter has this same defect today
with `color` (#15155).


