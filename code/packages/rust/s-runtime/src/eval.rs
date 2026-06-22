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
use crate::env::{define, exists, lookup, names_in, remove, same_env, super_assign, Env, Scope};
use crate::error::{SError, SResult};
use crate::value::{
    arithmetic, assign_index, assign_index2d, bounded_sequence, class_of, compare, format_value,
    index, index2d, logical_not, membership, negate, Arg, Param, SValue, MAX_SEQ_LEN,
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
    /// The session's **empty** environment (R-23): a parentless, bindingless root
    /// owned for the whole session, exactly like `global`. It is what `emptyenv()`
    /// returns and what `environmentName` recognises as `"R_EmptyEnv"`. Kept as a
    /// distinct strong `Rc` (never the same allocation as `global`) so the two can
    /// be told apart by `Rc` pointer identity. See [`Scope::empty`].
    empty: Env,
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
    /// Warnings raised by `warning(...)` during the current program, in order.
    /// Bounded by [`MAX_WARNINGS`] so a tight `warning()` loop cannot grow this
    /// without limit. Each warning is also printed immediately (R defers them by
    /// default, but immediate printing is simpler and equally faithful for a
    /// REPL); the buffer exists so a future `warnings()` accessor can read them.
    warnings: RefCell<Vec<String>>,
    /// The number of first-class environments reified this session via
    /// `new.env()` / `environment()` (R-22), capped by [`MAX_ENVIRONMENTS`].
    ///
    /// **Why this counter exists (the Rc-cycle caveat).** Making `Scope::parent` a
    /// `Weak` (see [`crate::env`]) breaks any cycle that closes *through the parent
    /// link*. It does **not** break a cycle that closes *through a value binding* —
    /// an environment value is a strong `Rc`, so `assign("self", e, envir = e)`
    /// (or a mutual `a`/`b` pair) stores a strong `Rc` to a scope inside that very
    /// scope's `vars`, an uncollectable cycle that `Rc` alone cannot reclaim
    /// (R relies on a tracing GC here; we have only `Rc`). Such a cycle leaks the
    /// bindings it retains once it becomes unreachable. We cannot cheaply collect
    /// it, but we *bound* the damage: this counter caps how many environments a
    /// single session can reify, so a crafted `for (i in 1:1e9) { e <- new.env();
    /// assign("self", e, envir = e) }` cannot drive unbounded heap growth — it
    /// hits a clean error at the cap instead. The cap is far beyond any realistic
    /// program's environment count.
    envs_created: Cell<usize>,
    /// The stack of call frames currently being evaluated, innermost last
    /// (R-20 / R-23). Each [`CallFrame`] records both the **closure** being run
    /// (so `Recall()` can re-invoke the enclosing function for anonymous
    /// recursion) and the **caller's environment** — the scope in which the call
    /// expression was evaluated — so `parent.frame()` (R-23) can reflect the
    /// caller's frame. `call_closure` pushes a frame before running the body and
    /// pops it on the way out via an RAII guard, so an early-return/error still
    /// pops and the top is always "the call we are inside right now". Its depth is
    /// naturally bounded by [`MAX_EVAL_DEPTH`] — every nested call increments the
    /// eval depth — so it cannot grow without limit, and because the caller env is
    /// dropped when its frame is popped it never outlives the call (no extra
    /// Rc-lifetime exposure beyond the live call).
    call_stack: RefCell<Vec<CallFrame>>,
}

/// One entry on the interpreter's call stack (R-23). Pairs the closure being run
/// (for `Recall`) with the environment of its **caller** (for `parent.frame()`).
#[derive(Clone)]
struct CallFrame {
    /// The closure currently executing in this frame (reconstructed cheaply from
    /// its `Rc` parts at the call site). Read by `Recall()`.
    closure: SValue,
    /// The environment in which the call expression was evaluated — i.e. the
    /// *caller's* current scope. Read by `parent.frame()`.
    caller: Env,
}

/// The most warnings retained per program. Beyond this, further `warning()`
/// calls still print but are not appended to the buffer — a bound against a
/// crafted `for (i in 1:1e9) warning("x")` exhausting memory.
const MAX_WARNINGS: usize = 10_000;

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

/// The most first-class environments a single session may reify via `new.env()` /
/// `environment()` (R-22). Because an environment value is a strong `Rc` that can
/// be stored inside the very scope it points at (`assign("self", e, envir = e)`),
/// a reference cycle through *value bindings* is constructible from user source —
/// the `Weak` parent link cannot break it (it only breaks parent-link cycles), and
/// `Rc` cannot collect it. This cap bounds the resulting leak: a crafted loop
/// building self-/mutually-referential environments hits a clean error here rather
/// than exhausting memory. 1,048,576 is far beyond any realistic program.
const MAX_ENVIRONMENTS: usize = 1 << 20;

/// RAII guard that decrements the interpreter's depth counter on scope exit,
/// so every early `return`/`?` in `eval_node` is accounted for.
struct DepthGuard<'a>(&'a Cell<usize>);

impl Drop for DepthGuard<'_> {
    fn drop(&mut self) {
        self.0.set(self.0.get().saturating_sub(1));
    }
}

/// RAII guard that pushes a [`CallFrame`] onto the interpreter's call stack
/// (R-20 / R-23) on construction and pops it on drop, so the stack stays balanced
/// even when the closure body returns early via `?` or raises an error.
/// `Recall()` reads the top frame's closure; `parent.frame()` reads its caller
/// env. Because the frame (and the `Rc` to the caller env it holds) is dropped
/// here, the caller env never outlives the call.
struct CallFrameGuard<'a>(&'a RefCell<Vec<CallFrame>>);

impl<'a> CallFrameGuard<'a> {
    fn push(stack: &'a RefCell<Vec<CallFrame>>, frame: CallFrame) -> Self {
        stack.borrow_mut().push(frame);
        CallFrameGuard(stack)
    }
}

impl Drop for CallFrameGuard<'_> {
    fn drop(&mut self) {
        self.0.borrow_mut().pop();
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
            // A distinct parentless root — the `emptyenv()` terminus (R-23).
            empty: Scope::empty(),
            out: RefCell::new(String::new()),
            visible: Cell::new(false),
            depth: Cell::new(0),
            rng: RefCell::new(RngState::new(DEFAULT_SEED)),
            warnings: RefCell::new(Vec::new()),
            envs_created: Cell::new(0),
            call_stack: RefCell::new(Vec::new()),
        }
    }

    /// Account for one newly reified first-class environment (R-22), failing
    /// closed at [`MAX_ENVIRONMENTS`] so a crafted loop building cyclic
    /// environments cannot grow the heap without bound. See `envs_created`.
    fn account_environment(&self) -> SResult<()> {
        let n = self.envs_created.get();
        if n >= MAX_ENVIRONMENTS {
            return Err(SError::BadArgs(format!(
                "too many environments created (limit {MAX_ENVIRONMENTS})"
            )));
        }
        self.envs_created.set(n + 1);
        Ok(())
    }

    /// The function currently being evaluated, if any — the closure of the top
    /// call frame. `Recall()` (R-20) uses this to re-invoke the enclosing closure.
    pub(crate) fn current_function(&self) -> Option<SValue> {
        self.call_stack
            .borrow()
            .last()
            .map(|frame| frame.closure.clone())
    }

    /// The caller's environment `n` frames up the call stack (R-23,
    /// `parent.frame(n)`). `n == 1` is the immediate caller (the top frame's
    /// recorded caller env); larger `n` walks further out. **Clamps** rather than
    /// indexes: an `n` that reaches past the bottom of the live stack (or
    /// `parent.frame()` evaluated at top level, where there is no enclosing call)
    /// falls back to the **global** environment, matching R's `R_GlobalEnv`
    /// terminus — so this can never index out of bounds or panic. `n` is a 1-based
    /// frame count; the caller has already validated `n >= 1`.
    pub(crate) fn caller_frame(&self, n: usize) -> Env {
        let stack = self.call_stack.borrow();
        // The top frame (last) is `parent.frame(1)`; the k-th from the top is
        // `parent.frame(k)`. `n.checked_sub(1)` and `len.checked_sub(n)` keep the
        // arithmetic panic-free; any underflow (n past the bottom) clamps to
        // global below.
        match stack.len().checked_sub(n) {
            Some(idx) => stack[idx].caller.clone(),
            None => Rc::clone(&self.global),
        }
    }

    /// Record and print a warning raised by `warning(...)`. The buffer is capped
    /// at [`MAX_WARNINGS`]; the message is always printed (so the user sees it)
    /// even once the buffer is full.
    pub(crate) fn warn(&self, message: &str) {
        {
            let mut w = self.warnings.borrow_mut();
            if w.len() < MAX_WARNINGS {
                w.push(message.to_string());
            }
        }
        self.emit_raw(&format!("Warning message:\n{message}\n"));
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
        self.warnings.borrow_mut().clear();

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
        // `<<-` / `->>` are *super-assignment* (R-21): rebind the nearest
        // ENCLOSING binding of the name, or create one in the global environment
        // if none exists. They only ever target a bare name (R does not define
        // `x[i] <<- v` sub-assignment in this subset).
        let is_super = op == "<<-" || op == "->>";
        // A bare-name target is the simple case; otherwise try `x[...] <- v`
        // sub-assignment (R-14) before giving up.
        match lvalue_name(target_node) {
            Ok(name) => {
                if is_super {
                    super_assign(env, &name, value.clone());
                } else {
                    define(env, &name, value.clone());
                }
                self.as_invisible(value)
            }
            Err(simple_err) => {
                if is_super {
                    // `<<-` with a non-name target (e.g. `x[1] <<- v`) is not
                    // supported in this subset — fail cleanly rather than silently
                    // falling through to ordinary (current-scope) sub-assignment.
                    return Err(SError::TypeError(
                        "super-assignment (`<<-`/`->>`) requires a bare-name target".into(),
                    ));
                }
                self.eval_indexed_assignment(target_node, value, env, simple_err)
            }
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
                if suffix.rule_name == "index_suffix"
                    || suffix.rule_name == "call_suffix"
                    || suffix.rule_name == "dollar_suffix" =>
            {
                (*primary, *suffix)
            }
            _ => return Err(SError::TypeError(
                "unsupported assignment target (only `x[...] <- v`, `x$name <- v` and `f(x) <- v` are supported)"
                    .into(),
            )),
        };

        // `obj$name <- value` (R-24). When `obj` evaluates to an environment (an
        // R5 instance, or any first-class environment), this is a **mutate-by-
        // reference** write: we bind `name` *in place* in that live scope, so two
        // references to the same instance both see the change. This is the
        // headline R5 reference semantics, and it falls straight out of reusing
        // R-22's shared `SValue::Environment`.
        if suffix.rule_name == "dollar_suffix" {
            return self.eval_dollar_assignment(primary, suffix, rhs, env);
        }

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

    /// Handle `obj$name <- value` (R-24). The `primary` expression is evaluated to
    /// the target; when it is an [`SValue::Environment`] (an R5 instance, or any
    /// first-class environment) the write is a **mutate-by-reference** `env::define`
    /// **in place** — the live scope gains/updates the `name` binding, so every
    /// reference to that same instance (`b <- a`) observes the change. This is the
    /// defining R5 reference semantic.
    ///
    /// Crucially `primary` is *evaluated* (not required to be a bare name): both
    /// `a$total <- v` (where `a` is a variable) and `.self$total <- v` (inside a
    /// method, where `.self` is the instance) resolve the same way — they each
    /// yield the shared `Rc` to the instance scope, and the `define` mutates it.
    /// A non-environment `obj` is a clean error (R5 `$<-` is only meaningful on a
    /// reference object / environment in this subset).
    fn eval_dollar_assignment(
        &self,
        primary: &GrammarASTNode,
        dollar_suffix: &GrammarASTNode,
        rhs: SValue,
        env: &Env,
    ) -> SResult<SValue> {
        let field = name_token(dollar_suffix)
            .ok_or_else(|| SError::Parse("malformed `$` assignment target".into()))?;
        let target = self.eval_node(primary, env)?;
        match target {
            SValue::Environment(e) => {
                // R-26 active binding: `obj$ab <- val` **calls** the binding function
                // as a setter with `v = val` (so `missing(v)` is FALSE inside it),
                // rather than overwriting the binding. Invoked through the ordinary
                // depth-bounded call path (same re-entrancy / borrow protection as the
                // getter). The assignment expression's value is the RHS (invisible).
                if let Some(setter) = crate::refclass::active_binding_fn(&e, &field) {
                    // Passed **positionally** so it binds the setter's single formal
                    // whatever it is named (`function(v)`, `function(value)`, …); that
                    // formal then tests FALSE under `missing()`, distinguishing the
                    // setter direction from the nullary getter.
                    let arg = [Arg {
                        name: None,
                        value: rhs.clone(),
                    }];
                    self.call_value(setter, &arg)?;
                    return self.as_invisible(rhs);
                }
                // In-place, by-reference field write. `env::define` takes and
                // releases the scope's `RefCell` borrow within the call, so a
                // method mutating a field mid-call never holds two borrows of the
                // same scope at once (no re-entrant-borrow panic).
                define(&e, &field, rhs.clone());
                self.as_invisible(rhs)
            }
            other => Err(SError::TypeError(format!(
                "$<- is invalid for {} (only environments / reference objects support `obj$name <- v`)",
                other.type_name()
            ))),
        }
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

        // The call must have at least one argument; the *first* is the bare-name
        // variable being modified (`names(x) <- v`, `attr(x, "foo") <- v`). Any
        // further arguments (`attr`'s `which`, etc.) are passed through to the
        // replacement function ahead of `value`, matching R's desugaring
        // `x <- \`f<-\`(x, <extra args>, value = v)`.
        let arg_values = self.eval_args(call_suffix, env)?;
        if arg_values.is_empty() {
            return Err(SError::BadArgs(format!(
                "{fn_name}(...) <- value: the replacement target must take at least one argument"
            )));
        }

        // Recover the bare-name of the *first* argument from the parse tree (we
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

        // Look up `\`f<-\`` and call it with (current, <extra args…>, value = rhs):
        // the first call arg is replaced by the variable's current value, the
        // remaining call args (e.g. `which`) pass through unchanged, and the RHS
        // is appended as the named `value` argument.
        let replacement_name = format!("{fn_name}<-");
        let func = lookup(env, &replacement_name)
            .ok_or_else(|| SError::Undefined(replacement_name.clone()))?;
        let mut call_args: Vec<Arg> = Vec::with_capacity(arg_values.len() + 1);
        call_args.push(Arg {
            name: None,
            value: current,
        });
        call_args.extend(arg_values.into_iter().skip(1));
        call_args.push(Arg {
            name: Some("value".to_string()),
            value: rhs.clone(),
        });
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
    ///
    /// Exposed `pub(crate)` so the `crossprod`/`tcrossprod` builtins (R-36) can
    /// reuse the *same* product — including its allocation guard (`MAX_SEQ_LEN`),
    /// its conformability error, and its `array_runtime` fast path — instead of
    /// reimplementing the inner loops.
    pub(crate) fn matrix_multiply(&self, lhs: &SValue, rhs: &SValue) -> SResult<SValue> {
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

        // MXF-4 — the shared f64 substrate.
        //
        // The clean win is the **NA-free** case: R's `Matrix` is already a
        // column-major `[nrow, ncol]` block of `f64`, which is exactly
        // `array_runtime::Array`'s layout, so we hand the two operands to
        // `array_runtime::execute(MatMul, …)`. That lowers to a `DType::F64`
        // `matrix-ir` graph and runs on the *cost-selected* backend (CPU today,
        // a GPU once one advertises `f64`), at full double precision (MXF-3's
        // 8-byte codec). The result is **bit-identical** to the loop below.
        //
        // Why not always? R's NA is a *specific* NaN bit pattern
        // (`r_vector::NA_REAL_BITS`). IEEE floating multiply/add on a NaN yields
        // an *implementation-defined* NaN payload, so an NA pushed through the
        // substrate would not reliably come back as R's NA — it would silently
        // become a plain `NaN`. So when either operand carries an NA we keep the
        // hand-written loop, which short-circuits any dotted column to
        // `na_real()` exactly as before. (Empty inner dim `ak == 0` is also kept
        // on the loop: that is a degenerate "sum of no terms" = 0 matrix, which
        // the loop already produces and `execute` does not model.)
        let has_na = a.iter().chain(b.iter()).any(|&x| is_na_real(x));
        if !has_na && ak > 0 {
            // Build column-major Arrays directly from R's column-major storage —
            // no transpose, no semantic copy. `from_shape` validates that the
            // data length matches `nrow*ncol`, which holds by construction here.
            if let (Ok(am_arr), Ok(bm_arr)) = (
                array_runtime::Array::from_shape(a.to_vec(), vec![am, ak]),
                array_runtime::Array::from_shape(b.to_vec(), vec![bk, bn]),
            ) {
                if let Ok(prod) =
                    array_runtime::execute(array_runtime::Kernel::MatMul, &am_arr, &bm_arr)
                {
                    return Ok(SValue::Matrix {
                        data: Double::from_values(prod.data().to_vec()),
                        nrow: am,
                        ncol: bn,
                    });
                }
                // Any substrate error (e.g. a future size cap) falls through to
                // the loop, which already bounded `total` by `MAX_SEQ_LEN` above.
            }
        }

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

        // --- Special forms: `switch` / `tryCatch` -----------------------------
        //
        // These cannot be ordinary (eager) builtins: they must inspect their
        // *unevaluated* argument expressions and evaluate only the selected arm /
        // protected expression / chosen handler. We intercept here, at the call
        // site, when the postfix is exactly `<bare-name> ( args )` — a single
        // `call_suffix` directly on a bare-name primary — and the name is one of
        // the special forms. (A name shadowed by a user variable is rare and R
        // treats these as language constructs; intercepting by name keeps the
        // laziness guarantee unconditional and simple.) Any other shape (extra
        // suffixes, indexing) falls through to the ordinary eager path below.
        if let Some(special) = special_form_name(primary) {
            // The only suffix nodes (ignoring `(`/`)` tokens) must be a lone
            // `call_suffix` — `switch(...)` / `tryCatch(...)`.
            let suffix_nodes: Vec<&GrammarASTNode> = node.children[1..]
                .iter()
                .filter_map(|c| match c {
                    ASTNodeOrToken::Node(n) => Some(n),
                    ASTNodeOrToken::Token(_) => None,
                })
                .collect();
            if let [suffix] = suffix_nodes.as_slice() {
                if suffix.rule_name == "call_suffix" {
                    let raw = raw_args(suffix);
                    return match special {
                        "switch" => self.eval_switch(&raw, env),
                        "tryCatch" => self.eval_try_catch(&raw, env),
                        // R-21 environment forms.
                        "local" => self.eval_local(&raw, env),
                        "assign" => self.eval_assign_fn(&raw, env),
                        "get" => self.eval_get_fn(&raw, env),
                        "exists" => self.eval_exists_fn(&raw, env),
                        "rm" => self.eval_rm_fn(&raw, env),
                        // R-22 first-class environments.
                        "new.env" => self.eval_new_env(&raw, env),
                        "environment" => self.eval_environment(&raw, env),
                        "ls" => self.eval_ls(&raw, env),
                        // R-23 closure environments & frame reflection.
                        "environmentName" => self.eval_environment_name(&raw, env),
                        "globalenv" | "baseenv" => self.eval_globalenv(),
                        "emptyenv" => self.eval_emptyenv(),
                        "parent.frame" => self.eval_parent_frame(&raw, env),
                        // R-24 R5 reference classes.
                        "setRefClass" => self.eval_set_ref_class(&raw, env),
                        // R-26 R5 method helpers.
                        "missing" => self.eval_missing(&raw, env),
                        "callSuper" => self.eval_call_super(&raw, env),
                        _ => unreachable!(),
                    };
                }
            }
        }

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
                    // `df$name` — column by name; `obj$field` / `obj$method` /
                    // `generator$new` for an R5 reference object (R-24).
                    let name = name_token(suffix)
                        .ok_or_else(|| SError::Parse("malformed $ access".into()))?;
                    value = self.dollar_read(&value, &name)?;
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
    fn apply(&self, callee: SValue, args: &[Arg], env: &Env) -> SResult<SValue> {
        // `generator$new(...)` (R-24): `generator$new` evaluated to an
        // instantiation marker; applying it builds a fresh instance. Handled
        // before the ordinary callable dispatch because the marker is not a
        // `Closure`/`Builtin`.
        if let Some(generator) = crate::refclass::as_new_marker(&callee) {
            return self.apply_ref_new(&generator, args);
        }
        // R-25 nullary reference methods: `obj$copy()`, `gen$fields()`,
        // `gen$methods()`. The marker is not a `Closure`/`Builtin`, so dispatch it
        // before the ordinary callable match.
        if let Some((action, target)) = crate::refclass::as_ref_method_marker(&callee) {
            return self.as_visible(self.apply_ref_method(action, &target)?);
        }
        match callee {
            SValue::Builtin { name, func } => {
                let result = func(self, args)?;
                // `print`/`cat` produce their own output, and `set.seed` mutates
                // RNG state — all three return invisibly (R's `invisible(NULL)`);
                // every other built-in yields a visible value.
                if name == "print" || name == "cat" || name == "set.seed" || name == "warning" {
                    self.as_invisible(result)
                } else {
                    self.as_visible(result)
                }
            }
            // The call site's `env` is the *caller's* environment — record it on
            // the frame so `parent.frame()` (R-23) inside the body can reflect it.
            SValue::Closure { params, body, env: cenv } => {
                self.call_closure(&params, &body, &cenv, env, args)
            }
            SValue::Negated(inner) => self.as_visible(self.call_negated(&inner, args)?),
            other => Err(SError::NotCallable(other.type_name().to_string())),
        }
    }

    /// Invoke a `Negate(f)` wrapper (R-20): call the wrapped `f` with `args`
    /// through the normal (depth-bounded) call path, then return the **logical
    /// negation** of its verdict. `!` flips `TRUE`/`FALSE` element-wise and
    /// preserves `NA`, so `Negate(is.na)(NA)` → `FALSE` and
    /// `Negate(\(x) x > 0)(5)` → `FALSE`. A non-callable inner `f` yields a clean
    /// `NotCallable` error (never a panic).
    fn call_negated(&self, inner: &SValue, args: &[Arg]) -> SResult<SValue> {
        let verdict = self.call_value(inner.clone(), args)?;
        logical_not(&verdict)
    }

    /// Call a callable value and return its result, *without* the top-level
    /// visibility/printing side effects. This is the entry point built-ins use
    /// to invoke a user function (`sapply`, `lapply`) or an S3 method.
    pub(crate) fn call_value(&self, callee: SValue, args: &[Arg]) -> SResult<SValue> {
        // `generator$new(...)` reached via a builtin (`do.call(gen$new, …)`).
        if let Some(generator) = crate::refclass::as_new_marker(&callee) {
            return self.apply_ref_new(&generator, args);
        }
        // R-25 nullary reference methods reached via a builtin (`do.call`, etc.).
        if let Some((action, target)) = crate::refclass::as_ref_method_marker(&callee) {
            return self.apply_ref_method(action, &target);
        }
        match callee {
            SValue::Builtin { func, .. } => func(self, args),
            // Invoked from a builtin (e.g. `sapply`/`Reduce`/`do.call`), there is
            // no R-level caller frame to thread through. The caller env is taken to
            // be the **global** environment, so a `parent.frame()` inside such a
            // callback sees the global frame — a faithful, panic-free default
            // (R itself reports the calling context here, which for our purposes is
            // the top level). Threading a real caller env through every builtin is
            // out of scope and far more invasive.
            SValue::Closure { params, body, env } => {
                let global = Rc::clone(&self.global);
                self.call_closure(&params, &body, &env, &global, args)
            }
            SValue::Negated(inner) => self.call_negated(&inner, args),
            other => Err(SError::NotCallable(other.type_name().to_string())),
        }
    }

    fn call_closure(
        &self,
        params: &[Param],
        body: &Rc<GrammarASTNode>,
        closure_env: &Env,
        caller_env: &Env,
        args: &[Arg],
    ) -> SResult<SValue> {
        // Record the call frame we are about to run: the function (so `Recall()`
        // (R-20) can re-invoke it) and the *caller's* environment (so
        // `parent.frame()` (R-23) can reflect it). The guard pops the frame again
        // on the way out — including on an early `?`/error — so the stack stays
        // balanced and the caller env never outlives the call. Reconstructing the
        // `Closure` value here is cheap: `params`/`body`/`env` are all `Rc`/clone
        // -friendly, and the alternative (threading the value through every call
        // site) is far more invasive.
        let _frame = CallFrameGuard::push(
            &self.call_stack,
            CallFrame {
                closure: SValue::Closure {
                    params: params.to_vec(),
                    body: Rc::clone(body),
                    env: Rc::clone(closure_env),
                },
                caller: Rc::clone(caller_env),
            },
        );

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
    // Special forms: switch / tryCatch (lazy — only the chosen arm evaluates)
    // -----------------------------------------------------------------------

    /// `switch(EXPR, ...)` — R's value-returning multi-way branch.
    ///
    /// The first argument `EXPR` is evaluated to choose *one* arm; only that arm
    /// is then evaluated (the rest are never touched — this is why `switch` must
    /// be a special form).
    ///
    /// - **Character `EXPR`**: match against the arm *names*. A matched arm that
    ///   is **empty** (`a = ,`) falls through to the next non-empty arm's value;
    ///   an **unnamed final arm** is the default when no name matches; no match
    ///   and no default yields an invisible `NULL`.
    /// - **Numeric `EXPR`**: select the n-th *value* arm by position (1-based),
    ///   ignoring names; out of range (or NA / < 1) yields invisible `NULL`.
    fn eval_switch(&self, raw: &[RawArg], env: &Env) -> SResult<SValue> {
        // `raw[0]` is EXPR; the remaining entries are the arms.
        let expr_node = raw
            .first()
            .and_then(|(_, n)| arm_body(n))
            .ok_or_else(|| SError::BadArgs("switch: EXPR is missing".into()))?;
        let arms = &raw[1..];
        let selector = self.eval_node(expr_node, env)?;

        // A length-1 character selector matches by name; anything else is taken
        // as a numeric position (matching R, which treats an integer selector
        // positionally).
        if let Some(name) = single_string(&selector) {
            // Find the arm whose name equals `name`. On a match, fall through any
            // empty arms to the next non-empty value.
            if let Some(pos) = arms
                .iter()
                .position(|(arm_name, _)| arm_name.as_deref() == Some(name.as_str()))
            {
                for (_, arm) in &arms[pos..] {
                    if let Some(body) = arm_body(arm) {
                        return self.eval_node(body, env);
                    }
                    // else: an empty arm — fall through to the next.
                }
                // Fell off the end with only empty arms → NULL.
                return self.as_invisible(SValue::Null);
            }
            // No name matched: an unnamed final arm is the default.
            if let Some((arm_name, arm)) = arms.last() {
                if arm_name.is_none() {
                    if let Some(body) = arm_body(arm) {
                        return self.eval_node(body, env);
                    }
                }
            }
            return self.as_invisible(SValue::Null);
        }

        // Numeric selector: the n-th arm, 1-based, by position.
        let n = scalar_f64(&selector).ok();
        match n {
            Some(x) if x >= 1.0 => {
                let idx = x as usize - 1;
                match arms.get(idx).and_then(|(_, arm)| arm_body(arm)) {
                    Some(body) => self.eval_node(body, env),
                    None => self.as_invisible(SValue::Null),
                }
            }
            _ => self.as_invisible(SValue::Null),
        }
    }

    /// `tryCatch(expr, error = handler, finally = cleanup)` — evaluate `expr`;
    /// on *any* catchable error route it to the `error` handler (called with a
    /// condition object), returning the handler's value; otherwise return the
    /// value of `expr`. The `finally` expression, if present, always runs (for
    /// its side effects) afterward. Everything is lazy: handlers and `finally`
    /// are unevaluated parse-tree nodes until needed.
    fn eval_try_catch(&self, raw: &[RawArg], env: &Env) -> SResult<SValue> {
        // The protected expression is the first positional argument; the
        // `error`/`finally` handlers are matched by name.
        let expr_node = raw
            .iter()
            .find(|(name, _)| name.is_none())
            .and_then(|(_, n)| arm_body(n))
            .ok_or_else(|| SError::BadArgs("tryCatch: expr is missing".into()))?;
        let handler_node = raw
            .iter()
            .find(|(name, _)| name.as_deref() == Some("error"))
            .and_then(|(_, n)| arm_body(n));
        let finally_node = raw
            .iter()
            .find(|(name, _)| name.as_deref() == Some("finally"))
            .and_then(|(_, n)| arm_body(n));

        // Evaluate the protected expression, intercepting any catchable error.
        let result = self.eval_node(expr_node, env);
        let outcome = match result {
            Ok(value) => Ok(value),
            Err(e) if e.is_catchable() => match handler_node {
                Some(h) => {
                    // Build the condition object and call the handler with it.
                    let condition = make_condition(&e.condition_message());
                    self.call_handler(h, condition, env)
                }
                // No `error` handler: the error still propagates (but `finally`
                // must run first — handled below).
                None => Err(e),
            },
            // A non-catchable control signal (break/next) is re-raised untouched,
            // but `finally` still runs first.
            Err(e) => Err(e),
        };

        // `finally` runs regardless of success or failure. If it itself errors,
        // that error supersedes (matching R).
        if let Some(f) = finally_node {
            self.eval_node(f, env)?;
        }
        outcome
    }

    /// Evaluate a `tryCatch` `error =` handler and apply it to the condition.
    /// The handler is normally a `function(e) ...`; we evaluate the handler
    /// expression to a callable and call it with the single condition argument.
    /// A non-callable handler is a clean error.
    fn call_handler(
        &self,
        handler_node: &GrammarASTNode,
        condition: SValue,
        env: &Env,
    ) -> SResult<SValue> {
        let handler = self.eval_node(handler_node, env)?;
        if !handler.is_callable() {
            return Err(SError::NotCallable(handler.type_name().to_string()));
        }
        let args = [Arg {
            name: None,
            value: condition,
        }];
        self.call_value(handler, &args)
    }

    // -----------------------------------------------------------------------
    // R-21 / R-22 environment forms
    //   R-21: local / assign / get / exists / rm  (against the current scope)
    //   R-22: new.env / environment / ls, and the `envir = e` argument on the
    //         binding ops, which now operate on a passed first-class environment.
    //
    // These are *lazy special forms* rather than ordinary builtins because they
    // must see the **current** environment `env`: `local` evaluates its block in
    // a fresh child of it, `new.env` makes the caller's scope the new parent, and
    // the by-name binding ops read/write the target frame directly. (Ordinary
    // builtins receive only their evaluated argument *values*, never the live
    // scope.)
    //
    // R-22 replaces R-21's runtime rejection of `envir = e` with the real
    // behaviour: the argument is evaluated, required to be an `SValue::Environment`
    // (any other value is a clean `BadArgs` error — never a panic), and the op
    // runs against *that* scope. Because an environment value shares the same
    // `Rc<RefCell<Scope>>`, mutating it through one alias is visible through every
    // other — environments are mutable **by reference**.
    // -----------------------------------------------------------------------

    /// `local({ ... })` — evaluate the (single, unevaluated) expression argument
    /// in a **fresh child environment** of the current scope and return its
    /// value. Because the block runs in its own frame, any `<-` bindings it makes
    /// are locals that do not leak: `local({ x <- 5; x * 2 })` is `10`, and `x`
    /// stays unbound in the caller. With `local(expr, envir = e)` (R-22) the block
    /// runs **directly in `e`** (R's second form), so its bindings persist in `e`.
    fn eval_local(&self, raw: &[RawArg], env: &Env) -> SResult<SValue> {
        let expr = raw
            .iter()
            .find(|(name, _)| name.is_none())
            .and_then(|(_, n)| arm_body(n))
            .ok_or_else(|| SError::BadArgs("local: expr is missing".into()))?;
        // With `envir = e`, run the block *in* `e`; otherwise in a fresh child
        // scope where locals live and die.
        let scope = match self.resolve_envir(raw, env)? {
            Some(target) => target,
            None => Scope::child(env),
        };
        self.eval_node(expr, &scope)
    }

    /// `assign(x, value [, envir = e])` — bind the name given by the length-one
    /// character `x` to `value` in the **target** environment: `e` if `envir =` is
    /// supplied (R-22), else the current scope. Returns `value` invisibly, like R.
    /// `assign("y", 1 + 1)` then `y` → `2`; `assign("x", 5, envir = e)` binds in
    /// `e` and is visible via `get("x", envir = e)`.
    fn eval_assign_fn(&self, raw: &[RawArg], env: &Env) -> SResult<SValue> {
        let positionals = self.positional_nodes(raw);
        let name_node = positionals
            .first()
            .ok_or_else(|| SError::BadArgs("assign: `x` (the name) is missing".into()))?;
        let value_node = positionals
            .get(1)
            .ok_or_else(|| SError::BadArgs("assign: `value` is missing".into()))?;
        let name = self.eval_name_string(name_node, env, "assign")?;
        let value = self.eval_node(value_node, env)?;
        // Resolve the target *after* evaluating the name/value, but the order is
        // immaterial (no aliasing between them); a non-environment `envir` errors.
        let target = self
            .resolve_envir(raw, env)?
            .unwrap_or_else(|| Rc::clone(env));
        define(&target, &name, value.clone());
        self.as_invisible(value)
    }

    /// `get(x [, envir = e])` — return the value bound to the name `x`, searching
    /// the target environment's chain (the frame outward). With `envir = e` the
    /// search starts in `e`; otherwise in the current frame. An unbound name is a
    /// clean error, exactly as in R (`Error: object 'x' not found`).
    fn eval_get_fn(&self, raw: &[RawArg], env: &Env) -> SResult<SValue> {
        let positionals = self.positional_nodes(raw);
        let name_node = positionals
            .first()
            .ok_or_else(|| SError::BadArgs("get: `x` (the name) is missing".into()))?;
        let name = self.eval_name_string(name_node, env, "get")?;
        let target = self
            .resolve_envir(raw, env)?
            .unwrap_or_else(|| Rc::clone(env));
        let value = lookup(&target, &name).ok_or_else(|| SError::Undefined(name.clone()))?;
        self.as_visible(value)
    }

    /// `exists(x [, envir = e])` — `TRUE` if the name `x` is bound anywhere on the
    /// target environment's chain, `FALSE` otherwise. `exists("mean")` → `TRUE`
    /// (a builtin in the global frame); `exists("zzz")` → `FALSE`. With `envir = e`
    /// the search walks `e`'s chain.
    fn eval_exists_fn(&self, raw: &[RawArg], env: &Env) -> SResult<SValue> {
        let positionals = self.positional_nodes(raw);
        let name_node = positionals
            .first()
            .ok_or_else(|| SError::BadArgs("exists: `x` (the name) is missing".into()))?;
        let name = self.eval_name_string(name_node, env, "exists")?;
        let target = self
            .resolve_envir(raw, env)?
            .unwrap_or_else(|| Rc::clone(env));
        self.as_visible(SValue::Logical(vec![Some(exists(&target, &name))]))
    }

    /// `rm(x [, envir = e])` — remove the binding `x` from the **target frame**
    /// directly (it does not reach into enclosing scopes, matching R's `rm(...,
    /// envir = environment())` default). With `envir = e` it deletes from `e`'s
    /// own frame; otherwise from the current one. Returns `NULL` invisibly.
    /// Removing a name that is not bound in the target frame is a quiet no-op.
    fn eval_rm_fn(&self, raw: &[RawArg], env: &Env) -> SResult<SValue> {
        let positionals = self.positional_nodes(raw);
        let name_node = positionals
            .first()
            .ok_or_else(|| SError::BadArgs("rm: a name to remove is missing".into()))?;
        let name = self.eval_name_string(name_node, env, "rm")?;
        let target = self
            .resolve_envir(raw, env)?
            .unwrap_or_else(|| Rc::clone(env));
        remove(&target, &name);
        self.as_invisible(SValue::Null)
    }

    // -----------------------------------------------------------------------
    // R-22 first-class environment forms (new.env / environment / ls)
    // -----------------------------------------------------------------------

    /// `new.env()` — create a **fresh** environment whose parent is the caller's
    /// current scope, and return it as a first-class value (R-22). Two calls
    /// produce two **independent** environments (distinct `Rc`s). The new scope's
    /// parent link is a `Weak` to `env` (see [`crate::env`]), so the returned
    /// value owning the only strong `Rc` to the child cannot form a cycle with its
    /// parent.
    fn eval_new_env(&self, _raw: &[RawArg], env: &Env) -> SResult<SValue> {
        self.account_environment()?;
        let scope = Scope::child(env);
        self.as_visible(SValue::Environment(scope))
    }

    /// `environment([f])` — the **current** environment (no argument, R-22) or, in
    /// the R-23 `environment(f)` form, the environment a closure `f` **captured**
    /// at definition. A `Closure` stores its defining scope in `Closure { env }`;
    /// we hand it back as an `SValue::Environment`, so for a top-level closure
    /// `identical(environment(f), globalenv())` is `TRUE`. A non-closure argument
    /// (a builtin, a number, …) yields `NULL`, matching R (`environment(sum)` is
    /// `NULL`). Reifying *either* environment counts against `MAX_ENVIRONMENTS`
    /// (both can participate in a value-binding cycle).
    fn eval_environment(&self, raw: &[RawArg], env: &Env) -> SResult<SValue> {
        // The `environment(f)` form: a single argument with a body expression.
        if let Some(arg_node) = raw.iter().find_map(|(_, n)| arm_body(n)) {
            let value = self.eval_node(arg_node, env)?;
            return match value {
                // A closure carries its captured (defining) environment.
                SValue::Closure { env: captured, .. } => {
                    self.account_environment()?;
                    self.as_visible(SValue::Environment(captured))
                }
                // R returns NULL for a primitive/builtin or any non-closure.
                _ => self.as_visible(SValue::Null),
            };
        }
        // `environment()` reifies the current scope as a value too — count it
        // against the same cap (it can equally participate in a value-binding
        // cycle, e.g. `assign("self", environment(), envir = environment())`).
        self.account_environment()?;
        self.as_visible(SValue::Environment(Rc::clone(env)))
    }

    /// `environmentName(e)` (R-23) — the well-known *name* of an environment:
    /// `"R_GlobalEnv"` for the session global env, `"R_EmptyEnv"` for the empty
    /// env, and `""` for every other environment (R does not name anonymous
    /// frames). Identity is by `Rc` **pointer** equality against the interpreter's
    /// long-lived `global`/`empty` handles — never by comparing contents — so it
    /// is O(1) and re-entrancy-safe. A non-environment argument is a clean
    /// `BadArgs` error.
    fn eval_environment_name(&self, raw: &[RawArg], env: &Env) -> SResult<SValue> {
        let node = self
            .positional_nodes(raw)
            .into_iter()
            .next()
            .ok_or_else(|| SError::BadArgs("environmentName: the environment is missing".into()))?;
        let target = match self.eval_node(node, env)? {
            SValue::Environment(e) => e,
            other => {
                return Err(SError::BadArgs(format!(
                    "environmentName: expected an environment, got {}",
                    other.type_name()
                )))
            }
        };
        let name = if same_env(&target, &self.global) {
            "R_GlobalEnv"
        } else if same_env(&target, &self.empty) {
            "R_EmptyEnv"
        } else {
            ""
        };
        self.as_visible(SValue::Character(vec![Some(name.to_string())]))
    }

    /// `globalenv()` / `baseenv()` (R-23) — the session **global** environment as
    /// a value. This runtime installs its builtins directly into the global frame
    /// (there is no separate base namespace), so `baseenv()` deliberately aliases
    /// the global env — documented in the spec. Hands back the *same* `Rc` every
    /// call (so `identical(globalenv(), globalenv())` holds); allocates nothing, so
    /// it does not count against `MAX_ENVIRONMENTS`.
    fn eval_globalenv(&self) -> SResult<SValue> {
        self.as_visible(SValue::Environment(Rc::clone(&self.global)))
    }

    /// `emptyenv()` (R-23) — the session **empty** environment as a value (the
    /// parentless, bindingless root). Same `Rc` every call; allocates nothing.
    fn eval_emptyenv(&self) -> SResult<SValue> {
        self.as_visible(SValue::Environment(Rc::clone(&self.empty)))
    }

    /// `parent.frame(n = 1)` (R-23) — the environment of the **caller** `n` frames
    /// up the call stack. R-20's call stack already records the closure being run;
    /// R-23 records the caller's env alongside it, which this reads. `n` defaults
    /// to `1` (the immediate caller) and must be a **positive whole number**; a
    /// non-positive, non-finite, or non-numeric `n` is a clean `BadArgs` error.
    /// The walk **clamps** to the global env past the bottom of the stack (or at
    /// top level), so it can never index out of bounds — see
    /// [`Interpreter::caller_frame`].
    fn eval_parent_frame(&self, raw: &[RawArg], env: &Env) -> SResult<SValue> {
        let n = match self.positional_nodes(raw).into_iter().next() {
            None => 1usize,
            Some(node) => {
                let v = self.eval_node(node, env)?;
                let x = scalar_f64(&v).map_err(|_| {
                    SError::BadArgs("parent.frame: n must be a single number".into())
                })?;
                if !x.is_finite() || x < 1.0 {
                    return Err(SError::BadArgs(
                        "parent.frame: n must be a positive whole number".into(),
                    ));
                }
                // Truncate toward zero (R coerces to integer); we have x >= 1.0 and
                // finite, so this is a safe, bounded cast.
                x as usize
            }
        };
        self.as_visible(SValue::Environment(self.caller_frame(n)))
    }

    /// `missing(x)` (R-26) — is the parameter `x` **absent** in the current call
    /// frame? Its single argument is a bare **name** (never evaluated). We report
    /// `TRUE` iff that name is not bound in the *immediate* frame `env` — which is
    /// exactly what distinguishes an R5 active binding's nullary getter call
    /// (`function(v)` invoked with no arg → `v` unbound → `missing(v)` TRUE) from its
    /// setter call (`v` supplied → bound → FALSE). A non-name argument is a clean
    /// error. (This is a faithful subset of R's `missing`: it does not chase default
    /// expressions or promise state, which this runtime does not model — an unbound
    /// formal is simply one no argument filled.)
    fn eval_missing(&self, raw: &[RawArg], env: &Env) -> SResult<SValue> {
        let node = self.positional_nodes(raw).into_iter().next().ok_or_else(|| {
            SError::BadArgs("missing: an argument name is required".into())
        })?;
        let name = lvalue_name(node)
            .map_err(|_| SError::BadArgs("missing: the argument must be a variable name".into()))?;
        // Frame-local only: `missing(x)` asks about *this* function's formal, not an
        // enclosing binding of the same name. `lookup_local` reads exactly this frame.
        let absent = crate::env::lookup_local(env, &name).is_none();
        self.as_visible(SValue::Logical(vec![Some(absent)]))
    }

    /// `callSuper(...)` (R-26) — inside an overriding R5 method, invoke the
    /// **same-named** method from the parent class. The running method's
    /// super-context (placed by `refclass::rebuild_method`) is read from the current
    /// environment to learn the method name and the generator(s) at which to restart
    /// resolution; the resolved super method is re-homed onto the **instance** (found
    /// via `.self`) and applied to the forwarded (evaluated) args. **Past-the-root**
    /// (no super definition of the name) returns `NULL` — R5's behaviour — with no
    /// recursion and no panic.
    fn eval_call_super(&self, raw: &[RawArg], env: &Env) -> SResult<SValue> {
        // The instance is reachable via `.self`, bound on the instance frame which is
        // an ancestor of the method-body env. Absent (`callSuper()` outside any R5
        // method) → a clean NULL rather than an error, matching R's lenient handling.
        let instance = match lookup(env, crate::refclass::KEY_SELF) {
            Some(SValue::Environment(e)) => e,
            _ => return self.as_visible(SValue::Null),
        };
        let Some(super_closure) = crate::refclass::call_super_method(env, &instance) else {
            // No super method of this name → NULL (no recursion past the root).
            return self.as_visible(SValue::Null);
        };
        // Forward the call args (evaluated in the *caller* env) to the super method.
        let mut args: Vec<Arg> = Vec::new();
        for (name, node) in raw {
            if let Some(body) = arm_body(node) {
                args.push(Arg {
                    name: name.clone(),
                    value: self.eval_node(body, env)?,
                });
            }
        }
        self.as_visible(self.call_value(super_closure, &args)?)
    }

    // -----------------------------------------------------------------------
    // R-24 R5 reference classes (setRefClass)
    // -----------------------------------------------------------------------

    /// `setRefClass("Name", fields = …, methods = …)` (R-24) — build a reference
    /// class **generator**. A lazy special form: it evaluates the class name and
    /// the `fields`/`methods` arguments *in the current environment* (so the
    /// method `function(...)` definitions close over the scope where the class was
    /// declared), then hands the pieces to [`crate::refclass::make_generator`].
    /// The generator is a first-class environment value carrying the class name,
    /// field names, and method closures, and it counts against `MAX_ENVIRONMENTS`.
    ///
    /// The class name is the first **positional** argument (a length-1 character);
    /// `fields` and `methods` are matched **by name** (R5's call shape). A missing
    /// `fields`/`methods` defaults to "no fields"/"no methods". A non-character
    /// name, or a malformed `fields`/`methods`, is a clean error (never a panic).
    fn eval_set_ref_class(&self, raw: &[RawArg], env: &Env) -> SResult<SValue> {
        // First positional argument → the class name.
        let name_node = self.positional_nodes(raw).into_iter().next().ok_or_else(|| {
            SError::BadArgs("setRefClass: the class name is required".into())
        })?;
        let class_name = self.eval_name_string(name_node, env, "setRefClass")?;

        // `fields =` / `methods =` (named); each defaults to NULL when omitted.
        let fields = self.eval_named_arg(raw, "fields", env)?.unwrap_or(SValue::Null);
        let methods = self
            .eval_named_arg(raw, "methods", env)?
            .unwrap_or(SValue::Null);

        // R-25/R-26: `contains =`. The argument is a parent **generator** value, a
        // length-1 character giving a parent class *name* (resolved by evaluating that
        // name as a variable in the current env — the generator was bound there by an
        // earlier `setRefClass`), or — for R-26 **multiple inheritance** — a character
        // vector `c("A", "B")` (each element a parent class name, left-to-right). A
        // missing `contains =` means a root (non-inheriting) class.
        let parents = match self.eval_named_arg(raw, "contains", env)? {
            None | Some(SValue::Null) => Vec::new(),
            Some(value) => self.resolve_contains(&value, env)?,
        };

        // Reifying the generator environment counts against the session cap, like
        // any `new.env()` — it can equally participate in a value-binding cycle.
        self.account_environment()?;
        let generator =
            crate::refclass::make_generator(&class_name, &fields, &methods, &parents, env)?;
        self.as_visible(generator)
    }

    /// Resolve a `contains =` argument to the parent **generator** environments, in
    /// left-to-right order. Accepts the generator value directly (an
    /// `SValue::Environment`, single parent), or a **character vector** of one or
    /// more parent class *names* — each looked up as a variable in `env` (where
    /// `setRefClass` binds its result). `c("A", "B")` is R-26 multiple inheritance.
    /// A name that is unbound, or bound to a non-environment, is a clean error (never
    /// a panic). The generator-ness of each resolved env is re-checked inside
    /// `make_generator`. An empty character vector yields no parents (a root class).
    fn resolve_contains(&self, value: &SValue, env: &Env) -> SResult<Vec<Env>> {
        match value {
            SValue::Environment(e) => Ok(vec![e.clone()]),
            other => {
                // A character vector of class names → resolve each, in order.
                let names = other.as_character();
                if names.is_empty() {
                    return Err(SError::TypeError(
                        "setRefClass: `contains =` must be a generator or class name(s)".into(),
                    ));
                }
                let mut out = Vec::with_capacity(names.len());
                for n in names {
                    let name = n.ok_or_else(|| {
                        SError::TypeError(
                            "setRefClass: `contains =` class name may not be NA".into(),
                        )
                    })?;
                    match lookup(env, &name) {
                        Some(SValue::Environment(e)) => out.push(e),
                        Some(_) => {
                            return Err(SError::TypeError(format!(
                                "setRefClass: `contains = \"{name}\"` is not a reference-class generator"
                            )))
                        }
                        None => {
                            return Err(SError::BadArgs(format!(
                                "setRefClass: `contains = \"{name}\"`: no such reference class in scope"
                            )))
                        }
                    }
                }
                Ok(out)
            }
        }
    }

    /// Apply a `generator$new(field = …, …)` instantiation (R-24): charge the new
    /// instance environment against `MAX_ENVIRONMENTS` (it is a fresh reified
    /// scope, exactly like `new.env()`, and `.self` makes it a value-binding
    /// self-cycle — the documented, bounded R-22 case), then build the instance via
    /// [`crate::refclass::instantiate`]. The already-evaluated `args` carry their
    /// names so fields are matched by keyword.
    fn apply_ref_new(&self, generator: &Env, args: &[Arg]) -> SResult<SValue> {
        self.account_environment()?;
        let init: Vec<(Option<String>, SValue)> = args
            .iter()
            .map(|a| (a.name.clone(), a.value.clone()))
            .collect();
        let instance = crate::refclass::instantiate(generator, &init)?;
        self.as_visible(instance)
    }

    /// Apply an R-25 nullary reference method (`obj$copy()`, `gen$fields()`,
    /// `gen$methods()`). `copy()` reifies a fresh instance environment, so it is
    /// charged against `MAX_ENVIRONMENTS` exactly like `$new` before the copy runs;
    /// the introspection accessors allocate no environment. The actual work lives in
    /// [`crate::refclass::apply_ref_method`].
    fn apply_ref_method(&self, action: &'static str, target: &Env) -> SResult<SValue> {
        if action == crate::refclass::REF_METHOD_COPY {
            self.account_environment()?;
        }
        crate::refclass::apply_ref_method(action, target)
    }

    /// Resolve `obj$name` for a `$` *read* (R-24). An [`SValue::Environment`]
    /// target is routed through [`crate::refclass::dollar_access`], which handles
    /// `generator$new` (returns the instantiation marker), `obj$method` (rebuilds
    /// a fresh instance-bound closure on access — see the refclass module note on
    /// the instance⇄method cycle), and falls through (`None`) to an ordinary field
    /// / binding lookup in the environment. A `NULL` is returned for an unset field
    /// (matching R5). Every other target keeps the existing data-frame / list `$`
    /// behaviour.
    fn dollar_read(&self, target: &SValue, name: &str) -> SResult<SValue> {
        if let SValue::Environment(e) = target {
            // R-26 active binding: reading `obj$ab` **calls** the binding function as
            // a getter (no argument — so `missing(v)` is TRUE inside it). Invoked
            // through the ordinary depth-bounded call path, so a getter that reads
            // its *own* binding recurses and hits `MAX_EVAL_DEPTH` with a clean error
            // rather than a borrow panic or a hang.
            if let Some(getter) = crate::refclass::active_binding_fn(e, name) {
                return self.call_value(getter, &[]);
            }
            if let Some(v) = crate::refclass::dollar_access(e, name) {
                return Ok(v);
            }
            // Ordinary environment access: the binding's value, or NULL if unset
            // (R5 reads an unset field as NULL; a plain `env$x` for a missing name
            // is likewise NULL in R rather than an error).
            return Ok(lookup(e, name).unwrap_or(SValue::Null));
        }
        crate::dataframe::column_by_name(target, name)
    }

    /// Evaluate the named argument `key` of a raw arg list (`key = expr`), if
    /// present, returning `Ok(None)` when it is absent. An empty arm (`key =`
    /// with no expression) is a clean error. Shared by `setRefClass`'s
    /// `fields =`/`methods =` handling.
    fn eval_named_arg(
        &self,
        raw: &[RawArg],
        key: &str,
        env: &Env,
    ) -> SResult<Option<SValue>> {
        let Some((_, node)) = raw.iter().find(|(name, _)| name.as_deref() == Some(key)) else {
            return Ok(None);
        };
        let expr = arm_body(node)
            .ok_or_else(|| SError::BadArgs(format!("setRefClass: `{key} = ` is missing a value")))?;
        Ok(Some(self.eval_node(expr, env)?))
    }

    /// `ls([envir = e])` — the names bound **directly** in the target frame (not
    /// the enclosing chain), as a **sorted** character vector (R-22). `ls(e)`
    /// accepts the environment positionally too (R's `ls` takes the env as its
    /// first argument); `ls()` lists the current environment. Other (non-env)
    /// arguments are a clean error.
    fn eval_ls(&self, raw: &[RawArg], env: &Env) -> SResult<SValue> {
        // `envir = e` takes precedence; otherwise a lone positional environment
        // argument (`ls(e)`) is honoured; otherwise the current scope.
        let target = match self.resolve_envir(raw, env)? {
            Some(t) => t,
            None => match self.positional_nodes(raw).first() {
                Some(node) => match self.eval_node(node, env)? {
                    SValue::Environment(e) => e,
                    other => {
                        return Err(SError::BadArgs(format!(
                            "ls: argument must be an environment, got {}",
                            other.type_name()
                        )))
                    }
                },
                None => Rc::clone(env),
            },
        };
        let names = names_in(&target);
        let chars = names.into_iter().map(Some).collect();
        self.as_visible(SValue::Character(chars))
    }

    /// Resolve an `envir = e` argument, if present, to the [`Env`] it names. R-22:
    /// the argument is evaluated and **must** be an `SValue::Environment`; any
    /// other value (a number, string, list, …) is a clean `BadArgs` error rather
    /// than a panic. Returns `Ok(None)` when no `envir =` argument was supplied
    /// (the caller then defaults to the current scope). The shared
    /// `Rc<RefCell<Scope>>` is cloned out — the same underlying frame — so writes
    /// through it are visible by reference.
    fn resolve_envir(&self, raw: &[RawArg], env: &Env) -> SResult<Option<Env>> {
        let Some((_, node)) = raw
            .iter()
            .find(|(name, _)| name.as_deref() == Some("envir"))
        else {
            return Ok(None);
        };
        let Some(expr) = arm_body(node) else {
            return Err(SError::BadArgs(
                "envir = : the environment is missing".into(),
            ));
        };
        match self.eval_node(expr, env)? {
            SValue::Environment(e) => Ok(Some(e)),
            other => Err(SError::BadArgs(format!(
                "envir = : expected an environment, got {}",
                other.type_name()
            ))),
        }
    }

    /// The *positional* (unnamed) argument nodes of a raw arg list, in order —
    /// the binding ops take `x` (and `value`) positionally.
    fn positional_nodes<'a>(&self, raw: &'a [RawArg<'a>]) -> Vec<&'a GrammarASTNode> {
        raw.iter()
            .filter(|(name, _)| name.is_none())
            .filter_map(|(_, n)| arm_body(n))
            .collect()
    }

    /// Evaluate a name-argument node to the `String` it denotes. R's binding ops
    /// take the name as a length-one **character** value (`get("x")`), and a
    /// variable holding such a string works too (`nm <- "x"; get(nm)`). We
    /// therefore evaluate the node and require a length-one character result;
    /// anything else (a number, a length-≠1 vector) is a clean `BadArgs` error.
    fn eval_name_string(
        &self,
        node: &GrammarASTNode,
        env: &Env,
        who: &str,
    ) -> SResult<String> {
        let value = self.eval_node(node, env)?;
        single_string(&value).ok_or_else(|| {
            SError::BadArgs(format!(
                "{who}: the name must be a single character string"
            ))
        })
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

/// One *unevaluated* call argument: its optional name and its expression node.
/// Special forms (`switch`/`tryCatch`) work on these so they can choose which
/// arm to evaluate.
type RawArg<'a> = (Option<String>, &'a GrammarASTNode);

/// If `primary` is a bare-name reference to one of the lazy special forms
/// (`switch`, `tryCatch`), return that name; otherwise `None`. A special form is
/// recognized only when the primary reduces to exactly that single `NAME` token
/// (so `f()(...)` or an indexed primary never trips the check).
fn special_form_name(primary: &GrammarASTNode) -> Option<&'static str> {
    let mut tokens: Vec<(&str, &str)> = Vec::new();
    collect_tokens(primary, &mut tokens);
    match tokens.as_slice() {
        [("NAME", "switch")] => Some("switch"),
        [("NAME", "tryCatch")] => Some("tryCatch"),
        // R-21 environment forms — special because they must see the *current*
        // environment (`local` evaluates its block in a fresh child scope; the
        // binding ops read/write `env` by name), which ordinary eager builtins
        // never receive.
        [("NAME", "local")] => Some("local"),
        [("NAME", "assign")] => Some("assign"),
        [("NAME", "get")] => Some("get"),
        [("NAME", "exists")] => Some("exists"),
        [("NAME", "rm")] => Some("rm"),
        // R-22 first-class environment forms — also special because they read the
        // *current* environment (`new.env`'s parent, `environment()`'s value,
        // `ls()`'s default frame). The dotted names lex as a single `NAME` token
        // (`.` is a name character), so `new.env` matches as one token.
        [("NAME", "new.env")] => Some("new.env"),
        [("NAME", "environment")] => Some("environment"),
        [("NAME", "ls")] => Some("ls"),
        // R-23 closure environments & frame reflection. These also need the
        // *current* environment (`parent.frame` reads the caller frame; the
        // others read the interpreter's well-known handles), so they are special
        // forms too. The dotted `parent.frame` lexes as a single `NAME` token.
        [("NAME", "environmentName")] => Some("environmentName"),
        [("NAME", "globalenv")] => Some("globalenv"),
        [("NAME", "emptyenv")] => Some("emptyenv"),
        [("NAME", "baseenv")] => Some("baseenv"),
        [("NAME", "parent.frame")] => Some("parent.frame"),
        // R-24 R5 reference classes. `setRefClass` is a special form because it
        // must (a) capture the *current* environment as the generator's lexical
        // parent — so a method's free variables resolve to where the class was
        // defined — and (b) evaluate `fields`/`methods` itself (the methods are
        // function definitions that must close over that captured scope). An
        // ordinary eager builtin receives neither the current env nor control over
        // argument evaluation.
        [("NAME", "setRefClass")] => Some("setRefClass"),
        // R-26 R5 method helpers. `missing(x)` must inspect whether the *named
        // parameter* `x` was supplied in the **current** call frame — it never
        // evaluates `x`, so it cannot be an eager builtin. `callSuper(...)` must read
        // the running method's super-context markers from the *current* environment
        // and forward its (evaluated) args to the resolved super method.
        [("NAME", "missing")] => Some("missing"),
        [("NAME", "callSuper")] => Some("callSuper"),
        _ => None,
    }
}

/// Collect the *unevaluated* arguments of a `call_suffix` as `(name, expr-node)`
/// pairs, preserving order. A named argument (`NAME = expr`) keeps its name; a
/// positional one has `None`. An empty arm (`a = ,` — a named `arg` with no
/// inner expression, as in `switch`) is represented by a node whose
/// [`arm_body`] is `None`.
fn raw_args(suffix: &GrammarASTNode) -> Vec<RawArg<'_>> {
    let mut out = Vec::new();
    let Some(arg_list) = suffix.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(n) if n.rule_name == "arg_list" => Some(n),
        _ => None,
    }) else {
        return out;
    };
    for child in &arg_list.children {
        if let ASTNodeOrToken::Node(arg) = child {
            if arg.rule_name != "arg" {
                continue;
            }
            // A named argument is `NAME = expr` (two leading tokens); positional
            // is just `expr`. (Mirrors `eval_arg`'s detection.)
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
            out.push((name, arg));
        }
    }
    out
}

/// The expression node inside an `arg`, or `None` if the arg is **empty** (a
/// `switch` fall-through arm like `a = ,`, which has a name but no value
/// expression). Used by both `switch` arm selection and `tryCatch` handler
/// lookup.
fn arm_body(arg: &GrammarASTNode) -> Option<&GrammarASTNode> {
    first_node(arg)
}

/// A length-1 character value as its `String`, else `None`. Sees through the
/// transparent wrappers. Used by `switch` to decide name- vs position-matching.
fn single_string(value: &SValue) -> Option<String> {
    match value.strip_names() {
        SValue::Character(v) if v.len() == 1 => v[0].clone(),
        SValue::Classed { inner, .. } => single_string(inner),
        SValue::Attributed { inner, .. } => single_string(inner),
        _ => None,
    }
}

/// Build the minimal **condition object** handed to a `tryCatch` `error =`
/// handler: a list `list(message = <chr>, call = NULL)` carrying the S3 class
/// `c("simpleError", "error", "condition")`. This is enough for
/// `conditionMessage(e)` and `e$message` to recover the message; full R
/// condition machinery (custom classes, restarts) is out of scope.
fn make_condition(message: &str) -> SValue {
    let inner = SValue::list(vec![
        (
            Some("message".to_string()),
            SValue::Character(vec![Some(message.to_string())]),
        ),
        (Some("call".to_string()), SValue::Null),
    ]);
    SValue::Classed {
        inner: Box::new(inner),
        class: vec![
            "simpleError".to_string(),
            "error".to_string(),
            "condition".to_string(),
        ],
    }
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
        SValue::Attributed { inner, .. } => nth_element(inner, i),
        SValue::List { items, .. } => items.get(i).cloned().unwrap_or(SValue::Null),
        _ => SValue::Null,
    }
}

/// Convenience: evaluate `src` in a fresh interpreter and return the value.
pub fn eval_s(src: &str) -> SResult<SValue> {
    Interpreter::new().eval_str(src).map(|o| o.value)
}
