//! # Sky130 PDK
//!
//! Metadata and teaching-subset cell list for the SkyWater Sky130 130 nm
//! open-source PDK. No filesystem access required for the teaching profile;
//! the FULL profile validates that a real Sky130 install exists at the given
//! path.
//!
//! ## Feature overview
//!
//! - **Process metadata** — V_DD, gate-oxide thickness, threshold voltages,
//!   mobility-Cox products, metal layer count, cell-row height.
//! - **Teaching cell subset** — ~33 cells: INV, BUF, NAND2/3, NOR2/3,
//!   AND2, OR2, XOR2, MUX2, DFF, latch, conb, tap, decap, fill.
//! - **Layer/datatype map** — GDS layer numbers and datatypes matching the
//!   sky130.layermap reference (li1, met1-5, poly, diff, etc.).
//! - **Loader** — `load_sky130(profile, root)`.
//!
//! ## Example
//!
//! ```rust
//! use sky130_pdk::{load_sky130, PdkProfile};
//!
//! let pdk = load_sky130(PdkProfile::Teaching, None::<&str>).unwrap();
//! assert!(pdk.cells.contains_key("sky130_fd_sc_hd__inv_1"));
//! let meta = &pdk.process;
//! assert_eq!(meta.feature_size_nm, 130);
//! ```

pub mod cells;
pub mod layers;
pub mod pdk;
pub mod process;

pub use cells::{CellInfo, TEACHING_CELLS};
pub use layers::{LayerInfo, LAYER_MAP};
pub use pdk::{load_sky130, Pdk, PdkError, PdkProfile};
pub use process::ProcessMetadata;
