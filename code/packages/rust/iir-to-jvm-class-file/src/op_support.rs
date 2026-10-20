//! LANG83 backend declaration: which standard operations the JVM class-file backend lowers.
//!
//! Every backend crate answers the same question for every entry of
//! [`interpreter_ir::op_registry::STANDARD_OPS`]: *do you lower this operation,
//! or do you refuse it, and why?* The answer is the [`OP_SUPPORT`] table below;
//! `lang-aot`'s `check_module_for` consults it to refuse an unsupported
//! operation **at compile time** with a typed error, instead of letting it reach
//! a lowering arm that might miscompile it.
//!
//! **What "Lowered" means here.** `lower_iir_to_jvm` emits JVM bytecode for the
//! operation (inline instructions, a `java.lang.Math` call, or an
//! `invokestatic env/…` host-runtime call).
//!
//! Three tests keep the table honest:
//!
//! 1. `every_standard_op_is_declared` — every registry entry is declared exactly
//!    once (so adding a registry op fails this crate's CI until it decides);
//! 2. `builtin_rows_agree_with_the_builtin_table` — every Builtin row agrees with
//!    the table the lowering itself consults, so the two cannot drift;
//! 3. `lang-aot/tests/op_conformance.rs` compiles a case for every `Lowered` op
//!    through this backend, which checks the Op rows behaviourally.
//!
//! See `code/specs/LANG83-iir-standard-operation-registry.md`.

use interpreter_ir::op_registry::{support_from_table, OpSupport};
use OpSupport::Lowered;

/// This backend's answer for every LANG83 standard operation.
pub const OP_SUPPORT: &[(&str, OpSupport)] = &[
    // ── Output ──────────────────────────────────────────────────────────────
    ("print_i64", Lowered),
    ("putchar", Lowered),
    ("print_str", Lowered),
    ("write_i64", Lowered),
    // LANG83 PR6a — synthetic `__iir_write_f64(DI)V` (BigDecimal shortest-digit search).
    ("write_f64_py", Lowered),
    ("write_f64_js", Lowered),
    // ── Strings (LANG-FULL E4) ──────────────────────────────────────────────
    ("str_const", Lowered),
    ("str_len", Lowered),
    ("str_index", Lowered),
    ("str_slice", Lowered),
    ("str_concat", Lowered),
    ("str_eq", Lowered),
    ("str_cmp", Lowered),
    // ── Arithmetic ──────────────────────────────────────────────────────────
    ("add", Lowered),
    ("sub", Lowered),
    ("mul", Lowered),
    ("div", Lowered),
    ("mod", Lowered),
    ("neg", Lowered),
    ("i64_div_floor", Lowered),
    ("i64_mod_floor", Lowered),
    // LANG83 PR4 — overflow-checked arithmetic (`Trap.Overflow`).
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
    ("f64_sqrt", Lowered),
    ("f64_sin", Lowered),
    ("f64_cos", Lowered),
    ("f64_ln", Lowered),
    ("f64_exp", Lowered),
    ("f64_atan", Lowered),
    ("f64_tan", Lowered),
    ("f64_pow", Lowered),
    // ── Conversions ─────────────────────────────────────────────────────────
    ("int_to_real", Lowered),
    ("real_to_int_trunc", Lowered),
    ("real_to_int_floor", Lowered),
    // ── Errors ──────────────────────────────────────────────────────────────
    ("trap", Lowered),
];

/// What this backend does with the standard operation `name` (LANG83).
///
/// A name the table does not declare (never a registry entry — the
/// declaration test guarantees that) is refused.
pub fn op_support(name: &str) -> OpSupport {
    support_from_table(OP_SUPPORT, name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::op_registry::{check_op_support_table, OpKind, STANDARD_OPS};

    #[test]
    fn every_standard_op_is_declared() {
        check_op_support_table(OP_SUPPORT).unwrap();
    }

    #[test]
    fn pr1b_ops_are_lowered() {
        for spec in STANDARD_OPS.iter().filter(|s| s.is_pr1b()) {
            assert_eq!(op_support(spec.name), Lowered, "{}", spec.name);
        }
        assert!(!op_support("not_a_standard_op").is_lowered());
    }

    #[test]
    fn builtin_rows_agree_with_the_builtin_table() {
        for spec in STANDARD_OPS.iter().filter(|s| s.kind == OpKind::Builtin) {
            let in_table = crate::validate::CALL_BUILTIN_SUPPORTED_NAMES.contains(&spec.name);
            assert_eq!(
                op_support(spec.name).is_lowered(),
                in_table,
                "{}: OP_SUPPORT and the backend's own builtin table disagree",
                spec.name
            );
        }
    }
}
