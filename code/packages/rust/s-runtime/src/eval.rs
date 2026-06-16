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
    arithmetic, bounded_sequence, compare, format_value, index, negate, Arg, Param, SValue,
};
use coding_adventures_s_parser::try_parse_s;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use r_vector::{is_na_real, na_real, Double};
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
}

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
        }
    }

    /// The global environment (for tests and the REPL).
    pub fn global(&self) -> &Env {
        &self.global
    }

    /// Append a block of `print()` output (used by the print built-in path).
    fn emit(&self, lines: &[String]) {
        let mut out = self.out.borrow_mut();
        for line in lines {
            out.push_str(line);
            out.push('\n');
        }
    }

    /// Parse and evaluate `src`, returning the value of the last statement.
    pub fn eval_str(&self, src: &str) -> SResult<Outcome> {
        self.out.borrow_mut().clear();
        let program = try_parse_s(src).map_err(SError::Parse)?;

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

        Ok(Outcome {
            value: last,
            visible: self.visible.get(),
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
        let name = lvalue_name(target_node)?;
        let value = self.eval_node(value_node, env)?;
        define(env, &name, value.clone());
        self.as_invisible(value)
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
                    let args = self.eval_args(suffix, env)?;
                    if args.len() != 1 {
                        return Err(SError::Index(
                            "v1 supports single-bracket indexing with one subscript".into(),
                        ));
                    }
                    value = index(&value, &args[0].value)?;
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

    /// Apply a callable value to evaluated arguments.
    fn apply(&self, callee: SValue, args: &[Arg], _env: &Env) -> SResult<SValue> {
        match callee {
            SValue::Builtin { name, func } => {
                let result = func(args)?;
                if name == "print" {
                    self.emit(&format_value(&result));
                    self.as_invisible(result)
                } else {
                    self.as_visible(result)
                }
            }
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
                    "STRING" => SValue::Character(vec![Some(strip_quotes(value))]),
                    "KEYWORD" => match value {
                        "TRUE" | "T" => SValue::Logical(vec![Some(true)]),
                        "FALSE" | "F" => SValue::Logical(vec![Some(false)]),
                        "NULL" => SValue::Null,
                        "NA" => SValue::Logical(vec![None]),
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
    let mut names = Vec::new();
    let mut total_tokens = 0;
    collect_tokens(node, &mut names, &mut total_tokens);
    if names.len() == 1 && total_tokens == 1 {
        Ok(names.remove(0))
    } else {
        Err(SError::TypeError(
            "invalid (non-name) assignment target".into(),
        ))
    }
}

/// Collect every `NAME` token value and a count of all tokens in a subtree.
fn collect_tokens(node: &GrammarASTNode, names: &mut Vec<String>, total: &mut usize) {
    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(t) => {
                *total += 1;
                if t.effective_type_name() == "NAME" {
                    names.push(t.value.clone());
                }
            }
            ASTNodeOrToken::Node(n) => collect_tokens(n, names, total),
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
fn nth_element(value: &SValue, i: usize) -> SValue {
    match value {
        SValue::Double(d) => SValue::Double(Double::from_values(vec![d
            .get_value(i)
            .unwrap_or_else(na_real)])),
        SValue::Logical(v) => SValue::Logical(vec![v.get(i).copied().flatten()]),
        SValue::Character(v) => SValue::Character(vec![v.get(i).cloned().flatten()]),
        _ => SValue::Null,
    }
}

/// Convenience: evaluate `src` in a fresh interpreter and return the value.
pub fn eval_s(src: &str) -> SResult<SValue> {
    Interpreter::new().eval_str(src).map(|o| o.value)
}
