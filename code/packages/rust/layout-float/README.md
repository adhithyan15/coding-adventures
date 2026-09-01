# layout-float

`layout-float` owns Venture's host-neutral `ext["float"]` contract and the
geometry needed by a block formatting context. It resolves `float: left/right`,
`clear: left/right/both`, shrink-to-fit widths, exclusion bands, and float
containment without depending on HTML, paint, or a native UI toolkit.

The block engine supplies measured boxes and consumes the returned coordinates.
This keeps CSS mapping in `html-to-layout`, flow geometry in this package, and
painting in `layout-to-paint`.

```rust
use layout_float::{Clear, ExclusionSpace, FloatSide};

let mut space = ExclusionSpace::new(320.0);
let placement = space.place(FloatSide::Left, 0.0, 96.0, 72.0);
assert_eq!(placement.x, 0.0);
assert_eq!(space.clearance_y(Clear::Left, 0.0), 72.0);
```

Malformed extension values retain standards-neutral defaults and can be
reported through `diagnostics`. Oversized floats are clamped to the formatting
context instead of producing negative or non-finite geometry.
