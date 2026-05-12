//! # heap — Phase 2 heap builtin lowering.
//!
//! This module implements Phase 2 of the LANG31 builtin lowering pass: it
//! rewrites `call_builtin` instructions for Lispy heap operations into typed
//! IIR heap opcodes that the `iir-to-*` backends understand.
//!
//! ## Why a separate module?
//!
//! Phase 1 (`numeric.rs`) does 1-to-1 opcode replacement — each `call_builtin`
//! becomes exactly one `add`, `cmp_eq`, etc.  Phase 2 is different: `cons`
//! expands to **three** instructions (`alloc` + two `field_store`s), and `car`/
//! `cdr` need the field index inserted as an `Int` immediate.  Keeping the two
//! phases in separate files prevents numeric.rs from growing into an unwieldy
//! catch-all.
//!
//! ## Lowering table
//!
//! | Builtin      | Arity | Emitted IIR ops |
//! |--------------|:-----:|-----------------|
//! | `cons`       | 2     | `alloc` + `field_store[0]` + `field_store[1]` |
//! | `car`        | 1     | `field_load[0]` |
//! | `cdr`        | 1     | `field_load[1]` |
//! | `null?`      | 1     | `is_null` |
//! | `make_nil`   | 0     | `const 0 : ref<LispyPair>` |
//!
//! **Not handled here** — left as `call_builtin` for later passes:
//! - `pair?`         — type predicate, needs a different mechanism.
//! - `make_closure`  — closure allocation (BEAM02 / CLR02 phase).
//! - `apply_closure` — closure invocation.
//! - `global_set`, `global_get`, `print` — LANG27 side-effects.
//!
//! ## cons expansion anatomy
//!
//! A Lisp cons cell is a two-field heap object:
//!
//! ```text
//!   ┌────────────┬────────────┐
//!   │  field[0]  │  field[1]  │
//!   │   (head)   │   (tail)   │
//!   └────────────┴────────────┘
//! ```
//!
//! In IIR:
//! ```text
//! BEFORE:   %cell = call_builtin(cons, %head, %tail) : ref<LispyPair>
//! AFTER:
//!   %cell     = alloc()                    : ref<LispyPair>  may_alloc=true
//!   (no dest)   field_store(%cell, 0, %head) : void
//!   (no dest)   field_store(%cell, 1, %tail) : void
//! ```
//!
//! The `alloc` instruction inherits the original `dest` so downstream code
//! that reads `%cell` finds it in the expected register.
//!
//! ## make_nil semantics
//!
//! Nil is the empty list.  In IIR we represent it as the integer 0 with
//! type_hint `"ref<LispyPair>"`.  Each backend maps this to its native nil:
//!
//! | Backend | Native nil |
//! |---------|-----------|
//! | BEAM    | `[]` atom  |
//! | JVM     | `null`     |
//! | CLR     | `null`     |
//! | WASM    | `ref.null` |

use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::function::IIRFunction;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Lower all heap `call_builtin` instructions in `fn_` to typed IIR heap ops.
///
/// This function replaces each matching `call_builtin` in-place by rebuilding
/// the instruction list.  We rebuild rather than mutate because `cons` expands
/// to three instructions — we cannot grow a Vec while iterating it under
/// Rust's borrow rules.
///
/// Instructions for builtins not in the heap table (`pair?`, `make_closure`,
/// etc.) are left completely unchanged.
///
/// # Returns
///
/// This function is infallible by design.  Malformed instructions (missing
/// operands, wrong dest) are left unchanged so that the backend's validator
/// can emit a clearer error with full context.
pub fn lower_heap_function(fn_: &mut IIRFunction) {
    // Swap out the existing instruction list so we can iterate it while
    // building the replacement.  `mem::take` is zero-copy — it replaces
    // fn_.instructions with an empty Vec without cloning anything.
    let old_instrs = std::mem::take(&mut fn_.instructions);

    // Pre-allocate with a small headroom for the cons expansion (worst-case
    // every instruction is a `cons`, tripling the count).
    let mut new_instrs: Vec<IIRInstr> = Vec::with_capacity(old_instrs.len() * 2);

    for instr in old_instrs {
        // Fast path: skip non-call_builtin instructions without any allocation.
        if instr.op != "call_builtin" {
            new_instrs.push(instr);
            continue;
        }

        // The first source operand of call_builtin is always the builtin name
        // as a Var.  Anything else is a malformed instruction — leave it alone.
        let builtin_name = match instr.srcs.first() {
            Some(Operand::Var(name)) => name.clone(),
            _ => {
                new_instrs.push(instr);
                continue;
            }
        };

        match builtin_name.as_str() {
            // ------------------------------------------------------------------
            // cons head tail → %cell
            //
            // Expands to three instructions:
            //   1. alloc() → %cell           — allocate the pair on the heap
            //   2. field_store(%cell, 0, head) — write the head (car) field
            //   3. field_store(%cell, 1, tail) — write the tail (cdr) field
            //
            // The original `dest` name (%cell) moves to the `alloc` so that
            // code that reads %cell after this sequence sees the heap pointer.
            // ------------------------------------------------------------------
            "cons" => {
                // srcs layout: [Var("cons"), Var(head), Var(tail)]
                // We need exactly 2 argument operands (indices 1 and 2).
                let head = match instr.srcs.get(1) {
                    Some(op) => op.clone(),
                    None => { new_instrs.push(instr); continue; }
                };
                let tail = match instr.srcs.get(2) {
                    Some(op) => op.clone(),
                    None => { new_instrs.push(instr); continue; }
                };

                // The cell's variable name comes from the original dest.
                // If there is no dest, the cons result is unused — still
                // emit the alloc+field_stores so side-effects (GC roots,
                // debugging) are visible; a dead-code pass can clean up.
                let cell_name: String = instr.dest.clone()
                    .unwrap_or_else(|| "__cons_cell".to_string());

                // Instruction 1: alloc a fresh LispyPair and name it %cell.
                // `may_alloc = true` tells the GC that this is a heap allocation
                // point — the collector needs to know where to scan for roots.
                let mut alloc = IIRInstr::new(
                    "alloc",
                    Some(cell_name.clone()),
                    vec![],  // alloc takes no source operands; size is implicit in type_hint
                    "ref<LispyPair>",
                );
                alloc.may_alloc = true;
                new_instrs.push(alloc);

                // Instruction 2: write head into field 0 of the new pair.
                // field_store has no dest (it is a write, not a read).
                // srcs = [cell_ptr, field_index, value]
                let field_store_head = IIRInstr::new(
                    "field_store",
                    None,  // stores never produce a value
                    vec![
                        Operand::Var(cell_name.clone()),  // the object
                        Operand::Int(0),                   // field index: 0 = car
                        head,                              // the value to write
                    ],
                    "void",  // stores have no result type
                );
                new_instrs.push(field_store_head);

                // Instruction 3: write tail into field 1 of the new pair.
                let field_store_tail = IIRInstr::new(
                    "field_store",
                    None,
                    vec![
                        Operand::Var(cell_name),  // same object
                        Operand::Int(1),           // field index: 1 = cdr
                        tail,                      // the value to write
                    ],
                    "void",
                );
                new_instrs.push(field_store_tail);
            }

            // ------------------------------------------------------------------
            // car pair → head
            //
            // Reads field 0 (the "car" / head) from a cons cell.
            // In memory:  %result = *(pair + 0)
            // IIR:        field_load(pair, 0) → %result : ref<any>
            // ------------------------------------------------------------------
            "car" => {
                // srcs layout: [Var("car"), Var(pair)]
                let pair = match instr.srcs.get(1) {
                    Some(op) => op.clone(),
                    None => { new_instrs.push(instr); continue; }
                };

                // field_load has a dest (the loaded value) and three srcs:
                // [object_ptr, field_index].  The type of the loaded field is
                // "ref<any>" because cons cells can hold any Lisp value.
                let field_load = IIRInstr::new(
                    "field_load",
                    instr.dest.clone(),
                    vec![
                        pair,           // the cons cell
                        Operand::Int(0), // field 0 = head
                    ],
                    "ref<any>",  // car can return any Lisp value
                );
                new_instrs.push(field_load);
            }

            // ------------------------------------------------------------------
            // cdr pair → tail
            //
            // Reads field 1 (the "cdr" / tail) from a cons cell.
            // In memory:  %result = *(pair + 1)
            // IIR:        field_load(pair, 1) → %result : ref<any>
            // ------------------------------------------------------------------
            "cdr" => {
                // srcs layout: [Var("cdr"), Var(pair)]
                let pair = match instr.srcs.get(1) {
                    Some(op) => op.clone(),
                    None => { new_instrs.push(instr); continue; }
                };

                let field_load = IIRInstr::new(
                    "field_load",
                    instr.dest.clone(),
                    vec![
                        pair,
                        Operand::Int(1), // field 1 = tail
                    ],
                    "ref<any>",
                );
                new_instrs.push(field_load);
            }

            // ------------------------------------------------------------------
            // null? x → bool
            //
            // Tests whether x is the nil sentinel (the empty list).
            // IIR: is_null(x) → %result : bool
            //
            // The `is_null` opcode maps to each backend's nil check:
            //   BEAM → is_nil/1
            //   JVM  → IFNONNULL / IFNULL
            //   WASM → ref.is_null
            // ------------------------------------------------------------------
            "null?" => {
                // srcs layout: [Var("null?"), Var(x)]
                let x = match instr.srcs.get(1) {
                    Some(op) => op.clone(),
                    None => { new_instrs.push(instr); continue; }
                };

                let is_null = IIRInstr::new(
                    "is_null",
                    instr.dest.clone(),
                    vec![x],
                    "bool",  // is_null always produces a boolean
                );
                new_instrs.push(is_null);
            }

            // ------------------------------------------------------------------
            // make_nil → %nil
            //
            // Produces the nil sentinel.  We represent nil as the integer 0
            // with type_hint "ref<LispyPair>" — the 0 value is the null/zero
            // sentinel, and each backend maps it to its native nil:
            //
            //   BEAM  → `[]` (the empty list atom)
            //   JVM   → `null`
            //   CLR   → `null`
            //   WASM  → `ref.null`
            //
            // This is a `const` (not `alloc`) because nil is a singleton —
            // there is no heap allocation; we are just naming the zero sentinel.
            // `may_alloc = false` (the default) is intentionally left set.
            // ------------------------------------------------------------------
            "make_nil" => {
                // srcs layout: [Var("make_nil")]  — no arguments
                let nil_const = IIRInstr::new(
                    "const",
                    instr.dest.clone(),
                    vec![Operand::Int(0)],  // 0 = the nil sentinel
                    "ref<LispyPair>",       // tells the backend this is a reference-typed nil
                );
                new_instrs.push(nil_const);
            }

            // ------------------------------------------------------------------
            // Unknown / unhandled heap builtins
            //
            // `pair?`, `make_closure`, `apply_closure`, `global_set`,
            // `global_get`, `print`, … are left as call_builtin so that
            // later passes or backends can handle them.
            // ------------------------------------------------------------------
            _ => {
                new_instrs.push(instr);
            }
        }
    }

    fn_.instructions = new_instrs;
}

// ---------------------------------------------------------------------------
// Module-level entry point (mirrors numeric::lower_function)
// ---------------------------------------------------------------------------

/// Lower all heap `call_builtin` instructions across the entire module.
///
/// Called from `lib.rs::lower_builtins()` after numeric lowering.
/// Returns no errors — malformed instructions are left for the backend.
pub fn lower_heap_builtins(module: &mut interpreter_ir::IIRModule) {
    for fn_ in &mut module.functions {
        lower_heap_function(fn_);
    }
}

// ---------------------------------------------------------------------------
// Tests (unit — in-module)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};

    /// Build a one-function module from a list of instructions.
    fn make_module(instrs: Vec<IIRInstr>) -> IIRModule {
        let fn_ = IIRFunction::new("main", vec![], "ref<LispyPair>", instrs);
        IIRModule {
            name: "test".into(),
            functions: vec![fn_],
            entry_point: Some("main".into()),
            language: "twig".into(),
            exports: vec![],
            imports: vec![],
        }
    }

    /// Build a `call_builtin "cons"` instruction.
    fn cons_call(head: &str, tail: &str) -> IIRInstr {
        IIRInstr::new(
            "call_builtin",
            Some("%cell".into()),
            vec![
                Operand::Var("cons".into()),
                Operand::Var(head.into()),
                Operand::Var(tail.into()),
            ],
            "ref<LispyPair>",
        )
    }

    #[test]
    fn cons_expands_to_three_instructions() {
        let mut m = make_module(vec![cons_call("%h", "%t")]);
        lower_heap_builtins(&mut m);
        // one alloc + two field_stores = 3
        assert_eq!(m.functions[0].instructions.len(), 3);
    }

    #[test]
    fn cons_first_instruction_is_alloc() {
        let mut m = make_module(vec![cons_call("%h", "%t")]);
        lower_heap_builtins(&mut m);
        assert_eq!(m.functions[0].instructions[0].op, "alloc");
    }

    #[test]
    fn cons_alloc_gets_original_dest() {
        // The alloc instruction must receive the original dest (%cell).
        let mut m = make_module(vec![cons_call("%h", "%t")]);
        lower_heap_builtins(&mut m);
        assert_eq!(
            m.functions[0].instructions[0].dest.as_deref(),
            Some("%cell"),
        );
    }

    #[test]
    fn cons_alloc_has_may_alloc_true() {
        // alloc triggers heap allocation → GC needs to know.
        let mut m = make_module(vec![cons_call("%h", "%t")]);
        lower_heap_builtins(&mut m);
        assert!(m.functions[0].instructions[0].may_alloc);
    }

    #[test]
    fn cons_alloc_type_hint_is_ref_lispy_pair() {
        let mut m = make_module(vec![cons_call("%h", "%t")]);
        lower_heap_builtins(&mut m);
        assert_eq!(m.functions[0].instructions[0].type_hint, "ref<LispyPair>");
    }

    #[test]
    fn cons_alloc_has_no_srcs() {
        // alloc takes no source operands; size is encoded in the type_hint.
        let mut m = make_module(vec![cons_call("%h", "%t")]);
        lower_heap_builtins(&mut m);
        assert!(m.functions[0].instructions[0].srcs.is_empty());
    }

    #[test]
    fn cons_second_instruction_is_field_store() {
        let mut m = make_module(vec![cons_call("%h", "%t")]);
        lower_heap_builtins(&mut m);
        assert_eq!(m.functions[0].instructions[1].op, "field_store");
    }

    #[test]
    fn cons_third_instruction_is_field_store() {
        let mut m = make_module(vec![cons_call("%h", "%t")]);
        lower_heap_builtins(&mut m);
        assert_eq!(m.functions[0].instructions[2].op, "field_store");
    }

    #[test]
    fn cons_field_store_head_dest_is_none() {
        // field_store (write op) never produces a value.
        let mut m = make_module(vec![cons_call("%h", "%t")]);
        lower_heap_builtins(&mut m);
        assert!(m.functions[0].instructions[1].dest.is_none());
    }

    #[test]
    fn cons_field_store_tail_dest_is_none() {
        let mut m = make_module(vec![cons_call("%h", "%t")]);
        lower_heap_builtins(&mut m);
        assert!(m.functions[0].instructions[2].dest.is_none());
    }

    #[test]
    fn cons_field_store_head_pair_ptr_is_first_src() {
        // field_store srcs[0] must be the cell pointer (%cell).
        let mut m = make_module(vec![cons_call("%h", "%t")]);
        lower_heap_builtins(&mut m);
        let instr = &m.functions[0].instructions[1];
        assert_eq!(instr.srcs[0], Operand::Var("%cell".into()));
    }

    #[test]
    fn cons_field_store_head_field_index_is_zero() {
        // srcs[1] is the field index: 0 for the head (car) slot.
        let mut m = make_module(vec![cons_call("%h", "%t")]);
        lower_heap_builtins(&mut m);
        let instr = &m.functions[0].instructions[1];
        assert_eq!(instr.srcs[1], Operand::Int(0));
    }

    #[test]
    fn cons_field_store_head_value_is_head_var() {
        // srcs[2] is the value: the head variable.
        let mut m = make_module(vec![cons_call("%h", "%t")]);
        lower_heap_builtins(&mut m);
        let instr = &m.functions[0].instructions[1];
        assert_eq!(instr.srcs[2], Operand::Var("%h".into()));
    }

    #[test]
    fn cons_field_store_tail_field_index_is_one() {
        // srcs[1] is the field index: 1 for the tail (cdr) slot.
        let mut m = make_module(vec![cons_call("%h", "%t")]);
        lower_heap_builtins(&mut m);
        let instr = &m.functions[0].instructions[2];
        assert_eq!(instr.srcs[1], Operand::Int(1));
    }

    #[test]
    fn cons_field_store_tail_value_is_tail_var() {
        let mut m = make_module(vec![cons_call("%h", "%t")]);
        lower_heap_builtins(&mut m);
        let instr = &m.functions[0].instructions[2];
        assert_eq!(instr.srcs[2], Operand::Var("%t".into()));
    }

    #[test]
    fn car_produces_field_load() {
        let instr = IIRInstr::new(
            "call_builtin",
            Some("%head".into()),
            vec![Operand::Var("car".into()), Operand::Var("%pair".into())],
            "ref<any>",
        );
        let mut m = make_module(vec![instr]);
        lower_heap_builtins(&mut m);
        assert_eq!(m.functions[0].instructions.len(), 1);
        assert_eq!(m.functions[0].instructions[0].op, "field_load");
    }

    #[test]
    fn car_field_load_index_is_zero() {
        let instr = IIRInstr::new(
            "call_builtin",
            Some("%head".into()),
            vec![Operand::Var("car".into()), Operand::Var("%pair".into())],
            "ref<any>",
        );
        let mut m = make_module(vec![instr]);
        lower_heap_builtins(&mut m);
        let load = &m.functions[0].instructions[0];
        assert_eq!(load.srcs[1], Operand::Int(0));
    }

    #[test]
    fn cdr_produces_field_load() {
        let instr = IIRInstr::new(
            "call_builtin",
            Some("%tail".into()),
            vec![Operand::Var("cdr".into()), Operand::Var("%pair".into())],
            "ref<any>",
        );
        let mut m = make_module(vec![instr]);
        lower_heap_builtins(&mut m);
        assert_eq!(m.functions[0].instructions[0].op, "field_load");
    }

    #[test]
    fn cdr_field_load_index_is_one() {
        let instr = IIRInstr::new(
            "call_builtin",
            Some("%tail".into()),
            vec![Operand::Var("cdr".into()), Operand::Var("%pair".into())],
            "ref<any>",
        );
        let mut m = make_module(vec![instr]);
        lower_heap_builtins(&mut m);
        let load = &m.functions[0].instructions[0];
        assert_eq!(load.srcs[1], Operand::Int(1));
    }

    #[test]
    fn null_pred_produces_is_null() {
        let instr = IIRInstr::new(
            "call_builtin",
            Some("%result".into()),
            vec![Operand::Var("null?".into()), Operand::Var("%x".into())],
            "bool",
        );
        let mut m = make_module(vec![instr]);
        lower_heap_builtins(&mut m);
        assert_eq!(m.functions[0].instructions[0].op, "is_null");
    }

    #[test]
    fn null_pred_type_hint_is_bool() {
        let instr = IIRInstr::new(
            "call_builtin",
            Some("%result".into()),
            vec![Operand::Var("null?".into()), Operand::Var("%x".into())],
            "bool",
        );
        let mut m = make_module(vec![instr]);
        lower_heap_builtins(&mut m);
        assert_eq!(m.functions[0].instructions[0].type_hint, "bool");
    }

    #[test]
    fn make_nil_produces_const_zero() {
        let instr = IIRInstr::new(
            "call_builtin",
            Some("%nil".into()),
            vec![Operand::Var("make_nil".into())],
            "ref<LispyPair>",
        );
        let mut m = make_module(vec![instr]);
        lower_heap_builtins(&mut m);
        let lowered = &m.functions[0].instructions[0];
        assert_eq!(lowered.op, "const");
        assert_eq!(lowered.srcs[0], Operand::Int(0));
    }

    #[test]
    fn make_nil_type_hint_is_ref_lispy_pair() {
        let instr = IIRInstr::new(
            "call_builtin",
            Some("%nil".into()),
            vec![Operand::Var("make_nil".into())],
            "ref<LispyPair>",
        );
        let mut m = make_module(vec![instr]);
        lower_heap_builtins(&mut m);
        assert_eq!(m.functions[0].instructions[0].type_hint, "ref<LispyPair>");
    }

    #[test]
    fn pair_pred_left_unchanged() {
        // `pair?` is NOT lowered by this pass — it is a type predicate that
        // requires a different mechanism (tag check on the value's type tag).
        let instr = IIRInstr::new(
            "call_builtin",
            Some("%r".into()),
            vec![Operand::Var("pair?".into()), Operand::Var("%x".into())],
            "bool",
        );
        let mut m = make_module(vec![instr]);
        lower_heap_builtins(&mut m);
        assert_eq!(m.functions[0].instructions[0].op, "call_builtin");
    }

    #[test]
    fn make_closure_left_unchanged() {
        // `make_closure` is a BEAM02/CLR02 builtin, not a heap builtin.
        let instr = IIRInstr::new(
            "call_builtin",
            Some("%clos".into()),
            vec![Operand::Var("make_closure".into()), Operand::Var("%fn".into())],
            "any",
        );
        let mut m = make_module(vec![instr]);
        lower_heap_builtins(&mut m);
        assert_eq!(m.functions[0].instructions[0].op, "call_builtin");
    }

    #[test]
    fn multiple_cons_in_sequence() {
        // Two cons cells in sequence — each should expand to 3 instructions.
        let instrs = vec![
            cons_call("%h1", "%t1"),
            cons_call("%h2", "%t2"),
        ];
        let mut m = make_module(instrs);
        lower_heap_builtins(&mut m);
        // 2 × 3 = 6 instructions
        assert_eq!(m.functions[0].instructions.len(), 6);
        // The two alloc dests should be distinct (their original dests are
        // both "%cell" in this helper, but in real code they would differ).
        assert_eq!(m.functions[0].instructions[0].op, "alloc");
        assert_eq!(m.functions[0].instructions[3].op, "alloc");
    }

    #[test]
    fn cons_then_car_then_null_in_same_function() {
        // A realistic sequence: build a pair, extract the head, check nil.
        let instrs = vec![
            // %cell = cons(%h, %t)
            cons_call("%h", "%t"),
            // %head = car(%cell)
            IIRInstr::new(
                "call_builtin",
                Some("%head".into()),
                vec![Operand::Var("car".into()), Operand::Var("%cell".into())],
                "ref<any>",
            ),
            // %is_nil = null?(%head)
            IIRInstr::new(
                "call_builtin",
                Some("%is_nil".into()),
                vec![Operand::Var("null?".into()), Operand::Var("%head".into())],
                "bool",
            ),
        ];
        let mut m = make_module(instrs);
        lower_heap_builtins(&mut m);

        // cons → 3, car → 1, null? → 1 = 5 instructions
        assert_eq!(m.functions[0].instructions.len(), 5);
        assert_eq!(m.functions[0].instructions[0].op, "alloc");
        assert_eq!(m.functions[0].instructions[1].op, "field_store");
        assert_eq!(m.functions[0].instructions[2].op, "field_store");
        assert_eq!(m.functions[0].instructions[3].op, "field_load");
        assert_eq!(m.functions[0].instructions[4].op, "is_null");
    }
}
