# IMG09 — Adaptive Threshold

## Status

Amendment to `IMG03-point-operations.md`, not a new package —
`VIS00-vision-roadmap.md` claimed this slot with exactly that framing:
"an amendment to the point-ops spec." Adds one function,
`adaptive_threshold_mean`, to the existing `image-point-ops` crate.

## 1. Why this exists

`image-point-ops` already has global threshold:

```rust
pub fn threshold(src: &PixelContainer, t: u8) -> PixelContainer
pub fn threshold_luminance(src: &PixelContainer, t: u8) -> PixelContainer
```

Both apply one scalar cutoff to every pixel. That's exact and sufficient
for a clean, evenly-lit source (a rendered image, a flatbed scan) — but
unusable on an unevenly-lit real photograph: a shadow crossing half a
photographed document pushes that half's pixels below any single global
cutoff regardless of where it's set, while the lit half stays correctly
classified. `VIS00`'s L3 QR-locate pipeline needs to binarize real
photographed images before `VIS04` can scan them for finder patterns —
this is the piece that makes that survive uneven lighting.

## 2. Scope

One function, added alongside the existing point operations:

```rust
pub fn adaptive_threshold_mean(src: &PixelContainer, window: u32, t: f64) -> PixelContainer
```

This is the one function in `image-point-ops` whose output at a pixel
depends on its neighbours, not just its own value — a deliberate,
documented exception to the crate's stated "zero neighbourhood radius"
contract (see its module doc), not an oversight.

Not in scope: any other adaptive method (Sauvola, Niblack — Bradley/Roth's
mean-based method is the simplest correct fix for the problem this
package exists to solve, and this workspace's convention throughout `VIS`
has been "build the standard textbook technique once, not every variant");
morphological cleanup of the thresholded result (that's `IMG08`, a
separate, still-unbuilt package).

## 3. Algorithm: Bradley/Roth integral-image adaptive mean threshold

Bradley & Roth, 2007 — the standard technique for exactly this problem.
A pixel is foreground (dark) when its luminance falls more than a
fraction `t` below the mean luminance of its local neighbourhood,
computed in O(1) per pixel via a summed-area table (integral image) so
the whole operation stays O(width × height), not
O(width × height × window²):

```text
1. Luminance buffer: same BT.601 formula threshold_luminance already
   uses (Y = 0.299R + 0.587G + 0.114B), one value per pixel.
2. Integral image: running 2D sum, (width+1) x (height+1), the standard
   summed-area table construction (row 0 / column 0 are a zero border).
3. For each pixel (x, y): local mean = sum over an approximately
   window x window neighbourhood centred at (x, y), read via 4 integral-
   image lookups (O(1)), divided by that neighbourhood's actual pixel
   count. The neighbourhood is clipped at the image edges -- a smaller
   effective window there, not a panic and not a wrap-around. The 4
   lookups combine as `(bottom_right + top_left) - (top_right +
   bottom_left)` -- add the two positive terms first, subtract once.
   The mathematically equivalent left-to-right `br - tr - bl + tl` is a
   real bug, not a style preference: `tr` and `bl` individually cover a
   larger row/column range than the partial result at that point in the
   evaluation restricts to, so an intermediate step can go negative in
   unsigned arithmetic and panic even though the fully-combined value is
   always >= 0 (caught by this package's own test suite the first time
   it was written the naive way).
4. foreground (dark) iff pixel_luminance < local_mean * (1.0 - t)
```

## 4. API

```rust
pub fn adaptive_threshold_mean(src: &PixelContainer, window: u32, t: f64) -> PixelContainer
```

- `window`: the approximate neighbourhood side length in pixels. Clamped
  to at least 1 (never a divide-by-zero) -- `window = 1` degenerates to
  every pixel comparing against its own value, which is never less than
  itself times `(1.0 - t)` for `t >= 0`, so it deterministically produces
  an all-background result. That's the documented, correct answer for a
  meaningless window size, not a special case in the code.
- `t`: the fraction below the local mean a pixel's luminance must fall to
  count as foreground. Clamped to `[0.0, 1.0]`; `NaN` is treated as `0.0`
  (the more permissive bound, since `f64::clamp` passes `NaN` through
  unchanged rather than clamping it -- confirmed against `f64::clamp`'s
  actual documented behaviour, not assumed). Never a NaN comparison
  reaches the per-pixel decision.
- Returns a `PixelContainer`: RGB set to `0` (foreground/dark) or `255`
  (background/light), alpha unchanged -- the same output convention
  `threshold`/`threshold_luminance` already use, so it composes with the
  rest of `image-point-ops` and a caller can `pixel_at` it directly.
- A `0`-width or `0`-height `src` returns an equally-empty result, never
  a panic.

### 4.1 Why `PixelContainer` in and out, not a bool bitmap

Consistent with every other function in this crate (`threshold`,
`threshold_luminance`, and the rest all take and return
`PixelContainer`) — a caller that wants a bitmap for `VIS02`/`VIS04`
converts at its own call site in one pass, exactly the boundary those two
packages' own specs already establish. Making this one function return a
different type than its siblings would make it harder to compose with
the rest of `image-point-ops`, not easier to use.

## 5. Test strategy

- **A single global threshold provably can't solve this, adaptive does**:
  a synthetic half-light/half-dark image (one solid luminance on the left
  half, a different one on the right, each internally uniform but with a
  small foreground mark in each half) where no single global cutoff
  correctly classifies both halves' marks — `adaptive_threshold_mean`
  gets both right.
- **Regression against the simple case**: a uniformly-lit image's
  adaptive result matches what a correctly-chosen global threshold would
  produce.
- **Edge pixels** (window clipped by the image boundary) don't panic and
  produce a sane classification, checked at all four corners and edges of
  a small test image.
- **Degenerate input**: `0`x`0`, `0`-width, `0`-height, `window = 0`,
  `t` = `NaN` / negative / `> 1.0` — all proven not to panic.

## 6. Acceptance gates

1. §5's full test matrix is green.
2. No panic on any input shape or parameter value.
3. `cargo clippy -p image-point-ops --all-targets -- -D warnings` clean.
