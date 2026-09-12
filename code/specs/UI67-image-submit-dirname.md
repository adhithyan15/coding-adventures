# UI67: Image Submit Coordinates and Directionality Fields

Image submit inputs are successful only when they are the active submitter.
At the input's document-order position, the planner emits `name.x` followed by
`name.y`; an unnamed image emits `x` and `y`. Pointer activation floors finite,
non-negative control-local coordinates and clamps them to `u32`. Keyboard and
accessibility activation use `(0, 0)`, so hosts never invent coordinate policy.

Text, search, and textarea controls with a non-empty `dirname` emit one
additional entry immediately after their ordinary values. Explicit `dir=ltr`
or `dir=rtl` wins, direction otherwise inherits, and `dir=auto` is recomputed
from the live value through the shared Unicode bidi analyzer with inherited
direction as the no-strong-character fallback. A `dirname` entry remains
successful when its owner has no ordinary `name`.

The browser control model owns retained direction metadata and accessibility
activation. The submission planner owns coordinate expansion, direction entry
ordering, entry bounds, and encoding. Venture's session supplies only the
control-local pointer point or the semantic keyboard/accessibility action, so
native and web hosts share behavior without toolkit-specific serialization.
