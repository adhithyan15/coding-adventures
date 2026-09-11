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
/// `line`, an empty `ratio`, or a non-positive/NaN `tolerance` all
/// simply yield no matches (the plain floating-point comparisons below
/// are false, never a panic, for every one of those cases -- see the
/// spec's §6 for why no special-casing is needed).
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
    if ratio.is_empty() || line.is_empty() {
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
