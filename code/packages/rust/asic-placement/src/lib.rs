//! # ASIC Placement
//!
//! Simulated-annealing placement on top of an ASIC floorplan.
//!
//! ## Algorithm summary
//!
//! 1. **Initialize** — assign each cell to a random row that has remaining
//!    capacity; pack left-to-right within rows.
//! 2. **Anneal** — for `iterations` steps: randomly swap two cells (which
//!    may be in different rows); accept if HPWL improves, or with probability
//!    `exp(-ΔE / T)` if it doesn't. Temperature `T` decays geometrically.
//! 3. **Legalize** — after annealing, snap each cell to row coordinates and
//!    pack left-to-right to eliminate overlaps.
//!
//! ## HPWL (Half-Perimeter WireLength)
//!
//! For each net, HPWL = (x_max − x_min) + (y_max − y_min) over all cells
//! on that net. Minimizing HPWL correlates well with minimizing total wire length.
//!
//! ## Usage
//!
//! ```rust
//! use asic_placement::{place, CellSize, PlacementOptions};
//! use asic_floorplan::{compute_floorplan, CellInstanceEstimate, FloorplanOptions};
//!
//! let cells = vec![
//!     CellInstanceEstimate { instance_name: "xor_0".into(), cell_type: "xor2_1".into(), area: 6.45 },
//!     CellInstanceEstimate { instance_name: "and_0".into(), cell_type: "and2_1".into(), area: 4.60 },
//! ];
//! let opts = FloorplanOptions::sky130_hd();
//! let fp = compute_floorplan(&cells, &[], &opts).unwrap();
//!
//! let mut sizes = std::collections::HashMap::new();
//! sizes.insert("xor2_1".to_string(), CellSize { cell_type: "xor2_1".into(), width: 1.38, height: 2.72 });
//! sizes.insert("and2_1".to_string(), CellSize { cell_type: "and2_1".into(), width: 1.38, height: 2.72 });
//!
//! let (placed_def, report) = place(&fp, &sizes, None, None).unwrap();
//! assert_eq!(report.cells_placed, 2);
//! ```

use std::collections::HashMap;

use asic_floorplan::Floorplan;
use lef_def::{Component, Def};

/// Physical footprint of one cell type.
#[derive(Debug, Clone)]
pub struct CellSize {
    pub cell_type: String,
    /// Width in µm.
    pub width: f64,
    /// Height in µm (typically = site_height, e.g. 2.72).
    pub height: f64,
}

/// Options for the placement engine.
#[derive(Debug, Clone)]
pub struct PlacementOptions {
    /// Number of annealing moves. 0 skips annealing (initial placement only).
    pub iterations: u32,
    /// Random seed for reproducibility.
    pub seed: u64,
    /// Whether to run legalization after annealing. Default `true`.
    pub legalize: bool,
}

impl Default for PlacementOptions {
    fn default() -> Self {
        Self { iterations: 50_000, seed: 42, legalize: true }
    }
}

/// Placement statistics.
#[derive(Debug, Default)]
pub struct PlacementReport {
    pub final_hpwl: f64,
    pub cells_placed: usize,
    pub accepted_swaps: u32,
    pub rejected_swaps: u32,
}

/// Placement error.
#[derive(Debug)]
pub enum PlacementError {
    NoRows,
    CellDoesNotFit { name: String, width: f64 },
}

impl std::fmt::Display for PlacementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlacementError::NoRows => write!(f, "floorplan has no rows; cannot place"),
            PlacementError::CellDoesNotFit { name, width } => {
                write!(f, "cell {name:?} (width {width}) doesn't fit in any row")
            }
        }
    }
}

impl std::error::Error for PlacementError {}

// ---------------------------------------------------------------------------
// Internal mutable placement state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct PlacedCell {
    name: String,
    cell_type: String,
    width: f64,
    x: f64,
    y: f64,
    row_index: usize,
}

// ---------------------------------------------------------------------------
// Minimal PRNG (xorshift64) — no external rand dependency
// ---------------------------------------------------------------------------

struct Xorshift64(u64);

impl Xorshift64 {
    fn new(seed: u64) -> Self { Self(if seed == 0 { 1 } else { seed }) }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn usize_below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn f64_01(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Place every component in the floorplan.
///
/// `nets`: optional list of nets, each a list of component instance names
/// that share a connection. Used for HPWL minimization. `None` → no
/// wirelength optimization (still produces legally placed cells).
///
/// `options`: `None` → `PlacementOptions::default()`.
pub fn place(
    fp: &Floorplan,
    cell_sizes: &HashMap<String, CellSize>,
    nets: Option<&[Vec<String>]>,
    options: Option<PlacementOptions>,
) -> Result<(Def, PlacementReport), PlacementError> {
    if fp.rows.is_empty() {
        return Err(PlacementError::NoRows);
    }
    let opts = options.unwrap_or_default();
    let mut rng = Xorshift64::new(opts.seed);

    let row_capacity = fp.rows[0].num_x as f64 * fp.rows[0].step_x;
    let mut row_widths_used = vec![0.0f64; fp.rows.len()];

    // Initialize placement: assign each cell to a row with space.
    let mut placed: Vec<PlacedCell> = Vec::with_capacity(fp.components.len());
    for c in &fp.components {
        let default_width = fp.rows[0].step_x;
        let size = cell_sizes
            .get(&c.cell_type)
            .map(|s| s.width)
            .unwrap_or(default_width);

        let row_idx = find_row(&row_widths_used, row_capacity, size, &mut rng)
            .ok_or_else(|| PlacementError::CellDoesNotFit {
                name: c.name.clone(),
                width: size,
            })?;

        let row = &fp.rows[row_idx];
        let x = row.origin_x + row_widths_used[row_idx];
        let y = row.origin_y;
        row_widths_used[row_idx] += size;

        placed.push(PlacedCell {
            name: c.name.clone(),
            cell_type: c.cell_type.clone(),
            width: size,
            x,
            y,
            row_index: row_idx,
        });
    }

    // Simulated annealing.
    let mut accepted = 0u32;
    let mut rejected = 0u32;
    let final_hpwl;

    if let Some(nets_list) = nets {
        if opts.iterations > 0 && placed.len() >= 2 {
            let mut hpwl = total_hpwl(&placed, nets_list);
            let t0 = (hpwl / placed.len().max(1) as f64).max(1.0);
            let mut t = t0;
            let cooling = (1e-3f64).powf(1.0 / opts.iterations as f64);

            for _ in 0..opts.iterations {
                let i = rng.usize_below(placed.len());
                let j = {
                    let mut j = rng.usize_below(placed.len() - 1);
                    if j >= i { j += 1; }
                    j
                };
                // Tentative swap.
                let (ox_i, oy_i, ori) = (placed[i].x, placed[i].y, placed[i].row_index);
                let (ox_j, oy_j, orj) = (placed[j].x, placed[j].y, placed[j].row_index);
                placed[i].x = ox_j; placed[i].y = oy_j; placed[i].row_index = orj;
                placed[j].x = ox_i; placed[j].y = oy_i; placed[j].row_index = ori;

                let new_hpwl = total_hpwl(&placed, nets_list);
                let delta = new_hpwl - hpwl;
                if delta < 0.0 || rng.f64_01() < (-delta / t.max(1e-9)).exp() {
                    accepted += 1;
                    hpwl = new_hpwl;
                } else {
                    // Revert.
                    placed[i].x = ox_i; placed[i].y = oy_i; placed[i].row_index = ori;
                    placed[j].x = ox_j; placed[j].y = oy_j; placed[j].row_index = orj;
                    rejected += 1;
                }
                t *= cooling;
            }
            final_hpwl = hpwl;
        } else {
            final_hpwl = total_hpwl(&placed, nets_list);
        }
    } else {
        final_hpwl = 0.0;
    }

    // Legalization: snap to row coordinates, pack left-to-right.
    if opts.legalize {
        legalize(&mut placed, fp);
    }

    let cells_placed = placed.len();
    let new_components: Vec<Component> = placed
        .into_iter()
        .map(|p| Component {
            name: p.name,
            cell_type: p.cell_type,
            placed: true,
            location_x: Some(p.x),
            location_y: Some(p.y),
            orientation: "N".to_string(),
        })
        .collect();

    let placed_def = Def {
        design: "placed".to_string(),
        version: "5.8".to_string(),
        units_microns: 1000,
        die_area: Some(fp.die.clone()),
        rows: fp.rows.clone(),
        components: new_components,
        pins: fp.pins.clone(),
        nets: vec![],
    };

    let report = PlacementReport {
        final_hpwl,
        cells_placed,
        accepted_swaps: accepted,
        rejected_swaps: rejected,
    };

    Ok((placed_def, report))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn find_row(
    widths_used: &[f64],
    capacity: f64,
    needed: f64,
    rng: &mut Xorshift64,
) -> Option<usize> {
    let candidates: Vec<usize> = widths_used
        .iter()
        .enumerate()
        .filter(|(_, &w)| w + needed <= capacity)
        .map(|(i, _)| i)
        .collect();
    if !candidates.is_empty() {
        return Some(candidates[rng.usize_below(candidates.len())]);
    }
    // Fallback: row with most remaining space.
    let best = widths_used
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())?
        .0;
    if widths_used[best] + needed <= capacity * 1.5 {
        Some(best)
    } else {
        None
    }
}

fn total_hpwl(cells: &[PlacedCell], nets: &[Vec<String>]) -> f64 {
    let by_name: HashMap<&str, &PlacedCell> = cells.iter().map(|c| (c.name.as_str(), c)).collect();
    let mut total = 0.0;
    for net in nets {
        if net.len() < 2 { continue; }
        let mut xs: Vec<f64> = Vec::new();
        let mut ys: Vec<f64> = Vec::new();
        for name in net {
            if let Some(c) = by_name.get(name.as_str()) {
                xs.push(c.x);
                ys.push(c.y);
            }
        }
        if xs.len() < 2 { continue; }
        let x_span = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
            - xs.iter().cloned().fold(f64::INFINITY, f64::min);
        let y_span = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
            - ys.iter().cloned().fold(f64::INFINITY, f64::min);
        total += x_span + y_span;
    }
    total
}

fn legalize(cells: &mut [PlacedCell], fp: &Floorplan) {
    let n_rows = fp.rows.len();
    let mut by_row: Vec<Vec<usize>> = vec![vec![]; n_rows];
    for (i, c) in cells.iter().enumerate() {
        let ri = c.row_index.min(n_rows - 1);
        by_row[ri].push(i);
    }
    for (row_idx, indices) in by_row.iter().enumerate() {
        let row = &fp.rows[row_idx];
        let mut sorted: Vec<usize> = indices.clone();
        sorted.sort_by(|&a, &b| cells[a].x.partial_cmp(&cells[b].x).unwrap());
        let mut cursor = row.origin_x;
        for &idx in &sorted {
            cells[idx].x = cursor;
            cells[idx].y = row.origin_y;
            cursor += cells[idx].width;
        }
    }
}
