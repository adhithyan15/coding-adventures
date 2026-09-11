//! # `vis-connected-components` — VIS02
//!
//! Two-pass union-find connected-component labeling over a binary
//! image: which lit ("foreground") pixels touch which, so a scattered
//! set of foreground pixels becomes a small list of *blobs*, each with
//! a pixel count, a bounding box, and a centroid.
//!
//! ## Why two passes
//!
//! A single left-to-right, top-to-bottom scan cannot always tell two
//! provisional labels are the same blob until a later pixel connects
//! them. The classic case is a U-shape:
//!
//! ```text
//! # . #      Scanning row-major, the two top pixels look like two
//! # . #      separate blobs (nothing on their row or the row above
//! # # #      connects them) -- until the bottom row is reached, which
//!            joins the left and right arms into one blob.
//! ```
//!
//! Pass 1 assigns a *provisional* label to every foreground pixel from
//! its already-visited neighbors, and records an *equivalence* (via
//! union-find) whenever two different provisional labels turn out to
//! touch the same pixel. Pass 2 resolves every pixel's provisional
//! label to its final, compacted (1-based) label using those
//! equivalences, and accumulates each blob's stats along the way.
//!
//! ## Connectivity
//!
//! Whether two diagonally-touching pixels belong to the same blob is a
//! caller choice ([`Connectivity::Four`] says no, [`Connectivity::
//! Eight`] says yes) — this crate doesn't hardcode one, since which is
//! correct depends on what the caller is trying to detect (see the
//! crate's own spec, `VIS02-connected-components.md`, §3.1).
//!
//! ## Usage
//!
//! ```
//! use vis_connected_components::{label_components, Connectivity};
//!
//! let bitmap = vec![
//!     vec![true,  true,  false],
//!     vec![false, false, false],
//!     vec![false, true,  true],
//! ];
//! let labeling = label_components(&bitmap, Connectivity::Four);
//! assert_eq!(labeling.components.len(), 2); // two separate blobs
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::HashMap;

/// Which neighboring pixels count as "touching" for the purpose of
/// belonging to the same blob.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Connectivity {
    /// Up, down, left, right only.
    Four,
    /// Four-connectivity plus the four diagonal neighbors.
    Eight,
}

/// One connected blob of foreground pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Component {
    /// 1-based label. Matches the value this component's pixels carry
    /// in [`Labeling::labels`]. Never `0` — `0` is reserved for
    /// background.
    pub label: usize,
    /// How many foreground pixels belong to this blob.
    pub pixel_count: usize,
    /// Bounding box, inclusive, in `(row, col)`.
    pub min_row: usize,
    /// Bounding box, inclusive, in `(row, col)`.
    pub max_row: usize,
    /// Bounding box, inclusive, in `(row, col)`.
    pub min_col: usize,
    /// Bounding box, inclusive, in `(row, col)`.
    pub max_col: usize,
    /// Mean row coordinate of every pixel in this blob.
    pub centroid_row: f64,
    /// Mean column coordinate of every pixel in this blob.
    pub centroid_col: f64,
}

/// The result of labeling one bitmap.
#[derive(Debug, Clone, PartialEq)]
pub struct Labeling {
    /// Same shape as the input bitmap (including any raggedness --
    /// `labels[r]` has exactly as many entries as `bitmap[r]` did).
    /// `0` means background; any other value is the 1-based label of
    /// the component that pixel belongs to.
    pub labels: Vec<Vec<usize>>,
    /// One entry per component, ordered by `label` ascending, so
    /// `components[i].label == i + 1`.
    pub components: Vec<Component>,
}

/// A minimal union-find (disjoint-set) structure with path compression
/// and union by rank -- the standard toolkit for resolving "these two
/// provisional labels turned out to be the same blob" as pass 1
/// discovers it, in effectively-constant amortized time per operation.
struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl UnionFind {
    fn new() -> Self {
        Self { parent: Vec::new(), rank: Vec::new() }
    }

    /// Add a new singleton set, returning its id.
    fn make_set(&mut self) -> usize {
        let id = self.parent.len();
        self.parent.push(id);
        self.rank.push(0);
        id
    }

    /// The representative (root) of the set containing `x`, with path
    /// compression so repeated lookups on the same tree flatten it.
    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    /// Merge the sets containing `a` and `b`, by rank (attach the
    /// shorter tree under the taller one's root, so no tree grows
    /// taller than necessary).
    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra == rb {
            return;
        }
        match self.rank[ra].cmp(&self.rank[rb]) {
            std::cmp::Ordering::Less => self.parent[ra] = rb,
            std::cmp::Ordering::Greater => self.parent[rb] = ra,
            std::cmp::Ordering::Equal => {
                self.parent[rb] = ra;
                self.rank[ra] += 1;
            }
        }
    }
}

/// Label every connected foreground blob in `bitmap` (`true` =
/// foreground, `false` = background).
///
/// Never panics: an empty bitmap, a ragged one (rows of differing
/// length), an all-background bitmap, and an all-foreground bitmap are
/// all valid input, each producing a well-defined (possibly empty)
/// result rather than an error -- there is no shape or content this
/// function rejects, only bitmaps it labels differently.
pub fn label_components(bitmap: &[Vec<bool>], connectivity: Connectivity) -> Labeling {
    let rows = bitmap.len();
    let mut provisional: Vec<Vec<usize>> = bitmap.iter().map(|row| vec![0usize; row.len()]).collect();
    let mut dsu = UnionFind::new();

    // --- Pass 1: provisional labels + equivalences -----------------
    for r in 0..rows {
        let cols = bitmap[r].len();
        for c in 0..cols {
            if !bitmap[r][c] {
                continue;
            }
            let neighbors = already_visited_neighbors(r, c, connectivity);
            let mut found: Vec<usize> = Vec::new();
            for (nr, nc) in neighbors {
                if nr >= rows {
                    continue;
                }
                if nc >= bitmap[nr].len() {
                    continue;
                }
                if bitmap[nr][nc] {
                    let label = provisional[nr][nc];
                    if label != 0 {
                        found.push(label);
                    }
                }
            }
            if found.is_empty() {
                // dsu ids are 0-based internally; provisional labels
                // stored in the grid are that id + 1, so 0 stays
                // reserved for "no label yet" without a separate
                // sentinel type.
                let id = dsu.make_set();
                provisional[r][c] = id + 1;
            } else {
                let min_label = *found.iter().min().unwrap();
                provisional[r][c] = min_label;
                for &label in &found {
                    dsu.union(min_label - 1, label - 1);
                }
            }
        }
    }

    // --- Pass 2: resolve to final compact labels + accumulate stats -
    let mut root_to_final: HashMap<usize, usize> = HashMap::new();
    let mut labels: Vec<Vec<usize>> = bitmap.iter().map(|row| vec![0usize; row.len()]).collect();
    // Accumulators indexed by final label - 1: (count, min_r, max_r, min_c, max_c, sum_r, sum_c)
    let mut acc: Vec<(usize, usize, usize, usize, usize, f64, f64)> = Vec::new();

    for r in 0..rows {
        let cols = bitmap[r].len();
        for c in 0..cols {
            if !bitmap[r][c] {
                continue;
            }
            let provisional_id = provisional[r][c] - 1;
            let root = dsu.find(provisional_id);
            let final_label = *root_to_final.entry(root).or_insert_with(|| {
                acc.push((0, r, r, c, c, 0.0, 0.0));
                acc.len() // 1-based: this is the index we just pushed to, +1
            });
            labels[r][c] = final_label;

            let entry = &mut acc[final_label - 1];
            entry.0 += 1; // count
            entry.1 = entry.1.min(r); // min_row
            entry.2 = entry.2.max(r); // max_row
            entry.3 = entry.3.min(c); // min_col
            entry.4 = entry.4.max(c); // max_col
            entry.5 += r as f64; // sum_row
            entry.6 += c as f64; // sum_col
        }
    }

    let components = acc
        .into_iter()
        .enumerate()
        .map(|(i, (count, min_r, max_r, min_c, max_c, sum_r, sum_c))| Component {
            label: i + 1,
            pixel_count: count,
            min_row: min_r,
            max_row: max_r,
            min_col: min_c,
            max_col: max_c,
            centroid_row: sum_r / count as f64,
            centroid_col: sum_c / count as f64,
        })
        .collect();

    Labeling { labels, components }
}

/// The already-visited neighbors of `(r, c)` in a row-major top-to-
/// bottom, left-to-right scan -- i.e. only neighbors that come strictly
/// before `(r, c)` in scan order can already carry a provisional label.
/// Returns candidate coordinates without bounds-checking against the
/// actual bitmap; the caller filters those (also handling raggedness,
/// since a neighbor's row may be shorter than `c`).
fn already_visited_neighbors(r: usize, c: usize, connectivity: Connectivity) -> Vec<(usize, usize)> {
    let mut out = Vec::with_capacity(4);
    if r > 0 {
        out.push((r - 1, c)); // up
    }
    if c > 0 {
        out.push((r, c - 1)); // left
    }
    if connectivity == Connectivity::Eight {
        if r > 0 && c > 0 {
            out.push((r - 1, c - 1)); // up-left
        }
        if r > 0 {
            out.push((r - 1, c + 1)); // up-right (bounds/raggedness filtered by caller)
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recompute_component_from_labels(labeling: &Labeling, label: usize) -> (usize, usize, usize, usize, usize, f64, f64) {
        let mut count = 0usize;
        let mut min_r = usize::MAX;
        let mut max_r = 0usize;
        let mut min_c = usize::MAX;
        let mut max_c = 0usize;
        let mut sum_r = 0.0f64;
        let mut sum_c = 0.0f64;
        for (r, row) in labeling.labels.iter().enumerate() {
            for (c, &l) in row.iter().enumerate() {
                if l == label {
                    count += 1;
                    min_r = min_r.min(r);
                    max_r = max_r.max(r);
                    min_c = min_c.min(c);
                    max_c = max_c.max(c);
                    sum_r += r as f64;
                    sum_c += c as f64;
                }
            }
        }
        (count, min_r, max_r, min_c, max_c, sum_r, sum_c)
    }

    fn assert_labeling_self_consistent(labeling: &Labeling) {
        for component in &labeling.components {
            let (count, min_r, max_r, min_c, max_c, sum_r, sum_c) =
                recompute_component_from_labels(labeling, component.label);
            assert_eq!(count, component.pixel_count, "pixel_count mismatch for label {}", component.label);
            assert_eq!(min_r, component.min_row, "min_row mismatch for label {}", component.label);
            assert_eq!(max_r, component.max_row, "max_row mismatch for label {}", component.label);
            assert_eq!(min_c, component.min_col, "min_col mismatch for label {}", component.label);
            assert_eq!(max_c, component.max_col, "max_col mismatch for label {}", component.label);
            assert!((sum_r / count as f64 - component.centroid_row).abs() < 1e-12);
            assert!((sum_c / count as f64 - component.centroid_col).abs() < 1e-12);
        }
    }

    #[test]
    fn single_isolated_pixel() {
        let bitmap = vec![
            vec![false, false, false],
            vec![false, true, false],
            vec![false, false, false],
        ];
        let labeling = label_components(&bitmap, Connectivity::Eight);
        assert_eq!(labeling.components.len(), 1);
        let c = labeling.components[0];
        assert_eq!(c.pixel_count, 1);
        assert_eq!((c.min_row, c.max_row, c.min_col, c.max_col), (1, 1, 1, 1));
        assert_eq!((c.centroid_row, c.centroid_col), (1.0, 1.0));
        assert_labeling_self_consistent(&labeling);
    }

    #[test]
    fn solid_rectangle_is_one_component_with_exact_stats() {
        let bitmap = vec![
            vec![false, false, false, false],
            vec![false, true, true, false],
            vec![false, true, true, false],
            vec![false, false, false, false],
        ];
        let labeling = label_components(&bitmap, Connectivity::Four);
        assert_eq!(labeling.components.len(), 1);
        let c = labeling.components[0];
        assert_eq!(c.pixel_count, 4);
        assert_eq!((c.min_row, c.max_row, c.min_col, c.max_col), (1, 2, 1, 2));
        assert_eq!((c.centroid_row, c.centroid_col), (1.5, 1.5));
        assert_labeling_self_consistent(&labeling);
    }

    #[test]
    fn two_separated_rectangles_stay_separate() {
        let bitmap = vec![
            vec![true, true, false, false, false],
            vec![true, true, false, false, false],
            vec![false, false, false, true, true],
            vec![false, false, false, true, true],
        ];
        let labeling = label_components(&bitmap, Connectivity::Eight);
        assert_eq!(labeling.components.len(), 2);
        let mut counts: Vec<usize> = labeling.components.iter().map(|c| c.pixel_count).collect();
        counts.sort_unstable();
        assert_eq!(counts, vec![4, 4]);
        assert_labeling_self_consistent(&labeling);
    }

    #[test]
    fn u_shape_is_one_component_requiring_two_passes() {
        // # . #
        // # . #
        // # # #
        let bitmap = vec![
            vec![true, false, true],
            vec![true, false, true],
            vec![true, true, true],
        ];
        let labeling = label_components(&bitmap, Connectivity::Four);
        assert_eq!(labeling.components.len(), 1);
        assert_eq!(labeling.components[0].pixel_count, 7);
        assert_labeling_self_consistent(&labeling);
    }

    #[test]
    fn diagonal_touch_differs_by_connectivity() {
        let bitmap = vec![
            vec![true, false],
            vec![false, true],
        ];
        let four = label_components(&bitmap, Connectivity::Four);
        let eight = label_components(&bitmap, Connectivity::Eight);
        assert_eq!(four.components.len(), 2, "4-connectivity must NOT merge a diagonal-only touch");
        assert_eq!(eight.components.len(), 1, "8-connectivity MUST merge a diagonal-only touch");
        assert_labeling_self_consistent(&four);
        assert_labeling_self_consistent(&eight);
    }

    #[test]
    fn empty_bitmap_has_no_components() {
        let bitmap: Vec<Vec<bool>> = vec![];
        let labeling = label_components(&bitmap, Connectivity::Eight);
        assert!(labeling.components.is_empty());
        assert!(labeling.labels.is_empty());
    }

    #[test]
    fn all_background_has_no_components() {
        let bitmap = vec![vec![false; 5]; 5];
        let labeling = label_components(&bitmap, Connectivity::Eight);
        assert!(labeling.components.is_empty());
        assert!(labeling.labels.iter().all(|row| row.iter().all(|&l| l == 0)));
    }

    #[test]
    fn all_foreground_is_one_component() {
        let bitmap = vec![vec![true; 4]; 3];
        let labeling = label_components(&bitmap, Connectivity::Four);
        assert_eq!(labeling.components.len(), 1);
        assert_eq!(labeling.components[0].pixel_count, 12);
        assert_labeling_self_consistent(&labeling);
    }

    #[test]
    fn ragged_bitmap_does_not_panic_and_stays_correct() {
        let bitmap = vec![
            vec![true, true, true],
            vec![true], // shorter row
            vec![true, true],
        ];
        let labeling = label_components(&bitmap, Connectivity::Eight);
        // Shape mirrors the input exactly, including raggedness.
        assert_eq!(labeling.labels[0].len(), 3);
        assert_eq!(labeling.labels[1].len(), 1);
        assert_eq!(labeling.labels[2].len(), 2);
        assert_labeling_self_consistent(&labeling);
    }

    #[test]
    fn ragged_bitmap_with_gap_does_not_panic() {
        // Row 0 is long, row 1 is short -- pixels at (0, 2) must not
        // try to read a nonexistent (1, 2)/(1, 3) neighbor.
        let bitmap = vec![vec![true, true, true, true], vec![true]];
        let labeling = label_components(&bitmap, Connectivity::Eight);
        assert_labeling_self_consistent(&labeling);
    }

    #[test]
    #[allow(clippy::needless_range_loop)] // building a bitmap fixture by (row, col) reads clearer than an iterator rewrite
    fn realistic_scale_multiple_irregular_blobs() {
        // A 40x40 bitmap with a handful of separated irregular blobs,
        // roughly the scale a downsampled finder-pattern search window
        // might actually produce.
        let mut bitmap = vec![vec![false; 40]; 40];
        // Blob A: an L-shape near the top-left.
        for r in 2..8 {
            bitmap[r][2] = true;
        }
        for c in 2..8 {
            bitmap[7][c] = true;
        }
        // Blob B: a solid square, fully separated (gap of >=2 in both axes).
        for r in 15..22 {
            for c in 15..22 {
                bitmap[r][c] = true;
            }
        }
        // Blob C: a thin diagonal line (touches only via 8-connectivity).
        for i in 0..10 {
            bitmap[30 + i][30 + i] = true;
        }

        let labeling = label_components(&bitmap, Connectivity::Eight);
        assert_eq!(labeling.components.len(), 3);
        let mut counts: Vec<usize> = labeling.components.iter().map(|c| c.pixel_count).collect();
        counts.sort_unstable();
        // L-shape: 6 + 6 - 1 (corner counted once) = 11. Square: 49. Diagonal: 10.
        assert_eq!(counts, vec![10, 11, 49]);
        assert_labeling_self_consistent(&labeling);

        // Same bitmap under 4-connectivity: the diagonal line becomes
        // 10 separate single-pixel components (no edge-adjacency at all).
        let four = label_components(&bitmap, Connectivity::Four);
        assert_eq!(four.components.len(), 3 - 1 + 10);
    }
}
