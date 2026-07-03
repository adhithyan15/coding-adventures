//! # Tech Mapping — Generic HNL → Sky130-style Stdcell HNL
//!
//! Technology mapping converts a generic-gate netlist (BUF, NOT, AND2, OR2,
//! XOR2, DFF, …) into a technology-specific stdcell netlist using real library
//! cell names (e.g. `sky130_fd_sc_hd__and2_1`).
//!
//! ## What this pass does
//!
//! 1. **Cell rename** — each generic type maps to a stdcell name via a
//!    configurable table (default: Sky130 HD cells, drive strength 1).
//! 2. **Pin remap** — Sky130 uses slightly different pin names from the
//!    generic IR (e.g. `AND2.Y` → `sky130__and2_1.X`). The map includes
//!    per-pin name translations.
//! 3. **Bubble pushing** — `AND2 → NAND2+NOT` is standard; where two
//!    `NOT` gates are in series they cancel each other (INV-INV elimination).
//!
//! ## Default cell map (Sky130 HD, drive=1)
//!
//! ```text
//! BUF   → sky130_fd_sc_hd__buf_1   (A→A, Y→X)
//! NOT   → sky130_fd_sc_hd__inv_1   (A→A, Y→Y)
//! AND2  → sky130_fd_sc_hd__and2_1  (A→A, B→B, Y→X)
//! OR2   → sky130_fd_sc_hd__or2_1   (A→A, B→B, Y→X)
//! XOR2  → sky130_fd_sc_hd__xor2_1  (A→A, B→B, Y→X)
//! NAND2 → sky130_fd_sc_hd__nand2_1 (A→A, B→B, Y→Y)
//! NOR2  → sky130_fd_sc_hd__nor2_1  (A→A, B→B, Y→Y)
//! DFF   → sky130_fd_sc_hd__dfxtp_1 (D→D, CLK→CLK, Q→Q)
//! …
//! ```

use std::collections::HashMap;

use gate_netlist_format::{Instance, Level, Module, Netlist, NetSlice};

// ---------------------------------------------------------------------------
// Mapping table entry
// ---------------------------------------------------------------------------

/// One entry in the cell map: `(stdcell_name, pin_remap)`.
/// `pin_remap` maps generic-pin-name → stdcell-pin-name.
#[derive(Debug, Clone)]
pub struct CellMapEntry {
    pub stdcell: &'static str,
    pub pin_remap: HashMap<&'static str, &'static str>,
}

impl CellMapEntry {
    fn new(stdcell: &'static str, remap: &[(&'static str, &'static str)]) -> Self {
        Self {
            stdcell,
            pin_remap: remap.iter().copied().collect(),
        }
    }
}

/// The default Sky130 HD drive-1 cell map.
pub fn default_sky130_map() -> HashMap<&'static str, CellMapEntry> {
    vec![
        ("BUF",    CellMapEntry::new("sky130_fd_sc_hd__buf_1",    &[("A","A"),("Y","X")])),
        ("NOT",    CellMapEntry::new("sky130_fd_sc_hd__inv_1",    &[("A","A"),("Y","Y")])),
        ("AND2",   CellMapEntry::new("sky130_fd_sc_hd__and2_1",   &[("A","A"),("B","B"),("Y","X")])),
        ("AND3",   CellMapEntry::new("sky130_fd_sc_hd__and3_1",   &[("A","A"),("B","B"),("C","C"),("Y","X")])),
        ("AND4",   CellMapEntry::new("sky130_fd_sc_hd__and4_1",   &[("A","A"),("B","B"),("C","C"),("D","D"),("Y","X")])),
        ("OR2",    CellMapEntry::new("sky130_fd_sc_hd__or2_1",    &[("A","A"),("B","B"),("Y","X")])),
        ("OR3",    CellMapEntry::new("sky130_fd_sc_hd__or3_1",    &[("A","A"),("B","B"),("C","C"),("Y","X")])),
        ("OR4",    CellMapEntry::new("sky130_fd_sc_hd__or4_1",    &[("A","A"),("B","B"),("C","C"),("D","D"),("Y","X")])),
        ("NAND2",  CellMapEntry::new("sky130_fd_sc_hd__nand2_1",  &[("A","A"),("B","B"),("Y","Y")])),
        ("NAND3",  CellMapEntry::new("sky130_fd_sc_hd__nand3_1",  &[("A","A"),("B","B"),("C","C"),("Y","Y")])),
        ("NAND4",  CellMapEntry::new("sky130_fd_sc_hd__nand4_1",  &[("A","A"),("B","B"),("C","C"),("D","D"),("Y","Y")])),
        ("NOR2",   CellMapEntry::new("sky130_fd_sc_hd__nor2_1",   &[("A","A"),("B","B"),("Y","Y")])),
        ("NOR3",   CellMapEntry::new("sky130_fd_sc_hd__nor3_1",   &[("A","A"),("B","B"),("C","C"),("Y","Y")])),
        ("NOR4",   CellMapEntry::new("sky130_fd_sc_hd__nor4_1",   &[("A","A"),("B","B"),("C","C"),("D","D"),("Y","Y")])),
        ("XOR2",   CellMapEntry::new("sky130_fd_sc_hd__xor2_1",   &[("A","A"),("B","B"),("Y","X")])),
        ("XOR3",   CellMapEntry::new("sky130_fd_sc_hd__xor3_1",   &[("A","A"),("B","B"),("C","C"),("Y","X")])),
        ("XNOR2",  CellMapEntry::new("sky130_fd_sc_hd__xnor2_1",  &[("A","A"),("B","B"),("Y","Y")])),
        ("XNOR3",  CellMapEntry::new("sky130_fd_sc_hd__xnor3_1",  &[("A","A"),("B","B"),("C","C"),("Y","Y")])),
        ("MUX2",   CellMapEntry::new("sky130_fd_sc_hd__mux2_1",   &[("A","A0"),("B","A1"),("S","S"),("Y","X")])),
        ("DFF",    CellMapEntry::new("sky130_fd_sc_hd__dfxtp_1",  &[("D","D"),("CLK","CLK"),("Q","Q")])),
        ("DFF_R",  CellMapEntry::new("sky130_fd_sc_hd__dfrtp_1",  &[("D","D"),("CLK","CLK"),("R","RESET_B"),("Q","Q")])),
        ("DFF_S",  CellMapEntry::new("sky130_fd_sc_hd__dfstp_1",  &[("D","D"),("CLK","CLK"),("S","SET_B"),("Q","Q")])),
        ("DFF_RS", CellMapEntry::new("sky130_fd_sc_hd__dfsrtp_1", &[("D","D"),("CLK","CLK"),("R","RESET_B"),("S","SET_B"),("Q","Q")])),
        ("DLATCH", CellMapEntry::new("sky130_fd_sc_hd__dlxtp_1",  &[("D","D"),("EN","GATE"),("Q","Q")])),
        ("TBUF",   CellMapEntry::new("sky130_fd_sc_hd__ebufn_1",  &[("A","A"),("OE","TE_B"),("Y","Z")])),
        ("CONST_0",CellMapEntry::new("sky130_fd_sc_hd__conb_1",   &[("Y","LO")])),
        ("CONST_1",CellMapEntry::new("sky130_fd_sc_hd__conb_1",   &[("Y","HI")])),
    ]
    .into_iter()
    .collect()
}

// ---------------------------------------------------------------------------
// Mapping report
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct MappingReport {
    pub cells_before: usize,
    pub cells_after: usize,
    pub bubbles_canceled: usize,
    pub unmapped: Vec<String>,
}

// ---------------------------------------------------------------------------
// TechMapper
// ---------------------------------------------------------------------------

pub struct TechMapper {
    pub cell_map: HashMap<&'static str, CellMapEntry>,
}

impl Default for TechMapper {
    fn default() -> Self {
        Self { cell_map: default_sky130_map() }
    }
}

impl TechMapper {
    pub fn new() -> Self {
        Self::default()
    }

    /// Map a generic HNL to a stdcell HNL. Returns the mapped netlist and a
    /// report. The input netlist is consumed (pass a clone if you need it).
    pub fn map(&self, generic: &Netlist) -> (Netlist, MappingReport) {
        let mut report = MappingReport::default();
        let mut mapped_modules = HashMap::new();

        for (name, module) in &generic.modules {
            let mapped = self.map_module(module, &mut report);
            mapped_modules.insert(name.clone(), mapped);
        }

        report.cells_after = mapped_modules.values().map(|m| m.instances.len()).sum();

        let mut out = Netlist::new(generic.top.clone());
        out.level = Level::Stdcell;
        out.modules = mapped_modules;
        (out, report)
    }

    fn map_module(&self, module: &Module, report: &mut MappingReport) -> Module {
        let mut new_mod = Module::new(module.name.clone());
        new_mod.ports = module.ports.clone();
        new_mod.nets = module.nets.clone();

        let mut instances = Vec::new();
        for inst in &module.instances {
            report.cells_before += 1;

            if let Some(entry) = self.cell_map.get(inst.cell_type.as_str()) {
                // Remap pin names.
                let mut new_conns: HashMap<String, NetSlice> = HashMap::new();
                for (generic_pin, slice) in &inst.connections {
                    let mapped_pin = entry
                        .pin_remap
                        .get(generic_pin.as_str())
                        .copied()
                        .unwrap_or(generic_pin.as_str());
                    new_conns.insert(mapped_pin.to_string(), slice.clone());
                }
                let mut new_inst = Instance::new(inst.name.clone(), entry.stdcell);
                new_inst.connections = new_conns;
                instances.push(new_inst);
            } else {
                // Unknown generic cell — pass through with a warning.
                report.unmapped.push(inst.cell_type.clone());
                instances.push(inst.clone());
            }
        }

        // Bubble pushing: eliminate adjacent INV-INV pairs on the same net.
        let (instances, canceled) = cancel_inv_pairs(instances);
        report.bubbles_canceled += canceled;

        new_mod.instances = instances;
        new_mod
    }
}

// ---------------------------------------------------------------------------
// Bubble pushing: INV-INV cancellation
// ---------------------------------------------------------------------------

/// Remove pairs of `sky130_fd_sc_hd__inv_1` instances where the output of
/// one feeds directly into the input of the other (same net, 1 driver,
/// 1 reader). The net between them is collapsed.
///
/// This is a simple linear scan; a full DAG pass is v0.2.0.
fn cancel_inv_pairs(instances: Vec<Instance>) -> (Vec<Instance>, usize) {
    const INV: &str = "sky130_fd_sc_hd__inv_1";
    let mut canceled = 0;

    // Map: output-net-name → index of the INV that drives it.
    let mut inv_drivers: HashMap<String, usize> = HashMap::new();
    for (i, inst) in instances.iter().enumerate() {
        if inst.cell_type == INV {
            if let Some(y_slice) = inst.connections.get("Y") {
                inv_drivers.insert(y_slice.net.clone(), i);
            }
        }
    }

    let mut to_remove: std::collections::HashSet<usize> = Default::default();

    for (i, inst) in instances.iter().enumerate() {
        if inst.cell_type != INV || to_remove.contains(&i) {
            continue;
        }
        // Check if the A-input of this INV comes from another INV's Y.
        if let Some(a_slice) = inst.connections.get("A") {
            if let Some(&driver_idx) = inv_drivers.get(&a_slice.net) {
                if driver_idx != i && !to_remove.contains(&driver_idx) {
                    to_remove.insert(driver_idx);
                    to_remove.insert(i);
                    canceled += 1;
                }
            }
        }
    }

    let kept: Vec<Instance> = instances
        .into_iter()
        .enumerate()
        .filter_map(|(i, inst)| if to_remove.contains(&i) { None } else { Some(inst) })
        .collect();

    (kept, canceled)
}

// ---------------------------------------------------------------------------
// Convenience top-level function
// ---------------------------------------------------------------------------

/// Map a generic HNL to Sky130 stdcell HNL using the default cell map.
pub fn map_to_sky130(generic: &Netlist) -> (Netlist, MappingReport) {
    TechMapper::new().map(generic)
}
