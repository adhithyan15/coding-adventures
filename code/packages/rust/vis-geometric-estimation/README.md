# `vis-geometric-estimation` — VIS03

Given a handful of point correspondences — where a landmark actually
sits, and where it should sit — solve for the geometric transform
between them. Produces the 3x3 matrix `image-geometric-transforms::
perspective_warp` already *applies*; nothing else in this workspace
*produces* one from real point measurements.

Part of `code/specs/VIS00-vision-roadmap.md`'s L2 "vision primitives"
layer — see `VIS03-geometric-estimation.md` for the full spec.

## Where this fits

```text
3-4 point correspondences (e.g. finder-pattern centers from VIS02,
once VIS04's pattern scanning locates them in a photo)
        │
        ▼
  vis_geometric_estimation::estimate_affine / estimate_homography
        │
        ▼
  3x3 homogeneous transform matrix
        │
        ▼
  image_geometric_transforms::perspective_warp (already exists,
  unchanged) -- samples the photographed image into a straightened
  ModuleGrid for qr_decoder::decode (already exists, unchanged)
```

## Usage

```rust
use vis_geometric_estimation::{estimate_affine, estimate_homography};

// Affine: exactly 3 correspondences.
let from = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)];
let to = [(10.0, 5.0), (11.0, 5.0), (10.0, 6.0)];
let m = estimate_affine(&from, &to)?;

// Homography: exactly 4 correspondences, also corrects perspective.
let from4 = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];
let to4 = [(2.0, 1.0), (12.0, 2.0), (3.0, 11.0), (13.0, 12.0)];
let h = estimate_homography(&from4, &to4)?;
```

## Two transform families, both solved exactly

- **`estimate_affine`** — rotation, scale, shear, translation; 6
  degrees of freedom, exactly determined by 3 points. Two independent
  3x3 `matrix::solve` calls sharing the same matrix.
- **`estimate_homography`** — a full projective transform (also
  corrects perspective/keystoning); 8 degrees of freedom, exactly
  determined by 4 points, via the standard DLT (direct linear
  transform) construction -- one 8x8 `matrix::solve` call.

Both solve *exactly* from the minimum point count, not as a
least-squares fit from more (that needs SVD or normal equations,
neither in `matrix`'s scope). Fitting from more points, and any
outlier-robust estimation (RANSAC) built on repeated exact solves, are
explicit follow-up work -- see the spec's §2/§5.

## Direction

Both functions solve "the transform mapping `from[i]` to `to[i]`."
`perspective_warp` reads an **output-to-input** matrix. To get a
`perspective_warp`-ready matrix directly, pass `from` = points in the
clean/destination space and `to` = the same points as actually
measured in the source image. Not enforced at the type level -- see
the spec's §3.3.

## Errors

```rust
pub enum EstimationError {
    WrongPointCount { needed: usize, got: usize },
    Degenerate(String), // collinear or coincident points; matrix::solve's own message
}
```

Never panics on any input shape (empty slices, length mismatches,
duplicate points) -- every degenerate case above is a documented `Err`.

## Out of scope

- Fitting from more than the minimum point count (least-squares).
- RANSAC or other outlier-robust estimation.
- Enforcing the `from`/`to` direction convention at the type level.
