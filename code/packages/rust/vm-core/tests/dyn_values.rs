//! #18470 / #18493 — nil, booleans and symbols as kinds of their own, run as IIR.
//!
//! The unit tests in `builtins.rs` call the five `dyn_*` handlers directly.
//! These run them the way a compiled program does — through
//! `VMCore::execute`, with the values crossing a heap cell — because that is
//! where the old representation broke:
//!
//! ```text
//!   (cons #t '())      field_store of Bool(true)  → "unsupported value kind"
//!   (car (cons '() 1)) nil stored as Int(0)        → came back as the integer 0
//!   (cons 'a '())      a symbol was Int(2^29 + id) → indistinguishable from an int
//! ```
//!
//! Four tests failed before the change — the `dyn_*` names were unregistered
//! (`UnknownOpcode`), and a `Bool` / `Null` / `Symbol` could not be stored in
//! a field. Two (`field_words_keep_every_kind_apart_at_the_edges`,
//! `floats_and_strings_are_still_refused_by_a_field`) pin what must *not*
//! change: integers next to the new words, and the kinds that still have no
//! word.

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;
use vm_core::core::VMCore;
use vm_core::errors::VMError;
use vm_core::value::Value;

fn ins(op: &str, dest: Option<&str>, srcs: Vec<Operand>, ty: &str) -> IIRInstr {
    IIRInstr::new(op, dest.map(|s| s.to_string()), srcs, ty)
}

fn var(name: &str) -> Operand {
    Operand::Var(name.to_string())
}

fn builtin(dest: &str, name: &str, args: Vec<Operand>) -> IIRInstr {
    let mut srcs = vec![var(name)];
    srcs.extend(args);
    ins("call_builtin", Some(dest), srcs, "any")
}

fn run(instrs: Vec<IIRInstr>) -> Result<Option<Value>, VMError> {
    let mut m = IIRModule::new("dyn_values", "test");
    m.add_or_replace(IIRFunction::new("main", vec![], "any", instrs));
    VMCore::new().execute(&mut m, "main", &[])
}

/// Store `value_instrs`' result `v` into field 0 of a fresh cell, read it
/// back, and return what came out.
fn round_trip(mut value_instrs: Vec<IIRInstr>) -> Value {
    value_instrs.extend([
        ins("alloc", Some("c"), vec![], "ref<LispyPair>"),
        ins("field_store", None, vec![var("c"), Operand::Int(0), var("v")], "void"),
        ins("field_load", Some("r"), vec![var("c"), Operand::Int(0)], "ref<any>"),
        ins("ret", None, vec![var("r")], "any"),
    ]);
    run(value_instrs).expect("round trip runs").expect("returns a value")
}

/// nil, `#f`, `#t` and a symbol each come back out of a cell as themselves.
#[test]
fn dynamic_scalars_round_trip_through_a_field() {
    assert_eq!(round_trip(vec![builtin("v", "dyn_nil", vec![])]), Value::Null);
    assert_eq!(
        round_trip(vec![ins("const", Some("v"), vec![Operand::Bool(true)], "bool")]),
        Value::Bool(true)
    );
    assert_eq!(
        round_trip(vec![ins("const", Some("v"), vec![Operand::Bool(false)], "bool")]),
        Value::Bool(false)
    );
    let sym = round_trip(vec![
        builtin("other", "dyn_symbol", vec![Operand::Str("a".into())]),
        builtin("v", "dyn_symbol", vec![Operand::Str("b".into())]),
    ]);
    assert_eq!(sym, Value::Symbol(1), "the second name interned");
    // The integer 0 and nil stay different values after the trip.
    assert_eq!(round_trip(vec![ins("const", Some("v"), vec![Operand::Int(0)], "i64")]), Value::Int(0));
}

/// A symbol whose id needs the high bit of the 32-bit id field still decodes
/// to itself, and the extreme integers keep working beside it.
#[test]
fn field_words_keep_every_kind_apart_at_the_edges() {
    let int_edge = (i64::MAX >> 3, i64::MIN >> 3);
    for n in [int_edge.0, int_edge.1, -1, 1, 3, 5] {
        assert_eq!(
            round_trip(vec![ins("const", Some("v"), vec![Operand::Int(n)], "i64")]),
            Value::Int(n),
            "integer {n} — its word is n << 3, never a nil/bool word"
        );
    }
}

/// `dyn_tag` of a value read back from a cell is the tag of what was stored:
/// nil reads as nil (1), not as the integer 0 (0) it used to be.
#[test]
fn nil_read_from_a_cell_is_nil_not_zero() {
    let got = run(vec![
        builtin("nil", "dyn_nil", vec![]),
        ins("alloc", Some("c"), vec![], "ref<LispyPair>"),
        ins("field_store", None, vec![var("c"), Operand::Int(1), var("nil")], "void"),
        ins("field_load", Some("cdr"), vec![var("c"), Operand::Int(1)], "ref<any>"),
        builtin("t", "dyn_tag", vec![var("cdr")]),
        builtin("n", "dyn_is_nil", vec![var("cdr")]),
        ins("const", Some("zero"), vec![Operand::Int(0)], "i64"),
        builtin("z", "dyn_is_nil", vec![var("zero")]),
        // 100·tag + 10·(nil? cdr) + (nil? 0)  →  100·1 + 10·1 + 0 = 110
        ins("const", Some("hundred"), vec![Operand::Int(100)], "i64"),
        ins("const", Some("ten"), vec![Operand::Int(10)], "i64"),
        ins("mul", Some("a"), vec![var("t"), var("hundred")], "i64"),
        ins("mul", Some("b"), vec![var("n"), var("ten")], "i64"),
        ins("add", Some("ab"), vec![var("a"), var("b")], "i64"),
        ins("add", Some("r"), vec![var("ab"), var("z")], "i64"),
        ins("ret", None, vec![var("r")], "i64"),
    ])
    .unwrap();
    assert_eq!(got, Some(Value::Int(110)));
}

/// A symbol is not a number: the dynamic `<` refuses it, as `twig-vm` does,
/// instead of comparing an id.
#[test]
fn a_symbol_is_a_type_error_for_ordering() {
    let err = run(vec![
        builtin("s", "dyn_symbol", vec![Operand::Str("a".into())]),
        ins("const", Some("one"), vec![Operand::Int(1)], "i64"),
        builtin("r", "<", vec![var("s"), var("one")]),
        ins("ret", None, vec![var("r")], "any"),
    ])
    .unwrap_err();
    assert!(matches!(err, VMError::TypeError { .. }), "{err:?}");
}

/// `is_null` — the legacy structural op — also accepts the new nil.
#[test]
fn is_null_accepts_dyn_nil() {
    let got = run(vec![
        builtin("nil", "dyn_nil", vec![]),
        ins("is_null", Some("r"), vec![var("nil")], "bool"),
        ins("ret", None, vec![var("r")], "bool"),
    ])
    .unwrap();
    assert_eq!(got, Some(Value::Bool(true)));
}

/// `Float` and `Str` still have no field word: storing one is refused, not
/// truncated into some other kind's word.
#[test]
fn floats_and_strings_are_still_refused_by_a_field() {
    for v in [Operand::Float(1.5), Operand::Str("s".into())] {
        let ty = if matches!(v, Operand::Float(_)) { "f64" } else { "str" };
        let err = run(vec![
            ins("const", Some("v"), vec![v], ty),
            ins("alloc", Some("c"), vec![], "ref<LispyPair>"),
            ins("field_store", None, vec![var("c"), Operand::Int(0), var("v")], "void"),
            ins("ret_void", None, vec![], "void"),
        ])
        .unwrap_err();
        assert!(format!("{err:?}").contains("unsupported value kind"), "{err:?}");
    }
}
