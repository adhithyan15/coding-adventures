use vis_pattern_scanning::{find_pattern_candidates, scan_line};

// ---------------------------------------------------------------------
// scan_line: exact and tolerant matches
// ---------------------------------------------------------------------

#[test]
fn scan_line_finds_exact_ratio_match() {
    let line = [true, false, true, true, true, false, true];
    let matches = scan_line(&line, &[1, 1, 3, 1, 1], 0.0);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].module_size, 1.0);
    assert_eq!(matches[0].center, 3.5);
}

#[test]
fn scan_line_finds_within_tolerance_match() {
    // Same 1:1:3:1:1 shape scaled up by 2 modules/unit, with the
    // middle run one pixel short of exactly 6 -- within a 50%
    // tolerance of the expected length, so still a match.
    let mut line = Vec::new();
    line.extend([true; 2]); // 1 unit dark
    line.extend([false; 2]); // 1 unit light
    line.extend([true; 5]); // 3 units dark, one pixel short of 6
    line.extend([false; 2]); // 1 unit light
    line.extend([true; 2]); // 1 unit dark
    let matches = scan_line(&line, &[1, 1, 3, 1, 1], 0.5);
    assert_eq!(matches.len(), 1);
}

#[test]
fn scan_line_rejects_just_outside_tolerance() {
    // Middle run of length 2 instead of ~6 (unit ~2) is far outside
    // even a generous 50% tolerance.
    let mut line = Vec::new();
    line.extend([true; 2]);
    line.extend([false; 2]);
    line.extend([true; 2]); // should be ~6 for a 1:1:3:1:1 shape
    line.extend([false; 2]);
    line.extend([true; 2]);
    let matches = scan_line(&line, &[1, 1, 3, 1, 1], 0.5);
    assert!(matches.is_empty());
}

#[test]
fn scan_line_requires_window_to_start_on_dark_run() {
    // The ratio [1, 1, 3, 1, 1] shape is present, but shifted by one
    // run so the window that would match starts on a light run --
    // must not be reported as a match at that (wrong) offset, and
    // there is no valid dark-starting window in this line at all.
    let line = [false, true, false, false, false, true, false];
    let matches = scan_line(&line, &[1, 1, 3, 1, 1], 0.2);
    assert!(matches.is_empty());
}

#[test]
fn scan_line_finds_real_match_not_just_rejects_misaligned_one() {
    // A light-starting run sequence, immediately followed by a real
    // dark-starting 1:1:3:1:1 match -- proving the scan finds the
    // correctly-aligned window rather than only correctly rejecting
    // the misaligned one.
    let line = [false, true, false, true, true, true, false, true];
    let matches = scan_line(&line, &[1, 1, 3, 1, 1], 0.2);
    assert_eq!(matches.len(), 1);
}

#[test]
fn scan_line_finds_multiple_non_overlapping_matches() {
    let mut line = Vec::new();
    line.extend([true, false, true, true, true, false, true]); // match 1
    line.push(false); // gap
    line.extend([true, false, true, true, true, false, true]); // match 2
    let matches = scan_line(&line, &[1, 1, 3, 1, 1], 0.0);
    assert_eq!(matches.len(), 2);
}

#[test]
fn scan_line_no_match_on_unrelated_line() {
    let line = [true, true, true, true, true, true, true];
    let matches = scan_line(&line, &[1, 1, 3, 1, 1], 0.2);
    assert!(matches.is_empty());
}

// ---------------------------------------------------------------------
// scan_line: degenerate input never panics
// ---------------------------------------------------------------------

#[test]
fn scan_line_empty_line_yields_no_matches() {
    let matches = scan_line(&[], &[1, 1, 3, 1, 1], 0.5);
    assert!(matches.is_empty());
}

#[test]
fn scan_line_empty_ratio_yields_no_matches() {
    let line = [true, false, true, true, true, false, true];
    let matches = scan_line(&line, &[], 0.5);
    assert!(matches.is_empty());
}

#[test]
fn scan_line_zero_tolerance_is_exact_only() {
    let line = [true, false, true, true, true, false, true];
    // Exact match at 0.0 tolerance still succeeds.
    assert_eq!(scan_line(&line, &[1, 1, 3, 1, 1], 0.0).len(), 1);

    // A one-pixel-off middle run fails at 0.0 tolerance.
    let mut off = Vec::new();
    off.extend([true, false]);
    off.extend([true; 4]); // 4 instead of 3
    off.extend([false, true]);
    assert!(scan_line(&off, &[1, 1, 3, 1, 1], 0.0).is_empty());
}

#[test]
fn scan_line_negative_tolerance_yields_no_matches() {
    let line = [true, false, true, true, true, false, true];
    let matches = scan_line(&line, &[1, 1, 3, 1, 1], -0.5);
    assert!(matches.is_empty());
}

#[test]
fn scan_line_nan_tolerance_yields_no_matches() {
    let line = [true, false, true, true, true, false, true];
    let matches = scan_line(&line, &[1, 1, 3, 1, 1], f64::NAN);
    assert!(matches.is_empty());
}

#[test]
fn scan_line_all_zero_ratio_yields_no_matches_not_division_by_zero() {
    let line = [true, false, true, true, true, false, true];
    let matches = scan_line(&line, &[0, 0, 0, 0, 0], 0.5);
    assert!(matches.is_empty());
}

// ---------------------------------------------------------------------
// find_pattern_candidates: synthetic finder pattern
// ---------------------------------------------------------------------

/// A 9x9 bitmap holding a classic QR-style finder pattern (dark
/// ring, light ring, dark 3x3 center) padded with a one-pixel light
/// border, centered at (4, 4).
fn synthetic_finder_pattern() -> Vec<Vec<bool>> {
    let t = true;
    let f = false;
    vec![
        vec![f, f, f, f, f, f, f, f, f],
        vec![f, t, t, t, t, t, t, t, f],
        vec![f, t, f, f, f, f, f, t, f],
        vec![f, t, f, t, t, t, f, t, f],
        vec![f, t, f, t, t, t, f, t, f],
        vec![f, t, f, t, t, t, f, t, f],
        vec![f, t, f, f, f, f, f, t, f],
        vec![f, t, t, t, t, t, t, t, f],
        vec![f, f, f, f, f, f, f, f, f],
    ]
}

#[test]
fn find_pattern_candidates_locates_synthetic_finder_pattern() {
    let bitmap = synthetic_finder_pattern();
    let candidates = find_pattern_candidates(&bitmap, &[1, 1, 3, 1, 1], 0.2);
    assert!(!candidates.is_empty());
    for c in &candidates {
        assert!((c.row - 4.0).abs() < 1.0, "row {} not near 4.0", c.row);
        assert!((c.col - 4.0).abs() < 1.0, "col {} not near 4.0", c.col);
        assert!((c.module_size - 1.0).abs() < 0.5);
    }
}

#[test]
fn find_pattern_candidates_cross_check_rejects_false_positive() {
    // A single wide dark bar produces a horizontal 1:1:3:1:1 match on
    // its own row, but nothing resembling that ratio vertically
    // through the same point -- the cross-check must reject it.
    let mut bitmap = Vec::new();
    for _ in 0..9 {
        bitmap.push(vec![false; 7]);
    }
    // Row 4 alone carries the horizontal ratio shape.
    bitmap[4] = vec![true, false, true, true, true, false, true];

    let candidates = find_pattern_candidates(&bitmap, &[1, 1, 3, 1, 1], 0.2);
    assert!(
        candidates.is_empty(),
        "expected no confirmed candidates, got {:?}",
        candidates
    );
}

#[test]
fn find_pattern_candidates_locates_two_separated_patterns() {
    let pattern = synthetic_finder_pattern();
    let mut bitmap = vec![vec![false; 22]; 9];
    for (r, row) in pattern.iter().enumerate() {
        for (c, &v) in row.iter().enumerate() {
            bitmap[r][c] = v;
            bitmap[r][c + 13] = v;
        }
    }

    let candidates = find_pattern_candidates(&bitmap, &[1, 1, 3, 1, 1], 0.2);
    let near_first = candidates.iter().any(|c| (c.col - 4.0).abs() < 1.0);
    let near_second = candidates.iter().any(|c| (c.col - 17.0).abs() < 1.0);
    assert!(near_first, "no candidate near the first pattern");
    assert!(near_second, "no candidate near the second pattern");
}

// ---------------------------------------------------------------------
// find_pattern_candidates: degenerate input never panics
// ---------------------------------------------------------------------

#[test]
fn find_pattern_candidates_empty_bitmap() {
    let candidates = find_pattern_candidates(&[], &[1, 1, 3, 1, 1], 0.2);
    assert!(candidates.is_empty());
}

#[test]
fn find_pattern_candidates_all_background_bitmap() {
    let bitmap = vec![vec![false; 10]; 10];
    let candidates = find_pattern_candidates(&bitmap, &[1, 1, 3, 1, 1], 0.2);
    assert!(candidates.is_empty());
}

#[test]
fn find_pattern_candidates_ragged_bitmap_does_not_panic() {
    let bitmap = vec![
        vec![true, false, true, true, true, false, true],
        vec![true, false, true],
        vec![],
        vec![true, false, true, true, true, false, true, false, true],
    ];
    // Only checking this returns without panicking; ragged input's
    // exact answer is documented as approximate, not asserted here.
    let _ = find_pattern_candidates(&bitmap, &[1, 1, 3, 1, 1], 0.2);
}

#[test]
fn find_pattern_candidates_empty_ratio_yields_no_candidates() {
    let bitmap = synthetic_finder_pattern();
    let candidates = find_pattern_candidates(&bitmap, &[], 0.2);
    assert!(candidates.is_empty());
}
