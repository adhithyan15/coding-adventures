//! # Synthesis — HIR → HNL
//!
//! Synthesis transforms the behavioral Hardware IR into a structural gate
//! netlist. This is the step where *what a circuit does* (described with
//! addition, logic ops, conditionals) becomes *how a circuit does it*
//! (AND, OR, XOR gates wired together).
//!
//! ## Scope (v0.1.0)
//!
//! - Combinational `ContAssign` mapping to gate networks.
//! - Operator lowering: `+`, `-`, `AND`, `OR`, `XOR`, `NOT`, etc.
//! - Adder lowering: N-bit `+` decomposes to a ripple-carry chain of full
//!   adders, each built from `XOR2 + AND2 + OR2` primitives.
//! - Width inference from HIR types (`Ty::vec(N)` → N wires).
//!
//! ## The 4-bit adder
//!
//! ```text
//! assign sum = a + b;   →   4× FullAdder = 8 XOR2 + 8 AND2 + 4 OR2
//! ```
//!
//! Each 1-bit full adder expands to:
//! ```text
//! s    = a XOR b XOR cin
//! cout = (a AND b) OR (cin AND (a XOR b))
//! ```

use gate_netlist_format::{Direction, Instance, Level, Module, Net, Netlist, NetSlice, Port};
use hdl_ir::{
    ContAssign, Direction as HirDir, Expr, Hir, Module as HirModule,
};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Synthesize an HIR document into a generic-level HNL netlist.
pub fn synthesize(hir: &Hir) -> Netlist {
    let mut nl = Netlist::new(hir.top.clone());
    nl.level = Level::Generic;
    for (name, module) in &hir.modules {
        let hnl_mod = synthesize_module(name, module);
        nl.modules.insert(name.clone(), hnl_mod);
    }
    nl
}

// ---------------------------------------------------------------------------
// Per-module synthesis context
// ---------------------------------------------------------------------------

struct Ctx {
    module: Module,
    intermediate_count: u32,
    cell_count: u32,
    /// net/port name → bit-width
    widths: std::collections::HashMap<String, u32>,
}

impl Ctx {
    fn new(name: &str) -> Self {
        Self {
            module: Module::new(name),
            intermediate_count: 0,
            cell_count: 0,
            widths: Default::default(),
        }
    }

    fn fresh_net(&mut self, width: u32, prefix: &str) -> String {
        let name = format!("{prefix}{}", self.intermediate_count);
        self.intermediate_count += 1;
        self.module.nets.push(Net::new(name.clone(), width));
        self.widths.insert(name.clone(), width);
        name
    }

    fn fresh_cell(&mut self, hint: &str) -> String {
        let name = format!("{hint}_{}", self.cell_count);
        self.cell_count += 1;
        name
    }

    fn add_cell(&mut self, cell_type: &str, hint: &str, conns: Vec<(&str, NetSlice)>) -> String {
        let name = self.fresh_cell(hint);
        let mut inst = Instance::new(name.clone(), cell_type);
        for (pin, slice) in conns {
            inst.connections.insert(pin.to_string(), slice);
        }
        self.module.instances.push(inst);
        name
    }

    fn width_of(&self, name: &str) -> u32 {
        self.widths.get(name).copied().unwrap_or(1)
    }
}

// ---------------------------------------------------------------------------
// Module synthesis
// ---------------------------------------------------------------------------

fn synthesize_module(name: &str, hir_mod: &HirModule) -> Module {
    let mut ctx = Ctx::new(name);

    // Translate ports.
    for port in &hir_mod.ports {
        let w = port.ty.width().unwrap_or(1);
        let dir = match port.direction {
            HirDir::In => Direction::Input,
            HirDir::Out => Direction::Output,
            HirDir::Inout => Direction::Inout,
        };
        ctx.module.ports.push(Port::new(port.name.clone(), dir, w));
        ctx.widths.insert(port.name.clone(), w);
    }

    // Translate internal nets.
    for net in &hir_mod.nets {
        let w = net.ty.width().unwrap_or(1);
        ctx.module.nets.push(Net::new(net.name.clone(), w));
        ctx.widths.insert(net.name.clone(), w);
    }

    // Synthesize ContAssigns.
    for ca in &hir_mod.cont_assigns {
        synth_cont_assign(&mut ctx, ca);
    }

    ctx.module
}

fn synth_cont_assign(ctx: &mut Ctx, ca: &ContAssign) {
    let target_name = expr_to_net_name(&ca.target);
    let target_width = ctx.width_of(&target_name.clone());

    let rhs_net = synth_expr(ctx, &ca.rhs, target_width);

    // Wire rhs_net → target via a chain of BUF cells (width=1 each bit).
    for bit in 0..target_width {
        ctx.add_cell(
            "BUF",
            "_buf",
            vec![
                ("A", NetSlice::single(rhs_net.clone(), bit)),
                ("Y", NetSlice::single(target_name.clone(), bit)),
            ],
        );
    }
}

/// Extract the net name from an lvalue expression (NetRef or PortRef).
fn expr_to_net_name(expr: &Expr) -> String {
    match expr {
        Expr::NetRef { name, .. } => name.clone(),
        Expr::PortRef { name, .. } => name.clone(),
        _ => "_unknown".to_string(),
    }
}

/// Synthesize `expr` into a fresh intermediate net of `width` bits.
/// Returns the name of the net holding the result.
fn synth_expr(ctx: &mut Ctx, expr: &Expr, width: u32) -> String {
    match expr {
        Expr::PortRef { name, .. } | Expr::NetRef { name, .. } => name.clone(),

        Expr::Lit { value, .. } => {
            // Constant: emit CONST_0 or CONST_1 gates per bit.
            let out = ctx.fresh_net(width, "_lit");
            let int_val = match value {
                hdl_ir::expr::LitValue::Int(i) => *i as u64,
                hdl_ir::expr::LitValue::Bool(b) => if *b { 1 } else { 0 },
                _ => 0,
            };
            for bit in 0..width {
                let cell = if (int_val >> bit) & 1 == 1 { "CONST_1" } else { "CONST_0" };
                ctx.add_cell(cell, "_const", vec![("Y", NetSlice::single(out.clone(), bit))]);
            }
            out
        }

        Expr::Unary { op, operand, .. } => {
            let a = synth_expr(ctx, operand, width);
            let out = ctx.fresh_net(width, "_unary");
            match op.as_str() {
                "NOT" => {
                    for bit in 0..width {
                        ctx.add_cell(
                            "NOT",
                            "_not",
                            vec![
                                ("A", NetSlice::single(a.clone(), bit)),
                                ("Y", NetSlice::single(out.clone(), bit)),
                            ],
                        );
                    }
                }
                _ => {
                    // Fallback: pass through.
                    for bit in 0..width {
                        ctx.add_cell(
                            "BUF", "_buf",
                            vec![
                                ("A", NetSlice::single(a.clone(), bit)),
                                ("Y", NetSlice::single(out.clone(), bit)),
                            ],
                        );
                    }
                }
            }
            out
        }

        Expr::Binary { op, lhs, rhs, .. } => {
            synth_binary_op(ctx, op, lhs, rhs, width)
        }

        Expr::Ternary { cond, then_expr, else_expr, .. } => {
            let sel = synth_expr(ctx, cond, 1);
            let t = synth_expr(ctx, then_expr, width);
            let f = synth_expr(ctx, else_expr, width);
            let out = ctx.fresh_net(width, "_mux");
            for bit in 0..width {
                ctx.add_cell(
                    "MUX2", "_mux2",
                    vec![
                        ("A", NetSlice::single(f.clone(), bit)),
                        ("B", NetSlice::single(t.clone(), bit)),
                        ("S", NetSlice::single(sel.clone(), 0)),
                        ("Y", NetSlice::single(out.clone(), bit)),
                    ],
                );
            }
            out
        }

        Expr::Concat { parts, .. } => {
            let out = ctx.fresh_net(width, "_cat");
            let mut bit = 0u32;
            for part in parts.iter().rev() {
                let part_w = expr_infer_width(part, ctx).unwrap_or(1);
                let src = synth_expr(ctx, part, part_w);
                for pb in 0..part_w {
                    if bit < width {
                        ctx.add_cell(
                            "BUF", "_cat_buf",
                            vec![
                                ("A", NetSlice::single(src.clone(), pb)),
                                ("Y", NetSlice::single(out.clone(), bit)),
                            ],
                        );
                        bit += 1;
                    }
                }
            }
            out
        }

        _ => {
            // Unknown: wire zeros.
            let out = ctx.fresh_net(width, "_unk");
            for bit in 0..width {
                ctx.add_cell("CONST_0", "_zero", vec![("Y", NetSlice::single(out.clone(), bit))]);
            }
            out
        }
    }
}

fn synth_binary_op(ctx: &mut Ctx, op: &str, lhs: &Expr, rhs: &Expr, width: u32) -> String {
    match op {
        // Bitwise ops — one gate per bit.
        "AND" | "&" => synth_bitwise(ctx, "AND2", lhs, rhs, width),
        "OR"  | "|" => synth_bitwise(ctx, "OR2",  lhs, rhs, width),
        "XOR" | "^" => synth_bitwise(ctx, "XOR2", lhs, rhs, width),
        "NAND"       => synth_bitwise(ctx, "NAND2", lhs, rhs, width),
        "NOR"        => synth_bitwise(ctx, "NOR2",  lhs, rhs, width),
        "XNOR"       => synth_bitwise(ctx, "XNOR2", lhs, rhs, width),

        // Arithmetic add — ripple-carry adder.
        "+" => {
            let operand_w = width.saturating_sub(1).max(1);
            let a = synth_expr(ctx, lhs, operand_w);
            let b = synth_expr(ctx, rhs, operand_w);
            synth_adder(ctx, &a, &b, operand_w, width)
        }

        // Logical comparisons — simplistic: compare bit 0 only in v0.1.0.
        "==" | "!=" | "<" | "<=" | ">" | ">=" | "&&" | "||" => {
            let a = synth_expr(ctx, lhs, 1);
            let b = synth_expr(ctx, rhs, 1);
            let out = ctx.fresh_net(1, "_cmp");
            let cell = if op == "&&" { "AND2" } else if op == "||" { "OR2" } else { "XOR2" };
            ctx.add_cell(cell, "_cmp", vec![
                ("A", NetSlice::single(a, 0)),
                ("B", NetSlice::single(b, 0)),
                ("Y", NetSlice::single(out.clone(), 0)),
            ]);
            out
        }

        // Subtract — complement + add (simple sign-magnitude; exact in v0.2.0).
        "-" => {
            let operand_w = width.saturating_sub(1).max(1);
            let a = synth_expr(ctx, lhs, operand_w);
            let raw_b = synth_expr(ctx, rhs, operand_w);
            // ~b
            let inv_b = ctx.fresh_net(operand_w, "_inv_b");
            for bit in 0..operand_w {
                ctx.add_cell("NOT", "_not_b", vec![
                    ("A", NetSlice::single(raw_b.clone(), bit)),
                    ("Y", NetSlice::single(inv_b.clone(), bit)),
                ]);
            }
            // a + ~b (= a - b - 1; carry-in=1 for two's complement handled in v0.2.0)
            synth_adder(ctx, &a, &inv_b, operand_w, width)
        }

        // Shifts — barrel-shifter in v0.2.0; pass-through for now.
        "<<" | ">>" | "<<<" | ">>>" => {
            synth_expr(ctx, lhs, width)
        }

        _ => synth_expr(ctx, lhs, width),
    }
}

/// One gate-per-bit bitwise operation.
fn synth_bitwise(ctx: &mut Ctx, gate: &str, lhs: &Expr, rhs: &Expr, width: u32) -> String {
    let a = synth_expr(ctx, lhs, width);
    let b = synth_expr(ctx, rhs, width);
    let out = ctx.fresh_net(width, "_bw");
    for bit in 0..width {
        ctx.add_cell(gate, "_bw", vec![
            ("A", NetSlice::single(a.clone(), bit)),
            ("B", NetSlice::single(b.clone(), bit)),
            ("Y", NetSlice::single(out.clone(), bit)),
        ]);
    }
    out
}

/// Ripple-carry adder. `a` and `b` are `operand_w`-bit nets; result is
/// `result_w` bits (usually operand_w + 1 to capture the carry-out).
///
/// Each full-adder bit:
/// ```text
/// xab   = a XOR b
/// sum_i = xab XOR cin
/// and_ab  = a AND b
/// and_cx  = cin AND xab
/// cout  = and_ab OR and_cx
/// ```
fn synth_adder(ctx: &mut Ctx, a: &str, b: &str, operand_w: u32, result_w: u32) -> String {
    let out = ctx.fresh_net(result_w, "_sum");
    let mut carry = ctx.fresh_net(1, "_c");
    // Bit 0 carry-in = 0.
    ctx.add_cell("CONST_0", "_cin", vec![("Y", NetSlice::single(carry.clone(), 0))]);

    for bit in 0..operand_w {
        // xab = a[bit] XOR b[bit]
        let xab = ctx.fresh_net(1, "_xab");
        ctx.add_cell("XOR2", "_xab", vec![
            ("A", NetSlice::single(a.to_string(), bit)),
            ("B", NetSlice::single(b.to_string(), bit)),
            ("Y", NetSlice::single(xab.clone(), 0)),
        ]);
        // sum[bit] = xab XOR cin
        let sum_bit = ctx.fresh_net(1, "_sb");
        ctx.add_cell("XOR2", "_sum_xor", vec![
            ("A", NetSlice::single(xab.clone(), 0)),
            ("B", NetSlice::single(carry.clone(), 0)),
            ("Y", NetSlice::single(sum_bit.clone(), 0)),
        ]);
        // Wire sum_bit → out[bit]
        ctx.add_cell("BUF", "_sbuf", vec![
            ("A", NetSlice::single(sum_bit.clone(), 0)),
            ("Y", NetSlice::single(out.clone(), bit)),
        ]);
        // and_ab = a[bit] AND b[bit]
        let and_ab = ctx.fresh_net(1, "_aab");
        ctx.add_cell("AND2", "_and_ab", vec![
            ("A", NetSlice::single(a.to_string(), bit)),
            ("B", NetSlice::single(b.to_string(), bit)),
            ("Y", NetSlice::single(and_ab.clone(), 0)),
        ]);
        // and_cx = cin AND xab
        let and_cx = ctx.fresh_net(1, "_acx");
        ctx.add_cell("AND2", "_and_cx", vec![
            ("A", NetSlice::single(carry.clone(), 0)),
            ("B", NetSlice::single(xab.clone(), 0)),
            ("Y", NetSlice::single(and_cx.clone(), 0)),
        ]);
        // cout = and_ab OR and_cx
        let cout = ctx.fresh_net(1, "_co");
        ctx.add_cell("OR2", "_or_co", vec![
            ("A", NetSlice::single(and_ab.clone(), 0)),
            ("B", NetSlice::single(and_cx.clone(), 0)),
            ("Y", NetSlice::single(cout.clone(), 0)),
        ]);
        carry = cout;
    }

    // Final carry bit → MSB of result (if result_w > operand_w).
    if result_w > operand_w {
        ctx.add_cell("BUF", "_cout", vec![
            ("A", NetSlice::single(carry.clone(), 0)),
            ("Y", NetSlice::single(out.clone(), operand_w)),
        ]);
    }

    out
}

/// Best-effort width inference from an expression without a full type-checker.
fn expr_infer_width(expr: &Expr, ctx: &Ctx) -> Option<u32> {
    match expr {
        Expr::PortRef { name, .. } | Expr::NetRef { name, .. } => ctx.widths.get(name).copied(),
        Expr::Lit { ty, .. } => ty.width(),
        Expr::Concat { parts, .. } => {
            parts.iter().map(|p| expr_infer_width(p, ctx).unwrap_or(1)).reduce(|a, b| a + b)
        }
        _ => None,
    }
}
