//! HIR expression evaluator.
//!
//! Walks an `Expr` tree and computes a 64-bit integer value given a signal
//! lookup function. This is the kernel that drives continuous-assignment
//! simulation: every time a signal changes, we re-evaluate all ContAssigns
//! whose RHS references that signal.
//!
//! ## Representation
//!
//! Values are `i64`. Multi-bit signals are stored as unsigned values in the
//! lower bits; bitwise NOT is masked to 32 bits. This matches the Python
//! reference implementation's "2-state int" semantics. 4-state (X/Z) is
//! documented as v0.2.0 work.
//!
//! ## Operator mapping (HIR op string → Rust)
//!
//! | Op string | Meaning |
//! |-----------|---------|
//! | `+`, `-`, `*`, `/`, `%` | Arithmetic |
//! | `**` | Exponentiation |
//! | `AND` / `&`, `OR` / `\|`, `XOR` / `^` | Bitwise |
//! | `NAND`, `NOR`, `XNOR` | Bitwise complement of AND/OR/XOR |
//! | `<<`, `>>`, `<<<`, `>>>` | Shifts (arithmetic right not width-aware in v0.1.0) |
//! | `==`, `!=`, `===`, `!==`, `<`, `<=`, `>`, `>=` | Comparisons → 0 or 1 |
//! | `&&`, `\|\|` | Logical |
//! | `NOT`, `NEG`, etc. | Unary |
//! | `AND_RED`, `OR_RED`, `XOR_RED`, … | Reduction operators |

use hdl_ir::expr::{Expr, LitValue};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Evaluate an HIR expression to an integer, using `lookup` to read signals.
pub fn evaluate<F>(expr: &Expr, lookup: &F) -> i64
where
    F: Fn(&str) -> i64,
{
    match expr {
        Expr::Lit { value, .. } => eval_lit(value),

        Expr::NetRef { name, .. } | Expr::VarRef { name, .. } | Expr::PortRef { name, .. } => {
            lookup(name)
        }

        Expr::Slice { base, msb, lsb, .. } => {
            let base_val = evaluate(base, lookup);
            let (msb, lsb) = if msb >= lsb { (*msb, *lsb) } else { (*lsb, *msb) };
            let width = msb - lsb + 1;
            let mask = (1i64 << width) - 1;
            (base_val >> lsb) & mask
        }

        Expr::Concat { parts, .. } => {
            // Pack parts MSB-first; each contributes `expr_width` bits.
            // Width is clamped to [1, 62] to prevent shift overflow.
            let mut result = 0i64;
            for part in parts {
                let part_val = evaluate(part, lookup);
                let w = expr_width(part, lookup).clamp(1, 62);
                result = (result << w) | (part_val & ((1i64 << w) - 1));
            }
            result
        }

        Expr::Replication { count, body, .. } => {
            // Replication count is capped to prevent DoS via huge iteration loops.
            const MAX_REPLICATION: i64 = 65_536;
            let count = evaluate(count, lookup).clamp(0, MAX_REPLICATION);
            let body_val = evaluate(body, lookup);
            let body_w = expr_width(body, lookup).clamp(1, 62);
            let body_mask = (1i64 << body_w) - 1;
            let mut result = 0i64;
            for _ in 0..count {
                result = (result << body_w) | (body_val & body_mask);
            }
            result
        }

        Expr::Unary { op, operand, .. } => {
            let operand = evaluate(operand, lookup);
            apply_unary(op, operand)
        }

        Expr::Binary { op, lhs, rhs, .. } => {
            let lhs = evaluate(lhs, lookup);
            let rhs = evaluate(rhs, lookup);
            apply_binary(op, lhs, rhs)
        }

        Expr::Ternary { cond, then_expr, else_expr, .. } => {
            if evaluate(cond, lookup) != 0 {
                evaluate(then_expr, lookup)
            } else {
                evaluate(else_expr, lookup)
            }
        }

        // FunCall, SystemCall, Attr — not simulated in v0.1.0
        _ => 0,
    }
}

/// Collect all Net/Port/Var names referenced in an expression — used for
/// sensitivity list inference on continuous assignments.
pub fn referenced_signals(expr: &Expr) -> Vec<String> {
    let mut out = Vec::new();
    collect_signals(expr, &mut out);
    out
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn eval_lit(v: &LitValue) -> i64 {
    match v {
        LitValue::Int(n)  => *n,
        LitValue::Bool(b) => *b as i64,
        LitValue::Float(f) => *f as i64,
        LitValue::Str(s)  => s.parse::<i64>().unwrap_or(0),
        LitValue::Bits(bits) => {
            let mut result = 0i64;
            for bit in bits {
                result = (result << 1) | (*bit as i64 & 1);
            }
            result
        }
    }
}

/// Estimate bit-width of an expression for concat/replication packing.
fn expr_width<F>(expr: &Expr, lookup: &F) -> i64
where
    F: Fn(&str) -> i64,
{
    match expr {
        Expr::Slice { msb, lsb, .. } => (*msb as i64 - *lsb as i64).abs() + 1,
        Expr::Concat { parts, .. } => parts.iter().map(|p| expr_width(p, lookup).max(1)).sum(),
        Expr::Replication { count, body, .. } => {
            let c = evaluate(count, lookup).clamp(0, 65_536);
            c.saturating_mul(expr_width(body, lookup).max(1)).min(62)
        }
        Expr::Lit { ty, .. } => {
            use hdl_ir::types::Ty;
            match ty {
                Ty::Vec { width, .. } => *width as i64,
                _ => 1,
            }
        }
        _ => 1,
    }
}

fn apply_unary(op: &str, operand: i64) -> i64 {
    match op {
        "NEG"      => operand.wrapping_neg(),
        "POS"      => operand,
        "LOGIC_NOT" => i64::from(operand == 0),
        "NOT"      => (!operand) & 0xFFFF_FFFF,
        "AND_RED"  => i64::from(operand != 0 && (operand & (operand + 1)) == 0),
        "OR_RED"   => i64::from(operand != 0),
        "XOR_RED"  => {
            let mut x = operand;
            let mut r = 0i64;
            while x != 0 { r ^= x & 1; x >>= 1; }
            r
        }
        "NAND_RED" => i64::from(!(operand != 0 && (operand & (operand + 1)) == 0)),
        "NOR_RED"  => i64::from(operand == 0),
        "XNOR_RED" => 1 - apply_unary("XOR_RED", operand),
        _ => 0,
    }
}

fn apply_binary(op: &str, lhs: i64, rhs: i64) -> i64 {
    match op {
        "+"   => lhs.wrapping_add(rhs),
        "-"   => lhs.wrapping_sub(rhs),
        "*"   => lhs.wrapping_mul(rhs),
        "/"   => if rhs == 0 { 0 } else { lhs / rhs },
        "%"   => if rhs == 0 { 0 } else { lhs % rhs },
        "**"  => lhs.wrapping_pow(rhs.clamp(0, 63) as u32),
        "AND" | "&"  => lhs & rhs,
        "OR"  | "|"  => lhs | rhs,
        "XOR" | "^"  => lhs ^ rhs,
        "NAND" => (!( lhs & rhs)) & 0xFFFF_FFFF,
        "NOR"  => (!(lhs | rhs)) & 0xFFFF_FFFF,
        "XNOR" => (!(lhs ^ rhs)) & 0xFFFF_FFFF,
        "<<" | "<<<" => { let s = rhs.clamp(0, 63) as u32; lhs << s }
        ">>" | ">>>" => { let s = rhs.clamp(0, 63) as u32; lhs >> s }
        "==" | "===" => i64::from(lhs == rhs),
        "!=" | "!==" => i64::from(lhs != rhs),
        "<"   => i64::from(lhs <  rhs),
        "<="  => i64::from(lhs <= rhs),
        ">"   => i64::from(lhs >  rhs),
        ">="  => i64::from(lhs >= rhs),
        "&&"  => i64::from(lhs != 0 && rhs != 0),
        "||"  => i64::from(lhs != 0 || rhs != 0),
        _     => 0,
    }
}

fn collect_signals(expr: &Expr, out: &mut Vec<String>) {
    match expr {
        Expr::NetRef { name, .. } | Expr::VarRef { name, .. } | Expr::PortRef { name, .. } => {
            out.push(name.clone());
        }
        Expr::Slice { base, .. } => collect_signals(base, out),
        Expr::Concat { parts, .. } => parts.iter().for_each(|p| collect_signals(p, out)),
        Expr::Replication { count, body, .. } => {
            collect_signals(count, out);
            collect_signals(body, out);
        }
        Expr::Unary { operand, .. } => collect_signals(operand, out),
        Expr::Binary { lhs, rhs, .. } => {
            collect_signals(lhs, out);
            collect_signals(rhs, out);
        }
        Expr::Ternary { cond, then_expr, else_expr, .. } => {
            collect_signals(cond, out);
            collect_signals(then_expr, out);
            collect_signals(else_expr, out);
        }
        Expr::FunCall { args, .. } | Expr::SystemCall { args, .. } => {
            args.iter().for_each(|a| collect_signals(a, out));
        }
        Expr::Attr { base, args, .. } => {
            collect_signals(base, out);
            args.iter().for_each(|a| collect_signals(a, out));
        }
        Expr::Lit { .. } => {}
    }
}
