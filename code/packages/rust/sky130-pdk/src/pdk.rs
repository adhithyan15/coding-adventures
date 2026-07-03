//! PDK loader.
//!
//! `load_sky130` returns a `Pdk` struct populated from the chosen profile:
//!
//! - **Teaching** (default) — in-memory only; no filesystem access needed.
//!   Contains all cells from `TEACHING_CELLS` and all layers from `LAYER_MAP`.
//!
//! - **Full** — requires `root` pointing at a real Sky130 install.
//!   v0.1.0 only validates the path; full LEF parsing is v0.2.0.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::cells::{CellInfo, TEACHING_CELLS};
use crate::layers::{LayerInfo, LAYER_MAP};
use crate::process::ProcessMetadata;

/// Which profile to use when loading the PDK.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PdkProfile {
    /// In-memory teaching subset. No real Sky130 install required.
    Teaching,
    /// Full install. Requires `root` pointing at a sky130A directory.
    Full,
}

/// A loaded Sky130 PDK reference.
#[derive(Debug, Clone)]
pub struct Pdk {
    /// Which profile was used to load this PDK.
    pub profile: PdkProfile,
    /// Path to the Sky130 install (None for Teaching).
    pub root: Option<PathBuf>,
    /// Process-level metadata.
    pub process: ProcessMetadata,
    /// Cells available in this PDK (name → info).
    pub cells: HashMap<String, CellInfo>,
    /// GDS layer map ("name.purpose" → LayerInfo).
    pub layers: HashMap<String, LayerInfo>,
}

impl Pdk {
    /// Sorted list of all cell names in this PDK.
    pub fn cell_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.cells.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    /// Look up a cell by name.
    pub fn get_cell(&self, name: &str) -> Option<&CellInfo> {
        self.cells.get(name)
    }

    /// Look up a layer by "name.purpose" key (e.g., "met1.drawing").
    pub fn get_layer(&self, key: &str) -> Option<&LayerInfo> {
        self.layers.get(key)
    }
}

/// Errors from `load_sky130`.
#[derive(Debug)]
pub enum PdkError {
    /// profile=Full was requested but no root was given.
    MissingRoot,
    /// The Sky130 install root does not exist on the filesystem.
    InstallNotFound(PathBuf),
}

impl std::fmt::Display for PdkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PdkError::MissingRoot => write!(f, "PdkProfile::Full requires a root path"),
            PdkError::InstallNotFound(p) => write!(f, "Sky130 install not found: {}", p.display()),
        }
    }
}

impl std::error::Error for PdkError {}

/// Load the Sky130 PDK.
///
/// # Teaching profile
/// ```rust
/// use sky130_pdk::{load_sky130, PdkProfile};
/// let pdk = load_sky130(PdkProfile::Teaching, None::<&str>).unwrap();
/// assert!(pdk.cells.len() > 30);
/// ```
///
/// # Full profile
/// ```rust,no_run
/// use sky130_pdk::{load_sky130, PdkProfile};
/// let pdk = load_sky130(PdkProfile::Full, Some("/path/to/sky130A")).unwrap();
/// ```
pub fn load_sky130(
    profile: PdkProfile,
    root: Option<impl Into<PathBuf>>,
) -> Result<Pdk, PdkError> {
    let root_path: Option<PathBuf> = root.map(Into::into);

    if profile == PdkProfile::Full {
        match &root_path {
            None => return Err(PdkError::MissingRoot),
            Some(p) if !p.exists() => return Err(PdkError::InstallNotFound(p.clone())),
            _ => {}
        }
        // v0.2.0: walk root_path/libs.ref/sky130_fd_sc_hd/lef/ and parse cells.
    }

    let cells: HashMap<String, CellInfo> = TEACHING_CELLS
        .iter()
        .map(|(&k, v)| (k.to_string(), v.clone()))
        .collect();

    let layers: HashMap<String, LayerInfo> = LAYER_MAP
        .iter()
        .map(|(&k, v)| (k.to_string(), v.clone()))
        .collect();

    Ok(Pdk {
        profile,
        root: root_path,
        process: ProcessMetadata::default(),
        cells,
        layers,
    })
}
