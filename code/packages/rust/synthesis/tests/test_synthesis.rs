use gate_netlist_format::Level;
use hdl_ir::{ContAssign, Direction as HirDir, Expr, Hir, Module, Port, Ty};
use synthesis::synthesize;

fn make_hir(top: &str, cont_assigns: Vec<ContAssign>, ports: Vec<Port>) -> Hir {
    let mut hir = Hir::new(top);
    let mut m = Module::new(top);
    m.ports = ports;
    m.cont_assigns = cont_assigns;
    hir.modules.insert(top.to_string(), m);
    hir
}

fn adder4_hir() -> Hir {
    make_hir(
        "adder4",
        vec![ContAssign {
            target: Expr::port_ref("sum"),
            rhs: Expr::binary("+", Expr::port_ref("a"), Expr::port_ref("b")),
            provenance: None,
        }],
        vec![
            Port { name: "a".into(), ty: Ty::vec(4), direction: HirDir::In, provenance: None },
            Port { name: "b".into(), ty: Ty::vec(4), direction: HirDir::In, provenance: None },
            Port { name: "sum".into(), ty: Ty::vec(5), direction: HirDir::Out, provenance: None },
        ],
    )
}

#[test]
fn test_synthesize_produces_hnl() {
    let hir = adder4_hir();
    let nl = synthesize(&hir);
    assert_eq!(nl.top, "adder4");
    assert_eq!(nl.level, Level::Generic);
    assert!(nl.modules.contains_key("adder4"));
}

#[test]
fn test_adder4_has_ports() {
    let nl = synthesize(&adder4_hir());
    let m = &nl.modules["adder4"];
    assert_eq!(m.ports.len(), 3);
    let port_names: Vec<_> = m.ports.iter().map(|p| p.name.as_str()).collect();
    assert!(port_names.contains(&"a"));
    assert!(port_names.contains(&"sum"));
}

#[test]
fn test_adder4_generates_xor_and_or_gates() {
    let nl = synthesize(&adder4_hir());
    let m = &nl.modules["adder4"];
    let types: Vec<_> = m.instances.iter().map(|i| i.cell_type.as_str()).collect();
    assert!(types.contains(&"XOR2"), "should have XOR2 gates");
    assert!(types.contains(&"AND2"), "should have AND2 gates");
    assert!(types.contains(&"OR2"),  "should have OR2 gates");
}

#[test]
fn test_adder4_gate_count() {
    // 4-bit ripple-carry: 4 full adders, each = 2 XOR + 2 AND + 1 OR + buffers
    let nl = synthesize(&adder4_hir());
    let m = &nl.modules["adder4"];
    assert!(m.instances.len() >= 20, "expected ~20+ gates, got {}", m.instances.len());
}

#[test]
fn test_bitwise_and_synth() {
    let hir = make_hir(
        "andmod",
        vec![ContAssign {
            target: Expr::port_ref("y"),
            rhs: Expr::binary("AND", Expr::port_ref("a"), Expr::port_ref("b")),
            provenance: None,
        }],
        vec![
            Port { name: "a".into(), ty: Ty::Bit, direction: HirDir::In, provenance: None },
            Port { name: "b".into(), ty: Ty::Bit, direction: HirDir::In, provenance: None },
            Port { name: "y".into(), ty: Ty::Bit, direction: HirDir::Out, provenance: None },
        ],
    );
    let nl = synthesize(&hir);
    let types: Vec<_> = nl.modules["andmod"].instances.iter().map(|i| i.cell_type.as_str()).collect();
    assert!(types.contains(&"AND2"));
}

#[test]
fn test_not_synth() {
    let hir = make_hir(
        "notmod",
        vec![ContAssign {
            target: Expr::port_ref("y"),
            rhs: Expr::unary("NOT", Expr::port_ref("a")),
            provenance: None,
        }],
        vec![
            Port { name: "a".into(), ty: Ty::Bit, direction: HirDir::In, provenance: None },
            Port { name: "y".into(), ty: Ty::Bit, direction: HirDir::Out, provenance: None },
        ],
    );
    let nl = synthesize(&hir);
    let types: Vec<_> = nl.modules["notmod"].instances.iter().map(|i| i.cell_type.as_str()).collect();
    assert!(types.contains(&"NOT"));
}

#[test]
fn test_empty_module_synthesizes() {
    let hir = Hir::new("empty");
    // No modules — result is empty.
    let nl = synthesize(&hir);
    assert!(nl.modules.is_empty());
}

#[test]
fn test_json_round_trip_after_synthesis() {
    let nl = synthesize(&adder4_hir());
    let json = nl.to_json().unwrap();
    let restored = gate_netlist_format::Netlist::from_json(&json).unwrap();
    assert_eq!(restored.top, "adder4");
    assert!(!restored.modules["adder4"].instances.is_empty());
}
