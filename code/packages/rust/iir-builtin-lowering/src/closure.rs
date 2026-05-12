//! # closure — Phase 4: closure builtin lowering (LANG34).
//!
//! This module is the fourth (and final) phase of the `iir-builtin-lowering`
//! pipeline.  It rewrites legacy `call_builtin "make_closure"` and
//! `call_builtin "apply_closure"` instructions — emitted by pre-LANG34
//! compilers and hand-built tests — into the first-class LANG34 opcodes:
//!
//! | Legacy form | LANG34 form |
//! |-------------|-------------|
//! | `call_builtin "make_closure" fn_name_reg cap0…` | `alloc_closure(Str(fn_name), cap0…)` |
//! | `call_builtin "apply_closure" handle arg0…` | `call_closure(handle, arg0…)` |
//!
//! ## Why this pass exists
//!
//! LANG34 changed the twig-ir-compiler to emit `alloc_closure`/`call_closure`
//! directly.  However, some IIR modules may have been compiled before LANG34,
//! or may be constructed by hand in tests using the older convention.  This
//! pass automatically upgrades them so the twig-vm dispatcher sees only the
//! canonical LANG34 form.
//!
//! The pass is **infallible** — instructions that cannot be resolved (e.g. a
//! `make_closure` whose `fn_name_reg` was not produced by an identifiable
//! `const`) are left unchanged.  The twig-vm can still execute them via the
//! backward-compatible fallback arms in `exec_call_builtin`.
//!
//! ## Algorithm (two-pass per IIRFunction)
//!
//! ### Pass 1 — build the const-string map
//!
//! Walk all instructions and record every `const` whose source is a string
//! literal.  Two conventions are supported:
//!
//! - Old convention (`Operand::Var(literal)`): used by the pre-LANG32 twig-ir-compiler.
//! - New convention (`Operand::Str(literal)`): used by LANG32+ global/IO lowering.
//!
//! Result: `HashMap<dest_register, literal_text>`.
//!
//! Additionally, track the set of `const` registers that are used *only* as
//! fn_name arguments to `make_closure`.  These are candidates for removal once
//! the `make_closure` is rewritten.  A `const` that is also read by other
//! instructions (arithmetic, `global_set`, etc.) is **not** a candidate —
//! removing it would break those uses.
//!
//! ### Pass 2 — rewrite and filter
//!
//! Walk the instruction list, rewriting and potentially dropping instructions:
//!
//! - `call_builtin "make_closure" fn_name_reg cap0…`:
//!   - Resolve `fn_name_reg` via the const-string map.
//!   - If resolvable: emit `alloc_closure(Str(fn_name), cap0…) : "closure"`.
//!   - If NOT resolvable: leave unchanged.
//!   - Add `fn_name_reg` to the removal-candidate set (used only by this op).
//! - `call_builtin "apply_closure" handle arg0…`:
//!   - Always rewriteable: emit `call_closure(handle, arg0…) : "any"`.
//! - `const` in the removal-candidate set:
//!   - Drop this instruction (it was only used to materialise the fn_name).
//!   - Only dropped if the same register is NOT used anywhere other than
//!     its corresponding `make_closure` (see single-use check).
//! - All other instructions: pass through unchanged.
//!
//! ## Idempotency
//!
//! Running `lower_closure_builtins` on a module that has already been lowered
//! (i.e. one that already uses `alloc_closure`/`call_closure`) is a no-op.
//! The rewrite conditions match only `call_builtin` instructions with the old
//! names; `alloc_closure`/`call_closure` instructions pass through untouched.

use std::collections::{HashMap, HashSet};

use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::function::IIRFunction;
use interpreter_ir::module::IIRModule;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Lower legacy `call_builtin "make_closure"` / `"apply_closure"` instructions
/// in every function of `module` to the LANG34 first-class opcodes.
///
/// Mutates `module` in place.  Infallible: instructions that cannot be lowered
/// are left unchanged.
///
/// Must be called **after** all earlier lowering phases (numeric, heap,
/// global_io) so the instruction list is in its final pre-backend form.
///
/// # Example
///
/// ```rust
/// use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
/// use iir_builtin_lowering::lower_closure_builtins;
///
/// // Build a minimal module with the legacy make_closure form:
/// //   %s0 = const("my_fn")
/// //   %c0 = call_builtin("make_closure", %s0)
/// let fn_ = IIRFunction::new(
///     "main",
///     vec![],
///     "any",
///     vec![
///         IIRInstr::new("const", Some("%s0".into()),
///             vec![Operand::Var("my_fn".into())], "any"),
///         IIRInstr::new("call_builtin", Some("%c0".into()),
///             vec![Operand::Var("make_closure".into()), Operand::Var("%s0".into())],
///             "any"),
///         IIRInstr::new("ret", None, vec![Operand::Var("%c0".into())], "any"),
///     ],
/// );
/// let mut module = IIRModule {
///     name: "test".into(),
///     functions: vec![fn_],
///     entry_point: Some("main".into()),
///     language: "twig".into(),
///     exports: vec![],
///     imports: vec![],
/// };
///
/// lower_closure_builtins(&mut module);
///
/// let instrs = &module.functions[0].instructions;
/// // The const instruction is removed (single-use, only fed make_closure).
/// // The call_builtin is rewritten to alloc_closure.
/// assert_eq!(instrs[0].op, "alloc_closure", "expected alloc_closure, got {}", instrs[0].op);
/// assert!(matches!(&instrs[0].srcs[0], Operand::Str(s) if s == "my_fn"));
/// assert_eq!(instrs[0].type_hint, "closure");
/// ```
pub fn lower_closure_builtins(module: &mut IIRModule) {
    for fn_ in &mut module.functions {
        lower_closure_builtins_function(fn_);
    }
}

// ---------------------------------------------------------------------------
// Per-function implementation
// ---------------------------------------------------------------------------

fn lower_closure_builtins_function(fn_: &mut IIRFunction) {
    // ------------------------------------------------------------------
    // Pass 1a — build the const-string map.
    //
    // Records `dest → literal_text` for every `const` instruction whose
    // source is a string literal (either Operand::Var or Operand::Str
    // convention, since both appear in real programs).
    // ------------------------------------------------------------------
    let mut const_str_map: HashMap<String, String> = HashMap::new();
    for instr in &fn_.instructions {
        if instr.op == "const" {
            if let Some(dest) = &instr.dest {
                let literal = match instr.srcs.first() {
                    Some(Operand::Var(s)) => Some(s.clone()),
                    Some(Operand::Str(s)) => Some(s.clone()),
                    _ => None,
                };
                if let Some(lit) = literal {
                    const_str_map.insert(dest.clone(), lit);
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Pass 1b — identify registers used ONLY as fn_name in make_closure.
    //
    // A `const` register is safe to remove only if:
    //   (a) it appears in const_str_map (it's a string literal), AND
    //   (b) its only consumer is a single `make_closure` call as srcs[1].
    //
    // We count uses: for each register in const_str_map, count how many
    // instructions reference it in their srcs (excluding the `const`
    // itself which produces it).  If the count is exactly 1 AND that one
    // consumer is a `make_closure` call_builtin, the const is removable.
    //
    // Implementation: count all uses first, then mark single-use ones
    // that feed make_closure as removable.
    // ------------------------------------------------------------------
    let mut use_count: HashMap<String, u32> = HashMap::new();
    for instr in &fn_.instructions {
        if instr.op == "const" {
            // Skip the definition instruction itself.
            continue;
        }
        for src in &instr.srcs {
            if let Operand::Var(name) = src {
                if const_str_map.contains_key(name) {
                    *use_count.entry(name.clone()).or_insert(0) += 1;
                }
            }
        }
    }

    // Registers whose sole use is as the fn_name in a make_closure call.
    let mut removable_const_regs: HashSet<String> = HashSet::new();
    for instr in &fn_.instructions {
        if instr.op != "call_builtin" {
            continue;
        }
        let builtin_name = match instr.srcs.first() {
            Some(Operand::Var(s)) => s.as_str(),
            _ => continue,
        };
        if builtin_name != "make_closure" {
            continue;
        }
        // srcs[1] is the fn_name register.
        if let Some(Operand::Var(name_reg)) = instr.srcs.get(1) {
            if const_str_map.contains_key(name_reg) && use_count.get(name_reg) == Some(&1) {
                removable_const_regs.insert(name_reg.clone());
            }
        }
    }

    // ------------------------------------------------------------------
    // Pass 2 — rebuild instruction list.
    //
    // Three rewrites:
    //   1. Drop `const` instructions whose register is in removable_const_regs.
    //   2. Rewrite `call_builtin "make_closure"` → `alloc_closure`.
    //   3. Rewrite `call_builtin "apply_closure"` → `call_closure`.
    // ------------------------------------------------------------------
    let old_instrs = std::mem::take(&mut fn_.instructions);
    let mut new_instrs: Vec<IIRInstr> = Vec::with_capacity(old_instrs.len());

    for instr in old_instrs {
        // Drop removable const instructions.
        if instr.op == "const" {
            if let Some(dest) = &instr.dest {
                if removable_const_regs.contains(dest) {
                    // This const was only used to feed a make_closure —
                    // it's no longer needed after the rewrite.
                    continue;
                }
            }
            new_instrs.push(instr);
            continue;
        }

        // Fast path: only call_builtin instructions are candidates.
        if instr.op != "call_builtin" {
            new_instrs.push(instr);
            continue;
        }

        let builtin_name = match instr.srcs.first() {
            Some(Operand::Var(s)) => s.clone(),
            _ => {
                new_instrs.push(instr);
                continue;
            }
        };

        match builtin_name.as_str() {
            // ------------------------------------------------------------------
            // make_closure fn_name_reg cap0 cap1 …
            //   → alloc_closure(Str(fn_name), cap0, cap1, …) : "closure"
            //
            // srcs layout:
            //   [0] Var("make_closure")  — builtin name
            //   [1] Var(fn_name_reg)     — register holding the fn name
            //   [2..] Var(cap_i)         — capture registers
            //
            // The dest and profiling fields are preserved.
            // ------------------------------------------------------------------
            "make_closure" => {
                let name_reg = match instr.srcs.get(1) {
                    Some(Operand::Var(r)) => r.clone(),
                    _ => {
                        new_instrs.push(instr);
                        continue;
                    }
                };

                let fn_name = match const_str_map.get(&name_reg) {
                    Some(n) => n.clone(),
                    // Cannot resolve name → leave unchanged.
                    None => {
                        new_instrs.push(instr);
                        continue;
                    }
                };

                // Rebuild srcs: Str(fn_name) + original captures from srcs[2..].
                let mut new_srcs: Vec<Operand> = Vec::with_capacity(instr.srcs.len() - 1);
                new_srcs.push(Operand::Str(fn_name));
                for cap in &instr.srcs[2..] {
                    new_srcs.push(cap.clone());
                }

                // Preserve dest and profiling fields; update op, srcs, type_hint.
                let mut new_instr = IIRInstr::new(
                    "alloc_closure",
                    instr.dest.clone(),
                    new_srcs,
                    "closure",
                );
                // Copy profiling state so JIT speculation data is not lost.
                new_instr.observed_slot = instr.observed_slot;
                new_instr.observed_type = instr.observed_type;
                new_instr.observation_count = instr.observation_count;
                new_instr.deopt_anchor = instr.deopt_anchor;
                new_instr.ic_slot = instr.ic_slot;
                new_instr.may_alloc = true;

                new_instrs.push(new_instr);
            }

            // ------------------------------------------------------------------
            // apply_closure handle arg0 arg1 …
            //   → call_closure(handle, arg0, arg1, …) : "any"
            //
            // srcs layout:
            //   [0] Var("apply_closure") — builtin name
            //   [1] Var(handle)          — the closure handle
            //   [2..] Var(arg_i)         — user arguments
            //
            // After LANG34 the handle is srcs[0] in call_closure (shift by 1).
            // ------------------------------------------------------------------
            "apply_closure" => {
                // Must have at least a handle (srcs[1]).
                if instr.srcs.len() < 2 {
                    new_instrs.push(instr);
                    continue;
                }

                // srcs[1..] from old apply_closure become srcs[0..] in call_closure.
                let new_srcs: Vec<Operand> = instr.srcs[1..].to_vec();

                let mut new_instr = IIRInstr::new(
                    "call_closure",
                    instr.dest.clone(),
                    new_srcs,
                    "any",
                );
                // Preserve profiling state.
                new_instr.observed_slot = instr.observed_slot;
                new_instr.observed_type = instr.observed_type;
                new_instr.observation_count = instr.observation_count;
                new_instr.deopt_anchor = instr.deopt_anchor;
                new_instr.ic_slot = instr.ic_slot;

                new_instrs.push(new_instr);
            }

            _ => {
                new_instrs.push(instr);
            }
        }
    }

    fn_.instructions = new_instrs;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
    use interpreter_ir::function::FunctionTypeStatus;

    fn make_module(instrs: Vec<IIRInstr>) -> IIRModule {
        let fn_ = IIRFunction {
            name: "main".into(),
            params: vec![],
            return_type: "any".into(),
            register_count: instrs.len() + 4,
            instructions: instrs,
            type_status: FunctionTypeStatus::Untyped,
            call_count: 0,
            feedback_slots: std::collections::HashMap::new(),
            source_map: vec![],
            param_refinements: Vec::new(),
            return_refinement: None,
        };
        IIRModule {
            name: "test".into(),
            functions: vec![fn_],
            entry_point: Some("main".into()),
            language: "twig".into(),
            exports: vec![],
            imports: vec![],
        }
    }

    // ── make_closure → alloc_closure ─────────────────────────────────────

    /// Zero captures: const + make_closure → alloc_closure (const removed).
    #[test]
    fn make_closure_zero_captures_is_rewritten() {
        let mut m = make_module(vec![
            IIRInstr::new("const", Some("%s0".into()),
                vec![Operand::Var("my_fn".into())], "any"),
            IIRInstr::new("call_builtin", Some("%c0".into()),
                vec![
                    Operand::Var("make_closure".into()),
                    Operand::Var("%s0".into()),
                ],
                "any"),
            IIRInstr::new("ret", None, vec![Operand::Var("%c0".into())], "any"),
        ]);

        lower_closure_builtins(&mut m);

        let instrs = &m.functions[0].instructions;
        // const must be removed; first instruction is now alloc_closure.
        assert_eq!(instrs.len(), 2, "expected 2 instructions (alloc_closure + ret)");
        assert_eq!(instrs[0].op, "alloc_closure");
        assert!(matches!(&instrs[0].srcs[0], Operand::Str(s) if s == "my_fn"));
        assert!(instrs[0].srcs.len() == 1, "zero captures → 1 src");
        assert_eq!(instrs[0].type_hint, "closure");
        assert_eq!(instrs[0].dest.as_deref(), Some("%c0"));
    }

    /// Two captures: const + make_closure(cap0, cap1) → alloc_closure(Str, cap0, cap1).
    #[test]
    fn make_closure_two_captures_is_rewritten() {
        let mut m = make_module(vec![
            IIRInstr::new("const", Some("%s0".into()),
                vec![Operand::Var("add_fn".into())], "any"),
            IIRInstr::new("call_builtin", Some("%c0".into()),
                vec![
                    Operand::Var("make_closure".into()),
                    Operand::Var("%s0".into()),
                    Operand::Var("x".into()),
                    Operand::Var("y".into()),
                ],
                "any"),
            IIRInstr::new("ret", None, vec![Operand::Var("%c0".into())], "any"),
        ]);

        lower_closure_builtins(&mut m);

        let instrs = &m.functions[0].instructions;
        // const removed, alloc_closure should have 3 srcs: Str + 2 caps.
        assert_eq!(instrs.len(), 2);
        assert_eq!(instrs[0].op, "alloc_closure");
        assert!(matches!(&instrs[0].srcs[0], Operand::Str(s) if s == "add_fn"));
        assert_eq!(instrs[0].srcs.len(), 3);
        assert_eq!(instrs[0].srcs[1], Operand::Var("x".into()));
        assert_eq!(instrs[0].srcs[2], Operand::Var("y".into()));
    }

    /// A const used by both make_closure AND another instruction must NOT be removed.
    #[test]
    fn const_with_multiple_uses_is_preserved() {
        let mut m = make_module(vec![
            // %s0 is used by BOTH make_closure (srcs[1]) and ret (srcs[0]).
            IIRInstr::new("const", Some("%s0".into()),
                vec![Operand::Var("fn_name".into())], "any"),
            IIRInstr::new("call_builtin", Some("%c0".into()),
                vec![
                    Operand::Var("make_closure".into()),
                    Operand::Var("%s0".into()),
                ],
                "any"),
            // Second use of %s0 — prevents const removal.
            IIRInstr::new("ret", None, vec![Operand::Var("%s0".into())], "any"),
        ]);

        lower_closure_builtins(&mut m);

        let instrs = &m.functions[0].instructions;
        // const must be retained (3 instructions total).
        assert_eq!(instrs.len(), 3, "const with multiple uses must not be removed");
        assert_eq!(instrs[0].op, "const");
        assert_eq!(instrs[1].op, "alloc_closure");
    }

    /// make_closure whose fn_name_reg is not a const literal is left unchanged.
    #[test]
    fn unresolvable_make_closure_is_left_unchanged() {
        let mut m = make_module(vec![
            // %name is a function parameter, not a const → not in const_str_map.
            IIRInstr::new("call_builtin", Some("%c0".into()),
                vec![
                    Operand::Var("make_closure".into()),
                    Operand::Var("dynamic_name".into()),
                ],
                "any"),
            IIRInstr::new("ret", None, vec![Operand::Var("%c0".into())], "any"),
        ]);

        lower_closure_builtins(&mut m);

        // Instruction must be unchanged.
        assert_eq!(m.functions[0].instructions[0].op, "call_builtin");
    }

    // ── apply_closure → call_closure ─────────────────────────────────────

    /// apply_closure(handle, arg) → call_closure(handle, arg).
    #[test]
    fn apply_closure_is_rewritten() {
        let mut m = make_module(vec![
            IIRInstr::new("call_builtin", Some("%r".into()),
                vec![
                    Operand::Var("apply_closure".into()),
                    Operand::Var("clos_handle".into()),
                    Operand::Int(42),
                ],
                "any"),
            IIRInstr::new("ret", None, vec![Operand::Var("%r".into())], "any"),
        ]);

        lower_closure_builtins(&mut m);

        let instrs = &m.functions[0].instructions;
        assert_eq!(instrs[0].op, "call_closure");
        // srcs[0] is the handle (was srcs[1] in apply_closure).
        assert_eq!(instrs[0].srcs[0], Operand::Var("clos_handle".into()));
        assert_eq!(instrs[0].srcs[1], Operand::Int(42));
        assert_eq!(instrs[0].type_hint, "any");
    }

    /// apply_closure with no args (no user args, just handle).
    #[test]
    fn apply_closure_no_args_is_rewritten() {
        let mut m = make_module(vec![
            IIRInstr::new("call_builtin", Some("%r".into()),
                vec![
                    Operand::Var("apply_closure".into()),
                    Operand::Var("thunk".into()),
                ],
                "any"),
            IIRInstr::new("ret", None, vec![Operand::Var("%r".into())], "any"),
        ]);

        lower_closure_builtins(&mut m);

        let instrs = &m.functions[0].instructions;
        assert_eq!(instrs[0].op, "call_closure");
        assert_eq!(instrs[0].srcs.len(), 1);
        assert_eq!(instrs[0].srcs[0], Operand::Var("thunk".into()));
    }

    // ── Mixed and idempotency ─────────────────────────────────────────────

    /// Module with both make_closure and apply_closure is lowered in one pass.
    #[test]
    fn both_forms_lowered_in_one_pass() {
        let mut m = make_module(vec![
            IIRInstr::new("const", Some("%s0".into()),
                vec![Operand::Var("inner".into())], "any"),
            IIRInstr::new("call_builtin", Some("%c0".into()),
                vec![
                    Operand::Var("make_closure".into()),
                    Operand::Var("%s0".into()),
                ],
                "any"),
            IIRInstr::new("call_builtin", Some("%r0".into()),
                vec![
                    Operand::Var("apply_closure".into()),
                    Operand::Var("%c0".into()),
                    Operand::Int(5),
                ],
                "any"),
            IIRInstr::new("ret", None, vec![Operand::Var("%r0".into())], "any"),
        ]);

        lower_closure_builtins(&mut m);

        let instrs = &m.functions[0].instructions;
        // const removed + make_closure → alloc_closure + apply_closure → call_closure + ret
        assert_eq!(instrs.len(), 3, "expected [alloc_closure, call_closure, ret]");
        assert_eq!(instrs[0].op, "alloc_closure");
        assert_eq!(instrs[1].op, "call_closure");
        assert_eq!(instrs[2].op, "ret");
    }

    /// Running lower_closure_builtins twice is a no-op (idempotent).
    #[test]
    fn lowering_is_idempotent() {
        let mut m = make_module(vec![
            IIRInstr::new("const", Some("%s0".into()),
                vec![Operand::Var("fn_a".into())], "any"),
            IIRInstr::new("call_builtin", Some("%c0".into()),
                vec![
                    Operand::Var("make_closure".into()),
                    Operand::Var("%s0".into()),
                ],
                "any"),
            IIRInstr::new("ret", None, vec![Operand::Var("%c0".into())], "any"),
        ]);

        lower_closure_builtins(&mut m);
        let snapshot1: Vec<String> = m.functions[0].instructions.iter()
            .map(|i| i.op.clone()).collect();

        lower_closure_builtins(&mut m);
        let snapshot2: Vec<String> = m.functions[0].instructions.iter()
            .map(|i| i.op.clone()).collect();

        assert_eq!(snapshot1, snapshot2, "second pass must be a no-op");
    }

    /// A module that already uses alloc_closure/call_closure is unchanged.
    #[test]
    fn already_lowered_module_is_unchanged() {
        let mut m = make_module(vec![
            IIRInstr::new("alloc_closure", Some("%c0".into()),
                vec![Operand::Str("my_fn".into())], "closure"),
            IIRInstr::new("call_closure", Some("%r0".into()),
                vec![Operand::Var("%c0".into()), Operand::Int(1)], "any"),
            IIRInstr::new("ret", None, vec![Operand::Var("%r0".into())], "any"),
        ]);

        lower_closure_builtins(&mut m);

        let instrs = &m.functions[0].instructions;
        assert_eq!(instrs.len(), 3);
        assert_eq!(instrs[0].op, "alloc_closure");
        assert_eq!(instrs[1].op, "call_closure");
    }
}
