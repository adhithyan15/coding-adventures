use hdl_ir::{
    validate, ContAssign, Direction, Expr, Hir, Module, Net, NetKind, Port, Provenance,
    SourceLang, Ty,
};

// ---------------------------------------------------------------------------
// 4-bit adder HIR fixture
// ---------------------------------------------------------------------------

fn adder4_hir() -> Hir {
    // module adder4(input [3:0] a, input [3:0] b, output [4:0] sum);
    //   assign sum = a + b;
    // endmodule
    let mut hir = Hir::new("adder4");
    let mut m = Module::new("adder4");

    m.ports.push(Port {
        name: "a".into(),
        ty: Ty::vec(4),
        direction: Direction::In,
        provenance: None,
    });
    m.ports.push(Port {
        name: "b".into(),
        ty: Ty::vec(4),
        direction: Direction::In,
        provenance: None,
    });
    m.ports.push(Port {
        name: "sum".into(),
        ty: Ty::vec(5),
        direction: Direction::Out,
        provenance: None,
    });

    m.cont_assigns.push(ContAssign {
        target: Expr::port_ref("sum"),
        rhs: Expr::binary("+", Expr::port_ref("a"), Expr::port_ref("b")),
        provenance: None,
    });

    hir.modules.insert("adder4".into(), m);
    hir
}

// ---------------------------------------------------------------------------
// HIR construction
// ---------------------------------------------------------------------------

#[test]
fn test_adder4_has_correct_ports() {
    let hir = adder4_hir();
    let m = &hir.modules["adder4"];
    assert_eq!(m.ports.len(), 3);
    assert_eq!(m.ports[0].name, "a");
    assert_eq!(m.ports[2].direction, Direction::Out);
}

#[test]
fn test_adder4_has_one_cont_assign() {
    let hir = adder4_hir();
    assert_eq!(hir.modules["adder4"].cont_assigns.len(), 1);
}

// ---------------------------------------------------------------------------
// JSON round-trip
// ---------------------------------------------------------------------------

#[test]
fn test_json_round_trip() {
    let orig = adder4_hir();
    let json = orig.to_json().unwrap();
    let restored = Hir::from_json(&json).unwrap();
    assert_eq!(restored.top, orig.top);
    assert_eq!(restored.modules.len(), orig.modules.len());
    let m = &restored.modules["adder4"];
    assert_eq!(m.ports.len(), 3);
    assert_eq!(m.cont_assigns.len(), 1);
}

#[test]
fn test_json_pretty_round_trip() {
    let orig = adder4_hir();
    let json = orig.to_json_pretty().unwrap();
    assert!(json.contains("\"format\": \"HIR\""));
    let restored = Hir::from_json(&json).unwrap();
    assert_eq!(restored.top, "adder4");
}

#[test]
fn test_json_rejects_wrong_format() {
    let bad = r#"{"format":"XYZ","version":"0.1.0","top":"x","modules":{}}"#;
    assert!(Hir::from_json(bad).is_err());
}

#[test]
fn test_json_rejects_major_version_mismatch() {
    let bad = r#"{"format":"HIR","version":"9.0.0","top":"x","modules":{}}"#;
    assert!(Hir::from_json(bad).is_err());
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[test]
fn test_stats() {
    let hir = adder4_hir();
    let stats = hir.stats();
    assert_eq!(stats.module_count, 1);
    assert_eq!(stats.cont_assign_count, 1);
    assert_eq!(stats.process_count, 0);
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

#[test]
fn test_validation_passes_for_adder4() {
    let hir = adder4_hir();
    let report = validate(&hir);
    assert!(report.ok(), "errors: {:?}", report.errors);
}

#[test]
fn test_h1_missing_top() {
    let mut hir = adder4_hir();
    hir.top = "nonexistent".into();
    let report = validate(&hir);
    assert!(!report.ok());
    assert!(report.errors[0].contains("H1"));
}

#[test]
fn test_h3_unknown_instance_module() {
    let mut hir = adder4_hir();
    hir.modules.get_mut("adder4").unwrap().instances.push(hdl_ir::Instance {
        name: "sub".into(),
        module: "ghost_module".into(),
        connections: Default::default(),
        parameters: Default::default(),
        provenance: None,
    });
    let report = validate(&hir);
    assert!(!report.ok());
    assert!(report.errors.iter().any(|e| e.contains("H3")));
}

#[test]
fn test_h4_bad_connection_key() {
    let mut hir = adder4_hir();
    // Add a second module so instance can resolve, but misspell a port.
    let mut inner = Module::new("inner");
    inner.ports.push(Port {
        name: "x".into(),
        ty: Ty::Bit,
        direction: Direction::In,
        provenance: None,
    });
    hir.modules.insert("inner".into(), inner);

    let mut conns = std::collections::HashMap::new();
    conns.insert("not_a_port".into(), Expr::port_ref("a"));
    hir.modules.get_mut("adder4").unwrap().instances.push(hdl_ir::Instance {
        name: "i0".into(),
        module: "inner".into(),
        connections: conns,
        parameters: Default::default(),
        provenance: None,
    });
    let report = validate(&hir);
    assert!(report.errors.iter().any(|e| e.contains("H4")));
}

#[test]
fn test_h20_self_instantiation() {
    let mut hir = adder4_hir();
    hir.modules.get_mut("adder4").unwrap().instances.push(hdl_ir::Instance {
        name: "self_ref".into(),
        module: "adder4".into(),
        connections: Default::default(),
        parameters: Default::default(),
        provenance: None,
    });
    let report = validate(&hir);
    assert!(report.errors.iter().any(|e| e.contains("H20")));
}

// ---------------------------------------------------------------------------
// Type widths
// ---------------------------------------------------------------------------

#[test]
fn test_ty_width() {
    assert_eq!(Ty::Bit.width(), Some(1));
    assert_eq!(Ty::vec(8).width(), Some(8));
    assert_eq!(Ty::Bool.width(), Some(1));
    assert_eq!(Ty::Int.width(), None);
    assert_eq!(Ty::Real.width(), None);
}

// ---------------------------------------------------------------------------
// Provenance
// ---------------------------------------------------------------------------

#[test]
fn test_provenance_verilog() {
    let p = Provenance::verilog("adder.v", 3, 1);
    assert_eq!(p.lang, SourceLang::Verilog);
    assert_eq!(p.location.as_ref().unwrap().line, 3);
}

#[test]
fn test_provenance_round_trips_in_json() {
    let mut hir = adder4_hir();
    hir.modules.get_mut("adder4").unwrap().provenance =
        Some(Provenance::vhdl("adder.vhd", 10, 5));
    let json = hir.to_json().unwrap();
    let restored = Hir::from_json(&json).unwrap();
    let prov = restored.modules["adder4"].provenance.as_ref().unwrap();
    assert_eq!(prov.lang, SourceLang::Vhdl);
    assert_eq!(prov.location.as_ref().unwrap().column, 5);
}

// ---------------------------------------------------------------------------
// Net kinds
// ---------------------------------------------------------------------------

#[test]
fn test_net_kinds_serialize() {
    let net = Net {
        name: "clk".into(),
        ty: Ty::Bit,
        kind: NetKind::Wire,
        initial: None,
        provenance: None,
    };
    let s = serde_json::to_string(&net).unwrap();
    assert!(s.contains("\"wire\""));
}

// ---------------------------------------------------------------------------
// Empty HIR
// ---------------------------------------------------------------------------

#[test]
fn test_empty_hir_validates_with_h1_error() {
    let hir = Hir::new("top");
    let report = validate(&hir);
    assert!(!report.ok());
    assert!(report.errors[0].contains("H1"));
}
