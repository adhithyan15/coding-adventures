//! Integration tests for fpga-place-route-bridge.

use gate_netlist_format::{Direction, Instance, Module, Net, Netlist, NetSlice, Port};
use fpga_place_route_bridge::{
    truth_table, truth_table_types, FpgaBridgeOptions, hnl_to_fpga_json,
};

// ---------------------------------------------------------------------------
// Helpers: tiny netlists
// ---------------------------------------------------------------------------

fn make_inverter_netlist() -> Netlist {
    let m = Module {
        ports: vec![
            Port::new("a", Direction::Input,  1),
            Port::new("y", Direction::Output, 1),
        ],
        instances: vec![
            Instance::new("u_inv", "NOT")
                .connect("A", NetSlice::single("a", 0))
                .connect("Y", NetSlice::single("y", 0)),
        ],
        ..Default::default()
    };
    let mut netlist = Netlist::new("inv_top");
    netlist.modules.insert("inv_top".into(), m);
    netlist
}

fn make_and2_netlist() -> Netlist {
    let m = Module {
        ports: vec![
            Port::new("a", Direction::Input,  1),
            Port::new("b", Direction::Input,  1),
            Port::new("y", Direction::Output, 1),
        ],
        instances: vec![
            Instance::new("u", "AND2")
                .connect("A", NetSlice::single("a", 0))
                .connect("B", NetSlice::single("b", 0))
                .connect("Y", NetSlice::single("y", 0)),
        ],
        ..Default::default()
    };
    let mut netlist = Netlist::new("and_top");
    netlist.modules.insert("and_top".into(), m);
    netlist
}

// ---------------------------------------------------------------------------
// Truth tables
// ---------------------------------------------------------------------------

#[test]
fn test_truth_tables_present() {
    let expected = [
        "BUF", "NOT",
        "AND2", "OR2", "NAND2", "NOR2", "XOR2", "XNOR2",
        "AND3", "OR3", "NAND3", "NOR3", "XOR3",
        "AND4", "OR4", "NAND4", "NOR4",
        "MUX2", "CONST_0", "CONST_1",
    ];
    let types: std::collections::HashSet<&str> = truth_table_types().iter().copied().collect();
    for &name in &expected {
        assert!(types.contains(name), "missing truth table for {name}");
    }
}

#[test]
fn test_and2_truth_table_correct() {
    let (pins, table) = truth_table("AND2").unwrap();
    assert_eq!(pins, &["A", "B"]);
    assert_eq!(table, &[0u8, 0, 0, 1]);
}

#[test]
fn test_xor2_truth_table_correct() {
    let (_pins, table) = truth_table("XOR2").unwrap();
    assert_eq!(table, &[0u8, 1, 1, 0]);
}

#[test]
fn test_const_0_table() {
    let (pins, table) = truth_table("CONST_0").unwrap();
    assert_eq!(pins.len(), 0);
    assert_eq!(table, &[0u8]);
}

#[test]
fn test_const_1_table() {
    let (_pins, table) = truth_table("CONST_1").unwrap();
    assert_eq!(table, &[1u8]);
}

#[test]
fn test_unknown_cell_returns_none() {
    assert!(truth_table("MYSTERY_GATE").is_none());
}

// ---------------------------------------------------------------------------
// hnl_to_fpga_json
// ---------------------------------------------------------------------------

#[test]
fn test_inverter_packs_one_clb() {
    let nl = make_inverter_netlist();
    let (cfg, report) = hnl_to_fpga_json(&nl, None);
    assert_eq!(report.cells_packed, 1);
    assert_eq!(cfg["clbs"].as_object().unwrap().len(), 1);
}

#[test]
fn test_and2_packs_one_clb_with_16_entry_truth_table() {
    let nl = make_and2_netlist();
    let (cfg, report) = hnl_to_fpga_json(&nl, None);
    assert_eq!(report.cells_packed, 1);
    let tt = &cfg["clbs"]["clb_0_0"]["lut_a"]["truth_table"];
    assert_eq!(tt.as_array().unwrap().len(), 16,
        "4-input LUT must have 16 entries");
}

#[test]
fn test_unmapped_cells_reported() {
    let m = Module {
        instances: vec![
            Instance::new("u", "MYSTERY"),
        ],
        ..Default::default()
    };
    let mut nl = Netlist::new("x");
    nl.modules.insert("x".into(), m);
    let (_, report) = hnl_to_fpga_json(&nl, None);
    assert!(report.cells_unmapped.contains(&"MYSTERY".to_string()));
    assert_eq!(report.cells_packed, 0);
}

#[test]
fn test_io_pins_emitted() {
    let nl = make_and2_netlist();
    let (cfg, _) = hnl_to_fpga_json(&nl, None);
    let io = cfg["io"].as_object().unwrap();
    let names: std::collections::HashSet<String> = io.values()
        .filter_map(|v| v["name"].as_str().map(|s| s.to_string()))
        .collect();
    assert!(names.contains("a"), "io must contain input 'a'");
    assert!(names.contains("b"), "io must contain input 'b'");
    assert!(names.contains("y"), "io must contain output 'y'");
}

#[test]
fn test_routes_emitted() {
    let nl = make_and2_netlist();
    let (cfg, report) = hnl_to_fpga_json(&nl, None);
    assert!(report.routes_emitted >= 2, "at least A and B inputs must be routed");
    let routing = cfg["routing"].as_array().unwrap();
    // At least one route should have a "from" containing "net_a" or "net_b"
    let sources: Vec<&str> = routing.iter()
        .filter_map(|r| r["from"].as_str())
        .collect();
    assert!(sources.iter().any(|s| s.contains("net_a") || s.contains("net_b")),
        "expected a route from net_a or net_b, got: {sources:?}");
}

#[test]
fn test_truth_table_expansion_to_16() {
    let nl = make_inverter_netlist(); // 1-input cell
    let (cfg, _) = hnl_to_fpga_json(&nl, Some(&FpgaBridgeOptions { lut_inputs: 4, ..Default::default() }));
    let tt = cfg["clbs"]["clb_0_0"]["lut_a"]["truth_table"].as_array().unwrap();
    assert_eq!(tt.len(), 16, "1-input cell expanded to 4-input LUT must have 16 entries");
}

#[test]
fn test_options_passed_through() {
    let nl = make_and2_netlist();
    let opts = FpgaBridgeOptions { rows: 8, cols: 8, lut_inputs: 6, seed: 42 };
    let (cfg, _) = hnl_to_fpga_json(&nl, Some(&opts));
    assert_eq!(cfg["device"]["rows"].as_u64().unwrap(), 8);
    assert_eq!(cfg["device"]["cols"].as_u64().unwrap(), 8);
    assert_eq!(cfg["device"]["lut_inputs"].as_u64().unwrap(), 6);
}

#[test]
fn test_multiple_cells_get_different_clbs() {
    let m = Module {
        ports: vec![
            Port::new("a", Direction::Input,  1),
            Port::new("b", Direction::Input,  1),
            Port::new("y", Direction::Output, 1),
        ],
        nets: vec![Net::new("mid", 1)],
        instances: vec![
            Instance::new("u1", "AND2")
                .connect("A", NetSlice::single("a", 0))
                .connect("B", NetSlice::single("b", 0))
                .connect("Y", NetSlice::single("mid", 0)),
            Instance::new("u2", "NOT")
                .connect("A", NetSlice::single("mid", 0))
                .connect("Y", NetSlice::single("y", 0)),
        ],
        ..Default::default()
    };
    let mut nl = Netlist::new("x");
    nl.modules.insert("x".into(), m);
    let (cfg, report) = hnl_to_fpga_json(&nl, None);
    assert_eq!(report.cells_packed, 2);
    assert_eq!(cfg["clbs"].as_object().unwrap().len(), 2);
    assert!(cfg["clbs"]["clb_0_0"].is_object(), "clb_0_0 must be present");
    assert!(cfg["clbs"]["clb_0_1"].is_object(), "clb_0_1 must be present");
}

#[test]
fn test_adder_20_cells_pack() {
    // 8 XOR2 + 8 AND2 + 4 OR2 = 20 cells
    let mut instances = Vec::new();
    for i in 0..8usize {
        instances.push(
            Instance::new(format!("x{i}"), "XOR2")
                .connect("A", NetSlice::single("a", (i % 4) as u32))
                .connect("B", NetSlice::single("b", (i % 4) as u32))
                .connect("Y", NetSlice::single("sum", (i % 4) as u32)),
        );
    }
    for i in 0..8usize {
        instances.push(
            Instance::new(format!("and{i}"), "AND2")
                .connect("A", NetSlice::single("a", (i % 4) as u32))
                .connect("B", NetSlice::single("b", (i % 4) as u32))
                .connect("Y", NetSlice::single("cout", 0)),
        );
    }
    for i in 0..4usize {
        instances.push(
            Instance::new(format!("or{i}"), "OR2")
                .connect("A", NetSlice::single("a", i as u32))
                .connect("B", NetSlice::single("b", i as u32))
                .connect("Y", NetSlice::single("cout", 0)),
        );
    }
    let m = Module {
        ports: vec![
            Port::new("a",    Direction::Input,  4),
            Port::new("b",    Direction::Input,  4),
            Port::new("sum",  Direction::Output, 4),
            Port::new("cout", Direction::Output, 1),
        ],
        instances,
        ..Default::default()
    };
    let mut nl = Netlist::new("adder4");
    nl.modules.insert("adder4".into(), m);
    let opts = FpgaBridgeOptions { rows: 8, cols: 8, ..Default::default() };
    let (cfg, report) = hnl_to_fpga_json(&nl, Some(&opts));
    assert_eq!(report.cells_packed, 20, "all 20 cells must pack");
    assert_eq!(cfg["clbs"].as_object().unwrap().len(), 20);
}
