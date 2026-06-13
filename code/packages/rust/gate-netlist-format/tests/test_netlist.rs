use gate_netlist_format::{Direction, Instance, Level, Module, Net, Netlist, NetSlice, Port};

fn adder4_hnl() -> Netlist {
    let mut nl = Netlist::new("adder4");
    let mut m = Module::new("adder4");
    m.ports.push(Port::new("a", Direction::Input, 4));
    m.ports.push(Port::new("b", Direction::Input, 4));
    m.ports.push(Port::new("sum", Direction::Output, 5));
    for i in 0..5u32 {
        m.nets.push(Net::new(format!("_n{i}"), 1));
    }
    // XOR gate for bit 0
    let mut xor = Instance::new("xor_0", "XOR2");
    xor.connections.insert("A".into(), NetSlice::single("a", 0));
    xor.connections.insert("B".into(), NetSlice::single("b", 0));
    xor.connections.insert("Y".into(), NetSlice::single("_n0", 0));
    m.instances.push(xor);
    nl.modules.insert("adder4".into(), m);
    nl
}

#[test]
fn test_hnl_construction() {
    let nl = adder4_hnl();
    assert_eq!(nl.top, "adder4");
    assert_eq!(nl.modules["adder4"].ports.len(), 3);
    assert_eq!(nl.modules["adder4"].instances.len(), 1);
}

#[test]
fn test_json_round_trip() {
    let orig = adder4_hnl();
    let json = orig.to_json().unwrap();
    let restored = Netlist::from_json(&json).unwrap();
    assert_eq!(restored.top, "adder4");
    assert_eq!(restored.modules["adder4"].ports.len(), 3);
}

#[test]
fn test_json_rejects_wrong_format() {
    let bad = r#"{"format":"XXX","version":"0.1.0","top":"x","modules":{}}"#;
    assert!(Netlist::from_json(bad).is_err());
}

#[test]
fn test_level_default_is_generic() {
    let nl = adder4_hnl();
    assert_eq!(nl.level, Level::Generic);
}

#[test]
fn test_validation_passes() {
    let nl = adder4_hnl();
    let report = nl.validate();
    assert!(report.ok(), "errors: {:?}", report.errors);
}

#[test]
fn test_r1_missing_top() {
    let mut nl = adder4_hnl();
    nl.top = "ghost".into();
    assert!(!nl.validate().ok());
}

#[test]
fn test_r2_unknown_cell_type() {
    let mut nl = adder4_hnl();
    nl.modules.get_mut("adder4").unwrap().instances.push(
        Instance::new("bad", "GHOST_CELL")
    );
    let report = nl.validate();
    assert!(report.errors.iter().any(|e| e.contains("R2")));
}

#[test]
fn test_r3_missing_input_pin() {
    let mut nl = adder4_hnl();
    // AND2 with only A connected — B missing.
    let mut inst = Instance::new("and_0", "AND2");
    inst.connections.insert("A".into(), NetSlice::single("a", 0));
    inst.connections.insert("Y".into(), NetSlice::single("_n0", 0));
    nl.modules.get_mut("adder4").unwrap().instances.push(inst);
    let report = nl.validate();
    assert!(report.errors.iter().any(|e| e.contains("R3")));
}

#[test]
fn test_r4_unknown_pin() {
    let mut nl = adder4_hnl();
    let mut inst = Instance::new("buf_0", "BUF");
    inst.connections.insert("A".into(), NetSlice::single("a", 0));
    inst.connections.insert("Y".into(), NetSlice::single("_n0", 0));
    inst.connections.insert("GHOST_PIN".into(), NetSlice::single("_n0", 0));
    nl.modules.get_mut("adder4").unwrap().instances.push(inst);
    let report = nl.validate();
    assert!(report.errors.iter().any(|e| e.contains("R4")));
}

#[test]
fn test_r11_self_instantiation() {
    let mut nl = adder4_hnl();
    let mut inst = Instance::new("self", "adder4");
    inst.connections.insert("a".into(), NetSlice::single("a", 0));
    inst.connections.insert("b".into(), NetSlice::single("b", 0));
    inst.connections.insert("sum".into(), NetSlice::single("sum", 0));
    nl.modules.get_mut("adder4").unwrap().instances.push(inst);
    let report = nl.validate();
    assert!(report.errors.iter().any(|e| e.contains("R11")));
}

#[test]
fn test_stats() {
    let nl = adder4_hnl();
    let stats = nl.stats();
    assert_eq!(stats.total_cells, 1);
    assert_eq!(stats.total_nets, 5);
    assert_eq!(stats.cell_counts["XOR2"], 1);
}

#[test]
fn test_net_slice_width() {
    let s = NetSlice::range("x", 3, 0);
    assert_eq!(s.width(), 4);
    assert_eq!(s.bits, vec![3, 2, 1, 0]);
}

#[test]
fn test_builtin_cells_present() {
    use gate_netlist_format::BUILTIN_CELL_TYPES;
    for name in &["BUF", "NOT", "AND2", "OR2", "XOR2", "NAND2", "NOR2", "DFF", "MUX2"] {
        assert!(BUILTIN_CELL_TYPES.contains_key(name), "missing: {name}");
    }
}
