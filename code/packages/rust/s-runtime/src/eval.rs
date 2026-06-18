//! The tree-walking evaluator.
//!
//! [`Interpreter`] walks the generic [`GrammarASTNode`] tree produced by the
//! S parser and computes [`SValue`]s. It deliberately does *not* lower to
//! bytecode or a VM — a direct tree-walk is the shortest faithful path to a
//! working REPL, in the spirit of `macsyma-runtime`. The numeric work is
//! delegated to the statistics substrate (`r-vector`, `statistics-core`).
//!
//! ## Result visibility
//!
//! S distinguishes *visible* results (auto-printed at the prompt) from
//! *invisible* ones (assignments, loops, `print()`'s return value). We track
//! this with a single [`Cell<bool>`]: every value-producing operation marks the
//! result visible on the way out, while assignment, loops, and `print` mark it
//! invisible. Because evaluation is post-order, the *outermost* operation runs
//! last and therefore wins — so `x <- 5` is invisible while `mean(x <- 5)` (a
//! call) is visible, exactly as in S.

use crate::builtins;
use crate::env::{define, lookup, Env, Scope};
use crate::error::{SError, SResult};
use crate::value::{
    arithmetic, assign_index, assign_index2d, bounded_sequence, class_of, compare, format_value,
    index, index2d, membership, negate, Arg, Param, SValue, MAX_SEQ_LEN,
};
use coding_adventures_s_parser::try_parse_s;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use r_vector::{is_na_real, na_real, Double};
use statistics_core::rng::RngState;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The result of evaluating a chunk of S source.
pub struct Outcome {
    /// The value of the last top-level statement.
    pub value: SValue,
    /// Whether that value should be auto-printed (visible) at the REPL.
    pub visible: bool,
    /// Anything the program wrote via `print()`, already formatted.
    pub printed: String,
}

/// A persistent S evaluation session: a global environment plus the
/// accumulated `print()` output and the current visibility flag.
pub struct Interpreter {
    global: Env,
    out: RefCell<String>,
    visible: Cell<bool>,
    /// Current `eval_node` recursion depth, bounded by [`MAX_EVAL_DEPTH`] so
    /// that pathologically nested input (e.g. thousands of nested parens, or a
    /// runaway recursive S function) returns a clean error instead of
    /// overflowing the native stack and aborting the process.
    depth: Cell<usize>,
    /// The session's random-number generator, shared across `r*` sampling calls
    /// (`rnorm`, `runif`, …) and reseeded by `set.seed()`. Held behind a
    /// `RefCell` because builtins receive `&Interpreter` but the generator must
    /// advance its state on every draw.
    rng: RefCell<RngState>,
}

/// The fixed seed a fresh session starts from. Real R seeds from the clock and
/// process state; we use a constant so a brand-new interpreter is reproducible
/// until the program calls `set.seed()`. (Macsyma-style determinism beats
/// surprise here — anyone wanting a fresh stream calls `set.seed`.)
const DEFAULT_SEED: u64 = 4357;

/// Maximum `eval_node` recursion depth. The precedence cascade adds roughly a
/// dozen frames per source nesting level, so this allows comfortably deep
/// real programs (including ordinary recursion) while staying well under the
/// native stack limit.
const MAX_EVAL_DEPTH: usize = 3000;

/// RAII guard that decrements the interpreter's depth counter on scope exit,
/// so every early `return`/`?` in `eval_node` is accounted for.
struct DepthGuard<'a>(&'a Cell<usize>);

impl Drop for DepthGuard<'_> {
    fn drop(&mut self) {
        self.0.set(self.0.get().saturating_sub(1));
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    /// Create a fresh session with all built-ins installed in the global scope.
    pub fn new() -> Self {
        let global = Scope::global();
        builtins::install(&global);
        Interpreter {
            global,
            out: RefCell::new(String::new()),
            visible: Cell::new(false),
            depth: Cell::new(0),
            rng: RefCell::new(RngState::new(DEFAULT_SEED)),
        }
    }

    /// The global environment (for tests and the REPL).
    pub fn global(&self) -> &Env {
        &self.global
    }

    /// Reseed the session generator — the engine behind `set.seed(n)`.
    pub(crate) fn reseed(&self, seed: u64) {
        self.rng.borrow_mut().seed(seed);
    }

    /// Draw from the session generator. The closure gets exclusive access to the
    /// `RngState` for the duration of one sampling builtin, so a single `rnorm`
    /// call advances the stream exactly as far as the values it produced.
    pub(crate) fn sample_with<T>(&self, f: impl FnOnce(&mut RngState) -> T) -> T {
        f(&mut self.rng.borrow_mut())
    }

    /// Append a block of output, one newline-terminated line each.
    fn emit(&self, lines: &[String]) {
        let mut out = self.out.borrow_mut();
        for line in lines {
            out.push_str(line);
            out.push('\n');
        }
    }

    /// Append raw text to the output buffer (used by `cat`, which controls its
    /// own newlines).
    pub(crate) fn emit_raw(&self, text: &str) {
        self.out.borrow_mut().push_str(text);
    }

    /// The S3 `print` generic: dispatch on the value's class to a user or
    /// built-in `print.<class>` method (then `print.default`), falling back to
    /// the standard formatting. The method does its own output (via `cat`,
    /// `print`, …); the default path emits `format_value`.
    pub(crate) fn dispatch_print(&self, value: &SValue) -> SResult<SValue> {
        let mut candidates = class_of(value);
        candidates.push("default".to_string());
        for cls in candidates {
            if let Some(method) = lookup(&self.global, &format!("print.{cls}")) {
                if method.is_callable() {
                    let args = [Arg {
                        name: None,
                        value: value.clone(),
                    }];
                    self.call_value(method, &args)?;
                    return Ok(value.clone());
                }
            }
        }
        self.emit(&format_value(value));
        Ok(value.clone())
    }

    /// Parse and evaluate `src`, returning the value of the last statement.
    pub fn eval_str(&self, src: &str) -> SResult<Outcome> {
        let program = try_parse_s(src).map_err(SError::Parse)?;
        self.eval_program(&program)
    }

    /// Evaluate an already-parsed `program` tree. This is the language-neutral
    /// entry point: it walks a [`GrammarASTNode`] by rule name and is agnostic
    /// to *which* front end produced it. The R runtime parses with `r-parser`
    /// (whose grammar uses the same rule names as `s.grammar`) and calls this
    /// directly, reusing the entire evaluator.
    pub fn eval_program(&self, program: &GrammarASTNode) -> SResult<Outcome> {
        self.out.borrow_mut().clear();

        let mut last = SValue::Null;
        self.visible.set(false);
        let global = Rc::clone(&self.global);

        for child in &program.children {
            if let ASTNodeOrToken::Node(stmt_line) = child {
                if let Some(stmt) = first_node(stmt_line) {
                    self.visible.set(true);
                    last = self.eval_node(stmt, &global)?;
                }
            }
        }

        // Auto-print a visible top-level result through the S3 `print` generic,
        // so factors, data frames, and user-classed values render via their own
        // methods (R does the same at the prompt).
        let visible = self.visible.get();
        if visible {
            self.dispatch_print(&last)?;
        }

        Ok(Outcome {
            value: last,
            visible,
            printed: self.out.borrow().clone(),
        })
    }

    // -----------------------------------------------------------------------
    // The node dispatcher
    // -----------------------------------------------------------------------

    fn eval_node(&self, node: &GrammarASTNode, env: &Env) -> SResult<SValue> {
        // Bound recursion so deeply nested input cannot overflow the stack.
        self.depth.set(self.depth.get() + 1);
        let _guard = DepthGuard(&self.depth);
        if self.depth.get() > MAX_EVAL_DEPTH {
            return Err(SError::Parse(
                "evaluation nested too deeply (possible infinite recursion)".into(),
            ));
        }

        match node.rule_name.as_str() {
            // Pure pass-through wrappers: delegate to the single child node and
            // preserve its visibility.
            "statement" | "expr" => self.eval_node(only_node(node)?, env),

            "statement_line" => match first_node(node) {
                Some(inner) => self.eval_node(inner, env),
                None => Ok(SValue::Null),
            },

            "assignment" => self.eval_assignment(node, env),
            "comparison" => self.eval_comparison(node, env),
            "range" => self.eval_range(node, env),
            "additive" | "multiplicative" => self.eval_arith_chain(node, env),
            "special" => self.eval_special(node, env),
            "pipe" => self.eval_pipe(node, env),
            "unary" => self.eval_unary(node, env),
            "power" => self.eval_power(node, env),
            "postfix" => self.eval_postfix(node, env),
            "primary" => self.eval_primary(node, env),
            "group" => {
                let v = self.eval_node(only_node(node)?, env)?;
                self.as_visible(v)
            }
            "block" => self.eval_block(node, env),
            "func_def" => self.eval_func_def(node, env),
            "if_expr" => self.eval_if(node, env),
            "for_expr" => self.eval_for(node, env),
            "while_expr" => self.eval_while(node, env),
            "repeat_expr" => self.eval_repeat(node, env),

            other => Err(SError::Parse(format!("cannot evaluate rule '{other}'"))),
        }
    }

    // -----------------------------------------------------------------------
    // Assignment
    // -----------------------------------------------------------------------

    fn eval_assignment(&self, node: &GrammarASTNode, env: &Env) -> SResult<SValue> {
        let nodes = node_children(node);
        // One child → pass through to the comparison level.
        if nodes.len() == 1 {
            return self.eval_node(nodes[0], env);
        }
        // Three children: <lhs> <op-token> <rhs>.
        let op = op_token(node).ok_or_else(|| SError::Parse("malformed assignment".into()))?;
        let (target_node, value_node) = if op == "->" || op == "->>" {
            (nodes[1], nodes[0]) // value -> target
        } else {
            (nodes[0], nodes[1]) // target <- value
        };
        let value = self.eval_node(value_node, env)?;
        // A bare-name target is the simple case; otherwise try `x[...] <- v`
        // sub-assignment (R-14) before giving up.
        match lvalue_name(target_node) {
            Ok(name) => {
                define(env, &name, value.clone());
                self.as_invisible(value)
            }
            Err(simple_err) => self.eval_indexed_assignment(target_node, value, env, simple_err),
        }
    }

    /// Handle `x[i] <- v` / `m[i, j] <- v` — evaluate the base variable, resolve
    /// the subscripts, write the recycled RHS into the selected cells of a
    /// *clone*, and rebind the base name (so other bindings can't be corrupted).
    fn eval_indexed_assignment(
        &self,
        target: &GrammarASTNode,
        rhs: SValue,
        env: &Env,
        simple_err: SError,
    ) -> SResult<SValue> {
        // The target must reduce to a `postfix` of the form `NAME [ subscripts ]`
        // (subscript assignment) or `fn ( x )` (a replacement-function call such
        // as `names(x) <- value`).
        let postfix = descend_to_postfix(target).ok_or(simple_err)?;
        let parts = node_children(postfix);
        let (primary, suffix) = match parts.as_slice() {
            [primary, suffix]
                if suffix.rule_name == "index_suffix" || suffix.rule_name == "call_suffix" =>
            {
                (*primary, *suffix)
            }
            _ => return Err(SError::TypeError(
                "unsupported assignment target (only `x[...] <- v` and `f(x) <- v` are supported)"
                    .into(),
            )),
        };

        // `f(x) <- value` — a replacement-function call. R desugars this to
        // `x <- \`f<-\`(x, value)`: the inner argument must be a bare-name
        // variable, and the replacement function (`names<-`, …) is looked up,
        // called with `(current_x, value)`, and its result rebound to `x`.
        if suffix.rule_name == "call_suffix" {
            return self.eval_replacement_assignment(primary, suffix, rhs, env);
        }

        let base_name = lvalue_name(primary)?;
        let current =
            lookup(env, &base_name).ok_or_else(|| SError::Undefined(base_name.clone()))?;

        let subs = self.eval_subscripts(suffix, env)?;
        let updated = match subs.len() {
            1 => assign_index(&current, subs[0].as_ref(), &rhs)?,
            2 => assign_index2d(&current, subs[0].as_ref(), subs[1].as_ref(), &rhs)?,
            n => {
                return Err(SError::Index(format!(
                    "incorrect number of dimensions ({n}) in assignment"
                )))
            }
        };
        define(env, &base_name, updated);
        self.as_invisible(rhs)
    }

    /// Handle a **replacement-function** assignment `f(x) <- value` (R-15).
    /// R defines this as sugar for `x <- \`f<-\`(x, value)`: the call's single
    /// argument `x` must be a bare-name variable; we look up the replacement
    /// function `\`f<-\``, call it with the *current* value of `x` and the RHS
    /// `value` (passed as the named `value =` argument), and rebind the result to
    /// `x`. Used for `names(x) <- …`; the same machinery serves any future
    /// `levels<-` / `dim<-` once those replacement builtins are registered.
    fn eval_replacement_assignment(
        &self,
        primary: &GrammarASTNode,
        call_suffix: &GrammarASTNode,
        rhs: SValue,
        env: &Env,
    ) -> SResult<SValue> {
        // The replacement-function base name (e.g. `names`).
        let fn_name = lvalue_name(primary)
            .map_err(|_| SError::TypeError("invalid replacement-function target".into()))?;

        // The call must have exactly one argument, and it must be a bare-name
        // variable (the object being modified): `names(x) <- v`.
        let arg_values = self.eval_args(call_suffix, env)?;
        if arg_values.len() != 1 {
            return Err(SError::BadArgs(format!(
                "{fn_name}(...) <- value: the replacement target must take exactly one argument"
            )));
        }

        // Recover the bare-name of that single argument from the parse tree (we
        // need the *name* to rebind, not just its value).
        let arg_node = call_suffix
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "arg_list" => Some(n),
                _ => None,
            })
            .and_then(first_node) // the first `arg`
            .and_then(|arg| only_node(arg).ok())
            .ok_or_else(|| SError::Parse("malformed replacement target".into()))?;
        let target_name = lvalue_name(arg_node).map_err(|_| {
            SError::TypeError(format!(
                "target of {fn_name}(...) <- value must be a variable"
            ))
        })?;
        let current =
            lookup(env, &target_name).ok_or_else(|| SError::Undefined(target_name.clone()))?;

        // Look up `\`f<-\`` and call it with (current, value = rhs).
        let replacement_name = format!("{fn_name}<-");
        let func = lookup(env, &replacement_name)
            .ok_or_else(|| SError::Undefined(replacement_name.clone()))?;
        let call_args = [
            Arg {
                name: None,
                value: current,
            },
            Arg {
                name: Some("value".to_string()),
                value: rhs.clone(),
            },
        ];
        let updated = self.call_value(func, &call_args)?;
        define(env, &target_name, updated);
        self.as_invisible(rhs)
    }

    // -----------------------------------------------------------------------
    // Operators
    // -----------------------------------------------------------------------

    fn eval_comparison(&self, node: &GrammarASTNode, env: &Env) -> SResult<SValue> {
        let nodes = node_children(node);
        if nodes.len() == 1 {
            return self.eval_node(nodes[0], env);
        }
        let op = op_token(node).ok_or_else(|| SError::Parse("malformed comparison".into()))?;
        let lhs = self.eval_node(nodes[0], env)?;
        let rhs = self.eval_node(nodes[1], env)?;
        let v = compare(&op, &lhs, &rhs)?;
        self.as_visible(v)
    }

    fn eval_range(&self, node: &GrammarASTNode, env: &Env) -> SResult<SValue> {
        let nodes = node_children(node);
        if nodes.len() == 1 {
            return self.eval_node(nodes[0], env);
        }
        let from = scalar_f64(&self.eval_node(nodes[0], env)?)?;
        let to = scalar_f64(&self.eval_node(nodes[1], env)?)?;
        let vals = bounded_sequence(from, to)?;
        self.as_visible(SValue::doubles(vals))
    }

    /// Left-associative `+ - * /` folding.
    fn eval_arith_chain(&self, node: &GrammarASTNode, env: &Env) -> SResult<SValue> {
        let mut acc: Option<SValue> = None;
        let mut pending: Option<String> = None;
        let mut applied = false;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Node(n) => {
                    let v = self.eval_node(n, env)?;
                    acc = Some(match (acc.take(), pending.take()) {
                        (None, _) => v,
                        (Some(a), Some(op)) => {
                            applied = true;
                            arithmetic(&op, &a, &v)?
                        }
                        (Some(a), None) => a, // unreachable in a well-formed tree
                    });
                }
                ASTNodeOrToken::Token(t) => pending = Some(t.value.clone()),
            }
        }
        let value = acc.ok_or_else(|| SError::Parse("empty arithmetic chain".into()))?;
        if applied {
            self.as_visible(value)
        } else {
            Ok(value)
        }
    }

    /// Left-associative fold over the `%op%` infix operators.
    fn eval_special(&self, node: &GrammarASTNode, env: &Env) -> SResult<SValue> {
        let mut acc: Option<SValue> = None;
        let mut pending: Option<String> = None;
        let mut applied = false;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Node(n) => {
                    let v = self.eval_node(n, env)?;
                    acc = Some(match (acc.take(), pending.take()) {
                        (None, _) => v,
                        (Some(a), Some(op)) => {
                            applied = true;
                            self.eval_infix(&op, &a, &v, env)?
                        }
                        (Some(a), None) => a,
                    });
                }
                ASTNodeOrToken::Token(t) => pending = Some(t.value.clone()),
            }
        }
        let value = acc.ok_or_else(|| SError::Parse("empty %op% chain".into()))?;
        if applied {
            self.as_visible(value)
        } else {
            Ok(value)
        }
    }

    /// Evaluate one `%op%` application. `%%`, `%/%`, `%in%`, and `%o%` are
    /// built in; any other `%name%` is looked up as a user-defined function
    /// (defined via `"%name%" <- function(a, b) …`) and called `(lhs, rhs)`.
    fn eval_infix(&self, op: &str, lhs: &SValue, rhs: &SValue, env: &Env) -> SResult<SValue> {
        match op {
            "%%" => arithmetic("%%", lhs, rhs),
            "%/%" => arithmetic("%/%", lhs, rhs),
            "%in%" => Ok(membership(lhs, rhs)),
            "%o%" => self.outer_product(lhs, rhs),
            "%*%" => self.matrix_multiply(lhs, rhs),
            _ => {
                let func = lookup(env, op).ok_or_else(|| SError::Undefined(op.to_string()))?;
                let args = [
                    Arg {
                        name: None,
                        value: lhs.clone(),
                    },
                    Arg {
                        name: None,
                        value: rhs.clone(),
                    },
                ];
                self.call_value(func, &args)
            }
        }
    }

    /// `a %o% b` — the outer product: every product `a[i] * b[j]`, laid out in
    /// row-major order (`length(a) * length(b)` elements).
    fn outer_product(&self, lhs: &SValue, rhs: &SValue) -> SResult<SValue> {
        let a = lhs.as_double()?;
        let b = rhs.as_double()?;
        // Each length is individually capped, but the product is not — bound it
        // so `1:1e6 %o% 1:1e6` can't request a petabyte-scale allocation.
        let total = a
            .len()
            .checked_mul(b.len())
            .filter(|&n| n <= MAX_SEQ_LEN)
            .ok_or_else(|| {
                SError::Index(format!(
                    "outer product result too large (limit {MAX_SEQ_LEN} elements)"
                ))
            })?;
        let mut out = Vec::with_capacity(total);
        for x in a.iter() {
            for y in b.iter() {
                out.push(if is_na_real(x) || is_na_real(y) {
                    na_real()
                } else {
                    x * y
                });
            }
        }
        Ok(SValue::doubles(out))
    }

    /// `a %*% b` — the matrix product (column-major). A bare vector on the left
    /// is taken as a `1×n` row; on the right as an `n×1` column (so `v %*% w` is
    /// the dot product), matching R's conformability rules. NA propagates.
    fn matrix_multiply(&self, lhs: &SValue, rhs: &SValue) -> SResult<SValue> {
        let (ad, am, ak) = match lhs {
            SValue::Matrix { data, nrow, ncol } => (data.clone(), *nrow, *ncol),
            other => {
                let d = other.as_double()?;
                let n = d.len();
                (d, 1, n) // a left vector is a row
            }
        };
        let (bd, bk, bn) = match rhs {
            SValue::Matrix { data, nrow, ncol } => (data.clone(), *nrow, *ncol),
            other => {
                let d = other.as_double()?;
                let n = d.len();
                (d, n, 1) // a right vector is a column
            }
        };
        if ak != bk {
            return Err(SError::TypeError(format!(
                "non-conformable arguments: {am}x{ak} %*% {bk}x{bn}"
            )));
        }
        let total = am
            .checked_mul(bn)
            .filter(|&n| n <= MAX_SEQ_LEN)
            .ok_or_else(|| {
                SError::Index(format!(
                    "matrix product result too large (limit {MAX_SEQ_LEN} elements)"
                ))
            })?;
        let (a, b) = (ad.data(), bd.data());
        let mut out = vec![0.0; total];
        for c in 0..bn {
            for r in 0..am {
                let mut acc = 0.0;
                let mut na = false;
                for p in 0..ak {
                    let (x, y) = (a[p * am + r], b[c * bk + p]); // column-major
                    if is_na_real(x) || is_na_real(y) {
                        na = true;
                        break;
                    }
                    acc += x * y;
                }
                out[c * am + r] = if na { na_real() } else { acc };
            }
        }
        Ok(SValue::Matrix {
            data: Double::from_values(out),
            nrow: am,
            ncol: bn,
        })
    }

    /// The native pipe `|>`. `x |> f(a)` desugars to `f(x, a)`: the left value
    /// is inserted as the first positional argument of the right-hand call. The
    /// repetition is left-associative and flat, so `x |> f() |> g()` evaluates as
    /// `g(f(x))`. The right-hand side of each `|>` must be a function call — a
    /// bare `x |> f` is an error, exactly as in R.
    fn eval_pipe(&self, node: &GrammarASTNode, env: &Env) -> SResult<SValue> {
        let stages = node_children(node);
        // No `|>` present: a plain pass-through to the single operand.
        if stages.len() == 1 {
            return self.eval_node(stages[0], env);
        }
        let mut acc = self.eval_node(stages[0], env)?;
        for rhs in &stages[1..] {
            // Descend the right operand to its postfix call and split it into the
            // callee and the existing arguments.
            let call = descend_to_postfix(rhs).ok_or_else(pipe_needs_call)?;
            let callee_node = first_node(call).ok_or_else(pipe_needs_call)?;
            let call_suffix = call
                .children
                .iter()
                .find_map(|c| match c {
                    ASTNodeOrToken::Node(n) if n.rule_name == "call_suffix" => Some(n),
                    _ => None,
                })
                .ok_or_else(pipe_needs_call)?;

            let callee = self.eval_node(callee_node, env)?;
            let mut args = self.eval_args(call_suffix, env)?;
            // Insert the piped value as the first positional argument.
            args.insert(
                0,
                Arg {
                    name: None,
                    value: acc,
                },
            );
            acc = self.call_value(callee, &args)?;
        }
        self.as_visible(acc)
    }

    fn eval_unary(&self, node: &GrammarASTNode, env: &Env) -> SResult<SValue> {
        // Either `MINUS unary` or a pass-through to `power`.
        if op_token(node).as_deref() == Some("-") {
            let inner = self.eval_node(only_node(node)?, env)?;
            let v = negate(&inner)?;
            return self.as_visible(v);
        }
        self.eval_node(only_node(node)?, env)
    }

    fn eval_power(&self, node: &GrammarASTNode, env: &Env) -> SResult<SValue> {
        let nodes = node_children(node);
        if nodes.len() == 1 {
            return self.eval_node(nodes[0], env);
        }
        let base = self.eval_node(nodes[0], env)?;
        let exp = self.eval_node(nodes[1], env)?;
        let v = arithmetic("^", &base, &exp)?;
        self.as_visible(v)
    }

    // -----------------------------------------------------------------------
    // Postfix: calls and indexing
    // -----------------------------------------------------------------------

    fn eval_postfix(&self, node: &GrammarASTNode, env: &Env) -> SResult<SValue> {
        let mut iter = node.children.iter();
        // The first child is the primary; remaining children are suffixes.
        let primary = match iter.next() {
            Some(ASTNodeOrToken::Node(n)) => n,
            _ => return Err(SError::Parse("malformed postfix".into())),
        };
        let mut value = self.eval_node(primary, env)?;
        let mut had_suffix = false;

        for child in iter {
            let suffix = match child {
                ASTNodeOrToken::Node(n) => n,
                ASTNodeOrToken::Token(_) => continue,
            };
            had_suffix = true;
            match suffix.rule_name.as_str() {
                "call_suffix" => {
                    let args = self.eval_args(suffix, env)?;
                    value = self.apply(value, &args, env)?;
                }
                "index_suffix" => {
                    // Each comma-separated position is optional (`m[i, ]`,
                    // `m[, j]`); `None` means "all of that dimension".
                    let subs = self.eval_subscripts(suffix, env)?;
                    value = match subs.len() {
                        // `x[i]` (or `x[]` → the whole object unchanged).
                        1 => match &subs[0] {
                            Some(i) => index(&value, i)?,
                            None => value,
                        },
                        // `m[rows, cols]` / `df[rows, cols]` — 2-D subsetting,
                        // with empty subscripts selecting a whole dimension.
                        2 => index2d(&value, subs[0].as_ref(), subs[1].as_ref())?,
                        _ => {
                            return Err(SError::Index(format!(
                                "incorrect number of dimensions ({})",
                                subs.len()
                            )))
                        }
                    };
                    self.visible.set(true);
                }
                "dindex_suffix" => {
                    // `x[[ key ]]` — single-column / single-element extraction.
                    let key = self.eval_node(only_node(suffix)?, env)?;
                    value = crate::dataframe::extract(&value, &key)?;
                    self.visible.set(true);
                }
                "dollar_suffix" => {
                    // `df$name` — column by name.
                    let name = name_token(suffix)
                        .ok_or_else(|| SError::Parse("malformed $ access".into()))?;
                    value = crate::dataframe::column_by_name(&value, &name)?;
                    self.visible.set(true);
                }
                other => return Err(SError::Parse(format!("unexpected suffix '{other}'"))),
            }
        }

        if had_suffix {
            Ok(value)
        } else {
            // Pure pass-through to the primary; its visibility already stands.
            Ok(value)
        }
    }

    /// Evaluate the arguments of a `call_suffix` / `index_suffix`.
    fn eval_args(&self, suffix: &GrammarASTNode, env: &Env) -> SResult<Vec<Arg>> {
        let mut args = Vec::new();
        // Find the optional arg_list node inside the suffix.
        let arg_list = suffix.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "arg_list" => Some(n),
            _ => None,
        });
        let Some(arg_list) = arg_list else {
            return Ok(args);
        };
        for child in &arg_list.children {
            if let ASTNodeOrToken::Node(arg) = child {
                if arg.rule_name == "arg" {
                    args.push(self.eval_arg(arg, env)?);
                }
            }
        }
        Ok(args)
    }

    /// Evaluate the comma-separated subscripts of an `index_suffix` into one
    /// slot per position. A position with no `subscript` node (an empty
    /// subscript like the row part of `m[, j]`) yields `None`, meaning "select
    /// the whole dimension". The number of positions is (number of commas + 1).
    fn eval_subscripts(&self, suffix: &GrammarASTNode, env: &Env) -> SResult<Vec<Option<SValue>>> {
        let mut slots: Vec<Option<SValue>> = Vec::new();
        let mut current: Option<SValue> = None;
        for child in &suffix.children {
            match child {
                ASTNodeOrToken::Node(n) if n.rule_name == "subscript" => {
                    current = Some(self.eval_node(only_node(n)?, env)?);
                }
                ASTNodeOrToken::Token(t) if t.value == "," => {
                    slots.push(current.take());
                }
                // Skip the surrounding `[` / `]` tokens.
                _ => {}
            }
        }
        slots.push(current.take());
        Ok(slots)
    }

    fn eval_arg(&self, arg: &GrammarASTNode, env: &Env) -> SResult<Arg> {
        // A named argument is `NAME = expr`: its first two children are the NAME
        // token and the `=` token. A positional argument is just `expr`.
        let named = matches!(arg.children.first(), Some(ASTNodeOrToken::Token(_)))
            && matches!(arg.children.get(1), Some(ASTNodeOrToken::Token(t)) if t.value == "=");
        let name = if named {
            match arg.children.first() {
                Some(ASTNodeOrToken::Token(t)) => Some(t.value.clone()),
                _ => None,
            }
        } else {
            None
        };
        let value = self.eval_node(only_node(arg)?, env)?;
        Ok(Arg { name, value })
    }

    /// Apply a callable value at a top-level call site, applying S's result
    /// visibility rules (`print` is invisible and emits output; other calls are
    /// visible; a closure's body decides its own visibility).
    fn apply(&self, callee: SValue, args: &[Arg], _env: &Env) -> SResult<SValue> {
        match callee {
            SValue::Builtin { name, func } => {
                let result = func(self, args)?;
                // `print`/`cat` produce their own output, and `set.seed` mutates
                // RNG state — all three return invisibly (R's `invisible(NULL)`);
                // every other built-in yields a visible value.
                if name == "print" || name == "cat" || name == "set.seed" {
                    self.as_invisible(result)
                } else {
                    self.as_visible(result)
                }
            }
            SValue::Closure { params, body, env } => self.call_closure(&params, &body, &env, args),
            other => Err(SError::NotCallable(other.type_name().to_string())),
        }
    }

    /// Call a callable value and return its result, *without* the top-level
    /// visibility/printing side effects. This is the entry point built-ins use
    /// to invoke a user function (`sapply`, `lapply`) or an S3 method.
    pub(crate) fn call_value(&self, callee: SValue, args: &[Arg]) -> SResult<SValue> {
        match callee {
            SValue::Builtin { func, .. } => func(self, args),
            SValue::Closure { params, body, env } => self.call_closure(&params, &body, &env, args),
            other => Err(SError::NotCallable(other.type_name().to_string())),
        }
    }

    fn call_closure(
        &self,
        params: &[Param],
        body: &Rc<GrammarASTNode>,
        closure_env: &Env,
        args: &[Arg],
    ) -> SResult<SValue> {
        let scope = Scope::child(closure_env);
        let mut bound = vec![false; params.len()];

        // 1. Named arguments bind to the matching parameter by name.
        let mut positional: Vec<&Arg> = Vec::new();
        for arg in args {
            match &arg.name {
                Some(name) => {
                    let idx = params.iter().position(|p| &p.name == name).ok_or_else(|| {
                        SError::BadArgs(format!("unused argument ({name} = ...)"))
                    })?;
                    define(&scope, name, arg.value.clone());
                    bound[idx] = true;
                }
                None => positional.push(arg),
            }
        }

        // 2. Positional arguments fill the remaining parameters in order.
        let mut pos_iter = positional.into_iter();
        for (i, param) in params.iter().enumerate() {
            if bound[i] {
                continue;
            }
            if let Some(arg) = pos_iter.next() {
                define(&scope, &param.name, arg.value.clone());
                bound[i] = true;
            }
        }
        if pos_iter.next().is_some() {
            return Err(SError::BadArgs("unused arguments".into()));
        }

        // 3. Unbound parameters fall back to their default (evaluated in the
        //    new frame, so a default may reference earlier parameters).
        for (i, param) in params.iter().enumerate() {
            if bound[i] {
                continue;
            }
            if let Some(default) = &param.default {
                let v = self.eval_node(default, &scope)?;
                define(&scope, &param.name, v);
            }
        }

        // The body's own visibility stands (we do not override it here).
        self.eval_node(body, &scope)
    }

    // -----------------------------------------------------------------------
    // Primary atoms
    // -----------------------------------------------------------------------

    fn eval_primary(&self, node: &GrammarASTNode, env: &Env) -> SResult<SValue> {
        match node.children.first() {
            Some(ASTNodeOrToken::Node(inner)) => self.eval_node(inner, env),
            Some(ASTNodeOrToken::Token(tok)) => {
                let type_name = tok.effective_type_name();
                let value = tok.value.as_str();
                let v = match type_name {
                    "NUMBER" => {
                        SValue::scalar(value.parse::<f64>().map_err(|_| {
                            SError::Parse(format!("invalid number literal '{value}'"))
                        })?)
                    }
                    // R's typed numeric literals (R-4). This subset has no
                    // distinct integer type, so the integer/hex forms become
                    // doubles. `L` (integer) and `0x` (hex) suffixes are dropped.
                    "INT_LIT" => {
                        let digits = value.trim_end_matches('L');
                        SValue::scalar(digits.parse::<f64>().map_err(|_| {
                            SError::Parse(format!("invalid integer literal '{value}'"))
                        })?)
                    }
                    "HEX_LIT" => {
                        // Strip the `0x`/`0X` prefix and an optional `L` suffix.
                        let body = value[2..].trim_end_matches('L');
                        let n = u64::from_str_radix(body, 16)
                            .map_err(|_| SError::Parse(format!("invalid hex literal '{value}'")))?;
                        SValue::scalar(n as f64)
                    }
                    // Complex literals (`1i`) lex and parse, but this subset has
                    // no complex type yet — report it clearly rather than lying.
                    "COMPLEX_LIT" => {
                        return Err(SError::TypeError(format!(
                            "complex numbers are not yet supported (literal '{value}')"
                        )))
                    }
                    "STRING" => SValue::Character(vec![Some(strip_quotes(value))]),
                    "KEYWORD" => match value {
                        "TRUE" | "T" => SValue::Logical(vec![Some(true)]),
                        "FALSE" | "F" => SValue::Logical(vec![Some(false)]),
                        "NULL" => SValue::Null,
                        "NA" => SValue::Logical(vec![None]),
                        // R's typed-NA constants. We have no distinct integer
                        // type, so the numeric ones share the double NA.
                        "NA_integer_" | "NA_real_" => SValue::Double(Double::na(1)),
                        "NA_character_" => SValue::Character(vec![None]),
                        "Inf" => SValue::scalar(f64::INFINITY),
                        "NaN" => SValue::scalar(f64::NAN),
                        "break" => return Err(SError::Break),
                        "next" => return Err(SError::Next),
                        other => {
                            return Err(SError::Parse(format!("unexpected keyword '{other}'")))
                        }
                    },
                    "NAME" => {
                        lookup(env, value).ok_or_else(|| SError::Undefined(value.to_string()))?
                    }
                    other => return Err(SError::Parse(format!("unexpected token type '{other}'"))),
                };
                self.as_visible(v)
            }
            None => Err(SError::Parse("empty primary".into())),
        }
    }

    // -----------------------------------------------------------------------
    // Blocks and control flow
    // -----------------------------------------------------------------------

    fn eval_block(&self, node: &GrammarASTNode, env: &Env) -> SResult<SValue> {
        let mut last = SValue::Null;
        self.visible.set(false);
        for child in &node.children {
            if let ASTNodeOrToken::Node(stmt_line) = child {
                if stmt_line.rule_name == "statement_line" {
                    if let Some(stmt) = first_node(stmt_line) {
                        last = self.eval_node(stmt, env)?;
                    }
                }
            }
        }
        Ok(last)
    }

    fn eval_func_def(&self, node: &GrammarASTNode, env: &Env) -> SResult<SValue> {
        let mut params = Vec::new();
        let mut body = None;
        for child in &node.children {
            if let ASTNodeOrToken::Node(n) = child {
                match n.rule_name.as_str() {
                    "param_list" => params = parse_params(n),
                    _ => body = Some(n), // the body expression
                }
            }
        }
        let body = body.ok_or_else(|| SError::Parse("function has no body".into()))?;
        let closure = SValue::Closure {
            params,
            body: Rc::new(body.clone()),
            env: Rc::clone(env),
        };
        self.as_visible(closure)
    }

    fn eval_if(&self, node: &GrammarASTNode, env: &Env) -> SResult<SValue> {
        let nodes = node_children(node);
        // nodes: [cond, then, (else)?]
        let cond = self.eval_node(nodes[0], env)?;
        if cond.truthy()? {
            self.eval_node(nodes[1], env)
        } else if let Some(else_branch) = nodes.get(2) {
            self.eval_node(else_branch, env)
        } else {
            self.as_invisible(SValue::Null)
        }
    }

    fn eval_for(&self, node: &GrammarASTNode, env: &Env) -> SResult<SValue> {
        // Tokens give us the loop variable NAME; nodes give the sequence and body.
        let var = name_token(node).ok_or_else(|| SError::Parse("malformed for".into()))?;
        let nodes = node_children(node);
        let seq = self.eval_node(nodes[0], env)?;
        let body = nodes[1];
        let len = seq.length();
        for i in 0..len {
            define(env, &var, nth_element(&seq, i));
            match self.eval_node(body, env) {
                Ok(_) => {}
                Err(SError::Break) => break,
                Err(SError::Next) => continue,
                Err(e) => return Err(e),
            }
        }
        self.as_invisible(SValue::Null)
    }

    fn eval_while(&self, node: &GrammarASTNode, env: &Env) -> SResult<SValue> {
        let nodes = node_children(node);
        let cond = nodes[0];
        let body = nodes[1];
        while self.eval_node(cond, env)?.truthy()? {
            match self.eval_node(body, env) {
                Ok(_) => {}
                Err(SError::Break) => break,
                Err(SError::Next) => continue,
                Err(e) => return Err(e),
            }
        }
        self.as_invisible(SValue::Null)
    }

    fn eval_repeat(&self, node: &GrammarASTNode, env: &Env) -> SResult<SValue> {
        let body = only_node(node)?;
        loop {
            match self.eval_node(body, env) {
                Ok(_) => {}
                Err(SError::Break) => break,
                Err(SError::Next) => continue,
                Err(e) => return Err(e),
            }
        }
        self.as_invisible(SValue::Null)
    }

    // -----------------------------------------------------------------------
    // Visibility helpers
    // -----------------------------------------------------------------------

    fn as_visible(&self, v: SValue) -> SResult<SValue> {
        self.visible.set(true);
        Ok(v)
    }

    fn as_invisible(&self, v: SValue) -> SResult<SValue> {
        self.visible.set(false);
        Ok(v)
    }
}

// ===========================================================================
// Free helpers for navigating the generic parse tree
// ===========================================================================

/// The standard error when the right-hand side of `|>` is not a function call.
fn pipe_needs_call() -> SError {
    SError::Parse("the right-hand side of |> must be a function call".into())
}

/// Descend a single-operand expression chain (`range → unary → power → …`) to
/// the `postfix` node at its core, returning `None` if any level along the way
/// applies an operator (more than one child node) — i.e. the operand is not a
/// plain call. Used to find the call on the right of a `|>`.
fn descend_to_postfix(node: &GrammarASTNode) -> Option<&GrammarASTNode> {
    let mut cur = node;
    loop {
        if cur.rule_name == "postfix" {
            return Some(cur);
        }
        let kids = node_children(cur);
        match kids.as_slice() {
            [only] => cur = only,
            _ => return None,
        }
    }
}

/// All child nodes (ignoring raw tokens), in order.
fn node_children(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

/// The first child node, if any.
fn first_node(node: &GrammarASTNode) -> Option<&GrammarASTNode> {
    node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(n) => Some(n),
        ASTNodeOrToken::Token(_) => None,
    })
}

/// The single child node, erroring if the structure is unexpected.
fn only_node(node: &GrammarASTNode) -> SResult<&GrammarASTNode> {
    first_node(node).ok_or_else(|| SError::Parse(format!("malformed '{}' node", node.rule_name)))
}

/// The value of the first operator/keyword token that is not a delimiter.
fn op_token(node: &GrammarASTNode) -> Option<String> {
    node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Token(t) => Some(t.value.clone()),
        _ => None,
    })
}

/// The first `NAME` token value (used by `for (NAME in ...)`).
fn name_token(node: &GrammarASTNode) -> Option<String> {
    node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Token(t) if t.effective_type_name() == "NAME" => Some(t.value.clone()),
        _ => None,
    })
}

/// Extract a simple assignment target name from an lvalue subtree. v1 supports
/// only bare names (`x <- ...`); complex targets like `x[1] <- ...` are not yet
/// handled and produce an error.
fn lvalue_name(node: &GrammarASTNode) -> SResult<String> {
    // The target subtree must reduce to a single token: a bare NAME, or a
    // STRING (which names a function operator, e.g. `"%between%" <- ...`).
    let mut tokens: Vec<(&str, &str)> = Vec::new();
    collect_tokens(node, &mut tokens);
    if let [(ty, val)] = tokens.as_slice() {
        match *ty {
            "NAME" => return Ok((*val).to_string()),
            "STRING" => return Ok(strip_quotes(val)),
            _ => {}
        }
    }
    Err(SError::TypeError(
        "invalid (non-name) assignment target".into(),
    ))
}

/// Collect every token (effective type name, value) in a subtree, in order.
fn collect_tokens<'a>(node: &'a GrammarASTNode, out: &mut Vec<(&'a str, &'a str)>) {
    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(t) => out.push((t.effective_type_name(), t.value.as_str())),
            ASTNodeOrToken::Node(n) => collect_tokens(n, out),
        }
    }
}

/// Build the parameter list of a function definition.
fn parse_params(param_list: &GrammarASTNode) -> Vec<Param> {
    let mut params = Vec::new();
    for child in &param_list.children {
        if let ASTNodeOrToken::Node(p) = child {
            if p.rule_name != "param" {
                continue;
            }
            let name = name_token(p).unwrap_or_default();
            let default = first_node(p).map(|d| Rc::new(d.clone()));
            params.push(Param { name, default });
        }
    }
    params
}

/// Strip the surrounding quote characters from a string literal token value.
fn strip_quotes(raw: &str) -> String {
    let bytes = raw.as_bytes();
    if raw.len() >= 2 && (bytes[0] == b'"' || bytes[0] == b'\'') {
        raw[1..raw.len() - 1].to_string()
    } else {
        raw.to_string()
    }
}

/// The first element of a value as an `f64` (for the `:` sequence bounds).
fn scalar_f64(value: &SValue) -> SResult<f64> {
    let d = value.as_double()?;
    d.get_value(0)
        .filter(|x| !is_na_real(*x))
        .ok_or_else(|| SError::TypeError("argument of length zero or NA".into()))
}

/// Extract element `i` of a vector as a fresh length-1 value.
pub(crate) fn nth_element(value: &SValue, i: usize) -> SValue {
    match value {
        SValue::Double(d) => SValue::Double(Double::from_values(vec![d
            .get_value(i)
            .unwrap_or_else(na_real)])),
        SValue::Logical(v) => SValue::Logical(vec![v.get(i).copied().flatten()]),
        SValue::Character(v) => SValue::Character(vec![v.get(i).cloned().flatten()]),
        SValue::Factor { codes, levels } => SValue::Factor {
            codes: vec![codes.get(i).copied().flatten()],
            levels: levels.clone(),
        },
        SValue::Classed { inner, .. } => nth_element(inner, i),
        SValue::Named { values, .. } => nth_element(values, i),
        SValue::List { items, .. } => items.get(i).cloned().unwrap_or(SValue::Null),
        _ => SValue::Null,
    }
}

/// Convenience: evaluate `src` in a fresh interpreter and return the value.
pub fn eval_s(src: &str) -> SResult<SValue> {
    Interpreter::new().eval_str(src).map(|o| o.value)
}
