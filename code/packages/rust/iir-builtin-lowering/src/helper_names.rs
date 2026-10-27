//! # helper_names — a pass's helper function is the pass's, or the module is refused
//!
//! Two passes in this crate add functions of their own to the module they
//! lower and then point the program's calls at them **by name**:
//!
//! | pass                      | helper(s) it adds                                   | the calls it rewrites      |
//! |---------------------------|-----------------------------------------------------|----------------------------|
//! | [`crate::list_ops`]       | `__dyn_list_length`, `__dyn_list_ref`, `__dyn_list_append`, `__dyn_list_reverse`, `__dyn_list_assoc` | `length`, `list-ref`, `append`, `reverse`, `assoc` |
//! | [`crate::closure_heap`]   | `__dyn_call_closure` (the closure dispatcher)       | every `call_closure`       |
//!
//! Both used to add the helper only "if no function of that name exists yet".
//! The point was idempotence: running the pass twice must not add a second
//! copy. The cost was that a module which *already* held some other function
//! under the helper's name kept it, and every rewritten call went to that
//! function instead:
//!
//! ```text
//! fn __dyn_list_length(l) { ret 99 }      ; written by the program, not the pass
//! fn main() { r = call_builtin length, xs }
//!
//!   lower_list_ops ──▶  r = call __dyn_list_length, xs   ; → 99, silently
//! ```
//!
//! Twig refuses `__dyn_` names in source (`twig-ir-compiler`'s
//! `reserved_names`), but these passes run on IIR from every frontend, so the
//! pass has to defend its own names. That is the rule of
//! [`crate::lisp_semantics::ReservedNameError`], applied here:
//!
//! | the module already has a function named like the helper … | result                      |
//! |-------------------------------------------------------------|-----------------------------|
//! | … no                                                        | the pass adds its helper    |
//! | … yes, and it *is* the helper, instruction for instruction  | kept (the pass ran before)  |
//! | … yes, and it is anything else                              | [`HelperNameError`]         |
//!
//! The check runs only when the pass is about to rewrite calls to point at
//! the helper, and before it rewrites anything. A module that defines a
//! function with a helper's name but never uses the operation is left alone:
//! nothing is pointed at it, so there is nothing to take over. (Another
//! stage may still refuse the name: the native pipeline refuses a function
//! named like a runtime symbol.)
//!
//! "Instruction for instruction" compares against a freshly built helper, so
//! it recognises the helper exactly as the pass added it. Later passes in a
//! pipeline (the heap lowering rewrites the helper's own `car`/`cdr`) change
//! it, and a changed copy would be refused. That is not a problem in
//! practice. Once a pass has run, the calls it rewrites are gone
//! (`call_builtin length` became `call __dyn_list_length`; `call_closure`
//! became a call of the dispatcher), so running it again finds nothing to
//! do and never reaches the check.

use interpreter_ir::function::IIRFunction;
use interpreter_ir::IIRModule;

/// A module already defines a function under the name a pass gives its own
/// helper, and that function is not the helper.
///
/// The pass would otherwise have kept the module's function and routed the
/// operation it lowers (`length`, a closure call, …) to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelperNameError {
    /// The helper's name, which the module already uses (`"__dyn_list_length"`).
    pub name: String,
    /// The pass that owns the name (`"lower_list_ops"`, `"lower_closures_to_heap"`).
    pub pass: &'static str,
}

impl std::fmt::Display for HelperNameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the program defines a function named {:?}, which is reserved for the \
             compiler: `{}` generates a helper of that name and would have called the \
             program's function in its place",
            self.name, self.pass
        )
    }
}

impl std::error::Error for HelperNameError {}

/// Is `f` the very function `want` — same parameters, return type and
/// instructions? (`IIRFunction` has no `PartialEq`; the instruction fields
/// compared are the ones that change what the code does.)
pub(crate) fn is_same_function(f: &IIRFunction, want: &IIRFunction) -> bool {
    f.name == want.name
        && f.params == want.params
        && f.return_type == want.return_type
        && f.instructions.len() == want.instructions.len()
        && f.instructions.iter().zip(&want.instructions).all(|(a, b)| {
            a.op == b.op
                && a.dest == b.dest
                && a.srcs == b.srcs
                && a.type_hint == b.type_hint
                && a.may_alloc == b.may_alloc
        })
}

/// Before `pass` points calls at `helper`: refuse a module that holds a
/// different function under `helper`'s name.
///
/// `Ok(true)` means the module already holds the helper itself (the pass has
/// run before and must not add a second copy); `Ok(false)` means the name is
/// free and the pass should add it.
pub(crate) fn claim_helper_name(
    module: &IIRModule,
    helper: &IIRFunction,
    pass: &'static str,
) -> Result<bool, HelperNameError> {
    match module.functions.iter().find(|f| f.name == helper.name) {
        None => Ok(false),
        Some(f) if is_same_function(f, helper) => Ok(true),
        Some(_) => Err(HelperNameError { name: helper.name.clone(), pass }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::instr::{IIRInstr, Operand};

    fn ret_const(name: &str, n: i64) -> IIRFunction {
        IIRFunction::new(
            name,
            vec![("l".into(), "ref<any>".into())],
            "ref<any>",
            vec![
                IIRInstr::new("const", Some("k".into()), vec![Operand::Int(n)], "i64"),
                IIRInstr::new("ret", None, vec![Operand::Var("k".into())], "i64"),
            ],
        )
    }

    fn module(functions: Vec<IIRFunction>) -> IIRModule {
        let mut m = IIRModule::new("m", "test");
        m.functions = functions;
        m
    }

    #[test]
    fn a_free_name_is_claimed() {
        let helper = ret_const("__dyn_h", 1);
        assert_eq!(claim_helper_name(&module(vec![]), &helper, "p"), Ok(false));
    }

    #[test]
    fn the_helper_itself_is_recognised() {
        let helper = ret_const("__dyn_h", 1);
        let m = module(vec![helper.clone()]);
        assert_eq!(claim_helper_name(&m, &helper, "p"), Ok(true));
    }

    #[test]
    fn any_other_function_under_the_name_is_refused() {
        let helper = ret_const("__dyn_h", 1);
        // Same name, one constant different: an impostor.
        let m = module(vec![ret_const("__dyn_h", 99)]);
        let err = claim_helper_name(&m, &helper, "lower_list_ops").unwrap_err();
        assert_eq!(err, HelperNameError { name: "__dyn_h".into(), pass: "lower_list_ops" });
        let text = err.to_string();
        assert!(text.contains("\"__dyn_h\"") && text.contains("lower_list_ops"), "{text}");
    }
}
