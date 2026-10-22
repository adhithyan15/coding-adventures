//! LANG83 backend declaration: which standard operations the generic JIT
//! compiles.
//!
//! Every backend crate answers the same question for every entry of
//! [`interpreter_ir::op_registry::STANDARD_OPS`]: *do you lower this operation,
//! or do you refuse it, and why?*
//!
//! # The JIT is judged without its safety net
//!
//! `JITCore` runs a program on two tiers: functions `GenericCirJit` can compile
//! run as JIT bytecode, and everything else silently falls back to the
//! `vm-core` interpreter. That fallback is a feature for users, but it would make
//! a dishonest declaration: "the JIT column passes" would be true for *every*
//! operation the interpreter knows, whether or not the JIT compiles it.
//!
//! So `Lowered` here means exactly one thing: **`GenericCirJit` compiles the
//! operation to JIT bytecode.** An operation that only runs because of the
//! interpreter fallback is declared `Refused`, with a reason that says so:
//!
//! | Operation | GenericCirJit | Declared |
//! |-----------|---------------|----------|
//! | `add sub mul div neg` (integer) | `ADD_I64` … `NEG_I64` | Lowered |
//! | `cmp_*` (integer / bool) | `CMP_*_I64` | Lowered |
//! | `print_i64`, `putchar`, `write_i64` | `CALL_BUILTIN` to the embedder-registered handler | Lowered |
//! | `write_f64_py`, `write_f64_js` | `CALL_BUILTIN_F64` (the float's bits, handed over as `Value::Float`) to the embedder-registered handler; `const_f64` compiles to the bits | Lowered |
//! | `i64_div_floor`, `i64_mod_floor` | `DIV_FLOOR_I64`, `MOD_FLOOR_I64` (defined traps) | Lowered |
//! | `trap` | `TRAP <kind id>` (error slot `trap: <kind>`) | Lowered |
//! | `i64_add_ovf`, `i64_sub_ovf`, `i64_mul_ovf`, `i64_neg_ovf` | `ADD_OVF_I64` … `NEG_OVF_I64` (`checked_*`; `Trap.Overflow`) | Lowered |
//! | `mod`, strings, `f64` math, conversions, `print_str` | — | Refused (interpreter tier only) |
//!
//! Like every backend's table, this one is name-granular: `add` is `Lowered`
//! even though the JIT refuses `add` on `f64` operands (type-level refusals stay
//! with the compiler). The `declared_support_matches_what_generic_jit_compiles`
//! test checks every row against the real compiler: it builds a one-op function
//! per registry entry, specialises it, and asks `GenericCirJit` to compile it.

use interpreter_ir::op_registry::{support_from_table, OpSupport};
use OpSupport::{Lowered, Refused};

/// The reason given for an operation that runs only on the interpreter tier.
pub const INTERPRETER_TIER_ONLY: &str = "GenericCirJit does not compile this operation; a \
     function using it runs on the vm-core interpreter tier, not as JIT code";

/// The generic JIT's answer for every LANG83 standard operation.
pub const OP_SUPPORT: &[(&str, OpSupport)] = &[
    // ── Output ──────────────────────────────────────────────────────────────
    ("print_i64", Lowered),
    ("putchar", Lowered),
    ("print_str", Refused(INTERPRETER_TIER_ONLY)),
    ("write_i64", Lowered),
    // LANG83 PR6a — the embedder formats (lang-aot's harness calls
    // `vm_core::float_format`, the same code as the VM's default builtins).
    ("write_f64_py", Lowered),
    ("write_f64_js", Lowered),
    // ── Strings (LANG-FULL E4) ──────────────────────────────────────────────
    ("str_const", Refused(INTERPRETER_TIER_ONLY)),
    ("str_len", Refused(INTERPRETER_TIER_ONLY)),
    ("str_index", Refused(INTERPRETER_TIER_ONLY)),
    ("str_slice", Refused(INTERPRETER_TIER_ONLY)),
    ("str_concat", Refused(INTERPRETER_TIER_ONLY)),
    ("str_eq", Refused(INTERPRETER_TIER_ONLY)),
    ("str_cmp", Refused(INTERPRETER_TIER_ONLY)),
    // ── Arithmetic ──────────────────────────────────────────────────────────
    ("add", Lowered),
    ("sub", Lowered),
    ("mul", Lowered),
    ("div", Lowered),
    ("mod", Refused(INTERPRETER_TIER_ONLY)),
    ("neg", Lowered),
    ("i64_div_floor", Lowered),
    ("i64_mod_floor", Lowered),
    ("i64_add_ovf", Lowered),
    ("i64_sub_ovf", Lowered),
    ("i64_mul_ovf", Lowered),
    ("i64_neg_ovf", Lowered),
    // ── Comparison ──────────────────────────────────────────────────────────
    ("cmp_eq", Lowered),
    ("cmp_ne", Lowered),
    ("cmp_lt", Lowered),
    ("cmp_le", Lowered),
    ("cmp_gt", Lowered),
    ("cmp_ge", Lowered),
    // ── Float math ──────────────────────────────────────────────────────────
    ("f64_sqrt", Refused(INTERPRETER_TIER_ONLY)),
    ("f64_sin", Refused(INTERPRETER_TIER_ONLY)),
    ("f64_cos", Refused(INTERPRETER_TIER_ONLY)),
    ("f64_ln", Refused(INTERPRETER_TIER_ONLY)),
    ("f64_exp", Refused(INTERPRETER_TIER_ONLY)),
    ("f64_atan", Refused(INTERPRETER_TIER_ONLY)),
    ("f64_tan", Refused(INTERPRETER_TIER_ONLY)),
    ("f64_pow", Refused(INTERPRETER_TIER_ONLY)),
    // ── Conversions ─────────────────────────────────────────────────────────
    ("int_to_real", Refused(INTERPRETER_TIER_ONLY)),
    ("real_to_int_trunc", Refused(INTERPRETER_TIER_ONLY)),
    ("real_to_int_floor", Refused(INTERPRETER_TIER_ONLY)),
    // ── Errors ──────────────────────────────────────────────────────────────
    ("trap", Lowered),
];

/// What the generic JIT does with the standard operation `name` (LANG83).
///
/// ```
/// use interpreter_ir::op_registry::OpSupport;
/// use jit_core::op_support::op_support;
///
/// assert_eq!(op_support("add"), OpSupport::Lowered);
/// assert!(!op_support("mod").is_lowered()); // interpreter tier only
/// ```
pub fn op_support(name: &str) -> OpSupport {
    support_from_table(OP_SUPPORT, name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{Backend, FunctionContext};
    use crate::generic_jit::GenericCirJit;
    use crate::specialise::specialise;
    use interpreter_ir::op_registry::{check_op_support_table, OpFamily, OpKind, OpSpec, STANDARD_OPS};
    use interpreter_ir::{IIRFunction, IIRInstr, Operand};
    use vm_core::value::Value;

    #[test]
    fn every_standard_op_is_declared() {
        check_op_support_table(OP_SUPPORT).unwrap();
    }

    /// The IIR type of one registry operand description.
    fn param_type(desc: &str) -> &'static str {
        match desc {
            "f64" => "f64",
            "str" => "str",
            _ => "i64",
        }
    }

    /// The `type_hint` a one-op test function gives the op's result.
    fn result_type(spec: &OpSpec) -> &'static str {
        match (spec.family, spec.name) {
            (OpFamily::Comparison, _) => "bool",
            (OpFamily::Float, _) | (_, "int_to_real") => "f64",
            (_, "str_const" | "str_concat" | "str_slice") => "str",
            _ => "i64",
        }
    }

    /// One function that does nothing but run `spec` — the smallest program
    /// that can only be JIT-compiled if the op itself can.
    fn one_op_function(spec: &OpSpec) -> IIRFunction {
        let mut params = Vec::new();
        let mut srcs = Vec::new();
        for (i, desc) in spec.operand_types.iter().enumerate() {
            match *desc {
                "literal" => srcs.push(Operand::Str("x".into())),
                "kind" => srcs.push(Operand::Str("Trap.User".into())),
                other => {
                    let name = format!("p{i}");
                    params.push((name.clone(), param_type(other).to_string()));
                    srcs.push(Operand::Var(name));
                }
            }
        }
        let (op, dest, ty) = match spec.kind {
            OpKind::Builtin => {
                srcs.insert(0, Operand::Var(spec.name.into()));
                ("call_builtin", None, "void")
            }
            OpKind::Op if spec.dest => (spec.name, Some("r".to_string()), result_type(spec)),
            OpKind::Op => (spec.name, None, "void"),
        };
        let mut body = vec![IIRInstr::new(op, dest.clone(), srcs, ty)];
        let ret_type = match dest {
            Some(r) => {
                body.push(IIRInstr::new("ret", None, vec![Operand::Var(r)], ty));
                ty
            }
            None => {
                body.push(IIRInstr::new("ret_void", None, vec![], "void"));
                "void"
            }
        };
        IIRFunction::new("f", params, ret_type, body)
    }

    #[test]
    fn declared_support_matches_what_generic_jit_compiles() {
        let jit = GenericCirJit::new();
        // The embedder registers the output builtins (the lang-aot matrix and
        // conformance harnesses do), exactly as for print_i64 and putchar.
        jit.register_builtin("print_i64", |_args: &[Value]| Value::Null);
        jit.register_builtin("putchar", |_args: &[Value]| Value::Null);
        jit.register_builtin("write_i64", |_args: &[Value]| Value::Null);
        jit.register_builtin("write_f64_py", |_args: &[Value]| Value::Null);
        jit.register_builtin("write_f64_js", |_args: &[Value]| Value::Null);
        for spec in STANDARD_OPS {
            let func = one_op_function(spec);
            let cir = specialise(&func, 0);
            // Compile WITH the parameter list, as JITCore does, so an `f64`
            // parameter is known to hold a float (LANG83 PR6a).
            let ctx = FunctionContext { name: "f", params: &func.params, return_type: &func.return_type };
            let compiles = jit.compile_function(&ctx, &cir).is_some();
            assert_eq!(
                compiles,
                op_support(spec.name).is_lowered(),
                "{}: OP_SUPPORT says {:?} but GenericCirJit {} it (CIR: {:?})",
                spec.name,
                op_support(spec.name),
                if compiles { "compiles" } else { "refuses" },
                cir.iter().map(|c| c.op.as_str()).collect::<Vec<_>>()
            );
        }
    }
}
