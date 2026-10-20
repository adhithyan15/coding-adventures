//! LANG83 backend declaration: which standard operations the VM executes.
//!
//! Every backend crate answers the same question for every entry of
//! [`interpreter_ir::op_registry::STANDARD_OPS`]: *do you lower this operation,
//! or do you refuse it (and why)?* Most backends answer with a hand-written
//! table checked against their own code by tests. The VM can do better: it
//! already owns the two authoritative answers, so its declaration is
//! **derived**, not listed:
//!
//! | Registry kind | The VM executes it iff… |
//! |---------------|--------------------------|
//! | Op (`instr.op == name`) | the standard dispatch table (`dispatch::lookup_standard`) has a handler |
//! | Builtin (`call_builtin name`) | the default [`BuiltinRegistry`] registers it |
//!
//! So when PR1b adds an `i64_div_floor` handler to the dispatch table, this
//! module's answer flips to `Lowered` with no edit here — there is no second
//! list to forget.

use interpreter_ir::op_registry::{
    lookup, OpKind, OpSupport, NOT_YET_LOWERED_PR1B, STANDARD_OPS,
};

use crate::builtins::BuiltinRegistry;
use crate::dispatch::lookup_standard;

/// The reason given for a registry op the VM neither dispatches nor registers
/// by default (and that is not a PR1b op, which gets the shared PR1b reason).
pub const NO_VM_HANDLER: &str =
    "vm-core has no dispatch handler or default builtin for this operation";

/// What `vm-core` does with the standard operation `name` (LANG83).
///
/// ```
/// use interpreter_ir::op_registry::OpSupport;
/// use vm_core::op_support::op_support;
///
/// assert_eq!(op_support("f64_sin"), OpSupport::Lowered);   // dispatch handler
/// assert_eq!(op_support("print_i64"), OpSupport::Lowered); // default builtin
/// assert!(!op_support("i64_div_floor").is_lowered());       // PR1b
/// ```
pub fn op_support(name: &str) -> OpSupport {
    let Some(spec) = lookup(name) else {
        return OpSupport::Refused("not a LANG83 standard operation");
    };
    let executes = match spec.kind {
        OpKind::Op => lookup_standard(name).is_some(),
        OpKind::Builtin => BuiltinRegistry::new().has(name),
    };
    if executes {
        OpSupport::Lowered
    } else if spec.is_pr1b() {
        OpSupport::Refused(NOT_YET_LOWERED_PR1B)
    } else {
        OpSupport::Refused(NO_VM_HANDLER)
    }
}

/// The VM's full `OP_SUPPORT` table, derived from [`op_support`] for every
/// registry entry (the other backends publish theirs as a `const`).
pub fn op_support_table() -> Vec<(&'static str, OpSupport)> {
    STANDARD_OPS.iter().map(|s| (s.name, op_support(s.name))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::op_registry::check_op_support_table;

    #[test]
    fn every_standard_op_is_declared() {
        check_op_support_table(&op_support_table()).unwrap();
    }

    #[test]
    fn every_existing_standard_op_is_executed_and_pr1b_ops_are_refused() {
        // Pins today's answer so a regression (a handler removed from the
        // dispatch table, a default builtin dropped) is loud.
        for spec in STANDARD_OPS {
            let support = op_support(spec.name);
            if spec.is_pr1b() {
                assert_eq!(support, OpSupport::Refused(NOT_YET_LOWERED_PR1B), "{}", spec.name);
            } else {
                assert_eq!(support, OpSupport::Lowered, "{}", spec.name);
            }
        }
    }

    #[test]
    fn non_registry_names_are_refused() {
        assert!(!op_support("jmp").is_lowered());
        assert!(!op_support("no_such_op").is_lowered());
    }
}
