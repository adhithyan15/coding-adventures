# `vis-pattern-scanning` — VIS04

Scan a binary image for a run-length ratio signature — QR's
`1:1:3:1:1` finder pattern, or any other 2D barcode format's own
ratio — and get back confirmed 2D candidate locations. The last of
the four `VIS00-vision-roadmap.md` L2 "vision primitives" packages —
see `VIS04-pattern-scanning.md` for the full spec.

## Where this fits

```text
binarized image (bool bitmap; true = foreground/dark)
        │
        ▼
  vis_pattern_scanning::find_pattern_candidates
        │
        ▼
  candidate finder-pattern centers (several per real pattern,
  not yet clustered)
        │
        ▼
  vis_connected_components::label_components  (cluster nearby
  candidates into one refined center per real pattern -- L3 work)
        │
        ▼
  vis_geometric_estimation::estimate_homography  (VIS03, already
  merged -- turns refined centers into a straightening transform)
```

## Usage

```rust
use vis_pattern_scanning::{find_pattern_candidates, scan_line};

// Scan a single row or column for a ratio.
let line = [true, false, true, true, true, false, true];
let matches = scan_line(&line, &[1, 1, 3, 1, 1], 0.2); // QR's ratio, +/-20%

// Scan a whole bitmap, cross-checking horizontal matches vertically.
let bitmap: Vec<Vec<bool>> = /* ... */ vec![];
let candidates = find_pattern_candidates(&bitmap, &[1, 1, 3, 1, 1], 0.2);
for c in &candidates {
    println!("candidate at ({}, {}), module size {}", c.row, c.col, c.module_size);
}
```

## Two functions, one algorithm applied twice

- **`scan_line`** — the primitive: decompose one line into alternating
  runs, slide a `ratio.len()`-wide window across them, and report
  every window within `tolerance` of the ratio (starting on a
  foreground/dark run, since runs always alternate color).
- **`find_pattern_candidates`** — scans every row for `scan_line`
  matches, then cross-checks each one with a vertical `scan_line`
  through the same point, confirming it only when the two agree
  closely (within one estimated module size). This is what filters
  out the many rows that coincidentally match the ratio with no real
  2D structure behind them.

## Clustering is not this crate's job

Several adjacent scanlines crossing the same real finder pattern
typically each produce their own confirmed candidate — this crate
does not merge them into one refined center. That's `vis-connected-
components`' job (stamp candidates onto a small bitmap, run
`label_components`, take each blob's centroid) — see the spec's §2
for why that's a separate concern from scanning.

## Never panics

Empty bitmaps, empty ratios, ragged bitmaps (rows of differing
length), and non-finite or negative `tolerance` all produce an empty
(or, at `tolerance = 0.0`, exact-only) result — never a panic. See the
spec's §6 for the full degenerate-input test matrix.

## Bounded, not just correct

Three constants keep every caller-supplied parameter from driving
this crate's cost past what the input's own size would suggest:
`MAX_TOLERANCE` (an unbounded `tolerance` would make every window
"match," regardless of shape), `MAX_MATCHES_PER_LINE` (a periodic,
texture-like line can satisfy the ratio check at a large fraction of
its windows using an entirely ordinary tolerance — real photographic
content, not only an adversarial one), and `MAX_RATIO_LEN` (each
window costs O(`ratio.len()`) to check). See the spec's
§4.2/§4.3/§4.4.

## Out of scope

- Clustering/deduplicating nearby candidates.
- Choosing what ratio to scan for — every format's ratio is a caller
  value, not a constant here.
- Diagonal cross-checks beyond horizontal + vertical.
