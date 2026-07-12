//! # ASIC Routing
//!
//! Lee maze routing on a 2-D track grid (single metal layer in v0.1.0).
//!
//! ## Algorithm
//!
//! For each net, BFS (Lee's algorithm) finds a path from the first pin to
//! each subsequent pin, avoiding blocked cells. After routing a segment,
//! the path cells are marked blocked so later nets can't use them.
//!
//! This implements a single-layer router on `met1` (or the configured layer).
//! Multi-layer routing with via insertion is v0.2.0.
//!
//! ## Grid mapping
//!
//! User-space µm coordinates are mapped to integer grid coordinates via:
//!   `grid_x = (x / pitch).round()`, similarly for y.
//!
//! ## Usage
//!
//! ```rust
//! use asic_routing::{route, PinAccess, RouteOptions};
//! use lef_def::Def;
//!
//! let mut placed_def = Def::new("adder4");
//! placed_def.die_area = Some(lef_def::Rect::new(0.0, 0.0, 10.0, 10.0));
//!
//! let pins = vec![
//!     PinAccess { cell_instance: "xor_0".into(), pin_name: "A".into(), x: 1, y: 2 },
//!     PinAccess { cell_instance: "and_0".into(), pin_name: "A".into(), x: 3, y: 2 },
//! ];
//! let nets = vec![("n_a".to_string(), pins)];
//! let (routed_def, report) = route(&placed_def, &nets, None).unwrap();
//! assert_eq!(report.nets_routed, 1);
//! ```

use std::collections::{HashMap, VecDeque};

use lef_def::{Def, Net, Segment};

/// Grid-space location of one cell pin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinAccess {
    pub cell_instance: String,
    pub pin_name: String,
    pub x: i32,
    pub y: i32,
}

/// Router parameters.
pub struct RouteOptions {
    /// Grid step size in µm. One grid cell = `pitch` µm.
    pub pitch: f64,
    /// Metal layer to route on. Default "met1".
    pub layer: String,
    /// Maximum BFS iterations per net-pin pair.
    pub max_iters_per_net: usize,
}

impl Default for RouteOptions {
    fn default() -> Self {
        Self {
            pitch: 0.34,
            layer: "met1".to_string(),
            max_iters_per_net: 100_000,
        }
    }
}

/// Routing results.
#[derive(Debug, Default)]
pub struct RouteReport {
    pub nets_routed: usize,
    pub nets_failed: usize,
    pub failed_nets: Vec<String>,
    pub total_wire_length: f64,
}

/// Routing error.
#[derive(Debug)]
pub enum RouteError {
    NoDieArea,
}

impl std::fmt::Display for RouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "placed Def has no die_area; can't size routing grid")
    }
}

impl std::error::Error for RouteError {}

/// Route all nets.
///
/// `nets`: list of `(net_name, Vec<PinAccess>)`. Each net is routed in a
/// star topology (first pin → every subsequent pin).
pub fn route(
    placed: &Def,
    nets: &[(String, Vec<PinAccess>)],
    options: Option<RouteOptions>,
) -> Result<(Def, RouteReport), RouteError> {
    let die = placed.die_area.as_ref().ok_or(RouteError::NoDieArea)?;
    let opts = options.unwrap_or_default();

    let width_grid = ((die.x2 - die.x1) / opts.pitch).ceil() as usize + 1;
    let height_grid = ((die.y2 - die.y1) / opts.pitch).ceil() as usize + 1;

    // Single-layer blocked map: blocked[x][y] = true if occupied.
    let mut blocked: Vec<Vec<bool>> = vec![vec![false; height_grid]; width_grid];
    mark_components_blocked(placed, opts.pitch, &mut blocked);

    let mut new_nets: Vec<Net> = Vec::new();
    let mut report = RouteReport::default();

    for (net_name, pins) in nets {
        if pins.len() < 2 {
            new_nets.push(Net::new(net_name));
            continue;
        }

        let mut net_segments: Vec<Segment> = Vec::new();
        let source = &pins[0];
        let mut net_ok = true;

        for sink in &pins[1..] {
            match lee_maze_route(&blocked, source, sink, opts.max_iters_per_net, width_grid, height_grid) {
                Some(path) => {
                    let seg = path_to_segment(&path, &opts.layer, opts.pitch);
                    report.total_wire_length += segment_length(&path) as f64 * opts.pitch;
                    // Mark routed cells as blocked.
                    for &(x, y) in &path {
                        if x < width_grid as i32 && y < height_grid as i32 && x >= 0 && y >= 0 {
                            blocked[x as usize][y as usize] = true;
                        }
                    }
                    net_segments.push(seg);
                }
                None => {
                    report.nets_failed += 1;
                    report.failed_nets.push(net_name.clone());
                    net_ok = false;
                    break;
                }
            }
        }

        if net_ok { report.nets_routed += 1; }

        let connections: Vec<(String, String)> = pins
            .iter()
            .map(|p| (p.cell_instance.clone(), p.pin_name.clone()))
            .collect();

        new_nets.push(Net {
            name: net_name.clone(),
            connections,
            routed_segments: net_segments,
        });
    }

    let routed_def = Def {
        design: placed.design.clone(),
        version: placed.version.clone(),
        units_microns: placed.units_microns,
        die_area: placed.die_area.clone(),
        rows: placed.rows.clone(),
        components: placed.components.clone(),
        pins: placed.pins.clone(),
        nets: new_nets,
    };

    Ok((routed_def, report))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn mark_components_blocked(def: &Def, pitch: f64, blocked: &mut [Vec<bool>]) {
    let width = blocked.len();
    let height = if width > 0 { blocked[0].len() } else { 0 };
    for c in &def.components {
        if let (Some(lx), Some(ly)) = (c.location_x, c.location_y) {
            let gx = (lx / pitch).round() as usize;
            let gy = (ly / pitch).round() as usize;
            if gx < width && gy < height {
                blocked[gx][gy] = true;
            }
        }
    }
}

fn lee_maze_route(
    blocked: &[Vec<bool>],
    source: &PinAccess,
    sink: &PinAccess,
    max_iters: usize,
    width: usize,
    height: usize,
) -> Option<Vec<(i32, i32)>> {
    let sx = source.x;
    let sy = source.y;
    let tx = sink.x;
    let ty = sink.y;

    if !(0..width as i32).contains(&sx) || !(0..height as i32).contains(&sy) { return None; }
    if !(0..width as i32).contains(&tx) || !(0..height as i32).contains(&ty) { return None; }
    if sx == tx && sy == ty { return Some(vec![(sx, sy)]); }

    let mut parent: HashMap<(i32,i32),(i32,i32)> = HashMap::new();
    parent.insert((sx, sy), (-1, -1));
    let mut queue: VecDeque<(i32,i32)> = VecDeque::new();
    queue.push_back((sx, sy));
    let mut iters = 0;

    while let Some((cx, cy)) = queue.pop_front() {
        iters += 1;
        if iters > max_iters { break; }
        if cx == tx && cy == ty {
            return Some(reconstruct_path(&parent, (tx, ty)));
        }
        for (dx, dy) in &[(1,0),(-1,0),(0,1),(0,-1)] {
            let nx = cx + dx;
            let ny = cy + dy;
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 { continue; }
            if parent.contains_key(&(nx, ny)) { continue; }
            let is_target = nx == tx && ny == ty;
            if !is_target && blocked[nx as usize][ny as usize] { continue; }
            parent.insert((nx, ny), (cx, cy));
            queue.push_back((nx, ny));
        }
    }
    None
}

fn reconstruct_path(parent: &HashMap<(i32,i32),(i32,i32)>, target: (i32,i32)) -> Vec<(i32,i32)> {
    let mut path = vec![target];
    let mut cur = target;
    loop {
        let p = parent[&cur];
        if p == (-1, -1) { break; }
        path.push(p);
        cur = p;
    }
    path.reverse();
    path
}

fn path_to_segment(path: &[(i32,i32)], layer: &str, pitch: f64) -> Segment {
    Segment {
        layer: layer.to_string(),
        points: path.iter().map(|&(x, y)| (x as f64 * pitch, y as f64 * pitch)).collect(),
    }
}

fn segment_length(path: &[(i32,i32)]) -> usize {
    path.windows(2).map(|w| {
        let (x0,y0) = w[0];
        let (x1,y1) = w[1];
        ((x1-x0).abs() + (y1-y0).abs()) as usize
    }).sum()
}

/// Convert a user-space µm coordinate to a grid coordinate.
pub fn to_grid(coord: f64, pitch: f64) -> i32 {
    (coord / pitch).round() as i32
}

/// Build a PinAccess from user-space coordinates.
pub fn pin_at(cell: impl Into<String>, pin: impl Into<String>, x: f64, y: f64, pitch: f64) -> PinAccess {
    PinAccess {
        cell_instance: cell.into(),
        pin_name: pin.into(),
        x: to_grid(x, pitch),
        y: to_grid(y, pitch),
    }
}
