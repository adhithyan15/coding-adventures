//! # symbol_intern — compile-time symbol interning for the native AOT path (LANG77 / L3b-2c-3).
//!
//! ## What a symbol is, and why intern at compile time
//!
//! A `lispy-runtime` symbol is a tagged immediate: the [`SymbolId`] in the high
//! 32 bits, the tag `0b010` in the low bits — `(id << 32) | TAG_SYMBOL`. Two
//! symbols with the same id compare bitwise-equal, so `EQ`/`equal?` on symbols
//! is just word equality, and interning (one id per distinct name) is what
//! makes that sound.
//!
//! The frontend lowers a symbol *literal* to `const Var(name) : symbol` — it
//! carries the *name*, deferring id assignment to whoever interns. The VM and
//! the managed backends intern at **runtime** (via `lispy-runtime`'s interner).
//! The native backend can't: it has no string-constant support, and a static
//! McCarthy program never needs a symbol's *name* at runtime — only its
//! *identity* (for `EQ`) and its *truthiness* (for `COND`). So we intern at
//! **compile time**: assign each distinct name a small id and replace the
//! const with the finished tagged immediate `const Int((id << 32) | 0b010)`.
//!
//! This is a general, language-agnostic pass — any lisp frontend's symbol
//! consts are interned the same way. The ids are module-local (they need not,
//! and do not, match the VM's runtime ids); all that matters is that within one
//! compiled program the *same name maps to the same id*, which is exactly what
//! makes `(EQ 'A 'A)` true and `(EQ 'A 'B)` false on native.
//!
//! ## Why not runtime `make_symbol` + string literals?
//!
//! That path (emit the name bytes into a data section, call
//! `__dyn_make_symbol(ptr, len)`) is what you need to *print* a symbol's
//! name or create symbols dynamically (`read`/`gensym`/`eval`). The native
//! backend has no data-section string-constant machinery yet, and static
//! programs don't need it — so it is deferred. Compile-time interning delivers
//! the full symbol *value* model (cons of symbols, `CAR`/`CDR`, `ATOM`, `EQ`)
//! without it.

use interpreter_ir::instr::Operand;
use interpreter_ir::IIRModule;
use std::collections::HashMap;

/// The frontend type hint for a symbol literal (`mccarthy-lisp-iir-compiler`'s
/// `emit_symbol`). Mirrors `lispy-runtime`'s symbol tag.
const SYMBOL_HINT: &str = "symbol";

/// The symbol tag (`lispy-runtime` `TAG_SYMBOL = 0b010`).
const TAG_SYMBOL: i64 = 0b010;

/// The base of the reserved **symbol id range** for the *managed* (uniform-ref)
/// backends (WASM/JVM/CLR/BEAM) — see [`intern_symbols_structural`].
///
/// Those backends have no spare tag bit in their boxed-integer representation
/// (a WASM `i31ref` is a bare 31-bit payload), so a symbol is interned to a
/// distinct integer in a **high reserved range** that ordinary integer atoms do
/// not reach. `2²⁹` carves the 31-bit space into integers `[−2²⁹, 2²⁹)` and
/// symbol ids `[2²⁹, 2³⁰)`, so a symbol value can never equal an integer value
/// — `(EQ 'A 5)` is `nil`. (A program that mixes symbols with integer atoms
/// `≥ 2²⁹` is out of range, the documented analogue of the native `±2⁶⁰` limit;
/// McCarthy 1.0 programs use small integers.)
pub const SYMBOL_ID_BASE: i64 = 1 << 29;

/// Intern every symbol literal to a distinct integer in the reserved symbol-id
/// range, for the **managed / uniform-reference** backends (WASM today;
/// JVM/CLR/BEAM as they land). The twin of [`intern_symbols`], which targets the
/// native tagged-word model.
///
/// A frontend symbol literal is `const Var(name) : symbol`. We assign each
/// distinct name a module-wide id (first-seen order, so one id per name across
/// all functions — what makes cross-function `EQ` sound) and rewrite the const
/// to `const Int(SYMBOL_ID_BASE + id) : i32`. From there the value flows like
/// any integer atom: the structural representation pass boxes it as an `i31ref`,
/// and `EQ` compares the payloads with `i32.eq` — so `(EQ 'A 'A)` is true,
/// `(EQ 'A 'B)` is false, with no new value type or polymorphic `EQ`.
///
/// Run it **before** `lower_lisp_repr_structural` (so the pass sees a plain
/// integer atom). Idempotent: a `const : symbol` that is already an `Int`
/// (re-run) is left untouched; non-symbol consts are never touched.
pub fn intern_symbols_structural(module: &mut IIRModule) {
    let mut ids: HashMap<String, u32> = HashMap::new();
    let mut next_id: u32 = 0;

    for func in &mut module.functions {
        for instr in &mut func.instructions {
            if instr.op != "const" || instr.type_hint != SYMBOL_HINT {
                continue;
            }
            let name = match instr.srcs.first() {
                Some(Operand::Var(n)) => n.clone(),
                _ => continue, // already interned (Int), or malformed — leave it.
            };
            let id = *ids.entry(name).or_insert_with(|| {
                let id = next_id;
                next_id += 1;
                id
            });
            // The interned symbol value: a distinct integer in the reserved
            // range. Retype to i32 so it boxes as an i31ref like an integer atom.
            instr.srcs[0] = Operand::Int(SYMBOL_ID_BASE + id as i64);
            instr.type_hint = "i32".to_string();
        }
    }
}

/// Intern every symbol literal in `module` to its tagged immediate, in place.
///
/// Assigns ids in first-seen order across the **whole module** (so a name used
/// in two functions gets one id). Runs in `twig-aot::prepare_module_for_aot`
/// before `lower_lisp_repr` (so the representation pass sees finished symbol
/// immediates). A `const : symbol` that is already an `Int` (re-run, or some
/// other producer) is left untouched; a non-symbol const is never touched.
pub fn intern_symbols(module: &mut IIRModule) {
    let mut ids: HashMap<String, u32> = HashMap::new();
    let mut next_id: u32 = 0;

    for func in &mut module.functions {
        for instr in &mut func.instructions {
            if instr.op != "const" || instr.type_hint != SYMBOL_HINT {
                continue;
            }
            // A frontend symbol literal is `const Var(name) : symbol`.
            let name = match instr.srcs.first() {
                Some(Operand::Var(n)) => n.clone(),
                _ => continue, // already interned (Int), or malformed — leave it.
            };
            let id = match ids.get(&name) {
                Some(id) => *id,
                None => {
                    let id = next_id;
                    next_id += 1;
                    ids.insert(name, id);
                    id
                }
            };
            // The finished tagged symbol immediate: (id << 32) | TAG_SYMBOL.
            instr.srcs[0] = Operand::Int(((id as i64) << 32) | TAG_SYMBOL);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::{IIRFunction, IIRInstr, IIRModule};

    fn sym_const(dest: &str, name: &str) -> IIRInstr {
        IIRInstr::new("const", Some(dest.into()), vec![Operand::Var(name.into())], "symbol")
    }

    fn module(funcs: Vec<IIRFunction>) -> IIRModule {
        IIRModule {
            name: "t".into(),
            functions: funcs,
            entry_point: Some("main".into()),
            language: "mccarthy-lisp".into(),
            exports: vec![],
            imports: vec![],
        }
    }

    fn const_bits(m: &IIRModule, fi: usize, dest: &str) -> i64 {
        for i in &m.functions[fi].instructions {
            if i.op == "const" && i.dest.as_deref() == Some(dest) {
                if let Some(Operand::Int(n)) = i.srcs.first() {
                    return *n;
                }
            }
        }
        panic!("no const {dest}");
    }

    #[test]
    fn same_name_gets_same_id() {
        let f = IIRFunction::new(
            "main",
            vec![],
            "any",
            vec![sym_const("a", "A"), sym_const("a2", "A"), sym_const("b", "B")],
        );
        let mut m = module(vec![f]);
        intern_symbols(&mut m);
        // A appears twice → identical bits; B differs.
        assert_eq!(const_bits(&m, 0, "a"), const_bits(&m, 0, "a2"), "same name → same id");
        assert_ne!(const_bits(&m, 0, "a"), const_bits(&m, 0, "b"), "A and B differ");
    }

    #[test]
    fn encoding_is_id_shifted_plus_tag() {
        let f = IIRFunction::new("main", vec![], "any", vec![sym_const("a", "A")]);
        let mut m = module(vec![f]);
        intern_symbols(&mut m);
        let bits = const_bits(&m, 0, "a");
        // First symbol → id 0 → (0 << 32) | 0b010 = 2.
        assert_eq!(bits, TAG_SYMBOL, "first symbol is id 0");
        assert_eq!(bits & 0b111, TAG_SYMBOL, "low bits are the symbol tag");
    }

    #[test]
    fn ids_are_module_wide() {
        // A used in `helper` and `main` gets ONE id (so cross-function EQ works).
        let helper = IIRFunction::new("helper", vec![], "any", vec![sym_const("h", "A")]);
        let main = IIRFunction::new("main", vec![], "any", vec![sym_const("m", "A")]);
        let mut m = module(vec![helper, main]);
        intern_symbols(&mut m);
        let helper_bits = const_bits(&m, 0, "h");
        let main_bits = const_bits(&m, 1, "m");
        assert_eq!(helper_bits, main_bits, "same name across functions → same id");
    }

    // ── intern_symbols_structural (managed / uniform-ref backends) ────────────

    #[test]
    fn structural_distinct_names_get_distinct_reserved_ids() {
        let f = IIRFunction::new(
            "main",
            vec![],
            "any",
            vec![sym_const("a", "A"), sym_const("a2", "A"), sym_const("b", "B")],
        );
        let mut m = module(vec![f]);
        intern_symbols_structural(&mut m);
        // A → SYMBOL_ID_BASE + 0 (twice), B → SYMBOL_ID_BASE + 1.
        assert_eq!(const_bits(&m, 0, "a"), SYMBOL_ID_BASE, "first symbol id 0");
        assert_eq!(const_bits(&m, 0, "a2"), SYMBOL_ID_BASE, "same name → same id");
        assert_eq!(const_bits(&m, 0, "b"), SYMBOL_ID_BASE + 1, "B → id 1");
        assert_ne!(const_bits(&m, 0, "a"), const_bits(&m, 0, "b"), "A ≠ B");
    }

    #[test]
    fn structural_symbols_are_retyped_i32_and_disjoint_from_integers() {
        let f = IIRFunction::new("main", vec![], "any", vec![sym_const("a", "A")]);
        let mut m = module(vec![f]);
        intern_symbols_structural(&mut m);
        let instr = m.functions[0]
            .instructions
            .iter()
            .find(|i| i.dest.as_deref() == Some("a"))
            .unwrap();
        assert_eq!(instr.type_hint, "i32", "interned symbol retyped to i32 (boxes as i31ref)");
        // A symbol id is in the reserved high range, never reached by a small
        // integer atom, so `(EQ 'A 5)` is false.
        assert!(const_bits(&m, 0, "a") >= SYMBOL_ID_BASE, "symbol id in reserved range");
    }

    #[test]
    fn structural_ids_are_module_wide() {
        let helper = IIRFunction::new("helper", vec![], "any", vec![sym_const("h", "A")]);
        let main = IIRFunction::new("main", vec![], "any", vec![sym_const("m", "A")]);
        let mut m = module(vec![helper, main]);
        intern_symbols_structural(&mut m);
        assert_eq!(
            const_bits(&m, 0, "h"),
            const_bits(&m, 1, "m"),
            "same name across functions → same id"
        );
    }

    #[test]
    fn structural_non_symbol_consts_untouched() {
        let f = IIRFunction::new(
            "main",
            vec![],
            "any",
            vec![
                IIRInstr::new("const", Some("i".into()), vec![Operand::Int(42)], "i64"),
                IIRInstr::new("const", Some("n".into()), vec![Operand::Int(0)], "ref<LispyPair>"),
            ],
        );
        let mut m = module(vec![f]);
        intern_symbols_structural(&mut m);
        assert_eq!(const_bits(&m, 0, "i"), 42, "i64 const untouched");
        assert_eq!(const_bits(&m, 0, "n"), 0, "nil const untouched");
    }

    #[test]
    fn non_symbol_consts_untouched() {
        let f = IIRFunction::new(
            "main",
            vec![],
            "any",
            vec![
                IIRInstr::new("const", Some("i".into()), vec![Operand::Int(42)], "i64"),
                IIRInstr::new("const", Some("n".into()), vec![Operand::Int(0)], "ref<LispyPair>"),
            ],
        );
        let mut m = module(vec![f]);
        intern_symbols(&mut m);
        assert_eq!(const_bits(&m, 0, "i"), 42, "i64 const untouched");
        assert_eq!(const_bits(&m, 0, "n"), 0, "nil const untouched");
    }
}
