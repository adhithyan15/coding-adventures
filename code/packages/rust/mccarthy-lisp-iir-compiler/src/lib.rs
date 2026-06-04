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
//! ## Which Lisp we can lower *today* (L2a scope)
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
//! ## Not yet (later phases)
//!
//! - **L2b** — `COND` (chained `jmp_if_false` + labels).
//! - **L2c** — `LAMBDA` / `LABEL` / user-defined function application.
//!
//! A bare (unquoted) symbol in value position is therefore an *unbound
//! variable* in L2a — there are no bindings yet — and is reported as a
//! [`CompileError`] rather than silently mis-lowered.
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
        c.instrs,
    );
    main.register_count = c.tmp;

    let mut module = IIRModule::new(module_name, "mccarthy-lisp");
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

/// Accumulates the instruction stream for the `main` function.
struct Compiler {
    instrs: Vec<IIRInstr>,
    /// Monotonic counter for fresh SSA temp names (`v0`, `v1`, …).
    tmp: usize,
}

impl Compiler {
    fn new() -> Self {
        Compiler { instrs: Vec::new(), tmp: 0 }
    }

    /// Allocate a fresh, never-reused temp register name.
    fn fresh(&mut self) -> String {
        let name = format!("v{}", self.tmp);
        self.tmp += 1;
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
            LispExpr::Symbol(name) => Err(CompileError::new(format!(
                "unbound variable `{name}`: bare symbols need a binding, which arrives \
                 with LAMBDA/LABEL in a later phase. Did you mean to quote it as `'{name}`?"
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

        let name = match head {
            LispExpr::Symbol(s) => s.as_str(),
            // Describe the head by *kind*, never via `Display` — a quoted
            // head like `('(A A …) …)` could be an arbitrarily large
            // structure, and `LispExpr`'s `Display` recurses on the
            // cdr-spine (a huge flat list would overflow formatting it).
            other => {
                return Err(CompileError::new(format!(
                    "the head of a call must be a primitive symbol in L2a, got a {}",
                    kind_of(other)
                )))
            }
        };

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
                // (EQ a b) ≡ atom identity.  On McCarthy's domain (atoms:
                // symbols / integers) `lispy-runtime`'s `equal?` *is*
                // identity — `=` is numeric-only and rejects symbols, so
                // `equal?` is the correct builtin here.
                let a = expect_arity(name, args, 2)?;
                let va = self.lower_expr(a[0])?;
                let vb = self.lower_expr(a[1])?;
                Ok(self.emit_builtin("equal?", &[va, vb], "bool"))
            }
            "COND" => Err(CompileError::new(
                "`COND` is not supported yet — it lands in L2b (chained jmp_if_false + labels)",
            )),
            "LAMBDA" | "LABEL" => Err(CompileError::new(format!(
                "`{name}` is not supported yet — user functions land in L2c"
            ))),
            other => Err(CompileError::new(format!(
                "unknown form `{other}`: L2a supports QUOTE, CONS, CAR, CDR, ATOM, EQ \
                 (COND→L2b, LAMBDA/LABEL→L2c)"
            ))),
        }
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
    fn cond_is_deferred() {
        assert!(compile_source("(COND ('T 'A))", "t").unwrap_err().message.contains("L2b"));
    }

    #[test]
    fn lambda_is_deferred() {
        assert!(compile_source("(LAMBDA (X) X)", "t").unwrap_err().message.contains("L2c"));
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
