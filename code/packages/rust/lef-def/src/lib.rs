//! # LEF/DEF Data Model and Writers
//!
//! LEF (Library Exchange Format) and DEF (Design Exchange Format) are the
//! standard industry formats for physical design interchange. LEF describes
//! technology and cell geometries; DEF describes an instance of a design
//! (die area, rows, placed components, nets, IO pins).
//!
//! ## Pipeline position
//!
//! ```text
//! HNL[STDCELL] → asic-floorplan → Def[unplaced]
//!                                → asic-placement → Def[placed]
//!                                                 → asic-routing → Def[routed]
//!                                                                → gdsii-writer → .gds
//! ```
//!
//! ## Crate structure
//!
//! - [`models`] — all data types (shared by floorplan, placement, routing, GDS)
//! - [`writer`] — text writers producing DEF 5.8 + LEF 5.8 strings

pub mod models;
pub mod writer;

pub use models::{
    CellLef, Component, Def, DefPin, Direction, LayerDef, Net, PinDef, PinPort,
    Rect, Row, Segment, SiteDef, TechLef, Use, ViaDef, ViaLayer,
};
pub use writer::{write_cells_lef_str, write_def_str, write_tech_lef_str};
