use hdl_elaboration::{elaborate_verilog, elaborate_verilog_with_top, ElaborationError};
use hdl_ir::module::Direction;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn adder_src() -> &'static str {
    "module adder(input [3:0] a, input [3:0] b, output [4:0] sum);\
     assign sum = a + b;\
     endmodule"
}

// ---------------------------------------------------------------------------
// Basic smoke tests
// ---------------------------------------------------------------------------

#[test]
fn test_minimal_module() {
    let hir = elaborate_verilog("module empty; endmodule").unwrap();
    assert_eq!(hir.top, "empty");
    assert!(hir.modules.contains_key("empty"));
}

#[test]
fn test_top_is_first_module() {
    let src = "module a; endmodule module b; endmodule";
    let hir = elaborate_verilog(src).unwrap();
    assert_eq!(hir.top, "a");
    assert!(hir.modules.contains_key("a"));
    assert!(hir.modules.contains_key("b"));
}

#[test]
fn test_top_override() {
    let src = "module a; endmodule module b; endmodule";
    let hir = elaborate_verilog_with_top(src, "b").unwrap();
    assert_eq!(hir.top, "b");
}

#[test]
fn test_unknown_top_returns_error() {
    let src = "module a; endmodule";
    let result = elaborate_verilog_with_top(src, "does_not_exist");
    assert!(matches!(result, Err(ElaborationError::TopModuleNotFound(_))));
}

// ---------------------------------------------------------------------------
// Port elaboration
// ---------------------------------------------------------------------------

#[test]
fn test_adder_port_count() {
    let hir = elaborate_verilog(adder_src()).unwrap();
    let m = &hir.modules["adder"];
    assert_eq!(m.ports.len(), 3, "expected a, b, sum");
}

#[test]
fn test_adder_port_directions() {
    let hir = elaborate_verilog(adder_src()).unwrap();
    let m = &hir.modules["adder"];
    let find = |name: &str| m.ports.iter().find(|p| p.name == name).unwrap();
    assert_eq!(find("a").direction,   Direction::In);
    assert_eq!(find("b").direction,   Direction::In);
    assert_eq!(find("sum").direction, Direction::Out);
}

#[test]
fn test_adder_port_widths() {
    use hdl_ir::types::Ty;
    let hir = elaborate_verilog(adder_src()).unwrap();
    let m = &hir.modules["adder"];
    let find = |name: &str| m.ports.iter().find(|p| p.name == name).unwrap();
    assert_eq!(find("a").ty,   Ty::vec(4));
    assert_eq!(find("b").ty,   Ty::vec(4));
    assert_eq!(find("sum").ty, Ty::vec(5));
}

#[test]
fn test_single_bit_port() {
    use hdl_ir::types::Ty;
    let src = "module gate(input a, input b, output y); endmodule";
    let hir = elaborate_verilog(src).unwrap();
    let m = &hir.modules["gate"];
    for port in &m.ports {
        assert_eq!(port.ty, Ty::Bit, "port {} should be Ty::Bit", port.name);
    }
}

#[test]
fn test_inout_port() {
    let src = "module buf_tri(inout a); endmodule";
    let hir = elaborate_verilog(src).unwrap();
    let m = &hir.modules["buf_tri"];
    assert_eq!(m.ports[0].direction, Direction::Inout);
}

// ---------------------------------------------------------------------------
// Continuous assignment elaboration
// ---------------------------------------------------------------------------

#[test]
fn test_adder_has_one_cont_assign() {
    let hir = elaborate_verilog(adder_src()).unwrap();
    assert_eq!(hir.modules["adder"].cont_assigns.len(), 1);
}

#[test]
fn test_bitwise_and() {
    use hdl_ir::expr::Expr;
    let src = "module g(input a, input b, output y); assign y = a & b; endmodule";
    let hir = elaborate_verilog(src).unwrap();
    let ca = &hir.modules["g"].cont_assigns[0];
    assert!(matches!(&ca.rhs, Expr::Binary { op, .. } if op == "AND"));
}

#[test]
fn test_bitwise_or() {
    use hdl_ir::expr::Expr;
    let src = "module g(input a, input b, output y); assign y = a | b; endmodule";
    let hir = elaborate_verilog(src).unwrap();
    let ca = &hir.modules["g"].cont_assigns[0];
    assert!(matches!(&ca.rhs, Expr::Binary { op, .. } if op == "OR"));
}

#[test]
fn test_bitwise_xor() {
    use hdl_ir::expr::Expr;
    let src = "module g(input a, input b, output y); assign y = a ^ b; endmodule";
    let hir = elaborate_verilog(src).unwrap();
    let ca = &hir.modules["g"].cont_assigns[0];
    assert!(matches!(&ca.rhs, Expr::Binary { op, .. } if op == "XOR"));
}

#[test]
fn test_subtract() {
    use hdl_ir::expr::Expr;
    let src = "module g(input [3:0] a, input [3:0] b, output [3:0] y); assign y = a - b; endmodule";
    let hir = elaborate_verilog(src).unwrap();
    let ca = &hir.modules["g"].cont_assigns[0];
    assert!(matches!(&ca.rhs, Expr::Binary { op, .. } if op == "-"));
}

#[test]
fn test_multiple_modules_all_elaborated() {
    let src = "module a; endmodule module b; endmodule module c; endmodule";
    let hir = elaborate_verilog(src).unwrap();
    assert_eq!(hir.modules.len(), 3);
    for name in ["a", "b", "c"] {
        assert!(hir.modules.contains_key(name), "module {name} missing");
    }
}

#[test]
fn test_hir_json_round_trip() {
    let hir = elaborate_verilog(adder_src()).unwrap();
    let json = hir.to_json().unwrap();
    let hir2 = hdl_ir::Hir::from_json(&json).unwrap();
    assert_eq!(hir2.top, "adder");
    assert_eq!(hir2.modules["adder"].ports.len(), 3);
}
