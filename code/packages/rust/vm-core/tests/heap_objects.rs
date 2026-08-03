//! E6d heap objects on the generic VM — `alloc` / `field_store` / `field_load`.
//!
//! A Twig record/union/closure builds `(car . cdr)` cons cells with the
//! word-granular heap ops the native/LLVM (`__twig_gc_alloc`) and structural
//! (`object[]`) backends already run. These tests confirm the generic
//! `vm-core` now executes them too, on the **real, collected** `FlatHeap`
//! (`alloc`/`field_store`/`field_load` are direct aliases for `gc_alloc`/
//! `gc_field_store`/`gc_field_load` — see `dispatch.rs`'s module comment): a
//! cons cell is a 2-word object by default, and `field_store`/`field_load`
//! write/read a field by index.

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;
use vm_core::core::VMCore;
use vm_core::value::Value;

fn ins(op: &str, dest: Option<&str>, srcs: Vec<Operand>, ty: &str) -> IIRInstr {
    IIRInstr::new(op, dest.map(|s| s.to_string()), srcs, ty)
}

fn run(instrs: Vec<IIRInstr>) -> Option<Value> {
    let f = IIRFunction::new("main", vec![], "i64", instrs);
    let mut m = IIRModule::new("heap", "heap");
    m.add_or_replace(f);
    VMCore::new().execute(&mut m, "main", &[]).unwrap()
}

/// `c = alloc; c[0] = 42; c[1] = 7; return c[0]` ⇒ 42, and reading `c[1]` ⇒ 7.
#[test]
fn alloc_store_load_roundtrips_both_fields() {
    let store = |idx: i64| {
        vec![
            ins("const", Some("v0"), vec![Operand::Int(42)], "i64"),
            ins("const", Some("v1"), vec![Operand::Int(7)], "i64"),
            ins("alloc", Some("c"), vec![], "ref<LispyPair>"),
            ins("field_store", None, vec![Operand::Var("c".into()), Operand::Int(0), Operand::Var("v0".into())], "void"),
            ins("field_store", None, vec![Operand::Var("c".into()), Operand::Int(1), Operand::Var("v1".into())], "void"),
            ins("field_load", Some("r"), vec![Operand::Var("c".into()), Operand::Int(idx)], "ref<any>"),
            ins("ret", None, vec![Operand::Var("r".into())], "i64"),
        ]
    };
    assert_eq!(run(store(0)), Some(Value::Int(42)), "car (field 0)");
    assert_eq!(run(store(1)), Some(Value::Int(7)), "cdr (field 1)");
}

/// A cons chain `(42 . (7 . nil))`: `field_load[1]` then `field_load[0]` reaches
/// the second element — the exact walk a record's second accessor performs.
#[test]
fn nested_cons_chain_second_field() {
    assert_eq!(
        run(vec![
            ins("const", Some("a"), vec![Operand::Int(42)], "i64"),
            ins("const", Some("b"), vec![Operand::Int(7)], "i64"),
            ins("const", Some("nil"), vec![Operand::Int(0)], "ref<LispyPair>"),
            // inner = (7 . nil)
            ins("alloc", Some("inner"), vec![], "ref<LispyPair>"),
            ins("field_store", None, vec![Operand::Var("inner".into()), Operand::Int(0), Operand::Var("b".into())], "void"),
            ins("field_store", None, vec![Operand::Var("inner".into()), Operand::Int(1), Operand::Var("nil".into())], "void"),
            // outer = (42 . inner)
            ins("alloc", Some("outer"), vec![], "ref<LispyPair>"),
            ins("field_store", None, vec![Operand::Var("outer".into()), Operand::Int(0), Operand::Var("a".into())], "void"),
            ins("field_store", None, vec![Operand::Var("outer".into()), Operand::Int(1), Operand::Var("inner".into())], "void"),
            // (car (cdr outer)) = 7
            ins("field_load", Some("cd"), vec![Operand::Var("outer".into()), Operand::Int(1)], "ref<any>"),
            ins("field_load", Some("r"), vec![Operand::Var("cd".into()), Operand::Int(0)], "ref<any>"),
            ins("ret", None, vec![Operand::Var("r".into())], "i64"),
        ]),
        Some(Value::Int(7))
    );
}

/// Distinct `alloc`s get distinct handles — a store to one object must not alias
/// another. `c1[0]=1; c2[0]=2; return c1[0]` ⇒ 1 (not 2).
#[test]
fn distinct_allocs_do_not_alias() {
    assert_eq!(
        run(vec![
            ins("const", Some("one"), vec![Operand::Int(1)], "i64"),
            ins("const", Some("two"), vec![Operand::Int(2)], "i64"),
            ins("alloc", Some("c1"), vec![], "ref<LispyPair>"),
            ins("alloc", Some("c2"), vec![], "ref<LispyPair>"),
            ins("field_store", None, vec![Operand::Var("c1".into()), Operand::Int(0), Operand::Var("one".into())], "void"),
            ins("field_store", None, vec![Operand::Var("c2".into()), Operand::Int(0), Operand::Var("two".into())], "void"),
            ins("field_load", Some("r"), vec![Operand::Var("c1".into()), Operand::Int(0)], "ref<any>"),
            ins("ret", None, vec![Operand::Var("r".into())], "i64"),
        ]),
        Some(Value::Int(1))
    );
}

/// `is_null`: nil (`const Int(0) : ref<LispyPair>`) is null; a real cons cell
/// (a genuine `Value::HeapRef` from `alloc`) is not. `c = alloc; return
/// is_null(c)` ⇒ false, and `is_null(const 0)` ⇒ true.
#[test]
fn is_null_distinguishes_nil_from_first_object() {
    // A freshly-allocated object is a HeapRef, never the Int(0) nil sentinel.
    assert_eq!(
        run(vec![
            ins("alloc", Some("c"), vec![], "ref<LispyPair>"),
            ins("is_null", Some("r"), vec![Operand::Var("c".into())], "bool"),
            ins("ret", None, vec![Operand::Var("r".into())], "bool"),
        ]),
        Some(Value::Bool(false)),
        "a freshly allocated object must not read as nil"
    );
    // The nil sentinel (const Int(0) : ref<LispyPair>) IS nil.
    assert_eq!(
        run(vec![
            ins("const", Some("nil"), vec![Operand::Int(0)], "ref<LispyPair>"),
            ins("is_null", Some("r"), vec![Operand::Var("nil".into())], "bool"),
            ins("ret", None, vec![Operand::Var("r".into())], "bool"),
        ]),
        Some(Value::Bool(true))
    );
}

/// A field that itself holds nil, read back through `field_load` with a
/// `"ref<...>"` type hint (the shape a `cdr` accessor uses), must still read
/// as null. Under the tag-based decode this round-trips as `Value::Int(0)`
/// (tag `000`, same as the top-level sentinel), not a `HeapRef` — exercising
/// the ordinary `Value::Int(0) => true` arm of `is_null`, not the defensive
/// `HeapRef::is_null()` one (which nothing in `dispatch.rs` currently
/// produces; see `handle_is_null`'s doc comment).
#[test]
fn is_null_recognizes_nil_stored_and_reloaded_through_a_field() {
    assert_eq!(
        run(vec![
            ins("const", Some("nil"), vec![Operand::Int(0)], "ref<LispyPair>"),
            ins("alloc", Some("c"), vec![], "ref<LispyPair>"),
            ins("field_store", None, vec![Operand::Var("c".into()), Operand::Int(1), Operand::Var("nil".into())], "void"),
            ins("field_load", Some("cdr"), vec![Operand::Var("c".into()), Operand::Int(1)], "ref<LispyPair>"),
            ins("is_null", Some("r"), vec![Operand::Var("cdr".into())], "bool"),
            ins("ret", None, vec![Operand::Var("r".into())], "bool"),
        ]),
        Some(Value::Bool(true)),
        "nil round-tripped through a field must still read as null"
    );
}

/// The core regression this reroute must not reintroduce: a cons cell field
/// is dynamically typed (`ref<any>`) — it can hold either a nested pair or a
/// plain integer depending on runtime data, and the *load* instruction's
/// type hint can't tell you which (both accessors use the same generic
/// `"ref<any>"` hint, exactly as real Twig lowering emits for `car`/`cdr`).
/// This proves both cases round-trip correctly out of the *same* field
/// position, decoded purely from the stored word's own tag bits.
#[test]
fn field_load_disambiguates_int_and_nested_pair_from_the_same_dynamically_typed_field() {
    // field 0 holds a plain integer.
    assert_eq!(
        run(vec![
            ins("const", Some("v"), vec![Operand::Int(42)], "i64"),
            ins("alloc", Some("c"), vec![], "ref<LispyPair>"),
            ins("field_store", None, vec![Operand::Var("c".into()), Operand::Int(0), Operand::Var("v".into())], "void"),
            ins("field_load", Some("r"), vec![Operand::Var("c".into()), Operand::Int(0)], "ref<any>"),
            ins("ret", None, vec![Operand::Var("r".into())], "i64"),
        ]),
        Some(Value::Int(42)),
        "a plain integer in a ref<any> field must decode as Int, not HeapRef"
    );
    // field 0 (same index, same type hint) holds a nested pair instead.
    let result = run(vec![
        ins("alloc", Some("inner"), vec![], "ref<LispyPair>"),
        ins("alloc", Some("outer"), vec![], "ref<LispyPair>"),
        ins("field_store", None, vec![Operand::Var("outer".into()), Operand::Int(0), Operand::Var("inner".into())], "void"),
        ins("field_load", Some("r"), vec![Operand::Var("outer".into()), Operand::Int(0)], "ref<any>"),
        ins("is_null", Some("n"), vec![Operand::Var("r".into())], "bool"),
        ins("ret", None, vec![Operand::Var("n".into())], "bool"),
    ]);
    assert_eq!(
        result,
        Some(Value::Bool(false)),
        "a nested pair in the same ref<any> field position must decode as a non-null HeapRef, not be misread as an integer"
    );
}

/// An integer outside the tag scheme's representable range (`[i64::MIN >>
/// 3, i64::MAX >> 3]`) is rejected by `field_store` rather than silently
/// truncated — the field storage layer traps cleanly instead of corrupting
/// data.
#[test]
fn field_store_rejects_an_integer_too_large_for_the_tag_scheme() {
    let f = IIRFunction::new(
        "main", vec![], "i64",
        vec![
            ins("const", Some("huge"), vec![Operand::Int(i64::MAX)], "i64"),
            ins("alloc", Some("c"), vec![], "ref<LispyPair>"),
            ins("field_store", None, vec![Operand::Var("c".into()), Operand::Int(0), Operand::Var("huge".into())], "void"),
            ins("ret", None, vec![Operand::Int(0)], "i64"),
        ],
    );
    let mut m = IIRModule::new("heap", "heap");
    m.add_or_replace(f);
    let r = VMCore::new().execute(&mut m, "main", &[]);
    assert!(r.is_err(), "storing i64::MAX into a dynamically-typed field must trap, got {r:?}");
}
