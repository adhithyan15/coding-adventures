use gate_netlist_format::{Direction, Instance, Level, Module, Net, Netlist, NetSlice, Port};
use tech_mapping::{map_to_sky130, TechMapper};

fn generic_adder_nl() -> Netlist {
    let mut nl = Netlist::new("adder4");
    let mut m = Module::new("adder4");
    m.ports.push(Port::new("a", Direction::Input, 4));
    m.ports.push(Port::new("b", Direction::Input, 4));
    m.ports.push(Port::new("sum", Direction::Output, 5));
    m.nets.push(Net::new("_n0", 1));
    m.nets.push(Net::new("_n1", 1));
    m.nets.push(Net::new("_n2", 1));

    let mut xor = Instance::new("xor_0", "XOR2");
    xor.connections.insert("A".into(), NetSlice::single("a", 0));
    xor.connections.insert("B".into(), NetSlice::single("b", 0));
    xor.connections.insert("Y".into(), NetSlice::single("_n0", 0));
    m.instances.push(xor);

    let mut and = Instance::new("and_0", "AND2");
    and.connections.insert("A".into(), NetSlice::single("a", 0));
    and.connections.insert("B".into(), NetSlice::single("b", 0));
    and.connections.insert("Y".into(), NetSlice::single("_n1", 0));
    m.instances.push(and);

    let mut or = Instance::new("or_0", "OR2");
    or.connections.insert("A".into(), NetSlice::single("_n0", 0));
    or.connections.insert("B".into(), NetSlice::single("_n1", 0));
    or.connections.insert("Y".into(), NetSlice::single("_n2", 0));
    m.instances.push(or);

    nl.modules.insert("adder4".into(), m);
    nl
}

#[test]
fn test_map_produces_stdcell_level() {
    let (mapped, _) = map_to_sky130(&generic_adder_nl());
    assert_eq!(mapped.level, Level::Stdcell);
}

#[test]
fn test_xor2_renamed_to_sky130() {
    let (mapped, _) = map_to_sky130(&generic_adder_nl());
    let types: Vec<_> = mapped.modules["adder4"].instances.iter()
        .map(|i| i.cell_type.as_str())
        .collect();
    assert!(types.contains(&"sky130_fd_sc_hd__xor2_1"), "types: {types:?}");
}

#[test]
fn test_and2_renamed() {
    let (mapped, _) = map_to_sky130(&generic_adder_nl());
    let types: Vec<_> = mapped.modules["adder4"].instances.iter()
        .map(|i| i.cell_type.as_str()).collect();
    assert!(types.contains(&"sky130_fd_sc_hd__and2_1"));
}

#[test]
fn test_pin_remap_y_to_x_for_and2() {
    let (mapped, _) = map_to_sky130(&generic_adder_nl());
    let and_inst = mapped.modules["adder4"].instances.iter()
        .find(|i| i.cell_type == "sky130_fd_sc_hd__and2_1")
        .unwrap();
    // AND2.Y → sky130.X
    assert!(and_inst.connections.contains_key("X"), "expected X pin, got {:?}", and_inst.connections.keys().collect::<Vec<_>>());
}

#[test]
fn test_pin_remap_y_stays_y_for_nand2() {
    let mut nl = Netlist::new("t");
    let mut m = Module::new("t");
    m.ports.push(Port::new("a", Direction::Input, 1));
    m.ports.push(Port::new("b", Direction::Input, 1));
    m.ports.push(Port::new("y", Direction::Output, 1));
    m.nets.push(Net::new("_n0", 1));
    let mut inst = Instance::new("n0", "NAND2");
    inst.connections.insert("A".into(), NetSlice::single("a", 0));
    inst.connections.insert("B".into(), NetSlice::single("b", 0));
    inst.connections.insert("Y".into(), NetSlice::single("_n0", 0));
    m.instances.push(inst);
    nl.modules.insert("t".into(), m);

    let (mapped, _) = map_to_sky130(&nl);
    let nand = mapped.modules["t"].instances.iter()
        .find(|i| i.cell_type.contains("nand2")).unwrap();
    // NAND2.Y stays Y in Sky130.
    assert!(nand.connections.contains_key("Y"));
}

#[test]
fn test_unmapped_cell_passes_through() {
    let mut nl = generic_adder_nl();
    nl.modules.get_mut("adder4").unwrap().instances.push(
        Instance::new("custom", "MY_CUSTOM_GATE")
    );
    let (mapped, report) = map_to_sky130(&nl);
    assert!(report.unmapped.contains(&"MY_CUSTOM_GATE".to_string()));
    let has_custom = mapped.modules["adder4"].instances.iter()
        .any(|i| i.cell_type == "MY_CUSTOM_GATE");
    assert!(has_custom, "unmapped cell should pass through");
}

#[test]
fn test_inv_inv_cancellation() {
    // Two back-to-back INVs should cancel.
    let mut nl = Netlist::new("t");
    let mut m = Module::new("t");
    m.ports.push(Port::new("a", Direction::Input, 1));
    m.ports.push(Port::new("y", Direction::Output, 1));
    m.nets.push(Net::new("_mid", 1));

    let mut inv1 = Instance::new("inv1", "NOT");
    inv1.connections.insert("A".into(), NetSlice::single("a", 0));
    inv1.connections.insert("Y".into(), NetSlice::single("_mid", 0));
    m.instances.push(inv1);

    let mut inv2 = Instance::new("inv2", "NOT");
    inv2.connections.insert("A".into(), NetSlice::single("_mid", 0));
    inv2.connections.insert("Y".into(), NetSlice::single("y", 0));
    m.instances.push(inv2);

    nl.modules.insert("t".into(), m);
    let (mapped, report) = map_to_sky130(&nl);
    assert_eq!(report.bubbles_canceled, 1);
    assert_eq!(mapped.modules["t"].instances.len(), 0);
}

#[test]
fn test_report_cells_before_after() {
    let (_, report) = map_to_sky130(&generic_adder_nl());
    assert_eq!(report.cells_before, 3);
    assert_eq!(report.cells_after, 3);
}

#[test]
fn test_all_sky130_cells_in_default_map() {
    let mapper = TechMapper::new();
    for generic in &["BUF","NOT","AND2","OR2","XOR2","NAND2","NOR2","MUX2","DFF"] {
        assert!(mapper.cell_map.contains_key(generic), "missing: {generic}");
    }
}
