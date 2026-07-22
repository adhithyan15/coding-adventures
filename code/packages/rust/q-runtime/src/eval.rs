//! The tree-walking evaluator.
//!
//! [`Interpreter`] walks the [`GrammarASTNode`] tree from `q-parser` and
//! computes values over `array_runtime::Array`. Q reuses APL/J's two-
//! nonterminal split (MA11 §3): `noun_expr` (arrays/scalars/functions) and
//! `verb_expr` (a callable — a primitive glyph, optionally with one adverb,
//! or a user-defined function value). Unlike APL/J, a Q value is not
//! *always* a bare array: a function literal (`{[x;y] ...}`) is itself an
//! ordinary noun value (MA11 §2/§3 bullet 1), so this evaluator's value
//! type — [`QValue`] — is `Arr(Array)` **or** `Fn(Rc<Lambda>)`, not just
//! `Array` the way `j-runtime`'s evaluator gets away with.
//!
//! ## `QFn` — generalizing `j-runtime::eval::JFn`, minus trains
//!
//! `QFn` is this crate's own representation of "which verb, and with which
//! adverb (if any) applied" — the direct analogue of `JFn`
//! (`j-runtime/src/eval.rs`), MA11 §2's own named reference point. Q has
//! **no trains and no `@` compose** (MA11 §3/§4: deferred, and not even a
//! genuine K/Q concept to begin with), so `QFn` has no `Compose`/`Hook`/
//! `Fork` counterpart at all — every variant here is a leaf dispatch with
//! no self-referential recursion, unlike `JFn` (which needed a hand-rolled
//! iterative `Drop` specifically because `Compose`/`Hook`/`Fork` box their
//! own operands). `QFn` needs no such thing:
//!
//! - **`Prim(Prim)`** — a bare primitive glyph, applied directly.
//! - **`Each(Prim)`** — a primitive with `'` (each) applied.
//! - **`Reduce(BinOp)`** — a `BinOp`-mappable primitive with `/` applied.
//! - **`Scan(BinOp)`** — a `BinOp`-mappable primitive with `\` applied.
//! - **`Lambda(Rc<Lambda>)`** — MA11's headline novelty: a user-defined
//!   function value, with named parameters, a multi-statement body, and
//!   (implicitly) whatever global bindings are visible at call time (see
//!   [`Lambda`]'s own doc comment for exactly what "capture" means here).
//!
//! `q.grammar` only ever attaches an adverb to a bare `verb_primitive`
//! (never to a `NAME` or `function_literal` — confirmed directly against
//! `code/grammars/q/q.grammar`'s own `verb_expr` production), so `Each`/
//! `Reduce`/`Scan` are only ever built from a `Prim`, never a `Lambda` —
//! this evaluator does not need to (and does not) handle "each/reduce/scan
//! of a user-defined function" at all.
//!
//! ## Calling a function value: one dispatch site, not two
//!
//! MA11 §3 bullet 1 is explicit: a function literal is "applied with the
//! same juxtaposition/`@` mechanism as a primitive verb — no new
//! *application* production, only a new way to *produce* a callable
//! value." This evaluator honors that literally: [`Interpreter::apply_monadic`]/
//! [`Interpreter::apply_dyadic`] are the **one** dispatch site for every
//! `QFn` variant, `Lambda` included — there is no separate "call a lambda"
//! code path bolted on elsewhere. The one place this evaluator *does* need
//! new logic beyond `j-runtime`'s shape is deciding *what is callable* at
//! all: `q.grammar`'s `noun_expr` widens its optional continuation to admit
//! a bare `term` (not just `verb_expr`) in the "apply" position (see
//! `q.grammar`'s own header comment and `q-parser`'s README for the full
//! grammar-level rationale) — [`Interpreter::eval_noun_expr`]'s 2-child
//! case disambiguates by inspecting which alternative actually matched
//! (`kids[0].rule_name`), and [`as_callable`] is the one place that decides
//! whether a [`QValue`] just evaluated is actually usable as a `QFn`.

use crate::builtins::{self, Prim};
use crate::value;
use array_runtime::{ops, ops::BinOp, Array};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// Maximum recursion depth for this evaluator's own tree-walk *and* call
/// chain. `q-parser`'s own `MAX_RULE_DEPTH` (32) already bounds how deep a
/// single parsed *statement*'s tree can be before it ever reaches this
/// crate — but unlike `j-runtime::eval::MAX_DEPTH` (which, per that
/// module's own doc comment, is "never actually reachable through genuine
/// parsed input" because J has no user-defined functions to chain calls
/// through), this guard **is** genuinely reachable by a legitimate (if
/// unusual) Q program here: a source file can define an arbitrarily long
/// chain of already-defined functions across many separate top-level
/// lines (`f1:{x+1}`, `f2:{f1 x+1}`, ..., `f10000:{f9999 x+1}`, each
/// individually shallow) and then invoke the last one, which recurses
/// through [`Interpreter::call_lambda`] once per link in that chain at
/// *runtime* — a recursion shape no single parse tree's own depth reflects
/// at all, and the reason this constant cannot just be copied from
/// `j-runtime`'s own (unreachable-in-practice) `MAX_DEPTH` without its own,
/// independent empirical measurement.
///
/// # Measured, not guessed: the real native-stack crash floor
///
/// Every recursive `call_lambda` (one per chained function call) leaves
/// exactly **4** [`Interpreter::enter`] guards on the stack for the
/// duration of the recursion — one each in
/// [`Interpreter::eval_assignment`], [`Interpreter::eval_noun_expr`],
/// [`Interpreter::apply_monadic`], and `call_lambda` itself (every other
/// call on the path — `eval_statement_value`, `eval_term`, the bounded
/// evaluation of a call's own argument expression — either doesn't guard
/// at all or fully unwinds before the next `call_lambda`, so it doesn't
/// contribute to the *peak* depth). This 4:1 ratio was independently
/// confirmed empirically (see below), not just derived by inspection.
///
/// Measured via the same binary-search methodology `apl-parser`/
/// `j-parser`/`q-parser` use for their own depth caps, adapted to this
/// evaluator's genuinely different recursion shape (a real Rust call
/// chain through `call_lambda`, not a parser's tree descent): a throwaway
/// **subprocess per data point** (`cargo test --exact` re-invoked per
/// candidate chain length — a real native-stack overflow calls
/// `abort()`, which kills the whole process, not just the offending
/// thread, so each data point must be independently disposable), each
/// running the chained-call source above on a
/// [`std::thread::Builder::stack_size`]-controlled worker thread with an
/// **explicit 2 MiB stack** (2,097,152 bytes — chosen as a known,
/// reproducible reference point, deliberately *not* relying on the
/// ambient default, which `RUST_MIN_STACK` can silently enlarge and
/// invalidate the whole measurement) and `MAX_DEPTH` itself temporarily
/// raised to 10,000,000 so the *guard* never interfered with finding the
/// real crash point. Result: **safe up to 268 chained calls, crashes
/// (`fatal runtime error: stack overflow`, `SIGABRT`) at 271** on a 2 MiB
/// stack.
///
/// `MAX_DEPTH` is set to **760** — `760 / 4 = 190` chained calls before
/// the guard fires, i.e. **189** succeed — about **29.5%** below the
/// measured 268-call safe ceiling (and 29.9% below the 271-call crash
/// point itself), comparable to `apl-parser`'s/`j-parser`'s/`q-parser`'s
/// own ~26.5–30% margins. This was cross-checked directly (not just
/// computed on paper): with `MAX_DEPTH` set to 760, a real capped run of
/// exactly 189 chained calls succeeds and 190 returns a clean `Err` (see
/// `tests::real_recursion_up_to_max_depth_succeeds_one_past_it_errors_cleanly_on_a_known_stack`),
/// confirming the 4:1 ratio holds exactly in practice, not just in
/// theory.
const MAX_DEPTH: usize = 760;

/// A persistent Q session: a stack of variable-binding frames (index 0 is
/// the always-present global frame; index 1+ are call-local frames pushed
/// around a [`Lambda`] call, MA11 §4's "local to that call only" scoping)
/// and the current evaluation depth.
///
/// Every field uses interior mutability (`RefCell`/`Cell`) so every
/// evaluation method can take `&self` uniformly — including
/// [`apply_monadic`](Interpreter::apply_monadic)/
/// [`apply_dyadic`](Interpreter::apply_dyadic), which must be able to push
/// and pop a call-local frame (calling a [`Lambda`]) from deep inside an
/// otherwise-read-only tree walk, mirroring why `j-runtime::eval::Interpreter`
/// keeps its own depth counter in a `Rc<Cell<usize>>` rather than threading
/// `&mut self` through every recursive call.
pub struct Interpreter {
    env: RefCell<Vec<HashMap<String, QValue>>>,
    depth: Rc<Cell<usize>>,
}

/// RAII guard that decrements the depth counter on every exit path
/// (including a `?` early return) -- mirrors
/// `j-runtime::eval::DepthGuard` exactly.
struct DepthGuard(Rc<Cell<usize>>);

impl Drop for DepthGuard {
    fn drop(&mut self) {
        self.0.set(self.0.get().saturating_sub(1));
    }
}

/// RAII guard that pops the call-local frame pushed by
/// [`Interpreter::call_lambda`], on every exit path (including an early
/// `?` return partway through evaluating the body) -- the same "guard owns
/// the undo" pattern as [`DepthGuard`], applied to the environment stack
/// instead of the depth counter.
struct FrameGuard<'a> {
    env: &'a RefCell<Vec<HashMap<String, QValue>>>,
}

impl Drop for FrameGuard<'_> {
    fn drop(&mut self) {
        self.env.borrow_mut().pop();
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

/// A Q value: either a plain numeric array (MA11 §4's in-scope value
/// model) or a user-defined function (MA11 §2/§3 bullet 1's headline
/// novelty) -- "a function literal is itself an ordinary noun value...
/// assignable, passable."
#[derive(Debug, Clone)]
pub enum QValue {
    Arr(Array),
    Fn(Rc<Lambda>),
}

/// A user-defined function literal: `{[x;y] stmt; stmt; ...}`, or the
/// bracket-omitted implicit-`x`/`y`/`z` form. This is the one genuinely new
/// evaluator concept MA11 §2 flags: neither `apl-runtime::eval::AplFn` nor
/// `j-runtime::eval::JFn` has ever needed to represent a named-parameter,
/// multi-statement, user-defined callable, since APL's/J's in-scope
/// grammars are expression-only (trains *recombine* existing primitives;
/// they never introduce a new parameter name a body can reference).
///
/// ## What "capture" means here (and why no snapshot is needed)
///
/// MA11 §5 describes this as capturing "the enclosing call's local-binding
/// environment at the point of definition" -- in this cut, that reduces to
/// something simple: since nested function *definitions* are explicitly out
/// of scope (MA11 §4; see [`Interpreter::build_lambda`]'s own guard against
/// them), every `Lambda` is always defined at the **top level**, and its
/// body only ever references top-level (global) variables plus its own
/// parameters. This evaluator therefore stores **no** captured environment
/// at all -- `params`/`body` are the complete representation -- and simply
/// resolves any non-parameter name against the *global* frame at call time
/// (via [`Interpreter`]'s ordinary two-tier lookup: call-local frame first,
/// global frame beneath it). Since no nested function literal can exist to
/// make "at the point of definition" and "at the point of call" diverge,
/// this is not an approximation for this cut's in-scope programs -- it is
/// the exact, correct behavior.
#[derive(Debug)]
pub struct Lambda {
    pub params: Vec<String>,
    /// Each element is a `statement` node (one child of the
    /// `function_literal`'s `stmt_seq`) -- owned clones out of the original
    /// parse tree (`GrammarASTNode: Clone`), since a [`Lambda`] must outlive
    /// the single `feed()` call that parsed it (variables, and therefore
    /// function values, persist across a REPL session's many `feed` calls).
    pub body: Vec<GrammarASTNode>,
}

/// This evaluator's own representation of a `verb_expr`: "which verb, and
/// with which adverb (if any) applied" -- generalizing `j-runtime::eval::JFn`
/// per MA11 §2's own explicit instruction, minus the train-shaped variants
/// Q has no equivalent of (see this module's own top doc comment).
enum QFn {
    /// A bare primitive glyph, applied directly.
    Prim(Prim),
    /// A primitive with `'` (each) applied. See
    /// [`Prim::each_monadic_supported`]/[`Prim::each_dyadic_supported`] for
    /// exactly which primitives this has a well-defined meaning for in this
    /// cut's flat, dense-array-only value model.
    Each(Prim),
    /// A `BinOp`-mappable primitive with `/` (reduce/over) applied --
    /// inherently monadic (`+/x` reduces the one array `x`).
    Reduce(BinOp),
    /// A `BinOp`-mappable primitive with `\` (scan) applied -- also
    /// monadic.
    Scan(BinOp),
    /// A user-defined function value (MA11 §2's headline novelty).
    Lambda(Rc<Lambda>),
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            env: RefCell::new(vec![HashMap::new()]),
            depth: Rc::new(Cell::new(0)),
        }
    }

    /// Enter one level of recursion, erroring if [`MAX_DEPTH`] is exceeded.
    /// The returned guard decrements the counter when it drops.
    fn enter(&self) -> Result<DepthGuard, String> {
        self.depth.set(self.depth.get() + 1);
        let guard = DepthGuard(Rc::clone(&self.depth));
        if self.depth.get() > MAX_DEPTH {
            return Err("q-runtime: expression nesting too deep".to_string());
        }
        Ok(guard)
    }

    /// Look up `name`, searching call-local frames innermost-first and
    /// falling back to the global frame (index 0) last -- a local binding
    /// *shadows* a global one of the same name for the duration of that
    /// call, MA11 §4's "local to that call only" scoping.
    fn lookup(&self, name: &str) -> Option<QValue> {
        let env = self.env.borrow();
        env.iter().rev().find_map(|frame| frame.get(name).cloned())
    }

    /// Bind `name` in the *current* frame -- the top of the stack, which is
    /// the global frame at the top level and a call-local frame inside a
    /// [`Lambda`] call. This single rule ("always write to the top frame")
    /// is what makes top-level assignment and in-body local assignment the
    /// same code path with no special-casing.
    fn assign(&self, name: &str, value: QValue) {
        let mut env = self.env.borrow_mut();
        let top = env.last_mut().expect("env always has at least the global frame");
        top.insert(name.to_string(), value);
    }

    /// Whether evaluation is currently nested inside at least one active
    /// [`Lambda`] call -- i.e. whether the environment stack holds more
    /// than just the global frame. Used by [`Interpreter::build_lambda`] to
    /// reject a *nested* function literal (MA11 §4: out of scope).
    fn inside_a_call(&self) -> bool {
        self.env.borrow().len() > 1
    }

    /// Evaluate a whole `program` node, returning the auto-print output for
    /// every statement that is *not* an assignment (MA11's own REPL/runtime
    /// convention, mirroring `j-runtime`'s/`apl-runtime`'s identical
    /// "assignment silent, bare expression auto-prints" behavior -- Q's own
    /// real console works the same way).
    pub fn run(&self, program: &GrammarASTNode) -> Result<String, String> {
        let mut out = String::new();
        for line in node_children(program) {
            if line.rule_name != "line" {
                continue;
            }
            let stmt = line.children.iter().find_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => Some(n),
                _ => None,
            });
            let Some(stmt) = stmt else { continue };
            let assignment = only_node(stmt)?;
            let is_assignment = assignment.children.len() == 3;
            let val = self.eval_assignment(assignment)?;
            if !is_assignment {
                out.push_str(&value::display(&val));
                out.push('\n');
            }
        }
        Ok(out)
    }

    /// `statement = assignment` -- a bare passthrough, always exactly one
    /// child. Evaluate it for its value, regardless of whether it happened
    /// to be an actual assignment (used by [`Interpreter::call_lambda`] to
    /// evaluate a `Lambda` body's statements in order without needing to
    /// separately decide "should this print", which only matters at the
    /// top level).
    fn eval_statement_value(&self, stmt: &GrammarASTNode) -> Result<QValue, String> {
        let assignment = only_node(stmt)?;
        self.eval_assignment(assignment)
    }

    /// `assignment = NAME COLON assignment | noun_expr`. A 3-child node
    /// (`[Token(NAME), Token(COLON), Node(assignment)]`) is a real
    /// assignment -- right-associative, so `x:y:3` binds `3` to *both* `y`
    /// and `x` (mirrors `j-runtime::eval::eval_assignment`'s identical
    /// chained-assignment shape). A 1-child node is a plain passthrough to
    /// `noun_expr`.
    fn eval_assignment(&self, node: &GrammarASTNode) -> Result<QValue, String> {
        let _guard = self.enter()?;
        if node.children.len() == 3 {
            let name = assignment_target_name(node)?;
            let inner = only_node(node)?;
            let value = self.eval_assignment(inner)?;
            self.assign(&name, value.clone());
            Ok(value)
        } else {
            let noun_expr = only_node(node)?;
            self.eval_noun_expr(noun_expr)
        }
    }

    /// `noun_expr = term [ verb_expr noun_expr | noun_expr ] | verb_expr noun_expr`.
    ///
    /// - 1 child `[Node(term)]` -- a bare term.
    /// - 3 children `[Node(term), Node(verb_expr), Node(noun_expr)]` --
    ///   ordinary dyadic application (`2+3`, or `x f y` for a NAME/lambda
    ///   `f`), right-recursive.
    /// - 2 children -- **ambiguous by count alone** (the one genuinely new
    ///   wrinkle `q.grammar` has that `j.grammar`/`apl.grammar` never
    ///   needed, see this module's own top doc comment and
    ///   `q-parser`'s README): `[Node(verb_expr), Node(noun_expr)]` is
    ///   ordinary monadic primitive application (`-5`); `[Node(term),
    ///   Node(noun_expr)]` is the genuinely new "apply a callable term"
    ///   fallback (`f 5`, or `{x*2} 5`). Disambiguated by inspecting
    ///   `kids[0].rule_name` -- exactly the check `q-parser`'s own grammar
    ///   file says a later pass (this one) needs to make.
    fn eval_noun_expr(&self, node: &GrammarASTNode) -> Result<QValue, String> {
        let _guard = self.enter()?;
        let kids = node_children(node);
        match kids.len() {
            1 => self.eval_term(kids[0]),
            2 => match kids[0].rule_name.as_str() {
                "verb_expr" => {
                    let f = self.parse_verb_expr(kids[0])?;
                    let arg = self.eval_noun_expr(kids[1])?;
                    self.apply_monadic(&f, &arg)
                }
                "term" => {
                    let callee = self.eval_term(kids[0])?;
                    let f = as_callable(callee)?;
                    let arg = self.eval_noun_expr(kids[1])?;
                    self.apply_monadic(&f, &arg)
                }
                other => Err(format!(
                    "q-runtime: malformed noun_expr (unexpected first child '{other}')"
                )),
            },
            3 => {
                let lhs = self.eval_term(kids[0])?;
                let f = self.parse_verb_expr(kids[1])?;
                let rhs = self.eval_noun_expr(kids[2])?;
                self.apply_dyadic(&f, &lhs, &rhs)
            }
            n => Err(format!("q-runtime: malformed noun_expr with {n} children")),
        }
    }

    /// `term = NUMBER { NUMBER } | NAME | function_literal | LPAREN noun_expr RPAREN | list_literal`.
    fn eval_term(&self, node: &GrammarASTNode) -> Result<QValue, String> {
        let _guard = self.enter()?;
        match node.children.first() {
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "NUMBER" => {
                // Stranding: one or more juxtaposed NUMBER tokens form a
                // single term -- `1 2 3` is one 3-element vector, a lone `5`
                // is a rank-0 scalar (MA11 §4, inherited unchanged from
                // APL/J). No grammar-level depth bound on the *count* of
                // stranded numbers (`term`'s repetition is flat, not
                // recursive), so it is capped here directly, mirroring
                // `j-runtime::eval::eval_term`'s own identical guard.
                if node.children.len() > builtins::MAX_ARRAY_LENGTH {
                    return Err(format!(
                        "q-runtime: stranded literal of {} numbers exceeds the cap of {} elements",
                        node.children.len(),
                        builtins::MAX_ARRAY_LENGTH
                    ));
                }
                let mut nums = Vec::new();
                for c in &node.children {
                    if let ASTNodeOrToken::Token(tok) = c {
                        nums.push(parse_q_number(&tok.value)?);
                    }
                }
                let arr = if nums.len() == 1 {
                    Array::scalar(nums[0])
                } else {
                    Array::from_vec(nums)
                };
                Ok(QValue::Arr(arr))
            }
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "NAME" => self
                .lookup(&t.value)
                .ok_or_else(|| format!("q-runtime: undefined variable '{}'", t.value)),
            Some(ASTNodeOrToken::Node(n)) if n.rule_name == "function_literal" => {
                Ok(QValue::Fn(self.build_lambda(n)?))
            }
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "LPAREN" => {
                let inner = only_node(node)?; // the Node(noun_expr) child
                self.eval_noun_expr(inner)
            }
            Some(ASTNodeOrToken::Node(n)) if n.rule_name == "list_literal" => {
                self.eval_list_literal(n)
            }
            _ => Err("q-runtime: malformed term".to_string()),
        }
    }

    /// `list_literal = LPAREN noun_expr SEMICOLON noun_expr { SEMICOLON noun_expr } RPAREN`
    /// (MA11 §3 bullet 3 / §4).
    ///
    /// Both list-literal syntaxes must "lower to the same list value" (MA11
    /// §3 bullet 3): for the case this actually reduces to -- every element
    /// a plain numeric scalar -- `(1;2;3)` produces the *identical* 3-element
    /// vector as stranding `1 2 3`, implemented here by simply collecting
    /// each element's scalar value.
    ///
    /// **Disclosed scope limit**: this cut's value model is "arrays only,
    /// dense and numeric" (MA11 §4) with *no* nested/boxed/heterogeneous
    /// list representation at all -- so a list literal containing a
    /// non-scalar element (itself a vector/matrix) or a function-valued
    /// element (`(2;{x+1};3)`, which `q-parser`'s own test suite confirms
    /// *parses* -- the grammar was built slightly ahead of this runtime's
    /// semantic scope, exactly as this task's own brief anticipates) has no
    /// representation this crate's dense-numeric `Array` can hold. Rather
    /// than silently flattening or truncating such a value, this is a
    /// clean, specific "not yet supported" error naming exactly why.
    fn eval_list_literal(&self, node: &GrammarASTNode) -> Result<QValue, String> {
        let _guard = self.enter()?;
        let elems = node_children(node);
        if elems.len() > builtins::MAX_ARRAY_LENGTH {
            return Err(format!(
                "q-runtime: a list literal of {} elements exceeds the cap of {} elements",
                elems.len(),
                builtins::MAX_ARRAY_LENGTH
            ));
        }
        let mut nums = Vec::with_capacity(elems.len());
        for e in elems {
            match self.eval_noun_expr(e)? {
                QValue::Arr(a) if a.is_scalar() => nums.push(a.data()[0]),
                QValue::Arr(_) => {
                    return Err(
                        "q-runtime: a list literal with a non-scalar element has no \
                         representation in this cut's dense-numeric-only value model (MA11 §4)"
                            .to_string(),
                    )
                }
                QValue::Fn(_) => {
                    return Err(
                        "q-runtime: a list literal containing a function-valued element has \
                         no representation in this cut's dense-numeric-only value model (MA11 §4)"
                            .to_string(),
                    )
                }
            }
        }
        Ok(QValue::Arr(Array::from_vec(nums)))
    }

    /// Build a [`Lambda`] from a `function_literal` node -- MA11 §2/§3
    /// bullet 1's headline novelty.
    ///
    /// **Rejects nested function literals** (MA11 §4: "no nested function
    /// literals to begin with", and the deferred list's "nested function
    /// definitions and their global/local scoping subtlety"): if this
    /// method is reached while already [`inside_a_call`](Self::inside_a_call)
    /// (evaluating a *previous* `Lambda`'s body), this is a function literal
    /// appearing textually inside another one's body -- exactly the shape
    /// `q.grammar` happens to parse (`term`'s own alternatives include
    /// `function_literal`, reachable from inside a `stmt_seq` too) but MA11
    /// §4 explicitly puts out of scope. This is rejected *unconditionally*,
    /// not just when the inner literal is actually called: even a nested
    /// literal that is merely constructed-and-returned (never invoked
    /// within the same call) would, if later invoked as an independent
    /// value, need real lexical closure capture to see the *outer* call's
    /// locals correctly -- and this crate deliberately captures none (see
    /// [`Lambda`]'s own doc comment) -- so allowing it to build at all
    /// would silently misrepresent Q's real (deferred) closure semantics
    /// rather than erroring cleanly, exactly the failure mode this task's
    /// brief warns against.
    fn build_lambda(&self, node: &GrammarASTNode) -> Result<Rc<Lambda>, String> {
        if self.inside_a_call() {
            return Err(
                "q-runtime: nested function literals are not supported in this cut (MA11 §4 -- \
                 every function body in scope calls only primitives and already-defined \
                 functions, with no nested function-literal definitions of its own)"
                    .to_string(),
            );
        }
        let param_list_node = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "param_list" => Some(n),
            _ => None,
        });
        let params: Vec<String> = match param_list_node {
            Some(pl) => {
                let names: Vec<String> = pl
                    .children
                    .iter()
                    .filter_map(|c| match c {
                        ASTNodeOrToken::Token(t) if t.effective_type_name() == "NAME" => {
                            Some(t.value.clone())
                        }
                        _ => None,
                    })
                    .collect();
                if names.is_empty() {
                    return Err(
                        "q-runtime: malformed function_literal (empty param_list)".to_string()
                    );
                }
                names
            }
            // The bracket-omitted implicit-parameter convenience (MA11 §3
            // bullet 1 / §4): no `[x;y]` at all defaults to the well-
            // documented `x`/`y`/`z` names. `q.grammar` never emits a
            // `param_list` node in this case (its own doc comment: "the
            // parse tree simply has no `param_list` child"), so an absent
            // node -- not an empty one -- is exactly the signal for this.
            None => vec!["x".to_string(), "y".to_string(), "z".to_string()],
        };
        let stmt_seq_node = node
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "stmt_seq" => Some(n),
                _ => None,
            })
            .ok_or_else(|| {
                "q-runtime: malformed function_literal (missing stmt_seq)".to_string()
            })?;
        let body: Vec<GrammarASTNode> = node_children(stmt_seq_node)
            .into_iter()
            .filter(|n| n.rule_name == "statement")
            .cloned()
            .collect();
        if body.is_empty() {
            return Err("q-runtime: malformed function_literal (empty body)".to_string());
        }
        Ok(Rc::new(Lambda { params, body }))
    }

    /// Call `lambda` with `args` (1 argument for a monadic application, 2
    /// for dyadic -- this grammar's call sites, `term noun_expr` and
    /// `term verb_expr noun_expr`, never produce more than two operands, so
    /// the well-documented implicit `z` third parameter is never actually
    /// reachable in practice; it is still honored by name for the
    /// 1-/2-argument cases MA11 §3 bullet 1 does describe).
    ///
    /// Binds `args` to `lambda.params` **positionally** (`args[0]` to
    /// `params[0]`, etc.) in a fresh call-local frame, pushed on top of the
    /// environment stack for the duration of the call (MA11 §4: "local to
    /// that call only") and popped again via [`FrameGuard`] on every exit
    /// path. Evaluates every body statement in order (the same
    /// [`eval_statement_value`](Self::eval_statement_value) top-level
    /// `run` uses, so an in-body assignment behaves identically to a
    /// top-level one except for *where* it writes -- the top of the
    /// stack, which is now this call's own frame, not the global one),
    /// returning the **last** statement's value as the call's result.
    fn call_lambda(&self, lambda: &Rc<Lambda>, args: Vec<QValue>) -> Result<QValue, String> {
        let _guard = self.enter()?;
        if args.len() > lambda.params.len() {
            return Err(format!(
                "q-runtime: function takes at most {} parameter(s), called with {}",
                lambda.params.len(),
                args.len()
            ));
        }
        let mut frame = HashMap::with_capacity(args.len());
        for (name, val) in lambda.params.iter().zip(args) {
            frame.insert(name.clone(), val);
        }
        self.env.borrow_mut().push(frame);
        let _frame_guard = FrameGuard { env: &self.env };

        let mut result = None;
        let last_index = lambda.body.len() - 1;
        for (i, stmt) in lambda.body.iter().enumerate() {
            let value = self.eval_statement_value(stmt)?;
            if i == last_index {
                result = Some(value);
            }
        }
        Ok(result.expect("build_lambda guarantees a non-empty body"))
    }

    /// `verb_expr = verb_primitive [ EACH | REDUCE | SCAN ] | NAME | function_literal`.
    /// Needs `&self` (unlike a hypothetical free function) because the
    /// `NAME` alternative requires a variable lookup, and `function_literal`
    /// requires [`Interpreter::build_lambda`]'s own nested-literal check.
    fn parse_verb_expr(&self, node: &GrammarASTNode) -> Result<QFn, String> {
        let _guard = self.enter()?;
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(prim)] if prim.rule_name == "verb_primitive" => {
                Ok(QFn::Prim(parse_verb_primitive(prim)?))
            }
            [ASTNodeOrToken::Node(prim), ASTNodeOrToken::Token(adverb)]
                if prim.rule_name == "verb_primitive" =>
            {
                let p = parse_verb_primitive(prim)?;
                match adverb.effective_type_name() {
                    "EACH" => Ok(QFn::Each(p)),
                    "REDUCE" => Ok(QFn::Reduce(require_scalar_binop(p, "/ (reduce)")?)),
                    "SCAN" => Ok(QFn::Scan(require_scalar_binop(p, "\\ (scan)")?)),
                    other => Err(format!("q-runtime: unexpected adverb token '{other}'")),
                }
            }
            [ASTNodeOrToken::Token(t)] if t.effective_type_name() == "NAME" => {
                let val = self
                    .lookup(&t.value)
                    .ok_or_else(|| format!("q-runtime: undefined variable '{}'", t.value))?;
                as_callable(val)
            }
            [ASTNodeOrToken::Node(fl)] if fl.rule_name == "function_literal" => {
                Ok(QFn::Lambda(self.build_lambda(fl)?))
            }
            _ => Err("q-runtime: malformed verb_expr".to_string()),
        }
    }

    /// Apply a monadic (one-argument) callable -- the single dispatch site
    /// for every `QFn` variant, `Lambda` included (see this module's own
    /// top doc comment).
    fn apply_monadic(&self, f: &QFn, y: &QValue) -> Result<QValue, String> {
        let _guard = self.enter()?;
        match f {
            QFn::Prim(p) => Ok(QValue::Arr(builtins::apply_monadic_prim(
                *p,
                as_array(y)?,
            )?)),
            QFn::Each(p) => {
                if !p.each_monadic_supported() {
                    return Err(format!(
                        "q-runtime: ' (each) has no well-defined per-element meaning for '{}' \
                         monadically in this cut's flat, dense-array-only value model (MA11 §4)",
                        p.glyph()
                    ));
                }
                Ok(QValue::Arr(builtins::apply_monadic_prim(*p, as_array(y)?)?))
            }
            QFn::Reduce(op) => Ok(QValue::Arr(ops::reduce(*op, as_array(y)?)?)),
            QFn::Scan(op) => Ok(QValue::Arr(ops::scan(*op, as_array(y)?)?)),
            QFn::Lambda(l) => self.call_lambda(l, vec![y.clone()]),
        }
    }

    /// Apply a dyadic (two-argument) callable.
    fn apply_dyadic(&self, f: &QFn, x: &QValue, y: &QValue) -> Result<QValue, String> {
        let _guard = self.enter()?;
        match f {
            QFn::Prim(p) => Ok(QValue::Arr(builtins::apply_dyadic_prim(
                *p,
                as_array(x)?,
                as_array(y)?,
            )?)),
            QFn::Each(p) => {
                if !p.each_dyadic_supported() {
                    return Err(format!(
                        "q-runtime: ' (each) has no well-defined per-element meaning for '{}' \
                         dyadically in this cut's flat, dense-array-only value model (MA11 §4)",
                        p.glyph()
                    ));
                }
                Ok(QValue::Arr(builtins::apply_dyadic_prim(
                    *p,
                    as_array(x)?,
                    as_array(y)?,
                )?))
            }
            QFn::Reduce(_) => Err(
                "q-runtime: / (reduce) takes exactly one operand, but was applied dyadically"
                    .to_string(),
            ),
            QFn::Scan(_) => Err(
                "q-runtime: \\ (scan) takes exactly one operand, but was applied dyadically"
                    .to_string(),
            ),
            QFn::Lambda(l) => self.call_lambda(l, vec![x.clone(), y.clone()]),
        }
    }
}

/// Require `v` to be a plain array, erroring cleanly (naming the fact that
/// it was a function value instead) rather than ever silently coercing one.
fn as_array(v: &QValue) -> Result<&Array, String> {
    match v {
        QValue::Arr(a) => Ok(a),
        QValue::Fn(_) => {
            Err("q-runtime: expected a plain array value, got a function value".to_string())
        }
    }
}

/// Require `v` to be callable, erroring cleanly (this is the one place that
/// decides whether a [`QValue`] just evaluated can play `verb_expr`'s role
/// -- see this module's own top doc comment's "Calling a function value"
/// section).
fn as_callable(v: QValue) -> Result<QFn, String> {
    match v {
        QValue::Fn(l) => Ok(QFn::Lambda(l)),
        QValue::Arr(_) => Err(
            "q-runtime: cannot apply a plain array value as a function (only a function \
             literal, or a name bound to one, can be applied via juxtaposition)"
                .to_string(),
        ),
    }
}

/// Parse one `NUMBER` token's source text into an `f64`. Unlike J (whose
/// `NUMBER` folds a leading underscore into ASCII `-` before parsing), a
/// Q `NUMBER` token's own value is already plain, standard `f64` syntax --
/// `q-lexer`'s `fold_negative_number_literals` post-tokenize hook already
/// prepends an ordinary ASCII `-` directly onto the token's `value` string
/// when a negative literal is recognized (MA11 §3 bullet 2), so no
/// translation is needed here at all; Rust's own `f64::from_str` parses the
/// result unchanged.
fn parse_q_number(s: &str) -> Result<f64, String> {
    s.parse::<f64>()
        .map_err(|_| format!("q-runtime: invalid number literal '{s}'"))
}

/// `verb_primitive`: always exactly one child, a single token naming the
/// primitive glyph.
fn parse_verb_primitive(node: &GrammarASTNode) -> Result<Prim, String> {
    let tok = match node.children.first() {
        Some(ASTNodeOrToken::Token(t)) => t,
        _ => return Err("q-runtime: malformed verb_primitive".to_string()),
    };
    Ok(match tok.effective_type_name() {
        "PLUS" => Prim::Plus,
        "MINUS" => Prim::Minus,
        "STAR" => Prim::Star,
        "PERCENT" => Prim::Percent,
        "BANG" => Prim::Bang,
        "COMMA" => Prim::Comma,
        "HASH" => Prim::Hash,
        "UNDERSCORE" => Prim::Underscore,
        "AMP" => Prim::Amp,
        "PIPE" => Prim::Pipe,
        "TILDE" => Prim::Tilde,
        "EQ" => Prim::Eq,
        "LT" => Prim::Lt,
        "GT" => Prim::Gt,
        "LE" => Prim::Le,
        "GE" => Prim::Ge,
        "NE" => Prim::Ne,
        other => return Err(format!("q-runtime: unknown verb primitive token '{other}'")),
    })
}

/// Reduce/scan apply only to the 12 primitives that map onto a `BinOp` --
/// `!`/`,`/`#`/`_`/`~` are not "a scalar dyadic function" at all, so
/// stacking an adverb onto one of them is a clean, explicit scope error
/// (mirrors `j-runtime::eval::require_scalar_binop` exactly, generalized to
/// Q's own primitive set).
fn require_scalar_binop(p: Prim, context: &str) -> Result<BinOp, String> {
    p.to_binop()
        .ok_or_else(|| format!("q-runtime: {context}: {} is not a scalar dyadic verb", p.glyph()))
}

/// The `NAME` token of an actual assignment's target -- the first child of
/// a 3-child `assignment` node (`[Token(NAME), Token(COLON), Node(assignment)]`).
fn assignment_target_name(node: &GrammarASTNode) -> Result<String, String> {
    match node.children.first() {
        Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "NAME" => Ok(t.value.clone()),
        _ => Err("q-runtime: malformed assignment (missing target name)".to_string()),
    }
}

// ── AST helpers ──────────────────────────────────────────────────────────────

fn node_children(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

fn first_node(node: &GrammarASTNode) -> Option<&GrammarASTNode> {
    node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(n) => Some(n),
        ASTNodeOrToken::Token(_) => None,
    })
}

fn only_node(node: &GrammarASTNode) -> Result<&GrammarASTNode, String> {
    first_node(node).ok_or_else(|| format!("q-runtime: malformed '{}' node", node.rule_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A direct, white-box unit test of the depth guard mechanism itself
    /// (`enter`/`DepthGuard`) -- mirrors
    /// `j-runtime::eval::tests::depth_guard_trips_after_max_depth_and_recovers`
    /// exactly. Unlike J's guard (never reachable through genuine parsed
    /// input, per that module's own doc comment), this crate's `MAX_DEPTH`
    /// *is* reachable via a long enough legitimate call chain (see
    /// `MAX_DEPTH`'s own doc comment) -- this test still exercises the
    /// guard mechanism directly, since driving it via 512+ distinct
    /// top-level function definitions in one test would be needlessly
    /// slow and indirect for what is fundamentally a unit test of `enter()`
    /// itself.
    #[test]
    fn depth_guard_trips_after_max_depth_and_recovers() {
        let interp = Interpreter::new();
        let mut guards = Vec::new();
        for _ in 0..MAX_DEPTH {
            guards.push(interp.enter().expect("should stay under the cap"));
        }
        assert!(interp.enter().is_err(), "one more enter() must trip the cap");
        drop(guards);
        assert!(
            interp.enter().is_ok(),
            "dropping every guard must let a fresh enter() succeed again"
        );
    }

    /// A long (but legitimate) chain of already-defined-function calls must
    /// still work -- confirms `MAX_DEPTH` (512) leaves comfortable headroom
    /// for realistic, non-adversarial call chains, not just single
    /// expressions.
    ///
    /// Each link calls the previous function with an explicitly
    /// parenthesised argument (`f98(x+1)`, not `f98 x+1`) -- without the
    /// parens, `q.grammar`'s own "which bare NAME plays the verb role"
    /// resolution (see this module's own top doc comment and
    /// `q-parser`'s README) would read a bare trailing NAME/expression
    /// pair like `f98 x+1` as `x` itself being the (dyadic) callable
    /// applied to `(f98, 1)`, not as "call f98 with argument x+1" -- a
    /// real, disclosed grammar-level ambiguity this test sidesteps the
    /// documented way (parenthesise the argument), not a bug in this
    /// evaluator.
    #[test]
    fn a_reasonably_long_call_chain_of_already_defined_functions_works() {
        let depth = 100;
        let src = chained_call_source(depth);
        let interp = Interpreter::new();
        let tree = coding_adventures_q_parser::try_parse_q(&src).expect("should parse");
        let out = interp.run(&tree).expect("a 100-deep call chain should not trip MAX_DEPTH");
        // f0(0)=1, f1(0)=f0(0+1)=f0(1)=2, ..., f_k(0) = k+1.
        assert_eq!(out.trim(), depth.to_string());
    }

    /// A minimal placeholder `GrammarASTNode` -- its own content is
    /// irrelevant to the test below, since [`Interpreter::eval_list_literal`]'s
    /// element-count cap check happens on `node_children(node).len()`
    /// *before* evaluating a single element (see that method's own
    /// implementation), so a `list_literal` node's children never need to
    /// be *semantically* valid `noun_expr`s to exercise this guard.
    fn placeholder_node(rule_name: &str) -> GrammarASTNode {
        GrammarASTNode {
            rule_name: rule_name.to_string(),
            children: Vec::new(),
            start_line: None,
            start_column: None,
            end_line: None,
            end_column: None,
        }
    }

    /// Direct, white-box regression test for `list_literal`'s own element-
    /// count cap, constructing the tree by hand rather than through real
    /// source text.
    ///
    /// A genuine `try_parse_q` call over 1,000,001 semicolon-separated
    /// elements is *drastically* slower than every other DoS-guard test in
    /// this crate (tens of seconds) -- `q.grammar`'s `list_literal`
    /// production has no cap on its own flat repetition *width* (only
    /// `q-parser`'s `MAX_RULE_DEPTH` bounds nesting *depth*, MA11 §6), so
    /// parsing that many elements pays real packrat-parser overhead this
    /// evaluator has no control over. Building the
    /// [`GrammarASTNode`] tree directly sidesteps that entirely -- this is
    /// a test of `eval_list_literal`'s own cap check, not of `q-parser`'s
    /// scalability, so there is no need to route it through a real parse
    /// at all.
    #[test]
    fn list_literal_rejects_an_oversized_element_count_before_evaluating_any_element() {
        let interp = Interpreter::new();
        let n = builtins::MAX_ARRAY_LENGTH + 1;
        let dummy = ASTNodeOrToken::Node(placeholder_node("noun_expr"));
        let node = GrammarASTNode {
            rule_name: "list_literal".to_string(),
            children: vec![dummy; n],
            start_line: None,
            start_column: None,
            end_line: None,
            end_column: None,
        };
        assert!(
            interp.eval_list_literal(&node).is_err(),
            "a list literal of {n} elements must be rejected before evaluating any of them"
        );
    }

    /// Build a chained-function-call Q source of `n` links: `f0:{x+1}`,
    /// `f1:{f0(x+1)}`, ..., `f{n-1}:{f{n-2}(x+1)}`, followed by a call to
    /// the last one -- the exact recursion shape `MAX_DEPTH` was measured
    /// against (see that constant's own doc comment for the full
    /// methodology and results).
    fn chained_call_source(n: usize) -> String {
        let mut src = String::from("f0:{x+1}\n");
        for i in 1..n {
            let prev = i - 1;
            src.push_str(&format!("f{i}:{{f{prev}(x+1)}}\n"));
        }
        src.push_str(&format!("f{}(0)\n", n - 1));
        src
    }

    /// Real (not white-box/synthetic) `call_lambda`-driven recursion,
    /// exercised right up to `MAX_DEPTH`'s own measured boundary and one
    /// step past it, run on an **explicit, known 2 MiB stack** -- the same
    /// stack size `MAX_DEPTH`'s own calibration measurement used (see that
    /// constant's doc comment) -- rather than relying on the ambient
    /// default (which `RUST_MIN_STACK` could silently enlarge, invalidating
    /// the comparison; see this repo's own
    /// `feedback_rust_min_stack_pollutes_default_stack_probes` lesson).
    ///
    /// 189 chained calls succeed outright; 190 hits `MAX_DEPTH` and returns
    /// a clean `Err` (a `join()` that returns `Ok` from the worker thread,
    /// not a panicked/aborted thread) -- proving the *guard*, not the
    /// *native stack*, is what stops it, with the real crash floor (271
    /// chained calls on this same 2 MiB stack, measured empirically) still
    /// 81 calls further out.
    #[test]
    fn real_recursion_up_to_max_depth_succeeds_one_past_it_errors_cleanly_on_a_known_stack() {
        const STACK_BYTES: usize = 2 * 1024 * 1024;

        let run_chain = |n: usize| -> Result<String, String> {
            std::thread::Builder::new()
                .stack_size(STACK_BYTES)
                .spawn(move || {
                    let src = chained_call_source(n);
                    let interp = Interpreter::new();
                    let tree = coding_adventures_q_parser::try_parse_q(&src)
                        .expect("should parse");
                    interp.run(&tree)
                })
                .expect("failed to spawn worker thread")
                .join()
                .expect("worker thread panicked/aborted -- this must never happen at these depths")
        };

        assert!(
            run_chain(189).is_ok(),
            "189 chained calls must succeed comfortably under MAX_DEPTH on a 2 MiB stack"
        );
        let err = run_chain(190)
            .expect_err("190 chained calls must trip MAX_DEPTH's guard, not silently succeed");
        assert!(
            err.contains("too deep"),
            "expected the depth-guard's own error message, got: {err}"
        );
    }
}
