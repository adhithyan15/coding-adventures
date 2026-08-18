//! # `derive-vm` — Derive's own v0 interpreter.
//!
//! Executes the [`IIRModule`] produced by `derive-iir-compiler` against
//! the [`dynval_runtime`] value model and returns a [`LispyValue`]. See
//! [`derive-iir-vm.md`](../../../specs/derive-iir-vm.md) for the full
//! design (the v0 scope and why arithmetic evaluates on the VM instead
//! of being frontend-folded).
//!
//! ## Why a dedicated VM (and not `twig-vm`/`mccarthy-lisp-vm`/`macsyma-vm`)
//!
//! Every language in this rollout gets its own small VM on top of
//! [`dynval_runtime`]'s tagged-`i64` value model — deliberately *not*
//! shared with any sibling (see `mccarthy-lisp-vm`'s own module doc, and
//! `macsyma-iir-vm.md` §6's explicit VM-sharing decision for this
//! rollout). Derive follows the identical precedent: its own VM, sharing
//! only the `dynval_runtime` foundation, not any other language's
//! instruction set or opcodes.
//!
//! ## The v0 instruction set
//!
//! `derive-iir-compiler` emits a deliberately tiny IIR — no branches, no
//! calls, no closures, since v0's accepted grammar (literal
//! arithmetic/assignment/unevaluated symbolic expressions) needs none of
//! those:
//!
//! | Op             | Meaning                                                                 |
//! |----------------|--------------------------------------------------------------------------|
//! | `const`        | `Int(n)` → tagged int, `Int(0):ref<LispyPair>` → the nil sentinel, `Var(name)` → interned symbol |
//! | `call_builtin` | `srcs[0]` is the builtin name (a `Var`), the rest are argument registers; dispatched to a `dynval-runtime` builtin |
//! | `ret`          | return the value in `srcs[0]`                                            |
//!
//! `call_builtin` backs both v0's real arithmetic (`+`/`-`/`*`/`/` on
//! concrete operands) and its unevaluated-symbolic-expression
//! representation (`cons`, building an inert `(head arg0 arg1 …)` chain).
//!
//! ## Quick start
//!
//! ```
//! use derive_vm::run;
//! use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
//!
//! // A hand-built module equivalent to the program `42`.
//! let main = IIRFunction::new(
//!     "main",
//!     Vec::new(),
//!     "any",
//!     vec![
//!         IIRInstr::new("const", Some("v0".into()), vec![Operand::Int(42)], "i64"),
//!         IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "any"),
//!     ],
//! );
//! let mut module = IIRModule::new("demo", "derive");
//! module.functions.push(main);
//! module.entry_point = Some("main".into());
//!
//! let result = run(&module).expect("run");
//! assert_eq!(result.as_int(), Some(42));
//! ```

#![warn(missing_docs)]

use std::collections::HashMap;

use dynval_runtime::value::{INT_MAX, INT_MIN};
use dynval_runtime::{builtins, intern, LispyValue};
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use lang_runtime_core::RuntimeError;

// ===========================================================================
// Errors
// ===========================================================================

/// A failure while interpreting a Derive [`IIRModule`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmError {
    /// The module has no `entry_point` set.
    NoEntryPoint,
    /// `entry_point` names a function the module does not contain.
    UnknownFunction(String),
    /// Control reached the end of a function without a `ret`.
    FellOffEnd(String),
    /// An instruction this VM doesn't execute (v0 has no branches, calls,
    /// or closures). Carries the opcode.
    UnsupportedOp(String),
    /// An instruction was missing a `dest`, an operand, etc.
    Malformed(String),
    /// A register was read before it was written.
    UndefinedRegister(String),
    /// A `call_builtin` named a builtin `dynval-runtime` doesn't provide.
    UnknownBuiltin(String),
    /// A `dynval-runtime` builtin raised a runtime trap (wrong arity,
    /// type error, division by zero, overflow, …). Carries its message.
    Runtime(String),
    /// The per-run instruction budget was exhausted.
    InstructionBudgetExceeded(u64),
    /// An integer literal outside `dynval-runtime`'s tagged-int range
    /// (`[-2^60, 2^60 - 1]`). A known v0 gap tied to a later wave (bignum
    /// support) — see `derive-iir-vm.md`.
    IntegerOutOfRange(i64),
}

impl std::fmt::Display for VmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VmError::NoEntryPoint => write!(f, "module has no entry_point"),
            VmError::UnknownFunction(n) => write!(f, "unknown function {n:?}"),
            VmError::FellOffEnd(n) => write!(f, "function {n:?} ran off the end without `ret`"),
            VmError::UnsupportedOp(op) => write!(f, "unsupported opcode {op:?}"),
            VmError::Malformed(m) => write!(f, "malformed instruction: {m}"),
            VmError::UndefinedRegister(r) => write!(f, "register {r:?} read before written"),
            VmError::UnknownBuiltin(b) => write!(f, "unknown builtin {b:?}"),
            VmError::Runtime(m) => write!(f, "runtime error: {m}"),
            VmError::InstructionBudgetExceeded(n) => {
                write!(
                    f,
                    "instruction budget ({n}) exceeded — possible infinite loop"
                )
            }
            VmError::IntegerOutOfRange(n) => write!(
                f,
                "integer literal {n} is outside dynval-runtime's tagged-int range [-2^60, 2^60-1]"
            ),
        }
    }
}

impl std::error::Error for VmError {}

// ===========================================================================
// Public entry points
// ===========================================================================

/// Default instruction budget. Far above any real v0 program's step count
/// (v0 has no loops), but a hard backstop nonetheless.
pub const DEFAULT_INSTRUCTION_BUDGET: u64 = 10_000_000;

/// Execute `module`'s entry-point function and return its result value.
///
/// # Errors
///
/// See [`VmError`] for the full list — missing entry point, unsupported
/// opcode, builtin trap, etc.
pub fn run(module: &IIRModule) -> Result<LispyValue, VmError> {
    run_with_budget(module, DEFAULT_INSTRUCTION_BUDGET)
}

/// Like [`run`], but with an explicit instruction budget (the maximum
/// number of instructions executed before
/// [`VmError::InstructionBudgetExceeded`]).
///
/// # Errors
///
/// See [`VmError`].
pub fn run_with_budget(module: &IIRModule, budget: u64) -> Result<LispyValue, VmError> {
    let entry = module.entry_point.as_deref().ok_or(VmError::NoEntryPoint)?;
    let func = module
        .get_function(entry)
        .ok_or_else(|| VmError::UnknownFunction(entry.to_string()))?;
    let mut steps: u64 = 0;
    run_function(func, &mut steps, budget)
}

// ===========================================================================
// Interpreter core
// ===========================================================================

/// The register file: register name → live value. A `HashMap` keeps the
/// VM independent of any register-numbering scheme the compiler uses.
type Frame = HashMap<String, LispyValue>;

fn run_function(func: &IIRFunction, steps: &mut u64, budget: u64) -> Result<LispyValue, VmError> {
    let mut frame: Frame = HashMap::new();
    let mut pc: usize = 0;

    while pc < func.instructions.len() {
        *steps += 1;
        if *steps > budget {
            return Err(VmError::InstructionBudgetExceeded(budget));
        }

        let instr = &func.instructions[pc];
        match instr.op.as_str() {
            "const" => {
                let value = eval_const(instr)?;
                bind_dest(instr, value, &mut frame)?;
                pc += 1;
            }
            "call_builtin" => {
                let value = eval_call_builtin(instr, &frame)?;
                bind_dest(instr, value, &mut frame)?;
                pc += 1;
            }
            "ret" => {
                let src = instr
                    .srcs
                    .first()
                    .ok_or_else(|| VmError::Malformed("`ret` requires a source operand".into()))?;
                return read_operand(src, &frame);
            }
            other => return Err(VmError::UnsupportedOp(other.to_string())),
        }
    }

    Err(VmError::FellOffEnd(func.name.clone()))
}

/// Store an instruction's result in its `dest` register.
fn bind_dest(instr: &IIRInstr, value: LispyValue, frame: &mut Frame) -> Result<(), VmError> {
    let dest = instr
        .dest
        .as_ref()
        .ok_or_else(|| VmError::Malformed(format!("`{}` requires a dest register", instr.op)))?;
    frame.insert(dest.clone(), value);
    Ok(())
}

/// Materialise a `const` instruction's literal into a [`LispyValue`].
///
/// The operand encodings match what `derive-iir-compiler` emits (the
/// same conventions `macsyma-iir-compiler`/`mccarthy-lisp-iir-compiler`
/// established for this value model):
///
/// - `Int(0)` with `type_hint == "ref<LispyPair>"` → the **nil** sentinel.
/// - any other `Int(n)` → a tagged integer.
/// - `Var(name)` → an **interned symbol** (not a register read — inside a
///   `const`, `Var` carries the symbol's textual name).
fn eval_const(instr: &IIRInstr) -> Result<LispyValue, VmError> {
    let src = instr
        .srcs
        .first()
        .ok_or_else(|| VmError::Malformed("`const` requires a source operand".into()))?;
    match src {
        Operand::Int(0) if instr.type_hint == "ref<LispyPair>" => Ok(LispyValue::NIL),
        Operand::Int(n) => dyn_int(*n),
        Operand::Var(name) => Ok(LispyValue::symbol(intern(name))),
        Operand::Bool(_) => Err(VmError::Malformed(
            "Derive v0 has no boolean literals; unexpected Bool operand in `const`".into(),
        )),
        Operand::Float(_) => Err(VmError::Malformed(
            "Derive v0 has no floats (derive-iir-vm.md); unexpected Float operand in `const`"
                .into(),
        )),
        Operand::Str(_) => Err(VmError::Malformed(
            "Derive v0 has no string literals; unexpected Str operand in `const`".into(),
        )),
    }
}

/// Execute a `call_builtin`: `srcs[0]` is the builtin's name (a `Var`),
/// the remaining sources are argument registers.
fn eval_call_builtin(instr: &IIRInstr, frame: &Frame) -> Result<LispyValue, VmError> {
    let name = match instr.srcs.first() {
        Some(Operand::Var(n)) => n.as_str(),
        _ => {
            return Err(VmError::Malformed(
                "`call_builtin` requires srcs[0] to be the builtin name (a Var)".into(),
            ))
        }
    };

    let mut args: Vec<LispyValue> = Vec::with_capacity(instr.srcs.len().saturating_sub(1));
    for src in &instr.srcs[1..] {
        args.push(read_operand(src, frame)?);
    }

    let builtin = resolve_builtin(name).ok_or_else(|| VmError::UnknownBuiltin(name.to_string()))?;
    builtin(&args).map_err(|e| VmError::Runtime(e.to_string()))
}

/// A `dynval-runtime` builtin: takes a slice of arguments, returns a value
/// or a runtime trap. This is the shape every `builtins::*` function has.
type BuiltinFn = fn(&[LispyValue]) -> Result<LispyValue, RuntimeError>;

/// Map a Derive v0 builtin name to its `dynval-runtime` implementation.
///
/// `+`/`-`/`*`/`/` back v0's real arithmetic; `cons` backs its
/// unevaluated-symbolic-expression representation. `car`/`cdr` are wired
/// even though v0's own lowering never emits them, matching
/// `macsyma-vm`'s precedent of a slightly-generous table — free to
/// include, and useful for hand-built test modules that inspect a result.
fn resolve_builtin(name: &str) -> Option<BuiltinFn> {
    match name {
        "+" => Some(builtins::add),
        "-" => Some(builtins::sub),
        "*" => Some(builtins::mul),
        "/" => Some(builtins::div),
        "cons" => Some(builtins::cons),
        "car" => Some(builtins::car),
        "cdr" => Some(builtins::cdr),
        _ => None,
    }
}

/// Build a tagged integer, rejecting values outside `dynval-runtime`'s
/// 61-bit tagged-int range rather than letting `LispyValue::int` silently
/// truncate them in release builds.
fn dyn_int(n: i64) -> Result<LispyValue, VmError> {
    if (INT_MIN..=INT_MAX).contains(&n) {
        Ok(LispyValue::int(n))
    } else {
        Err(VmError::IntegerOutOfRange(n))
    }
}

/// Read an operand in *value* position: a `Var` is a register read; an
/// `Int` is an immediate. (`const` handles its `Var` specially — see
/// [`eval_const`].)
fn read_operand(op: &Operand, frame: &Frame) -> Result<LispyValue, VmError> {
    match op {
        Operand::Var(name) => frame
            .get(name)
            .copied()
            .ok_or_else(|| VmError::UndefinedRegister(name.clone())),
        Operand::Int(n) => dyn_int(*n),
        Operand::Bool(_) => Err(VmError::Malformed("unexpected Bool operand".into())),
        Operand::Float(_) => Err(VmError::Malformed("unexpected Float operand".into())),
        Operand::Str(_) => Err(VmError::Malformed("unexpected Str operand".into())),
    }
}

// ===========================================================================
// Tests (hand-built IIR; the compiler's own tests cover the source path)
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use dynval_runtime::name_of;

    fn module(instrs: Vec<IIRInstr>) -> IIRModule {
        let main = IIRFunction::new("main", Vec::new(), "any", instrs);
        let mut m = IIRModule::new("test", "derive");
        m.functions.push(main);
        m.entry_point = Some("main".into());
        m
    }

    fn konst(dest: &str, op: Operand, ty: &str) -> IIRInstr {
        IIRInstr::new("const", Some(dest.into()), vec![op], ty)
    }

    fn ret(reg: &str) -> IIRInstr {
        IIRInstr::new("ret", None, vec![Operand::Var(reg.into())], "any")
    }

    fn builtin_instr(dest: &str, name: &str, args: &[&str]) -> IIRInstr {
        let mut srcs = vec![Operand::Var(name.into())];
        srcs.extend(args.iter().map(|a| Operand::Var((*a).into())));
        IIRInstr::new("call_builtin", Some(dest.into()), srcs, "any")
    }

    #[test]
    fn integer_const() {
        let m = module(vec![konst("v0", Operand::Int(42), "i64"), ret("v0")]);
        assert_eq!(run(&m).unwrap().as_int(), Some(42));
    }

    #[test]
    fn nil_const() {
        let m = module(vec![
            konst("v0", Operand::Int(0), "ref<LispyPair>"),
            ret("v0"),
        ]);
        assert!(run(&m).unwrap().is_nil());
    }

    #[test]
    fn symbol_const_interns() {
        let m = module(vec![
            konst("v0", Operand::Var("X".into()), "symbol"),
            ret("v0"),
        ]);
        let v = run(&m).unwrap();
        assert_eq!(v.as_symbol(), Some(intern("X")));
        assert_eq!(name_of(v.as_symbol().unwrap()).as_deref(), Some("X"));
    }

    #[test]
    fn add_two_concrete_ints() {
        let m = module(vec![
            konst("a", Operand::Int(2), "i64"),
            konst("b", Operand::Int(3), "i64"),
            builtin_instr("r", "+", &["a", "b"]),
            ret("r"),
        ]);
        assert_eq!(run(&m).unwrap().as_int(), Some(5));
    }

    #[test]
    fn sub_unary_negates() {
        let m = module(vec![
            konst("a", Operand::Int(5), "i64"),
            builtin_instr("r", "-", &["a"]),
            ret("r"),
        ]);
        assert_eq!(run(&m).unwrap().as_int(), Some(-5));
    }

    #[test]
    fn cons_builds_an_inert_pair() {
        // The shape `derive-iir-compiler` uses for an unevaluated Apply
        // node: cons("Add", cons(x, cons(y, nil))).
        let m = module(vec![
            konst("head", Operand::Var("Add".into()), "symbol"),
            konst("x", Operand::Var("X".into()), "symbol"),
            konst("y", Operand::Var("Y".into()), "symbol"),
            konst("nil", Operand::Int(0), "ref<LispyPair>"),
            builtin_instr("t1", "cons", &["y", "nil"]),
            builtin_instr("t2", "cons", &["x", "t1"]),
            builtin_instr("r", "cons", &["head", "t2"]),
            ret("r"),
        ]);
        let v = run(&m).unwrap();
        let h = builtins::car(&[v]).unwrap();
        assert_eq!(name_of(h.as_symbol().unwrap()).as_deref(), Some("Add"));
    }

    #[test]
    fn missing_entry_point() {
        let mut m = module(vec![konst("v0", Operand::Int(1), "i64"), ret("v0")]);
        m.entry_point = None;
        assert_eq!(run(&m), Err(VmError::NoEntryPoint));
    }

    #[test]
    fn unknown_entry_function() {
        let mut m = module(vec![ret("v0")]);
        m.entry_point = Some("nope".into());
        assert!(matches!(run(&m), Err(VmError::UnknownFunction(_))));
    }

    #[test]
    fn fell_off_end_without_ret() {
        let m = module(vec![konst("v0", Operand::Int(1), "i64")]);
        assert!(matches!(run(&m), Err(VmError::FellOffEnd(_))));
    }

    #[test]
    fn unsupported_op() {
        let m = module(vec![IIRInstr::new("jmp", None, vec![], "void")]);
        assert!(matches!(run(&m), Err(VmError::UnsupportedOp(_))));
    }

    #[test]
    fn unknown_builtin() {
        let m = module(vec![
            konst("x", Operand::Int(1), "i64"),
            builtin_instr("r", "frobnicate", &["x"]),
            ret("r"),
        ]);
        assert!(matches!(run(&m), Err(VmError::UnknownBuiltin(_))));
    }

    #[test]
    fn undefined_register() {
        let m = module(vec![ret("never_written")]);
        assert!(matches!(run(&m), Err(VmError::UndefinedRegister(_))));
    }

    #[test]
    fn division_by_zero_is_a_clean_runtime_error() {
        let m = module(vec![
            konst("a", Operand::Int(1), "i64"),
            konst("b", Operand::Int(0), "i64"),
            builtin_instr("r", "/", &["a", "b"]),
            ret("r"),
        ]);
        assert!(matches!(run(&m), Err(VmError::Runtime(_))));
    }

    #[test]
    fn integer_out_of_tagged_range_is_rejected() {
        let m = module(vec![konst("v0", Operand::Int(i64::MAX), "i64"), ret("v0")]);
        assert!(matches!(run(&m), Err(VmError::IntegerOutOfRange(_))));
        let ok = module(vec![
            konst("v0", Operand::Int((1 << 60) - 1), "i64"),
            ret("v0"),
        ]);
        assert_eq!(run(&ok).unwrap().as_int(), Some((1 << 60) - 1));
    }

    #[test]
    fn instruction_budget() {
        let m = module(vec![konst("v0", Operand::Int(1), "i64"), ret("v0")]);
        assert!(matches!(
            run_with_budget(&m, 1),
            Err(VmError::InstructionBudgetExceeded(_))
        ));
    }
}
