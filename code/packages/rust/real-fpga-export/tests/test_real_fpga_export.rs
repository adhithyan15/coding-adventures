//! Integration tests for real-fpga-export.
//!
//! All tests build HIR in-memory and check the emitted Verilog string.
//! Toolchain tests use `skip_missing = true` so they pass without yosys.

use hdl_ir::{
    ContAssign, Direction, Expr, Hir, Instance, Module, Net, NetKind, Port, Ty,
};
use real_fpga_export::{ToolchainOptions, to_ice40, write_verilog, write_verilog_str};

// ---------------------------------------------------------------------------
// Helpers: tiny HIRs
// ---------------------------------------------------------------------------

fn buffer_hir() -> Hir {
    let m = Module {
        name: "my_buffer".into(),
        ports: vec![
            Port { name: "a".into(), ty: Ty::Bit, direction: Direction::In,  provenance: None },
            Port { name: "y".into(), ty: Ty::Bit, direction: Direction::Out, provenance: None },
        ],
        cont_assigns: vec![ContAssign {
            target: Expr::port_ref("y"),
            rhs:    Expr::port_ref("a"),
            provenance: None,
        }],
        ..Default::default()
    };
    let mut hir = Hir::new("my_buffer");
    hir.modules.insert("my_buffer".into(), m);
    hir
}

fn adder4_hir() -> Hir {
    let m = Module {
        name: "adder4".into(),
        ports: vec![
            Port { name: "a".into(),   ty: Ty::vec(4), direction: Direction::In,  provenance: None },
            Port { name: "b".into(),   ty: Ty::vec(4), direction: Direction::In,  provenance: None },
            Port { name: "cin".into(), ty: Ty::Bit,    direction: Direction::In,  provenance: None },
            Port { name: "sum".into(), ty: Ty::vec(4), direction: Direction::Out, provenance: None },
            Port { name: "cout".into(),ty: Ty::Bit,    direction: Direction::Out, provenance: None },
        ],
        cont_assigns: vec![ContAssign {
            target: Expr::Concat {
                parts: vec![Expr::port_ref("cout"), Expr::port_ref("sum")],
                provenance: None,
            },
            rhs: Expr::Binary {
                op: "+".into(),
                lhs: Box::new(Expr::Binary {
                    op: "+".into(),
                    lhs: Box::new(Expr::port_ref("a")),
                    rhs: Box::new(Expr::port_ref("b")),
                    provenance: None,
                }),
                rhs: Box::new(Expr::port_ref("cin")),
                provenance: None,
            },
            provenance: None,
        }],
        ..Default::default()
    };
    let mut hir = Hir::new("adder4");
    hir.modules.insert("adder4".into(), m);
    hir
}

// ---------------------------------------------------------------------------
// write_verilog / write_verilog_str
// ---------------------------------------------------------------------------

#[test]
fn test_buffer_string() {
    let s = write_verilog_str(&buffer_hir());
    assert!(s.contains("module my_buffer"), "missing module declaration");
    assert!(s.contains("input"), "missing input port");
    assert!(s.contains("output"), "missing output port");
    assert!(s.contains("assign y = a"), "missing assignment");
    assert!(s.contains("endmodule"), "missing endmodule");
}

#[test]
fn test_adder4_string() {
    let s = write_verilog_str(&adder4_hir());
    assert!(s.contains("module adder4"));
    assert!(s.contains("[3:0]"), "missing vector range");
    // The concatenation target {cout, sum}
    assert!(s.contains("{cout, sum}"), "missing concat");
    // Plus operators
    assert!(s.contains("+ cin"), "missing addition");
}

#[test]
fn test_vector_port_range() {
    let m = Module {
        name: "x".into(),
        ports: vec![
            Port { name: "data".into(), ty: Ty::vec(8), direction: Direction::In, provenance: None },
        ],
        ..Default::default()
    };
    let mut hir = Hir::new("x");
    hir.modules.insert("x".into(), m);
    let s = write_verilog_str(&hir);
    assert!(s.contains("[7:0]"), "8-bit port must have [7:0] range");
}

#[test]
fn test_scalar_port_no_range() {
    let m = Module {
        name: "x".into(),
        ports: vec![
            Port { name: "clk".into(), ty: Ty::Bit, direction: Direction::In, provenance: None },
        ],
        ..Default::default()
    };
    let mut hir = Hir::new("x");
    hir.modules.insert("x".into(), m);
    let s = write_verilog_str(&hir);
    // The port line should not have a range bracket
    // Find the port list section (between '(' and ');\n')
    if let Some(port_sec) = s.find("module x (").map(|i| &s[i..]).and_then(|t| t.find(");\n").map(|e| &t[..e])) {
        assert!(!port_sec.contains('['), "1-bit port must not have a range");
    }
}

#[test]
fn test_binary_and_op() {
    let m = Module {
        name: "x".into(),
        ports: vec![
            Port { name: "a".into(), ty: Ty::Bit, direction: Direction::In,  provenance: None },
            Port { name: "b".into(), ty: Ty::Bit, direction: Direction::In,  provenance: None },
            Port { name: "y".into(), ty: Ty::Bit, direction: Direction::Out, provenance: None },
        ],
        cont_assigns: vec![ContAssign {
            target: Expr::port_ref("y"),
            rhs: Expr::Binary {
                op: "AND".into(),
                lhs: Box::new(Expr::port_ref("a")),
                rhs: Box::new(Expr::port_ref("b")),
                provenance: None,
            },
            provenance: None,
        }],
        ..Default::default()
    };
    let mut hir = Hir::new("x");
    hir.modules.insert("x".into(), m);
    let s = write_verilog_str(&hir);
    assert!(s.contains("(a & b)"), "AND must map to &");
}

#[test]
fn test_unary_not() {
    let m = Module {
        name: "x".into(),
        ports: vec![
            Port { name: "a".into(), ty: Ty::Bit, direction: Direction::In,  provenance: None },
            Port { name: "y".into(), ty: Ty::Bit, direction: Direction::Out, provenance: None },
        ],
        cont_assigns: vec![ContAssign {
            target: Expr::port_ref("y"),
            rhs: Expr::Unary {
                op: "NOT".into(),
                operand: Box::new(Expr::port_ref("a")),
                provenance: None,
            },
            provenance: None,
        }],
        ..Default::default()
    };
    let mut hir = Hir::new("x");
    hir.modules.insert("x".into(), m);
    let s = write_verilog_str(&hir);
    assert!(s.contains("(~a)"), "NOT must map to ~");
}

#[test]
fn test_ternary() {
    let m = Module {
        name: "x".into(),
        ports: vec![
            Port { name: "s".into(), ty: Ty::Bit, direction: Direction::In,  provenance: None },
            Port { name: "a".into(), ty: Ty::Bit, direction: Direction::In,  provenance: None },
            Port { name: "b".into(), ty: Ty::Bit, direction: Direction::In,  provenance: None },
            Port { name: "y".into(), ty: Ty::Bit, direction: Direction::Out, provenance: None },
        ],
        cont_assigns: vec![ContAssign {
            target: Expr::port_ref("y"),
            rhs: Expr::Ternary {
                cond:      Box::new(Expr::port_ref("s")),
                then_expr: Box::new(Expr::port_ref("a")),
                else_expr: Box::new(Expr::port_ref("b")),
                provenance: None,
            },
            provenance: None,
        }],
        ..Default::default()
    };
    let mut hir = Hir::new("x");
    hir.modules.insert("x".into(), m);
    let s = write_verilog_str(&hir);
    assert!(s.contains("? a : b"), "ternary must use ? : syntax");
}

#[test]
fn test_slice() {
    let m = Module {
        name: "x".into(),
        ports: vec![
            Port { name: "d".into(), ty: Ty::vec(8), direction: Direction::In,  provenance: None },
            Port { name: "y".into(), ty: Ty::vec(4), direction: Direction::Out, provenance: None },
        ],
        cont_assigns: vec![ContAssign {
            target: Expr::port_ref("y"),
            rhs: Expr::Slice {
                base: Box::new(Expr::port_ref("d")),
                msb: 3, lsb: 0,
                provenance: None,
            },
            provenance: None,
        }],
        ..Default::default()
    };
    let mut hir = Hir::new("x");
    hir.modules.insert("x".into(), m);
    let s = write_verilog_str(&hir);
    assert!(s.contains("d[3:0]"), "slice must emit [msb:lsb]");
}

#[test]
fn test_concat() {
    let m = Module {
        name: "x".into(),
        ports: vec![
            Port { name: "a".into(), ty: Ty::Bit,    direction: Direction::In,  provenance: None },
            Port { name: "b".into(), ty: Ty::vec(4), direction: Direction::In,  provenance: None },
            Port { name: "y".into(), ty: Ty::vec(5), direction: Direction::Out, provenance: None },
        ],
        cont_assigns: vec![ContAssign {
            target: Expr::port_ref("y"),
            rhs: Expr::Concat {
                parts: vec![Expr::port_ref("a"), Expr::port_ref("b")],
                provenance: None,
            },
            provenance: None,
        }],
        ..Default::default()
    };
    let mut hir = Hir::new("x");
    hir.modules.insert("x".into(), m);
    let s = write_verilog_str(&hir);
    assert!(s.contains("{a, b}"), "concat must use {{a, b}} syntax");
}

#[test]
fn test_lit_int() {
    use hdl_ir::expr::LitValue;
    let m = Module {
        name: "x".into(),
        ports: vec![
            Port { name: "y".into(), ty: Ty::vec(4), direction: Direction::Out, provenance: None },
        ],
        cont_assigns: vec![ContAssign {
            target: Expr::port_ref("y"),
            rhs: Expr::Lit { value: LitValue::Int(10), ty: Ty::vec(4), provenance: None },
            provenance: None,
        }],
        ..Default::default()
    };
    let mut hir = Hir::new("x");
    hir.modules.insert("x".into(), m);
    let s = write_verilog_str(&hir);
    assert!(s.contains("4'd10"), "int lit must emit <width>'d<value>");
}

#[test]
fn test_lit_bool() {
    use hdl_ir::expr::LitValue;
    let m = Module {
        name: "x".into(),
        ports: vec![
            Port { name: "y".into(), ty: Ty::Bit, direction: Direction::Out, provenance: None },
        ],
        cont_assigns: vec![ContAssign {
            target: Expr::port_ref("y"),
            rhs: Expr::Lit { value: LitValue::Bool(true), ty: Ty::Bit, provenance: None },
            provenance: None,
        }],
        ..Default::default()
    };
    let mut hir = Hir::new("x");
    hir.modules.insert("x".into(), m);
    let s = write_verilog_str(&hir);
    assert!(s.contains("1'b1"), "true literal must emit 1'b1");
}

#[test]
fn test_instance_emission() {
    let child = Module {
        name: "child".into(),
        ports: vec![
            Port { name: "a".into(), ty: Ty::Bit, direction: Direction::In,  provenance: None },
            Port { name: "y".into(), ty: Ty::Bit, direction: Direction::Out, provenance: None },
        ],
        ..Default::default()
    };
    let parent = Module {
        name: "parent".into(),
        ports: vec![
            Port { name: "p_a".into(), ty: Ty::Bit, direction: Direction::In,  provenance: None },
            Port { name: "p_y".into(), ty: Ty::Bit, direction: Direction::Out, provenance: None },
        ],
        instances: vec![Instance {
            name: "u".into(),
            module: "child".into(),
            connections: {
                let mut m = std::collections::HashMap::new();
                m.insert("a".to_string(), Expr::port_ref("p_a"));
                m.insert("y".to_string(), Expr::port_ref("p_y"));
                m
            },
            parameters: Default::default(),
            provenance: None,
        }],
        ..Default::default()
    };
    let mut hir = Hir::new("parent");
    hir.modules.insert("parent".into(), parent);
    hir.modules.insert("child".into(), child);
    let s = write_verilog_str(&hir);
    assert!(s.contains("module parent"), "missing parent module");
    assert!(s.contains("module child"), "missing child module");
    assert!(s.contains("child u"), "missing instance");
    assert!(s.contains(".a(p_a)"), "missing port connection a");
    assert!(s.contains(".y(p_y)"), "missing port connection y");
}

#[test]
fn test_reserved_word_escaped() {
    // A port named "and" (a reserved word) must be emitted as \and  (note space)
    let m = Module {
        name: "x".into(),
        ports: vec![
            Port { name: "and".into(), ty: Ty::Bit, direction: Direction::In,  provenance: None },
            Port { name: "y".into(),   ty: Ty::Bit, direction: Direction::Out, provenance: None },
        ],
        ..Default::default()
    };
    let mut hir = Hir::new("x");
    hir.modules.insert("x".into(), m);
    let s = write_verilog_str(&hir);
    assert!(s.contains("\\and "), "reserved word 'and' must be escaped");
}

#[test]
fn test_internal_net() {
    let m = Module {
        name: "x".into(),
        ports: vec![
            Port { name: "a".into(), ty: Ty::Bit, direction: Direction::In,  provenance: None },
            Port { name: "y".into(), ty: Ty::Bit, direction: Direction::Out, provenance: None },
        ],
        nets: vec![Net {
            name: "internal".into(),
            ty: Ty::Bit,
            kind: NetKind::Wire,
            initial: None,
            provenance: None,
        }],
        cont_assigns: vec![
            ContAssign { target: Expr::net_ref("internal"), rhs: Expr::port_ref("a"), provenance: None },
            ContAssign { target: Expr::port_ref("y"),       rhs: Expr::net_ref("internal"), provenance: None },
        ],
        ..Default::default()
    };
    let mut hir = Hir::new("x");
    hir.modules.insert("x".into(), m);
    let s = write_verilog_str(&hir);
    assert!(s.contains("wire internal"), "internal net must declare a wire");
}

#[test]
fn test_write_verilog_to_file() {
    let dir = std::env::temp_dir().join("real_fpga_export_test");
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join("buf.v");
    write_verilog(&buffer_hir(), &path).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("module my_buffer"));
}

// ---------------------------------------------------------------------------
// Toolchain (skip_missing path)
// ---------------------------------------------------------------------------

#[test]
fn test_to_ice40_skip_missing_no_yosys() {
    let dir = std::env::temp_dir().join("real_fpga_export_toolchain_test");
    std::fs::create_dir_all(&dir).ok();

    let opts = ToolchainOptions {
        yosys: "this-tool-does-not-exist-xyz".to_string(),
        ..Default::default()
    };
    let result = to_ice40(
        &buffer_hir(), "my_buffer", None,
        &dir, "hx1k", "tq144", Some(&opts), true,
    ).unwrap();
    assert!(result.verilog_path.exists(), "verilog file must be written even when yosys is absent");
    assert!(result.json_path.is_none(), "json must not be set if yosys was skipped");
}

#[test]
fn test_to_ice40_creates_output_dir() {
    let dir = std::env::temp_dir()
        .join("real_fpga_export_mkdir_test")
        .join("build")
        .join("nested");
    if dir.exists() {
        std::fs::remove_dir_all(&dir).ok();
    }
    let opts = ToolchainOptions {
        yosys: "this-tool-does-not-exist-xyz".to_string(),
        ..Default::default()
    };
    let result = to_ice40(
        &buffer_hir(), "my_buffer", None,
        &dir, "hx1k", "tq144", Some(&opts), true,
    ).unwrap();
    assert!(dir.exists(), "output directory must be created");
    assert!(result.verilog_path.exists());
}
