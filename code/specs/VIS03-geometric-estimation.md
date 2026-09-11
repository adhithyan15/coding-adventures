# VIS03 — Geometric Estimation

## Status

Third package in `VIS00-vision-roadmap.md`'s L2 build order. New crate
(`vis-geometric-estimation`), depending on `matrix` (`VIS01`'s
`solve`). Given point correspondences — where a handful of known
landmarks actually sit in a photographed image, and where they *should*
sit in a straightened one — this crate solves for the transform
between them.

## 1. Why this exists

`image-geometric-transforms::perspective_warp` already *applies* a
given `[[f32; 3]; 3]` transform (confirmed exact signature: inverse-
warp sampling, output-plane to input-plane). Nothing in this workspace
*produces* that matrix from real point measurements — `VIS00` verified
this directly. That's the gap this crate closes: given a small number
of point pairs, solve for the matrix `perspective_warp` needs.

Concretely, once `VIS02` (connected components, merged) and `VIS04`
(pattern scanning, not yet built) locate a QR code's three finder-
pattern centers in a photograph, `VIS03` is what turns "here's where
those three points actually are, and here's where they should be in a
clean square" into the one matrix that straightens the whole image.

## 2. Scope: exact solve from a fixed point count, not least-squares

Two transform families, each solved *exactly* from the minimum number
of correspondences that determine it — not fit from an arbitrary,
possibly-overdetermined set via least squares (which would need SVD or
the normal-equations method, neither in scope: `VIS01` deliberately
built a small dense *exact* solver, not a least-squares one).

- **Affine** (`estimate_affine`) — rotation, scale, shear, translation;
  6 degrees of freedom, determined exactly by **3** point
  correspondences.
- **Homography** (`estimate_homography`) — a full projective transform,
  additionally correcting perspective/keystoning (the real camera-
  photo case, not just a screenshot); 8 degrees of freedom (the 9th
  entry of a 3×3 matrix is pure scale and is fixed by a normalization,
  §3.2), determined exactly by **4** point correspondences, via the
  standard DLT (direct linear transform) construction.

**Explicitly deferred, matching `VIS00`'s own framing** ("RANSAC-style
outlier rejection... is in scope for this package once the plain
4-point solve is proven, not a separate package"): fitting from *more*
than the minimum point count, and any outlier-robust estimation
(RANSAC) built on top of repeatedly calling the exact solve on
candidate subsets. Both are real, legitimate follow-up work once real
`VIS04` candidate points exist to test against — not assumed correct
in advance of that evidence.

## 3. The math

### 3.1 Affine from 3 points

An affine map is `(x, y) ↦ (a·x + b·y + c, d·x + e·y + f)`. Given three
correspondences `(x_i, y_i) ↦ (x'_i, y'_i)` for `i = 1, 2, 3`, `a, b, c`
and `d, e, f` are each determined by the *same* 3×3 system with two
different right-hand sides:

```text
M = [[x1, y1, 1], [x2, y2, 1], [x3, y3, 1]]

M · [a, b, c]ᵀ = [x'1, x'2, x'3]ᵀ   (via matrix::solve)
M · [d, e, f]ᵀ = [y'1, y'2, y'3]ᵀ   (via matrix::solve, same M)
```

`M` is singular exactly when the three source points are collinear —
`matrix::solve`'s existing singularity detection (`VIS01` §4.2) is
reused unchanged; this crate does not re-derive it.

Result, as the 3×3 homogeneous matrix `perspective_warp` consumes:

```text
[a b c]
[d e f]
[0 0 1]
```

### 3.2 Homography from 4 points (DLT)

A homography maps `(x, y) ↦ ((h11·x + h12·y + h13) / (h31·x + h32·y +
h33), (h21·x + h22·y + h23) / (h31·x + h32·y + h33))`. Clearing the
denominator turns each correspondence into two *linear* equations in
the 9 entries of `H`; `H` is only defined up to scale (scaling every
entry by the same nonzero constant produces the identical transform),
so fixing `h33 = 1` removes that redundancy and leaves exactly 8
unknowns — 4 correspondences, 2 equations each, exactly determine
them:

```text
For each correspondence i, two rows of an 8×8 system in
[h11, h12, h13, h21, h22, h23, h31, h32]:

  x_i·h11 + y_i·h12 + h13                     - x_i·x'_i·h31 - y_i·x'_i·h32 = x'_i
                          x_i·h21 + y_i·h22 + h23 - x_i·y'_i·h31 - y_i·y'_i·h32 = y'_i
```

Solved via one `matrix::solve` call on the resulting 8×8 `A`/`b` (the
exact system size `VIS01`'s own spec named as its motivating case).
Singular `A` (degenerate point configurations — collinear or
coincident points among the four) is detected by `matrix::solve`
unchanged, surfaced as `EstimationError::Degenerate`.

**The `h33 = 1` normalization is a deliberate, documented limitation,
not an oversight**: it assumes the true transform's `h33` entry is
nonzero, which holds for every realistic "straighten a photographed
planar surface" use case (QR finder corners, document corners) and
fails only for a vanishing-point-at-the-origin configuration that does
not arise there. Not re-derived as a general-purpose projective solver
for every possible point configuration.

### 3.3 Direction is the caller's choice, not assumed

`perspective_warp` consumes an **output-to-input** matrix: "given a
point in the destination (straightened) image, where do I read from
the source (photographed) image." This crate does not bake in which
direction `from`/`to` represent — `estimate_affine`/
`estimate_homography` simply solve "the transform mapping `from[i]` to
`to[i]`," for whichever two point sets the caller supplies. To produce
a `perspective_warp`-ready matrix directly, a caller passes
`from` = points in the clean/straightened space and `to` = the same
points as actually measured in the photograph — documented explicitly
in the API doc comment (§4) rather than assumed silently, since
getting the direction backwards is a real, easy mistake with no type-
level way to catch it.

## 4. API

```rust
pub enum EstimationError {
    /// `estimate_affine` needs exactly 3 correspondences,
    /// `estimate_homography` needs exactly 4 -- this reports both what
    /// was needed and what was actually supplied.
    WrongPointCount { needed: usize, got: usize },
    /// The underlying linear system was singular -- degenerate input
    /// (collinear or coincident points) with no unique solution, not a
    /// numerical near-miss silently producing a wrong-but-plausible
    /// matrix. Carries `matrix::solve`'s own error message unchanged.
    Degenerate(String),
}

/// Estimate the 3x3 homogeneous affine transform mapping `from[i]` to
/// `to[i]`, for exactly 3 correspondences (affine's 6 degrees of
/// freedom are exactly determined by 3 points -- not a least-squares
/// fit from more).
///
/// The returned matrix maps `from`-space points to `to`-space points.
/// To produce a matrix directly usable by
/// `image_geometric_transforms::perspective_warp` (which reads
/// output-to-input), pass `from` = clean/straightened-space points and
/// `to` = the same points as measured in the photographed image.
pub fn estimate_affine(
    from: &[(f64, f64)],
    to: &[(f64, f64)],
) -> Result<[[f64; 3]; 3], EstimationError>;

/// Estimate the 3x3 homography mapping `from[i]` to `to[i]`, for
/// exactly 4 correspondences, via the standard DLT construction
/// normalized so `h33 = 1` (§3.2). Same from/to direction convention
/// as `estimate_affine`.
pub fn estimate_homography(
    from: &[(f64, f64)],
    to: &[(f64, f64)],
) -> Result<[[f64; 3]; 3], EstimationError>;
```

Both functions validate `from.len() == to.len() == 3` (or `4`) before
building any linear system — a length mismatch or wrong count is
`WrongPointCount`, checked first, not a panic from indexing past a
short slice.

## 5. What this does not decide

- Fitting from more than the minimum point count (least-squares,
  needing SVD or normal equations) — real follow-up work once `VIS04`
  produces real candidate points to test against, not assumed correct
  here without that evidence.
- RANSAC or any other outlier-robust estimation built on repeated exact
  solves — same reasoning, explicitly named as `VIS00`'s own next step
  after this package, not part of it.
- Which direction (`from`/`to`) a specific caller should use — §3.3
  documents the convention, does not enforce it at the type level.

## 6. Test strategy

- **Known-transform round trips**: construct a known affine matrix
  (rotation + scale + translation) and a known homography (including a
  real perspective term, `h31`/`h32` nonzero) by hand, apply each to a
  set of source points to generate correspondences, then recover the
  matrix via `estimate_affine`/`estimate_homography` and assert it
  matches the original (within floating-point tolerance) — proving
  actual numerical correctness, not just "returns `Ok`."
- **Identity and pure-translation** special cases, as the simplest
  possible known-answer checks.
- **Degenerate input**: three collinear points (`estimate_affine`),
  four collinear or four coincident points (`estimate_homography`) —
  both must return `Degenerate`, not a wrong-but-plausible matrix.
- **Wrong point count**: 2 or 4 points to `estimate_affine`, 3 or 5
  points to `estimate_homography`, and a `from`/`to` length mismatch —
  all `WrongPointCount`, checked before any linear system is built
  (provable the same way `matrix`'s own tests prove bound-checking
  order: a case that would also fail differently downstream if the
  length check were skipped).
- **Affine is a special case of homography**: apply a known *affine*
  transform (so the true `h31 = h32 = 0`) and confirm
  `estimate_homography` on 4 of its correspondences recovers a matrix
  equivalent to the direct affine estimate on 3 of them — cross-
  validating the two solvers against each other, not only against
  hand-constructed matrices.
- **No panics** on any input shape (empty slices, length mismatches,
  duplicate points) — every case above is provably `Err`, never a
  panic, matching the discipline `matrix`'s own `solve`/`invert`/
  `determinant` already established.

## 7. Acceptance gates

1. §6's full test matrix is green.
2. Every recovered matrix, in the known-transform round-trip tests,
   matches the original transform's action on a held-out point (not
   one of the points used to estimate it) within a small floating-
   point tolerance — proving the estimate is actually the right
   transform, not merely consistent with the points it was fit from.
3. No panic on any input shape or point count.
4. `BUILD`, `README.md`, `CHANGELOG.md`, `required_capabilities.json`
   (empty — pure computation) present, matching this workspace's
   per-package convention.
