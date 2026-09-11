// # qr-locate
//
// The L3 application `code/specs/VIS00-vision-roadmap.md` set out to
// reach: given a photographed image, find and decode the QR code in it.
// Assembles `IMG09` (adaptive threshold), `VIS04` (pattern scanning),
// `VIS02` (connected components), `VIS03` (geometric estimation), and
// the already-shipped `perspective_warp` + `qr_decoder::decode` --
// nothing below this layer is new. See `code/specs/qr-locate.md` for
// the full pipeline design.
//
// One public entry point, no tuning parameters -- matches
// `qr_decoder::decode`'s own minimal surface. Every internal threshold
// is a documented constant.

use pixel_container::PixelContainer;
use vis_pattern_scanning::PatternCandidate;
use image_geometric_transforms::{Interpolation, OutOfBounds};

/// The QR finder pattern's run-length ratio (dark:light:dark:light:dark).
const SCAN_RATIO: [u32; 5] = [1, 1, 3, 1, 1];
/// Relative tolerance passed to `find_pattern_candidates`.
const SCAN_TOLERANCE: f64 = 0.4;
/// `adaptive_threshold_mean`'s window is `image_size / ADAPTIVE_WINDOW_DIVISOR`
/// (Bradley & Roth's own recommendation), not a fixed pixel count. A QR
/// finder pattern's solid 3-module center block is a real uniform-dark
/// region several modules wide; a window smaller than it gives that
/// region's own interior a local mean of ~0, so nothing looks "darker
/// than the mean" and the interior misclassifies as background --
/// caught empirically (an unrotated synthetic QR found zero finder-
/// pattern candidates with a fixed 25px window) before this crate's
/// first push. Scaling with the image's own size keeps the window
/// bigger than any real QR's finder-pattern block regardless of how
/// large the photographed code is in frame.
const ADAPTIVE_WINDOW_DIVISOR: u32 = 8;
/// Floor on the computed window, for small images where `size / 8`
/// would otherwise be too small to give any local context.
const ADAPTIVE_WINDOW_MIN: u32 = 15;
/// Fraction below the local mean a pixel must fall to count as dark.
const ADAPTIVE_T: f64 = 0.15;
/// Source pixels rendered per module in the straightened output.
const MODULE_PIXELS: u32 = 4;

/// Cap on raw `PatternCandidate`s kept before clustering (§4.1 of the
/// spec). Truncated in scan order -- the same defensive-not-optimal
/// truncation `vis_pattern_scanning::MAX_MATCHES_PER_LINE` already
/// established as this workspace's precedent -- so a pathological
/// bitmap can't make the O(candidates) stamp-and-cluster step below
/// scale with an unbounded `VIS04` output. This bounds `cluster_candidates`
/// specifically, not `find_pattern_candidates`'s own scan cost -- that's
/// already separately bounded by `vis_pattern_scanning::MAX_MATCHES_PER_LINE`
/// (per scanline) within VIS04 itself, reviewed and fixed there.
pub const MAX_RAW_CANDIDATES: usize = 4096;
/// Cap on refined (post-clustering) candidates kept before the O(n^3)
/// triangle search (§4.3). Truncated by descending group size (how
/// many raw candidates corroborate it) -- a real correctness-relevant
/// ranking, not just a DoS-safety truncation: the best-supported
/// clusters are also the most likely to be real finder patterns.
pub const MAX_REFINED_CANDIDATES: usize = 64;

/// Relative deviation from `1.0` a triangle's two leg lengths may
/// differ by and still be considered a candidate finder-pattern
/// triangle.
const TRIANGLE_LEG_RATIO_TOLERANCE: f64 = 0.35;
/// How far from `0.0` (a perfect right angle) `|cos(angle)|` at the
/// candidate corner may be.
const TRIANGLE_ANGLE_COS_TOLERANCE: f64 = 0.35;
/// Relative deviation from `sqrt(2)` the hypotenuse-to-leg ratio may
/// differ by.
const TRIANGLE_HYP_RATIO_TOLERANCE: f64 = 0.35;
/// Largest relative error `estimate_version` accepts between a legal
/// size's implied module size and the measured one. Generous relative
/// to the ~4% bias a rotated finder pattern's run-length measurement
/// can carry (§ `estimate_version`'s own doc) while still rejecting
/// genuinely implausible triangles.
const VERSION_MATCH_TOLERANCE: f64 = 0.3;

/// Why `locate_and_decode` couldn't produce decoded bytes.
#[derive(Debug, Clone, PartialEq)]
pub enum QrLocateError {
    /// No group of 3 refined candidates formed a plausible right-
    /// isoceles finder-pattern triangle (§4.3).
    NoFinderPatternTriangleFound,
    /// A triangle was found, but the modules-between-finders spacing
    /// it implies doesn't round to a legal QR version size (`21 + 4k`).
    InvalidEstimatedVersion { measured_modules: f64 },
    /// A triangle and a legal version were found, but neither of the
    /// two possible corner-assignment chiralities (§4.5) decoded --
    /// one entry per attempt.
    Decode(Vec<qr_decoder::QrDecodeError>),
}

/// Locate and decode a QR code somewhere in `image`.
///
/// # Examples
///
/// See the crate's integration tests for a full synthetic
/// photograph-to-decoded-bytes round trip -- this doctest only checks
/// the "no finder pattern present" failure path, since a passing
/// example needs a real rendered QR code (a dev-dependency, not
/// available to a doctest).
///
/// ```
/// use pixel_container::PixelContainer;
/// use qr_locate::{locate_and_decode, QrLocateError};
///
/// let blank = PixelContainer::new(100, 100);
/// assert_eq!(
///     locate_and_decode(&blank),
///     Err(QrLocateError::NoFinderPatternTriangleFound)
/// );
/// ```
pub fn locate_and_decode(image: &PixelContainer) -> Result<Vec<u8>, QrLocateError> {
    let bitmap = binarize_bitmap(image);

    let mut raw = vis_pattern_scanning::find_pattern_candidates(&bitmap, &SCAN_RATIO, SCAN_TOLERANCE);
    raw.truncate(MAX_RAW_CANDIDATES);

    let mut refined = cluster_candidates(bitmap.first().map_or(0, |r| r.len()), bitmap.len(), &raw);
    refined.truncate(MAX_REFINED_CANDIDATES);

    let triangle =
        find_best_triangle(&refined).ok_or(QrLocateError::NoFinderPatternTriangleFound)?;
    let n = estimate_version(triangle.leg_px, triangle.module_size).ok_or(
        QrLocateError::InvalidEstimatedVersion {
            measured_modules: triangle.leg_px / triangle.module_size,
        },
    )?;

    let corner = &refined[triangle.corner];
    let p_a = &refined[triangle.a];
    let p_b = &refined[triangle.b];
    let corner_pt = (corner.row, corner.col);
    let a_pt = (p_a.row, p_a.col);
    let b_pt = (p_b.row, p_b.col);

    let mut errors = Vec::new();
    for (top_right, bottom_left) in [(a_pt, b_pt), (b_pt, a_pt)] {
        match attempt_decode(image, corner_pt, top_right, bottom_left, n) {
            Ok(bytes) => return Ok(bytes),
            Err(e) => errors.push(e),
        }
    }
    Err(QrLocateError::Decode(errors))
}

/// Adaptive-threshold `image`, then convert to the `true` = dark
/// bitmap convention `vis_pattern_scanning`/`vis_connected_components`
/// both use.
fn binarize_bitmap(image: &PixelContainer) -> Vec<Vec<bool>> {
    let window = (image.width.min(image.height) / ADAPTIVE_WINDOW_DIVISOR).max(ADAPTIVE_WINDOW_MIN);
    let binarized = image_point_ops::adaptive_threshold_mean(image, window, ADAPTIVE_T);
    let width = binarized.width as usize;
    let height = binarized.height as usize;
    let mut bitmap = vec![vec![false; width]; height];
    for (y, row) in bitmap.iter_mut().enumerate() {
        for (x, cell) in row.iter_mut().enumerate() {
            let (r, _, _, _) = binarized.pixel_at(x as u32, y as u32);
            *cell = r == 0; // threshold_luminance's convention: 0 = dark/foreground.
        }
    }
    bitmap
}

/// One refined finder-pattern candidate: the mean `(row, col,
/// module_size)` of every raw `PatternCandidate` that clustered
/// together (§4.2), plus how many contributed (`support`).
struct RefinedCandidate {
    row: f64,
    col: f64,
    module_size: f64,
    support: usize,
}

/// Cluster raw candidates into refined centers via VIS02, per
/// `VIS04`'s own spec §2 recommendation: stamp a small square around
/// each raw candidate onto a fresh bitmap, `label_components` (Eight
/// connectivity) over it, group raw candidates by which component
/// their own stamped center landed in. A refined center averages the
/// *original measured candidates* in its group, not the stamped
/// pixels' own centroid -- sub-pixel precision the integer stamp grid
/// alone wouldn't preserve.
fn cluster_candidates(width: usize, height: usize, raw: &[PatternCandidate]) -> Vec<RefinedCandidate> {
    if raw.is_empty() || width == 0 || height == 0 {
        return Vec::new();
    }

    let mut stamp = vec![vec![false; width]; height];
    for c in raw {
        // module_size.max(1.0) handles NaN safely: f64::max returns the
        // non-NaN operand when one side is NaN, so this never produces a
        // NaN radius even from a pathological candidate.
        let radius = c.module_size.round().max(1.0) as i64;
        let center_row = c.row.round() as i64;
        let center_col = c.col.round() as i64;
        // saturating_add, not `+`: center_row/center_col/radius all come
        // from a f64 `.round() as i64` cast, which saturates toward
        // i64::MAX/MIN for a value far outside i64's range rather than
        // panicking -- but a plain `+` on two already-saturated-near-MAX
        // values would then overflow and panic (in a debug/overflow-
        // checked build) or silently wrap (in release). This function's
        // safety shouldn't depend on an implicit assumption that
        // `vis_pattern_scanning`'s candidates always stay near the
        // bitmap's own dimensions -- saturating_add keeps it true
        // regardless, since the clamp below only needs the sum to not
        // wrap past `height`/`width`'s own small range.
        let row0 = center_row.saturating_sub(radius).max(0) as usize;
        // If (center_row + radius) is negative (candidate entirely off-
        // image), row1 clamps to 0 while row0 (from the branch above) can
        // exceed it, making `row0..=row1` an empty range -- safe in Rust,
        // not a panic -- rather than needing an explicit extra clamp.
        let row1 = center_row.saturating_add(radius).clamp(0, height as i64 - 1) as usize;
        let col0 = center_col.saturating_sub(radius).max(0) as usize;
        let col1 = center_col.saturating_add(radius).clamp(0, width as i64 - 1) as usize;
        for r in row0..=row1 {
            if let Some(row) = stamp.get_mut(r) {
                for cell in row.iter_mut().take(col1 + 1).skip(col0) {
                    *cell = true;
                }
            }
        }
    }

    let labeling = vis_connected_components::label_components(
        &stamp,
        vis_connected_components::Connectivity::Eight,
    );

    let mut groups: std::collections::HashMap<usize, Vec<&PatternCandidate>> =
        std::collections::HashMap::new();
    for c in raw {
        if c.row < 0.0 || c.col < 0.0 {
            continue;
        }
        let r = c.row.round() as usize;
        let col = c.col.round() as usize;
        let Some(label) = labeling.labels.get(r).and_then(|row| row.get(col)) else {
            continue;
        };
        if *label == 0 {
            continue;
        }
        groups.entry(*label).or_default().push(c);
    }

    let mut refined: Vec<RefinedCandidate> = groups
        .into_values()
        .map(|members| {
            let n = members.len() as f64;
            RefinedCandidate {
                row: members.iter().map(|c| c.row).sum::<f64>() / n,
                col: members.iter().map(|c| c.col).sum::<f64>() / n,
                module_size: members.iter().map(|c| c.module_size).sum::<f64>() / n,
                support: members.len(),
            }
        })
        .collect();
    refined.sort_by_key(|c| std::cmp::Reverse(c.support));
    refined
}

/// A candidate right-isoceles finder-pattern triangle: `corner` is the
/// right-angle vertex, `a`/`b` the other two -- all indices into the
/// `refined` slice `find_best_triangle` was called with.
struct FoundTriangle {
    corner: usize,
    a: usize,
    b: usize,
    leg_px: f64,
    module_size: f64,
}

/// Try every 3-combination of `refined` and every choice of which
/// vertex is the right-angle corner (§4.3); return the best-scoring
/// one that clears all three geometric tolerances, or `None` if no
/// triple does. O(n^3) in `refined.len()`, bounded by the caller's
/// `MAX_REFINED_CANDIDATES` truncation.
fn find_best_triangle(refined: &[RefinedCandidate]) -> Option<FoundTriangle> {
    if refined.len() < 3 {
        return None;
    }
    let mut best: Option<(f64, FoundTriangle)> = None;
    for c in 0..refined.len() {
        for i in 0..refined.len() {
            if i == c {
                continue;
            }
            for j in (i + 1)..refined.len() {
                if j == c {
                    continue;
                }
                let corner = &refined[c];
                let p1 = &refined[i];
                let p2 = &refined[j];
                let u = (p1.row - corner.row, p1.col - corner.col);
                let v = (p2.row - corner.row, p2.col - corner.col);
                let len_u = (u.0 * u.0 + u.1 * u.1).sqrt();
                let len_v = (v.0 * v.0 + v.1 * v.1).sqrt();
                if len_u < 1e-6 || len_v < 1e-6 {
                    continue;
                }
                let leg_ratio_dev = len_u.max(len_v) / len_u.min(len_v) - 1.0;
                if leg_ratio_dev.abs() > TRIANGLE_LEG_RATIO_TOLERANCE {
                    continue;
                }
                let cos_angle = (u.0 * v.0 + u.1 * v.1) / (len_u * len_v);
                if cos_angle.abs() > TRIANGLE_ANGLE_COS_TOLERANCE {
                    continue;
                }
                let hyp = ((p1.row - p2.row).powi(2) + (p1.col - p2.col).powi(2)).sqrt();
                let leg_avg = (len_u + len_v) / 2.0;
                let hyp_ratio_dev = hyp / (leg_avg * std::f64::consts::SQRT_2) - 1.0;
                if hyp_ratio_dev.abs() > TRIANGLE_HYP_RATIO_TOLERANCE {
                    continue;
                }

                let score = leg_ratio_dev.abs() + cos_angle.abs() + hyp_ratio_dev.abs();
                let is_better = match &best {
                    None => true,
                    Some((best_score, _)) => score < *best_score,
                };
                if is_better {
                    let module_size = (corner.module_size + p1.module_size + p2.module_size) / 3.0;
                    best = Some((
                        score,
                        FoundTriangle {
                            corner: c,
                            a: i,
                            b: j,
                            leg_px: leg_avg,
                            module_size,
                        },
                    ));
                }
            }
        }
    }
    best.map(|(_, t)| t)
}

/// A finder-pattern center sits 3.5 modules from its own edge, so the
/// gap between the top-left and top-right (or top-left and bottom-
/// left) centers is `N - 7` modules for an `N`x`N`-module QR symbol.
/// Recover `N` from the measured leg length, and validate it's a
/// legal QR size (`21 + 4k`, `k` in `0..40`).
fn estimate_version(leg_px: f64, module_size: f64) -> Option<u32> {
    if !(module_size.is_finite() && module_size > 0.0) {
        return None;
    }
    if !(leg_px.is_finite() && leg_px > 0.0) {
        return None;
    }
    // Search every legal QR size directly (21, 25, ..., 177) rather
    // than rounding leg_px / module_size and checking the result is
    // legal. VIS04's module_size estimate is a run-length measurement
    // along a horizontal/vertical scanline; for a *rotated* finder
    // pattern that scanline crosses the pattern diagonally, not
    // perpendicular to its edges, so the measured run is a real,
    // rotation-dependent foreshortened/elongated value -- not a bug in
    // VIS04, an inherent property of measuring width along an axis
    // that isn't aligned with the shape. Caught empirically: a QR
    // photographed at 17 degrees gave a module_size ~4% high, enough
    // to round leg_modules down to the *wrong* legal-looking value
    // (13 instead of 14) rather than just missing "legal" outright.
    // Picking the legal size whose *implied* module size (leg_px /
    // (n - 7)) is closest to the estimate is robust to exactly this
    // bias: neighbouring legal sizes are 4 modules apart, so even a
    // 20-30% biased estimate still lands closest to the right one.
    let mut best: Option<(u32, f64)> = None; // (n, relative error)
    let mut n = 21u32;
    while n <= 177 {
        let implied_module_size = leg_px / (n - 7) as f64;
        let relative_error = (implied_module_size - module_size).abs() / module_size;
        if best.is_none_or(|(_, best_err)| relative_error < best_err) {
            best = Some((n, relative_error));
        }
        n += 4;
    }
    match best {
        Some((n, relative_error)) if relative_error <= VERSION_MATCH_TOLERANCE => Some(n),
        _ => None,
    }
}

/// `perspective_warp`'s own doc: `h` maps `[x', y', 1]` in the output
/// plane to `[x, y, 1]` in the input plane, standard graphics
/// convention (`x` = column, `y` = row) -- not this crate's `(row,
/// col)` convention everything else uses. Every point handed to
/// `estimate_affine` goes through this swap so the resulting matrix is
/// directly usable by `perspective_warp` without a transpose step. See
/// the spec's §3.
fn xy(row: f64, col: f64) -> (f64, f64) {
    (col, row)
}

/// Affine-fit, warp, re-threshold, sample, and decode for one
/// (top_right, bottom_left) assignment of the two non-corner
/// candidates (§4.5).
fn attempt_decode(
    image: &PixelContainer,
    corner: (f64, f64),
    top_right: (f64, f64),
    bottom_left: (f64, f64),
    n: u32,
) -> Result<Vec<u8>, qr_decoder::QrDecodeError> {
    let mp = MODULE_PIXELS as f64;
    let nf = n as f64;
    let from = [
        xy(3.5 * mp, 3.5 * mp),
        xy(3.5 * mp, (nf - 3.5) * mp),
        xy((nf - 3.5) * mp, 3.5 * mp),
    ];
    let to = [
        xy(corner.0, corner.1),
        xy(top_right.0, top_right.1),
        xy(bottom_left.0, bottom_left.1),
    ];

    let h64 = match vis_geometric_estimation::estimate_affine(&from, &to) {
        Ok(h) => h,
        // Unreachable in practice: find_best_triangle already rejects
        // near-collinear geometry (the angle-cosine and leg-ratio gates)
        // before a triple ever reaches here, so estimate_affine's own
        // error variants (WrongPointCount -- always exactly 3 passed;
        // Degenerate -- collinear points) shouldn't trigger. Folded into
        // a decode failure rather than panicking or inventing a new
        // error variant for an effectively-dead path.
        Err(_) => return Err(qr_decoder::QrDecodeError::Truncated),
    };
    let mut h32 = [[0f32; 3]; 3];
    for (r, row) in h64.iter().enumerate() {
        for (c, &v) in row.iter().enumerate() {
            h32[r][c] = v as f32;
        }
    }

    let out_size = n * MODULE_PIXELS;
    let warped = image_geometric_transforms::perspective_warp(
        image,
        h32,
        out_size,
        out_size,
        Interpolation::Bilinear,
        OutOfBounds::Zero,
    );

    let t = otsu_threshold(&warped);
    let binarized = image_point_ops::threshold_luminance(&warped, t);

    let mut modules = vec![vec![false; n as usize]; n as usize];
    for (r, row) in modules.iter_mut().enumerate() {
        for (c, cell) in row.iter_mut().enumerate() {
            let px = c as u32 * MODULE_PIXELS + MODULE_PIXELS / 2;
            let py = r as u32 * MODULE_PIXELS + MODULE_PIXELS / 2;
            let (red, _, _, _) = binarized.pixel_at(px, py);
            *cell = red == 0;
        }
    }

    let grid = barcode_2d::ModuleGrid {
        rows: n,
        cols: n,
        modules,
        module_shape: barcode_2d::ModuleShape::Square,
    };
    qr_decoder::decode(&grid)
}

/// Otsu's method: the global threshold maximizing between-class
/// luminance variance, picked from `image`'s own histogram. Used
/// *after* the affine warp, not before -- the warp has already
/// normalized scale/rotation, so a global (not adaptive/local)
/// threshold is enough at this stage, and cheaper. Self-contained
/// rather than its own `IMG` package: it's a well-scoped implementation
/// detail of this one step, not a reusable primitive the way `IMG09`'s
/// adaptive threshold is.
fn otsu_threshold(image: &PixelContainer) -> u8 {
    let (width, height) = (image.width, image.height);
    if width == 0 || height == 0 {
        return 128;
    }

    let mut histogram = [0u32; 256];
    for y in 0..height {
        for x in 0..width {
            let (r, g, b, _a) = image.pixel_at(x, y);
            let luminance = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32).round() as usize;
            histogram[luminance.min(255)] += 1;
        }
    }

    let total = width as u64 * height as u64;
    let sum_all: f64 = histogram
        .iter()
        .enumerate()
        .map(|(i, &count)| i as f64 * count as f64)
        .sum();

    let mut sum_below = 0.0f64;
    let mut weight_below = 0u64;
    let mut best_variance = -1.0f64;
    // Track the low and high ends of the plateau of t values tied for
    // the best between-class variance, not just the first one reached.
    // A cleanly bimodal image (e.g. a QR straightened by a near-integer
    // scale factor, with no intermediate luminance values at all) ties
    // the variance across the *entire* empty gap between the two
    // clusters -- picking the first t in that tie (as small as 0) is a
    // real bug, not just a style choice: threshold_luminance classifies
    // a pixel dark iff its luminance is strictly less than the
    // threshold, so t=0 makes every non-negative luminance value
    // (i.e. every pixel) light, the exact opposite of a usable
    // threshold. Caught empirically -- a hardcoded mid-range threshold
    // decoded a synthetic test image perfectly, but the plain Otsu
    // first-tie-wins version decoded nothing -- before this crate's
    // first push. The midpoint of the tied plateau is the textbook-
    // correct Otsu answer for well-separated clusters regardless: it
    // sits in the actual gap between them, not at either edge.
    let mut plateau_lo = 0u8;
    let mut plateau_hi = 0u8;
    for (t, &count) in histogram.iter().enumerate() {
        weight_below += count as u64;
        if weight_below == 0 {
            continue;
        }
        let weight_above = total - weight_below;
        if weight_above == 0 {
            break;
        }
        sum_below += t as f64 * count as f64;
        let mean_below = sum_below / weight_below as f64;
        let mean_above = (sum_all - sum_below) / weight_above as f64;
        let variance_between =
            weight_below as f64 * weight_above as f64 * (mean_below - mean_above).powi(2);
        if variance_between > best_variance {
            best_variance = variance_between;
            plateau_lo = t as u8;
            plateau_hi = t as u8;
        } else if variance_between == best_variance {
            plateau_hi = t as u8;
        }
    }
    ((plateau_lo as u32 + plateau_hi as u32) / 2) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rc(row: f64, col: f64, module_size: f64) -> RefinedCandidate {
        RefinedCandidate {
            row,
            col,
            module_size,
            support: 1,
        }
    }

    #[test]
    fn estimate_version_recovers_known_sizes() {
        assert_eq!(estimate_version(140.0, 10.0), Some(21)); // version 1: 14 modules * 10px
        assert_eq!(estimate_version(240.0, 8.0), Some(37)); // version 5: 30 modules * 8px
        assert_eq!(estimate_version(1360.0, 8.0), Some(177)); // version 40: 170 modules * 8px
    }

    #[test]
    fn estimate_version_tolerates_rotation_scale_bias() {
        // A rotated finder pattern's scanline-measured module_size can be
        // biased (~4% observed empirically); the nearest legal size by
        // implied module size should still win over a naive
        // round-the-ratio approach, which would have picked n=20 (illegal)
        // for this input instead of the correct n=21.
        assert_eq!(estimate_version(140.065, 10.38), Some(21));
    }

    #[test]
    fn estimate_version_rejects_implausible_measurements() {
        // Even the closest legal size is a wildly poor match -- no legal
        // QR size has anywhere near this module size for this leg length.
        assert_eq!(estimate_version(1_000_000.0, 10.0), None);
        assert_eq!(estimate_version(5.0, 10.0), None);
    }

    #[test]
    fn estimate_version_rejects_bad_inputs() {
        assert_eq!(estimate_version(140.0, 0.0), None);
        assert_eq!(estimate_version(140.0, -5.0), None);
        assert_eq!(estimate_version(140.0, f64::NAN), None);
        assert_eq!(estimate_version(140.0, f64::INFINITY), None);
        assert_eq!(estimate_version(0.0, 10.0), None);
        assert_eq!(estimate_version(-140.0, 10.0), None);
        assert_eq!(estimate_version(f64::NAN, 10.0), None);
    }

    #[test]
    fn find_best_triangle_locates_a_clean_right_triangle() {
        let candidates = vec![rc(0.0, 0.0, 10.0), rc(0.0, 100.0, 10.0), rc(100.0, 0.0, 10.0)];
        let t = find_best_triangle(&candidates).expect("should find the triangle");
        assert_eq!(t.corner, 0);
        assert!((t.leg_px - 100.0).abs() < 1.0);
        assert!((t.module_size - 10.0).abs() < 1e-9);
    }

    #[test]
    fn find_best_triangle_needs_at_least_three() {
        let candidates = vec![rc(0.0, 0.0, 10.0), rc(0.0, 100.0, 10.0)];
        assert!(find_best_triangle(&candidates).is_none());
    }

    #[test]
    fn find_best_triangle_rejects_a_scalene_triangle() {
        // Legs of wildly different lengths from every possible corner
        // choice -- not remotely a right isoceles triangle.
        let candidates = vec![rc(0.0, 0.0, 10.0), rc(0.0, 10.0, 10.0), rc(200.0, 5.0, 10.0)];
        assert!(find_best_triangle(&candidates).is_none());
    }

    #[test]
    fn cluster_candidates_merges_nearby_and_separates_far() {
        let raw = vec![
            PatternCandidate {
                row: 10.0,
                col: 10.0,
                module_size: 2.0,
            },
            PatternCandidate {
                row: 11.0,
                col: 10.0,
                module_size: 2.0,
            },
            PatternCandidate {
                row: 80.0,
                col: 80.0,
                module_size: 2.0,
            },
        ];
        let refined = cluster_candidates(100, 100, &raw);
        assert_eq!(refined.len(), 2, "expected the two nearby candidates to merge");
        let total_support: usize = refined.iter().map(|r| r.support).sum();
        assert_eq!(total_support, 3);
    }

    #[test]
    fn cluster_candidates_handles_empty_and_zero_size() {
        assert!(cluster_candidates(100, 100, &[]).is_empty());
        let raw = vec![PatternCandidate {
            row: 5.0,
            col: 5.0,
            module_size: 2.0,
        }];
        assert!(cluster_candidates(0, 0, &raw).is_empty());
    }
}
