//! Sky130 GDS layer/datatype map.
//!
//! Maps `"name.purpose"` strings (e.g., `"met1.drawing"`) to GDS layer
//! number and datatype. Source: sky130_fd_pr/cells/sky130.layermap from the
//! open Sky130 reference.
//!
//! These are used by the GDSII writer to encode each physical layer correctly.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

/// A single GDS layer entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerInfo {
    /// Logical layer name (e.g., "met1", "poly").
    pub name: String,
    /// GDS layer number.
    pub layer_number: u32,
    /// GDS datatype (20 = drawing, 16 = pin, 44 = contact/via).
    pub datatype: u32,
    /// Semantic purpose: "drawing", "pin", "label", etc.
    pub purpose: String,
}

impl LayerInfo {
    fn new(name: &str, layer: u32, dt: u32, purpose: &str) -> Self {
        Self {
            name: name.to_string(),
            layer_number: layer,
            datatype: dt,
            purpose: purpose.to_string(),
        }
    }
}

/// The full Sky130 GDS layer/datatype map (simplified teaching subset).
///
/// Keys are `"<layer>.<purpose>"` (e.g., `"met1.drawing"`, `"li1.pin"`).
pub static LAYER_MAP: LazyLock<HashMap<&'static str, LayerInfo>> = LazyLock::new(|| {
    vec![
        ("nwell.drawing",  LayerInfo::new("nwell",  64, 20, "drawing")),
        ("pwell.drawing",  LayerInfo::new("pwell",  64, 16, "drawing")),
        ("diff.drawing",   LayerInfo::new("diff",   65, 20, "drawing")),
        ("tap.drawing",    LayerInfo::new("tap",    65, 44, "drawing")),
        ("poly.drawing",   LayerInfo::new("poly",   66, 20, "drawing")),
        ("licon1.drawing", LayerInfo::new("licon1", 66, 44, "drawing")),
        ("li1.drawing",    LayerInfo::new("li1",    67, 20, "drawing")),
        ("li1.pin",        LayerInfo::new("li1",    67, 16, "pin")),
        ("mcon.drawing",   LayerInfo::new("mcon",   67, 44, "drawing")),
        ("met1.drawing",   LayerInfo::new("met1",   68, 20, "drawing")),
        ("met1.pin",       LayerInfo::new("met1",   68, 16, "pin")),
        ("via.drawing",    LayerInfo::new("via",    68, 44, "drawing")),
        ("met2.drawing",   LayerInfo::new("met2",   69, 20, "drawing")),
        ("met2.pin",       LayerInfo::new("met2",   69, 16, "pin")),
        ("via2.drawing",   LayerInfo::new("via2",   69, 44, "drawing")),
        ("met3.drawing",   LayerInfo::new("met3",   70, 20, "drawing")),
        ("met3.pin",       LayerInfo::new("met3",   70, 16, "pin")),
        ("via3.drawing",   LayerInfo::new("via3",   70, 44, "drawing")),
        ("met4.drawing",   LayerInfo::new("met4",   71, 20, "drawing")),
        ("met4.pin",       LayerInfo::new("met4",   71, 16, "pin")),
        ("via4.drawing",   LayerInfo::new("via4",   71, 44, "drawing")),
        ("met5.drawing",   LayerInfo::new("met5",   72, 20, "drawing")),
        ("met5.pin",       LayerInfo::new("met5",   72, 16, "pin")),
    ]
    .into_iter()
    .collect()
});
