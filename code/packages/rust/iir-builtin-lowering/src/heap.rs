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
// Runtime-call lowering (LANG77) — the *native* counterpart of the structural
// lowering above.
// ---------------------------------------------------------------------------
//
// ## Two lowerings, one decision: where do lisp values live?
//
// The `lower_heap_builtins` pass above expands `cons` into an `alloc` + two
// `field_store`s and reads `car`/`cdr` with `field_load`.  That is exactly
// right for the **managed** backends (wasm / jvm / clr / beam): they have a
// garbage-collected object model, so a cons cell is a host object and the
// fields are host slots.
//
// The **native** backends (aarch64 / x86_64, driven by `twig-aot`) have no
// managed heap.  Instead they link the shared C lisp runtime
// (`twig-aot/runtime/dynval_runtime.c`, see LANG77) which implements
// `lispy-runtime`'s tagged-value model — `__dyn_cons`/`car`/`cdr`.
// For those backends a cons cell is a *runtime call*, not an inline
// allocation, so the value is a proper NaN-box-tagged `LispyValue` (a
// heap-tagged pointer) rather than a raw machine word.  That tag is what
// makes `pair?`/`ATOM`/`EQ`/symbols possible later (L3b-2c) — a raw word
// carries no type tag; a tagged value does.
//
// So the lowering is **target-aware**, not language-specific: a managed
// target gets the structural form, a native target gets the runtime-call
// form.  Both are driven by the *same* frontend IIR (`call_builtin
// "cons"/"car"/"cdr"`), so every lisp-family frontend (McCarthy Lisp, Twig,
// future lisps) reaches both worlds for free — there is nothing
// McCarthy-specific here.
//
// ## What the rename does
//
// | Frontend builtin | Native runtime symbol  |
// |------------------|------------------------|
// | `cons`           | `lispy_cons` (→ `__dyn_cons(car, cdr)`) |
// | `car`            | `lispy_car`  (→ `__dyn_car(pair)`)      |
// | `cdr`            | `lispy_cdr`  (→ `__dyn_cdr(pair)`)      |
//
// The argument order already matches the C ABI (`cons head tail` →
// `lispy_cons(car, cdr)`), so the transform is a pure **rename** of the
// builtin name in `srcs[0]` — no operand shuffling, no instruction
// expansion.  The native backends turn `call_builtin "lispy_cons"` into
// `BL/CALL __dyn_cons` via their generic `call_builtin` dispatch +
// the `V1_BUILTINS` table; no new backend opcodes are needed.
//
// `null?` / `make_nil` / `make_symbol` are intentionally **not** renamed
// here — `make_symbol` needs string-literal emission (L3b-2c-3) and the
// `null?`/`make_nil` nil-handling rides along with it.
//
// `pair?` / `not` / `equal?` (the `ATOM`/`EQ` predicates) ARE renamed as of
// L3b-2c-2 — they consume/produce tagged `LispyValue`s, which the
// representation pass (`lower_lisp_repr`) has set up by the time the native
// backend runs.

/// The frontend→native-runtime builtin renames (LANG77 / L3b-2b, L3b-2c-2).
const RUNTIME_RENAMES: &[(&str, &str)] = &[
    // L3b-2b — the cons data path.
    ("cons", "lispy_cons"),
    ("car", "lispy_car"),
    ("cdr", "lispy_cdr"),
    // L3b-2c-2 — the predicates (ATOM = pair? + not; EQ = equal?).
    //
    // `pair?` and `equal?` are unambiguous lisp builtins (no machine meaning),
    // so the rename is safe here. `not` is NOT renamed here: it is also a
    // *numeric* builtin (machine boolean-not, used by Twig), so renaming it
    // unconditionally would hijack Twig's `not`. McCarthy's `not` (the second
    // half of `ATOM` = `not(pair?)`) is renamed *type-directed* in
    // `lisp_repr` — only when its argument is a `lispy_*` result.
    ("pair?", "lispy_pair_p"),
    ("equal?", "lispy_equal"),
];

/// Rename the cons/car/cdr `call_builtin`s in `fn_` to their `lispy_*`
/// runtime-call form (see the module note above).  In-place, allocation-free
/// (a rename never expands an instruction), and a no-op for any
/// `call_builtin` not in [`RUNTIME_RENAMES`].
pub fn lower_heap_function_runtime(fn_: &mut IIRFunction) {
    for instr in &mut fn_.instructions {
        if instr.op != "call_builtin" {
            continue;
        }
        // The builtin name is always `srcs[0] = Var(name)`.  Rewrite it in
        // place if it is one we route to the runtime; leave everything else
        // (including a malformed call_builtin) untouched for the backend.
        if let Some(Operand::Var(name)) = instr.srcs.first_mut() {
            if let Some((_, runtime)) = RUNTIME_RENAMES.iter().find(|(orig, _)| orig == name) {
                *name = (*runtime).to_string();
            }
        }
    }
}

/// Module-level entry point for the native runtime-call lowering.
///
/// `twig-aot::prepare_module_for_aot` calls this **instead of**
/// [`lower_heap_builtins`] so the native pipeline routes cons cells through
/// the linked C lisp runtime rather than inline `alloc`/`field_*`.  The
/// managed `iir-to-*` backends keep calling [`lower_heap_builtins`].
pub fn lower_heap_builtins_runtime(module: &mut interpreter_ir::IIRModule) {
    for fn_ in &mut module.functions {
        lower_heap_function_runtime(fn_);
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

    // ── Runtime-call lowering (LANG77 / L3b-2b) ───────────────────────────

    /// Pull the builtin name out of a `call_builtin` instruction's `srcs[0]`.
    fn builtin_name(instr: &IIRInstr) -> &str {
        match instr.srcs.first() {
            Some(Operand::Var(n)) => n.as_str(),
            _ => "<not-a-builtin>",
        }
    }

    #[test]
    fn runtime_cons_is_renamed_not_expanded() {
        // The runtime lowering does NOT expand cons to alloc+field_store —
        // it stays a single `call_builtin`, just renamed to `lispy_cons`.
        let mut m = make_module(vec![cons_call("%h", "%t")]);
        lower_heap_builtins_runtime(&mut m);
        assert_eq!(m.functions[0].instructions.len(), 1, "cons must stay one instr");
        let instr = &m.functions[0].instructions[0];
        assert_eq!(instr.op, "call_builtin");
        assert_eq!(builtin_name(instr), "lispy_cons");
    }

    #[test]
    fn runtime_cons_preserves_dest_and_args() {
        // The rename must keep the dest (%cell) and the head/tail operands.
        let mut m = make_module(vec![cons_call("%h", "%t")]);
        lower_heap_builtins_runtime(&mut m);
        let instr = &m.functions[0].instructions[0];
        assert_eq!(instr.dest.as_deref(), Some("%cell"));
        // srcs = [Var("lispy_cons"), Var("%h"), Var("%t")] — args unchanged.
        assert_eq!(instr.srcs[1], Operand::Var("%h".into()));
        assert_eq!(instr.srcs[2], Operand::Var("%t".into()));
    }

    #[test]
    fn runtime_car_and_cdr_are_renamed() {
        let car = IIRInstr::new(
            "call_builtin",
            Some("%head".into()),
            vec![Operand::Var("car".into()), Operand::Var("%pair".into())],
            "any",
        );
        let cdr = IIRInstr::new(
            "call_builtin",
            Some("%tail".into()),
            vec![Operand::Var("cdr".into()), Operand::Var("%pair".into())],
            "any",
        );
        let mut m = make_module(vec![car, cdr]);
        lower_heap_builtins_runtime(&mut m);
        assert_eq!(builtin_name(&m.functions[0].instructions[0]), "lispy_car");
        assert_eq!(builtin_name(&m.functions[0].instructions[1]), "lispy_cdr");
        // car/cdr stay 1-arg, dest preserved.
        assert_eq!(m.functions[0].instructions[0].srcs[1], Operand::Var("%pair".into()));
        assert_eq!(m.functions[0].instructions[0].dest.as_deref(), Some("%head"));
    }

    #[test]
    fn runtime_renames_atom_eq_predicates() {
        // L3b-2c-2: pair?/equal? are renamed here (unambiguous lisp builtins).
        // `not` is renamed type-directed in lisp_repr (it is also a numeric
        // builtin), so it is NOT renamed by this pass — see
        // `runtime_leaves_not_for_type_directed_rename`.
        for (name, renamed) in [
            ("pair?", "lispy_pair_p"),
            ("equal?", "lispy_equal"),
        ] {
            let instr = IIRInstr::new(
                "call_builtin",
                Some("%r".into()),
                vec![Operand::Var(name.into()), Operand::Var("%x".into())],
                "any",
            );
            let mut m = make_module(vec![instr]);
            lower_heap_builtins_runtime(&mut m);
            assert_eq!(
                builtin_name(&m.functions[0].instructions[0]), renamed,
                "{name} must be renamed to {renamed}",
            );
        }
    }

    #[test]
    fn runtime_leaves_not_for_type_directed_rename() {
        // `not` is a numeric builtin too (machine boolean-not), so this pass
        // must NOT rename it — lisp_repr renames it only when its arg is a
        // lispy_* result (ATOM = not(pair?)). Renaming here would hijack Twig.
        let instr = IIRInstr::new(
            "call_builtin",
            Some("%r".into()),
            vec![Operand::Var("not".into()), Operand::Var("%x".into())],
            "bool",
        );
        let mut m = make_module(vec![instr]);
        lower_heap_builtins_runtime(&mut m);
        assert_eq!(builtin_name(&m.functions[0].instructions[0]), "not");
    }

    #[test]
    fn runtime_leaves_symbol_and_nil_builtins_unchanged() {
        // null?/make_nil/make_symbol are NOT renamed yet — make_symbol needs
        // string-literal emission (L3b-2c-3).
        for name in ["null?", "make_nil", "make_symbol"] {
            let instr = IIRInstr::new(
                "call_builtin",
                Some("%r".into()),
                vec![Operand::Var(name.into()), Operand::Var("%x".into())],
                "any",
            );
            let mut m = make_module(vec![instr]);
            lower_heap_builtins_runtime(&mut m);
            assert_eq!(
                builtin_name(&m.functions[0].instructions[0]), name,
                "{name} must be left for L3b-2c-3, not renamed",
            );
        }
    }

    #[test]
    fn runtime_lowering_is_noop_for_non_lisp_module() {
        // A module with no cons/car/cdr (e.g. a Twig/Nib arithmetic program)
        // is left byte-for-byte unchanged — no regression for non-lisp code.
        let arith = IIRInstr::new(
            "add",
            Some("%s".into()),
            vec![Operand::Var("%a".into()), Operand::Var("%b".into())],
            "i64",
        );
        let mut m = make_module(vec![arith.clone()]);
        lower_heap_builtins_runtime(&mut m);
        assert_eq!(m.functions[0].instructions.len(), 1);
        assert_eq!(m.functions[0].instructions[0].op, "add");
    }
}
