//! `vm_lisp`'s output, run on the VM it is written for (#18470, #18493).
//!
//! The unit tests in `src/vm_lisp.rs` check the *shape* of each rewrite. These
//! check what it *means*: each module below is written the way a Lisp-family
//! frontend writes it (nil as `const 0 : ref<LispyPair>`, a symbol as
//! `const Var(name) : symbol`, a source-level `call_builtin "not"` …), lowered
//! with `lower_lisp_for_vm`, and executed by `vm-core`. Every rule is checked
//! over every kind of value against TW00's definition, stated independently
//! in each table.
//!
//! Without the pass every one of these fails on the VM: the predicates and the
//! division names are `UnknownOpcode`, `(if 0 …)` takes the else branch, and
//! `(null? 0)` is `#t`.

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_builtin_lowering::vm_lisp::lower_lisp_for_vm;
use vm_core::core::VMCore;
use vm_core::errors::VMError;
use vm_core::value::Value;

fn ins(op: &str, dest: Option<&str>, srcs: Vec<Operand>, ty: &str) -> IIRInstr {
    IIRInstr::new(op, dest.map(str::to_string), srcs, ty)
}

fn var(n: &str) -> Operand {
    Operand::Var(n.to_string())
}

/// One Twig value, written as the frontend writes its literal, into `dest`.
#[derive(Clone, Copy, Debug)]
enum Lit {
    Int(i64),
    Bool(bool),
    Nil,
    Sym(&'static str),
    Str(&'static str),
    /// `(cons 1 nil)`, through the structural ops `lower_heap_builtins` emits.
    Pair,
}

fn emit(lit: Lit, dest: &str, out: &mut Vec<IIRInstr>) {
    match lit {
        Lit::Int(n) => out.push(ins("const", Some(dest), vec![Operand::Int(n)], "i64")),
        Lit::Bool(b) => out.push(ins("const", Some(dest), vec![Operand::Bool(b)], "bool")),
        Lit::Nil => out.push(ins("const", Some(dest), vec![Operand::Int(0)], "ref<LispyPair>")),
        Lit::Sym(name) => out.push(ins("const", Some(dest), vec![var(name)], "symbol")),
        Lit::Str(s) => out.push(ins("const", Some(dest), vec![Operand::Str(s.into())], "str")),
        Lit::Pair => {
            let (h, t) = (format!("{dest}_h"), format!("{dest}_t"));
            emit(Lit::Int(1), &h, out);
            emit(Lit::Nil, &t, out);
            out.push(ins("alloc", Some(dest), vec![], "ref<LispyPair>"));
            out.push(ins("field_store", None, vec![var(dest), Operand::Int(0), var(&h)], "void"));
            out.push(ins("field_store", None, vec![var(dest), Operand::Int(1), var(&t)], "void"));
        }
    }
}

/// Lower `body` as a Lisp-family `main` and run it on the VM.
fn run(body: Vec<IIRInstr>) -> Result<Option<Value>, VMError> {
    let mut m = IIRModule::new("t", "twig");
    m.add_or_replace(IIRFunction::new("main", vec![], "any", body));
    lower_lisp_for_vm(&mut m);
    VMCore::new().execute(&mut m, "main", &[])
}

/// `(if x 1 2)` — the branch the frontend writes, on a value of every kind.
fn if_then_else(x: Lit) -> i64 {
    let mut b = Vec::new();
    emit(x, "x", &mut b);
    b.extend([
        ins("jmp_if_false", None, vec![var("x"), var("else")], "void"),
        ins("const", Some("r"), vec![Operand::Int(1)], "i64"),
        ins("ret", None, vec![var("r")], "i64"),
        ins("label", None, vec![var("else")], "void"),
        ins("const", Some("r2"), vec![Operand::Int(2)], "i64"),
        ins("ret", None, vec![var("r2")], "i64"),
    ]);
    match run(b).expect("runs") {
        Some(Value::Int(n)) => n,
        other => panic!("{x:?}: {other:?}"),
    }
}

/// `(<name> x)` for a one-argument source builtin.
fn predicate(name: &str, x: Lit) -> Value {
    let mut b = Vec::new();
    emit(x, "x", &mut b);
    b.push(ins("call_builtin", Some("r"), vec![var(name), var("x")], "any"));
    b.push(ins("ret", None, vec![var("r")], "any"));
    run(b).unwrap_or_else(|e| panic!("({name} {x:?}): {e:?}")).expect("a value")
}

const EVERY_KIND: [Lit; 9] = [
    Lit::Int(0),
    Lit::Int(-3),
    Lit::Bool(true),
    Lit::Bool(false),
    Lit::Nil,
    Lit::Sym("a"),
    Lit::Str(""),
    Lit::Str("s"),
    Lit::Pair,
];

/// TW00: "only `#f` and `nil` are false; everything else is true."
fn tw00_truthy(x: Lit) -> bool {
    !matches!(x, Lit::Bool(false) | Lit::Nil)
}

#[test]
fn truthiness_is_tw00_for_every_kind() {
    for x in EVERY_KIND {
        let want = if tw00_truthy(x) { 1 } else { 2 };
        assert_eq!(if_then_else(x), want, "(if {x:?} 1 2)");
    }
}

#[test]
fn not_is_the_negation_of_truthiness() {
    for x in EVERY_KIND {
        assert_eq!(predicate("not", x), Value::Bool(!tw00_truthy(x)), "(not {x:?})");
    }
}

#[test]
fn type_predicates_name_exactly_one_kind() {
    for x in EVERY_KIND {
        let number = matches!(x, Lit::Int(_));
        let symbol = matches!(x, Lit::Sym(_));
        let boolean = matches!(x, Lit::Bool(_));
        let null = matches!(x, Lit::Nil);
        let pair = matches!(x, Lit::Pair);
        assert_eq!(predicate("number?", x), Value::Bool(number), "(number? {x:?})");
        assert_eq!(predicate("symbol?", x), Value::Bool(symbol), "(symbol? {x:?})");
        assert_eq!(predicate("boolean?", x), Value::Bool(boolean), "(boolean? {x:?})");
        assert_eq!(predicate("pair?", x), Value::Bool(pair), "(pair? {x:?})");
        // `null?` reaches the VM as `is_null` (lower_heap_builtins).
        let mut b = Vec::new();
        emit(x, "x", &mut b);
        b.push(ins("is_null", Some("r"), vec![var("x")], "bool"));
        b.push(ins("ret", None, vec![var("r")], "bool"));
        assert_eq!(run(b).unwrap(), Some(Value::Bool(null)), "(null? {x:?})");
    }
}

/// `list?` — nil and a nil-terminated chain are lists; an atom, and a pair
/// whose last rest is an atom, are not.
#[test]
fn list_p_walks_to_nil() {
    for x in EVERY_KIND {
        let want = matches!(x, Lit::Nil | Lit::Pair);
        assert_eq!(predicate("list?", x), Value::Bool(want), "(list? {x:?})");
    }
    // (cons 1 (cons 2 3)) — improper; (cons 1 (cons 2 nil)) — proper.
    for (tail, want) in [(Lit::Int(3), false), (Lit::Nil, true)] {
        let mut b = Vec::new();
        emit(Lit::Int(1), "one", &mut b);
        emit(Lit::Int(2), "two", &mut b);
        emit(tail, "end", &mut b);
        b.extend([
            ins("alloc", Some("inner"), vec![], "ref<LispyPair>"),
            ins("field_store", None, vec![var("inner"), Operand::Int(0), var("two")], "void"),
            ins("field_store", None, vec![var("inner"), Operand::Int(1), var("end")], "void"),
            ins("alloc", Some("outer"), vec![], "ref<LispyPair>"),
            ins("field_store", None, vec![var("outer"), Operand::Int(0), var("one")], "void"),
            ins("field_store", None, vec![var("outer"), Operand::Int(1), var("inner")], "void"),
            ins("call_builtin", Some("r"), vec![var("list?"), var("outer")], "any"),
            ins("ret", None, vec![var("r")], "any"),
        ]);
        assert_eq!(run(b).unwrap(), Some(Value::Bool(want)), "tail {tail:?}");
    }
}

/// Symbols keep their identity through the rewrite, and through a cell.
#[test]
fn symbols_are_equal_by_name_and_never_an_integer() {
    let eq = |a: Lit, b: Lit| {
        let mut body = Vec::new();
        emit(a, "a", &mut body);
        emit(b, "b", &mut body);
        body.push(ins("call_builtin", Some("r"), vec![var("="), var("a"), var("b")], "any"));
        body.push(ins("ret", None, vec![var("r")], "any"));
        run(body).unwrap().unwrap()
    };
    assert_eq!(eq(Lit::Sym("a"), Lit::Sym("a")), Value::Bool(true));
    assert_eq!(eq(Lit::Sym("a"), Lit::Sym("b")), Value::Bool(false));
    // The old reserved-range encoding made 'a equal to 2^29 (its id).
    assert_eq!(eq(Lit::Sym("a"), Lit::Int(1 << 29)), Value::Bool(false));
    assert_eq!(eq(Lit::Sym("a"), Lit::Int(0)), Value::Bool(false));
    // LANG85 §11.1 decision 4: (= 0 #f) is false; so is (= 0 '()).
    assert_eq!(eq(Lit::Int(0), Lit::Bool(false)), Value::Bool(false));
    assert_eq!(eq(Lit::Int(0), Lit::Nil), Value::Bool(false));
}

/// The dynamic division names, with twig-vm's rounding (`dynval-runtime`):
/// truncating `/`, `quotient`, `remainder`; floored `modulo`.
#[test]
fn division_rounds_as_twig_vm_does() {
    let div = |name: &str, a: i64, b: i64| {
        let body = vec![
            // Parameters reach these as `any`; a `mov` hides the literal type.
            ins("const", Some("a0"), vec![Operand::Int(a)], "i64"),
            ins("mov", Some("a"), vec![var("a0")], "any"),
            ins("const", Some("b0"), vec![Operand::Int(b)], "i64"),
            ins("mov", Some("b"), vec![var("b0")], "any"),
            ins("call_builtin", Some("r"), vec![var(name), var("a"), var("b")], "any"),
            ins("ret", None, vec![var("r")], "any"),
        ];
        run(body)
    };
    // a, b, truncated quotient, remainder (sign of a), modulo (sign of b)
    let table: [(i64, i64, i64, i64, i64); 5] = [
        (7, 2, 3, 1, 1),
        (-7, 2, -3, -1, 1),
        (7, -2, -3, 1, -1),
        (-7, -2, 3, -1, -1),
        (6, 3, 2, 0, 0),
    ];
    for (a, b, q, r, m) in table {
        assert_eq!(div("/", a, b).unwrap(), Some(Value::Int(q)), "(/ {a} {b})");
        assert_eq!(div("quotient", a, b).unwrap(), Some(Value::Int(q)), "(quotient {a} {b})");
        assert_eq!(div("remainder", a, b).unwrap(), Some(Value::Int(r)), "(remainder {a} {b})");
        assert_eq!(div("modulo", a, b).unwrap(), Some(Value::Int(m)), "(modulo {a} {b})");
    }
    assert_eq!(div("modulo", -13, 4).unwrap(), Some(Value::Int(3)), "dynval-runtime's own example");
    assert_eq!(div("modulo", 13, -4).unwrap(), Some(Value::Int(-3)), "dynval-runtime's own example");
    for name in ["/", "quotient", "remainder", "modulo"] {
        assert!(div(name, 1, 0).is_err(), "({name} 1 0) stops the program");
    }
}
