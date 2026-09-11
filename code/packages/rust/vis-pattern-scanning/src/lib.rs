// # vis-pattern-scanning (VIS04)
//
// The general form of "scan a row of pixels for a 1:1:3:1:1 ratio of
// alternating dark/light runs." Every 2D barcode format locates itself
// in an image the same way -- a distinctive run-length signature,
// scanned across rows and columns, cross-checked between the two. QR,
// Aztec, Data Matrix, and PDF417 each use a different ratio sequence;
// this crate scans for whichever one the caller supplies.
//
// See `code/specs/VIS04-pattern-scanning.md` for the full design.
//
// ## The two-step pipeline
//
// ```text
// line of bool (one row, or one column)
//   │  decompose into alternating runs, slide a ratio-sized window
//   ▼
// RatioMatch { center, module_size }    (scan_line -- one line only)
//   │  for each horizontal match, cross-check with a vertical scan
//   │  through the same point (rejects most false positives)
//   ▼
// PatternCandidate { row, col, module_size }  (find_pattern_candidates)
// ```
//
// Clustering several nearby confirmed candidates into one refined
// center per real pattern is deliberately out of scope here -- see
// the spec's §2 for why that belongs to whatever assembles the full
// pipeline (using `vis-connected-components`), not to this crate.

/// The largest relative `tolerance` `scan_line` accepts. A `tolerance`
/// this permissive already accepts a run four and a half times its
/// expected length -- generous well past any real 2D-barcode use --
/// so the bound exists only to keep `tolerance * expected` (and, in
/// turn, how many windows "match") bounded by the input's own size,
/// not by how large a value a caller happens to pass. Without this,
/// `tolerance = f64::INFINITY` would make every foreground-starting
/// window match regardless of its actual proportions, turning
/// `find_pattern_candidates`' near-linear cost quadratic-or-worse on
/// a caller-controlled bitmap.
pub const MAX_TOLERANCE: f64 = 10.0;

/// The largest number of matches `scan_line` will report for one
/// line, regardless of `tolerance`. `MAX_TOLERANCE` alone only bounds
/// the *extreme* case (a non-finite or absurdly large tolerance); a
/// line with a periodic/checkerboard-like run structure -- real
/// photographic texture, not just an adversarial construction -- can
/// satisfy the ratio check at a large fraction of its windows using
/// an entirely ordinary, in-range `tolerance`. Since
/// `find_pattern_candidates` does an independent O(bitmap height)
/// vertical rescan per horizontal match, an unbounded match count
/// here still lets a caller-controlled bitmap drive that function's
/// cost well past what its size alone would suggest. `256` is
/// generous well past what any real finder-pattern scan needs (a
/// real 2D barcode image has at most a handful of true finder
/// patterns in frame); once hit, `scan_line` stops scanning further
/// windows on that line rather than continuing to accumulate more.
pub const MAX_MATCHES_PER_LINE: usize = 256;

/// The longest `ratio` `scan_line` accepts. Every window in the
/// matching loop costs O(`ratio.len()`) (summing and checking that
/// many runs), so an unbounded `ratio` would be a fourth caller-
/// controlled way to inflate this crate's cost beyond what the
/// line's own length suggests -- the same shape `MAX_TOLERANCE` and
/// `MAX_MATCHES_PER_LINE` close for the other two. Every real 2D-
/// barcode format's finder-pattern ratio is a short, fixed sequence
/// (QR's is 5 elements); `32` stays generous past any of them while
/// still bounding the cost.
pub const MAX_RATIO_LEN: usize = 32;

/// One ratio match found along a single scanned line, in that line's
/// own coordinate (a column position for a horizontal scan, a row
/// position for a vertical scan -- the caller knows which it passed
/// in).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RatioMatch {
    pub center: f64,
    pub module_size: f64,
}

/// One confirmed 2D location where both a horizontal and a vertical
/// scan through it found the requested run-length ratio.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PatternCandidate {
    pub row: f64,
    pub col: f64,
    /// Average of the horizontal and vertical scans' estimated pixel
    /// size for one ratio unit (for QR, approximately one module's
    /// width in the photographed image).
    pub module_size: f64,
}

/// Decompose `line` into its alternating runs as `(color, length)`
/// pairs, in order. An empty `line` yields an empty `Vec`.
fn compute_runs(line: &[bool]) -> Vec<(bool, usize)> {
    let mut runs = Vec::new();
    let mut iter = line.iter();
    if let Some(&first) = iter.next() {
        let mut current = first;
        let mut len = 1usize;
        for &value in iter {
            if value == current {
                len += 1;
            } else {
                runs.push((current, len));
                current = value;
                len = 1;
            }
        }
        runs.push((current, len));
    }
    runs
}

/// Scan one line (a single row or column, already extracted by the
/// caller) for windows of `ratio.len()` consecutive runs matching
/// `ratio` within `tolerance` (a relative fraction, e.g. `0.5` for
/// QR's usual +/-50%).
///
/// A window matches only if its first run is foreground (`true`) --
/// runs always alternate color by construction, so this alone fixes
/// every other run's expected color too. Never panics: an empty
/// `line`, an empty `ratio`, a `ratio` longer than `MAX_RATIO_LEN`, or
/// a `tolerance` outside `(0.0, MAX_TOLERANCE]` (which covers
/// non-positive, NaN, infinite, and unreasonably large values alike)
/// all simply yield no matches rather than a panic -- see the spec's
/// §6 for the full degenerate-input matrix, and `MAX_TOLERANCE`'s own
/// docs for why these bounds are enforced explicitly rather than left
/// to fall out of comparison arithmetic that happens to work for the
/// common case. The result also never exceeds `MAX_MATCHES_PER_LINE`
/// matches, regardless of `line`'s content.
///
/// # Examples
///
/// ```
/// use vis_pattern_scanning::scan_line;
///
/// // 1 dark, 1 light, 3 dark, 1 light, 1 dark -- QR's 1:1:3:1:1.
/// let line = [true, false, true, true, true, false, true];
/// let matches = scan_line(&line, &[1, 1, 3, 1, 1], 0.2);
/// assert_eq!(matches.len(), 1);
/// assert_eq!(matches[0].module_size, 1.0);
/// ```
pub fn scan_line(line: &[bool], ratio: &[u32], tolerance: f64) -> Vec<RatioMatch> {
    let mut matches = Vec::new();
    if ratio.is_empty() || line.is_empty() || ratio.len() > MAX_RATIO_LEN {
        return matches;
    }
    // A tolerance of, say, `f64::INFINITY` would make every foreground-
    // starting window "match" regardless of its actual run-length
    // proportions -- `tolerance * expected` is `+inf`, and everything
    // compares `<=` to it. That turns what should be a handful of real
    // matches into O(line length) matches, which `find_pattern_candidates`
    // then does O(bitmap height) work for *each of*, on a bitmap whose
    // size the caller controls. Bounding tolerance keeps the amount of
    // work proportional to the input size, not to how permissive a
    // caller-supplied tolerance happens to be.
    if !(0.0..=MAX_TOLERANCE).contains(&tolerance) {
        return matches;
    }
    let ratio_sum: u64 = ratio.iter().map(|&r| r as u64).sum();
    if ratio_sum == 0 {
        return matches;
    }

    let runs = compute_runs(line);
    let window_len = ratio.len();
    if runs.len() < window_len {
        return matches;
    }

    let mut starts = Vec::with_capacity(runs.len());
    let mut offset = 0usize;
    for &(_, len) in &runs {
        starts.push(offset);
        offset += len;
    }

    for window_start in 0..=(runs.len() - window_len) {
        let window = &runs[window_start..window_start + window_len];
        if !window[0].0 {
            continue;
        }

        let total_len: u64 = window.iter().map(|&(_, len)| len as u64).sum();
        let unit = total_len as f64 / ratio_sum as f64;

        let all_within_tolerance = window.iter().enumerate().all(|(i, &(_, len))| {
            let expected = ratio[i] as f64 * unit;
            let diff = (len as f64 - expected).abs();
            diff <= tolerance * expected
        });
        if !all_within_tolerance {
            continue;
        }

        let mid_lo = (window_len - 1) / 2;
        let mid_hi = window_len / 2;
        let midpoint_of = |i: usize| -> f64 {
            let (_, len) = window[i];
            starts[window_start + i] as f64 + len as f64 / 2.0
        };
        let center = if mid_lo == mid_hi {
            midpoint_of(mid_lo)
        } else {
            (midpoint_of(mid_lo) + midpoint_of(mid_hi)) / 2.0
        };

        matches.push(RatioMatch {
            center,
            module_size: unit,
        });
        // A periodic/checkerboard-like line -- runs of nearly equal
        // length throughout, the kind of texture a real photograph
        // can genuinely contain (fabric, blinds, brick) -- can satisfy
        // the ratio check at an entirely ordinary `tolerance` (nothing
        // near `MAX_TOLERANCE`) at a large fraction of its windows,
        // not just the handful a real finder pattern produces.
        // `find_pattern_candidates` does an independent O(bitmap
        // height) vertical rescan *per* horizontal match, so an
        // unbounded match count here is the same algorithmic-
        // complexity shape `MAX_TOLERANCE` closes at the tolerance
        // extreme, just reachable through the bitmap's content
        // instead. Capping matches per line bounds it regardless of
        // content, the same way `MAX_TOLERANCE` bounds it regardless
        // of the tolerance value.
        if matches.len() >= MAX_MATCHES_PER_LINE {
            break;
        }
    }

    matches
}

/// Map a (possibly fractional) index into a column line, built from
/// only the rows that reached that column (`row_for_index[i]` is the
/// real bitmap row the column line's `i`-th entry came from), back to
/// a real row coordinate. For a rectangular bitmap `row_for_index[i]
/// == i`, so this is the identity; ragged input interpolates linearly
/// between the two real rows nearest the fractional index, which is
/// an approximation (documented in the spec's §4.1), not a claim of
/// exactness for that degenerate case.
fn map_column_index_to_row(index: f64, row_for_index: &[usize]) -> f64 {
    if row_for_index.is_empty() {
        return index;
    }
    let last = row_for_index.len() - 1;
    let lo = (index.floor().max(0.0) as usize).min(last);
    let hi = (index.ceil().max(0.0) as usize).min(last);
    if lo == hi {
        return row_for_index[lo] as f64;
    }
    let frac = index - index.floor();
    row_for_index[lo] as f64 * (1.0 - frac) + row_for_index[hi] as f64 * frac
}

/// Scan every row and column of `bitmap` (`true` = foreground/dark,
/// matching `vis-connected-components`' convention) for `ratio`,
/// cross-checking each horizontal match against a vertical scan
/// through the same point. A horizontal match is confirmed only when
/// the nearest vertical match's center lands within one estimated
/// module size of it -- filtering out the many rows that match the
/// ratio by coincidence with no real vertical structure.
///
/// Returns one `PatternCandidate` per confirmed match -- deliberately
/// not deduplicated or clustered; several adjacent scanlines crossing
/// the same real pattern typically produce several nearby candidates
/// (see the spec's §2 for why clustering is a separate concern).
/// Never panics, including on an empty or ragged (`Vec<Vec<bool>>`
/// with rows of differing length) bitmap: a column scan only includes
/// rows that actually reach that column.
///
/// # Examples
///
/// ```
/// use vis_pattern_scanning::find_pattern_candidates;
///
/// // A synthetic QR-style finder pattern (dark ring, light ring,
/// // dark 3x3 center) padded with a light border, so both the
/// // horizontal and vertical scanlines through its middle see
/// // 1:1:3:1:1.
/// let t = true;
/// let f = false;
/// let bitmap = vec![
///     vec![f, f, f, f, f, f, f, f, f],
///     vec![f, t, t, t, t, t, t, t, f],
///     vec![f, t, f, f, f, f, f, t, f],
///     vec![f, t, f, t, t, t, f, t, f],
///     vec![f, t, f, t, t, t, f, t, f],
///     vec![f, t, f, t, t, t, f, t, f],
///     vec![f, t, f, f, f, f, f, t, f],
///     vec![f, t, t, t, t, t, t, t, f],
///     vec![f, f, f, f, f, f, f, f, f],
/// ];
/// let candidates = find_pattern_candidates(&bitmap, &[1, 1, 3, 1, 1], 0.2);
/// assert!(!candidates.is_empty());
/// assert!((candidates[0].row - 4.0).abs() < 1.0);
/// assert!((candidates[0].col - 4.0).abs() < 1.0);
/// ```
pub fn find_pattern_candidates(
    bitmap: &[Vec<bool>],
    ratio: &[u32],
    tolerance: f64,
) -> Vec<PatternCandidate> {
    let mut candidates = Vec::new();
    if ratio.is_empty() || bitmap.is_empty() {
        return candidates;
    }

    for (row_idx, row) in bitmap.iter().enumerate() {
        for h in scan_line(row, ratio, tolerance) {
            let col = h.center;
            if !col.is_finite() || col < 0.0 {
                continue;
            }
            let col_idx = col.round() as usize;

            let mut column_line = Vec::new();
            let mut row_for_index = Vec::new();
            for (r_idx, r) in bitmap.iter().enumerate() {
                if col_idx < r.len() {
                    column_line.push(r[col_idx]);
                    row_for_index.push(r_idx);
                }
            }

            let mut best_row: Option<f64> = None;
            let mut best_module = 0.0_f64;
            let mut best_dist = f64::INFINITY;
            for v in scan_line(&column_line, ratio, tolerance) {
                let mapped_row = map_column_index_to_row(v.center, &row_for_index);
                let dist = (mapped_row - row_idx as f64).abs();
                let avg_module = (h.module_size + v.module_size) / 2.0;
                if dist <= avg_module && dist < best_dist {
                    best_dist = dist;
                    best_row = Some(mapped_row);
                    best_module = avg_module;
                }
            }

            if let Some(row_est) = best_row {
                candidates.push(PatternCandidate {
                    row: row_est,
                    col,
                    module_size: best_module,
                });
            }
        }
    }

    candidates
}
