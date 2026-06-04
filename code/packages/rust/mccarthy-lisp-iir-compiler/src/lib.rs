//! # `mccarthy-lisp-iir-compiler` — McCarthy 1960 Lisp → IIR.
//!
//! This is **L2a** of the McCarthy Lisp implementation (see
//! [`MCCARTHY-LISP-PLAN.md`](../../../specs/MCCARTHY-LISP-PLAN.md)).  It
//! lowers the [`LispExpr`] AST produced by `mccarthy-lisp-parser` into
//! an [`IIRModule`] — the architecture-independent intermediate
//! representation that every backend in the chain (the McCarthy VM,
//! the JIT, wasm/jvm/clr/beam, the historical-arch encoders) consumes.
//!
//! ## What "compile" means here
//!
//! A McCarthy program is a sequence of top-level S-expressions.  We
//! lower them into a single IIR function called `main` whose value is
//! the value of the **last** form (an empty program returns `nil`).
//! The module's `entry_point` is `"main"`.
//!
//! ## Which Lisp we can lower *today* (through L2c-3a)
//!
//! | Form                | Lowering                                             |
//! |---------------------|------------------------------------------------------|
//! | `42`                | `const v, Int(42)`                                   |
//! | `()`                | `const v, Int(0) : ref<LispyPair>`  (the nil sentinel)|
//! | `(QUOTE X)` / `'X`  | materialise `X` as runtime data (see below)          |
//! | `(CONS A B)`        | `call_builtin "cons" [A, B]`                         |
//! | `(CAR X)`           | `call_builtin "car" [X]`                             |
//! | `(CDR X)`           | `call_builtin "cdr" [X]`                             |
//! | `(ATOM X)`          | `call_builtin "pair?" [X]` then `call_builtin "not"` |
//! | `(EQ A B)`          | `call_builtin "equal?" [A, B]`                       |
//! | `(COND (p e) …)`    | chained `jmp_if_false` + `label`s; `mov` funnels each clause's value into one register; no match → nil |
//! | `((LAMBDA (p…) body) a…)` | a fresh `IIRFunction` for the lambda + a `call` to it with the lowered args |
//! | `((LABEL F (LAMBDA (p…) body)) a…)` | like a lambda, but `F` is bound — inside `body` a call `(F …)` lowers to a `call` to the same function, so `F` can **recurse** |
//! | `(LAMBDA (p…) body)` *in value position* | lift the lambda + materialise a **closure value** `(*CLOSURE* fn-name)` (L2c-3a; empty env, no capture yet) |
//! | `(F a…)` where `F` is a parameter, or `((g…) a…)` | **dynamic apply**: evaluate the head to a closure value, then the `apply` opcode runs it |
//!
//! `QUOTE` materialises a literal S-expression as runtime values:
//! a symbol `A` becomes `const v, Var("A")` (the IIR convention the
//! runtime interns into a symbol), an integer becomes `const v,
//! Int(n)`, the empty list becomes the nil sentinel, and a pair
//! `(a . b)` becomes `call_builtin "cons"` of the lowered halves — so
//! `'(A B C)` expands to `(CONS 'A (CONS 'B (CONS 'C ())))`.
//!
//! ## Why these exact opcodes
//!
//! They are the conventions of the shared `lispy-runtime` value model:
//! `const Var(name)` interns a symbol, `const 0 : ref<LispyPair>` is the
//! nil sentinel, and `call_builtin "cons"/"car"/"cdr"/…` dispatch to
//! `lispy-runtime` builtins.  Emitting them means the McCarthy frontend
//! runs on its own `mccarthy-lisp-vm` (a small interpreter over
//! `lispy-runtime`) *and* feeds every IIR backend, with no new runtime
//! code.  (Twig happens to use the same `lispy-runtime` conventions —
//! McCarthy Lisp is its untyped cousin — but the two share only that
//! foundation, not a VM.)
//!
//! `ATOM` and `EQ` are *derived*: McCarthy's `(ATOM x)` is "x is not a
//! cons", i.e. `(not (pair? x))`; `(EQ a b)` is atom identity, which
//! `lispy-runtime`'s `equal?` builtin implements (the numeric `=`
//! builtin rejects symbols, so `equal?` — identity on atoms — is the
//! right choice).
//!
//! ## `LABEL` and recursion (L2c-2)
//!
//! `(LABEL F (LAMBDA (p…) body))` is a *named* lambda: the name `F` is in
//! scope **inside `body`**, so the body can call itself.  We lower it like
//! a lambda — one fresh `IIRFunction` — but first bind `F` to that
//! function's (internal) name in a *function scope*.  A call `(F a…)`
//! whose head is a function-scope name lowers to a `call` to that
//! function.  A self-call therefore compiles to a `call` back into the
//! same function, and the **VM already handles that**: its `call` opcode
//! looks the callee up by name and runs it in a fresh frame, bounded by
//! `MAX_CALL_DEPTH` + the shared instruction budget.  So recursion needed
//! **no new VM opcode** — only the compiler learned to bind `F`.
//!
//! ```text
//! ;; first atom of a structure — McCarthy's canonical recursive example
//! ((LABEL FF (LAMBDA (X)
//!     (COND ((ATOM X) X)
//!           ('T (FF (CAR X))))))
//!  '((A B) C))                       ;; ⇒ A
//! ```
//!
//! ## Closures as values (L2c-3a)
//!
//! A `LAMBDA` in **value** position — passed as an argument, returned from
//! a body, or standing alone — becomes a **closure value**: the tagged
//! cons `(*CLOSURE* fn-name)` (a 2-element list; the captured environment
//! is empty in L2c-3a).  The tag `*CLOSURE*` is **un-forgeable from
//! source** — a McCarthy symbol is `[A-Z][A-Z0-9-]*`, so the lexer can
//! never produce `*CLOSURE*` and no `QUOTE` can fabricate one.
//!
//! Applying such a value — a call `(F a…)` whose head `F` is a parameter,
//! or `((g…) a…)` whose head is a nested application — lowers to the new
//! VM `apply` opcode, which destructures the closure, looks the function
//! up by name, and runs it.  Higher-order functions now work:
//!
//! ```text
//! ((LAMBDA (F) (F (QUOTE A))) (LAMBDA (X) X))   ;; ⇒ A
//! ```
//!
//! ## Not yet (later phases)
//!
//! - **Free-variable capture (L2c-3b)** — a lambda body still sees only
//!   its own parameters; a reference to an enclosing binding is an
//!   unbound-variable error until capture threads it through the closure's
//!   environment.  **`LABEL` as a value** (a recursive closure) also lands
//!   in L2c-3b.
//!
//! A bare (unquoted) symbol in value position is an *unbound variable*
//! unless it is a parameter of the enclosing lambda; a labelled function
//! name used in value position is reported as needing L2c-3b.  Either way
//! it is a [`CompileError`] rather than silently mis-lowered.
//!
//! ## Quick start
//!
//! ```
//! use mccarthy_lisp_iir_compiler::compile_source;
//!
//! let module = compile_source("(CAR '(A B C))", "demo").expect("compile");
//! assert_eq!(module.entry_point.as_deref(), Some("main"));
//! assert!(module.validate().is_empty());
//! ```

#![warn(missing_docs)]

use std::collections::{HashMap, HashSet};

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use mccarthy_lisp_parser::{parse, LispExpr};

// ===========================================================================
// Errors
// ===========================================================================

/// A failure to compile a McCarthy Lisp program to IIR.
///
/// Wraps a parser failure (malformed source) or a lowering failure (a
/// well-formed S-expression that L2a does not yet support, or a bare
/// unbound symbol).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError {
    /// Human-readable explanation.
    pub message: String,
}

impl CompileError {
    fn new(message: impl Into<String>) -> Self {
        CompileError { message: message.into() }
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "McCarthy compile error: {}", self.message)
    }
}

impl std::error::Error for CompileError {}

impl From<mccarthy_lisp_parser::ParseError> for CompileError {
    fn from(e: mccarthy_lisp_parser::ParseError) -> Self {
        CompileError::new(format!("parse: {e}"))
    }
}

// ===========================================================================
// Public API
// ===========================================================================

/// Compile McCarthy Lisp source text into an [`IIRModule`].
///
/// Tokenises + parses via `mccarthy-lisp-parser`, then lowers the form
/// sequence into a `main` function.
///
/// # Errors
///
/// Returns a [`CompileError`] for a lex/parse failure or an
/// unsupported / unbound form (see the crate docs for L2a scope).
pub fn compile_source(source: &str, module_name: &str) -> Result<IIRModule, CompileError> {
    let forms = parse(source)?;
    compile_forms(&forms, module_name)
}

/// Lower an already-parsed form sequence into an [`IIRModule`].
///
/// # Errors
///
/// Returns a [`CompileError`] for an unsupported / unbound form.
pub fn compile_forms(forms: &[LispExpr], module_name: &str) -> Result<IIRModule, CompileError> {
    let mut c = Compiler::new();
    c.lower_program(forms)?;

    let mut main = IIRFunction::new(
        "main",
        Vec::new(),       // no parameters
        "any",            // a Lisp value of any shape
        std::mem::take(&mut c.instrs),
    );
    main.register_count = c.tmp;

    let mut module = IIRModule::new(module_name, "mccarthy-lisp");
    // The `LAMBDA` functions first, then the `main` entry point.
    module.functions = c.functions;
    module.functions.push(main);
    module.entry_point = Some("main".to_string());

    let problems = module.validate();
    if !problems.is_empty() {
        return Err(CompileError::new(format!(
            "internal: emitted IIR failed validation: {}",
            problems.join("; ")
        )));
    }
    Ok(module)
}

// ===========================================================================
// Compiler
// ===========================================================================

/// IIR type-hint string for the nil / cons reference type.  The
/// McCarthy VM and the wasm/jvm/clr/beam backends special-case
/// `const 0 : ref<LispyPair>` into the nil sentinel and
/// `call_builtin "cons"` into a fresh pair.
const REF_PAIR: &str = "ref<LispyPair>";

/// The reserved tag symbol at the head of every closure value
/// `(*CLOSURE* fn-name . env)`.  **Must stay in sync with the same
/// constant in `mccarthy-lisp-vm`**, which destructures closures the
/// `apply` opcode receives.  It is intentionally un-lexable McCarthy
/// source (a symbol is `[A-Z][A-Z0-9-]*`; this starts with `*`), so no
/// user program can forge a closure via `QUOTE`.
const CLOSURE_TAG: &str = "*CLOSURE*";

/// Accumulates the instruction stream of the function currently being
/// lowered (the top-level `main` body, or — temporarily, while lowering
/// a `LAMBDA` — that lambda's body).
struct Compiler {
    /// Instructions of the function under construction.
    instrs: Vec<IIRInstr>,
    /// Completed `LAMBDA` functions, to be added to the module alongside
    /// `main`.
    functions: Vec<IIRFunction>,
    /// Monotonic counter for fresh SSA temp names (`v0`, `v1`, …).
    tmp: usize,
    /// Monotonic counter for fresh, unique branch-label names.
    label: usize,
    /// Monotonic counter for gensym `LAMBDA` function names.
    fn_ctr: usize,
    /// The parameter names visible in the function currently being
    /// lowered.  A bare symbol resolves to a register read **only** if it
    /// is one of these — McCarthy Lisp lambdas do not capture free
    /// variables (no closures yet), so an out-of-scope symbol is an
    /// unbound-variable error.
    params: HashSet<String>,
    /// `LABEL`-bound function names in lexical scope: source name →
    /// (internal IIR function name, parameter count).  When a call
    /// `(F a…)` has a head `F` in this map, it lowers to a `call` to the
    /// internal function — this is exactly what lets a `LABEL` body
    /// recurse (`F` calls itself).  Cloned/restored around each `LABEL`
    /// body so the binding is visible only inside that body (plus any
    /// nested forms), matching lexical scope.
    functions_in_scope: HashMap<String, (String, usize)>,
}

impl Compiler {
    fn new() -> Self {
        Compiler {
            instrs: Vec::new(),
            functions: Vec::new(),
            tmp: 0,
            label: 0,
            fn_ctr: 0,
            params: HashSet::new(),
            functions_in_scope: HashMap::new(),
        }
    }

    /// Allocate a fresh, never-reused temp register name.
    fn fresh(&mut self) -> String {
        let name = format!("v{}", self.tmp);
        self.tmp += 1;
        name
    }

    /// Allocate a fresh, never-reused label name (for `COND` branches).
    fn fresh_label(&mut self, hint: &str) -> String {
        let name = format!("L_{hint}_{}", self.label);
        self.label += 1;
        name
    }

    fn emit(&mut self, instr: IIRInstr) {
        self.instrs.push(instr);
    }

    /// Lower the whole program: each top-level form in order, then a
    /// `ret` of the last form's value (or of `nil` for an empty program).
    fn lower_program(&mut self, forms: &[LispExpr]) -> Result<(), CompileError> {
        let mut last: Option<String> = None;
        for form in forms {
            last = Some(self.lower_expr(form)?);
        }
        let result = match last {
            Some(v) => v,
            None => self.emit_nil(),
        };
        self.emit(IIRInstr::new("ret", None, vec![Operand::Var(result)], "any"));
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Expression lowering (value position)
    // -----------------------------------------------------------------------

    /// Lower an expression in *value* position, returning the name of the
    /// register holding its runtime value.
    fn lower_expr(&mut self, expr: &LispExpr) -> Result<String, CompileError> {
        match expr {
            LispExpr::Int(n) => Ok(self.emit_int(*n)),
            LispExpr::Nil => Ok(self.emit_nil()),
            // A bare symbol is a variable reference.  It resolves to a
            // register read *only* if it is a parameter of the function
            // currently being lowered; the VM binds each parameter to a
            // register named after it, so the register name *is* the
            // parameter name.
            LispExpr::Symbol(name) if self.params.contains(name) => Ok(name.clone()),
            // A labelled function name used in *value* position (not
            // called) would be a first-class function value — that needs
            // closures (L2c-3).  Calls `(F …)` are handled in
            // `lower_application`, which consults the same scope.
            LispExpr::Symbol(name) if self.functions_in_scope.contains_key(name) => {
                Err(CompileError::new(format!(
                    "`{name}` is a function (a LABEL-bound name); using it as a value — \
                     passing or returning it — needs closures, which are a later phase. \
                     You can only *call* it: `({name} …)`."
                )))
            }
            LispExpr::Symbol(name) => Err(CompileError::new(format!(
                "unbound variable `{name}`: it is not a parameter in scope. \
                 (Lambdas don't capture free variables yet — closures are a later phase.) \
                 Did you mean to quote it as `'{name}`?"
            ))),
            LispExpr::Cons(..) => self.lower_application(expr),
        }
    }

    /// Lower a `(head arg …)` application.  In L2a `head` must be one of
    /// the built-in primitives; user-defined calls arrive in L2c.
    fn lower_application(&mut self, expr: &LispExpr) -> Result<String, CompileError> {
        let items = proper_list(expr).ok_or_else(|| {
            CompileError::new(
                "improper list in call position (a dotted form like (A . B) is data, \
                 not a callable form — quote it)",
            )
        })?;
        let (head, args) = items.split_first().ok_or_else(|| {
            CompileError::new("empty application `()` cannot be evaluated (quote it as `'()` for nil)")
        })?;

        // A `LAMBDA` form in head position is a *direct application*:
        // `((LAMBDA (p …) body) a …)`.  (A bare, unapplied `LAMBDA` is
        // handled as a `Symbol("LAMBDA")` head below — it's an error,
        // since lambda-as-a-value needs closures.)
        if is_lambda_form(head) {
            return self.lower_lambda_application(head, args);
        }
        // A `LABEL` form in head position is a *direct application* of a
        // named (recursive) lambda: `((LABEL F (LAMBDA (p …) body)) a …)`.
        // (A bare, unapplied `LABEL` falls through to the `"LABEL"` arm
        // below — also an error, for the same closures reason.)
        if is_label_form(head) {
            return self.lower_label_application(head, args);
        }

        match head {
            LispExpr::Symbol(name) => {
                let name = name.as_str();

                // A call whose head is a `LABEL`-bound name in scope is a
                // (possibly recursive) user-function call — lower it to a
                // `call`.  Checked *before* the parameter check and the
                // primitive table so a labelled name lexically shadows both
                // (and so a body can refer to its own `F`).
                if let Some((internal, arity)) = self.functions_in_scope.get(name).cloned() {
                    if args.len() != arity {
                        return Err(CompileError::new(format!(
                            "function `{name}` expects {arity} argument(s), got {}",
                            args.len()
                        )));
                    }
                    return self.lower_call_to(&internal, args);
                }

                // A parameter in scope holds a runtime *value* (possibly a
                // closure).  `(F a…)` where `F` is a parameter is a dynamic
                // application — evaluate `F` and `apply` it.  Checked before
                // the primitive table, so a parameter lexically shadows a
                // primitive of the same spelling (correct scoping).
                if self.params.contains(name) {
                    return self.lower_dynamic_apply(head, args);
                }

                match name {
                    "QUOTE" => {
                        let datum = expect_arity(name, args, 1)?[0];
                        self.lower_quote(datum)
                    }
                    "CONS" => {
                        let a = expect_arity(name, args, 2)?;
                        let va = self.lower_expr(a[0])?;
                        let vb = self.lower_expr(a[1])?;
                        Ok(self.emit_builtin("cons", &[va, vb], REF_PAIR))
                    }
                    "CAR" => {
                        let a = expect_arity(name, args, 1)?;
                        let vx = self.lower_expr(a[0])?;
                        Ok(self.emit_builtin("car", &[vx], "any"))
                    }
                    "CDR" => {
                        let a = expect_arity(name, args, 1)?;
                        let vx = self.lower_expr(a[0])?;
                        Ok(self.emit_builtin("cdr", &[vx], "any"))
                    }
                    "ATOM" => {
                        // (ATOM x) ≡ (not (pair? x))
                        let a = expect_arity(name, args, 1)?;
                        let vx = self.lower_expr(a[0])?;
                        let is_pair = self.emit_builtin("pair?", &[vx], "bool");
                        Ok(self.emit_builtin("not", &[is_pair], "bool"))
                    }
                    "EQ" => {
                        // (EQ a b) ≡ atom identity.  On McCarthy's domain
                        // (atoms: symbols / integers) `lispy-runtime`'s
                        // `equal?` *is* identity — `=` is numeric-only and
                        // rejects symbols, so `equal?` is the right builtin.
                        let a = expect_arity(name, args, 2)?;
                        let va = self.lower_expr(a[0])?;
                        let vb = self.lower_expr(a[1])?;
                        Ok(self.emit_builtin("equal?", &[va, vb], "bool"))
                    }
                    "COND" => self.lower_cond(args),
                    // A bare `LAMBDA` reached here is in *value* position
                    // (not directly applied).  As of L2c-3a that is a
                    // first-class **closure value** — lower it to
                    // `(*CLOSURE* fn-name)`.  (A *directly applied* lambda
                    // `((LAMBDA …) a…)` is intercepted by `is_lambda_form`
                    // above and stays a static `call`.)
                    "LAMBDA" => self.lower_lambda_value(expr),
                    // A bare `LABEL` in value position is a *recursive*
                    // closure value — that needs free-variable capture
                    // machinery, so it lands in L2c-3b.  Applied directly —
                    // `((LABEL F (LAMBDA …)) a…)` — it is intercepted by
                    // `is_label_form` above and works.
                    "LABEL" => Err(CompileError::new(
                        "a LABEL must be applied directly — \
                         `((LABEL name (LAMBDA (params) body)) args …)`. Using a labelled \
                         function as a value (a recursive closure) lands in L2c-3b.",
                    )),
                    other => Err(CompileError::new(format!(
                        "unknown form `{other}`: supported forms are QUOTE, CONS, CAR, CDR, ATOM, \
                         EQ, COND, LAMBDA, and direct LAMBDA / LABEL application"
                    ))),
                }
            }
            // The head is itself an application (it is not a bare `LAMBDA`/
            // `LABEL` form — those were intercepted above).  Evaluate it; if
            // it yields a closure, `apply` runs it, otherwise the VM raises a
            // clean `NotAClosure` at runtime.  e.g. `((CAR FNS) 'A)`.
            LispExpr::Cons(..) => self.lower_dynamic_apply(head, args),
            // Integers and the empty list are not callable.
            other => Err(CompileError::new(format!(
                "cannot apply a {} as a function (only a primitive, a LAMBDA/LABEL form, a \
                 parameter holding a closure, or an expression that returns one)",
                kind_of(other)
            ))),
        }
    }

    // -----------------------------------------------------------------------
    // LAMBDA application (L2c-1)
    // -----------------------------------------------------------------------

    /// Lower a direct lambda application
    /// `((LAMBDA (p1 … pn) body) a1 … an)`.
    ///
    /// The lambda becomes a fresh top-level [`IIRFunction`] (a gensym
    /// name) whose parameters are `p1…pn` and whose body is lowered with
    /// **only those** parameters in scope — McCarthy Lisp lambdas do not
    /// capture free variables in L2c-1 (closures are a later phase), so a
    /// body reference to anything other than its own params is an
    /// unbound-variable error.  The application itself lowers its
    /// arguments in the *caller's* scope and emits a `call` to the new
    /// function.
    fn lower_lambda_application(
        &mut self,
        lambda: &LispExpr,
        args: &[&LispExpr],
    ) -> Result<String, CompileError> {
        let (params, body) = lambda_parts(lambda)?;
        if args.len() != params.len() {
            return Err(CompileError::new(format!(
                "lambda expects {} argument(s), got {}",
                params.len(),
                args.len()
            )));
        }
        // A direct application knows its target statically, so it can emit a
        // plain `call` (no closure value needed).
        let fn_name = self.lift_lambda(&params, body)?;
        self.lower_call_to(&fn_name, args)
    }

    /// Lift a `(params, body)` lambda to a fresh top-level [`IIRFunction`]
    /// (gensym `lambda_<n>`) and return its name.  The body is lowered in a
    /// **fresh parameter scope** — only `params` are visible — so a
    /// reference to anything else is an unbound-variable error (McCarthy
    /// lambdas do not capture free variables until L2c-3b).  Shared by
    /// direct application ([`lower_lambda_application`]) and lambda-as-value
    /// ([`lower_lambda_value`]).
    ///
    /// [`lower_lambda_application`]: Self::lower_lambda_application
    /// [`lower_lambda_value`]: Self::lower_lambda_value
    fn lift_lambda(
        &mut self,
        params: &[&str],
        body: &LispExpr,
    ) -> Result<String, CompileError> {
        let fn_name = format!("lambda_{}", self.fn_ctr);
        self.fn_ctr += 1;

        // Build the lambda's body in a fresh instruction buffer with a
        // fresh parameter scope, then restore the caller's buffer + scope.
        let saved_instrs = std::mem::take(&mut self.instrs);
        let saved_params =
            std::mem::replace(&mut self.params, params.iter().map(|p| p.to_string()).collect());
        let body_reg = self.lower_expr(body)?;
        self.emit(IIRInstr::new("ret", None, vec![Operand::Var(body_reg)], "any"));
        let body_instrs = std::mem::replace(&mut self.instrs, saved_instrs);
        self.params = saved_params;

        let mut f = IIRFunction::new(
            fn_name.clone(),
            params.iter().map(|p| (p.to_string(), "any".to_string())).collect(),
            "any",
            body_instrs,
        );
        f.register_count = self.tmp;
        self.functions.push(f);
        Ok(fn_name)
    }

    /// Lower a `LAMBDA` used in **value** position into a *closure value*:
    /// `((LAMBDA (p…) body))` → `(*CLOSURE* fn-name)`.  The lambda is lifted
    /// exactly as for a direct application, but instead of a `call` we
    /// materialise a tagged cons that the VM's `apply` opcode can dispatch
    /// on at runtime.  In L2c-3a the captured environment is empty (nil) —
    /// the body still sees only its own parameters; free-variable capture is
    /// L2c-3b.
    fn lower_lambda_value(&mut self, lambda: &LispExpr) -> Result<String, CompileError> {
        let (params, body) = lambda_parts(lambda)?;
        let fn_name = self.lift_lambda(&params, body)?;
        Ok(self.emit_closure(&fn_name))
    }

    /// Materialise a closure value `(*CLOSURE* fn-name . env)` for the
    /// already-lifted function `fn_name`.  In L2c-3a `env` is nil, so the
    /// value is the 2-element list `(*CLOSURE* fn-name)` =
    /// `cons(*CLOSURE*, cons(fn-name, nil))`.
    ///
    /// The tag is the interned symbol `*CLOSURE*`.  It is deliberately
    /// **un-forgeable from source**: a McCarthy symbol is `[A-Z][A-Z0-9-]*`,
    /// so `*CLOSURE*` cannot be produced by the lexer — a user program can
    /// never `QUOTE` one into existence, so a value the VM accepts as a
    /// closure was always built here.
    fn emit_closure(&mut self, fn_name: &str) -> String {
        let tag = self.emit_symbol(CLOSURE_TAG);
        let fnsym = self.emit_symbol(fn_name);
        let nil = self.emit_nil();
        let env_cell = self.emit_builtin("cons", &[fnsym, nil], REF_PAIR);
        self.emit_builtin("cons", &[tag, env_cell], REF_PAIR)
    }

    /// Lower a *dynamic application* — a call whose head is an expression
    /// that evaluates to a closure value (a parameter holding a closure, or
    /// a nested application that returns one).  Unlike a static `call`
    /// (which names its callee), this evaluates the head to a register and
    /// emits the `apply` opcode; the VM destructures the closure at runtime.
    /// Arity is therefore **not** known at compile time — the VM checks it
    /// when it runs the callee.
    fn lower_dynamic_apply(
        &mut self,
        head: &LispExpr,
        args: &[&LispExpr],
    ) -> Result<String, CompileError> {
        let fn_reg = self.lower_expr(head)?;
        let mut arg_regs = Vec::with_capacity(args.len());
        for a in args {
            arg_regs.push(self.lower_expr(a)?);
        }
        let dest = self.fresh();
        let mut srcs = Vec::with_capacity(arg_regs.len() + 1);
        srcs.push(Operand::Var(fn_reg));
        srcs.extend(arg_regs.into_iter().map(Operand::Var));
        self.emit(IIRInstr::new("apply", Some(dest.clone()), srcs, "any"));
        Ok(dest)
    }

    // -----------------------------------------------------------------------
    // LABEL application (L2c-2) — named / recursive functions
    // -----------------------------------------------------------------------

    /// Lower a direct application of a *named* lambda
    /// `((LABEL F (LAMBDA (p1 … pn) body)) a1 … an)`.
    ///
    /// This is exactly [`lower_lambda_application`], with one addition: the
    /// name `F` is bound to the new function in `functions_in_scope`
    /// **before** the body is lowered, so a call `(F …)` inside `body`
    /// resolves to a `call` back into the same function — i.e. `F` can
    /// recurse (or call itself indirectly through nested forms).  The
    /// binding is scoped to the body: we save the previously-bound entry
    /// (if any) and restore it afterwards, so `F` is invisible outside.
    ///
    /// No new VM machinery is needed — a self-call is an ordinary `call`,
    /// and the VM bounds call nesting with `MAX_CALL_DEPTH` + the shared
    /// instruction budget, so even a non-terminating recursion errors
    /// cleanly instead of overflowing the native stack.
    ///
    /// [`lower_lambda_application`]: Self::lower_lambda_application
    fn lower_label_application(
        &mut self,
        label: &LispExpr,
        args: &[&LispExpr],
    ) -> Result<String, CompileError> {
        let (name, lambda) = label_parts(label)?;
        let (params, body) = lambda_parts(lambda)?;
        if args.len() != params.len() {
            return Err(CompileError::new(format!(
                "labelled function `{name}` expects {} argument(s), got {}",
                params.len(),
                args.len()
            )));
        }

        let fn_name = format!("label_{}", self.fn_ctr);
        self.fn_ctr += 1;

        // Build the body in a fresh instruction buffer + parameter scope,
        // with `F` bound to this function so the body can recurse.  Save
        // and restore all three pieces of mutable state.
        let saved_instrs = std::mem::take(&mut self.instrs);
        let saved_params =
            std::mem::replace(&mut self.params, params.iter().map(|p| p.to_string()).collect());
        let shadowed = self
            .functions_in_scope
            .insert(name.to_string(), (fn_name.clone(), params.len()));

        let body_reg = self.lower_expr(body)?;
        self.emit(IIRInstr::new("ret", None, vec![Operand::Var(body_reg)], "any"));

        let body_instrs = std::mem::replace(&mut self.instrs, saved_instrs);
        self.params = saved_params;
        match shadowed {
            Some(prev) => {
                self.functions_in_scope.insert(name.to_string(), prev);
            }
            None => {
                self.functions_in_scope.remove(name);
            }
        }

        let mut f = IIRFunction::new(
            fn_name.clone(),
            params.iter().map(|p| (p.to_string(), "any".to_string())).collect(),
            "any",
            body_instrs,
        );
        f.register_count = self.tmp;
        self.functions.push(f);

        // Back in the caller: lower the arguments, then emit the call.
        self.lower_call_to(&fn_name, args)
    }

    /// Lower a call to an already-registered IIR function `fn_name`:
    /// evaluate `args` in the *current* scope and emit
    /// `call dest, [Var(fn_name), arg_regs…]`.  Shared by direct `LAMBDA`
    /// application, `LABEL` application, and in-body calls to a labelled
    /// name (recursion).
    fn lower_call_to(
        &mut self,
        fn_name: &str,
        args: &[&LispExpr],
    ) -> Result<String, CompileError> {
        let mut arg_regs = Vec::with_capacity(args.len());
        for a in args {
            arg_regs.push(self.lower_expr(a)?);
        }
        let dest = self.fresh();
        let mut srcs = Vec::with_capacity(arg_regs.len() + 1);
        srcs.push(Operand::Var(fn_name.to_string()));
        srcs.extend(arg_regs.into_iter().map(Operand::Var));
        self.emit(IIRInstr::new("call", Some(dest.clone()), srcs, "any"));
        Ok(dest)
    }

    // -----------------------------------------------------------------------
    // COND lowering (control flow)
    // -----------------------------------------------------------------------

    /// Lower `(COND (p1 e1) (p2 e2) … (pn en))`.
    ///
    /// Evaluate each predicate in turn; the value of the COND is the `ei`
    /// of the first `pi` that is true (non-`nil`, non-`#f`).  If no clause
    /// matches, the result is `nil` (McCarthy's 1960 `cond` is undefined
    /// in that case; returning `nil` is the conventional total extension).
    ///
    /// The lowering is a chain of `jmp_if_false` + labels, with every
    /// clause's value funnelled into one `result` register via `mov`:
    ///
    /// ```text
    ///       <lower p1> → vp1
    ///       jmp_if_false vp1, L_next_0
    ///       <lower e1>  → ve1
    ///       mov result, ve1
    ///       jmp L_end_0
    ///   L_next_0:
    ///       <lower p2> → vp2
    ///       jmp_if_false vp2, L_next_1
    ///       <lower e2>  → ve2
    ///       mov result, ve2
    ///       jmp L_end_0
    ///   L_next_1:
    ///       const result, nil          ; no clause matched
    ///   L_end_0:
    ///       ; result holds the value
    /// ```
    ///
    /// The catch-all clause is written with a truthy predicate — e.g.
    /// `('T …)`.  A bare `T` would be an unbound variable (bindings arrive
    /// with `LAMBDA`/`LABEL` in L2c), so quote it.
    fn lower_cond(&mut self, clauses: &[&LispExpr]) -> Result<String, CompileError> {
        let result = self.fresh();
        let end_label = self.fresh_label("cond_end");

        for clause in clauses {
            // Each clause is a proper list `(predicate expression)`.
            let parts = proper_list(clause).ok_or_else(|| {
                CompileError::new("a COND clause must be a list `(predicate expression)`")
            })?;
            if parts.len() != 2 {
                return Err(CompileError::new(format!(
                    "a COND clause must be `(predicate expression)` — 2 elements, got {}",
                    parts.len()
                )));
            }
            let (predicate, expression) = (parts[0], parts[1]);

            let next_label = self.fresh_label("cond_next");
            let vp = self.lower_expr(predicate)?;
            self.emit_jmp_if_false(&vp, &next_label);
            let ve = self.lower_expr(expression)?;
            self.emit_mov(&result, &ve);
            self.emit_jmp(&end_label);
            self.emit_label(&next_label);
        }

        // Fell past every clause → nil.
        let nil = self.emit_nil();
        self.emit_mov(&result, &nil);
        self.emit_label(&end_label);
        Ok(result)
    }

    // -----------------------------------------------------------------------
    // Quote lowering (data position)
    // -----------------------------------------------------------------------

    /// Materialise a quoted datum as runtime values.  Unlike
    /// [`lower_expr`], a `Symbol` here is *data* (a symbol literal), not
    /// a variable reference, and a `Cons` is a literal pair, not a call.
    ///
    /// The **cdr-spine is walked iteratively**, not recursively: a flat
    /// quoted list `'(A A … A)` of N elements has paren-depth 1, so it
    /// sails past the parser's nesting cap (which bounds *depth*, not
    /// list *length*) — a recursive `cdr` walk would then recurse N
    /// native frames deep and overflow the stack on large N (a cheap
    /// single-line DoS).  We instead collect the spine's `car`s in a
    /// loop, lower the final tail, then fold the `cons` cells from the
    /// tail back.  `car` positions still recurse, but each nested list
    /// there requires a paren and so is bounded by the parser's
    /// `MAX_PAREN_DEPTH`.
    fn lower_quote(&mut self, datum: &LispExpr) -> Result<String, CompileError> {
        let mut cars: Vec<String> = Vec::new();
        let mut cur = datum;
        let tail = loop {
            match cur {
                LispExpr::Cons(car, cdr) => {
                    cars.push(self.lower_quote(car)?); // car-nesting bounded by paren depth
                    cur = cdr.as_ref();
                }
                LispExpr::Int(n) => break self.emit_int(*n),
                LispExpr::Nil => break self.emit_nil(),
                LispExpr::Symbol(s) => break self.emit_symbol(s),
            }
        };
        // Fold: cons(car_0, cons(car_1, … cons(car_{k-1}, tail))).
        let mut acc = tail;
        for car in cars.into_iter().rev() {
            acc = self.emit_builtin("cons", &[car, acc], REF_PAIR);
        }
        Ok(acc)
    }

    // -----------------------------------------------------------------------
    // Instruction emitters
    // -----------------------------------------------------------------------

    /// `const v, Int(n) : i64`
    fn emit_int(&mut self, n: i64) -> String {
        let v = self.fresh();
        self.emit(IIRInstr::new("const", Some(v.clone()), vec![Operand::Int(n)], "i64"));
        v
    }

    /// `const v, Int(0) : ref<LispyPair>` — the runtime nil sentinel.
    fn emit_nil(&mut self) -> String {
        let v = self.fresh();
        self.emit(IIRInstr::new("const", Some(v.clone()), vec![Operand::Int(0)], REF_PAIR));
        v
    }

    /// `const v, Var(name) : symbol` — the IIR convention the runtime
    /// interns into a symbol value (NOT a heap string, which `Str` would
    /// produce).
    fn emit_symbol(&mut self, name: &str) -> String {
        let v = self.fresh();
        self.emit(IIRInstr::new(
            "const",
            Some(v.clone()),
            vec![Operand::Var(name.to_string())],
            "symbol",
        ));
        v
    }

    /// `call_builtin v, [Var(builtin), args…] : type_hint`.
    ///
    /// The `call_builtin` convention puts the builtin's name as the
    /// first source operand (a `Var`), followed by the argument
    /// registers.  `may_alloc` is set for `cons`, which heap-allocates.
    fn emit_builtin(&mut self, builtin: &str, args: &[String], type_hint: &str) -> String {
        let v = self.fresh();
        let mut srcs = Vec::with_capacity(args.len() + 1);
        srcs.push(Operand::Var(builtin.to_string()));
        srcs.extend(args.iter().map(|a| Operand::Var(a.clone())));
        let mut instr = IIRInstr::new("call_builtin", Some(v.clone()), srcs, type_hint);
        instr.may_alloc = builtin == "cons";
        self.emit(instr);
        v
    }

    // -----------------------------------------------------------------------
    // Control-flow emitters (COND)
    // -----------------------------------------------------------------------

    /// `mov dest, src` — copy a register's value.
    fn emit_mov(&mut self, dest: &str, src: &str) {
        self.emit(IIRInstr::new(
            "mov",
            Some(dest.to_string()),
            vec![Operand::Var(src.to_string())],
            "any",
        ));
    }

    /// `label NAME` — a branch-target marker.
    fn emit_label(&mut self, name: &str) {
        self.emit(IIRInstr::new("label", None, vec![Operand::Var(name.to_string())], "void"));
    }

    /// `jmp NAME` — unconditional branch.
    fn emit_jmp(&mut self, target: &str) {
        self.emit(IIRInstr::new("jmp", None, vec![Operand::Var(target.to_string())], "void"));
    }

    /// `jmp_if_false cond, NAME` — branch to `NAME` when `cond` is falsy.
    fn emit_jmp_if_false(&mut self, cond: &str, target: &str) {
        self.emit(IIRInstr::new(
            "jmp_if_false",
            None,
            vec![Operand::Var(cond.to_string()), Operand::Var(target.to_string())],
            "void",
        ));
    }
}

// ===========================================================================
// AST helpers
// ===========================================================================

/// A short, allocation-free description of an expression's *kind*, for
/// error messages — never recurses into the structure (unlike `Display`).
fn kind_of(expr: &LispExpr) -> &'static str {
    match expr {
        LispExpr::Nil => "empty list",
        LispExpr::Symbol(_) => "symbol",
        LispExpr::Int(_) => "integer",
        LispExpr::Cons(..) => "list",
    }
}

/// True iff `expr` is a `(LAMBDA …)` form (a list whose head is the
/// symbol `LAMBDA`).
fn is_lambda_form(expr: &LispExpr) -> bool {
    matches!(expr, LispExpr::Cons(car, _) if matches!(car.as_ref(), LispExpr::Symbol(s) if s == "LAMBDA"))
}

/// Destructure a `(LAMBDA (p1 … pn) body)` form into its parameter names
/// and body.  The caller guarantees `is_lambda_form(lambda)`.
fn lambda_parts(lambda: &LispExpr) -> Result<(Vec<&str>, &LispExpr), CompileError> {
    let items = proper_list(lambda)
        .ok_or_else(|| CompileError::new("malformed LAMBDA (improper list)"))?;
    if items.len() != 3 {
        return Err(CompileError::new(format!(
            "a LAMBDA must be `(LAMBDA (params) body)` — 3 elements, got {}",
            items.len()
        )));
    }
    let param_list = proper_list(items[1]).ok_or_else(|| {
        CompileError::new("a LAMBDA parameter list must be a proper list `(p1 p2 …)`")
    })?;
    let mut params: Vec<&str> = Vec::with_capacity(param_list.len());
    for p in &param_list {
        match p {
            LispExpr::Symbol(s) => params.push(s.as_str()),
            other => {
                return Err(CompileError::new(format!(
                    "a LAMBDA parameter must be a symbol, got a {}",
                    kind_of(other)
                )))
            }
        }
    }
    let mut seen = HashSet::new();
    for p in &params {
        if !seen.insert(*p) {
            return Err(CompileError::new(format!("duplicate LAMBDA parameter `{p}`")));
        }
    }
    Ok((params, items[2]))
}

/// True iff `expr` is a `(LABEL …)` form (a list whose head is the symbol
/// `LABEL`).
fn is_label_form(expr: &LispExpr) -> bool {
    matches!(expr, LispExpr::Cons(car, _) if matches!(car.as_ref(), LispExpr::Symbol(s) if s == "LABEL"))
}

/// Destructure a `(LABEL name (LAMBDA (params) body))` form into the
/// labelled name and the inner `LAMBDA` form.  The caller guarantees
/// `is_label_form(label)`; the `LAMBDA` is destructured separately by
/// [`lambda_parts`].
fn label_parts(label: &LispExpr) -> Result<(&str, &LispExpr), CompileError> {
    let items = proper_list(label)
        .ok_or_else(|| CompileError::new("malformed LABEL (improper list)"))?;
    if items.len() != 3 {
        return Err(CompileError::new(format!(
            "a LABEL must be `(LABEL name (LAMBDA (params) body))` — 3 elements, got {}",
            items.len()
        )));
    }
    let name = match items[1] {
        LispExpr::Symbol(s) => s.as_str(),
        other => {
            return Err(CompileError::new(format!(
                "a LABEL name must be a symbol, got a {}",
                kind_of(other)
            )))
        }
    };
    if !is_lambda_form(items[2]) {
        return Err(CompileError::new(
            "the body of a LABEL must be a LAMBDA form — `(LABEL name (LAMBDA (params) body))`",
        ));
    }
    Ok((name, items[2]))
}

/// Flatten a proper list (`Cons` chain terminated by `Nil`) into its
/// elements.  Returns `None` if the chain is improper (ends in a dotted
/// tail) — those are data, never callable forms.
fn proper_list(expr: &LispExpr) -> Option<Vec<&LispExpr>> {
    let mut items = Vec::new();
    let mut cur = expr;
    loop {
        match cur {
            LispExpr::Cons(car, cdr) => {
                items.push(car.as_ref());
                cur = cdr.as_ref();
            }
            LispExpr::Nil => return Some(items),
            _ => return None, // improper / dotted tail
        }
    }
}

/// Check that `args` has exactly `n` elements, returning them as a slice
/// of references.
fn expect_arity<'a>(
    form: &str,
    args: &'a [&'a LispExpr],
    n: usize,
) -> Result<&'a [&'a LispExpr], CompileError> {
    if args.len() == n {
        Ok(args)
    } else {
        Err(CompileError::new(format!(
            "`{form}` expects {n} argument(s), got {}",
            args.len()
        )))
    }
}

// ===========================================================================
// Unit tests — IIR shape (execution is covered by tests/run_e2e.rs)
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// The `main` function's instruction ops, in order.
    fn ops(src: &str) -> Vec<String> {
        let m = compile_source(src, "t").expect("compile");
        m.functions[0].instructions.iter().map(|i| i.op.clone()).collect()
    }

    #[test]
    fn module_metadata() {
        let m = compile_source("42", "demo").unwrap();
        assert_eq!(m.name, "demo");
        assert_eq!(m.language, "mccarthy-lisp");
        assert_eq!(m.entry_point.as_deref(), Some("main"));
        assert_eq!(m.functions.len(), 1);
        assert_eq!(m.functions[0].name, "main");
        assert!(m.validate().is_empty());
    }

    #[test]
    fn integer_literal_is_const_then_ret() {
        assert_eq!(ops("42"), vec!["const", "ret"]);
    }

    #[test]
    fn empty_program_returns_nil() {
        // No forms → just a nil const + ret.
        assert_eq!(ops(""), vec!["const", "ret"]);
    }

    #[test]
    fn quoted_list_expands_to_cons_chain() {
        // '(A B C) → cons(A, cons(B, cons(C, nil)))
        // consts: A, B, C, nil = 4 consts; conses = 3; plus ret.
        let o = ops("'(A B C)");
        assert_eq!(o.iter().filter(|x| *x == "const").count(), 4);
        assert_eq!(o.iter().filter(|x| *x == "call_builtin").count(), 3);
        assert_eq!(o.last().unwrap(), "ret");
    }

    #[test]
    fn car_emits_a_car_builtin() {
        let m = compile_source("(CAR '(A B C))", "t").unwrap();
        let calls: Vec<&str> = m.functions[0]
            .instructions
            .iter()
            .filter(|i| i.op == "call_builtin")
            .map(|i| match &i.srcs[0] {
                Operand::Var(s) => s.as_str(),
                _ => "?",
            })
            .collect();
        assert!(calls.contains(&"car"));
        assert!(calls.contains(&"cons")); // building the quoted list
    }

    #[test]
    fn atom_lowers_to_pair_then_not() {
        let m = compile_source("(ATOM 'X)", "t").unwrap();
        let builtins: Vec<&str> = m.functions[0]
            .instructions
            .iter()
            .filter(|i| i.op == "call_builtin")
            .map(|i| match &i.srcs[0] {
                Operand::Var(s) => s.as_str(),
                _ => "?",
            })
            .collect();
        assert_eq!(builtins, vec!["pair?", "not"]);
    }

    #[test]
    fn cons_sets_may_alloc() {
        let m = compile_source("(CONS 'A 'B)", "t").unwrap();
        let cons = m.functions[0]
            .instructions
            .iter()
            .find(|i| i.op == "call_builtin" && matches!(&i.srcs[0], Operand::Var(s) if s == "cons"))
            .unwrap();
        assert!(cons.may_alloc);
    }

    #[test]
    fn symbol_const_uses_var_operand() {
        // A quoted symbol must be `const Var(name)`, not Str (which would
        // make a heap string), so the VM interns it as a symbol.
        let m = compile_source("'FOO", "t").unwrap();
        let c = &m.functions[0].instructions[0];
        assert_eq!(c.op, "const");
        assert!(matches!(&c.srcs[0], Operand::Var(s) if s == "FOO"));
    }

    #[test]
    fn nil_const_is_ref_pair_zero() {
        let m = compile_source("'()", "t").unwrap();
        let c = &m.functions[0].instructions[0];
        assert_eq!(c.op, "const");
        assert!(matches!(&c.srcs[0], Operand::Int(0)));
        assert_eq!(c.type_hint, "ref<LispyPair>");
    }

    // ---- error paths ----

    #[test]
    fn bare_symbol_is_unbound() {
        let e = compile_source("X", "t").unwrap_err();
        assert!(e.message.contains("unbound variable"));
    }

    #[test]
    fn cond_emits_branch_and_funnel_ops() {
        // (COND ((ATOM 'X) 'A) ('T 'B)) — two clauses → two jmp_if_false,
        // and every clause path funnels into one register via `mov`.
        let m = compile_source("(COND ((ATOM 'X) 'A) ('T 'B))", "t").unwrap();
        let ops: Vec<&str> =
            m.functions[0].instructions.iter().map(|i| i.op.as_str()).collect();
        assert_eq!(ops.iter().filter(|o| **o == "jmp_if_false").count(), 2);
        assert!(ops.contains(&"jmp"));
        assert!(ops.contains(&"label"));
        assert!(ops.contains(&"mov"));
        // The emitted IIR is well-formed (all branch targets are defined).
        assert!(m.validate().is_empty());
    }

    #[test]
    fn cond_clause_must_be_a_pair() {
        assert!(compile_source("(COND ('T))", "t").unwrap_err().message.contains("2 elements"));
        assert!(compile_source("(COND ('T 'A 'B))", "t").unwrap_err().message.contains("2 elements"));
    }

    #[test]
    fn empty_cond_is_nil() {
        // (COND) with no clauses lowers to a nil result — and validates.
        let m = compile_source("(COND)", "t").unwrap();
        assert!(m.validate().is_empty());
    }

    #[test]
    fn applied_lambda_emits_a_function_and_a_call() {
        // ((LAMBDA (X) (CAR X)) '(A B)) → a `lambda_0` function + a `call`.
        let m = compile_source("((LAMBDA (X) (CAR X)) '(A B))", "t").unwrap();
        // Two functions: the lambda and `main`.
        assert_eq!(m.functions.len(), 2);
        assert!(m.functions.iter().any(|f| f.name == "lambda_0"));
        let lam = m.functions.iter().find(|f| f.name == "lambda_0").unwrap();
        assert_eq!(lam.param_names(), vec!["X"]);
        // main contains a `call` to the lambda.
        let main = m.get_function("main").unwrap();
        assert!(main.instructions.iter().any(|i| i.op == "call"
            && matches!(i.srcs.first(), Some(Operand::Var(s)) if s == "lambda_0")));
        assert!(m.validate().is_empty());
    }

    #[test]
    fn lambda_param_is_in_scope_in_its_body() {
        // The body may reference its own param `X` (a register read).
        assert!(compile_source("((LAMBDA (X) X) 'A)", "t").is_ok());
    }

    // ---- L2c-3a: closures as values + dynamic apply ----

    #[test]
    fn applying_a_parameter_emits_apply_not_call() {
        // ((LAMBDA (F) (F 'A)) (LAMBDA (X) X)) — inside the outer body, `F`
        // is a parameter, so `(F 'A)` is a *dynamic* application → `apply`.
        let m = compile_source("((LAMBDA (F) (F 'A)) (LAMBDA (X) X))", "t").unwrap();
        // The outer lambda's body (lambda_0) contains an `apply` whose head
        // operand is the parameter register `F` (not a function name).
        let outer = m.get_function("lambda_0").unwrap();
        let apply = outer.instructions.iter().find(|i| i.op == "apply").expect("an apply op");
        assert!(matches!(apply.srcs.first(), Some(Operand::Var(s)) if s == "F"));
        // The argument lambda is lifted + wrapped as a closure value in main.
        let main = m.get_function("main").unwrap();
        assert!(main.instructions.iter().any(|i| i.op == "const"
            && matches!(i.srcs.first(), Some(Operand::Var(s)) if s == "*CLOSURE*")));
        assert!(m.validate().is_empty());
    }

    #[test]
    fn closure_value_has_unforgeable_tag() {
        // The `*CLOSURE*` tag is not a lexable symbol, so a user program
        // literally cannot type it — confirm the lexer rejects it as source
        // even though the compiler emits it internally.
        assert!(compile_source("'*CLOSURE*", "t").is_err());
    }

    #[test]
    fn applying_a_nested_application_emits_apply() {
        // ((CAR (CONS (LAMBDA (X) X) '())) 'A) — the head is a nested
        // application returning a closure → dynamic apply.
        let m = compile_source("((CAR (CONS (LAMBDA (X) X) '())) 'A)", "t").unwrap();
        let main = m.get_function("main").unwrap();
        assert!(main.instructions.iter().any(|i| i.op == "apply"));
        assert!(m.validate().is_empty());
    }

    #[test]
    fn cannot_apply_an_integer() {
        let e = compile_source("(5 'A)", "t").unwrap_err();
        assert!(e.message.contains("cannot apply"));
    }

    #[test]
    fn lambda_body_free_variable_is_unbound() {
        // `Y` is not a parameter — no closures yet, so it is unbound.
        let e = compile_source("((LAMBDA (X) Y) 'A)", "t").unwrap_err();
        assert!(e.message.contains("unbound variable"));
    }

    #[test]
    fn lambda_arity_is_checked() {
        assert!(compile_source("((LAMBDA (X Y) X) 'A)", "t").unwrap_err().message.contains("argument"));
    }

    #[test]
    fn duplicate_lambda_param_rejected() {
        assert!(compile_source("((LAMBDA (X X) X) 'A 'B)", "t").unwrap_err().message.contains("duplicate"));
    }

    #[test]
    fn bare_lambda_value_lowers_to_a_closure() {
        // As of L2c-3a, a LAMBDA in value position is a first-class closure
        // value, not an error: it lifts a `lambda_0` function and emits the
        // closure cons `(*CLOSURE* lambda_0)`.
        let m = compile_source("(LAMBDA (X) X)", "t").unwrap();
        assert!(m.functions.iter().any(|f| f.name == "lambda_0"));
        let main = m.get_function("main").unwrap();
        // The tag symbol `*CLOSURE*` is emitted as a `const Var` and is not
        // a lexable McCarthy symbol (so no source could forge it).
        assert!(main.instructions.iter().any(|i| i.op == "const"
            && matches!(i.srcs.first(), Some(Operand::Var(s)) if s == "*CLOSURE*")));
        // The function name is embedded as a symbol const too.
        assert!(main.instructions.iter().any(|i| i.op == "const"
            && matches!(i.srcs.first(), Some(Operand::Var(s)) if s == "lambda_0")));
        // No `call`/`apply` — it is a value, not an application.
        assert!(main.instructions.iter().all(|i| i.op != "call" && i.op != "apply"));
        assert!(m.validate().is_empty());
    }

    #[test]
    fn bare_label_value_is_rejected() {
        // An unapplied LABEL (a *recursive* closure) needs capture → L2c-3b.
        assert!(compile_source("(LABEL F (LAMBDA (X) X))", "t")
            .unwrap_err()
            .message
            .contains("L2c-3b"));
    }

    #[test]
    fn applied_label_emits_a_function_and_a_call() {
        // ((LABEL F (LAMBDA (X) X)) 'A) → a `label_0` function + a `call`.
        let m = compile_source("((LABEL F (LAMBDA (X) X)) 'A)", "t").unwrap();
        assert!(m.functions.iter().any(|f| f.name == "label_0"));
        let lam = m.functions.iter().find(|f| f.name == "label_0").unwrap();
        assert_eq!(lam.param_names(), vec!["X"]);
        let main = m.get_function("main").unwrap();
        assert!(main.instructions.iter().any(|i| i.op == "call"
            && matches!(i.srcs.first(), Some(Operand::Var(s)) if s == "label_0")));
        assert!(m.validate().is_empty());
    }

    #[test]
    fn label_body_can_call_itself_recursively() {
        // The body references `FF` in call position — it must lower to a
        // `call` back into the same `label_0` function (recursion), and
        // the module must validate (the call target exists).
        let src = "((LABEL FF (LAMBDA (X) (COND ((ATOM X) X) ('T (FF (CAR X)))))) '((A) B))";
        let m = compile_source(src, "t").unwrap();
        let body = m.get_function("label_0").unwrap();
        // Exactly one self `call` to `label_0` inside the body.
        let self_calls = body
            .instructions
            .iter()
            .filter(|i| i.op == "call"
                && matches!(i.srcs.first(), Some(Operand::Var(s)) if s == "label_0"))
            .count();
        assert_eq!(self_calls, 1);
        assert!(m.validate().is_empty());
    }

    #[test]
    fn label_name_in_value_position_needs_closures() {
        // Using `F` as a value (not calling it) is a first-class function
        // value → closures error, with a clear message.
        let e = compile_source("((LABEL F (LAMBDA (X) F)) 'A)", "t").unwrap_err();
        assert!(e.message.contains("needs closures"));
    }

    #[test]
    fn label_recursive_call_arity_is_checked() {
        // `(FF X Y)` — two args to a one-param labelled function.
        let e =
            compile_source("((LABEL FF (LAMBDA (X) (FF X X))) 'A)", "t").unwrap_err();
        assert!(e.message.contains("expects 1 argument"));
    }

    #[test]
    fn label_application_arity_is_checked() {
        let e = compile_source("((LABEL F (LAMBDA (X Y) X)) 'A)", "t").unwrap_err();
        assert!(e.message.contains("argument"));
    }

    #[test]
    fn label_name_must_be_a_symbol() {
        let e = compile_source("((LABEL (F) (LAMBDA (X) X)) 'A)", "t").unwrap_err();
        assert!(e.message.contains("LABEL name must be a symbol"));
    }

    #[test]
    fn label_body_must_be_a_lambda() {
        let e = compile_source("((LABEL F 'A) 'B)", "t").unwrap_err();
        assert!(e.message.contains("body of a LABEL must be a LAMBDA"));
    }

    #[test]
    fn label_wrong_element_count_rejected() {
        let e = compile_source("((LABEL F) 'A)", "t").unwrap_err();
        assert!(e.message.contains("3 elements"));
    }

    #[test]
    fn label_name_is_out_of_scope_after_its_body() {
        // `FF` is bound only inside the LABEL body. A *sibling* top-level
        // reference to it must be unbound (lexical scope).
        let e = compile_source("((LABEL FF (LAMBDA (X) X)) 'A) (FF 'B)", "t").unwrap_err();
        assert!(e.message.contains("unknown form") || e.message.contains("unbound"));
    }

    #[test]
    fn wrong_arity_errors() {
        assert!(compile_source("(CAR '(A) '(B))", "t").unwrap_err().message.contains("expects 1"));
        assert!(compile_source("(CONS 'A)", "t").unwrap_err().message.contains("expects 2"));
    }

    #[test]
    fn unknown_primitive_errors() {
        assert!(compile_source("(FROBNICATE 'A)", "t").unwrap_err().message.contains("unknown form"));
    }

    #[test]
    fn lexer_parse_errors_propagate() {
        assert!(compile_source("car", "t").is_err()); // lowercase → parse error
    }

    #[test]
    fn huge_flat_quoted_list_does_not_overflow() {
        // Regression: a flat quoted list has paren-depth 1, so it bypasses
        // the parser's nesting cap. The cdr-spine walk must be iterative,
        // or N tens-of-thousands recurses the stack to death. 50k elements
        // is well past the overflow point of a recursive walk.
        let n = 50_000;
        let mut src = String::with_capacity(2 * n + 4);
        src.push('\'');
        src.push('(');
        for _ in 0..n {
            src.push_str("A ");
        }
        src.push(')');
        let m = compile_source(&src, "t").expect("should compile without overflow");
        // n cons cells + n symbol consts + 1 nil const + 1 ret.
        assert_eq!(m.functions[0].instructions.len(), 2 * n + 2);
    }
}
