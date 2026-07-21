//! The tree-walking evaluator.
//!
//! [`Interpreter`] walks the [`GrammarASTNode`] tree from `matlab-parser` and
//! computes [`MatValue`]s over `array-runtime`. The headline: a matrix product
//! `A * B` lowers to [`array_runtime::execute`]`(MatMul, …)`, which plans the op
//! and runs it on the cheapest available backend (CPU today, a GPU executor the
//! moment one is registered) — so `gpuArray`-style acceleration is automatic and
//! by cost, with no language-level GPU code. Element-wise operators use the
//! `array_runtime::ops` reference path (exact `f64`, with scalar broadcasting).

use crate::builtins;
use crate::value::{echo, MatValue};
use array_runtime::{execute, ops, Array, Kernel};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

/// Maximum expression/block nesting depth. Crafted input like `((((…))))` or
/// `if…if…end…end` recurses one frame per level; this bound turns a stack
/// overflow (which would abort the whole process) into a clean error.
const MAX_DEPTH: usize = 512;

/// A persistent MATLAB session: a variable workspace, the value of `end`
/// currently in scope (set while evaluating an index expression), and the
/// current evaluation depth.
pub struct Interpreter {
    vars: HashMap<String, MatValue>,
    end_value: Cell<Option<f64>>,
    depth: Rc<Cell<usize>>,
}

/// RAII guard that decrements the depth counter on every exit path (including
/// `?` early returns).
struct DepthGuard(Rc<Cell<usize>>);

impl Drop for DepthGuard {
    fn drop(&mut self) {
        self.0.set(self.0.get().saturating_sub(1));
    }
}

/// One index argument: a value, or a bare colon meaning "the whole dimension".
enum Index {
    Colon,
    At(MatValue),
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
            end_value: Cell::new(None),
            depth: Rc::new(Cell::new(0)),
        }
    }

    /// Enter one level of recursion, erroring if the nesting limit is exceeded.
    /// The returned guard decrements the counter when it drops.
    fn enter(&self) -> Result<DepthGuard, String> {
        self.depth.set(self.depth.get() + 1);
        let guard = DepthGuard(Rc::clone(&self.depth));
        if self.depth.get() > MAX_DEPTH {
            return Err("matlab-runtime: expression or block nesting too deep".to_string());
        }
        Ok(guard)
    }

    /// Evaluate a whole program (the `program` node), returning the prompt echo
    /// for every statement whose result is *not* suppressed by a trailing `;`.
    pub fn run(&mut self, program: &GrammarASTNode) -> Result<String, String> {
        let mut out = String::new();
        for line in node_children(program) {
            if line.rule_name != "statement_line" {
                continue;
            }
            if let Some(text) = self.eval_statement_line(line)? {
                out.push_str(&text);
                out.push('\n');
            }
        }
        Ok(out)
    }

    /// Evaluate one statement line. Returns `Some(echo)` for a visible result,
    /// `None` when suppressed (`;`) or when the line is just a separator.
    fn eval_statement_line(&mut self, line: &GrammarASTNode) -> Result<Option<String>, String> {
        let stmt = match first_node(line) {
            Some(n) if n.rule_name == "statement" => n,
            _ => return Ok(None), // a bare terminator / blank line
        };
        // A trailing `;` suppresses display; a newline or comma shows it. The
        // terminator is a `stmt_term` child node holding the punctuation token.
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
            "if_stmt" => {
                self.eval_if(inner)?;
                Ok(None)
            }
            "for_stmt" => {
                self.eval_for(inner)?;
                Ok(None)
            }
            "while_stmt" => {
                self.eval_while(inner)?;
                Ok(None)
            }
            // An expression statement: either an assignment (`x = …`) or a bare
            // expression (echoed as `ans`).
            "expr" => {
                let (name, value) = self.eval_expr_or_assign(inner)?;
                if suppressed {
                    Ok(None)
                } else {
                    Ok(Some(echo(&name, &value)))
                }
            }
            other => Err(format!("matlab-runtime: unsupported statement '{other}'")),
        }
    }

    /// Evaluate an `expr`. If it is a top-level assignment `lhs = rhs`, bind the
    /// variable and report its name; otherwise evaluate it and report `ans`
    /// (also binding `ans`, as MATLAB does).
    fn eval_expr_or_assign(&mut self, expr: &GrammarASTNode) -> Result<(String, MatValue), String> {
        let assignment = only_node(expr)?; // expr = assignment
                                           // assignment = logical_or [ EQ assignment ]
        let kids = node_children(assignment);
        let has_eq = assignment
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "="));
        if has_eq && kids.len() == 2 {
            // The LHS must be a plain variable name (indexed-assignment is a
            // documented deferral).
            let name = lhs_name(kids[0]).ok_or_else(|| {
                "matlab-runtime: assignment target must be a variable".to_string()
            })?;
            let value = self.eval_node(kids[1])?;
            self.vars.insert(name.clone(), value.clone());
            Ok((name, value))
        } else {
            let value = self.eval_node(assignment)?;
            self.vars.insert("ans".to_string(), value.clone());
            Ok(("ans".to_string(), value))
        }
    }

    /// Evaluate any expression node, dispatching by rule name.
    fn eval_node(&self, node: &GrammarASTNode) -> Result<MatValue, String> {
        let _guard = self.enter()?;
        match node.rule_name.as_str() {
            "expr" | "assignment" | "statement" => self.eval_node(only_node(node)?),
            "logical_or" | "logical_and" | "bit_or" | "bit_and" | "comparison" | "additive"
            | "multiplicative" => self.eval_binary_chain(node),
            "colon_expr" => self.eval_colon(node),
            "unary" => self.eval_unary(node),
            "power" => self.eval_power(node),
            "postfix" => self.eval_postfix(node),
            "primary" => self.eval_primary(node),
            "group" => self.eval_node(only_node(node)?),
            "matrix_literal" => self.eval_matrix(node),
            other => Err(format!("matlab-runtime: cannot evaluate '{other}'")),
        }
    }

    /// Left-associative fold of a binary-operator chain (`a op b op c …`). The
    /// operator token between operands selects the operation.
    fn eval_binary_chain(&self, node: &GrammarASTNode) -> Result<MatValue, String> {
        let mut acc: Option<MatValue> = None;
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
        acc.ok_or_else(|| "matlab-runtime: empty expression".to_string())
    }

    /// The colon: `a:b` or `a:b:c`. Builds an inclusive numeric row vector.
    fn eval_colon(&self, node: &GrammarASTNode) -> Result<MatValue, String> {
        let parts = node_children(node);
        if parts.len() == 1 {
            return self.eval_node(parts[0]);
        }
        let scalar = |v: &MatValue| -> Result<f64, String> {
            let a = v.as_num("range")?;
            a.data()
                .first()
                .copied()
                .ok_or_else(|| "range bound is empty".to_string())
        };
        let (from, step, to) = match parts.as_slice() {
            [a, b] => (
                scalar(&self.eval_node(a)?)?,
                1.0,
                scalar(&self.eval_node(b)?)?,
            ),
            [a, s, b] => (
                scalar(&self.eval_node(a)?)?,
                scalar(&self.eval_node(s)?)?,
                scalar(&self.eval_node(b)?)?,
            ),
            _ => return Err("range: too many colons".to_string()),
        };
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
        // A range is a 1×n row vector.
        let n = data.len();
        Array::from_shape(data, if n == 0 { vec![1, 0] } else { vec![1, n] }).map(MatValue::Num)
    }

    fn eval_unary(&self, node: &GrammarASTNode) -> Result<MatValue, String> {
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

    fn eval_power(&self, node: &GrammarASTNode) -> Result<MatValue, String> {
        // `postfix [ (^|.^) unary ]`.
        let kids = node_children(node);
        if kids.len() == 1 {
            return self.eval_node(kids[0]);
        }
        let base = self.eval_node(kids[0])?;
        let exp = self.eval_node(kids[1])?;
        // `^` and `.^` coincide for scalars (the common case); matrix power is a
        // documented deferral.
        let (b, e) = (base.as_num("^")?, exp.as_num("^")?);
        broadcast(b, e, |x, y| x.powf(y)).map(MatValue::Num)
    }

    fn eval_postfix(&self, node: &GrammarASTNode) -> Result<MatValue, String> {
        let mut children = node.children.iter();
        let primary = match children.next() {
            Some(ASTNodeOrToken::Node(n)) => n,
            _ => return Err("matlab-runtime: malformed postfix".to_string()),
        };
        let suffixes: Vec<&GrammarASTNode> = children
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) => Some(n),
                ASTNodeOrToken::Token(_) => None,
            })
            .collect();

        // Resolve the head: a bare NAME with a `(…)` suffix is either indexing a
        // variable or calling a builtin; everything else evaluates as a value.
        let (mut value, rest): (MatValue, &[&GrammarASTNode]) =
            if let Some(name) = bare_name(primary) {
                if name == "end" {
                    // The index sentinel, bound while evaluating index arguments.
                    let e = self
                        .end_value
                        .get()
                        .ok_or_else(|| "matlab-runtime: 'end' used outside an index".to_string())?;
                    (MatValue::scalar(e), &suffixes)
                } else if let Some(v) = self.vars.get(&name) {
                    (v.clone(), &suffixes)
                } else if suffixes.first().map(|s| s.rule_name.as_str()) == Some("call_suffix") {
                    // A builtin call: arguments are values (a bare colon is not
                    // valid here).
                    let args = self
                        .eval_index_args(suffixes[0], None)?
                        .into_iter()
                        .map(|a| match a {
                            Index::At(v) => Ok(v),
                            Index::Colon => {
                                Err("matlab-runtime: ':' is not a valid call argument".to_string())
                            }
                        })
                        .collect::<Result<Vec<_>, String>>()?;
                    let v = builtins::call(&name, &args)?;
                    (v, &suffixes[1..])
                } else {
                    return Err(format!("matlab-runtime: undefined variable '{name}'"));
                }
            } else {
                (self.eval_node(primary)?, &suffixes[..])
            };

        for suffix in rest {
            value = match suffix.rule_name.as_str() {
                "transpose_suffix" => MatValue::Num(ops::transpose(value.as_num("transpose")?)),
                "call_suffix" => self.index_value(&value, suffix)?,
                other => return Err(format!("matlab-runtime: '{other}' is not yet supported")),
            };
        }
        Ok(value)
    }

    /// Index into an already-evaluated array value: `A(i)`, `A(i,j)`, `A(:,j)`,
    /// `A(i,:)`, `A(:)`, `A(end)`. All subscripts are 1-based.
    fn index_value(&self, value: &MatValue, call: &GrammarASTNode) -> Result<MatValue, String> {
        let a = value.as_num("indexing")?;
        let args = self.eval_index_args(call, Some(a))?;
        match args.as_slice() {
            // Linear indexing `A(k)` (column-major) or `A(:)` (linearize).
            [Index::Colon] => Ok(MatValue::Num(
                Array::from_shape(a.data().to_vec(), vec![a.len(), 1])
                    .unwrap_or_else(|_| a.clone()),
            )),
            [Index::At(k)] => {
                let idx = scalar_index(k, a.len(), "index")?;
                Ok(MatValue::scalar(a.data()[idx]))
            }
            // 2-D indexing `A(i, j)` with optional colons for whole rows/columns.
            [ri, ci] => {
                let rows = dim_indices(ri, a.nrows(), "row")?;
                let cols = dim_indices(ci, a.ncols(), "column")?;
                // Each index vector's own length is bounded independently
                // (by whatever constructed it) but nothing bounds their
                // PRODUCT: `A(idx, idx)` with two independently-in-bounds
                // index vectors can still request an astronomical result.
                // Checked before `Vec::with_capacity`/`Array::from_shape`
                // ever allocate. Security regression, mirrors
                // scilab-runtime's fix.
                crate::builtins::check_total_elements("indexing", rows.len(), cols.len())?;
                let mut out = Vec::with_capacity(rows.len() * cols.len());
                // Column-major output: iterate columns, then rows.
                for &c in &cols {
                    for &r in &rows {
                        out.push(
                            a.get(r, c)
                                .ok_or_else(|| "index out of bounds".to_string())?,
                        );
                    }
                }
                Array::from_shape(out, vec![rows.len(), cols.len()]).map(MatValue::Num)
            }
            _ => Err("matlab-runtime: only 1-D and 2-D indexing is supported".to_string()),
        }
    }

    /// Evaluate a call/index argument list into [`Index`] values. When `host` is
    /// `Some`, `end` resolves to that array's relevant dimension size.
    fn eval_index_args(
        &self,
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
            // Bind `end` to the size of the dimension this argument indexes.
            let end = host.map(|h| match (n_args, i) {
                (1, _) => h.len() as f64,   // linear: numel
                (_, 0) => h.nrows() as f64, // first subscript: rows
                _ => h.ncols() as f64,      // second subscript: cols
            });
            self.end_value.set(end);
            let v = self.eval_node(only_node(arg)?);
            self.end_value.set(None);
            out.push(Index::At(v?));
        }
        Ok(out)
    }

    fn eval_primary(&self, node: &GrammarASTNode) -> Result<MatValue, String> {
        // A primary is a single token (NUMBER/STRING/NAME) or a sub-rule node.
        if let Some(inner) = first_node(node) {
            return self.eval_node(inner);
        }
        let tok = match node.children.first() {
            Some(ASTNodeOrToken::Token(t)) => t,
            _ => return Err("matlab-runtime: empty primary".to_string()),
        };
        match tok.effective_type_name() {
            "NUMBER" => tok
                .value
                .parse::<f64>()
                .map(MatValue::scalar)
                .map_err(|_| format!("invalid number '{}'", tok.value)),
            "STRING" => Ok(MatValue::Char(tok.value.clone())),
            "NAME" => {
                if tok.value == "end" {
                    if let Some(e) = self.end_value.get() {
                        return Ok(MatValue::scalar(e));
                    }
                }
                self.vars
                    .get(&tok.value)
                    .cloned()
                    .ok_or_else(|| format!("matlab-runtime: undefined variable '{}'", tok.value))
            }
            other => Err(format!("matlab-runtime: unexpected token {other}")),
        }
    }

    /// A matrix literal `[ rows ]`: columns are juxtaposed/comma-separated
    /// elements, rows are `;`/newline-separated. Each row is concatenated
    /// horizontally, then the rows are stacked vertically.
    fn eval_matrix(&self, node: &GrammarASTNode) -> Result<MatValue, String> {
        let rows_node = match first_node(node) {
            Some(n) if n.rule_name == "matrix_rows" => n,
            _ => {
                return Ok(MatValue::Num(
                    Array::from_shape(vec![], vec![0, 0]).unwrap(),
                ))
            } // []
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
        vcat(&row_arrays).map(MatValue::Num)
    }

    // --- control flow ----------------------------------------------------

    fn eval_if(&mut self, node: &GrammarASTNode) -> Result<(), String> {
        // if_stmt = "if" expr block_body { elseif_clause } [ else_clause ] "end"
        let kids = node_children(node);
        // Leading `if expr block_body`.
        if self.eval_node(kids[0])?.is_true() {
            return self.eval_block(kids[1]);
        }
        let mut i = 2;
        while i < kids.len() {
            match kids[i].rule_name.as_str() {
                "elseif_clause" => {
                    let c = node_children(kids[i]);
                    if self.eval_node(c[0])?.is_true() {
                        return self.eval_block(c[1]);
                    }
                    i += 1;
                }
                "else_clause" => {
                    return self.eval_block(first_node(kids[i]).unwrap());
                }
                _ => i += 1,
            }
        }
        Ok(())
    }

    fn eval_for(&mut self, node: &GrammarASTNode) -> Result<(), String> {
        // for_stmt = "for" NAME EQ expr block_body "end"
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
        // kids: [expr (the range), block_body]
        let range = self.eval_node(kids[0])?;
        let cols = range.as_num("for")?.clone();
        // MATLAB iterates over the COLUMNS of the range/matrix; for a row vector
        // that is each element in turn.
        let (nr, nc) = (cols.nrows(), cols.ncols());
        for c in 0..nc {
            let col: Vec<f64> = (0..nr).map(|r| cols.get(r, c).unwrap()).collect();
            let v = if col.len() == 1 {
                MatValue::scalar(col[0])
            } else {
                MatValue::Num(Array::from_shape(col, vec![nr, 1]).unwrap())
            };
            self.vars.insert(name.clone(), v);
            self.eval_block(kids[1])?;
        }
        Ok(())
    }

    fn eval_while(&mut self, node: &GrammarASTNode) -> Result<(), String> {
        // while_stmt = "while" expr block_body "end"
        let kids = node_children(node);
        const MAX_ITERS: usize = 10_000_000;
        let mut iters = 0;
        while self.eval_node(kids[0])?.is_true() {
            self.eval_block(kids[1])?;
            iters += 1;
            if iters > MAX_ITERS {
                return Err("while: exceeded the iteration limit".to_string());
            }
        }
        Ok(())
    }

    /// Run a `block_body` (a run of statement lines) for its side effects.
    fn eval_block(&mut self, body: &GrammarASTNode) -> Result<(), String> {
        let _guard = self.enter()?;
        for line in node_children(body) {
            if line.rule_name == "statement_line" {
                self.eval_statement_line(line)?;
            }
        }
        Ok(())
    }
}

// ── operators ────────────────────────────────────────────────────────────────

/// Apply a binary operator (selected by its source token) to two values.
fn apply_binop(op: &str, lhs: &MatValue, rhs: &MatValue) -> Result<MatValue, String> {
    let (a, b) = (lhs.as_num(op)?, rhs.as_num(op)?);
    let num = |a: Array| Ok(MatValue::Num(a));
    match op {
        "+" => ops::add(a, b).and_then(num),
        "-" => ops::sub(a, b).and_then(num),
        ".*" => ops::mul(a, b).and_then(num),
        "./" => ops::div(a, b).and_then(num),
        ".\\" => broadcast(a, b, |x, y| y / x).and_then(num),
        ".^" => broadcast(a, b, |x, y| x.powf(y)).and_then(num),
        // `*`: scalar on either side is an element-wise scale; otherwise it is a
        // true matrix product — lowered to the array-runtime planner + executor.
        "*" => {
            if a.is_scalar() || b.is_scalar() {
                ops::mul(a, b).and_then(num)
            } else {
                execute(Kernel::MatMul, a, b).and_then(num)
            }
        }
        "/" if b.is_scalar() => ops::div(a, b).and_then(num),
        "==" => broadcast(a, b, |x, y| (x == y) as i32 as f64).and_then(num),
        "~=" => broadcast(a, b, |x, y| (x != y) as i32 as f64).and_then(num),
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
            "matlab-runtime: operator '{other}' is not yet supported"
        )),
    }
}

/// Element-wise binary map with MATLAB scalar broadcasting (either operand may
/// be `1×1`); otherwise the shapes must agree.
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

fn unary_map(v: &MatValue, f: impl Fn(f64) -> f64) -> Result<MatValue, String> {
    let a = v.as_num("unary")?;
    Array::from_shape(a.data().iter().map(|&x| f(x)).collect(), a.shape().to_vec())
        .map(MatValue::Num)
}

/// Concatenate arrays horizontally (a matrix row): all must share their row
/// count; columns are appended. (`[1 2 3]`, `[A B]`.)
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
        // `[A A]` repeated (e.g. `A = [A A];` many times) doubles the
        // element count each time with no individually-large input --
        // only the ACCUMULATED result grows exponentially. Checked
        // incrementally, as the result is built, so the check fires
        // before the oversized intermediate `cols`/`data` is ever
        // allocated. Security regression, mirrors scilab-runtime's fix.
        crate::builtins::check_total_elements("matrix literal", nrows, cols.len() + a.ncols())?;
        for c in 0..a.ncols() {
            cols.push((0..nrows).map(|r| a.get(r, c).unwrap()).collect());
        }
    }
    // Assemble column-major.
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
        // See the matching comment in `hcat`: checked incrementally so the
        // oversized `data` buffer is never allocated.
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
fn scalar_index(v: &MatValue, len: usize, what: &str) -> Result<usize, String> {
    let a = v.as_num(what)?;
    let one = a.data().first().copied().unwrap_or(0.0);
    let idx = one.round() as i64;
    if idx < 1 || idx as usize > len {
        return Err(format!("{what} {idx} is out of bounds 1..={len}"));
    }
    Ok((idx - 1) as usize)
}

/// Resolve one subscript of a 2-D index into 0-based offsets along a dimension
/// of size `dim`. A colon means the whole dimension; a vector means several.
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
    first_node(node).ok_or_else(|| format!("matlab-runtime: malformed '{}' node", node.rule_name))
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

/// The variable name of an assignment LHS, if it is a simple `postfix → … →
/// primary → NAME` with no suffixes.
fn lhs_name(node: &GrammarASTNode) -> Option<String> {
    let mut cur = node;
    loop {
        if cur.rule_name == "primary" {
            return bare_name(cur);
        }
        match node_children(cur).as_slice() {
            [only] => cur = only,
            _ => return None,
        }
    }
}
