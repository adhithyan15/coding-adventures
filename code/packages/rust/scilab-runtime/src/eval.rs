//! The tree-walking evaluator.
//!
//! [`Interpreter`] walks the [`GrammarASTNode`] tree from `scilab-parser` and
//! computes [`ScilabValue`]s over `array-runtime`, mirroring
//! `matlab-runtime::eval`'s overall shape (MA10 §5: same control-flow/
//! indexing/precedence-cascade pattern, MATLAB-family) with four genuine
//! additions matlab-runtime never needed: `select`/`case`, `break`/
//! `continue`, user-defined `function`/`endfunction` (with multiple return
//! values), and `$`-based last-index resolution.
//!
//! ## Structural deviation from `matlab-runtime`: no `Rc`, and `&mut self`
//! throughout
//!
//! `matlab_runtime::eval::Interpreter` threads its recursion-depth counter as
//! `Rc<Cell<usize>>` (an owned, cloneable handle a RAII `DepthGuard` can hold
//! without borrowing `&self`) and its expression-evaluating methods all take
//! `&self`, because MATLAB-runtime never needs to *mutate* the variable
//! workspace anywhere below the one top-level assignment method. Scilab
//! genuinely does: a user-defined function call can appear anywhere inside an
//! expression (`y = f(x) + 1`), and calling one requires temporarily
//! swapping in a fresh local workspace (see [`Interpreter::call_user_function`]).
//! So every evaluating method here takes `&mut self` instead.
//!
//! This crate also runs each [`Interpreter::run`] call on a dedicated worker
//! thread inside `catch_unwind` (see the crate doc comment in `lib.rs`,
//! following `maple-runtime`'s more rigorous pattern rather than
//! `matlab-runtime`'s simpler, older one) — which requires `Interpreter` to
//! be `Send`. `Rc<T>` is *never* `Send`, regardless of `T`, so keeping
//! `matlab-runtime`'s `Rc<Cell<usize>>` shape verbatim would have made this
//! whole crate impossible to run on a worker thread. [`Interpreter::depth`]
//! is therefore a plain `usize` field (no `Cell`, no `Rc` at all), incremented
//! and decremented by [`Interpreter::guarded`] rather than by an RAII
//! `DepthGuard` the way `matlab_runtime::eval::DepthGuard` does — an early
//! design that instead paired `Cell<usize>` with an RAII guard ran straight
//! into a *different* problem, not a `Send` one: the guard's borrow of
//! `&Cell<usize>` had to stay alive across the very `&mut self` recursive
//! call it was meant to bound, which the borrow checker rejects outright
//! (see [`Interpreter::depth`]'s own doc comment for the full account). The
//! one field that genuinely still needs `Cell` is
//! [`Interpreter::dollar_value`] (read/written via only a momentary
//! reborrow, never held across a call), which stays `Cell<Option<f64>>` —
//! Send exactly the same way `Cell<usize>` would have been, just without the
//! borrow-checker conflict `depth`'s own RAII-guard shape ran into.

use crate::builtins;
use crate::value::{echo, ScilabValue};
use array_runtime::{execute, ops, Array, Kernel};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use std::cell::Cell;
use std::collections::HashMap;

/// Maximum expression/block/**function-call** nesting depth. Crafted input
/// like `((((…))))` or `if…if…end…end` recurses one frame per level, exactly
/// as in `matlab-runtime` — but Scilab adds a genuinely new native-recursion
/// vector `matlab-runtime` never has at all: a **user-defined function
/// calling itself** (`function y=f(n) ... y=f(n-1) ... endfunction`) recurses
/// through the *entire* expression cascade plus one `eval_block` frame on
/// every call, not just through nested *source* the parser's own
/// `MAX_RULE_DEPTH` already bounds. This bound turns either kind of runaway
/// recursion (deeply nested source OR deeply recursive function calls) into
/// a clean `Err` instead of a native stack overflow (which — unlike an
/// ordinary panic — `catch_unwind` in `lib.rs` *cannot* catch: overflowing
/// the real machine stack aborts the process directly, no unwinding
/// possible; this cooperative counter is what actually prevents that, not
/// the worker thread's panic handling).
///
/// Set generously (2000, versus `matlab-runtime`'s 512) relative to the
/// worker thread's 512 MiB stack (`crate::EVAL_STACK_SIZE`) specifically to
/// give a hand-written *recursive* Scilab function (factorial, Fibonacci,
/// Ackermann-for-small-inputs, ...) real headroom — each level of Scilab-level
/// recursion cascades through roughly fifteen to twenty `eval_node`/
/// `eval_block` frames here (the full `assignment -> ... -> postfix` tier
/// list, per call), so 2000 gives on the order of 100+ levels of genuine
/// function-call recursion, comfortably beyond anything a textbook session
/// writes by hand, while 2000 native frames is nowhere near enough to
/// threaten a 512 MiB stack even at a generous few-KiB-per-frame debug-build
/// estimate. Verified empirically (not just estimated) by this crate's own
/// `runaway_recursion_is_a_clean_error_not_a_crash` test, which runs on a
/// worker thread with the **default** (~2 MiB, unboosted) stack size and
/// confirms the cap trips before that smaller stack would ever overflow.
const MAX_DEPTH: usize = 2000;

/// A user-defined function: its parameter names, its declared output
/// (return) variable name(s) — zero, one, or many, MA10's
/// `function [y1,...,yn] = f(...) ... endfunction` surface — and its body,
/// **cloned** out of whatever (transient, per-`run`-call) parse tree defined
/// it. Cloning is required because `GrammarASTNode` only borrows from the one
/// `program` tree `try_parse_scilab` hands back for a single `feed` call;
/// without cloning, a function defined in one `feed` call could not survive
/// to be called from a *later* one, breaking the "functions persist across
/// calls, exactly like variables" contract `Interpreter::feed` gives every
/// other binding.
#[derive(Clone)]
struct FunctionDef {
    params: Vec<String>,
    returns: Vec<String>,
    body: GrammarASTNode,
}

/// What a statement/block's execution should do next.
///
/// `break`/`continue` are ordinary **control values** threaded as an explicit
/// return, not exceptions/panics — ordinary `Result` already carries genuine
/// errors, and `break`/`continue` are not errors (`matlab-runtime` has no
/// analogue at all, since MATLAB support in that crate never got as far as
/// loops with early-exit). Every statement/block-evaluating method below
/// returns `Result<Flow, String>` (or wraps one): `if`/`select` propagate
/// whichever `Flow` their taken branch produced straight up to their own
/// caller — the same "in a `Result<T,E>`'s error slot, propagate with `?`
/// until something means to handle it" shape, just with `Flow` riding in the
/// success slot instead, since `break`/`continue` are not failures. Only
/// `while`/`for` (the nearest enclosing loop) — and, at the top of a
/// function's own body or a whole program, an *absent* enclosing loop —
/// actually interpret a non-`Normal` `Flow` rather than merely forwarding it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Flow {
    Normal,
    Break,
    Continue,
}

/// A persistent Scilab session: a variable workspace, the user-defined
/// functions registered so far, the value `$` currently resolves to (set
/// while evaluating one indexing argument), and the current evaluation
/// depth.
pub struct Interpreter {
    vars: HashMap<String, ScilabValue>,
    functions: HashMap<String, FunctionDef>,
    /// The value `$` resolves to while evaluating exactly one indexing
    /// argument — see [`Interpreter::eval_call_args`]'s own doc comment for
    /// the full resolution mechanism and why a plain set-then-clear (no
    /// save/restore stack) is safe here. `Cell` (not a plain field) because
    /// it is read/written from deep inside a long `&mut self` recursive call
    /// chain via only a momentary reborrow (see that method), which never
    /// needs to *hold* a borrow the way the depth counter's own guard would.
    dollar_value: Cell<Option<f64>>,
    /// The current expression/block/function-call recursion depth — see
    /// [`Interpreter::guarded`] for how it is incremented/checked/
    /// decremented. A plain `usize`, **not** `Cell<usize>`: unlike
    /// `dollar_value` above, this needs no interior mutability at all, since
    /// every evaluating method here already takes `&mut self` (the module
    /// doc comment's own "structural deviation from `matlab-runtime`"
    /// section explains why) — reaching for `Cell` here anyway (as a naive
    /// port of `matlab_runtime::eval::Interpreter::end_value`'s shape might
    /// suggest) turned out to actively fight the borrow checker: an earlier
    /// draft paired it with an RAII `DepthGuard` holding `&Cell<usize>`
    /// across the recursive `&mut self` call it was meant to guard, which
    /// the borrow checker correctly rejects (a live shared borrow of one
    /// field is treated as a live borrow of the *whole* struct across a
    /// method call, since the checker cannot see inside `guarded` to know it
    /// only touches `self.depth`). [`Interpreter::guarded`]'s
    /// closure-wrapping shape sidesteps this entirely: no borrow is ever
    /// *held* across the wrapped call, so there is nothing for the checker
    /// to object to, and the decrement still runs on every exit path
    /// (including an early `?`) because it happens as ordinary code
    /// immediately after the inner closure call returns, not via `Drop`.
    depth: usize,
}

/// One index argument: a value, or a bare colon meaning "the whole
/// dimension".
enum Index {
    Colon,
    At(ScilabValue),
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            vars: HashMap::new(),
            functions: HashMap::new(),
            dollar_value: Cell::new(None),
            depth: 0,
        }
    }

    /// Run `f` one recursion level deeper, erroring (without running `f` at
    /// all) if that would exceed [`MAX_DEPTH`]. The counter is decremented
    /// again immediately after `f` returns — on **every** exit path,
    /// including an early `?` inside `f`'s own body, since that only unwinds
    /// out of the closure, not out of this function (see
    /// [`Interpreter::depth`]'s own doc comment for why this shape was
    /// chosen over an RAII guard). Every recursive entry point in this file
    /// (`eval_node`, `eval_block`) is wrapped in exactly one `guarded` call,
    /// mirroring how `matlab_runtime::eval::Interpreter::enter` is called
    /// from exactly its two analogous entry points.
    fn guarded<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, String>,
    ) -> Result<T, String> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            self.depth -= 1;
            return Err(
                "scilab-runtime: expression, block, or function-call nesting too deep"
                    .to_string(),
            );
        }
        let result = f(self);
        self.depth -= 1;
        result
    }

    /// Evaluate a whole program (the `program` node), returning the prompt
    /// echo for every statement whose result is *not* suppressed by a
    /// trailing `;`.
    ///
    /// Runs in **two passes**, mirroring `matlab-to-semantic-ir`'s own
    /// `collect_function_names` + `lower_file` two-pass structure: pass 1
    /// registers every top-level `function ... endfunction` definition in
    /// this program (so a statement can call a function defined *later* in
    /// the same script/session — the ordinary "functions are usually at the
    /// bottom of the file" Scilab/MATLAB convention); pass 2 executes every
    /// *other* top-level statement in source order, skipping `func_def`
    /// lines (already handled). A function's own body, once registered,
    /// executes later via a call — never as a side effect of merely being
    /// defined.
    pub fn run(&mut self, program: &GrammarASTNode) -> Result<String, String> {
        for line in node_children(program) {
            if line.rule_name != "statement_line" {
                continue;
            }
            if let Some(def) = first_node(line)
                .filter(|n| n.rule_name == "statement")
                .and_then(first_node)
                .filter(|n| n.rule_name == "func_def")
            {
                let (name, function) = self.register_function(def)?;
                self.functions.insert(name, function);
            }
        }

        let mut out = String::new();
        for line in node_children(program) {
            if line.rule_name != "statement_line" {
                continue;
            }
            if is_func_def_line(line) {
                continue; // already registered in pass 1
            }
            let (flow, text) = self.eval_statement_line(line)?;
            if flow != Flow::Normal {
                let word = if flow == Flow::Break { "break" } else { "continue" };
                return Err(format!("scilab-runtime: '{word}' used outside a loop"));
            }
            if let Some(text) = text {
                out.push_str(&text);
                out.push('\n');
            }
        }
        Ok(out)
    }

    /// Evaluate one statement line. Returns `(Flow::Normal, Some(echo))` for
    /// a visible result, `(Flow::Normal, None)` when suppressed (`;`) or the
    /// line is just a separator, or a non-`Normal` `Flow` when the statement
    /// is (or contains, via `if`/`select`) a `break`/`continue`.
    fn eval_statement_line(
        &mut self,
        line: &GrammarASTNode,
    ) -> Result<(Flow, Option<String>), String> {
        let stmt = match first_node(line) {
            Some(n) if n.rule_name == "statement" => n,
            _ => return Ok((Flow::Normal, None)), // a bare terminator / blank line
        };
        // A trailing `;` suppresses display; a newline or comma shows it.
        let suppressed = line.children.iter().any(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "stmt_term" => n
                .children
                .iter()
                .any(|t| matches!(t, ASTNodeOrToken::Token(t) if t.value == ";")),
            ASTNodeOrToken::Token(t) => t.value == ";",
            _ => false,
        });

        let inner = only_node(stmt)?;
        match inner.rule_name.as_str() {
            // Deliberately NO "func_def" arm here: `run`'s pass 2 skips
            // every top-level `func_def` line (via `is_func_def_line`)
            // BEFORE ever calling this method, since pass 1 already
            // registered it — so a `func_def` reaching this match can only
            // be a NESTED one (MA10 §4 never lists nested function
            // definitions in scope), and correctly falls through to the
            // `other => Err(...)` catch-all below: a clean, honest
            // rejection, not a silent no-op.
            "if_stmt" => Ok((self.eval_if(inner)?, None)),
            "select_stmt" => Ok((self.eval_select(inner)?, None)),
            "for_stmt" => Ok((self.eval_for(inner)?, None)),
            "while_stmt" => Ok((self.eval_while(inner)?, None)),
            "break_stmt" => Ok((Flow::Break, None)),
            "continue_stmt" => Ok((Flow::Continue, None)),
            "expr" => {
                let bindings = self.eval_expr_or_assign(inner)?;
                if suppressed {
                    Ok((Flow::Normal, None))
                } else {
                    // Ordinarily exactly one binding; a multi-return
                    // destructuring assignment (`[a,b] = f(x)`) echoes one
                    // line per bound name, joined here into one text block
                    // for this one statement line (mirrors real MATLAB's own
                    // `[a,b] = size(x)` echoing both `a =` and `b =`).
                    let mut text = String::new();
                    for (name, value) in &bindings {
                        text.push_str(&echo(name, value));
                        text.push('\n');
                    }
                    text.pop(); // drop the final newline; `run` appends its own
                    Ok((Flow::Normal, Some(text)))
                }
            }
            other => Err(format!("scilab-runtime: unsupported statement '{other}'")),
        }
    }

    /// Evaluate an `expr`. If it is a top-level assignment `lhs = rhs`, bind
    /// the variable(s) and report every `(name, value)` bound (more than one
    /// only for the multi-return `[a,b] = f(x)` shape); otherwise evaluate it
    /// and report `ans` (also binding `ans`, as MATLAB/Scilab both do).
    fn eval_expr_or_assign(
        &mut self,
        expr: &GrammarASTNode,
    ) -> Result<Vec<(String, ScilabValue)>, String> {
        let assignment = only_node(expr)?; // expr = assignment
        let kids = node_children(assignment); // assignment = logical_or [ EQ assignment ]
        let has_eq = assignment
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "="));
        if has_eq && kids.len() == 2 {
            // `[a, b] = f(x)` — MA10's multiple-return surface. The grammar
            // has no dedicated destructuring-pattern production of its own:
            // `[a, b]` parses as an ordinary `matrix_literal` of bare-NAME
            // elements (the identical bracket-list-of-names shape
            // `func_returns`'s own `LBRACKET [name_list] RBRACKET EQ` form
            // uses on the *definition* side). Detected here, at the
            // assignment-target level, by inspecting the LHS's own shape —
            // exactly the same kind of runtime-level judgment `lhs_name`'s
            // "is this a bare variable name?" check already makes for the
            // single-return case, just one layer further out.
            if let Some(names) = lhs_name_list(kids[0]) {
                let values = self.eval_multi(kids[1], names.len())?;
                let mut bound = Vec::with_capacity(names.len());
                for (name, value) in names.into_iter().zip(values) {
                    self.vars.insert(name.clone(), value.clone());
                    bound.push((name, value));
                }
                return Ok(bound);
            }
            // The ordinary single-target case: the LHS must be a plain
            // variable name (indexed assignment, `A(i) = ...`, is a
            // documented deferral — matlab-runtime has the identical gap).
            let name = lhs_name(kids[0]).ok_or_else(|| {
                "scilab-runtime: assignment target must be a variable".to_string()
            })?;
            let value = self.eval_node(kids[1])?;
            self.vars.insert(name.clone(), value.clone());
            Ok(vec![(name, value)])
        } else {
            let value = self.eval_node(assignment)?;
            self.vars.insert("ans".to_string(), value.clone());
            Ok(vec![("ans".to_string(), value)])
        }
    }

    /// Evaluate the right-hand side of a multi-return assignment
    /// (`[a,...] = <rhs>`). MA10 §4's multi-return surface is exactly "call a
    /// user-defined function with the matching number of declared outputs" —
    /// there is no other construct in this cut that ever produces more than
    /// one value at once (every builtin in `builtins.rs` is single-valued),
    /// so an RHS that is not a direct `NAME(...)` call is a clean error
    /// rather than a silent single-value substitution.
    fn eval_multi(&mut self, node: &GrammarASTNode, want: usize) -> Result<Vec<ScilabValue>, String> {
        let postfix = find_call_postfix(node).ok_or_else(|| {
            "scilab-runtime: a multiple-return assignment's right-hand side must be a direct \
             function call"
                .to_string()
        })?;
        let (name, call_suffix) = bare_call(postfix).ok_or_else(|| {
            "scilab-runtime: a multiple-return assignment's right-hand side must be a direct \
             function call"
                .to_string()
        })?;
        let def = self
            .functions
            .get(&name)
            .cloned()
            .ok_or_else(|| format!("scilab-runtime: '{name}' is not a known function"))?;
        if def.returns.len() != want {
            return Err(format!(
                "scilab-runtime: '{name}' declares {} output(s), but {want} were requested",
                def.returns.len()
            ));
        }
        let args = self.eval_call_args(call_suffix, None)?;
        let args = values_only(args)?;
        self.call_user_function(&def, args)
    }

    /// Evaluate any expression node, dispatching by rule name.
    fn eval_node(&mut self, node: &GrammarASTNode) -> Result<ScilabValue, String> {
        self.guarded(|this| match node.rule_name.as_str() {
            "expr" | "assignment" | "statement" => this.eval_node(only_node(node)?),
            "logical_or" | "logical_and" | "bit_or" | "bit_and" | "comparison" | "additive"
            | "multiplicative" => this.eval_binary_chain(node),
            "colon_expr" => this.eval_colon(node),
            "unary" => this.eval_unary(node),
            "power" => this.eval_power(node),
            "postfix" => this.eval_postfix(node),
            "primary" => this.eval_primary(node),
            "group" => this.eval_node(only_node(node)?),
            "matrix_literal" => this.eval_matrix(node),
            other => Err(format!("scilab-runtime: cannot evaluate '{other}'")),
        })
    }

    /// Left-associative fold of a binary-operator chain (`a op b op c …`).
    ///
    /// **The flat-chain-recursion vector, closed by construction**: this is a
    /// plain iterative loop over `node.children` accumulating a running
    /// total (a `for`/`fold` shape, not a self-recursive
    /// `eval_binary_chain(&operands[1..])`-style helper) — so a
    /// `scilab-parser`-produced `additive`/`multiplicative`/etc. node with
    /// *thousands* of flat operands (from source like `1+1+1+...+1`, which
    /// `scilab-parser`'s own `MAX_RULE_DEPTH` doc comment confirms parses
    /// with *zero* added parser rule-frames, since `{ x }` EBNF repetition is
    /// a flat loop at parse time too) costs **O(1) native stack** here,
    /// regardless of operand count. Confirmed by direct inspection of
    /// `matlab_runtime::eval::Interpreter::eval_binary_chain`, which already
    /// uses this identical iterative shape for MATLAB's own identically-flat
    /// chain productions — `scilab-runtime` inherits the same safety
    /// property for free by mirroring that shape here, not by adding a
    /// separate token-count guard the way `maple-runtime`/`reduce-runtime`/
    /// `derive-runtime` must for *their* languages (where the analogous
    /// lowering step *does* recursively fold a flat chain into a nested
    /// tree — confirmed NOT to be the case here; see this crate's own
    /// `long_flat_arithmetic_chain_evaluates_without_a_dedicated_guard`
    /// test). Every other flat `{ x }`-repetition production this grammar
    /// has (`{ elseif_clause }`, `{ case_clause }`, `arg_list`, `name_list`,
    /// `matrix_rows`, `program`, `block_body`) is, by the same direct-
    /// inspection standard, *also* walked by a plain `for` loop somewhere in
    /// this file (`eval_if`/`eval_select`'s own `while i < kids.len()`
    /// loops, `eval_call_args`'s `for (i, arg) in args.iter().enumerate()`,
    /// `register_function`'s `.filter_map(...).collect()`, `eval_matrix`'s
    /// nested `for row`/`for el` loops, `run`/`eval_block`'s own `for line`
    /// loops) — none of them fold their operand list into a recursive helper
    /// either.
    fn eval_binary_chain(&mut self, node: &GrammarASTNode) -> Result<ScilabValue, String> {
        let mut acc: Option<ScilabValue> = None;
        let mut op: Option<String> = None;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Node(n) => {
                    let v = self.eval_node(n)?;
                    acc = Some(match (acc.take(), op.take()) {
                        (None, _) => v,
                        (Some(a), Some(o)) => apply_binop(&o, &a, &v)?,
                        (Some(a), None) => a,
                    });
                }
                ASTNodeOrToken::Token(t) => op = Some(t.value.clone()),
            }
        }
        acc.ok_or_else(|| "scilab-runtime: empty expression".to_string())
    }

    /// The colon: `a:b` or `a:b:c`. Builds an inclusive numeric row vector —
    /// identical rule to MATLAB's own (MA10 §1's closing paragraph: ranges
    /// are inherited unchanged).
    fn eval_colon(&mut self, node: &GrammarASTNode) -> Result<ScilabValue, String> {
        let parts = node_children(node);
        if parts.len() == 1 {
            return self.eval_node(parts[0]);
        }
        let from_node;
        let step_node;
        let to_node;
        match parts.as_slice() {
            [a, b] => {
                from_node = *a;
                step_node = None;
                to_node = *b;
            }
            [a, s, b] => {
                from_node = *a;
                step_node = Some(*s);
                to_node = *b;
            }
            _ => return Err("range: too many colons".to_string()),
        }
        let from = self.eval_node(from_node)?;
        let from = scalar_of(&from, "range")?;
        let step = match step_node {
            Some(s) => {
                let v = self.eval_node(s)?;
                scalar_of(&v, "range")?
            }
            None => 1.0,
        };
        let to = self.eval_node(to_node)?;
        let to = scalar_of(&to, "range")?;
        if step == 0.0 {
            return Err("range: step cannot be zero".to_string());
        }
        let mut data = Vec::new();
        let mut x = from;
        // Bound the length so a crafted `1:1e18` can't exhaust memory.
        const MAX_RANGE: usize = 1 << 26;
        while (step > 0.0 && x <= to + 1e-9) || (step < 0.0 && x >= to - 1e-9) {
            data.push(x);
            if data.len() > MAX_RANGE {
                return Err(format!("range produces more than {MAX_RANGE} elements"));
            }
            x += step;
        }
        let n = data.len();
        Array::from_shape(data, if n == 0 { vec![1, 0] } else { vec![1, n] }).map(ScilabValue::Num)
    }

    fn eval_unary(&mut self, node: &GrammarASTNode) -> Result<ScilabValue, String> {
        // `(+|-|~) unary` or a pass-through to `power`.
        let op = node.children.first().and_then(|c| match c {
            ASTNodeOrToken::Token(t) => Some(t.value.clone()),
            _ => None,
        });
        let operand = only_node(node)?;
        let v = self.eval_node(operand)?;
        match op.as_deref() {
            Some("-") => unary_map(&v, |x| -x),
            Some("+") => Ok(v),
            Some("~") => unary_map(&v, |x| if x == 0.0 { 1.0 } else { 0.0 }),
            _ => Ok(v),
        }
    }

    /// `postfix [ (^|.^) unary ]`. Mirrors `matlab-runtime`'s own
    /// simplification: `^` and `.^` are treated identically (elementwise
    /// power, with scalar broadcasting) — a genuine matrix power (via
    /// eigendecomposition, for a non-diagonal square matrix exponent) is a
    /// documented deferral, exactly as it already is in `matlab-runtime`.
    fn eval_power(&mut self, node: &GrammarASTNode) -> Result<ScilabValue, String> {
        let kids = node_children(node);
        if kids.len() == 1 {
            return self.eval_node(kids[0]);
        }
        let base = self.eval_node(kids[0])?;
        let exp = self.eval_node(kids[1])?;
        let (b, e) = (base.as_num("^")?, exp.as_num("^")?);
        broadcast(b, e, |x, y| x.powf(y)).map(ScilabValue::Num)
    }

    fn eval_postfix(&mut self, node: &GrammarASTNode) -> Result<ScilabValue, String> {
        let mut children = node.children.iter();
        let primary = match children.next() {
            Some(ASTNodeOrToken::Node(n)) => n,
            _ => return Err("scilab-runtime: malformed postfix".to_string()),
        };
        let suffixes: Vec<&GrammarASTNode> = children
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) => Some(n),
                ASTNodeOrToken::Token(_) => None,
            })
            .collect();

        // Resolve the head: a bare NAME with a `(…)` suffix is a variable
        // index, a user-defined function call, or a builtin call, in that
        // priority order (a variable of the same name always shadows a
        // function — the same "vars checked before builtins" precedence
        // `matlab-runtime` already uses); everything else evaluates as a
        // value.
        let (mut value, rest): (ScilabValue, &[&GrammarASTNode]) =
            if let Some(name) = bare_name(primary) {
                if let Some(v) = self.vars.get(&name).cloned() {
                    (v, &suffixes[..])
                } else if suffixes.first().map(|s| s.rule_name.as_str()) == Some("call_suffix") {
                    if let Some(def) = self.functions.get(&name).cloned() {
                        let args = self.eval_call_args(suffixes[0], None)?;
                        let args = values_only(args)?;
                        let mut outs = self.call_user_function(&def, args)?;
                        if outs.is_empty() {
                            return Err(format!(
                                "scilab-runtime: function '{name}' has no return value to use \
                                 in an expression"
                            ));
                        }
                        (outs.remove(0), &suffixes[1..])
                    } else {
                        let args = self.eval_call_args(suffixes[0], None)?;
                        let args = values_only(args)?;
                        let v = builtins::call(&name, &args)?;
                        (v, &suffixes[1..])
                    }
                } else {
                    return Err(format!("scilab-runtime: undefined variable '{name}'"));
                }
            } else {
                (self.eval_node(primary)?, &suffixes[..])
            };

        for suffix in rest {
            value = match suffix.rule_name.as_str() {
                "transpose_suffix" => ScilabValue::Num(ops::transpose(value.as_num("transpose")?)),
                "call_suffix" => self.index_value(&value, suffix)?,
                other => return Err(format!("scilab-runtime: '{other}' is not yet supported")),
            };
        }
        Ok(value)
    }

    /// Index into an already-evaluated array value: `A(i)`, `A(i,j)`,
    /// `A(:,j)`, `A(i,:)`, `A(:)`, `A($)`, `A($-1)`. All subscripts are
    /// 1-based.
    fn index_value(&mut self, value: &ScilabValue, call: &GrammarASTNode) -> Result<ScilabValue, String> {
        let a = value.as_num("indexing")?.clone();
        let args = self.eval_call_args(call, Some(&a))?;
        match args.as_slice() {
            // Linear indexing `A(k)` (column-major) or `A(:)` (linearize).
            [Index::Colon] => Ok(ScilabValue::Num(
                Array::from_shape(a.data().to_vec(), vec![a.len(), 1]).unwrap_or_else(|_| a.clone()),
            )),
            [Index::At(k)] => {
                let idx = scalar_index(k, a.len(), "index")?;
                Ok(ScilabValue::scalar(a.data()[idx]))
            }
            // 2-D indexing `A(i, j)` with optional colons for whole rows/columns.
            [ri, ci] => {
                let rows = dim_indices(ri, a.nrows(), "row")?;
                let cols = dim_indices(ci, a.ncols(), "column")?;
                let mut out = Vec::with_capacity(rows.len() * cols.len());
                for &c in &cols {
                    for &r in &rows {
                        out.push(
                            a.get(r, c)
                                .ok_or_else(|| "index out of bounds".to_string())?,
                        );
                    }
                }
                Array::from_shape(out, vec![rows.len(), cols.len()]).map(ScilabValue::Num)
            }
            _ => Err("scilab-runtime: only 1-D and 2-D indexing is supported".to_string()),
        }
    }

    /// Evaluate a call/index argument list into [`Index`] values.
    ///
    /// ## `$` resolution — "the last valid index along the CURRENT indexing
    /// dimension"
    ///
    /// When `host` is `Some(array)` (a real indexing operation, `A(...)`),
    /// [`Interpreter::dollar_value`] is set to that dimension's own size
    /// **immediately before**, and cleared **immediately after**, evaluating
    /// each argument — the identical shape `matlab_runtime::eval`'s own
    /// `end_value` uses for MATLAB's context-sensitive `end`, reimplemented
    /// independently here for Scilab's context-*free* `$` (MA10 §1 finding
    /// 5/§3: `$` is an ordinary, always-unambiguous `DOLLAR` token, so unlike
    /// MATLAB's own lexer/parser, nothing upstream of this method needed any
    /// retagging pass at all — the *only* remaining work is this runtime-side
    /// "what number does `$` mean right now" resolution).
    ///
    /// A **plain set-then-clear, not a save/restore stack**, is safe here for
    /// exactly the reason it is already safe in `matlab-runtime`: nothing
    /// ever reads `dollar_value` except the recursive `eval_node` call made
    /// *immediately* after it is set, on the very next line — by the time
    /// that call returns (however deeply it recursed, including through a
    /// *nested* indexing expression like `A(B($))`, which sets/reads/clears
    /// `dollar_value` for `B`'s own dimension entirely within its own
    /// recursive call, before ever returning control here), this method's
    /// own `.set(None)` runs next, and the *following* loop iteration sets a
    /// fresh value before any read can occur. No code path ever reads a
    /// `dollar_value` some *other*, still-in-flight call originally set.
    /// `host = None` (a builtin or user-defined function call, not indexing a
    /// real array) means `$` has no meaning there at all — MA10's own wording
    /// ("the current indexing *dimension*") — so a `$` used inside an
    /// ordinary function call's arguments cleanly errors via
    /// `eval_primary`'s own `dollar_value.get()` check, exactly mirroring how
    /// `matlab-runtime` passes `host: None` for its own builtin-call
    /// argument evaluation.
    fn eval_call_args(
        &mut self,
        call: &GrammarASTNode,
        host: Option<&Array>,
    ) -> Result<Vec<Index>, String> {
        let arg_list = match call.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "arg_list" => Some(n),
            _ => None,
        }) {
            Some(a) => a,
            None => return Ok(Vec::new()), // `f()` / `A()`
        };
        let args: Vec<&GrammarASTNode> = node_children(arg_list)
            .into_iter()
            .filter(|n| n.rule_name == "arg")
            .collect();
        let n_args = args.len();
        let mut out = Vec::with_capacity(n_args);
        for (i, arg) in args.iter().enumerate() {
            // A bare colon argument.
            if node_children(arg).is_empty() {
                out.push(Index::Colon);
                continue;
            }
            // Bind `$` to the size of the dimension this argument indexes.
            let dollar = host.map(|h| match (n_args, i) {
                (1, _) => h.len() as f64,   // linear: numel
                (_, 0) => h.nrows() as f64, // first subscript: rows
                _ => h.ncols() as f64,      // second subscript: cols
            });
            self.dollar_value.set(dollar);
            let v = self.eval_node(only_node(arg)?);
            self.dollar_value.set(None);
            out.push(Index::At(v?));
        }
        Ok(out)
    }

    fn eval_primary(&mut self, node: &GrammarASTNode) -> Result<ScilabValue, String> {
        // A primary is a single token (NUMBER/STRING/PERCENT_CONST/DOLLAR/NAME)
        // or a sub-rule node (matrix_literal/group/...).
        if let Some(inner) = first_node(node) {
            return self.eval_node(inner);
        }
        let tok = match node.children.first() {
            Some(ASTNodeOrToken::Token(t)) => t,
            _ => return Err("scilab-runtime: empty primary".to_string()),
        };
        match tok.effective_type_name() {
            "NUMBER" => tok
                .value
                .parse::<f64>()
                .map(ScilabValue::scalar)
                .map_err(|_| format!("invalid number '{}'", tok.value)),
            "STRING" => Ok(ScilabValue::Str(tok.value.clone())),
            "PERCENT_CONST" => builtins::percent_const(&tok.value),
            "DOLLAR" => {
                let d = self.dollar_value.get().ok_or_else(|| {
                    "scilab-runtime: '$' used outside an index".to_string()
                })?;
                Ok(ScilabValue::scalar(d))
            }
            // Defensive-only: every legitimate path reaches a NAME primary
            // through `eval_postfix`'s own `bare_name` fast path (which
            // resolves variables/functions/builtins directly) before this
            // arm is ever visited — mirrors `matlab_runtime::eval_primary`'s
            // identical (also effectively unreachable in normal use) `NAME`
            // arm, kept for robustness if `eval_primary` is ever invoked on
            // a `primary` node directly.
            "NAME" => self
                .vars
                .get(&tok.value)
                .cloned()
                .ok_or_else(|| format!("scilab-runtime: undefined variable '{}'", tok.value)),
            other => Err(format!("scilab-runtime: unexpected token {other}")),
        }
    }

    /// A matrix literal `[ rows ]`: columns are juxtaposed/comma-separated
    /// elements, rows are `;`/newline-separated — inherited unchanged from
    /// MATLAB (MA10 §1's closing paragraph, §3). Each row is concatenated
    /// horizontally, then the rows are stacked vertically. `cell_literal`
    /// (`{ ... }`) is deliberately **not** handled here — MA10 §4 defers
    /// Scilab's `list`/`tlist`/`mlist` aggregate system entirely (§1 finding
    /// 8, §2), the same "cell arrays are a documented deferral" choice
    /// `matlab-runtime`'s own `eval_node` already makes for MATLAB's `{ }`
    /// (its dispatch match has no `"cell_literal"` arm either) — a
    /// `cell_literal` node falls through to `eval_node`'s own
    /// `other => Err(...)` catch-all, an honest rejection rather than a
    /// silently wrong evaluation.
    fn eval_matrix(&mut self, node: &GrammarASTNode) -> Result<ScilabValue, String> {
        let rows_node = match first_node(node) {
            Some(n) if n.rule_name == "matrix_rows" => n,
            _ => return Ok(ScilabValue::Num(Array::from_shape(vec![], vec![0, 0]).unwrap())), // []
        };
        let mut row_arrays = Vec::new();
        for row in node_children(rows_node) {
            if row.rule_name != "matrix_row" {
                continue;
            }
            let mut cells = Vec::new();
            for el in node_children(row) {
                cells.push(self.eval_node(el)?.as_num("matrix element")?.clone());
            }
            row_arrays.push(hcat(&cells)?);
        }
        vcat(&row_arrays).map(ScilabValue::Num)
    }

    // --- control flow ------------------------------------------------------

    fn eval_if(&mut self, node: &GrammarASTNode) -> Result<Flow, String> {
        // if_stmt = "if" expr stmt_sep block_body { elseif_clause } [ else_clause ] "end"
        //
        // `stmt_sep` (MA10 §3's own new production, threaded through all six
        // header sites) is a genuine RULE REFERENCE, not a bare token
        // alternation folded away by the grammar engine — so it appears as
        // its OWN child node between `expr` and `block_body`, exactly like
        // `elseif_clause`/`else_clause` do. `node_children(node)` is
        // therefore `[expr, stmt_sep, block_body, elseif_clause*,
        // else_clause?]`, one slot wider than `matlab.grammar`'s own
        // `if_stmt` (which has no linker production at all) — kids[1] is
        // `stmt_sep`, **not** `block_body`; the true branch's body is
        // kids[2]. Confirmed directly by this crate's own early test
        // failures (assignments inside an `if`'s true branch were silently
        // never executing) before this indexing was corrected — a
        // regression this file's own tests now guard against.
        let kids = node_children(node);
        if self.eval_node(kids[0])?.is_true() {
            return self.eval_block(kids[2]);
        }
        let mut i = 3;
        while i < kids.len() {
            match kids[i].rule_name.as_str() {
                "elseif_clause" => {
                    // elseif_clause = "elseif" expr stmt_sep block_body ->
                    // node_children = [expr, stmt_sep, block_body].
                    let c = node_children(kids[i]);
                    if self.eval_node(c[0])?.is_true() {
                        return self.eval_block(c[2]);
                    }
                    i += 1;
                }
                // else_clause = "else" block_body -- no stmt_sep of its own
                // (MA10 §3: `else` is not one of the six `stmt_sep` sites),
                // so its only child node already IS block_body.
                "else_clause" => return self.eval_block(first_node(kids[i]).unwrap()),
                _ => i += 1,
            }
        }
        Ok(Flow::Normal)
    }

    /// `select expr { case expr stmt_sep block_body } [ else block_body ] end`
    /// — Scilab's own multi-way conditional (MA10 §1 finding 4), with no
    /// MATLAB `switch`/`otherwise` analogue to copy. The grammar's own shape
    /// (`select_stmt = "select" expr stmt_sep { case_clause } [ else_clause ]
    /// "end"`) gives the natural, unsurprising semantics implemented here:
    /// evaluate `select`'s own expression **once**, then compare it in turn
    /// against each `case`'s expression (equality — see
    /// [`crate::eval::values_equal`]), running the **first** matching
    /// `case`'s body; if none match, run `else`'s body if present; if
    /// neither, do nothing. This is the reading `help.scilab.org`'s own
    /// `select`/`case` pages describe and the one construction this grammar
    /// shape actually supports (there is no fallthrough production, and no
    /// bare-value-list-per-case production either — one `case` is one
    /// expression, matched by ordinary equality).
    fn eval_select(&mut self, node: &GrammarASTNode) -> Result<Flow, String> {
        // select_stmt = "select" expr stmt_sep { case_clause } [ else_clause ] "end"
        // -- `select`'s own header is one of the six `stmt_sep` sites (MA10
        // §3), so kids[1] is `stmt_sep`, not the first `case_clause`; the
        // scan for case/else clauses starts at kids[2]. See `eval_if`'s own
        // doc comment for the full "stmt_sep is a real child node" account.
        let kids = node_children(node);
        let selector = self.eval_node(kids[0])?;
        let mut i = 2;
        while i < kids.len() {
            match kids[i].rule_name.as_str() {
                "case_clause" => {
                    // case_clause = "case" expr stmt_sep block_body ->
                    // node_children = [expr, stmt_sep, block_body].
                    let c = node_children(kids[i]);
                    let case_value = self.eval_node(c[0])?;
                    if values_equal(&selector, &case_value)? {
                        return self.eval_block(c[2]);
                    }
                    i += 1;
                }
                "else_clause" => return self.eval_block(first_node(kids[i]).unwrap()),
                _ => i += 1,
            }
        }
        Ok(Flow::Normal)
    }

    fn eval_for(&mut self, node: &GrammarASTNode) -> Result<Flow, String> {
        // for_stmt = "for" NAME EQ expr stmt_sep block_body "end" -- NAME/EQ
        // are bare tokens (not child nodes), but `stmt_sep` IS a child node
        // between `expr` and `block_body` (`for` is one of the six `stmt_sep`
        // sites, MA10 §3), so `node_children(node)` is `[expr, stmt_sep,
        // block_body]` -- kids[2], not kids[1], is the loop body. See
        // `eval_if`'s own doc comment for the full account of why `stmt_sep`
        // shows up as a real node here.
        let name = node
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Token(t) if t.effective_type_name() == "NAME" => {
                    Some(t.value.clone())
                }
                _ => None,
            })
            .ok_or_else(|| "for: missing loop variable".to_string())?;
        let kids = node_children(node);
        let range = self.eval_node(kids[0])?;
        let cols = range.as_num("for")?.clone();
        // Scilab, like MATLAB, iterates over the COLUMNS of the range/matrix;
        // for a row vector that is each element in turn.
        let (nr, nc) = (cols.nrows(), cols.ncols());
        // Defense in depth, matching `while`'s own explicit `MAX_ITERS`: `nc`
        // is transitively bounded today by whatever produced `cols` (a range
        // capped by `eval_colon`'s own `MAX_RANGE`, or a constructed/
        // concatenated array now capped by `builtins::check_total_elements`,
        // per the security review that added both of those), but an explicit,
        // `while`-consistent cap here doesn't depend on every future array
        // producer remembering its own size guard.
        const MAX_ITERS: usize = 10_000_000;
        if nc > MAX_ITERS {
            return Err(format!("for: exceeded the {MAX_ITERS}-iteration limit"));
        }
        for c in 0..nc {
            let col: Vec<f64> = (0..nr).map(|r| cols.get(r, c).unwrap()).collect();
            let v = if col.len() == 1 {
                ScilabValue::scalar(col[0])
            } else {
                ScilabValue::Num(Array::from_shape(col, vec![nr, 1]).unwrap())
            };
            self.vars.insert(name.clone(), v);
            match self.eval_block(kids[2])? {
                Flow::Break => break,
                Flow::Continue | Flow::Normal => {}
            }
        }
        Ok(Flow::Normal)
    }

    fn eval_while(&mut self, node: &GrammarASTNode) -> Result<Flow, String> {
        // while_stmt = "while" expr stmt_sep block_body "end" -- `stmt_sep`
        // is a real child node between `expr` and `block_body` (`while` is
        // one of the six `stmt_sep` sites, MA10 §3), so `node_children(node)`
        // is `[expr, stmt_sep, block_body]` -- kids[2], not kids[1], is the
        // loop body. See `eval_if`'s own doc comment for the full account.
        let kids = node_children(node);
        const MAX_ITERS: usize = 10_000_000;
        let mut iters = 0;
        while self.eval_node(kids[0])?.is_true() {
            match self.eval_block(kids[2])? {
                Flow::Break => break,
                Flow::Continue | Flow::Normal => {}
            }
            iters += 1;
            if iters > MAX_ITERS {
                return Err("while: exceeded the iteration limit".to_string());
            }
        }
        Ok(Flow::Normal)
    }

    /// Run a `block_body` (a run of statement lines), stopping early and
    /// propagating a non-`Normal` [`Flow`] the moment one statement produces
    /// one (a `break`/`continue`, or an `if`/`select` that itself contains
    /// one) — this is what lets `break`/`continue` unwind out of arbitrarily
    /// nested `if`/`select` blocks up to their nearest enclosing loop. Echo
    /// text from statements *inside* a block is not collected (mirrors
    /// `matlab_runtime::eval::Interpreter::eval_block`'s identical existing
    /// choice — inherited, not a new gap: only top-level statement lines,
    /// via `run`, ever produce a displayed echo).
    fn eval_block(&mut self, body: &GrammarASTNode) -> Result<Flow, String> {
        self.guarded(|this| {
            for line in node_children(body) {
                if line.rule_name == "statement_line" {
                    let (flow, _echo) = this.eval_statement_line(line)?;
                    if flow != Flow::Normal {
                        return Ok(flow);
                    }
                }
            }
            Ok(Flow::Normal)
        })
    }

    // --- functions -----------------------------------------------------

    /// Parse a `func_def` node into its registered name and [`FunctionDef`].
    /// `func_def = "function" [ func_returns ] NAME [ LPAREN [ name_list ]
    /// RPAREN ] block_body "endfunction"`.
    fn register_function(&self, def: &GrammarASTNode) -> Result<(String, FunctionDef), String> {
        // The function's OWN name is a bare NAME token directly under
        // `func_def` — distinct from the *output* variable's name, which (if
        // present at all) lives one level deeper inside `func_returns` and is
        // therefore invisible to this direct, shallow scan (mirrors
        // `matlab-to-semantic-ir::MatlabLower::func_def_name`'s identical
        // technique, confirmed directly against that crate's own source).
        let name = def
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Token(t) if t.effective_type_name() == "NAME" => {
                    Some(t.value.clone())
                }
                _ => None,
            })
            .ok_or_else(|| "scilab-runtime: malformed function definition: no name".to_string())?;

        let mut returns: Vec<String> = Vec::new();
        if let Some(ret) = def.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "func_returns" => Some(n),
            _ => None,
        }) {
            if let Some(name_list) = first_node(ret).filter(|n| n.rule_name == "name_list") {
                // `LBRACKET [name_list] RBRACKET EQ` — multi-output.
                returns.extend(name_list.children.iter().filter_map(|c| match c {
                    ASTNodeOrToken::Token(t) if t.effective_type_name() == "NAME" => {
                        Some(t.value.clone())
                    }
                    _ => None,
                }));
            } else if let Some(t) = ret.children.iter().find_map(|c| match c {
                ASTNodeOrToken::Token(t) if t.effective_type_name() == "NAME" => Some(t),
                _ => None,
            }) {
                // `NAME EQ` — single output; the NAME is a bare token
                // directly under `func_returns` (no wrapping node).
                returns.push(t.value.clone());
            }
            // Neither branch matching (`[] = f(...)`, an explicitly
            // zero-output bracket form) leaves `returns` empty, which is
            // exactly correct.
        }

        let mut params: Vec<String> = Vec::new();
        if let Some(name_list) = def.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "name_list" => Some(n),
            _ => None,
        }) {
            params.extend(name_list.children.iter().filter_map(|c| match c {
                ASTNodeOrToken::Token(t) if t.effective_type_name() == "NAME" => {
                    Some(t.value.clone())
                }
                _ => None,
            }));
        }

        let body = def
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "block_body" => Some(n.clone()),
                _ => None,
            })
            .ok_or_else(|| "scilab-runtime: malformed function: no body".to_string())?;

        Ok((
            name.clone(),
            FunctionDef {
                params,
                returns,
                body,
            },
        ))
    }

    /// Call a user-defined function with already-evaluated argument values,
    /// returning its declared output(s) in order.
    ///
    /// Scilab functions (like MATLAB's) get a **fresh workspace** — no
    /// closures, no access to the caller's variables (`global` is an
    /// explicit, separately-deferred mechanism, MA10 §4) — so the caller's
    /// `vars` is swapped out for a new one seeded with the parameter
    /// bindings, and swapped back afterward regardless of how the call
    /// finishes (the swap-back runs unconditionally, right after
    /// `eval_block` returns, whether that is `Ok` or `Err` — `?` is applied
    /// only *after* the restore, so a mid-body error can never leave the
    /// caller's own variables replaced by the callee's).
    fn call_user_function(
        &mut self,
        def: &FunctionDef,
        args: Vec<ScilabValue>,
    ) -> Result<Vec<ScilabValue>, String> {
        if args.len() != def.params.len() {
            return Err(format!(
                "scilab-runtime: function expects {} argument(s), got {}",
                def.params.len(),
                args.len()
            ));
        }
        let mut local_vars: HashMap<String, ScilabValue> = HashMap::new();
        for (name, value) in def.params.iter().zip(args) {
            local_vars.insert(name.clone(), value);
        }
        let caller_vars = std::mem::replace(&mut self.vars, local_vars);

        let result = self.eval_block(&def.body);
        let mut callee_vars = std::mem::replace(&mut self.vars, caller_vars);

        let flow = result?;
        if flow != Flow::Normal {
            return Err(
                "scilab-runtime: 'break'/'continue' used outside a loop inside a function body"
                    .to_string(),
            );
        }
        // A function's return value(s) are simply whatever its declared
        // output variable(s) hold once the body finishes running — this
        // cut's grammar has no `return <expr>` statement at all (MA10 §4
        // never lists `return` in scope), so there is no early-return value
        // to thread; falling off the end of the body IS the return,
        // identical to real Scilab/MATLAB semantics.
        let mut outs = Vec::with_capacity(def.returns.len());
        for name in &def.returns {
            let v = callee_vars.remove(name).ok_or_else(|| {
                format!("scilab-runtime: output variable '{name}' was never assigned")
            })?;
            outs.push(v);
        }
        Ok(outs)
    }
}

// ── operators ────────────────────────────────────────────────────────────────

/// Apply a binary operator (selected by its source token) to two values.
///
/// **The one place `+`-over-strings could sneak back in, and provably
/// doesn't**: `==`/`~=`/`<>` are the *only* three spellings handled before
/// either operand is coerced to numeric (`ScilabValue::as_num`) — every other
/// arm (`+`, `-`, `*`, `/`, `\`, `.* ./ .\`, `< <= > >=`, `& | && ||`) reaches
/// `as_num` unconditionally via the shared `(a, b)` destructure below, so
/// `'a' + 'b'` (or any other operator over a string) errors cleanly through
/// exactly the same path an ordinary type error would, with **no** separate
/// "is this a string, try concatenation" branch anywhere in this function —
/// the deliberate absence MA10 §4 calls for, not a special case that has to
/// be maintained to keep rejecting it.
fn apply_binop(op: &str, lhs: &ScilabValue, rhs: &ScilabValue) -> Result<ScilabValue, String> {
    match op {
        "==" => return Ok(ScilabValue::scalar(bool_to_f64(values_equal(lhs, rhs)?))),
        "~=" | "<>" => return Ok(ScilabValue::scalar(bool_to_f64(!values_equal(lhs, rhs)?))),
        _ => {}
    }
    let (a, b) = (lhs.as_num(op)?, rhs.as_num(op)?);
    let num = |a: Array| Ok(ScilabValue::Num(a));
    match op {
        "+" => ops::add(a, b).and_then(num),
        "-" => ops::sub(a, b).and_then(num),
        ".*" => ops::mul(a, b).and_then(num),
        "./" => ops::div(a, b).and_then(num),
        // Left division `\`/`.\ ` — solving `A*X = B` in general is a real
        // matrix-inverse/least-squares problem `array-runtime` does not
        // implement (no `solve`/`inverse` anywhere in its public API,
        // confirmed directly), so — mirroring `matlab-runtime`'s own
        // documented `^`/`.^` simplification ("coincide for scalars ...
        // matrix power is a documented deferral") — bare `\` and `.\ ` are
        // both treated as the elementwise reference operation (`y / x`,
        // broadcasting scalars), which is exact for the common textbook case
        // (scalar or elementwise division) and an honest, disclosed
        // simplification for the general matrix case.
        ".\\" | "\\" => broadcast(a, b, |x, y| y / x).and_then(num),
        // `*`: scalar on either side is an element-wise scale; otherwise it
        // is a true matrix product — lowered to the array-runtime planner +
        // executor, exactly like `matlab-runtime`.
        "*" => {
            if a.is_scalar() || b.is_scalar() {
                ops::mul(a, b).and_then(num)
            } else {
                execute(Kernel::MatMul, a, b).and_then(num)
            }
        }
        "/" if b.is_scalar() => ops::div(a, b).and_then(num),
        "<" => broadcast(a, b, |x, y| (x < y) as i32 as f64).and_then(num),
        "<=" => broadcast(a, b, |x, y| (x <= y) as i32 as f64).and_then(num),
        ">" => broadcast(a, b, |x, y| (x > y) as i32 as f64).and_then(num),
        ">=" => broadcast(a, b, |x, y| (x >= y) as i32 as f64).and_then(num),
        "&" | "&&" => {
            broadcast(a, b, |x, y| ((x != 0.0) && (y != 0.0)) as i32 as f64).and_then(num)
        }
        "|" | "||" => {
            broadcast(a, b, |x, y| ((x != 0.0) || (y != 0.0)) as i32 as f64).and_then(num)
        }
        other => Err(format!(
            "scilab-runtime: operator '{other}' is not yet supported"
        )),
    }
}

/// Equality used by both `==`/`~=`/`<>` and `select`/`case` matching
/// (MA10 §4 scopes string equality in; §7's own citation restricts *ordering*
/// comparisons to numeric types only, so this is deliberately narrower than
/// full cross-type coercion): two strings compare by content; two numeric
/// arrays compare exactly like every other numeric comparison operator
/// (elementwise, then MATLAB/Scilab's own "true iff every element matches"
/// truthiness convention — see `apply_binop`'s numeric `==`/`<` siblings for
/// the identical `broadcast` shape); a string and a number are never equal.
/// This last rule is a judgment call (flagged for a reviewer): MA10 does not
/// document what `'abc' == 5` should do, and there is no shared-with-MATLAB
/// answer to inherit here on purpose (MA10 §1 finding 1's whole point) — the
/// chosen behavior "never equal, never an error" was chosen so a `select`/
/// `case` mixing types across cases degrades to "this case doesn't match"
/// rather than aborting the entire construct, which is friendlier and no
/// less honest than erroring, since equality across genuinely different
/// value kinds does not risk landing on a wrong *numeric* answer the way a
/// silently-implemented `+` would.
fn values_equal(a: &ScilabValue, b: &ScilabValue) -> Result<bool, String> {
    match (a, b) {
        (ScilabValue::Str(x), ScilabValue::Str(y)) => Ok(x == y),
        (ScilabValue::Num(_), ScilabValue::Num(_)) => {
            let eq = broadcast(a.as_num("==")?, b.as_num("==")?, |x, y| {
                (x == y) as i32 as f64
            })?;
            Ok(!eq.is_empty() && eq.data().iter().all(|&x| x != 0.0))
        }
        _ => Ok(false),
    }
}

fn bool_to_f64(b: bool) -> f64 {
    if b {
        1.0
    } else {
        0.0
    }
}

/// Element-wise binary map with MATLAB/Scilab scalar broadcasting (either
/// operand may be `1×1`); otherwise the shapes must agree.
fn broadcast(a: &Array, b: &Array, f: impl Fn(f64, f64) -> f64) -> Result<Array, String> {
    let (ad, bd) = (a.data(), b.data());
    let data: Vec<f64> = if a.is_scalar() {
        bd.iter().map(|&y| f(ad[0], y)).collect()
    } else if b.is_scalar() {
        ad.iter().map(|&x| f(x, bd[0])).collect()
    } else {
        if a.shape() != b.shape() {
            return Err(format!(
                "matrix dimensions must agree: {:?} vs {:?}",
                a.shape(),
                b.shape()
            ));
        }
        ad.iter().zip(bd).map(|(&x, &y)| f(x, y)).collect()
    };
    let shape = if a.is_scalar() { b.shape() } else { a.shape() };
    Array::from_shape(data, shape.to_vec())
}

fn unary_map(v: &ScilabValue, f: impl Fn(f64) -> f64) -> Result<ScilabValue, String> {
    let a = v.as_num("unary")?;
    Array::from_shape(a.data().iter().map(|&x| f(x)).collect(), a.shape().to_vec())
        .map(ScilabValue::Num)
}

/// Concatenate arrays horizontally (a matrix row): all must share their row
/// count; columns are appended. (`[1 2 3]`, `[A B]`.)
///
/// `[A A]` repeated (`A = [A A];` a few dozen times) *doubles* the element
/// count each time with no per-call size individually large enough to look
/// suspicious — a purely local `nrows`/`ncols` check on each call's INPUTS
/// can't catch this (each input is small; only the accumulated OUTPUT grows
/// exponentially), so the guard has to be on the constructed RESULT's total
/// element count, matching `builtins::check_total_elements`'s identical cap.
/// Found during security review of MA-10d: 24 repetitions (~260 bytes of
/// source, trivially under every other limit in this crate — `MAX_INPUT_LEN`,
/// `MAX_DEPTH`) reached 2^24 elements with no error before this guard existed.
fn hcat(cells: &[Array]) -> Result<Array, String> {
    let cells: Vec<&Array> = cells.iter().filter(|a| !a.is_empty()).collect();
    let Some(first) = cells.first() else {
        return Array::from_shape(vec![], vec![1, 0]);
    };
    let nrows = first.nrows();
    let mut cols: Vec<Vec<f64>> = Vec::new();
    for a in &cells {
        if a.nrows() != nrows {
            return Err("horizontal concatenation: row counts must match".to_string());
        }
        crate::builtins::check_total_elements("matrix literal", nrows, cols.len() + a.ncols())?;
        for c in 0..a.ncols() {
            cols.push((0..nrows).map(|r| a.get(r, c).unwrap()).collect());
        }
    }
    let ncols = cols.len();
    let mut data = vec![0.0; nrows * ncols];
    for (c, col) in cols.iter().enumerate() {
        for (r, &v) in col.iter().enumerate() {
            data[c * nrows + r] = v;
        }
    }
    Array::from_shape(data, vec![nrows, ncols])
}

/// Stack row-arrays vertically (`[a; b]`): all must share their column count.
/// See `hcat`'s own doc comment for why the guard must bound the accumulated
/// OUTPUT size (`[A; A]` repeated has the identical exponential-doubling
/// shape), not just each individual input.
fn vcat(rows: &[Array]) -> Result<Array, String> {
    let rows: Vec<&Array> = rows.iter().filter(|a| !a.is_empty()).collect();
    let Some(first) = rows.first() else {
        return Array::from_shape(vec![], vec![0, 0]);
    };
    let ncols = first.ncols();
    let mut total_rows: usize = 0;
    for a in &rows {
        if a.ncols() != ncols {
            return Err("vertical concatenation: column counts must match".to_string());
        }
        total_rows += a.nrows();
        crate::builtins::check_total_elements("matrix literal", total_rows, ncols)?;
    }
    let mut data = vec![0.0; total_rows * ncols];
    let mut row_off = 0;
    for a in &rows {
        for c in 0..ncols {
            for r in 0..a.nrows() {
                data[c * total_rows + (row_off + r)] = a.get(r, c).unwrap();
            }
        }
        row_off += a.nrows();
    }
    Array::from_shape(data, vec![total_rows, ncols])
}

/// Resolve a scalar 1-based subscript to a 0-based offset, bounds-checked.
fn scalar_index(v: &ScilabValue, len: usize, what: &str) -> Result<usize, String> {
    let a = v.as_num(what)?;
    let one = a.data().first().copied().unwrap_or(0.0);
    let idx = one.round() as i64;
    if idx < 1 || idx as usize > len {
        return Err(format!("{what} {idx} is out of bounds 1..={len}"));
    }
    Ok((idx - 1) as usize)
}

/// Resolve one subscript of a 2-D index into 0-based offsets along a
/// dimension of size `dim`. A colon means the whole dimension; a vector
/// means several.
fn dim_indices(idx: &Index, dim: usize, what: &str) -> Result<Vec<usize>, String> {
    match idx {
        Index::Colon => Ok((0..dim).collect()),
        Index::At(v) => {
            let a = v.as_num(what)?;
            a.data()
                .iter()
                .map(|&x| {
                    let i = x.round() as i64;
                    if i < 1 || i as usize > dim {
                        Err(format!("{what} index {i} is out of bounds 1..={dim}"))
                    } else {
                        Ok((i - 1) as usize)
                    }
                })
                .collect()
        }
    }
}

fn scalar_of(v: &ScilabValue, ctx: &str) -> Result<f64, String> {
    let a = v.as_num(ctx)?;
    a.data()
        .first()
        .copied()
        .ok_or_else(|| format!("{ctx} bound is empty"))
}

/// Convert an [`Index`] list into plain values for a builtin/user-function
/// call, erroring if a bare colon (only meaningful for real array indexing)
/// was used as an ordinary call argument.
fn values_only(args: Vec<Index>) -> Result<Vec<ScilabValue>, String> {
    args.into_iter()
        .map(|a| match a {
            Index::At(v) => Ok(v),
            Index::Colon => Err("scilab-runtime: ':' is not a valid call argument".to_string()),
        })
        .collect()
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
    first_node(node).ok_or_else(|| format!("scilab-runtime: malformed '{}' node", node.rule_name))
}

/// Is this `statement_line` a top-level `function ... endfunction`
/// definition? (Already registered by `run`'s pass 1, so pass 2 skips it.)
fn is_func_def_line(line: &GrammarASTNode) -> bool {
    first_node(line)
        .filter(|n| n.rule_name == "statement")
        .and_then(first_node)
        .is_some_and(|n| n.rule_name == "func_def")
}

/// If `node` is a `primary` that is a bare `NAME`, return that name.
fn bare_name(primary: &GrammarASTNode) -> Option<String> {
    if primary.rule_name != "primary" || first_node(primary).is_some() {
        return None;
    }
    match primary.children.first() {
        Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "NAME" => {
            Some(t.value.clone())
        }
        _ => None,
    }
}

/// Follow a chain of single-child wrapper nodes (the precedence cascade —
/// `logical_or -> logical_and -> ... -> postfix -> primary`, or similar) down
/// to the first node named `target`. Returns `None` the moment a tier along
/// the way has anything other than exactly one child node (meaning a real
/// operator was applied there, so the expression is not a single bare
/// `target`-shaped value).
fn drill_to<'a>(node: &'a GrammarASTNode, target: &str) -> Option<&'a GrammarASTNode> {
    let mut cur = node;
    loop {
        if cur.rule_name == target {
            return Some(cur);
        }
        match node_children(cur).as_slice() {
            [only] => cur = only,
            _ => return None,
        }
    }
}

/// The variable name of an assignment LHS, if it is a simple `postfix → … →
/// primary → NAME` with no suffixes.
fn lhs_name(node: &GrammarASTNode) -> Option<String> {
    drill_to(node, "primary").and_then(bare_name)
}

/// If `node` is (after drilling through the same wrapper chain [`lhs_name`]
/// uses) a `matrix_literal` containing exactly one row of bare-NAME
/// elements, return those names in order — the `[a, b] = f(x)` multi-return
/// assignment target shape (MA10 §4). `None` for anything else (a single
/// name, an indexed target, a real matrix-literal *value*, ...), so the
/// caller falls through to ordinary single-target assignment (and its
/// existing error if that also fails to match).
fn lhs_name_list(node: &GrammarASTNode) -> Option<Vec<String>> {
    let matrix = drill_to(node, "matrix_literal")?;
    let rows_node = first_node(matrix).filter(|n| n.rule_name == "matrix_rows")?;
    let rows = node_children(rows_node);
    let [row] = rows.as_slice() else {
        return None; // more than one row -- not a plain name list
    };
    let mut names = Vec::new();
    for el in node_children(row) {
        names.push(lhs_name(el)?);
    }
    if names.is_empty() {
        None
    } else {
        Some(names)
    }
}

/// Follow the same single-child wrapper chain down to a `postfix` node, if
/// `node`'s value is nothing but one bare postfix expression with no
/// operator applied at any tier above it.
fn find_call_postfix(node: &GrammarASTNode) -> Option<&GrammarASTNode> {
    drill_to(node, "postfix")
}

/// If `postfix` is `NAME(...)` — a bare name whose first suffix is a
/// `call_suffix` — return the name and that `call_suffix` node.
fn bare_call(postfix: &GrammarASTNode) -> Option<(String, &GrammarASTNode)> {
    let mut children = postfix.children.iter();
    let primary = match children.next() {
        Some(ASTNodeOrToken::Node(n)) => n,
        _ => return None,
    };
    let name = bare_name(primary)?;
    let first_suffix = children.find_map(|c| match c {
        ASTNodeOrToken::Node(n) => Some(n),
        _ => None,
    })?;
    if first_suffix.rule_name == "call_suffix" {
        Some((name, first_suffix))
    } else {
        None
    }
}
