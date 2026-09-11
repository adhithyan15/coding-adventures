# `vis-connected-components` — VIS02

Two-pass union-find connected-component labeling over a binary image:
which lit ("foreground") pixels touch which, so a scattered set of
foreground pixels becomes a small list of blobs, each with a pixel
count, a bounding box, and a centroid.

Part of `code/specs/VIS00-vision-roadmap.md`'s L2 "vision primitives"
layer — see `VIS02-connected-components.md` for the full spec.

## Where this fits

```text
binarized image (e.g. image-point-ops::threshold_luminance output,
converted to a bool bitmap)
        │
        ▼
  vis_connected_components::label_components(bitmap, connectivity)
        │
        ▼
  Labeling { labels, components }
        │
        ▼
  VIS04 (pattern scanning, not yet built) clusters/verifies candidate
  finder-pattern blobs by their Component stats, hands real centers
  to VIS03 (geometric estimation)
```

## Usage

```rust
use vis_connected_components::{label_components, Connectivity};

let bitmap = vec![
    vec![true,  true,  false],
    vec![false, false, false],
    vec![false, true,  true],
];
let labeling = label_components(&bitmap, Connectivity::Four);
assert_eq!(labeling.components.len(), 2); // two separate blobs

for c in &labeling.components {
    println!(
        "blob {}: {} px, bbox ({},{})..=({},{}), centroid ({:.1},{:.1})",
        c.label, c.pixel_count, c.min_row, c.min_col, c.max_row, c.max_col,
        c.centroid_row, c.centroid_col,
    );
}
```

## Connectivity

```rust
pub enum Connectivity {
    Four,  // up/down/left/right only
    Eight, // Four, plus the four diagonals
}
```

A caller-supplied choice, not hardcoded — a diagonal-only touch between
two foreground pixels is one blob under `Eight`, two under `Four`. Which
is correct depends on what the caller is looking for (see the spec's
§3.1).

## Bitmap shape

`bitmap: &[Vec<bool>]` — matches the bitmap convention this workspace
already uses for exactly this kind of grid (`barcode_2d::ModuleGrid::
modules`). Not coupled to `pixel-container::PixelContainer` (RGBA8) —
connected-component labeling has no notion of color channels, so a
caller with a `PixelContainer` converts to a bool bitmap once at the
call site (e.g. after thresholding).

Never panics on any bitmap shape or content: empty, ragged (rows of
differing length), all-background, and all-foreground are all valid
input, each producing a well-defined result.

## Out of scope

- Which `Connectivity` a specific downstream use case (QR finder-
  pattern detection, etc.) should actually pick — this crate answers
  "what connects to what," not "is this the shape I'm looking for."
- Any shape/size filtering ("is this blob roughly square," "is it the
  right size") — the caller inspects the returned `Component`s and
  decides.
