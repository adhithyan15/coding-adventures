//! # `mccarthy-lisp-vm` — McCarthy 1960 Lisp's own interpreter.
//!
//! This is the reference VM for McCarthy Lisp (Lisp 1.0).  It executes
//! the [`IIRModule`] produced by `mccarthy-lisp-iir-compiler` against
//! the [`lispy_runtime`] value model and returns a [`LispyValue`].
//!
//! ## Why a dedicated VM (and not `vm-core` or `twig-vm`)
//!
//! - **`vm-core`** is a *scalar* interpreter: its `Value` is only
//!   `Int / Float / Bool / Str / Null`.  It has no way to represent a
//!   symbol or a cons cell, so it cannot run `(CAR '(A B C))` → `A`.
//! - **`twig-vm`** *can* run such programs — but it is the VM for the
//!   **Twig** language: it carries Twig's typed-CIR mnemonics, closures,
//!   module forms, and so on.  Coupling McCarthy Lisp to Twig's VM would
//!   be an architectural mistake.
//!
//! The thing both languages genuinely share is the **value model**:
//! [`lispy_runtime`]'s tagged-`i64` [`LispyValue`] (`int / nil / symbol /
//! #t / #f / heap-cons`), its interner, and its `cons / car / cdr /
//! pair? / not / equal?` builtins.  So McCarthy Lisp gets its *own* small
//! VM built directly on that foundation — exactly what this crate is.
//!
//! ## The instruction set it executes (through L2b)
//!
//! `mccarthy-lisp-iir-compiler` emits a deliberately tiny IIR:
//!
//! | Op            | Meaning                                                        |
//! |---------------|----------------------------------------------------------------|
//! | `const`       | materialise a literal: `Int(n)`→int, `Int(0):ref<LispyPair>`→nil, `Var(name)`→interned symbol, `Bool(b)`→bool |
//! | `call_builtin`| `srcs[0]` is the builtin name (a `Var`), the rest are argument registers; dispatched to a `lispy-runtime` builtin |
//! | `mov`         | copy a register (`dest ← srcs[0]`)                             |
//! | `jmp`         | unconditional branch to the label in `srcs[0]`                |
//! | `jmp_if_false`| branch to the label in `srcs[1]` when `srcs[0]` is falsy (`#f`/`nil`); else fall through |
//! | `label`       | branch-target marker (`srcs[0]` is its name)                  |
//! | `ret`         | return the value in `srcs[0]`                                  |
//!
//! `mov`/`jmp`/`jmp_if_false`/`label` are what `COND` lowers to (L2b).
//! User-function `call` (for `LAMBDA` / `LABEL`) arrives with L2c; this
//! VM grows to match the compiler phase by phase.
//!
//! ## Quick start
//!
//! ```
//! use mccarthy_lisp_vm::run;
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
//! let mut module = IIRModule::new("demo", "mccarthy-lisp");
//! module.functions.push(main);
//! module.entry_point = Some("main".into());
//!
//! let result = run(&module).expect("run");
//! assert_eq!(result.as_int(), Some(42));
//! ```

#![warn(missing_docs)]

use std::collections::HashMap;

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use lang_runtime_core::RuntimeError;
use lispy_runtime::value::{INT_MAX, INT_MIN};
use lispy_runtime::{builtins, intern, LispyValue};

// ===========================================================================
// Errors
// ===========================================================================

/// A failure while interpreting a McCarthy Lisp [`IIRModule`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmError {
    /// The module has no `entry_point` set.
    NoEntryPoint,
    /// `entry_point` (or a `call` target) names a function the module
    /// does not contain.
    UnknownFunction(String),
    /// Control reached the end of a function without a `ret`.
    FellOffEnd(String),
    /// An instruction this VM doesn't (yet) execute.  Carries the opcode.
    UnsupportedOp(String),
    /// An instruction was missing a `dest`, an operand, etc.
    Malformed(String),
    /// A register was read before it was written.
    UndefinedRegister(String),
    /// A `call_builtin` named a builtin `lispy-runtime` doesn't provide.
    UnknownBuiltin(String),
    /// A `lispy-runtime` builtin raised a runtime trap (wrong arity,
    /// type error, …).  Carries its message.
    Runtime(String),
    /// The per-run instruction budget was exhausted (a runaway program).
    InstructionBudgetExceeded(u64),
    /// An integer literal outside `lispy-runtime`'s tagged-int range
    /// (`[-2^60, 2^60 - 1]`).  Caught here so it surfaces as a clean
    /// error rather than being silently truncated by `LispyValue::int`.
    IntegerOutOfRange(i64),
    /// A `jmp` / `jmp_if_false` targeted a label the function doesn't
    /// define.
    UnknownLabel(String),
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
                write!(f, "instruction budget ({n}) exceeded — possible infinite loop")
            }
            VmError::IntegerOutOfRange(n) => write!(
                f,
                "integer literal {n} is outside lispy-runtime's tagged-int range [-2^60, 2^60-1]"
            ),
            VmError::UnknownLabel(l) => write!(f, "branch to undefined label {l:?}"),
        }
    }
}

impl std::error::Error for VmError {}

// ===========================================================================
// Public entry points
// ===========================================================================

/// Default instruction budget.  Far above any real McCarthy program's
/// step count, but a hard backstop against a runaway loop (once `jmp`
/// arrives in L2b) blocking the interpreter forever.
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
/// number of instructions executed before [`VmError::InstructionBudgetExceeded`]).
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

/// The register file: register name → live value.  A `HashMap` keeps the
/// VM independent of any register-numbering scheme the compiler uses.
type Frame = HashMap<String, LispyValue>;

fn run_function(func: &IIRFunction, steps: &mut u64, budget: u64) -> Result<LispyValue, VmError> {
    let mut frame: Frame = HashMap::new();
    let mut pc: usize = 0;

    // Resolve every `label NAME` to its instruction index once, so a jump
    // is an O(1) lookup.  (`COND` lowering emits forward jumps to labels
    // it also defines, so every target is present.)
    let labels = label_table(func);

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
                let src = instr.srcs.first().ok_or_else(|| {
                    VmError::Malformed("`ret` requires a source operand".into())
                })?;
                return read_operand(src, &frame);
            }
            // `mov dest, src` — copy a register (used by `COND` to funnel
            // each clause's value into one result register).
            "mov" => {
                let src = instr.srcs.first().ok_or_else(|| {
                    VmError::Malformed("`mov` requires a source operand".into())
                })?;
                let value = read_operand(src, &frame)?;
                bind_dest(instr, value, &mut frame)?;
                pc += 1;
            }
            // `jmp LABEL` — unconditional branch.  srcs[0] = Var(label).
            "jmp" => {
                let label = label_operand(instr, 0)?;
                pc = resolve_label(&labels, label)?;
            }
            // `jmp_if_false COND, LABEL` — branch to LABEL when COND is
            // falsy (`#f` or `nil`); otherwise fall through.  srcs[0] =
            // Var(cond register), srcs[1] = Var(label).
            "jmp_if_false" => {
                let cond_src = instr.srcs.first().ok_or_else(|| {
                    VmError::Malformed("`jmp_if_false` requires srcs[0] (the condition)".into())
                })?;
                let cond = read_operand(cond_src, &frame)?;
                let label = label_operand(instr, 1)?;
                if cond.is_truthy() {
                    pc += 1;
                } else {
                    pc = resolve_label(&labels, label)?;
                }
            }
            // `label` is a no-op marker; its only role is as a jump target.
            "label" => pc += 1,
            other => return Err(VmError::UnsupportedOp(other.to_string())),
        }
    }

    Err(VmError::FellOffEnd(func.name.clone()))
}

/// Store an instruction's result in its `dest` register.
fn bind_dest(instr: &IIRInstr, value: LispyValue, frame: &mut Frame) -> Result<(), VmError> {
    let dest = instr.dest.as_ref().ok_or_else(|| {
        VmError::Malformed(format!("`{}` requires a dest register", instr.op))
    })?;
    frame.insert(dest.clone(), value);
    Ok(())
}

/// Materialise a `const` instruction's literal into a [`LispyValue`].
///
/// The operand encodings match what `mccarthy-lisp-iir-compiler` emits
/// (and what `lispy-runtime` / the IIR backends expect):
///
/// - `Int(0)` with `type_hint == "ref<LispyPair>"` → the **nil** sentinel.
/// - any other `Int(n)` → a tagged integer.
/// - `Var(name)` → an **interned symbol** (NOT a register read; inside a
///   `const` the `Var` carries the symbol's textual name).
/// - `Bool(b)` → `#t` / `#f`.
fn eval_const(instr: &IIRInstr) -> Result<LispyValue, VmError> {
    let src = instr
        .srcs
        .first()
        .ok_or_else(|| VmError::Malformed("`const` requires a source operand".into()))?;
    match src {
        Operand::Int(0) if instr.type_hint == "ref<LispyPair>" => Ok(LispyValue::NIL),
        Operand::Int(n) => lispy_int(*n),
        Operand::Bool(b) => Ok(LispyValue::bool(*b)),
        Operand::Var(name) => Ok(LispyValue::symbol(intern(name))),
        Operand::Float(_) => Err(VmError::Malformed(
            "McCarthy Lisp 1.0 has no floats; unexpected Float operand in `const`".into(),
        )),
        Operand::Str(_) => Err(VmError::Malformed(
            "unexpected Str operand in `const` (symbols use Var, not Str)".into(),
        )),
    }
}

/// Execute a `call_builtin`: `srcs[0]` is the builtin's name (a `Var`),
/// the remaining sources are argument registers (or immediates).
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

/// A `lispy-runtime` builtin: takes a slice of arguments, returns a
/// value or a runtime trap.  This is the shape every `builtins::*`
/// function has.
type BuiltinFn = fn(&[LispyValue]) -> Result<LispyValue, RuntimeError>;

/// Map a McCarthy builtin name to its `lispy-runtime` implementation.
///
/// L2a needs exactly the data primitives + predicates the compiler
/// emits.  More builtins can be wired here as later phases need them;
/// `lispy-runtime` already provides the implementations.
fn resolve_builtin(name: &str) -> Option<BuiltinFn> {
    match name {
        "cons" => Some(builtins::cons),
        "car" => Some(builtins::car),
        "cdr" => Some(builtins::cdr),
        "pair?" => Some(builtins::pair_p),
        "not" => Some(builtins::not),
        "equal?" => Some(builtins::equal_p),
        _ => None,
    }
}

/// Map each `label NAME` instruction to its index in the function body.
fn label_table(func: &IIRFunction) -> HashMap<String, usize> {
    let mut labels = HashMap::new();
    for (i, instr) in func.instructions.iter().enumerate() {
        if instr.op == "label" {
            if let Some(Operand::Var(name)) = instr.srcs.first() {
                labels.insert(name.clone(), i);
            }
        }
    }
    labels
}

/// Read the label-name operand at `idx` from a branch instruction.
fn label_operand(instr: &IIRInstr, idx: usize) -> Result<&str, VmError> {
    match instr.srcs.get(idx) {
        Some(Operand::Var(name)) => Ok(name.as_str()),
        _ => Err(VmError::Malformed(format!(
            "`{}` requires a label name (a Var) at srcs[{idx}]",
            instr.op
        ))),
    }
}

/// Resolve a label name to its instruction index.
fn resolve_label(labels: &HashMap<String, usize>, name: &str) -> Result<usize, VmError> {
    labels
        .get(name)
        .copied()
        .ok_or_else(|| VmError::UnknownLabel(name.to_string()))
}

/// Build a tagged integer, rejecting values outside `lispy-runtime`'s
/// 61-bit tagged-int range rather than letting `LispyValue::int` silently
/// truncate them in release builds.
fn lispy_int(n: i64) -> Result<LispyValue, VmError> {
    if (INT_MIN..=INT_MAX).contains(&n) {
        Ok(LispyValue::int(n))
    } else {
        Err(VmError::IntegerOutOfRange(n))
    }
}

/// Read an operand in *value* position: a `Var` is a register read; an
/// `Int` / `Bool` is an immediate.  (`const` handles its `Var` specially —
/// see [`eval_const`].)
fn read_operand(op: &Operand, frame: &Frame) -> Result<LispyValue, VmError> {
    match op {
        Operand::Var(name) => frame
            .get(name)
            .copied()
            .ok_or_else(|| VmError::UndefinedRegister(name.clone())),
        Operand::Int(n) => lispy_int(*n),
        Operand::Bool(b) => Ok(LispyValue::bool(*b)),
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
    use lispy_runtime::name_of;

    /// Build a single-`main` module from a list of instructions.
    fn module(instrs: Vec<IIRInstr>) -> IIRModule {
        let main = IIRFunction::new("main", Vec::new(), "any", instrs);
        let mut m = IIRModule::new("test", "mccarthy-lisp");
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

    #[test]
    fn integer_const() {
        let m = module(vec![konst("v0", Operand::Int(42), "i64"), ret("v0")]);
        assert_eq!(run(&m).unwrap().as_int(), Some(42));
    }

    #[test]
    fn nil_const() {
        let m = module(vec![konst("v0", Operand::Int(0), "ref<LispyPair>"), ret("v0")]);
        assert!(run(&m).unwrap().is_nil());
    }

    #[test]
    fn symbol_const_interns() {
        let m = module(vec![konst("v0", Operand::Var("FOO".into()), "symbol"), ret("v0")]);
        let v = run(&m).unwrap();
        assert_eq!(v.as_symbol(), Some(intern("FOO")));
        assert_eq!(name_of(v.as_symbol().unwrap()).as_deref(), Some("FOO"));
    }

    #[test]
    fn cons_car_cdr() {
        // (CONS 'A 'B) then CAR / CDR of the result.
        let build = vec![
            konst("a", Operand::Var("A".into()), "symbol"),
            konst("b", Operand::Var("B".into()), "symbol"),
            IIRInstr::new(
                "call_builtin",
                Some("p".into()),
                vec![Operand::Var("cons".into()), Operand::Var("a".into()), Operand::Var("b".into())],
                "ref<LispyPair>",
            ),
        ];
        let mut car = build.clone();
        car.push(IIRInstr::new(
            "call_builtin",
            Some("h".into()),
            vec![Operand::Var("car".into()), Operand::Var("p".into())],
            "any",
        ));
        car.push(ret("h"));
        let head = run(&module(car)).unwrap();
        assert_eq!(name_of(head.as_symbol().unwrap()).as_deref(), Some("A"));

        let mut cdr = build;
        cdr.push(IIRInstr::new(
            "call_builtin",
            Some("t".into()),
            vec![Operand::Var("cdr".into()), Operand::Var("p".into())],
            "any",
        ));
        cdr.push(ret("t"));
        let tail = run(&module(cdr)).unwrap();
        assert_eq!(name_of(tail.as_symbol().unwrap()).as_deref(), Some("B"));
    }

    #[test]
    fn atom_via_pair_then_not() {
        // (not (pair? 'X)) → #t
        let m = module(vec![
            konst("x", Operand::Var("X".into()), "symbol"),
            IIRInstr::new(
                "call_builtin",
                Some("ip".into()),
                vec![Operand::Var("pair?".into()), Operand::Var("x".into())],
                "bool",
            ),
            IIRInstr::new(
                "call_builtin",
                Some("r".into()),
                vec![Operand::Var("not".into()), Operand::Var("ip".into())],
                "bool",
            ),
            ret("r"),
        ]);
        assert!(run(&m).unwrap().is_true());
    }

    #[test]
    fn equal_on_symbols() {
        let eq = |x: &str, y: &str| {
            module(vec![
                konst("a", Operand::Var(x.into()), "symbol"),
                konst("b", Operand::Var(y.into()), "symbol"),
                IIRInstr::new(
                    "call_builtin",
                    Some("r".into()),
                    vec![Operand::Var("equal?".into()), Operand::Var("a".into()), Operand::Var("b".into())],
                    "bool",
                ),
                ret("r"),
            ])
        };
        assert!(run(&eq("A", "A")).unwrap().is_true());
        assert!(run(&eq("A", "B")).unwrap().is_false());
    }

    // ---- control flow (L2b) ----

    fn label(name: &str) -> IIRInstr {
        IIRInstr::new("label", None, vec![Operand::Var(name.into())], "void")
    }
    fn jmp(target: &str) -> IIRInstr {
        IIRInstr::new("jmp", None, vec![Operand::Var(target.into())], "void")
    }
    fn jmp_if_false(cond: &str, target: &str) -> IIRInstr {
        IIRInstr::new(
            "jmp_if_false",
            None,
            vec![Operand::Var(cond.into()), Operand::Var(target.into())],
            "void",
        )
    }
    fn mov(dest: &str, src: &str) -> IIRInstr {
        IIRInstr::new("mov", Some(dest.into()), vec![Operand::Var(src.into())], "any")
    }

    #[test]
    fn mov_copies_a_register() {
        let m = module(vec![konst("v0", Operand::Int(7), "i64"), mov("r", "v0"), ret("r")]);
        assert_eq!(run(&m).unwrap().as_int(), Some(7));
    }

    #[test]
    fn unconditional_jmp_skips_instructions() {
        // jmp over a `mov result, wrong` to the right answer.
        let m = module(vec![
            konst("right", Operand::Int(1), "i64"),
            konst("wrong", Operand::Int(2), "i64"),
            jmp("skip"),
            mov("result", "wrong"), // jumped over
            label("skip"),
            mov("result", "right"),
            ret("result"),
        ]);
        assert_eq!(run(&m).unwrap().as_int(), Some(1));
    }

    #[test]
    fn jmp_if_false_branches_on_falsy_and_falls_through_on_truthy() {
        // A two-clause COND skeleton: if `cond` is truthy → 1, else → 2.
        let cond_program = |cond_is_true: bool| {
            module(vec![
                IIRInstr::new("const", Some("c".into()), vec![Operand::Bool(cond_is_true)], "bool"),
                jmp_if_false("c", "elsewhere"),
                konst("t", Operand::Int(1), "i64"),
                mov("result", "t"),
                jmp("end"),
                label("elsewhere"),
                konst("e", Operand::Int(2), "i64"),
                mov("result", "e"),
                label("end"),
                ret("result"),
            ])
        };
        assert_eq!(run(&cond_program(true)).unwrap().as_int(), Some(1));
        assert_eq!(run(&cond_program(false)).unwrap().as_int(), Some(2));
    }

    #[test]
    fn nil_is_falsy_for_jmp_if_false() {
        // nil branches (it is falsy), so this lands on the else value.
        let m = module(vec![
            konst("c", Operand::Int(0), "ref<LispyPair>"), // nil
            jmp_if_false("c", "els"),
            konst("t", Operand::Int(1), "i64"),
            mov("result", "t"),
            jmp("end"),
            label("els"),
            konst("e", Operand::Int(2), "i64"),
            mov("result", "e"),
            label("end"),
            ret("result"),
        ]);
        assert_eq!(run(&m).unwrap().as_int(), Some(2));
    }

    #[test]
    fn jump_to_undefined_label_errors() {
        let m = module(vec![jmp("nowhere")]);
        assert!(matches!(run(&m), Err(VmError::UnknownLabel(_))));
    }

    // ---- error paths ----

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
        let m = module(vec![IIRInstr::new("frobnicate", None, vec![], "any")]);
        assert!(matches!(run(&m), Err(VmError::UnsupportedOp(_))));
    }

    #[test]
    fn unknown_builtin() {
        let m = module(vec![
            konst("x", Operand::Var("X".into()), "symbol"),
            IIRInstr::new(
                "call_builtin",
                Some("r".into()),
                vec![Operand::Var("frobnicate".into()), Operand::Var("x".into())],
                "any",
            ),
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
    fn integer_out_of_tagged_range_is_rejected() {
        // i64::MAX is well outside lispy's 61-bit tagged-int range and
        // must surface as a clean error, not a silently-truncated value.
        let m = module(vec![konst("v0", Operand::Int(i64::MAX), "i64"), ret("v0")]);
        assert!(matches!(run(&m), Err(VmError::IntegerOutOfRange(_))));
        // The boundary value is accepted.
        let ok = module(vec![konst("v0", Operand::Int((1 << 60) - 1), "i64"), ret("v0")]);
        assert_eq!(run(&ok).unwrap().as_int(), Some((1 << 60) - 1));
    }

    #[test]
    fn instruction_budget() {
        let m = module(vec![konst("v0", Operand::Int(1), "i64"), ret("v0")]);
        // Budget of 1 trips before `ret` (the const is instruction #1).
        assert!(matches!(run_with_budget(&m, 1), Err(VmError::InstructionBudgetExceeded(_))));
    }
}
