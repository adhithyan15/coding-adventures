//! The tree-walking evaluator.
//!
//! [`Interpreter`] walks the [`GrammarASTNode`] tree from `apl-parser` and
//! computes `array_runtime::Array` values. APL has exactly **two** expression
//! nonterminals (MA05 §3): `value_expr` (arrays/scalars) and `function_expr`
//! (a primitive glyph, optionally combined with the `/`/`\`/`∘.` operators
//! into a *derived function*). This evaluator mirrors that split:
//! [`Interpreter::eval_value_expr`] walks the value tree and calls
//! [`Interpreter::apply_monadic`]/[`Interpreter::apply_dyadic`] whenever it
//! meets a `function_expr`, which in turn dispatch on an internal [`AplFn`]
//! — the runtime's own representation of "which function, and with which
//! operator (if any) applied".
//!
//! The 12 primitive atoms that are ordinary scalar dyadic functions
//! (`+ - × ÷ ⌈ ⌊ = ≠ < ≤ ≥ >`) share `array_runtime::ops::BinOp` for both
//! their dyadic meaning (`ops::elementwise`) and reduce/scan/outer-product
//! (`ops::reduce`/`ops::scan`/`ops::outer`) — see `AplFn::Atom`. `⍴`/`⍳`/`,`
//! do not fit that shape at all (their monadic and dyadic meanings are
//! unrelated to each other), so they get bespoke logic in `builtins.rs`
//! instead (see `AplFn::NonScalar`).

use crate::builtins;
use crate::value::display;
use array_runtime::{ops, ops::BinOp, Array};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

/// Maximum recursion depth for this evaluator's own tree-walk. `apl-parser`'s
/// own `MAX_RULE_DEPTH` (see that crate's `lib.rs`) already bounds
/// how deep a CST built from untrusted input can possibly be, so this bound
/// can never actually trip on a tree that came from `try_parse_apl` — it
/// exists purely as **defense in depth**, exactly like every other runtime
/// crate in this repo (`matlab-runtime::eval::MAX_DEPTH`,
/// `wolfram-runtime`'s own guard): a from-scratch native-stack-floor
/// derivation would be redundant here, since the input tree is already
/// depth-bounded by construction before it ever reaches this evaluator.
const MAX_DEPTH: usize = 512;

/// A persistent APL session: a variable workspace and the current
/// evaluation depth.
pub struct Interpreter {
    vars: HashMap<String, Array>,
    depth: Rc<Cell<usize>>,
}

/// RAII guard that decrements the depth counter on every exit path
/// (including a `?` early return).
struct DepthGuard(Rc<Cell<usize>>);

impl Drop for DepthGuard {
    fn drop(&mut self) {
        self.0.set(self.0.get().saturating_sub(1));
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

/// A glyph one of `apl.tokens`' `NonScalarAtom` variant, kept around so error
/// messages can name the actual glyph (`"⍴"`, not `"RHO"`).
#[derive(Clone, Copy)]
enum NonScalarAtom {
    Rho,
    Iota,
    Ravel,
}

impl NonScalarAtom {
    fn glyph(self) -> &'static str {
        match self {
            NonScalarAtom::Rho => "⍴",
            NonScalarAtom::Iota => "⍳",
            NonScalarAtom::Ravel => ",",
        }
    }
}

/// The runtime's own representation of a `function_expr`: "which function,
/// and with which operator (if any) applied" (MA05 §3's "derived function").
enum AplFn {
    /// One of the 12 atoms that map onto `array_runtime::ops::BinOp`
    /// (`+ - × ÷ ⌈ ⌊ = ≠ < ≤ ≥ >`). There is exactly one glyph per `BinOp`
    /// variant (`CEILING` is the only atom that produces `Max`, `FLOOR` the
    /// only one that produces `Min`, etc.), so `BinOp` alone is enough to
    /// recover which glyph this was for monadic dispatch — no separate
    /// glyph tag needed here, unlike [`NonScalar`](AplFn::NonScalar).
    Atom(BinOp),
    /// `⍴`/`⍳`/`,` — bespoke monadic+dyadic logic (`builtins.rs`) that does
    /// not fit "an operator over a scalar dyadic function" at all, so it
    /// never plugs into reduce/scan/outer-product.
    NonScalar(NonScalarAtom),
    /// A `BinOp`-mappable atom with `/` (reduce) applied — inherently a
    /// *monadic* derived function (`+/A` reduces the one array `A`).
    Reduce(BinOp),
    /// A `BinOp`-mappable atom with `\` (scan) applied — also monadic.
    Scan(BinOp),
    /// A `BinOp`-mappable atom with `∘.` (outer product) applied —
    /// inherently *dyadic* (`A∘.×B` needs both arrays).
    Outer(BinOp),
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            vars: HashMap::new(),
            depth: Rc::new(Cell::new(0)),
        }
    }

    /// Enter one level of recursion, erroring if [`MAX_DEPTH`] is exceeded.
    /// The returned guard decrements the counter when it drops.
    fn enter(&self) -> Result<DepthGuard, String> {
        self.depth.set(self.depth.get() + 1);
        let guard = DepthGuard(Rc::clone(&self.depth));
        if self.depth.get() > MAX_DEPTH {
            return Err("apl-runtime: expression nesting too deep".to_string());
        }
        Ok(guard)
    }

    /// Evaluate a whole `program` node, returning the auto-print output for
    /// every statement that is *not* an assignment (MA05 §4: assignment is
    /// silent; a bare `value_expr` result auto-prints — real, textbook-
    /// accurate APL session behavior, not a MATLAB-style `;`-suppression).
    pub fn run(&mut self, program: &GrammarASTNode) -> Result<String, String> {
        let mut out = String::new();
        for line in node_children(program) {
            if line.rule_name != "line" {
                continue;
            }
            // A `line` with just a bare NEWLINE (blank line, or a
            // comment-only line — `⍝` comments are already stripped by the
            // lexer's skip pattern) has no `statement` child at all; skip it.
            let stmt = line.children.iter().find_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => Some(n),
                _ => None,
            });
            let Some(stmt) = stmt else { continue };
            if let Some(text) = self.eval_statement(stmt)? {
                out.push_str(&text);
                out.push('\n');
            }
        }
        Ok(out)
    }

    /// `statement = assignment` (a pure passthrough rule, always exactly one
    /// child) — evaluate the `assignment` and decide whether to print based
    /// on whether it was an *actual* assignment (`NAME ARROW …`, 3 children)
    /// or a plain `value_expr` passthrough (1 child).
    fn eval_statement(&mut self, stmt: &GrammarASTNode) -> Result<Option<String>, String> {
        let assignment = only_node(stmt)?;
        let is_assignment = assignment.children.len() == 3;
        let value = self.eval_assignment(assignment)?;
        if is_assignment {
            Ok(None)
        } else {
            Ok(Some(display(&value)))
        }
    }

    /// `assignment = NAME ARROW assignment | value_expr`. The right-hand
    /// side of a `NAME ARROW …` is not a restricted "simpler" rule — it
    /// recurses back through `assignment` itself, so `A←B←3` binds `3` to
    /// *both* `B` and `A` (right-associative chained assignment, MA05 §4).
    fn eval_assignment(&mut self, node: &GrammarASTNode) -> Result<Array, String> {
        let _guard = self.enter()?;
        if node.children.len() == 3 {
            let name = assignment_target_name(node)?;
            // `only_node` finds the lone `Node` child among
            // `[Token(NAME), Token(ARROW), Node(assignment)]` — the nested
            // `assignment` to recurse into.
            let inner = only_node(node)?;
            let value = self.eval_assignment(inner)?;
            self.vars.insert(name, value.clone());
            Ok(value)
        } else {
            let value_expr = only_node(node)?;
            self.eval_value_expr(value_expr)
        }
    }

    /// `value_expr`:
    /// - 1 child `[Node(term)]` — a bare term.
    /// - 2 children `[Node(function_expr), Node(value_expr)]` — monadic
    ///   application.
    /// - 3 children `[Node(term), Node(function_expr), Node(value_expr)]` —
    ///   dyadic application, right-recursive (`A+B+C` is `A+(B+C)`).
    fn eval_value_expr(&self, node: &GrammarASTNode) -> Result<Array, String> {
        let _guard = self.enter()?;
        let kids = node_children(node);
        match kids.len() {
            1 => self.eval_term(kids[0]),
            2 => {
                let f = parse_function_expr(kids[0])?;
                let arg = self.eval_value_expr(kids[1])?;
                self.apply_monadic(&f, &arg)
            }
            3 => {
                let lhs = self.eval_term(kids[0])?;
                let f = parse_function_expr(kids[1])?;
                let rhs = self.eval_value_expr(kids[2])?;
                self.apply_dyadic(&f, &lhs, &rhs)
            }
            n => Err(format!("apl-runtime: malformed value_expr with {n} children")),
        }
    }

    /// `term = NUMBER { NUMBER } | NAME | LPAREN value_expr RPAREN`.
    fn eval_term(&self, node: &GrammarASTNode) -> Result<Array, String> {
        match node.children.first() {
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "NUMBER" => {
                // "Stranding": one or more juxtaposed NUMBER tokens form a
                // single term — `1 2 3` is one 3-element vector, a lone
                // `5` is a rank-0 scalar (MA05 §4). Unlike every builtin in
                // `builtins.rs`, this literal-construction path has no
                // grammar-level depth bound on the *count* of stranded
                // numbers (`term`'s repetition is flat, not recursive, so
                // `apl-parser`'s `MAX_RULE_DEPTH` never sees it) — cap it
                // the same way, before doing any parsing work, so
                // `1 1 1 ... 1` (1.5M times) can't bypass every other
                // allocation cap in this crate just by using literal syntax.
                if node.children.len() > builtins::MAX_ARRAY_LENGTH {
                    return Err(format!(
                        "apl-runtime: stranded literal of {} numbers exceeds the cap of {} elements",
                        node.children.len(),
                        builtins::MAX_ARRAY_LENGTH
                    ));
                }
                let mut nums = Vec::new();
                for c in &node.children {
                    if let ASTNodeOrToken::Token(tok) = c {
                        nums.push(parse_apl_number(&tok.value)?);
                    }
                }
                if nums.len() == 1 {
                    Ok(Array::scalar(nums[0]))
                } else {
                    Ok(Array::from_vec(nums))
                }
            }
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "NAME" => self
                .vars
                .get(&t.value)
                .cloned()
                .ok_or_else(|| format!("apl-runtime: undefined variable '{}'", t.value)),
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "LPAREN" => {
                let inner = only_node(node)?; // the Node(value_expr) child
                self.eval_value_expr(inner)
            }
            _ => Err("apl-runtime: malformed term".to_string()),
        }
    }

    /// Apply a monadic (one-argument) function.
    fn apply_monadic(&self, f: &AplFn, x: &Array) -> Result<Array, String> {
        match f {
            AplFn::Atom(op) => apply_monadic_scalar(*op, x),
            AplFn::NonScalar(NonScalarAtom::Rho) => Ok(builtins::shape(x)),
            AplFn::NonScalar(NonScalarAtom::Iota) => builtins::index_generator(x),
            AplFn::NonScalar(NonScalarAtom::Ravel) => Ok(builtins::ravel(x)),
            AplFn::Reduce(op) => ops::reduce(*op, x),
            AplFn::Scan(op) => ops::scan(*op, x),
            AplFn::Outer(_) => Err(
                "apl-runtime: ∘. (outer product) needs two operands, but was applied monadically"
                    .to_string(),
            ),
        }
    }

    /// Apply a dyadic (two-argument) function.
    fn apply_dyadic(&self, f: &AplFn, a: &Array, b: &Array) -> Result<Array, String> {
        match f {
            AplFn::Atom(op) => ops::elementwise(*op, a, b),
            AplFn::NonScalar(NonScalarAtom::Rho) => builtins::reshape(a, b),
            AplFn::NonScalar(NonScalarAtom::Iota) => builtins::index_of(a, b),
            AplFn::NonScalar(NonScalarAtom::Ravel) => builtins::catenate(a, b),
            AplFn::Outer(op) => {
                // `ops::outer`'s own `checked_mul` only guards against usize
                // *overflow*, not an excessive-but-representable product —
                // (⍳1000000)∘.×(⍳1000000) would ask for 10^12 elements
                // without this check. Same cap and rationale as `⍳`/dyadic
                // `⍴` (builtins::MAX_ARRAY_LENGTH), checked before the
                // allocation, not after.
                let out_len = a.len().checked_mul(b.len()).filter(|&n| n <= builtins::MAX_ARRAY_LENGTH);
                if out_len.is_none() {
                    return Err(format!(
                        "∘.: outer product of {} and {} elements exceeds the cap of {} elements",
                        a.len(),
                        b.len(),
                        builtins::MAX_ARRAY_LENGTH
                    ));
                }
                ops::outer(*op, a, b)
            }
            AplFn::Reduce(_) => Err(
                "apl-runtime: / (reduce) takes exactly one operand, but was applied dyadically"
                    .to_string(),
            ),
            AplFn::Scan(_) => Err(
                "apl-runtime: \\ (scan) takes exactly one operand, but was applied dyadically"
                    .to_string(),
            ),
        }
    }
}

/// Monadic meaning of the six `BinOp`-mappable atoms that have one (MA05
/// §4): `+` conjugate (identity — this cut has no complex numbers, so
/// conjugate is a no-op), `-` negate, `×` sign, `÷` reciprocal, `⌈` ceiling,
/// `⌊` floor. The six comparisons (`= ≠ < ≤ ≥ >`, mapped onto
/// `Eq`/`Ne`/`Lt`/`Le`/`Ge`/`Gt`) have **no** monadic meaning in APL — a
/// clean, explicit error rather than silently picking a behavior.
fn apply_monadic_scalar(op: BinOp, x: &Array) -> Result<Array, String> {
    let f: fn(f64) -> f64 = match op {
        BinOp::Add => |v| v,
        BinOp::Sub => |v| -v,
        BinOp::Mul => apl_sign,
        BinOp::Div => |v| 1.0 / v,
        BinOp::Max => f64::ceil, // CEILING atom
        BinOp::Min => f64::floor, // FLOOR atom
        BinOp::Eq => return Err("apl-runtime: no monadic form for =".to_string()),
        BinOp::Ne => return Err("apl-runtime: no monadic form for ≠".to_string()),
        BinOp::Lt => return Err("apl-runtime: no monadic form for <".to_string()),
        BinOp::Le => return Err("apl-runtime: no monadic form for ≤".to_string()),
        BinOp::Ge => return Err("apl-runtime: no monadic form for ≥".to_string()),
        BinOp::Gt => return Err("apl-runtime: no monadic form for >".to_string()),
    };
    Ok(
        Array::from_shape(x.data().iter().map(|&v| f(v)).collect(), x.shape().to_vec())
            .expect("monadic map preserves shape/length"),
    )
}

/// APL's monadic `×` (sign): `1` for positive, `¯1` for negative, `0` for
/// zero. **Not** `f64::signum()` — that returns `1.0` for `0.0`, which is
/// wrong for APL's sign function (and `-1.0`/`1.0` for negative/positive
/// zero in a way that doesn't match APL's three-way sign either).
fn apl_sign(x: f64) -> f64 {
    if x.is_nan() {
        f64::NAN
    } else if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// Parse one `NUMBER` token's source text into an `f64`. APL's historical
/// "high minus" `¯` (U+00AF) stands in for `-` in a negative literal's
/// mantissa and/or exponent (`¯3`, `1.5E¯3`) — see `apl.tokens`' `NUMBER`
/// rule — so it is translated to ASCII `-` before handing the text to
/// Rust's `f64` parser, which does not know the glyph.
fn parse_apl_number(s: &str) -> Result<f64, String> {
    s.replace('¯', "-")
        .parse::<f64>()
        .map_err(|_| format!("apl-runtime: invalid number literal '{s}'"))
}

/// `function_expr`:
/// - 1 child `[Node(function_atom)]` — a plain function, no operator.
/// - 2 children `[Node(function_atom), Token(REDUCE|SCAN)]` — reduce/scan
///   (the operator token comes *second*).
/// - 2 children `[Token(OUTER), Node(function_atom)]` — outer product (the
///   operator token comes *first*). Both alternatives have 2 children, so
///   they are told apart by *which position* holds the token, not by
///   length alone.
fn parse_function_expr(node: &GrammarASTNode) -> Result<AplFn, String> {
    match node.children.as_slice() {
        [ASTNodeOrToken::Node(atom)] => parse_function_atom(atom),
        [ASTNodeOrToken::Node(atom), ASTNodeOrToken::Token(op)] => {
            match op.effective_type_name() {
                "REDUCE" => Ok(AplFn::Reduce(require_scalar_binop(atom, "reduce")?)),
                "SCAN" => Ok(AplFn::Scan(require_scalar_binop(atom, "scan")?)),
                other => Err(format!("apl-runtime: unexpected operator token '{other}'")),
            }
        }
        [ASTNodeOrToken::Token(_outer), ASTNodeOrToken::Node(atom)] => {
            Ok(AplFn::Outer(require_scalar_binop(atom, "outer")?))
        }
        _ => Err("apl-runtime: malformed function_expr".to_string()),
    }
}

/// `function_atom`: always exactly one child, a single token naming the
/// primitive glyph.
fn parse_function_atom(node: &GrammarASTNode) -> Result<AplFn, String> {
    let tok = match node.children.first() {
        Some(ASTNodeOrToken::Token(t)) => t,
        _ => return Err("apl-runtime: malformed function_atom".to_string()),
    };
    Ok(match tok.effective_type_name() {
        "PLUS" => AplFn::Atom(BinOp::Add),
        "MINUS" => AplFn::Atom(BinOp::Sub),
        "TIMES" => AplFn::Atom(BinOp::Mul),
        "DIVIDE" => AplFn::Atom(BinOp::Div),
        "CEILING" => AplFn::Atom(BinOp::Max),
        "FLOOR" => AplFn::Atom(BinOp::Min),
        "EQ" => AplFn::Atom(BinOp::Eq),
        "NE" => AplFn::Atom(BinOp::Ne),
        "LT" => AplFn::Atom(BinOp::Lt),
        "LE" => AplFn::Atom(BinOp::Le),
        "GE" => AplFn::Atom(BinOp::Ge),
        "GT" => AplFn::Atom(BinOp::Gt),
        "RHO" => AplFn::NonScalar(NonScalarAtom::Rho),
        "IOTA" => AplFn::NonScalar(NonScalarAtom::Iota),
        "RAVEL" => AplFn::NonScalar(NonScalarAtom::Ravel),
        other => return Err(format!("apl-runtime: unknown function atom '{other}'")),
    })
}

/// Reduce/scan/outer-product apply only to the 12 atoms that map onto a
/// `BinOp` — `⍴`/`⍳`/`,` are not "a scalar dyadic function" at all, so
/// stacking an operator on one of them is a clean, explicit scope error
/// (mirroring MA05 §4's "Deferred" list convention) rather than an attempt
/// to generalize reduce/scan/outer beyond their `array_runtime::ops` scope.
fn require_scalar_binop(atom: &GrammarASTNode, context: &str) -> Result<BinOp, String> {
    match parse_function_atom(atom)? {
        AplFn::Atom(op) => Ok(op),
        AplFn::NonScalar(a) => Err(format!(
            "{context}: {} is not a scalar dyadic function",
            a.glyph()
        )),
        AplFn::Reduce(_) | AplFn::Scan(_) | AplFn::Outer(_) => {
            unreachable!("parse_function_atom never produces an operator-bearing AplFn")
        }
    }
}

/// The `NAME` token of an actual assignment's target — the first child of a
/// 3-child `assignment` node (`[Token(NAME), Token(ARROW), Node(assignment)]`).
fn assignment_target_name(node: &GrammarASTNode) -> Result<String, String> {
    match node.children.first() {
        Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "NAME" => Ok(t.value.clone()),
        _ => Err("apl-runtime: malformed assignment (missing target name)".to_string()),
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
    first_node(node).ok_or_else(|| format!("apl-runtime: malformed '{}' node", node.rule_name))
}
