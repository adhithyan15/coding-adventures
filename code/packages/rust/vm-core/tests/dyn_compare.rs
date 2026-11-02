//! #18463 — the dynamic comparison builtins and `pair?`, run as IIR.
//!
//! The unit tests in `builtins.rs` call the handlers directly. These run the
//! **shape the Twig compiler emits**, through `VMCore::execute`, because the
//! bug was observed at that level: the `lang_matrix` Vm and Jit columns ran
//!
//! ```scheme
//! (define (f x) (if (< x 0) 42 7)) (f -5)
//! ```
//!
//! and stopped with `UnknownOpcode("builtin \"<\"")`. `x` has no static type,
//! so `twig-ir-compiler` cannot pick the typed `cmp_lt` opcode; it emits
//!
//! ```text
//! f(x: any):
//!     zero = const 0            : i64
//!     r    = call_builtin "<", x, zero   : any     ← the dynamic compare
//!     jmp_if_false r, else
//!     a = const 42 ; ret a
//! else:
//!     b = const 7  ; ret b
//! ```
//!
//! and `twig-vm` (via `dynval-runtime`) runs it. Every test here failed before
//! the fix, at the `call_builtin`.

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

/// `f(x) = if (OP x 0) 42 else 7`, with `x` typed `any` as Twig types an
/// un-annotated parameter, and `main() = f(arg)`.
fn branch_on_param(op: &str, arg: i64) -> IIRModule {
    let f = IIRFunction::new(
        "f",
        vec![("x".to_string(), "any".to_string())],
        "any",
        vec![
            ins("const", Some("zero"), vec![Operand::Int(0)], "i64"),
            ins("call_builtin", Some("r"), vec![var(op), var("x"), var("zero")], "any"),
            ins("jmp_if_false", None, vec![var("r"), var("else")], "void"),
            ins("const", Some("a"), vec![Operand::Int(42)], "i64"),
            ins("ret", None, vec![var("a")], "any"),
            ins("label", None, vec![var("else")], "void"),
            ins("const", Some("b"), vec![Operand::Int(7)], "i64"),
            ins("ret", None, vec![var("b")], "any"),
        ],
    );
    let main = IIRFunction::new(
        "main",
        vec![],
        "any",
        vec![
            ins("const", Some("n"), vec![Operand::Int(arg)], "i64"),
            ins("call", Some("v"), vec![var("f"), var("n")], "any"),
            ins("ret", None, vec![var("v")], "any"),
        ],
    );
    let mut m = IIRModule::new("dyn_compare", "twig");
    m.add_or_replace(f);
    m.add_or_replace(main);
    m
}

fn run(mut m: IIRModule) -> Result<Option<Value>, VMError> {
    VMCore::new().execute(&mut m, "main", &[])
}

/// The issue's own program: `(f -5)` takes the `< 0` branch.
#[test]
fn issue_18463_program_returns_42() {
    assert_eq!(run(branch_on_param("<", -5)).unwrap(), Some(Value::Int(42)));
}

/// Both branches of every ordering builtin, with the boundary value `0` on
/// each side — so a strict/non-strict mix-up or a dropped sign fails.
#[test]
fn every_ordering_takes_the_right_branch() {
    //            op     arg  expected
    let cases = [
        ("<", -5, 42),
        ("<", 0, 7),
        ("<", 5, 7),
        ("<=", 0, 42),
        ("<=", 1, 7),
        (">", 5, 42),
        (">", 0, 7),
        (">", -5, 7),
        (">=", 0, 42),
        (">=", -1, 7),
    ];
    for (op, arg, want) in cases {
        assert_eq!(
            run(branch_on_param(op, arg)).unwrap(),
            Some(Value::Int(want)),
            "(if ({op} {arg} 0) 42 7)"
        );
    }
}

/// A non-number reaching a dynamic compare stops the program with a
/// `TypeError`, the way `twig-vm` does — it is not read as some integer.
#[test]
fn a_string_operand_is_a_type_error() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "any",
        vec![
            ins("const", Some("s"), vec![Operand::Str("a".into())], "str"),
            ins("const", Some("one"), vec![Operand::Int(1)], "i64"),
            ins("call_builtin", Some("r"), vec![var("<"), var("s"), var("one")], "any"),
            ins("ret", None, vec![var("r")], "any"),
        ],
    );
    let mut m = IIRModule::new("dyn_compare_err", "twig");
    m.add_or_replace(f);
    assert!(matches!(run(m), Err(VMError::TypeError { .. })));
}

/// `pair?` on what `lower_heap_builtins` makes of `(cons 1 2)` — an `alloc`
/// cell — is true; on nil (`const 0 : ref<LispyPair>`) and an integer it is
/// false. The result is a `Bool`, so it feeds `jmp_if_false` directly.
#[test]
fn pair_p_on_a_real_cell_nil_and_an_atom() {
    let probe = |subject: Vec<IIRInstr>| {
        let mut body = subject;
        body.push(ins("call_builtin", Some("p"), vec![var("pair?"), var("x")], "any"));
        body.push(ins("ret", None, vec![var("p")], "any"));
        let mut m = IIRModule::new("pair_p", "twig");
        m.add_or_replace(IIRFunction::new("main", vec![], "any", body));
        run(m).unwrap()
    };
    let cell = vec![
        ins("const", Some("one"), vec![Operand::Int(1)], "i64"),
        ins("const", Some("two"), vec![Operand::Int(2)], "i64"),
        ins("alloc", Some("x"), vec![], "ref<LispyPair>"),
        ins("field_store", None, vec![var("x"), Operand::Int(0), var("one")], "void"),
        ins("field_store", None, vec![var("x"), Operand::Int(1), var("two")], "void"),
    ];
    assert_eq!(probe(cell), Some(Value::Bool(true)), "(pair? (cons 1 2))");
    let nil = vec![ins("const", Some("x"), vec![Operand::Int(0)], "ref<LispyPair>")];
    assert_eq!(probe(nil), Some(Value::Bool(false)), "(pair? '())");
    let atom = vec![ins("const", Some("x"), vec![Operand::Int(42)], "i64")];
    assert_eq!(probe(atom), Some(Value::Bool(false)), "(pair? 42)");
}
