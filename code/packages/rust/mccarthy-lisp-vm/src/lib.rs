//! # `mccarthy-lisp-vm` — McCarthy 1960 Lisp's own interpreter.
//!
//! This is the reference VM for McCarthy Lisp (Lisp 1.0).  It executes
//! the [`IIRModule`] produced by `mccarthy-lisp-iir-compiler` against
//! the [`dynval_runtime`] value model and returns a [`LispyValue`].
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
//! [`dynval_runtime`]'s tagged-`i64` [`LispyValue`] (`int / nil / symbol /
//! #t / #f / heap-cons`), its interner, and its `cons / car / cdr /
//! pair? / not / equal?` builtins.  So McCarthy Lisp gets its *own* small
//! VM built directly on that foundation — exactly what this crate is.
//!
//! ## The instruction set it executes (through L2c-3a)
//!
//! `mccarthy-lisp-iir-compiler` emits a deliberately tiny IIR:
//!
//! | Op            | Meaning                                                        |
//! |---------------|----------------------------------------------------------------|
//! | `const`       | materialise a literal: `Int(n)`→int, `Int(0):ref<LispyPair>`→nil, `Var(name)`→interned symbol, `Bool(b)`→bool |
//! | `call_builtin`| `srcs[0]` is the builtin name (a `Var`), the rest are argument registers; dispatched to a `lispy-runtime` builtin |
//! | `call`        | `srcs[0]` is the callee function *name*; the rest are arguments. Runs the callee in a fresh frame (params bound to args), returns into `dest` |
//! | `apply`       | `srcs[0]` is a register holding a *closure value* `(*CLOSURE* fn-name . env)`; the rest are arguments. Destructures the closure, looks the function up by name, runs it in a fresh frame — dynamic dispatch (L2c-3a) |
//! | `mov`         | copy a register (`dest ← srcs[0]`)                             |
//! | `jmp`         | unconditional branch to the label in `srcs[0]`                |
//! | `jmp_if_false`| branch to the label in `srcs[1]` when `srcs[0]` is falsy (`#f`/`nil`); else fall through |
//! | `label`       | branch-target marker (`srcs[0]` is its name)                  |
//! | `ret`         | return the value in `srcs[0]`                                  |
//!
//! `mov`/`jmp`/`jmp_if_false`/`label` are what `COND` lowers to (L2b);
//! `call` is what `LAMBDA` application lowers to (L2c-1).
//!
//! **`LABEL` recursion (L2c-2) needed no new opcode.**  A named recursive
//! function `(LABEL F (LAMBDA … (F …) …))` compiles to a function whose
//! body simply `call`s itself by name — and `call` already resolves the
//! callee from the module and runs it in a fresh frame.  Because call
//! nesting is bounded by [`MAX_CALL_DEPTH`] and the shared instruction
//! budget, even a *non-terminating* recursion errors cleanly
//! ([`VmError::CallDepthExceeded`]) rather than overflowing the native
//! stack.
//!
//! **`apply` (L2c-3a) is the one new opcode for closures.**  A `LAMBDA`
//! used as a value compiles to a closure `(*CLOSURE* fn-name . env)`; a
//! call whose head is such a value uses `apply` to dispatch *dynamically*
//! (the callee isn't known until run time).  It shares `call`'s
//! depth/budget guards, so the Ω combinator
//! `((LAMBDA (X) (X X)) (LAMBDA (X) (X X)))` — which type-checks in this
//! untyped Lisp and loops forever — terminates with `CallDepthExceeded`
//! instead of a stack overflow.  Applying a non-closure value is a clean
//! [`VmError::NotAClosure`].
//!
//! Capture (L2c-3b) and recursive closures (L2c-3c) need **no further VM
//! change**: a captured `env` is just leading arguments `apply` prepends,
//! and a recursive closure's body recurses through an ordinary static
//! `call` to its own (compiler-assigned) name — so a `LABEL` used as a
//! value runs, and a non-terminating one still hits `CallDepthExceeded`.
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
use dynval_runtime::value::{INT_MAX, INT_MIN};
use dynval_runtime::{builtins, intern, name_of, LispyValue};

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
    /// A `call` was made to a function whose parameter count doesn't match
    /// the number of arguments supplied.
    ArityMismatch {
        /// The callee's name.
        function: String,
        /// How many parameters it declares.
        expected: usize,
        /// How many arguments the `call` supplied.
        got: usize,
    },
    /// User-function call recursion exceeded [`MAX_CALL_DEPTH`].  Guards
    /// against a stack overflow from a self-calling (or mutually
    /// recursive) `IIRModule` on untrusted input.
    CallDepthExceeded(usize),
    /// An `apply` was given a value that is not a closure
    /// (`(*CLOSURE* fn-name . env)`).  Carries a short description of what
    /// was found, e.g. applying a symbol or an integer.
    NotAClosure(String),
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
            VmError::ArityMismatch { function, expected, got } => write!(
                f,
                "function {function:?} expects {expected} argument(s), got {got}"
            ),
            VmError::CallDepthExceeded(d) => {
                write!(f, "call depth exceeded {d} — possible unbounded recursion")
            }
            VmError::NotAClosure(what) => {
                write!(f, "cannot apply a non-closure value ({what})")
            }
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

/// Maximum user-function call-nesting depth.  `call` recurses in native
/// Rust (one [`run_function`] frame per active call), so an untrusted
/// `IIRModule` with a self-calling function could otherwise overflow the
/// OS thread stack.  256 is deep enough for any realistic McCarthy
/// program yet far below the stack ceiling.
pub const MAX_CALL_DEPTH: usize = 256;

/// Maximum number of arguments a single `call` may carry.  Bounds the
/// up-front argument-vector allocation so a hand-crafted `call` with
/// millions of operands can't OOM before the instruction budget fires.
pub const MAX_CALL_ARGS: usize = 4096;

/// The reserved tag symbol at the head of every closure value
/// `(*CLOSURE* fn-name . env)` the `apply` opcode receives.  **Must match
/// the same constant in `mccarthy-lisp-iir-compiler`**, which builds these
/// values.  It is intentionally un-lexable McCarthy source (a symbol is
/// `[A-Z][A-Z0-9-]*`; this starts with `*`), so a value the VM accepts as
/// a closure can only have been emitted by the compiler — never forged by
/// a user program via `QUOTE`.
const CLOSURE_TAG: &str = "*CLOSURE*";

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
    // The entry point takes no arguments and starts at call depth 0.
    run_function(module, func, &[], &mut steps, budget, 0)
}

// ===========================================================================
// Interpreter core
// ===========================================================================

/// The register file: register name → live value.  A `HashMap` keeps the
/// VM independent of any register-numbering scheme the compiler uses.
type Frame = HashMap<String, LispyValue>;

fn run_function(
    module: &IIRModule,
    func: &IIRFunction,
    args: &[LispyValue],
    steps: &mut u64,
    budget: u64,
    depth: usize,
) -> Result<LispyValue, VmError> {
    if depth > MAX_CALL_DEPTH {
        return Err(VmError::CallDepthExceeded(MAX_CALL_DEPTH));
    }

    // Bind the arguments to the callee's parameter registers.  Arity must
    // match exactly — McCarthy Lisp has no variadics.
    if args.len() != func.params.len() {
        return Err(VmError::ArityMismatch {
            function: func.name.clone(),
            expected: func.params.len(),
            got: args.len(),
        });
    }
    let mut frame: Frame = HashMap::new();
    for ((pname, _ty), val) in func.params.iter().zip(args.iter()) {
        frame.insert(pname.clone(), *val);
    }
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
            // `call FN, args…` — invoke a user function.  srcs[0] =
            // Var(callee name); the rest are argument registers.  The
            // callee runs in a fresh frame (its params bound to the
            // argument values) at depth+1; its return value lands in
            // `dest`.  The instruction budget is shared across the whole
            // call tree (passed by `&mut`), and `depth` is bounded by
            // `MAX_CALL_DEPTH`, so neither the budget nor the native stack
            // can be exhausted by recursion.
            "call" => {
                let callee_name = match instr.srcs.first() {
                    Some(Operand::Var(n)) => n.as_str(),
                    _ => {
                        return Err(VmError::Malformed(
                            "`call` requires srcs[0] = Var(function name)".into(),
                        ))
                    }
                };
                if instr.srcs.len() > MAX_CALL_ARGS + 1 {
                    return Err(VmError::Malformed(format!(
                        "`call` has too many arguments ({})",
                        instr.srcs.len() - 1
                    )));
                }
                let callee = module
                    .get_function(callee_name)
                    .ok_or_else(|| VmError::UnknownFunction(callee_name.to_string()))?;
                let mut call_args: Vec<LispyValue> = Vec::with_capacity(instr.srcs.len() - 1);
                for src in &instr.srcs[1..] {
                    call_args.push(read_operand(src, &frame)?);
                }
                let result =
                    run_function(module, callee, &call_args, steps, budget, depth + 1)?;
                bind_dest(instr, result, &mut frame)?;
                pc += 1;
            }
            // `apply CLOSURE, args…` — invoke a *closure value*.  Unlike
            // `call` (whose `srcs[0]` is a static function name), here
            // `srcs[0]` is a register holding a closure value
            // `(*CLOSURE* fn-name . env)`.  We destructure it, flatten the
            // captured environment `env = (v1 … vk)` into the **leading**
            // call arguments (the lifted function's parameters are
            // `captured ∪ own`, captured first), append the supplied
            // arguments, then run the callee in a fresh frame.  Same
            // depth/budget guards as `call`, so a self-applying closure (the
            // Ω combinator) hits `CallDepthExceeded`, never a stack overflow.
            "apply" => {
                let cl_src = instr.srcs.first().ok_or_else(|| {
                    VmError::Malformed("`apply` requires srcs[0] (the closure register)".into())
                })?;
                let closure = read_operand(cl_src, &frame)?;
                let (callee_name, env) = destructure_closure(closure)?;
                if instr.srcs.len() > MAX_CALL_ARGS + 1 {
                    return Err(VmError::Malformed(format!(
                        "`apply` has too many arguments ({})",
                        instr.srcs.len() - 1
                    )));
                }
                let callee = module
                    .get_function(&callee_name)
                    .ok_or(VmError::UnknownFunction(callee_name))?;
                // Leading args: the captured environment, in order.  The
                // env is a compiler-built finite acyclic list, but we still
                // cap its length (defensively) so a hand-crafted module
                // can't make `apply` allocate without bound.
                let mut call_args = flatten_env(env)?;
                for src in &instr.srcs[1..] {
                    call_args.push(read_operand(src, &frame)?);
                }
                let result =
                    run_function(module, callee, &call_args, steps, budget, depth + 1)?;
                bind_dest(instr, result, &mut frame)?;
                pc += 1;
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
        Operand::Int(n) => dyn_int(*n),
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

/// Destructure a closure value `(*CLOSURE* fn-name . env)` into its callee
/// function name and captured environment.
///
/// The value must be a pair whose `car` is the reserved `*CLOSURE*` symbol;
/// then `cadr` is the function-name symbol and `cddr` is the environment
/// (nil in L2c-3a).  Anything else — a symbol, an integer, nil, or a pair
/// with the wrong head — is a [`VmError::NotAClosure`].  Because `*CLOSURE*`
/// is not a lexable McCarthy symbol, only the compiler can have built a
/// value that passes this check; a user program cannot forge one.  We walk
/// the structure with the `lispy-runtime` `car`/`cdr` builtins (so the
/// inspection goes through the same safe accessors as everything else).
fn destructure_closure(v: LispyValue) -> Result<(String, LispyValue), VmError> {
    let not = |what: &str| VmError::NotAClosure(what.to_string());
    let head = builtins::car(&[v]).map_err(|_| not("not a pair"))?;
    if head.as_symbol() != Some(intern(CLOSURE_TAG)) {
        return Err(not("a pair, but not a closure (wrong tag)"));
    }
    let rest = builtins::cdr(&[v]).map_err(|_| not("malformed closure"))?;
    let fn_sym = builtins::car(&[rest]).map_err(|_| not("closure missing its function name"))?;
    let env = builtins::cdr(&[rest]).map_err(|_| not("closure missing its environment"))?;
    let id = fn_sym
        .as_symbol()
        .ok_or_else(|| not("closure function slot is not a symbol"))?;
    let name = name_of(id).ok_or_else(|| not("closure names an un-interned function"))?;
    Ok((name, env))
}

/// Flatten a closure environment `(v1 v2 … vk)` — a proper list of the
/// captured values — into a `Vec`, in order.  These become the leading
/// arguments of an `apply` (the lifted function's captured parameters).
///
/// The list is built by the compiler and is always finite and acyclic
/// (McCarthy source has no mutation, so no cycles), but we still bound its
/// length by [`MAX_CALL_ARGS`] so a hand-crafted module can't make `apply`
/// allocate without limit.  A non-list `env` (anything but a proper list)
/// is a malformed closure → [`VmError::NotAClosure`].
fn flatten_env(env: LispyValue) -> Result<Vec<LispyValue>, VmError> {
    let not = |what: &str| VmError::NotAClosure(what.to_string());
    let mut out = Vec::new();
    let mut cur = env;
    while !cur.is_nil() {
        if out.len() >= MAX_CALL_ARGS {
            return Err(VmError::Malformed(
                "closure environment is too large".into(),
            ));
        }
        let v = builtins::car(&[cur]).map_err(|_| not("closure env is not a proper list"))?;
        out.push(v);
        cur = builtins::cdr(&[cur]).map_err(|_| not("closure env is not a proper list"))?;
    }
    Ok(out)
}

/// Build a tagged integer, rejecting values outside `lispy-runtime`'s
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
/// `Int` / `Bool` is an immediate.  (`const` handles its `Var` specially —
/// see [`eval_const`].)
fn read_operand(op: &Operand, frame: &Frame) -> Result<LispyValue, VmError> {
    match op {
        Operand::Var(name) => frame
            .get(name)
            .copied()
            .ok_or_else(|| VmError::UndefinedRegister(name.clone())),
        Operand::Int(n) => dyn_int(*n),
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
    use dynval_runtime::name_of;

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

    // ---- user-function calls (L2c) ----

    /// Build a module from a `main` body plus extra (callee) functions.
    fn module_with(funcs: Vec<IIRFunction>, main_body: Vec<IIRInstr>) -> IIRModule {
        let mut m = IIRModule::new("test", "mccarthy-lisp");
        m.functions = funcs;
        m.functions.push(IIRFunction::new("main", Vec::new(), "any", main_body));
        m.entry_point = Some("main".into());
        m
    }

    fn func(name: &str, params: &[&str], body: Vec<IIRInstr>) -> IIRFunction {
        let params = params.iter().map(|p| (p.to_string(), "any".to_string())).collect();
        IIRFunction::new(name, params, "any", body)
    }

    fn call(dest: &str, callee: &str, args: &[&str]) -> IIRInstr {
        let mut srcs = vec![Operand::Var(callee.into())];
        srcs.extend(args.iter().map(|a| Operand::Var((*a).into())));
        IIRInstr::new("call", Some(dest.into()), srcs, "any")
    }

    fn builtin_instr(dest: &str, name: &str, args: &[&str]) -> IIRInstr {
        let mut srcs = vec![Operand::Var(name.into())];
        srcs.extend(args.iter().map(|a| Operand::Var((*a).into())));
        IIRInstr::new("call_builtin", Some(dest.into()), srcs, "any")
    }

    /// `apply DEST, [closure_reg, arg_regs…]`.
    fn apply(dest: &str, closure: &str, args: &[&str]) -> IIRInstr {
        let mut srcs = vec![Operand::Var(closure.into())];
        srcs.extend(args.iter().map(|a| Operand::Var((*a).into())));
        IIRInstr::new("apply", Some(dest.into()), srcs, "any")
    }

    /// Instructions that build a closure value `(*CLOSURE* fn_name)` into
    /// register `dest` (empty env — the L2c-3a shape the compiler emits).
    fn build_closure(dest: &str, fn_name: &str) -> Vec<IIRInstr> {
        build_closure_env(dest, fn_name, &[])
    }

    /// Instructions that build a closure value `(*CLOSURE* fn_name v…)` into
    /// `dest`, where `env_regs` are the captured-value registers (the
    /// L2c-3b shape: `cons(tag, cons(fn, env))`, `env = (v1 … vk)`).
    fn build_closure_env(dest: &str, fn_name: &str, env_regs: &[&str]) -> Vec<IIRInstr> {
        let mut out = vec![
            konst("_ctag", Operand::Var("*CLOSURE*".into()), "symbol"),
            konst("_cfn", Operand::Var(fn_name.into()), "symbol"),
            konst("_cenv", Operand::Int(0), "ref<LispyPair>"),
        ];
        // Build env = (v1 … vk) tail-first into `_cenv`.
        for r in env_regs.iter().rev() {
            out.push(builtin_instr("_cenv", "cons", &[r, "_cenv"]));
        }
        out.push(builtin_instr("_cinner", "cons", &["_cfn", "_cenv"]));
        out.push(builtin_instr(dest, "cons", &["_ctag", "_cinner"]));
        out
    }

    #[test]
    fn call_binds_param_and_returns() {
        // f(X) = X ; main = f(42)
        let f = func("f", &["X"], vec![ret("X")]);
        let m = module_with(vec![f], vec![konst("a", Operand::Int(42), "i64"), call("r", "f", &["a"]), ret("r")]);
        assert_eq!(run(&m).unwrap().as_int(), Some(42));
    }

    #[test]
    fn call_runs_builtin_in_callee() {
        // head(X) = (CAR X) ; main = head( (CONS 'A 'B) ) → A
        let head = func(
            "head",
            &["X"],
            vec![
                IIRInstr::new(
                    "call_builtin",
                    Some("h".into()),
                    vec![Operand::Var("car".into()), Operand::Var("X".into())],
                    "any",
                ),
                ret("h"),
            ],
        );
        let main = vec![
            konst("a", Operand::Var("A".into()), "symbol"),
            konst("b", Operand::Var("B".into()), "symbol"),
            IIRInstr::new(
                "call_builtin",
                Some("p".into()),
                vec![Operand::Var("cons".into()), Operand::Var("a".into()), Operand::Var("b".into())],
                "ref<LispyPair>",
            ),
            call("r", "head", &["p"]),
            ret("r"),
        ];
        let v = run(&module_with(vec![head], main)).unwrap();
        assert_eq!(name_of(v.as_symbol().unwrap()).as_deref(), Some("A"));
    }

    #[test]
    fn call_arity_mismatch_errors() {
        let f = func("f", &["X", "Y"], vec![ret("X")]);
        // Only one argument supplied for a two-param function.
        let m = module_with(vec![f], vec![konst("a", Operand::Int(1), "i64"), call("r", "f", &["a"]), ret("r")]);
        assert!(matches!(run(&m), Err(VmError::ArityMismatch { .. })));
    }

    #[test]
    fn call_to_unknown_function_errors() {
        let m = module_with(vec![], vec![konst("a", Operand::Int(1), "i64"), call("r", "ghost", &["a"]), ret("r")]);
        assert!(matches!(run(&m), Err(VmError::UnknownFunction(_))));
    }

    #[test]
    fn unbounded_recursion_hits_call_depth_guard() {
        // loop() = loop()  — a self-calling function must hit the call-depth
        // guard (a clean error), never a native stack overflow.  This is
        // the DoS regression for L2c-2 recursion: a non-terminating `LABEL`
        // compiles to exactly this shape (a function whose body `call`s
        // itself), so the guard here is what protects the native stack.
        let loop_fn = func("loop", &[], vec![call("r", "loop", &[]), ret("r")]);
        let m = module_with(vec![loop_fn], vec![call("r", "loop", &[]), ret("r")]);
        assert!(matches!(run(&m), Err(VmError::CallDepthExceeded(_))));
    }

    #[test]
    fn terminating_recursion_computes_correctly() {
        // A hand-built recursive `LABEL` body: `last`, which walks the
        // cdr-spine to the final element.  This is what
        // `((LABEL LAST (LAMBDA (L)
        //     (COND ((ATOM (CDR L)) (CAR L)) ('T (LAST (CDR L)))))) '(A B))`
        // lowers to — proving the VM's existing `call` opcode supports
        // recursion (no new opcode was needed for L2c-2):
        //
        //   last(L):
        //     t      = cdr(L)
        //     ip     = pair?(t)
        //     isatom = not(ip)            ; (ATOM (CDR L))
        //     jmp_if_false isatom, recur  ; not an atom → recurse
        //     ret car(L)                  ; cdr is nil → L is the last cell
        //   recur:
        //     r = last(cdr(L))
        //     ret r
        let builtin = |dest: &str, name: &str, args: &[&str]| {
            let mut srcs = vec![Operand::Var(name.into())];
            srcs.extend(args.iter().map(|a| Operand::Var((*a).into())));
            IIRInstr::new("call_builtin", Some(dest.into()), srcs, "any")
        };
        let last = func(
            "last",
            &["L"],
            vec![
                builtin("t", "cdr", &["L"]),
                builtin("ip", "pair?", &["t"]),
                builtin("isatom", "not", &["ip"]),
                jmp_if_false("isatom", "recur"),
                builtin("h", "car", &["L"]),
                ret("h"),
                label("recur"),
                builtin("t2", "cdr", &["L"]),
                call("r", "last", &["t2"]),
                ret("r"),
            ],
        );
        // main: build (A B) = cons(A, cons(B, nil)), then last((A B)) → B.
        let main = vec![
            konst("a", Operand::Var("A".into()), "symbol"),
            konst("b", Operand::Var("B".into()), "symbol"),
            konst("nil", Operand::Int(0), "ref<LispyPair>"),
            builtin("inner", "cons", &["b", "nil"]),
            builtin("lst", "cons", &["a", "inner"]),
            call("r", "last", &["lst"]),
            ret("r"),
        ];
        let v = run(&module_with(vec![last], main)).unwrap();
        assert_eq!(name_of(v.as_symbol().unwrap()).as_deref(), Some("B"));
    }

    // ---- closures + dynamic apply (L2c-3a) ----

    #[test]
    fn apply_runs_a_closure() {
        // id(X) = X ; main builds closure (*CLOSURE* id) and applies it to 42.
        let id = func("id", &["X"], vec![ret("X")]);
        let mut main = build_closure("cl", "id");
        main.push(konst("a", Operand::Int(42), "i64"));
        main.push(apply("r", "cl", &["a"]));
        main.push(ret("r"));
        let m = module_with(vec![id], main);
        assert_eq!(run(&m).unwrap().as_int(), Some(42));
    }

    #[test]
    fn apply_closure_runs_a_builtin_in_the_callee() {
        // head(X) = (CAR X); apply the closure to (CONS 'A 'B) → A.
        let head = func("head", &["X"], vec![builtin_instr("h", "car", &["X"]), ret("h")]);
        let mut main = build_closure("cl", "head");
        main.extend(vec![
            konst("a", Operand::Var("A".into()), "symbol"),
            konst("b", Operand::Var("B".into()), "symbol"),
            builtin_instr("p", "cons", &["a", "b"]),
            apply("r", "cl", &["p"]),
            ret("r"),
        ]);
        let m = module_with(vec![head], main);
        let v = run(&m).unwrap();
        assert_eq!(name_of(v.as_symbol().unwrap()).as_deref(), Some("A"));
    }

    #[test]
    fn apply_of_a_plain_symbol_is_not_a_closure() {
        // Applying a bare symbol (not a *CLOSURE* pair) errors cleanly.
        let m = module(vec![
            konst("s", Operand::Var("FOO".into()), "symbol"),
            konst("a", Operand::Int(1), "i64"),
            apply("r", "s", &["a"]),
            ret("r"),
        ]);
        assert!(matches!(run(&m), Err(VmError::NotAClosure(_))));
    }

    #[test]
    fn apply_of_a_pair_without_the_tag_is_not_a_closure() {
        // A pair whose car is not `*CLOSURE*` — e.g. (A . B) — is rejected.
        let m = module(vec![
            konst("a", Operand::Var("A".into()), "symbol"),
            konst("b", Operand::Var("B".into()), "symbol"),
            builtin_instr("p", "cons", &["a", "b"]),
            apply("r", "p", &["a"]),
            ret("r"),
        ]);
        assert!(matches!(run(&m), Err(VmError::NotAClosure(_))));
    }

    #[test]
    fn apply_to_unknown_function_errors() {
        // A well-formed closure naming a function the module lacks.
        let mut main = build_closure("cl", "ghost");
        main.push(konst("a", Operand::Int(1), "i64"));
        main.push(apply("r", "cl", &["a"]));
        main.push(ret("r"));
        let m = module_with(vec![], main);
        assert!(matches!(run(&m), Err(VmError::UnknownFunction(_))));
    }

    #[test]
    fn self_applying_closure_hits_call_depth_guard() {
        // The Ω combinator's shape: omega(F) = (apply F F).  main applies
        // the omega closure to itself → unbounded dynamic self-application,
        // which must hit the call-depth guard, never a native stack
        // overflow.  (This is the DoS regression for the new `apply` op.)
        let omega = func("omega", &["F"], vec![apply("r", "F", &["F"]), ret("r")]);
        let mut main = build_closure("cl", "omega");
        main.push(apply("r", "cl", &["cl"]));
        main.push(ret("r"));
        let m = module_with(vec![omega], main);
        assert!(matches!(run(&m), Err(VmError::CallDepthExceeded(_))));
    }

    #[test]
    fn apply_binds_captured_env_then_args() {
        // L2c-3b: a closure with a non-empty env.  pair(X, Y) = (CONS X Y);
        // the closure captures X='A (env = (A)); applying it to 'B binds the
        // env value to the leading param X and 'B to Y → (A . B).
        let pair = func("pair", &["X", "Y"], vec![builtin_instr("p", "cons", &["X", "Y"]), ret("p")]);
        let mut main = vec![konst("cap", Operand::Var("A".into()), "symbol")];
        main.extend(build_closure_env("cl", "pair", &["cap"])); // env = (A)
        main.extend(vec![
            konst("b", Operand::Var("B".into()), "symbol"),
            apply("r", "cl", &["b"]), // supply only Y; X comes from env
            ret("r"),
        ]);
        let v = run(&module_with(vec![pair], main)).unwrap();
        let head = builtins::car(&[v]).unwrap();
        let tail = builtins::cdr(&[v]).unwrap();
        assert_eq!(name_of(head.as_symbol().unwrap()).as_deref(), Some("A"));
        assert_eq!(name_of(tail.as_symbol().unwrap()).as_deref(), Some("B"));
    }

    #[test]
    fn apply_with_malformed_env_is_not_a_closure() {
        // A closure whose env slot is a non-list atom (here a symbol) is
        // malformed — `flatten_env` rejects it cleanly, never loops/panics.
        // Shape: cons(*CLOSURE*, cons(fn-name, ENV)) with ENV = a symbol.
        let id = func("id", &["X"], vec![ret("X")]);
        let main = vec![
            konst("tag", Operand::Var("*CLOSURE*".into()), "symbol"),
            konst("fn", Operand::Var("id".into()), "symbol"),
            konst("badenv", Operand::Var("OOPS".into()), "symbol"), // not a list
            builtin_instr("inner", "cons", &["fn", "badenv"]),
            builtin_instr("cl", "cons", &["tag", "inner"]),
            konst("a", Operand::Int(1), "i64"),
            apply("r", "cl", &["a"]),
            ret("r"),
        ];
        assert!(matches!(run(&module_with(vec![id], main)), Err(VmError::NotAClosure(_))));
    }

    #[test]
    fn recursive_closure_value_applied() {
        // L2c-3c shape: a recursive function invoked through a closure value.
        // `last` recurses via a static `call last` (just like a compiled
        // LABEL body), but the *outer* invocation is an `apply` on a closure
        // value `(*CLOSURE* last)`.  Proves a recursive closure works when
        // applied — no new VM machinery beyond `apply` + `call`.
        let last = func(
            "last",
            &["L"],
            vec![
                builtin_instr("t", "cdr", &["L"]),
                builtin_instr("ip", "pair?", &["t"]),
                builtin_instr("isatom", "not", &["ip"]),
                jmp_if_false("isatom", "recur"),
                builtin_instr("h", "car", &["L"]),
                ret("h"),
                label("recur"),
                builtin_instr("t2", "cdr", &["L"]),
                call("r", "last", &["t2"]),
                ret("r"),
            ],
        );
        // main: build (A B), wrap `last` in a closure, apply it → B.
        let mut main = vec![
            konst("a", Operand::Var("A".into()), "symbol"),
            konst("b", Operand::Var("B".into()), "symbol"),
            konst("nil", Operand::Int(0), "ref<LispyPair>"),
            builtin_instr("inner", "cons", &["b", "nil"]),
            builtin_instr("lst", "cons", &["a", "inner"]),
        ];
        main.extend(build_closure("cl", "last"));
        main.push(apply("r", "cl", &["lst"]));
        main.push(ret("r"));
        let v = run(&module_with(vec![last], main)).unwrap();
        assert_eq!(name_of(v.as_symbol().unwrap()).as_deref(), Some("B"));
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
