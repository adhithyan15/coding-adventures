//! E6d heap objects on the generic VM — `alloc` / `field_store` / `field_load`.
//!
//! A Twig record/union/closure builds `(car . cdr)` cons cells with the
//! word-granular heap ops the native/LLVM (`__twig_gc_alloc`) and structural
//! (`object[]`) backends already run. These tests confirm the generic `vm-core`
//! now executes them too, reusing its bounds-checked array heap: `alloc` reserves
//! a fixed-size object (default 2 words), and `field_store`/`field_load`
//! write/read a field by index — the same handle+index model as `array_set`/
//! `array_get`.

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
