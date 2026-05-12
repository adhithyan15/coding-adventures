//! # global_io — Phase 3: global variable and I/O builtin lowering (LANG32).
//!
//! This module lowers three `call_builtin` families that the twig-ir-compiler
//! emits for module-level variables and standard output:
//!
//! | Builtin        | Lowered to      | Notes |
//! |----------------|-----------------|-------|
//! | `global_set`   | `global_store`  | requires look-back for name resolution |
//! | `global_get`   | `global_load`   | requires look-back for name resolution |
//! | `print`        | `io_out`        | direct 1-to-1 rewrite |
//!
//! ## Name-resolution look-back
//!
//! The twig-ir-compiler does NOT encode the global variable name directly in
//! the `call_builtin` operand list.  Instead it emits a separate `const`
//! instruction that loads the name as a string literal, then passes the
//! resulting register to `call_builtin "global_set"`:
//!
//! ```text
//! const  %n1 = Operand::Var("x")     -- string literal, NOT a register ref
//! call_builtin "global_set", %n1, %v  -- %n1 holds the name
//! ```
//!
//! To recover the name at compile time we must look *back* at the `const`
//! instructions that precede each `call_builtin`.  We do this in two passes:
//!
//! **Pass 1** — build `const_str_map: HashMap<register, literal_text>`.
//! Walk all instructions and record every `const %reg = Operand::Var("text")`
//! entry.  This covers the "string-as-Var" convention used throughout the
//! Twig IR compiler for global names, lambda names, and symbol names.
//!
//! **Pass 2** — rewrite matching `call_builtin` instructions.
//! For each `global_set`/`global_get`/`print`, look up the name register in
//! the map, then emit the corresponding `global_store`/`global_load`/`io_out`.
//!
//! Instructions that cannot be resolved (name register not in the map,
//! missing operands) are left unchanged so the backend validator can emit a
//! clear error with full context.
//!
//! ## Operand encoding for global_load / global_store
//!
//! Unlike heap or arithmetic opcodes, globals need a compile-time string name.
//! We use the new `Operand::Str(String)` variant (LANG32) so backends can
//! distinguish the literal name from a register reference:
//!
//! ```text
//! global_store : srcs = [Str("x"), Var("%v")]
//! global_load  : srcs = [Str("x")]            dest = Some("%r")
//! io_out       : srcs = [Var("%val")]
//! ```

use std::collections::HashMap;

use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::function::IIRFunction;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Lower global and I/O `call_builtin` instructions in `fn_` to typed ops.
///
/// Runs in two passes (see module-level doc).  Malformed instructions and
/// those whose name register cannot be resolved are left unchanged.
pub fn lower_global_io_function(fn_: &mut IIRFunction) {
    // ------------------------------------------------------------------
    // Pass 1 — collect const-string definitions.
    //
    // `const_str_map[reg] = literal` for all instructions of the form:
    //   const  %reg = Operand::Var("literal_text")
    //
    // This is the twig-ir-compiler's convention for embedding string
    // literals (global names, lambda names) in the instruction stream.
    // The `Operand::Var("text")` wrapping is confusingly named but is
    // intentional — the vm-core `exec_const` handler interns the text
    // as a symbol.  We unwrap it here at compile time.
    // ------------------------------------------------------------------
    let mut const_str_map: HashMap<String, String> = HashMap::new();

    for instr in &fn_.instructions {
        if instr.op == "const" {
            if let (Some(dest), Some(Operand::Var(literal))) =
                (&instr.dest, instr.srcs.first())
            {
                // Only record entries that look like string literals.
                // A register reference that happens to look like a name
                // (e.g. "%r0") is excluded by the presence of the `const`
                // opcode — real registers don't appear in `const` srcs.
                const_str_map.insert(dest.clone(), literal.clone());
            }
        }
    }

    // ------------------------------------------------------------------
    // Pass 2 — rewrite call_builtin "global_set" / "global_get" / "print".
    //
    // We rebuild the instruction list rather than mutating in-place
    // (same rationale as heap.rs: can't grow a Vec while iterating it).
    // ------------------------------------------------------------------
    let old_instrs = std::mem::take(&mut fn_.instructions);
    let mut new_instrs: Vec<IIRInstr> = Vec::with_capacity(old_instrs.len());

    for instr in old_instrs {
        // Fast path: only call_builtin instructions are candidates.
        if instr.op != "call_builtin" {
            new_instrs.push(instr);
            continue;
        }

        // Extract the builtin name from srcs[0].
        let builtin_name = match instr.srcs.first() {
            Some(Operand::Var(s)) => s.clone(),
            _ => {
                new_instrs.push(instr);
                continue;
            }
        };

        match builtin_name.as_str() {
            // ------------------------------------------------------------------
            // global_set name_reg, val_reg
            //
            // call_builtin "global_set", %name_reg, %val
            //   →  global_store Str("name"), Var("%val")
            //
            // %name_reg is resolved via const_str_map.  If unresolvable
            // (e.g. the name was computed dynamically), leave unchanged —
            // the backend validator will error with context.
            // ------------------------------------------------------------------
            "global_set" => {
                // srcs layout: [Var("global_set"), Var(name_reg), Var(val_reg)]
                let name_reg = match instr.srcs.get(1) {
                    Some(Operand::Var(r)) => r.clone(),
                    _ => { new_instrs.push(instr); continue; }
                };
                let val_src = match instr.srcs.get(2) {
                    Some(op) => op.clone(),
                    None => { new_instrs.push(instr); continue; }
                };

                // Resolve the global name from the look-back table.
                let global_name = match const_str_map.get(&name_reg) {
                    Some(n) => n.clone(),
                    // Name not in the map → dynamic name, cannot compile-time lower.
                    None => { new_instrs.push(instr); continue; }
                };

                // Emit: global_store Str("name"), val_src
                //
                // srcs[0] = Operand::Str(name)  — the compile-time name
                // srcs[1] = val_src              — the value register
                //
                // No dest (global_store is void).
                let global_store = IIRInstr::new(
                    "global_store",
                    None,
                    vec![Operand::Str(global_name), val_src],
                    "void",
                );
                new_instrs.push(global_store);
            }

            // ------------------------------------------------------------------
            // global_get name_reg → %dest
            //
            // call_builtin "global_get", %name_reg
            //   →  %dest = global_load Str("name")
            //
            // The dest of the call_builtin becomes the dest of global_load.
            // ------------------------------------------------------------------
            "global_get" => {
                // srcs layout: [Var("global_get"), Var(name_reg)]
                let name_reg = match instr.srcs.get(1) {
                    Some(Operand::Var(r)) => r.clone(),
                    _ => { new_instrs.push(instr); continue; }
                };

                let global_name = match const_str_map.get(&name_reg) {
                    Some(n) => n.clone(),
                    None => { new_instrs.push(instr); continue; }
                };

                // Emit: %dest = global_load Str("name")
                //
                // srcs[0] = Operand::Str(name)
                // dest    = original dest (or None if result discarded)
                let global_load = IIRInstr::new(
                    "global_load",
                    instr.dest.clone(),
                    vec![Operand::Str(global_name)],
                    instr.type_hint.clone(),
                );
                new_instrs.push(global_load);
            }

            // ------------------------------------------------------------------
            // print val_reg
            //
            // call_builtin "print", %val
            //   →  io_out %val
            //
            // Direct 1-to-1 rewrite: no look-back needed because the value
            // is a normal register reference, not a string name.
            //
            // `io_out` already exists in interpreter-ir and is classified as
            // a side-effecting void instruction.  The backends wire it to
            // their native print call:
            //   BEAM → erlang:display/1
            //   WASM → imported $__print_i64
            //   JVM  → System.out.println(long)
            //   CLR  → System.Console.WriteLine(int64)
            // ------------------------------------------------------------------
            "print" => {
                // srcs layout: [Var("print"), Var(val_reg)]
                let val_src = match instr.srcs.get(1) {
                    Some(op) => op.clone(),
                    None => { new_instrs.push(instr); continue; }
                };

                let io_out = IIRInstr::new(
                    "io_out",
                    None,
                    vec![val_src],
                    "void",
                );
                new_instrs.push(io_out);
            }

            // Unhandled builtins — leave for later passes or the backend.
            _ => {
                new_instrs.push(instr);
            }
        }
    }

    fn_.instructions = new_instrs;
}

// ---------------------------------------------------------------------------
// Module-level entry point
// ---------------------------------------------------------------------------

/// Lower global and I/O `call_builtin` instructions across the entire module.
///
/// Called from `lib.rs::lower_builtins()` after heap lowering.
/// Returns no errors — malformed/unresolvable instructions are left unchanged.
pub fn lower_global_io(module: &mut interpreter_ir::IIRModule) {
    for fn_ in &mut module.functions {
        lower_global_io_function(fn_);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};

    /// Build a one-function module from a list of instructions.
    fn make_module(instrs: Vec<IIRInstr>) -> IIRModule {
        let fn_ = IIRFunction::new("main", vec![], "void", instrs);
        IIRModule {
            name: "test".into(),
            functions: vec![fn_],
            entry_point: Some("main".into()),
            language: "twig".into(),
        }
    }

    /// Build the two-instruction sequence twig-ir-compiler emits for a global store:
    ///   const  %name_reg = Operand::Var("name")
    ///   call_builtin "global_set", %name_reg, %val
    fn global_set_sequence(reg: &str, global_name: &str, val_reg: &str) -> Vec<IIRInstr> {
        let const_name = IIRInstr::new(
            "const",
            Some(reg.into()),
            vec![Operand::Var(global_name.into())],
            "any",
        );
        let global_set = IIRInstr::new(
            "call_builtin",
            None,
            vec![
                Operand::Var("global_set".into()),
                Operand::Var(reg.into()),
                Operand::Var(val_reg.into()),
            ],
            "void",
        );
        vec![const_name, global_set]
    }

    /// Build the two-instruction sequence for a global load:
    ///   const  %name_reg = Operand::Var("name")
    ///   call_builtin "global_get", %name_reg  → %dest
    fn global_get_sequence(reg: &str, global_name: &str, dest: &str) -> Vec<IIRInstr> {
        let const_name = IIRInstr::new(
            "const",
            Some(reg.into()),
            vec![Operand::Var(global_name.into())],
            "any",
        );
        let global_get = IIRInstr::new(
            "call_builtin",
            Some(dest.into()),
            vec![
                Operand::Var("global_get".into()),
                Operand::Var(reg.into()),
            ],
            "any",
        );
        vec![const_name, global_get]
    }

    // ------------------------------------------------------------------
    // global_set tests
    // ------------------------------------------------------------------

    #[test]
    fn global_set_becomes_global_store() {
        let mut m = make_module(global_set_sequence("%n1", "x", "%v"));
        lower_global_io(&mut m);
        // Two instructions in → two instructions out (const + global_store).
        // The const is preserved (it may be read by other code); the call_builtin
        // is replaced by global_store.
        let ops: Vec<&str> = m.functions[0].instructions.iter().map(|i| i.op.as_str()).collect();
        assert!(ops.contains(&"global_store"), "expected global_store in {:?}", ops);
    }

    #[test]
    fn global_store_has_no_dest() {
        let mut m = make_module(global_set_sequence("%n1", "myvar", "%v"));
        lower_global_io(&mut m);
        let store = m.functions[0].instructions.iter().find(|i| i.op == "global_store").unwrap();
        assert!(store.dest.is_none());
    }

    #[test]
    fn global_store_src0_is_str_name() {
        let mut m = make_module(global_set_sequence("%n1", "counter", "%v"));
        lower_global_io(&mut m);
        let store = m.functions[0].instructions.iter().find(|i| i.op == "global_store").unwrap();
        assert_eq!(store.srcs[0], Operand::Str("counter".into()));
    }

    #[test]
    fn global_store_src1_is_value_reg() {
        let mut m = make_module(global_set_sequence("%n1", "x", "%myval"));
        lower_global_io(&mut m);
        let store = m.functions[0].instructions.iter().find(|i| i.op == "global_store").unwrap();
        assert_eq!(store.srcs[1], Operand::Var("%myval".into()));
    }

    #[test]
    fn global_store_type_hint_is_void() {
        let mut m = make_module(global_set_sequence("%n1", "x", "%v"));
        lower_global_io(&mut m);
        let store = m.functions[0].instructions.iter().find(|i| i.op == "global_store").unwrap();
        assert_eq!(store.type_hint, "void");
    }

    // ------------------------------------------------------------------
    // global_get tests
    // ------------------------------------------------------------------

    #[test]
    fn global_get_becomes_global_load() {
        let mut m = make_module(global_get_sequence("%n1", "x", "%r"));
        lower_global_io(&mut m);
        let ops: Vec<&str> = m.functions[0].instructions.iter().map(|i| i.op.as_str()).collect();
        assert!(ops.contains(&"global_load"), "expected global_load in {:?}", ops);
    }

    #[test]
    fn global_load_has_original_dest() {
        let mut m = make_module(global_get_sequence("%n1", "myvar", "%result"));
        lower_global_io(&mut m);
        let load = m.functions[0].instructions.iter().find(|i| i.op == "global_load").unwrap();
        assert_eq!(load.dest.as_deref(), Some("%result"));
    }

    #[test]
    fn global_load_src0_is_str_name() {
        let mut m = make_module(global_get_sequence("%n1", "counter", "%r"));
        lower_global_io(&mut m);
        let load = m.functions[0].instructions.iter().find(|i| i.op == "global_load").unwrap();
        assert_eq!(load.srcs[0], Operand::Str("counter".into()));
    }

    // ------------------------------------------------------------------
    // print → io_out tests
    // ------------------------------------------------------------------

    #[test]
    fn print_becomes_io_out() {
        let print_call = IIRInstr::new(
            "call_builtin",
            None,
            vec![Operand::Var("print".into()), Operand::Var("%val".into())],
            "void",
        );
        let mut m = make_module(vec![print_call]);
        lower_global_io(&mut m);
        assert_eq!(m.functions[0].instructions[0].op, "io_out");
    }

    #[test]
    fn io_out_has_no_dest() {
        let print_call = IIRInstr::new(
            "call_builtin",
            None,
            vec![Operand::Var("print".into()), Operand::Var("%val".into())],
            "void",
        );
        let mut m = make_module(vec![print_call]);
        lower_global_io(&mut m);
        assert!(m.functions[0].instructions[0].dest.is_none());
    }

    #[test]
    fn io_out_src0_is_value_reg() {
        let print_call = IIRInstr::new(
            "call_builtin",
            None,
            vec![Operand::Var("print".into()), Operand::Var("%myval".into())],
            "void",
        );
        let mut m = make_module(vec![print_call]);
        lower_global_io(&mut m);
        assert_eq!(m.functions[0].instructions[0].srcs[0], Operand::Var("%myval".into()));
    }

    #[test]
    fn io_out_type_hint_is_void() {
        let print_call = IIRInstr::new(
            "call_builtin",
            None,
            vec![Operand::Var("print".into()), Operand::Var("%v".into())],
            "void",
        );
        let mut m = make_module(vec![print_call]);
        lower_global_io(&mut m);
        assert_eq!(m.functions[0].instructions[0].type_hint, "void");
    }

    // ------------------------------------------------------------------
    // Unresolvable global_set (name is dynamic) → left unchanged
    // ------------------------------------------------------------------

    #[test]
    fn unresolvable_global_set_left_unchanged() {
        // No const instruction defines %n1 — the name is unresolvable.
        let global_set = IIRInstr::new(
            "call_builtin",
            None,
            vec![
                Operand::Var("global_set".into()),
                Operand::Var("%n1".into()),  // not in const_str_map
                Operand::Var("%v".into()),
            ],
            "void",
        );
        let mut m = make_module(vec![global_set]);
        lower_global_io(&mut m);
        // Must still be call_builtin — no resolution happened.
        assert_eq!(m.functions[0].instructions[0].op, "call_builtin");
    }

    #[test]
    fn unresolvable_global_get_left_unchanged() {
        let global_get = IIRInstr::new(
            "call_builtin",
            Some("%r".into()),
            vec![
                Operand::Var("global_get".into()),
                Operand::Var("%n1".into()),
            ],
            "any",
        );
        let mut m = make_module(vec![global_get]);
        lower_global_io(&mut m);
        assert_eq!(m.functions[0].instructions[0].op, "call_builtin");
    }

    // ------------------------------------------------------------------
    // Mixed sequence: set + get + print in same function
    // ------------------------------------------------------------------

    #[test]
    fn mixed_global_and_print_sequence() {
        // (define x 42) (print x)
        // Emits: const %n1 = Var("x"); global_set %n1, %v;
        //        const %n2 = Var("x"); global_get %n2 → %r;
        //        print %r
        let mut instrs = global_set_sequence("%n1", "x", "%v");
        instrs.extend(global_get_sequence("%n2", "x", "%r"));
        instrs.push(IIRInstr::new(
            "call_builtin",
            None,
            vec![Operand::Var("print".into()), Operand::Var("%r".into())],
            "void",
        ));
        let mut m = make_module(instrs);
        lower_global_io(&mut m);
        let ops: Vec<&str> = m.functions[0].instructions.iter().map(|i| i.op.as_str()).collect();
        assert!(ops.contains(&"global_store"));
        assert!(ops.contains(&"global_load"));
        assert!(ops.contains(&"io_out"));
        // No residual call_builtin for global_set/global_get/print.
        for (op, name) in m.functions[0].instructions.iter()
            .filter(|i| i.op == "call_builtin")
            .filter_map(|i| i.srcs.first().and_then(|o| o.as_var()).map(|n| (i.op.as_str(), n)))
        {
            assert!(
                !matches!(name, "global_set" | "global_get" | "print"),
                "unexpected residual call_builtin \"{name}\" ({op})",
            );
        }
    }

    // ------------------------------------------------------------------
    // Other call_builtin names are not touched
    // ------------------------------------------------------------------

    #[test]
    fn make_closure_not_touched() {
        let instr = IIRInstr::new(
            "call_builtin",
            Some("%c".into()),
            vec![Operand::Var("make_closure".into()), Operand::Var("%fn".into())],
            "any",
        );
        let mut m = make_module(vec![instr]);
        lower_global_io(&mut m);
        assert_eq!(m.functions[0].instructions[0].op, "call_builtin");
    }

    #[test]
    fn apply_closure_not_touched() {
        let instr = IIRInstr::new(
            "call_builtin",
            Some("%r".into()),
            vec![
                Operand::Var("apply_closure".into()),
                Operand::Var("%clos".into()),
                Operand::Var("%arg".into()),
            ],
            "any",
        );
        let mut m = make_module(vec![instr]);
        lower_global_io(&mut m);
        assert_eq!(m.functions[0].instructions[0].op, "call_builtin");
    }
}
