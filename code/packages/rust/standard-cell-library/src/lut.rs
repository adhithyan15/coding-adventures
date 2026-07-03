//! Non-Linear Delay Model lookup table.
//!
//! A 2-D grid of delay (or transition) values indexed by:
//!   - row axis: input slew (ns)
//!   - column axis: output load capacitance (fF)
//!
//! Delay grows roughly linearly with both slew and load — this is the
//! fundamental insight behind the NLDM model. Bilinear interpolation
//! between grid points gives realistic values for intermediate conditions.
//!
//! ## Bilinear interpolation
//!
//! Given fractional indices (sx, lx) into a 2-D grid:
//!
//! ```text
//! f(sx, lx) = (1-sf)*(1-lf)*v00 + (1-sf)*lf*v01
//!           + sf*(1-lf)*v10      + sf*lf*v11
//! ```
//! where v00-v11 are the four surrounding grid values and sf, lf are the
//! fractional parts of sx and lx.

/// A 2-D NLDM lookup table.
#[derive(Debug, Clone, PartialEq)]
pub struct LookupTable {
    /// Input slew breakpoints in nanoseconds (ascending).
    pub slew_index: Vec<f64>,
    /// Output load breakpoints in femtofarads (ascending).
    pub load_index: Vec<f64>,
    /// `values[slew_i][load_i]` — delay in nanoseconds.
    pub values: Vec<Vec<f64>>,
}

impl LookupTable {
    /// Bilinear interpolation. Out-of-range queries are clamped to the boundary.
    pub fn lookup(&self, slew_ns: f64, load_ff: f64) -> f64 {
        let sx = frac_index(&self.slew_index, slew_ns);
        let lx = frac_index(&self.load_index, load_ff);
        bilinear(&self.values, sx, lx)
    }
}

/// Compute fractional index into a sorted breakpoint array.
///
/// Returns 0.0 if `v` is below the first breakpoint, and `len-1` if above
/// the last. For intermediate values, returns a float in `[i, i+1)` where
/// `i` is the lower bracketing index.
fn frac_index(idx: &[f64], v: f64) -> f64 {
    if v <= idx[0] { return 0.0; }
    let last = idx.len() - 1;
    if v >= idx[last] { return last as f64; }
    for i in 0..last {
        if idx[i] <= v && v < idx[i + 1] {
            let f = (v - idx[i]) / (idx[i + 1] - idx[i]);
            return i as f64 + f;
        }
    }
    last as f64
}

/// Bilinear interpolation from four surrounding grid cells.
fn bilinear(values: &[Vec<f64>], sx: f64, lx: f64) -> f64 {
    let n_rows = values.len();
    let n_cols = values[0].len();
    let sx = sx.clamp(0.0, (n_rows - 1) as f64);
    let lx = lx.clamp(0.0, (n_cols - 1) as f64);
    let s_lo = sx.floor() as usize;
    let l_lo = lx.floor() as usize;
    let s_hi = (s_lo + 1).min(n_rows - 1);
    let l_hi = (l_lo + 1).min(n_cols - 1);
    let sf = sx - s_lo as f64;
    let lf = lx - l_lo as f64;
    let v00 = values[s_lo][l_lo];
    let v01 = values[s_lo][l_hi];
    let v10 = values[s_hi][l_lo];
    let v11 = values[s_hi][l_hi];
    v00 * (1.0 - sf) * (1.0 - lf)
        + v01 * (1.0 - sf) * lf
        + v10 * sf * (1.0 - lf)
        + v11 * sf * lf
}
