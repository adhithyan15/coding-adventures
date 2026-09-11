# VIS02 — Connected-Component Labeling

## Status

Second package in `VIS00-vision-roadmap.md`'s L2 build order. New crate
(`vis-connected-components`), since — unlike `VIS01`, which closed a
gap in an existing dense-matrix type — no existing crate in this
workspace is a natural home for a binary-image blob-labeling algorithm;
it doesn't belong in `image-point-ops` (pixel-independent point
operations only) or `barcode-2d` (a `ModuleGrid`-specific, not a
general-image, concern).

## 1. Why this exists

`VIS00` scoped this precisely: "which lit pixels touch which. Turns 'a
blob of dark pixels' into 'one object with a bounding box, a centroid,
and a pixel count' — the shape a finder-pattern candidate needs to be
in before it can be measured or compared against its neighbors."

Concretely, `VIS04` (pattern scanning, not yet built) will scan a
binarized image for the 1:1:3:1:1-ratio run-length signature a QR
finder pattern produces, and get back a set of *candidate pixels*
scattered across the image — some real finder-pattern hits, many false
positives from unrelated dark regions that happen to match the ratio
on one scanline. Turning "a pixel matched" into "here is the actual
blob that pixel belongs to, and its centroid" is exactly what lets a
later stage cluster candidates, reject blobs the wrong size or shape,
and hand real finder-pattern centers to `VIS03` (geometric estimation).
No code anywhere in this workspace does this today — confirmed
directly in `VIS00`'s own research (the closest hit, `graph::
connected_components`, operates on a generic string-keyed `Graph`, not
a pixel grid, and using it here would mean building one graph node per
pixel — impractical at image sizes).

## 2. Scope

A general **binary-image** blob labeler: input is a bitmap (true =
foreground/dark, false = background/light), output is every connected
blob's label, pixel count, bounding box, and centroid. Not specific to
QR or any barcode format — `VIS04` (and anything else that ever needs
"which pixels form one connected shape" over a bitmap) is the
consumer, not a dependency this crate takes on.

Two-dimensional bitmaps only (this is image work); not a generalization
to arbitrary graphs (that already exists, as `graph::
connected_components`, for the cases it actually fits) and not 3D/
volumetric connectivity.

## 3. Algorithm

Classic **two-pass union-find** labeling (also called Hoshen-Kopelman
in some literature) — the standard approach for this problem, O(n·α(n))
where α is the (effectively constant) inverse Ackermann function from
union-find's path compression, not a novel design:

```text
Pass 1 (provisional labeling), scanning row-major:
  for each foreground pixel (r, c):
    look at its already-visited neighbors (per the chosen
    Connectivity, §3.1) that are also foreground
    if none are foreground:
      assign a new provisional label
    if some are foreground:
      assign the smallest of their labels
      record a union between every pair of distinct labels seen
      among those neighbors (this pixel is proof they're the same blob)

Pass 2 (resolution + aggregation), scanning row-major again:
  for each foreground pixel:
    resolve its provisional label to its union-find root
    look up (or assign, on first sight) that root's final 1-based
    output label
    accumulate that component's pixel count, bounding box, and
    centroid running sum
  finalize centroids: sum / count, once per component
```

Two passes (not one) because a pixel's true component isn't always
knowable from already-visited neighbors alone — two provisional labels
can turn out to be the same blob only once a later pixel connects them
(the classic "U-shaped blob" case), which is exactly what the
union-find structure built during pass 1 resolves before pass 2 ever
runs.

### 3.1 Connectivity is a parameter, not a hardcoded choice

```rust
pub enum Connectivity {
    /// Up, down, left, right only.
    Four,
    /// Four-connectivity plus the four diagonals.
    Eight,
}
```

QR finder patterns are axis-aligned squares, which 4-connectivity
already labels correctly — but a thresholded, slightly-rotated, or
anti-aliased real photo can leave a one-pixel diagonal gap at a corner
that 4-connectivity would see as two separate blobs and 8-connectivity
correctly merges into one. Which is right depends on what `VIS04`
observes in practice once it exists, not something `VIS02` should
decide unilaterally — so it's a caller-supplied parameter, not a fixed
internal choice.

## 4. API

```rust
pub enum Connectivity { Four, Eight }

pub struct Component {
    /// 1-based; matches the value this component's pixels carry in
    /// `Labeling::labels`. Never 0 — 0 is reserved for background.
    pub label: usize,
    pub pixel_count: usize,
    /// Inclusive bounding box, in (row, col).
    pub min_row: usize,
    pub max_row: usize,
    pub min_col: usize,
    pub max_col: usize,
    pub centroid_row: f64,
    pub centroid_col: f64,
}

pub struct Labeling {
    /// Same shape as the input bitmap. `0` = background; otherwise the
    /// 1-based label of the component that pixel belongs to.
    pub labels: Vec<Vec<usize>>,
    /// One entry per component, ordered by `label` ascending (so
    /// `components[i].label == i + 1`).
    pub components: Vec<Component>,
}

/// Label every connected foreground blob in `bitmap` (`true` =
/// foreground). Never panics — including on an empty bitmap, a
/// ragged one (rows of differing length), or one with zero
/// foreground pixels at all (returns an all-zero label grid and no
/// components, not an error: "no blobs found" is a valid, ordinary
/// answer, not a failure case).
pub fn label_components(bitmap: &[Vec<bool>], connectivity: Connectivity) -> Labeling;
```

### 4.1 Why a bitmap (`Vec<Vec<bool>>`), not `PixelContainer`

Connected-component labeling is a pure binary-image algorithm with no
notion of color channels — coupling it to `pixel-container::
PixelContainer` (RGBA8) would mean every caller pays for a channel
model this algorithm never looks at, and would make `VIS02` depend on
`pixel-container` for no algorithmic reason. `Vec<Vec<bool>>` also
matches the bitmap convention this workspace already uses in exactly
this shape for exactly this kind of grid (`barcode_2d::ModuleGrid::
modules`) — a caller that already has a `PixelContainer` (e.g. after
`image-point-ops::threshold_luminance`) converts to a bool bitmap in
one pass at the call site; that conversion is a caller concern, not
this crate's.

### 4.2 Ragged and degenerate input

`bitmap` is not statically required to be rectangular (`Vec<Vec<bool>>`
permits rows of different lengths) — matching this workspace's now-
established discipline (`MA04-qr-decoder.md` §8 gate 6, `matrix`'s own
`solve`/`invert`/`determinant`) that a publicly-constructible grid type
is not guaranteed well-formed by its type alone, and a general-purpose
primitive must stay bounds-safe regardless. `label_components` never
indexes past a row's own actual length; a ragged bitmap is handled
(each row contributes only its own real columns to neighbor lookups),
never causes an out-of-bounds panic.

## 5. What this does not decide

- Which `Connectivity` `VIS04` should actually use for QR finder-
  pattern candidates — that's `VIS04`'s own call, informed by real
  test images once it exists, not assumed here.
- Any notion of "is this blob shaped like a finder pattern" (aspect
  ratio, fill fraction, size thresholds) — this crate answers "what
  connects to what," not "is this the shape I'm looking for"; that
  judgment belongs to whatever calls `label_components` and inspects
  the returned `Component`s.

## 6. Test strategy

- **Known shapes**: a single isolated pixel (1 component, count 1); a
  solid rectangle (1 component, exact expected bounding box/centroid);
  two separated rectangles (2 components, correct per-component stats,
  not merged); a U-shape / C-shape (one component — the case that
  specifically requires two passes, since the two "arms" only turn out
  to be connected once the pixel closing the loop is reached).
- **Connectivity matters**: a diagonal-only touch (two foreground
  pixels touching only at a corner) — `Four` reports 2 components,
  `Eight` reports 1, over the *same* input bitmap. This is the one
  test that would still pass if connectivity were silently ignored (if
  the implementation always behaved as `Eight`, `Four`'s case would
  wrongly also report 1) — so it must assert the *count differs*
  between the two calls, not merely that each call succeeds.
- **Empty and degenerate**: 0-row bitmap; an all-background bitmap
  (zero components, all-zero label grid); a ragged bitmap (rows of
  different lengths) proven not to panic and to produce a sane result
  (every real pixel gets a label or is background; nothing reads past
  a short row).
- **Label grid / component list agreement**: for every test case
  above, independently recompute each component's pixel count, bounding
  box, and centroid *from the returned label grid itself* (not just
  trust the `Component` struct) and assert they match — proving
  `labels` and `components` are actually consistent with each other,
  not two independently-computed answers that happen to both look
  plausible.
- **A realistic-scale case**: a bitmap with several irregularly-shaped
  blobs of varying size at a size comparable to what a downsampled
  finder-pattern search window might actually produce (tens to a few
  hundred pixels per side), checked for the right component count and
  spot-checked stats — proving the algorithm behaves correctly past
  the small hand-constructed cases above, not only at toy scale.

## 7. Acceptance gates

1. §6's full test matrix is green.
2. No panic on any bitmap shape or content — empty, ragged, all-
   background, all-foreground.
3. `labels` and `components` are provably consistent (§6, the
   independent-recomputation test), not merely both present.
4. `BUILD`, `README.md`, `CHANGELOG.md`, `required_capabilities.json`
   (empty — pure computation, no I/O) present, matching this
   workspace's per-package convention.
