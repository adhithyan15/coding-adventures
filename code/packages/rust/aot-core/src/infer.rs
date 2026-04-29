//! Static type inference for `IIRFunction` — the first pass of AOT compilation.
//!
//! AOT cannot observe runtime types the way JIT can, so it runs a lightweight
//! **flow-insensitive** type inference pass over the `IIRInstr` sequence.
//!
//! # How it works
//!
//! The pass maintains an *environment* `env: HashMap<String, String>` mapping
//! virtual variable names to their inferred IIR type strings.  It walks
//! instructions in order, applying inference rules:
//!
//! 1. **Seed** — function parameters contribute `name → type` entries using
//!    their declared types.
//!
//! 2. **Typed instructions** — if `instr.type_hint != "any"`, the dest is
//!    immediately bound to `type_hint`.  No inference needed.
//!
//! 3. **`const`** — the dest type is derived from the Rust operand in
//!    `srcs[0]`: `Bool → "bool"`, small ints → `"u8"`/`"u16"`/etc.,
//!    `Float → "f64"`, `Var → "str"`, nothing → `"any"`.
//!
//! 4. **Arithmetic / bitwise ops** — both sources must resolve to numeric types;
//!    the result is the *wider* of the two (numeric promotion):
//!
//!    ```text
//!    u8 + u8  → u8
//!    u8 + u16 → u16    (promotion)
//!    f64 + u8 → f64
//!    str + u8 → any    (incompatible)
//!    ```
//!
//!    Numeric rank order: `bool < u8 < u16 < u32 < u64 < f64`.
//!    `"add"` on two `"str"` operands is a special case (string concatenation)
//!    and infers `"str"`.
//!
//! 5. **Comparison ops** — result is always `"bool"` when all sources have
//!    known non-`"any"` types; `"any"` otherwise.
//!
//! 6. **Unary ops** (`neg`, `not`) — result is the same type as the source.
//!
//! 7. **Passthrough / unknown** — any instruction not covered above leaves the
//!    dest as `"any"`.
//!
//! # Flow-insensitivity
//!
//! The pass makes a single forward scan with no phi-node merging.  For code
//! where two branches produce different types for the same variable, the result
//! type is `"any"` because the later assignment overwrites the earlier binding.
//! This is correct and conservative.
//!
//! # Example
//!
//! ```
//! use interpreter_ir::function::IIRFunction;
//! use interpreter_ir::instr::{IIRInstr, Operand};
//! use aot_core::infer::infer_types;
//!
//! let fn_ = IIRFunction::new(
//!     "add",
//!     vec![("a".into(), "u8".into()), ("b".into(), "u8".into())],
//!     "u8",
//!     vec![
//!         IIRInstr::new("add", Some("v0".into()),
//!             vec![Operand::Var("a".into()), Operand::Var("b".into())], "any"),
//!         IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "any"),
//!     ],
//! );
//! let env = infer_types(&fn_);
//! // params seeded directly
//! assert_eq!(env.get("a").map(String::as_str), Some("u8"));
//! assert_eq!(env.get("b").map(String::as_str), Some("u8"));
//! // add of two u8 → u8
//! assert_eq!(env.get("v0").map(String::as_str), Some("u8"));
//! ```

use std::collections::HashMap;
use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};

// ---------------------------------------------------------------------------
// ALLOWED_TYPES allowlist (mirrors specialise.rs)
// ---------------------------------------------------------------------------

/// Type strings that are safe to embed in CIR mnemonics and the type environment.
///
/// This allowlist is intentionally duplicated from `specialise.rs` so that
/// `infer_types()` validates at the point of ingestion — before untrusted
/// strings enter the environment — rather than only at the point of use.
/// Defence-in-depth: any future consumer of the `env` map does not need to
/// re-validate.
const ALLOWED_TYPES: &[&str] = &[
    "u8",  "u16", "u32", "u64",
    "i8",  "i16", "i32", "i64",
    "f32", "f64",
    "bool", "str", "any", "void",
];

// ---------------------------------------------------------------------------
// Numeric rank table for type promotion
// ---------------------------------------------------------------------------

/// Priority order for numeric types.
///
/// When two operands have different numeric types, the wider type wins.
/// Types not in this table (e.g., `"str"`, `"any"`) have no rank and cause
/// the promotion to fall back to `"any"`.
///
/// | Type | Rank |
/// |------|------|
/// | `bool` | 0 (narrowest) |
/// | `u8`   | 1 |
/// | `u16`  | 2 |
/// | `u32`  | 3 |
/// | `u64`  | 4 |
/// | `f64`  | 5 (widest) |
fn numeric_rank(ty: &str) -> Option<u8> {
    match ty {
        "bool" => Some(0),
        "u8"   => Some(1),
        "u16"  => Some(2),
        "u32"  => Some(3),
        "u64"  => Some(4),
        "f64"  => Some(5),
        _      => None,
    }
}

// ---------------------------------------------------------------------------
// Op classification
// ---------------------------------------------------------------------------

fn is_arithmetic_op(op: &str) -> bool {
    matches!(op, "add" | "sub" | "mul" | "div" | "mod")
}

fn is_bitwise_op(op: &str) -> bool {
    matches!(op, "and" | "or" | "xor" | "shl" | "shr")
}

fn is_comparison_op(op: &str) -> bool {
    matches!(op, "cmp_eq" | "cmp_ne" | "cmp_lt" | "cmp_le" | "cmp_gt" | "cmp_ge")
}

fn is_unary_op(op: &str) -> bool {
    matches!(op, "neg" | "not")
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Infer variable types for an `IIRFunction`.
///
/// Returns a `HashMap` from virtual variable name to its inferred IIR type
/// string.  Parameters are always present.  Variables that could not be typed
/// map to `"any"`.
///
/// # Parameters
///
/// - `fn_` — the function to analyse.  Its `params` provide the seed types.
///
/// # Returns
///
/// `HashMap<String, String>` — variable name → inferred type.
pub fn infer_types(fn_: &IIRFunction) -> HashMap<String, String> {
    let mut env: HashMap<String, String> = HashMap::new();

    // 1. Seed with declared parameter types.
    for (name, ty) in &fn_.params {
        env.insert(name.clone(), ty.clone());
    }

    // 2. Walk instructions in order.
    for instr in &fn_.instructions {
        let Some(dest) = &instr.dest else {
            continue;   // no output register → nothing to bind
        };

        if instr.type_hint != "any" {
            // Statically typed instruction — bind directly, but only after
            // validating against the allowlist.  Untrusted IIR (e.g. from a
            // deserialized IIRModule) may carry arbitrary type_hint strings;
            // admitting them would let malicious type strings propagate into
            // the type environment and potentially reach downstream consumers
            // that do not re-validate.  Unknown types fall back to "any".
            let ty = if ALLOWED_TYPES.contains(&instr.type_hint.as_str()) {
                instr.type_hint.clone()
            } else {
                "any".to_string()
            };
            env.insert(dest.clone(), ty);
        } else {
            let t = infer_instr(instr, &env);
            env.insert(dest.clone(), t);
        }
    }

    env
}

// ---------------------------------------------------------------------------
// Per-instruction inference
// ---------------------------------------------------------------------------

fn infer_instr(instr: &IIRInstr, env: &HashMap<String, String>) -> String {
    let op = instr.op.as_str();

    if op == "const" {
        let src = instr.srcs.first();
        return literal_type(src);
    }

    if is_arithmetic_op(op) || is_bitwise_op(op) {
        if instr.srcs.len() < 2 {
            return "any".into();
        }
        let t0 = resolve_operand(&instr.srcs[0], env);
        let t1 = resolve_operand(&instr.srcs[1], env);
        if op == "add" && t0 == "str" && t1 == "str" {
            return "str".into();
        }
        return promote(&t0, &t1);
    }

    if is_comparison_op(op) {
        if instr.srcs.len() < 2 {
            return "any".into();
        }
        let t0 = resolve_operand(&instr.srcs[0], env);
        let t1 = resolve_operand(&instr.srcs[1], env);
        if t0 == "any" || t1 == "any" {
            return "any".into();
        }
        return "bool".into();
    }

    if is_unary_op(op) {
        let src = instr.srcs.first();
        return resolve_operand_opt(src, env);
    }

    "any".into()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Resolve the type of a source `Operand` using the current environment.
fn resolve_operand(src: &Operand, env: &HashMap<String, String>) -> String {
    match src {
        Operand::Bool(_) => "bool".into(),
        Operand::Int(n) => {
            let u = *n as u64;
            if u <= 255 { "u8" }
            else if u <= 65_535 { "u16" }
            else if u <= 4_294_967_295 { "u32" }
            else { "u64" }
        }.into(),
        Operand::Float(_) => "f64".into(),
        Operand::Var(name) => env.get(name).cloned().unwrap_or_else(|| "any".into()),
    }
}

fn resolve_operand_opt(src: Option<&Operand>, env: &HashMap<String, String>) -> String {
    match src {
        None => "any".into(),
        Some(op) => resolve_operand(op, env),
    }
}

/// Infer an IIR type string from a literal `Operand`.
///
/// | Operand | Returned type |
/// |---------|---------------|
/// | `Bool(_)` | `"bool"` |
/// | `Int(0..=255)` | `"u8"` |
/// | `Int(256..=65535)` | `"u16"` |
/// | `Int(65536..=4294967295)` | `"u32"` |
/// | `Int(_)` | `"u64"` |
/// | `Float(_)` | `"f64"` |
/// | `Var(_)` | `"str"` (string literal stored as var name) |
/// | `None` | `"any"` |
pub fn literal_type(src: Option<&Operand>) -> String {
    match src {
        None => "any".into(),
        Some(Operand::Bool(_)) => "bool".into(),
        Some(Operand::Int(n)) => {
            let u = *n as u64;
            if u <= 255 { "u8" }
            else if u <= 65_535 { "u16" }
            else if u <= 4_294_967_295 { "u32" }
            else { "u64" }
        }.into(),
        Some(Operand::Float(_)) => "f64".into(),
        Some(Operand::Var(_)) => "str".into(),
    }
}

/// Return the wider of two numeric types, or `"any"` if incompatible.
///
/// Both `a` and `b` must appear in the numeric rank table; otherwise the
/// promotion is undefined and `"any"` is returned.
///
/// ```
/// use aot_core::infer::promote;
/// assert_eq!(promote("u8", "u16"), "u16");
/// assert_eq!(promote("f64", "u32"), "f64");
/// assert_eq!(promote("str", "u8"), "any");
/// ```
pub fn promote(a: &str, b: &str) -> String {
    if a == "any" || b == "any" {
        return "any".into();
    }
    let ra = numeric_rank(a);
    let rb = numeric_rank(b);
    match (ra, rb) {
        (Some(ra), Some(rb)) => if ra >= rb { a } else { b }.into(),
        _ => "any".into(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::function::IIRFunction;
    use interpreter_ir::instr::{IIRInstr, Operand};

    fn make_fn(
        params: Vec<(&str, &str)>,
        instrs: Vec<IIRInstr>,
    ) -> IIRFunction {
        IIRFunction::new(
            "test",
            params.into_iter().map(|(n, t)| (n.into(), t.into())).collect(),
            "any",
            instrs,
        )
    }

    // ------------------------------------------------------------------
    // promote()
    // ------------------------------------------------------------------

    #[test]
    fn promote_same_type() {
        assert_eq!(promote("u8", "u8"), "u8");
    }

    #[test]
    fn promote_wider_wins() {
        assert_eq!(promote("u8", "u16"), "u16");
        assert_eq!(promote("u32", "f64"), "f64");
    }

    #[test]
    fn promote_incompatible_returns_any() {
        assert_eq!(promote("str", "u8"), "any");
    }

    #[test]
    fn promote_any_input_returns_any() {
        assert_eq!(promote("any", "u8"), "any");
        assert_eq!(promote("u8", "any"), "any");
    }

    #[test]
    fn promote_bool_u8() {
        // bool rank 0 < u8 rank 1
        assert_eq!(promote("bool", "u8"), "u8");
    }

    // ------------------------------------------------------------------
    // literal_type()
    // ------------------------------------------------------------------

    #[test]
    fn literal_type_none() {
        assert_eq!(literal_type(None), "any");
    }

    #[test]
    fn literal_type_bool() {
        assert_eq!(literal_type(Some(&Operand::Bool(true))), "bool");
    }

    #[test]
    fn literal_type_small_int() {
        assert_eq!(literal_type(Some(&Operand::Int(42))), "u8");
    }

    #[test]
    fn literal_type_u16() {
        assert_eq!(literal_type(Some(&Operand::Int(1000))), "u16");
    }

    #[test]
    fn literal_type_u32() {
        assert_eq!(literal_type(Some(&Operand::Int(100_000))), "u32");
    }

    #[test]
    fn literal_type_u64() {
        assert_eq!(literal_type(Some(&Operand::Int(5_000_000_000_i64))), "u64");
    }

    #[test]
    fn literal_type_float() {
        assert_eq!(literal_type(Some(&Operand::Float(3.14))), "f64");
    }

    #[test]
    fn literal_type_var_is_str() {
        assert_eq!(literal_type(Some(&Operand::Var("hello".into()))), "str");
    }

    // ------------------------------------------------------------------
    // infer_types()
    // ------------------------------------------------------------------

    #[test]
    fn params_seeded() {
        let fn_ = make_fn(
            vec![("a", "u8"), ("b", "u16")],
            vec![],
        );
        let env = infer_types(&fn_);
        assert_eq!(env["a"], "u8");
        assert_eq!(env["b"], "u16");
    }

    #[test]
    fn const_instr_inferred_from_literal() {
        let fn_ = make_fn(
            vec![],
            vec![IIRInstr::new("const", Some("x".into()), vec![Operand::Int(7)], "any")],
        );
        let env = infer_types(&fn_);
        assert_eq!(env["x"], "u8");
    }

    #[test]
    fn typed_instr_bound_directly() {
        let fn_ = make_fn(
            vec![],
            vec![IIRInstr::new("add", Some("v".into()), vec![], "i32")],
        );
        let env = infer_types(&fn_);
        assert_eq!(env["v"], "i32");
    }

    #[test]
    fn add_two_u8_params() {
        let fn_ = make_fn(
            vec![("a", "u8"), ("b", "u8")],
            vec![
                IIRInstr::new("add", Some("v0".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())], "any"),
            ],
        );
        let env = infer_types(&fn_);
        assert_eq!(env["v0"], "u8");
    }

    #[test]
    fn add_u8_u16_promotes_to_u16() {
        let fn_ = make_fn(
            vec![("a", "u8"), ("b", "u16")],
            vec![
                IIRInstr::new("add", Some("v0".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())], "any"),
            ],
        );
        let env = infer_types(&fn_);
        assert_eq!(env["v0"], "u16");
    }

    #[test]
    fn cmp_lt_infers_bool() {
        let fn_ = make_fn(
            vec![("a", "u8"), ("b", "u8")],
            vec![
                IIRInstr::new("cmp_lt", Some("v".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())], "any"),
            ],
        );
        let env = infer_types(&fn_);
        assert_eq!(env["v"], "bool");
    }

    #[test]
    fn cmp_with_any_source_infers_any() {
        let fn_ = make_fn(
            vec![],
            vec![
                IIRInstr::new("cmp_eq", Some("v".into()),
                    vec![Operand::Var("x".into()), Operand::Var("y".into())], "any"),
            ],
        );
        let env = infer_types(&fn_);
        assert_eq!(env["v"], "any");
    }

    #[test]
    fn unary_neg_inherits_source_type() {
        let fn_ = make_fn(
            vec![("x", "i32")],
            vec![
                IIRInstr::new("neg", Some("v".into()), vec![Operand::Var("x".into())], "any"),
            ],
        );
        let env = infer_types(&fn_);
        assert_eq!(env["v"], "i32");
    }

    #[test]
    fn add_str_str_is_str() {
        let fn_ = make_fn(
            vec![("a", "str"), ("b", "str")],
            vec![
                IIRInstr::new("add", Some("v".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())], "any"),
            ],
        );
        let env = infer_types(&fn_);
        assert_eq!(env["v"], "str");
    }

    #[test]
    fn no_dest_instr_skipped() {
        let fn_ = make_fn(
            vec![],
            vec![
                IIRInstr::new("ret_void", None, vec![], "void"),
            ],
        );
        let env = infer_types(&fn_);
        // No entries except parameters (none in this case).
        assert!(env.is_empty());
    }
}
