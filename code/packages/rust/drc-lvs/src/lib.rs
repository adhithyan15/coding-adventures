//! # DRC + LVS
//!
//! **DRC (Design Rule Check)** — geometric rules that a physical layout
//! must satisfy for reliable manufacturing. v0.1.0 implements:
//! - `min_width` — every rectangle must be at least W µm wide and tall.
//! - `min_spacing` — any two rectangles on the same layer must be at least S µm apart.
//! - `min_area` — every rectangle must have area ≥ A µm².
//!
//! **LVS (Layout vs Schematic)** — compares two flat netlists (layout
//! extracted from GDS vs the synthesized schematic) via bag-of-cell-signatures.
//! Signatures encode (cell_type, sorted pin→net-equivalence-class) tuples.
//!
//! ## Usage (DRC)
//!
//! ```rust
//! use drc_lvs::{run_drc, DrcRect, Rule, RuleKind};
//!
//! let rects = vec![DrcRect { layer: "met1".into(), x1: 0.0, y1: 0.0, x2: 0.10, y2: 0.28 }];
//! let rules = vec![Rule { name: "met1.W".into(), layer: "met1".into(),
//!                         kind: RuleKind::MinWidth, value: 0.14, severity: "error".into() }];
//! let report = run_drc(&rects, &rules);
//! assert!(!report.clean()); // 0.10 < 0.14
//! ```
//!
//! ## Usage (LVS)
//!
//! ```rust
//! use drc_lvs::{lvs, LvsCell, LvsNetlist};
//!
//! let cell = LvsCell { name: "inv_0".into(), cell_type: "inv_1".into(),
//!                      pins: vec![("A".into(), "net_a".into()), ("Y".into(), "net_y".into())] };
//! let nl = LvsNetlist { cells: vec![cell.clone()] };
//! let report = lvs(&nl, &nl); // identical netlists match
//! assert!(report.matched);
//! ```

pub mod drc;
pub mod lvs;

pub use drc::{run_drc, DrcRect, DrcReport, Rule, RuleKind, Violation};
pub use lvs::{lvs, LvsCell, LvsNetlist, LvsReport};
