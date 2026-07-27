//! Lowering the C CST to Semantic IR (SIR27), with C's static integer typing.
//!
//! The heart of the frontend.  It walks the generic `GrammarASTNode` CST from
//! `c-parser`, keeps a symbol table `name → IntSpec`, assigns a concrete
//! integer type to every expression, and **inserts `Expr::Convert` nodes** at
//! the points C changes an integer's width — integer promotion, the usual
//! arithmetic conversions, assignment, cast, and call-argument/return.  Because
//! SIR arithmetic stays exact, the `Convert` after each width-bounded operation
//! is what reproduces C's fixed-width overflow at every step (see SIR27).
//!
//! Milestone 1: functions, typed integer `+`/`-`/`*`, casts, declarations,
//! `printf`, and a trailing `return`.
//!
//! Milestone 2 adds **comparisons and control flow** —
//! `< > <= >= == !=`, `if`/`else`, `while`, `for`, and re-assignment — bridging
//! the C-vs-SIR *truthiness mismatch* (C: `0` is false and a comparison yields
//! `int` `0`/`1`; SIR: only nil/false are falsy and comparisons yield `bool`).
//! A C condition lowers to a SIR bool (`!=(e, 0)`, or the comparison builtin
//! directly), and a comparison used as a value lowers back to an int via
//! `If(cmp, 1, 0)`.
//!
//! Milestone 3 (this revision) adds **early `return`**.  SIR functions have no
//! early-exit statement — they yield their block's value — so a returning `if`
//! is *lifted* into a value-producing `Expr::If` whose non-returning branch
//! continues with the rest of the function (see `lower_seq`).  That makes the
//! guard-clause shape, and hence idiomatic recursion like `fib`, translatable.
//! A `return` inside a loop still errors: it would need a break-with-value.

use std::collections::HashMap;

use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, IntSpec, IntWidth, Metadata,
    Module, Overflow, Param, ParamKind, Scope, SirType, Span, Stmt, CURRENT_SIR_VERSION,
};

/// A positioned lowering error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CLowerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for CLowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (at {}:{})", self.message, self.line, self.column)
    }
}

fn err<T>(message: impl Into<String>, node: &GrammarASTNode) -> Result<T, CLowerError> {
    Err(CLowerError {
        message: message.into(),
        line: node.start_line.unwrap_or(0),
        column: node.start_column.unwrap_or(0),
    })
}

// ── CST navigation helpers ──────────────────────────────────────────────────

fn child_nodes(n: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    n.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(x) => Some(x),
            _ => None,
        })
        .collect()
}

fn child_tokens(n: &GrammarASTNode) -> Vec<&lexer::token::Token> {
    n.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Token(t) => Some(t),
            _ => None,
        })
        .collect()
}

fn first_node<'a>(n: &'a GrammarASTNode, rule: &str) -> Option<&'a GrammarASTNode> {
    child_nodes(n).into_iter().find(|x| x.rule_name == rule)
}

/// Peel a single-child precedence-cascade chain down to the first node that
/// carries real content (an operator, a leaf token, or a parenthesised form).
fn peel(n: &GrammarASTNode) -> &GrammarASTNode {
    let mut cur = n;
    while cur.children.len() == 1 {
        match &cur.children[0] {
            ASTNodeOrToken::Node(x) => cur = x,
            ASTNodeOrToken::Token(_) => break,
        }
    }
    cur
}

// ── C type resolution (SIR27 table) ─────────────────────────────────────────

fn i32_spec() -> IntSpec {
    IntSpec::sized(IntWidth::W32, true, Overflow::Undefined)
}

/// Resolve a `type_spec` node (a sequence of type-keyword tokens) to an
/// `IntSpec`, or `None` for `void`.
fn resolve_type_spec(ts: &GrammarASTNode) -> Result<Option<IntSpec>, CLowerError> {
    let mut kws: Vec<String> = Vec::new();
    collect_type_kws(ts, &mut kws);
    if kws.iter().any(|k| k == "void") {
        return Ok(None);
    }
    // Fixed-width <stdint.h> names / size_t map directly.
    for k in &kws {
        let direct = match k.as_str() {
            "int8_t" => Some((IntWidth::W8, true)),
            "int16_t" => Some((IntWidth::W16, true)),
            "int32_t" => Some((IntWidth::W32, true)),
            "int64_t" => Some((IntWidth::W64, true)),
            "uint8_t" => Some((IntWidth::W8, false)),
            "uint16_t" => Some((IntWidth::W16, false)),
            "uint32_t" => Some((IntWidth::W32, false)),
            "uint64_t" => Some((IntWidth::W64, false)),
            "size_t" => Some((IntWidth::W64, false)),
            _ => None,
        };
        if let Some((w, signed)) = direct {
            return Ok(Some(spec_of(w, signed)));
        }
    }
    // Native specifier combination.  Signed unless `unsigned` appears (plain
    // `char` is treated as signed, per SIR27).
    let unsigned = kws.iter().any(|k| k == "unsigned");
    let width = if kws.iter().any(|k| k == "char") {
        IntWidth::W8
    } else if kws.iter().any(|k| k == "short") {
        IntWidth::W16
    } else if kws.iter().any(|k| k == "long") {
        IntWidth::W64 // long / long long modelled as 64-bit (LP64)
    } else {
        IntWidth::W32 // int (or a lone `unsigned`/`signed`)
    };
    Ok(Some(spec_of(width, !unsigned)))
}

fn spec_of(width: IntWidth, signed: bool) -> IntSpec {
    let overflow = if signed {
        Overflow::Undefined
    } else {
        Overflow::Wrap
    };
    IntSpec::sized(width, signed, overflow)
}

fn collect_type_kws(n: &GrammarASTNode, out: &mut Vec<String>) {
    for c in &n.children {
        match c {
            ASTNodeOrToken::Token(t) => out.push(t.value.clone()),
            ASTNodeOrToken::Node(x) => collect_type_kws(x, out),
        }
    }
}

// ── Integer promotion + usual arithmetic conversions ────────────────────────

fn rank(w: IntWidth) -> u8 {
    match w {
        IntWidth::W8 => 0,
        IntWidth::W16 => 1,
        IntWidth::W32 => 2,
        IntWidth::W64 => 3,
        IntWidth::W128 => 4,
        IntWidth::Arbitrary => 5,
    }
}

/// C integer promotion: anything narrower than `int` becomes `int` (i32).
fn promote(s: IntSpec) -> IntSpec {
    if rank(s.width) < rank(IntWidth::W32) {
        i32_spec()
    } else {
        s
    }
}

/// The usual arithmetic conversions on two *already-promoted* operands.
fn common_type(a: IntSpec, b: IntSpec) -> IntSpec {
    if a == b {
        return a;
    }
    let (wider, other) = if rank(a.width) >= rank(b.width) {
        (a, b)
    } else {
        (b, a)
    };
    if rank(wider.width) > rank(other.width) {
        return wider;
    }
    // Same width, differing signedness → unsigned wins.
    let unsigned = !a.signed || !b.signed;
    spec_of(wider.width, !unsigned)
}

// ── SIR node constructors ────────────────────────────────────────────────────

fn sp() -> Span {
    Span::synthetic()
}

fn int_lit(v: i64) -> Expr {
    Expr::IntLit {
        value: v,
        span: sp(),
    }
}

fn convert(e: Expr, to: IntSpec) -> Expr {
    Expr::Convert {
        value: Box::new(e),
        to,
        span: sp(),
    }
}

/// Convert `e` (currently of type `from`) to `to`, eliding the wrapper when the
/// type is unchanged (a no-op identity conversion).
fn convert_to(e: Expr, from: IntSpec, to: IntSpec) -> Expr {
    if from == to {
        e
    } else {
        convert(e, to)
    }
}

fn builtin(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall {
        name: name.to_string(),
        args,
        effects: EffectSet::PURE,
        span: sp(),
    }
}

/// A block that is just a value expression (no statements) — the branches of a
/// value-producing `If`, and the shape a nested `{ }` block reduces to.
fn value_block(value: Expr) -> Block {
    Block {
        stmts: vec![],
        value,
        span: sp(),
    }
}

/// Turn a SIR **bool** `cond` into the C **int** `0`/`1` it denotes as a value:
/// `If(cond, 1, 0)`.  This restores C's rule that a comparison has type `int`.
fn if_int(cond: Expr) -> Expr {
    Expr::If {
        cond: Box::new(cond),
        then_branch: Box::new(value_block(int_lit(1))),
        else_branch: Box::new(value_block(int_lit(0))),
        span: sp(),
    }
}

// ── The lowerer ──────────────────────────────────────────────────────────────

/// The deepest tree one function may emit, counting lifted guards *and*
/// expression nesting together — they add in the output, so they share a budget
/// (`budget_used`).
///
/// Same output-depth argument as [`MAX_LIFTED_GUARDS`]: every consumer of the IR
/// walks it recursively.  Expression depth comes from CST nesting (`((((x))))`)
/// *and* from a **flat operator chain**, which the parser does not bound at all:
/// `x + 1 + 1 + …` is one `additive` node with N operands that folds left into
/// an N-deep tree.  Both are charged here, and a chain's width is *held* while
/// its operands are lowered — checking without spending let widths at different
/// nesting levels multiply rather than add, which reached ~14× this cap and
/// aborted the process on a 369-byte input.
///
/// The value is empirical, calibrated against the most hostile realistic
/// configuration: a **debug** build on a **1 MiB** stack (the Windows
/// main-thread default — `cargo test` threads are larger, which is exactly how
/// earlier versions of this bound looked safe while crashing in the wild).
/// Ordinary C is far below it: an 8-term chain costs 8, a 3-deep `if` costs 9,
/// 20 guards with small conditions cost ~23.
const MAX_EXPR_DEPTH: usize = 48;

/// The most early-return `if`s one function may have lifted.
///
/// Each lifted guard adds a level of `Expr::If` nesting to the emitted IR, and
/// every consumer of that IR walks it **recursively** — the validator, all five
/// backends, the text printer, even `Drop`.  So the bound that matters is on the
/// *output* depth, not on the lowering (which is iterative).  Measured: a
/// 150-guard function lowers, validates and emits fine, while 250 aborts the
/// process inside the validator.  64 is comfortably clear of that and far beyond
/// any real C function, and exceeding it is a clean positioned error instead of
/// a stack-overflow crash on untrusted input.
/// Kept equal to [`MAX_EXPR_DEPTH`] — guards are charged into that joint budget
/// too, so this only adds a friendlier message for a pure guard chain.
const MAX_LIFTED_GUARDS: usize = MAX_EXPR_DEPTH;

struct Lowerer {
    /// Function signatures for call-site type resolution.
    fns: HashMap<String, (Vec<IntSpec>, Option<IntSpec>)>,
    /// Current function's in-scope names → (type, SIR scope).  Parameters bind
    /// as `Scope::Param`, local declarations as `Scope::Local`.
    vars: HashMap<String, (IntSpec, Scope)>,
    /// Early-return `if`s lifted in the function currently being lowered — see
    /// [`MAX_LIFTED_GUARDS`].  Reset per function.
    lifted: usize,
    /// Current expression nesting — see [`MAX_EXPR_DEPTH`].
    expr_depth: usize,
    /// Current *statement* nesting (`if`/`while`/`for` bodies and nested
    /// blocks) — the third dimension of the same budget.
    stmt_depth: usize,
}

pub fn compile(tree: &GrammarASTNode, module_name: &str) -> Result<Module, CLowerError> {
    let mut lo = Lowerer {
        fns: HashMap::new(),
        vars: HashMap::new(),
        lifted: 0,
        expr_depth: 0,
        stmt_depth: 0,
    };

    // Pre-pass: collect function signatures.
    for f in child_nodes(tree) {
        if f.rule_name != "function_def" {
            continue;
        }
        let (name, params, ret) = lo.function_header(f)?;
        lo.fns
            .insert(name, (params.iter().map(|(_, t)| *t).collect(), ret));
    }

    let mut functions = Vec::new();
    for f in child_nodes(tree) {
        if f.rule_name == "function_def" {
            functions.push(lo.lower_function(f)?);
        }
    }

    let manifest = FeatureManifest::from_features(&[
        Feature::OptionalTypeAnnotations,
        Feature::MutualRecursion,
        Feature::Conversions,
        Feature::SizedIntegers,
        Feature::Unsigned,
        Feature::WrappingArithmetic,
        // Milestone 2: `while`/`for` → `Stmt::While` (Loops); re-assignment →
        // `Stmt::Assign` (MutableBindings).  `Expr::If` and the comparison
        // builtins need no feature of their own (core control flow / intrinsics).
        Feature::Loops,
        Feature::MutableBindings,
        // Milestone 4: `&&`/`||`/`!` lower to the short-circuiting `and`/`or`/
        // `not` builtins.
        Feature::ShortCircuit,
    ]);

    let module = Module {
        name: module_name.to_string(),
        manifest,
        imports: vec![],
        exports: vec![],
        functions,
        globals: vec![],
        metadata: Metadata::new()
            .with_sir_version(CURRENT_SIR_VERSION)
            .with_source_language("c"),
        span: sp(),
    };

    let result = semantic_ir::validate(&module);
    if !result.is_ok() {
        let first = result
            .issues
            .iter()
            .find(|i| i.severity == semantic_ir::Severity::Error);
        return Err(CLowerError {
            message: format!(
                "produced an invalid SIR module: {}",
                first
                    .map(|i| i.message.as_str())
                    .unwrap_or("validation failed")
            ),
            line: 0,
            column: 0,
        });
    }
    Ok(module)
}

impl Lowerer {
    /// Extract `(name, [(param_name, type)], return_type)` from a function_def.
    // Returns (function name, parameter (name, type) pairs, optional return type)
    // — a one-off internal tuple; a named type alias would not aid readability.
    #[allow(clippy::type_complexity)]
    fn function_header(
        &self,
        f: &GrammarASTNode,
    ) -> Result<(String, Vec<(String, IntSpec)>, Option<IntSpec>), CLowerError> {
        let toks = child_tokens(f);
        let name = toks
            .iter()
            .find(|t| t.effective_type_name() == "NAME")
            .map(|t| t.value.clone())
            .ok_or_else(|| CLowerError {
                message: "function has no name".into(),
                line: f.start_line.unwrap_or(0),
                column: f.start_column.unwrap_or(0),
            })?;
        // The first type_spec child is the return type.
        let ret = match first_node(f, "type_spec") {
            Some(ts) => resolve_type_spec(ts)?,
            None => None,
        };
        let mut params = Vec::new();
        if let Some(pl) = first_node(f, "param_list") {
            for p in child_nodes(pl) {
                if p.rule_name == "param" {
                    let pts = first_node(p, "type_spec").ok_or_else(|| CLowerError {
                        message: "param without type".into(),
                        line: p.start_line.unwrap_or(0),
                        column: p.start_column.unwrap_or(0),
                    })?;
                    let ty = resolve_type_spec(pts)?.ok_or_else(|| CLowerError {
                        message: "void parameter".into(),
                        line: p.start_line.unwrap_or(0),
                        column: p.start_column.unwrap_or(0),
                    })?;
                    let pname = child_tokens(p)
                        .iter()
                        .find(|t| t.effective_type_name() == "NAME")
                        .map(|t| t.value.clone())
                        .unwrap_or_default();
                    params.push((pname, ty));
                }
            }
        }
        Ok((name, params, ret))
    }

    fn lower_function(&mut self, f: &GrammarASTNode) -> Result<Function, CLowerError> {
        let (name, params, ret) = self.function_header(f)?;
        self.vars.clear();
        self.lifted = 0;
        let mut sir_params = Vec::new();
        for (pname, ty) in &params {
            self.vars.insert(pname.clone(), (*ty, Scope::Param));
            sir_params.push(Param {
                name: pname.clone(),
                sir_type: Some(SirType::Int(*ty)),
                kind: ParamKind::Required,
                default: None,
                span: sp(),
            });
        }
        let body_node = first_node(f, "compound_stmt").ok_or_else(|| CLowerError {
            message: "function has no body".into(),
            line: f.start_line.unwrap_or(0),
            column: f.start_column.unwrap_or(0),
        })?;
        let body = self.lower_body(body_node, ret)?;
        Ok(Function {
            name,
            params: sir_params,
            return_type: ret.map(SirType::Int),
            captures: vec![],
            body,
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: sp(),
        })
    }

    /// Lower a `compound_stmt` as a function body.
    fn lower_body(
        &mut self,
        body: &GrammarASTNode,
        ret: Option<IntSpec>,
    ) -> Result<Block, CLowerError> {
        let items = block_items_of(body);
        self.lower_seq(&items, ret)
    }

    /// **The early-return transformation (milestone 3).**
    ///
    /// SIR functions have no early-exit statement — a function yields its
    /// block's *value*.  C exits early all the time (guard clauses).  So we
    /// lower a statement sequence to the block whose value *is* the function's
    /// result, and when an `if` returns we make the **rest of the sequence the
    /// continuation of the branch that doesn't return**:
    ///
    /// ```text
    /// if (n < 2) return n;          If( n<2,
    /// return fib(n-1)+fib(n-2);  →      {n},                    // then: returns
    ///                                   {fib(n-1)+fib(n-2)} )   // else: the rest
    /// ```
    ///
    /// Because the continuation attaches only to a branch that does *not*
    /// already return, the common guard-clause shape never duplicates code.
    /// (When *neither* branch returns on all paths yet one contains a return,
    /// the tail is lowered into both branches — correct, since exactly one runs,
    /// just larger.)
    fn lower_seq(
        &mut self,
        items: &[&GrammarASTNode],
        ret: Option<IntSpec>,
    ) -> Result<Block, CLowerError> {
        // This walk is **iterative in two dimensions**, and both matter:
        //
        //  * per *statement* — a function body is an unbounded statement list,
        //    and recursing per statement overflowed the stack at a few hundred;
        //  * per *sibling guard clause* — `if (a) return 1; if (b) return 2; …`
        //    is the `sign()` idiom, and it is a flat sequence too.  Recursing
        //    once per guard (lower_seq → lift_if → lower_branch → lower_seq)
        //    overflowed at ~200 guards.
        //
        // So a returning `if` does not recurse into the continuation.  Instead
        // its condition and its *returning* branch are pushed on a `frames`
        // stack, the falling-through branch is spliced onto the work queue, and
        // the nested `Expr::If` is folded bottom-up after the loop.  The only
        // recursion left is into a *nested* sub-sequence (a returning branch),
        // whose depth is bounded only incidentally — the C parser is itself
        // recursive and dies on deeply nested statements before the lowering
        // would, so this is not a guarantee to lean on.  Giving the parser a
        // rule-depth counter (and this walk an explicit cap) is a follow-up.
        struct Frame {
            cond: Expr,
            /// The branch that returns — already lowered.
            branch: Block,
            /// Is `branch` the `then` side (else the `else` side)?
            branch_is_then: bool,
            /// Statements that preceded this `if` at its own level.
            before: Vec<Stmt>,
        }

        let mut work: std::collections::VecDeque<&GrammarASTNode> = items.iter().copied().collect();
        let mut stmts: Vec<Stmt> = Vec::new();
        let mut frames: Vec<Frame> = Vec::new();
        // The value of the innermost block, once the walk finishes.  Falling off
        // the end of a function yields nil.
        let mut value = Expr::NilLit { span: sp() };

        while let Some(head) = work.pop_front() {
            let e = seq_elem(head);
            match e.rule_name.as_str() {
                // `return e;` supplies the value — anything after it is dead.
                "return_stmt" => {
                    value = self.lower_return(e, ret)?;
                    break;
                }
                // A nested `{ … }` splices into this sequence, in order (v1 has
                // flat scoping).  Spliced in place rather than by recursing.
                "compound_stmt" => {
                    for it in block_items_of(e).into_iter().rev() {
                        work.push_front(it);
                    }
                }
                // An `if` that returns becomes a value-producing `If`.
                "if_stmt" if if_returns(e) => {
                    let cond_node = first_node(e, "expr").ok_or_else(|| CLowerError {
                        message: "`if` without a condition".into(),
                        line: e.start_line.unwrap_or(0),
                        column: e.start_column.unwrap_or(0),
                    })?;
                    let cond = self.lower_cond(cond_node)?;
                    let branches = if_branches(e);
                    let then_items = branches
                        .first()
                        .map(|b| branch_items(b))
                        .unwrap_or_default();
                    let else_items = branches.get(1).map(|b| branch_items(b)).unwrap_or_default();
                    let then_ret = always_returns(&then_items);
                    let else_ret = always_returns(&else_items);

                    // Bound the *emitted* nesting — see `MAX_LIFTED_GUARDS`.
                    self.lifted += 1;
                    if self.lifted > MAX_LIFTED_GUARDS {
                        return err(
                            format!(
                                "too many early returns in one function (limit \
                                 {MAX_LIFTED_GUARDS}); each one nests the emitted IR one \
                                 level deeper, and every consumer of that IR walks it \
                                 recursively"
                            ),
                            e,
                        );
                    }

                    // Nothing left to thread (or both branches exit): lower both
                    // branches directly.  Any queued statements are unreachable.
                    if work.is_empty() || (then_ret && else_ret) {
                        let saved = self.vars.clone();
                        let tb = self.lower_seq(&then_items, ret)?;
                        self.vars = saved.clone();
                        let eb = self.lower_seq(&else_items, ret)?;
                        self.vars = saved;
                        value = Expr::If {
                            cond: Box::new(cond),
                            then_branch: Box::new(tb),
                            else_branch: Box::new(eb),
                            span: sp(),
                        };
                        break;
                    }

                    // Neither branch exits on every path, so the continuation
                    // would have to be copied into *both*.  Correct but the
                    // duplication compounds through nesting (4^N nodes), so it
                    // is refused, like the `return`-inside-a-loop rule.
                    if !then_ret && !else_ret {
                        return err(
                            "an `if` where neither branch returns on all paths but one \
                             contains a `return` is not supported yet (lifting it would \
                             duplicate the rest of the function into both branches)",
                            e,
                        );
                    }

                    // Exactly one branch exits; the other receives the
                    // continuation and is spliced onto the work queue.
                    let (ret_items, fall_items) = if then_ret {
                        (then_items, else_items)
                    } else {
                        (else_items, then_items)
                    };

                    // (A declaration in the falling-through branch that shadows
                    // an outer name would silently re-bind it for the
                    // continuation, which is lowered inside that branch.  That
                    // is caught centrally by `lower_init_declarator`, which
                    // refuses any re-use of a live name.)

                    // Lower the exiting branch now (recursion here is per
                    // *nesting* level; see the note in `lower_seq`).
                    let saved = self.vars.clone();
                    let branch = self.lower_seq(&ret_items, ret)?;
                    self.vars = saved;

                    frames.push(Frame {
                        cond,
                        branch,
                        branch_is_then: then_ret,
                        before: std::mem::take(&mut stmts),
                    });
                    for it in fall_items.into_iter().rev() {
                        work.push_front(it);
                    }
                }
                // Anything else is an ordinary statement; lower it and continue.
                "declaration" => stmts.push(self.lower_declaration(e)?),
                _ => self.lower_stmt(e, &mut stmts)?,
            }
        }

        // Fold the guards bottom-up: each frame wraps everything accumulated so
        // far as the continuation of its non-returning side.
        let mut cur = Block {
            stmts,
            value,
            span: sp(),
        };
        for f in frames.into_iter().rev() {
            let (then_branch, else_branch) = if f.branch_is_then {
                (f.branch, cur)
            } else {
                (cur, f.branch)
            };
            cur = Block {
                stmts: f.before,
                value: Expr::If {
                    cond: Box::new(f.cond),
                    then_branch: Box::new(then_branch),
                    else_branch: Box::new(else_branch),
                    span: sp(),
                },
                span: sp(),
            };
        }
        Ok(cur)
    }

    /// The value a trailing `return e;` (or bare `return;`) supplies, converted
    /// to the function's declared return type.
    fn lower_return(
        &mut self,
        s: &GrammarASTNode,
        ret: Option<IntSpec>,
    ) -> Result<Expr, CLowerError> {
        Ok(match first_node(s, "expr") {
            Some(e) => {
                let (ex, ty) = self.lower_expr(e)?;
                match ret {
                    Some(rt) => convert_to(ex, ty, rt),
                    None => ex,
                }
            }
            None => Expr::NilLit { span: sp() },
        })
    }

    // ── statements (milestone 2) ─────────────────────────────────────────────

    /// Lower one *statement-kind* node (already peeled from `statement`),
    /// appending the resulting SIR statement(s) to `out`.
    fn lower_stmt(&mut self, s: &GrammarASTNode, out: &mut Vec<Stmt>) -> Result<(), CLowerError> {
        match s.rule_name.as_str() {
            "compound_stmt" => {
                // A nested `{ … }` block: splice its statements in (v1 shares one
                // flat symbol table — no per-block scoping).  Charged for depth
                // like any other nested body, so this recursion shares the
                // budget too (see `charge_stmt_nesting`).
                let inner = self.charge_stmt_nesting(s, |lo| lo.lower_block_items(s))?;
                out.extend(inner);
            }
            "expr_stmt" => {
                if let Some(e) = first_node(s, "expr") {
                    self.lower_expr_stmt(e, out)?;
                }
            }
            "if_stmt" => self.lower_if(s, out)?,
            "while_stmt" => self.lower_while(s, out)?,
            "for_stmt" => self.lower_for(s, out)?,
            // `return` in a *statement* position that the early-return lifting
            // could not turn into a value — in practice, inside a loop body.
            // Exiting a loop early needs a break-with-value, which SIR has no
            // node for, so this is a clear error rather than a miscompile.
            "return_stmt" => {
                return err(
                    "`return` inside a loop is not supported yet (early return is lifted \
                     through `if`/`else`, but leaving a `while`/`for` early needs a \
                     break-with-value)",
                    s,
                )
            }
            other => return err(format!("statement `{other}` not supported"), s),
        }
        Ok(())
    }

    /// Lower every `block_item` of a `compound_stmt` into a flat `Vec<Stmt>`.
    fn lower_block_items(&mut self, compound: &GrammarASTNode) -> Result<Vec<Stmt>, CLowerError> {
        let mut out = Vec::new();
        for item in child_nodes(compound) {
            if item.rule_name != "block_item" {
                continue;
            }
            let inner = peel_block_item(item);
            match inner.rule_name.as_str() {
                "declaration" => out.push(self.lower_declaration(inner)?),
                "statement" => {
                    let s = peel(inner);
                    self.lower_stmt(s, &mut out)?;
                }
                other => return err(format!("block item `{other}` unsupported"), inner),
            }
        }
        Ok(out)
    }

    /// Lower a loop/branch body (`statement`, possibly a `compound_stmt`) to a
    /// SIR `Block` with a nil value — control-flow bodies are evaluated for
    /// their effects, not a value.
    /// Run `f` one level deeper in statement nesting, charged against the joint
    /// depth budget (see [`Self::budget_used`]) and restored on every path.
    /// Both recursion sites for nested statements — loop/branch bodies via
    /// [`Self::lower_void_block`] and bare `{ }` blocks via [`Self::lower_stmt`]
    /// — go through here, so neither can grow the emitted tree without paying.
    fn charge_stmt_nesting<T>(
        &mut self,
        at: &GrammarASTNode,
        f: impl FnOnce(&mut Self) -> Result<T, CLowerError>,
    ) -> Result<T, CLowerError> {
        self.stmt_depth += 1;
        if self.budget_used() > MAX_EXPR_DEPTH {
            self.stmt_depth -= 1;
            return err(
                format!("statement nesting exceeds the limit of {MAX_EXPR_DEPTH}"),
                at,
            );
        }
        let result = f(self);
        self.stmt_depth -= 1;
        result
    }

    fn lower_void_block(&mut self, stmt: &GrammarASTNode) -> Result<Block, CLowerError> {
        self.charge_stmt_nesting(stmt, |lo| lo.lower_void_block_inner(stmt))
    }

    fn lower_void_block_inner(&mut self, stmt: &GrammarASTNode) -> Result<Block, CLowerError> {
        let s = peel(stmt);
        let stmts = if s.rule_name == "compound_stmt" {
            self.lower_block_items(s)?
        } else {
            let mut v = Vec::new();
            self.lower_stmt(s, &mut v)?;
            v
        };
        Ok(Block {
            stmts,
            value: Expr::NilLit { span: sp() },
            span: sp(),
        })
    }

    /// An expression used as a statement: an assignment `x = e` becomes
    /// `Stmt::Assign`, anything else a bare `ExprStmt` (evaluated for effect).
    fn lower_expr_stmt(
        &mut self,
        expr_node: &GrammarASTNode,
        out: &mut Vec<Stmt>,
    ) -> Result<(), CLowerError> {
        let p = peel(expr_node);
        if p.rule_name == "assignment" && child_tokens(p).iter().any(|t| t.value == "=") {
            out.push(self.lower_assignment(p)?);
        } else {
            let (ex, _) = self.lower_expr(expr_node)?;
            out.push(Stmt::ExprStmt {
                expr: ex,
                span: sp(),
            });
        }
        Ok(())
    }

    /// `x = e` → `Stmt::Assign` with the RHS converted to `x`'s declared type.
    /// The LHS must be a bare, already-declared name (the only lvalue in v1).
    fn lower_assignment(&mut self, n: &GrammarASTNode) -> Result<Stmt, CLowerError> {
        let nodes = child_nodes(n);
        let lhs = nodes.first().copied().ok_or_else(|| CLowerError {
            message: "assignment without a left-hand side".into(),
            line: n.start_line.unwrap_or(0),
            column: n.start_column.unwrap_or(0),
        })?;
        let name = child_tokens(peel(lhs))
            .iter()
            .find(|t| t.effective_type_name() == "NAME")
            .map(|t| t.value.clone())
            .ok_or_else(|| CLowerError {
                message: "assignment target is not a plain variable".into(),
                line: n.start_line.unwrap_or(0),
                column: n.start_column.unwrap_or(0),
            })?;
        let (ty, scope) = self.vars.get(&name).copied().ok_or_else(|| CLowerError {
            message: format!("assignment to undeclared variable `{name}`"),
            line: n.start_line.unwrap_or(0),
            column: n.start_column.unwrap_or(0),
        })?;
        let rhs = nodes.get(1).copied().ok_or_else(|| CLowerError {
            message: "assignment without a right-hand side".into(),
            line: n.start_line.unwrap_or(0),
            column: n.start_column.unwrap_or(0),
        })?;
        let (e, et) = self.lower_expr(rhs)?;
        Ok(Stmt::Assign {
            name,
            scope,
            value: convert_to(e, et, ty),
            span: sp(),
        })
    }

    /// `if (c) S1 [else S2]` → an `If` expression evaluated as a statement.
    fn lower_if(&mut self, s: &GrammarASTNode, out: &mut Vec<Stmt>) -> Result<(), CLowerError> {
        let cond_node = first_node(s, "expr").ok_or_else(|| CLowerError {
            message: "`if` without a condition".into(),
            line: s.start_line.unwrap_or(0),
            column: s.start_column.unwrap_or(0),
        })?;
        let cond = self.lower_cond(cond_node)?;
        let branches: Vec<&GrammarASTNode> = child_nodes(s)
            .into_iter()
            .filter(|x| x.rule_name == "statement")
            .collect();
        let then_branch =
            self.lower_void_block(branches.first().ok_or_else(|| CLowerError {
                message: "`if` without a body".into(),
                line: s.start_line.unwrap_or(0),
                column: s.start_column.unwrap_or(0),
            })?)?;
        let else_branch = match branches.get(1) {
            Some(e) => self.lower_void_block(e)?,
            None => Block {
                stmts: vec![],
                value: Expr::NilLit { span: sp() },
                span: sp(),
            },
        };
        out.push(Stmt::ExprStmt {
            expr: Expr::If {
                cond: Box::new(cond),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
                span: sp(),
            },
            span: sp(),
        });
        Ok(())
    }

    /// `while (c) S` → `Stmt::While`.
    fn lower_while(&mut self, s: &GrammarASTNode, out: &mut Vec<Stmt>) -> Result<(), CLowerError> {
        let cond_node = first_node(s, "expr").ok_or_else(|| CLowerError {
            message: "`while` without a condition".into(),
            line: s.start_line.unwrap_or(0),
            column: s.start_column.unwrap_or(0),
        })?;
        let cond = self.lower_cond(cond_node)?;
        let body_node = first_node(s, "statement").ok_or_else(|| CLowerError {
            message: "`while` without a body".into(),
            line: s.start_line.unwrap_or(0),
            column: s.start_column.unwrap_or(0),
        })?;
        let body = self.lower_void_block(body_node)?;
        out.push(Stmt::While {
            cond,
            body,
            span: sp(),
        });
        Ok(())
    }

    /// `for (init; cond; step) S` desugars to `init; while (cond) { S; step }`.
    /// Children are walked in order, bucketed by the two `;` separators.
    fn lower_for(&mut self, s: &GrammarASTNode, out: &mut Vec<Stmt>) -> Result<(), CLowerError> {
        let (mut init, mut cond, mut step, mut body) = (None, None, None, None);
        let mut semicolons = 0;
        for child in &s.children {
            match child {
                ASTNodeOrToken::Token(t) if t.value == ";" => semicolons += 1,
                ASTNodeOrToken::Token(_) => {}
                ASTNodeOrToken::Node(x) => match x.rule_name.as_str() {
                    "for_clause" => init = Some(x),
                    "statement" => body = Some(x),
                    "expr" if semicolons == 1 => cond = Some(x),
                    "expr" => step = Some(x),
                    _ => {}
                },
            }
        }

        // init clause: a declaration (`int i = 0`) or an expression (`i = 0`).
        if let Some(fc) = init {
            let inner = child_nodes(fc)
                .into_iter()
                .next()
                .ok_or_else(|| CLowerError {
                    message: "empty `for` init clause".into(),
                    line: fc.start_line.unwrap_or(0),
                    column: fc.start_column.unwrap_or(0),
                })?;
            if inner.rule_name == "init_declarator" {
                out.push(self.lower_init_declarator(inner)?);
            } else {
                self.lower_expr_stmt(inner, out)?;
            }
        }

        // condition: absent means `true` (an unconditional loop).
        let cond_expr = match cond {
            Some(c) => self.lower_cond(c)?,
            None => Expr::BoolLit {
                value: true,
                span: sp(),
            },
        };

        // body then the step expression, all inside the loop.
        let body_node = body.ok_or_else(|| CLowerError {
            message: "`for` without a body".into(),
            line: s.start_line.unwrap_or(0),
            column: s.start_column.unwrap_or(0),
        })?;
        let mut body_block = self.lower_void_block(body_node)?;
        if let Some(st) = step {
            self.lower_expr_stmt(st, &mut body_block.stmts)?;
        }
        out.push(Stmt::While {
            cond: cond_expr,
            body: body_block,
            span: sp(),
        });
        Ok(())
    }

    // ── conditions & comparisons (milestone 2) ───────────────────────────────

    /// Lower a C condition expression to a SIR **bool**, bridging truthiness.
    /// A syntactic comparison yields a bool directly; any other integer
    /// expression `e` becomes `!=(e, 0)` (C: non-zero is true; SIR: 0 is truthy,
    /// so the explicit compare is required).
    fn lower_cond(&mut self, node: &GrammarASTNode) -> Result<Expr, CLowerError> {
        let n = peel(node);
        match n.rule_name.as_str() {
            // Short-circuiting `&&` / `||`: fold the operands *as conditions*
            // with the matching SIR builtin (left-associative, like C).
            "logical_and" if child_nodes(n).len() >= 2 => self.lower_logical(n, "and"),
            "logical_or" if child_nodes(n).len() >= 2 => self.lower_logical(n, "or"),
            // A comparison already yields a SIR bool.
            "equality" | "relational" if child_nodes(n).len() >= 2 => self.lower_compare_bool(n),
            // Unary `!c` → `not(cond(c))`.  Charged against the depth budget:
            // `!!!…c` recurses here per `!`, and nests the emitted tree the same.
            "unary" if unary_op(n).as_deref() == Some("!") => {
                let operand = child_nodes(n)
                    .into_iter()
                    .next()
                    .ok_or_else(|| CLowerError {
                        message: "`!` without an operand".into(),
                        line: n.start_line.unwrap_or(0),
                        column: n.start_column.unwrap_or(0),
                    })?;
                self.expr_depth += 1;
                if self.budget_used() > MAX_EXPR_DEPTH {
                    self.expr_depth -= 1;
                    return err(
                        format!("expression nests deeper than the limit of {MAX_EXPR_DEPTH}"),
                        n,
                    );
                }
                let inner = self.lower_cond(operand);
                self.expr_depth -= 1;
                Ok(builtin("not", vec![inner?]))
            }
            // Any other integer expression `e` is true iff `e != 0` (C treats 0
            // as false; SIR treats it as truthy).
            _ => {
                let (e, _t) = self.lower_expr(node)?;
                Ok(builtin("!=", vec![e, int_lit(0)]))
            }
        }
    }

    /// Fold a `logical_and`/`logical_or` node into left-associative short-circuit
    /// builtins, lowering each operand *as a condition*.  Its width is charged
    /// against the depth budget (the fold nests as deep as the chain is wide).
    fn lower_logical(&mut self, n: &GrammarASTNode, op: &str) -> Result<Expr, CLowerError> {
        let width = self.charge_chain(n)?;
        self.expr_depth += width;
        let result = self.lower_logical_inner(n, op);
        self.expr_depth -= width;
        result
    }

    fn lower_logical_inner(&mut self, n: &GrammarASTNode, op: &str) -> Result<Expr, CLowerError> {
        let mut acc: Option<Expr> = None;
        for c in &n.children {
            if let ASTNodeOrToken::Node(operand) = c {
                let cond = self.lower_cond(operand)?;
                acc = Some(match acc {
                    None => cond,
                    Some(a) => builtin(op, vec![a, cond]),
                });
            }
        }
        acc.ok_or_else(|| CLowerError {
            message: "empty logical expression".into(),
            line: n.start_line.unwrap_or(0),
            column: n.start_column.unwrap_or(0),
        })
    }

    /// Lower an `equality`/`relational` node to a SIR bool.  Left-associative:
    /// an intermediate comparison result feeds the next as an `int` `0`/`1`
    /// (matching C's chained-comparison semantics), and the final step is the
    /// bool we return.
    /// Charge a flat operator chain's width against the expression-depth
    /// budget: folding N operands left produces an N-deep tree.  Returns the
    /// width, which the caller must **hold** (add to `expr_depth`) for as long
    /// as it lowers the chain's operands — otherwise widths at different
    /// nesting levels each restart from the same low base and multiply instead
    /// of adding, which is how `((x+1…) +1…) +1…` reached ~14× the cap.
    fn charge_chain(&mut self, n: &GrammarASTNode) -> Result<usize, CLowerError> {
        let operands = child_nodes(n).len();
        if self.budget_used() + operands > MAX_EXPR_DEPTH {
            return err(
                format!(
                    "operator chain of {operands} operands nests the emitted expression \
                     deeper than the limit of {MAX_EXPR_DEPTH}"
                ),
                n,
            );
        }
        Ok(operands)
    }

    /// Depth already committed in the emitted tree.  Lifted guards, expression
    /// nesting and **statement** nesting all add in the same output tree and in
    /// the same recursive walk, so they share one budget.  Statement nesting is
    /// weighted 3× because a level of it costs roughly three times the lowering
    /// stack of one expression level (measured ~23 KB vs ~8 KB in a debug
    /// build).
    fn budget_used(&self) -> usize {
        self.lifted + self.expr_depth + 3 * self.stmt_depth
    }

    fn lower_compare_bool(&mut self, n: &GrammarASTNode) -> Result<Expr, CLowerError> {
        let width = self.charge_chain(n)?;
        self.expr_depth += width;
        let result = self.lower_compare_bool_inner(n);
        self.expr_depth -= width;
        result
    }

    fn lower_compare_bool_inner(&mut self, n: &GrammarASTNode) -> Result<Expr, CLowerError> {
        let mut acc: Option<(Expr, IntSpec)> = None;
        let mut pending_op: Option<String> = None;
        let mut last_bool: Option<Expr> = None;
        for c in &n.children {
            match c {
                ASTNodeOrToken::Node(operand) => {
                    let (e, t) = self.lower_expr(operand)?;
                    match acc.take() {
                        None => acc = Some((e, t)),
                        Some((le, lt)) => {
                            let op = pending_op.take().ok_or_else(|| CLowerError {
                                message: "comparison operands with no operator between them".into(),
                                line: n.start_line.unwrap_or(0),
                                column: n.start_column.unwrap_or(0),
                            })?;
                            let b = self.compare(&op, le, lt, e, t)?;
                            last_bool = Some(b.clone());
                            acc = Some((if_int(b), i32_spec()));
                        }
                    }
                }
                ASTNodeOrToken::Token(t) => pending_op = Some(t.value.clone()),
            }
        }
        last_bool.ok_or_else(|| CLowerError {
            message: "comparison without an operator".into(),
            line: n.start_line.unwrap_or(0),
            column: n.start_column.unwrap_or(0),
        })
    }

    /// Emit a comparison builtin, applying the usual arithmetic conversions to
    /// the operands first (like arithmetic).  The result is a SIR bool.
    fn compare(
        &self,
        op: &str,
        le: Expr,
        lt: IntSpec,
        re: Expr,
        rt: IntSpec,
    ) -> Result<Expr, CLowerError> {
        let lp = promote(lt);
        let rp = promote(rt);
        let le = convert_to(le, lt, lp);
        let re = convert_to(re, rt, rp);
        let c = common_type(lp, rp);
        let le = convert_to(le, lp, c);
        let re = convert_to(re, rp, c);
        match op {
            "<" | ">" | "<=" | ">=" | "==" | "!=" => Ok(builtin(op, vec![le, re])),
            other => Err(CLowerError {
                message: format!("comparison operator `{other}` unsupported"),
                line: 0,
                column: 0,
            }),
        }
    }

    fn lower_declaration(&mut self, decl: &GrammarASTNode) -> Result<Stmt, CLowerError> {
        let init = first_node(decl, "init_declarator").ok_or_else(|| CLowerError {
            message: "declaration without declarator".into(),
            line: decl.start_line.unwrap_or(0),
            column: decl.start_column.unwrap_or(0),
        })?;
        self.lower_init_declarator(init)
    }

    /// Lower an `init_declarator` (`T name [= expr]`) to a `LetStarBinding`.
    /// Shared by ordinary declarations and `for`-init clauses.
    fn lower_init_declarator(&mut self, init: &GrammarASTNode) -> Result<Stmt, CLowerError> {
        let ts = first_node(init, "type_spec").ok_or_else(|| CLowerError {
            message: "declaration without a type specifier".into(),
            line: init.start_line.unwrap_or(0),
            column: init.start_column.unwrap_or(0),
        })?;
        let ty = resolve_type_spec(ts)?.ok_or_else(|| CLowerError {
            message: "void variable".into(),
            line: init.start_line.unwrap_or(0),
            column: init.start_column.unwrap_or(0),
        })?;
        let name = child_tokens(init)
            .iter()
            .find(|t| t.effective_type_name() == "NAME")
            .map(|t| t.value.clone())
            .unwrap_or_default();
        // v1's symbol table is **flat** — it has no per-block scopes — and the
        // lowering splices nested `{ }` blocks into the enclosing sequence.  So
        // a declaration that re-uses a live name cannot be modelled: the two
        // bindings collapse into one SIR block, which silently takes the wrong
        // type (a wrong-value miscompile) and makes the emitted C a
        // `redefinition of 'x'` error.  Refuse it here — one check covering
        // every path that can bind a name (plain blocks, `if`/`else` branches,
        // loop bodies, `for`-inits, and the lifted early-return continuation).
        if self.vars.contains_key(&name) {
            return err(
                format!(
                    "declaration of `{name}` re-uses a name that is already in scope; \
                     shadowing is not supported yet, because the symbol table has no \
                     per-block scopes (two sequential `for (int i = …)` loops hit this \
                     too)"
                ),
                init,
            );
        }
        let value = match first_node(init, "expr") {
            Some(e) => {
                let (ex, ety) = self.lower_expr(e)?;
                convert_to(ex, ety, ty) // assignment conversion to the declared type
            }
            None => convert(int_lit(0), ty), // uninitialised → 0 of the type
        };
        self.vars.insert(name.clone(), (ty, Scope::Local));
        Ok(Stmt::LetStarBinding {
            name,
            sir_type: Some(SirType::Int(ty)),
            value,
            span: sp(),
        })
    }

    /// Lower an expression, returning both the SIR node and its C type.
    fn lower_expr(&mut self, node: &GrammarASTNode) -> Result<(Expr, IntSpec), CLowerError> {
        // Bound the depth of the tree we emit — see `MAX_EXPR_DEPTH`.  The
        // counter is restored on every path so a rejected sub-expression does
        // not poison the rest of the function.
        self.expr_depth += 1;
        if self.budget_used() > MAX_EXPR_DEPTH {
            self.expr_depth -= 1;
            return err(
                format!("expression nests deeper than the limit of {MAX_EXPR_DEPTH}"),
                node,
            );
        }
        let result = self.lower_expr_inner(node);
        self.expr_depth -= 1;
        result
    }

    fn lower_expr_inner(&mut self, node: &GrammarASTNode) -> Result<(Expr, IntSpec), CLowerError> {
        let n = peel(node);
        match n.rule_name.as_str() {
            // Binary arithmetic / bitwise / shift — left-associative fold.
            "additive" | "multiplicative" | "shift" | "bit_and" | "bit_or" | "bit_xor" => {
                self.lower_binary(n)
            }
            // A comparison or logical operator used as a *value* has type `int`
            // in C (0 or 1), so it lowers to `If(bool, 1, 0)` — restoring the
            // integer from the SIR bool.
            "equality" | "relational" if child_nodes(n).len() >= 2 => {
                let b = self.lower_compare_bool(n)?;
                Ok((if_int(b), i32_spec()))
            }
            "logical_and" | "logical_or" if child_nodes(n).len() >= 2 => {
                let b = self.lower_cond(n)?;
                Ok((if_int(b), i32_spec()))
            }
            "cast" => self.lower_cast(n),
            "unary" => self.lower_unary(n),
            "postfix" => self.lower_postfix(n),
            "primary" => self.lower_primary(n),
            // Assignment-as-subexpression remains deferred (assignment is handled
            // in statement position).
            other => err(format!("expression `{other}` not yet supported"), n),
        }
    }

    fn lower_binary(&mut self, n: &GrammarASTNode) -> Result<(Expr, IntSpec), CLowerError> {
        // A chain is flat in the CST but folds left into a tree as deep as it is
        // wide, so its width is charged against the same budget as nesting — and
        // *held* while its operands are lowered.
        let width = self.charge_chain(n)?;
        self.expr_depth += width;
        let result = self.lower_binary_inner(n);
        self.expr_depth -= width;
        result
    }

    fn lower_binary_inner(&mut self, n: &GrammarASTNode) -> Result<(Expr, IntSpec), CLowerError> {
        // children: operand (op operand)+   — fold left.
        let mut acc: Option<(Expr, IntSpec)> = None;
        let mut pending_op: Option<String> = None;
        for c in &n.children {
            match c {
                ASTNodeOrToken::Node(operand) => {
                    let (e, t) = self.lower_expr(operand)?;
                    match acc.take() {
                        None => acc = Some((e, t)),
                        Some((le, lt)) => {
                            let op = pending_op.take().unwrap();
                            acc = Some(self.combine(&op, le, lt, e, t)?);
                        }
                    }
                }
                ASTNodeOrToken::Token(t) => pending_op = Some(t.value.clone()),
            }
        }
        acc.ok_or_else(|| CLowerError {
            message: "empty binary expression".into(),
            line: n.start_line.unwrap_or(0),
            column: n.start_column.unwrap_or(0),
        })
    }

    /// Apply the usual arithmetic conversions and emit `Convert{C}(op(a,b))`.
    fn combine(
        &self,
        op: &str,
        le: Expr,
        lt: IntSpec,
        re: Expr,
        rt: IntSpec,
    ) -> Result<(Expr, IntSpec), CLowerError> {
        let lp = promote(lt);
        let rp = promote(rt);
        let le = convert_to(le, lt, lp);
        let re = convert_to(re, rt, rp);

        // Shifts are the exception to the usual arithmetic conversions: C does
        // *not* bring the operands to a common type.  Each is promoted on its
        // own, the result has the type of the promoted **left** operand, and the
        // right operand is only a count.  (`>>` on a signed value is arithmetic,
        // on unsigned logical — the backends get that right because the operand
        // carries its signedness through `Convert`.)
        if op == "<<" || op == ">>" {
            // `>>` is arithmetic on a signed operand and **logical** on an
            // unsigned one.  The backends store everything in a signed int64, so
            // a `uint64_t` whose top bit is set is a *negative* int64 and a
            // native `>>` would sign-extend it.  Route unsigned `>>` to a
            // distinct `u>>` builtin the backends render as a logical shift.
            let name = if op == ">>" && !lp.signed { "u>>" } else { op };
            return Ok((convert(builtin(name, vec![le, re]), lp), lp));
        }

        let c = common_type(lp, rp);
        let le = convert_to(le, lp, c);
        let re = convert_to(re, rp, c);
        // `+ - *` and the bitwise operators `& | ^` all take the usual
        // arithmetic conversions and are performed at the common type.  Division
        // and remainder (`/ %`) still need the truncate-vs-floor split (C
        // truncates toward zero; SIR/Ruby floor), so they stay deferred.
        let sir_op = match op {
            "+" | "-" | "*" | "&" | "|" | "^" => op,
            other => {
                return Err(CLowerError {
                    message: format!("binary operator `{other}` not yet supported"),
                    line: 0,
                    column: 0,
                })
            }
        };
        // The operation is performed at width `c`; its result is a value of
        // type `c`, so wrap it to enforce C's fixed-width overflow.
        Ok((convert(builtin(sir_op, vec![le, re]), c), c))
    }

    fn lower_cast(&mut self, n: &GrammarASTNode) -> Result<(Expr, IntSpec), CLowerError> {
        // cast = LPAREN type_spec RPAREN cast
        let ts = first_node(n, "type_spec").unwrap();
        let to = resolve_type_spec(ts)?.ok_or_else(|| CLowerError {
            message: "cast to void".into(),
            line: n.start_line.unwrap_or(0),
            column: n.start_column.unwrap_or(0),
        })?;
        let inner = child_nodes(n)
            .into_iter()
            .find(|x| x.rule_name == "cast")
            .unwrap();
        let (e, from) = self.lower_expr(inner)?;
        Ok((convert_to(e, from, to), to))
    }

    fn lower_unary(&mut self, n: &GrammarASTNode) -> Result<(Expr, IntSpec), CLowerError> {
        // unary = (PLUS|MINUS|TILDE|BANG) unary
        let op = child_tokens(n).first().map(|t| t.value.clone()).unwrap();
        let operand = child_nodes(n).into_iter().next().unwrap();

        // `!c` used as a *value* has type `int` (0/1), so lower its operand as a
        // condition and wrap the resulting bool: `If(not(cond(c)), 1, 0)`.
        if op == "!" {
            let cond = builtin("not", vec![self.lower_cond(operand)?]);
            return Ok((if_int(cond), i32_spec()));
        }

        let (e, t) = self.lower_expr(operand)?;
        let tp = promote(t); // unary applies integer promotion
        let e = convert_to(e, t, tp);
        match op.as_str() {
            "+" => Ok((e, tp)),
            // Unary minus as `0 - x` (both backends render binary `-`); the
            // subtract happens at the promoted type and its result is wrapped.
            "-" => Ok((convert(builtin("-", vec![int_lit(0), e]), tp), tp)),
            // Bitwise NOT at the promoted type, wrapped to enforce its width.
            "~" => Ok((convert(builtin("~", vec![e]), tp), tp)),
            other => err(format!("unary `{other}` not yet supported"), n),
        }
    }

    fn lower_postfix(&mut self, n: &GrammarASTNode) -> Result<(Expr, IntSpec), CLowerError> {
        // postfix = primary { call_suffix }
        let nodes = child_nodes(n);
        let callee = nodes.first().copied().unwrap();
        let call = nodes.iter().find(|x| x.rule_name == "call_suffix").copied();
        let Some(call) = call else {
            return self.lower_primary(callee);
        };
        // The callee is a bare name (function).
        let name = child_tokens(peel(callee))
            .iter()
            .find(|t| t.effective_type_name() == "NAME")
            .map(|t| t.value.clone())
            .ok_or_else(|| CLowerError {
                message: "call of non-identifier".into(),
                line: n.start_line.unwrap_or(0),
                column: n.start_column.unwrap_or(0),
            })?;
        let arg_exprs: Vec<&GrammarASTNode> = first_node(call, "arg_list")
            .map(|al| {
                child_nodes(al)
                    .into_iter()
                    .filter(|x| x.rule_name == "expr")
                    .collect()
            })
            .unwrap_or_default();
        self.lower_call(&name, &arg_exprs, call)
    }

    fn lower_call(
        &mut self,
        name: &str,
        args: &[&GrammarASTNode],
        call: &GrammarASTNode,
    ) -> Result<(Expr, IntSpec), CLowerError> {
        // printf("<fmt>", e) → puts(e) when <fmt> ends in \n, else print(e).
        if name == "printf" {
            let fmt = call_string_literal(call);
            let newline = fmt.map(|s| s.ends_with("\\n")).unwrap_or(false);
            // The value argument (skip the format string) — the last expr.
            let val = args.last().ok_or_else(|| CLowerError {
                message: "printf without a value argument".into(),
                line: call.start_line.unwrap_or(0),
                column: call.start_column.unwrap_or(0),
            })?;
            let (e, _t) = self.lower_expr(val)?;
            let helper = if newline { "puts" } else { "print" };
            return Ok((builtin(helper, vec![e]), i32_spec()));
        }
        // Ordinary function call: convert each argument to its parameter type.
        let (ptypes, ret) = self.fns.get(name).cloned().ok_or_else(|| CLowerError {
            message: format!("call to unknown function `{name}`"),
            line: call.start_line.unwrap_or(0),
            column: call.start_column.unwrap_or(0),
        })?;
        let mut sir_args = Vec::new();
        for (i, a) in args.iter().enumerate() {
            let (e, t) = self.lower_expr(a)?;
            let e = match ptypes.get(i) {
                Some(pt) => convert_to(e, t, *pt),
                None => e,
            };
            sir_args.push(e);
        }
        let rt = ret.unwrap_or_else(i32_spec);
        Ok((
            Expr::DirectCall {
                fn_name: name.to_string(),
                args: sir_args,
                effects: EffectSet::PURE,
                span: sp(),
            },
            rt,
        ))
    }

    fn lower_primary(&mut self, n: &GrammarASTNode) -> Result<(Expr, IntSpec), CLowerError> {
        // primary = INT_LIT | CHAR_LIT | STR_LIT | NAME | LPAREN expr RPAREN
        if let Some(inner) = first_node(n, "expr") {
            return self.lower_expr(inner); // parenthesised
        }
        let tok = child_tokens(n)
            .into_iter()
            .next()
            .ok_or_else(|| CLowerError {
                message: "empty primary".into(),
                line: n.start_line.unwrap_or(0),
                column: n.start_column.unwrap_or(0),
            })?;
        match tok.effective_type_name() {
            "INT_LIT" => {
                let (v, ty) = parse_int_literal(&tok.value);
                Ok((int_lit(v), ty))
            }
            "CHAR_LIT" => {
                let v = parse_char_literal(&tok.value);
                Ok((int_lit(v), i32_spec())) // a char constant has type int in C
            }
            "NAME" => {
                let name = tok.value.clone();
                let (ty, scope) = self.vars.get(&name).copied().ok_or_else(|| CLowerError {
                    message: format!("use of undeclared variable `{name}`"),
                    line: n.start_line.unwrap_or(0),
                    column: n.start_column.unwrap_or(0),
                })?;
                Ok((
                    Expr::VarRef {
                        name,
                        scope,
                        span: sp(),
                    },
                    ty,
                ))
            }
            other => err(format!("primary token `{other}` unsupported"), n),
        }
    }
}

// ── leaf parsing ─────────────────────────────────────────────────────────────

fn peel_block_item(item: &GrammarASTNode) -> &GrammarASTNode {
    child_nodes(item).into_iter().next().unwrap_or(item)
}

/// The leading operator token of a `unary` node (`!`, `~`, `-`, `+`), if any.
fn unary_op(n: &GrammarASTNode) -> Option<String> {
    child_tokens(n).first().map(|t| t.value.clone())
}

// ── sequence / control-flow shape analysis (milestone 3) ────────────────────
//
// The early-return transformation reasons about *sequences* of statements that
// may come from a `compound_stmt` (a list of `block_item`s) or from a single
// unbraced branch (`if (c) return 1;`).  These helpers normalise both shapes so
// `lower_seq` can walk them uniformly.

/// Reduce a `block_item` (or a bare `statement`) to the node carrying its kind:
/// a `declaration`, or the peeled statement kind (`return_stmt`, `if_stmt`, …).
fn seq_elem(n: &GrammarASTNode) -> &GrammarASTNode {
    let inner = if n.rule_name == "block_item" {
        peel_block_item(n)
    } else {
        n
    };
    if inner.rule_name == "declaration" {
        inner
    } else {
        peel(inner)
    }
}

/// The `block_item` children of a `compound_stmt`.
fn block_items_of(compound: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    child_nodes(compound)
        .into_iter()
        .filter(|x| x.rule_name == "block_item")
        .collect()
}

/// The `statement` children of an `if_stmt`: `[then]` or `[then, else]`.
fn if_branches(s: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    child_nodes(s)
        .into_iter()
        .filter(|x| x.rule_name == "statement")
        .collect()
}

/// A branch as a statement sequence: a braced branch contributes its
/// `block_item`s, an unbraced one contributes itself as a single element.
fn branch_items(branch: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    let s = peel(branch);
    if s.rule_name == "compound_stmt" {
        block_items_of(s)
    } else {
        vec![branch]
    }
}

/// Does this sequence return on **every** path?  Used to decide whether a
/// branch needs the continuation appended.  Conservative: an `if` without an
/// `else` never qualifies, and loops are not analysed (a `while` may run zero
/// times, so it can never guarantee a return).
fn always_returns(items: &[&GrammarASTNode]) -> bool {
    items.iter().any(|it| {
        let e = seq_elem(it);
        match e.rule_name.as_str() {
            "return_stmt" => true,
            "compound_stmt" => always_returns(&block_items_of(e)),
            "if_stmt" => {
                let br = if_branches(e);
                match (br.first(), br.get(1)) {
                    (Some(t), Some(f)) => {
                        always_returns(&branch_items(t)) && always_returns(&branch_items(f))
                    }
                    _ => false, // no `else` — the fall-through path doesn't return
                }
            }
            _ => false,
        }
    })
}

/// Does either branch of this `if` contain a `return` in a *liftable* position?
/// Deliberately does not descend into loops: a `return` inside a `while`/`for`
/// cannot be turned into a value (it needs a break-with-value), so it is left
/// for `lower_stmt` to reject with a clear, positioned error.
fn if_returns(s: &GrammarASTNode) -> bool {
    if_branches(s)
        .iter()
        .any(|b| contains_return(&branch_items(b)))
}

fn contains_return(items: &[&GrammarASTNode]) -> bool {
    items.iter().any(|it| {
        let e = seq_elem(it);
        match e.rule_name.as_str() {
            "return_stmt" => true,
            "compound_stmt" => contains_return(&block_items_of(e)),
            "if_stmt" => if_returns(e),
            _ => false,
        }
    })
}

/// The string-literal value inside a call's arg_list (the printf format), with
/// surrounding quotes stripped.
fn call_string_literal(call: &GrammarASTNode) -> Option<String> {
    let al = first_node(call, "arg_list")?;
    for e in child_nodes(al) {
        let p = peel(e);
        if let Some(t) = child_tokens(p).into_iter().next() {
            if t.effective_type_name() == "STR_LIT" {
                let s = &t.value;
                return Some(s.trim_matches('"').to_string());
            }
        }
    }
    None
}

/// Parse a C integer literal (hex/decimal + u/l suffix) → (value, type).
fn parse_int_literal(raw: &str) -> (i64, IntSpec) {
    let lower = raw.to_ascii_lowercase();
    let unsigned = lower.contains('u');
    let long = lower.matches('l').count() >= 1;
    let digits: String = raw
        .chars()
        .take_while(|c| c.is_ascii_hexdigit() || *c == 'x' || *c == 'X')
        .collect();
    let value = if let Some(hex) = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
    {
        i64::from_str_radix(hex, 16).unwrap_or(0)
    } else {
        digits.parse::<i64>().unwrap_or(0)
    };
    let width = if long { IntWidth::W64 } else { IntWidth::W32 };
    (value, spec_of(width, !unsigned))
}

/// Parse a C char constant like `'A'` or `'\n'` → its code point.
fn parse_char_literal(raw: &str) -> i64 {
    let inner = raw.trim_matches('\'');
    let mut chars = inner.chars();
    match chars.next() {
        Some('\\') => match chars.next() {
            Some('n') => 10,
            Some('t') => 9,
            Some('r') => 13,
            Some('0') => 0,
            Some('\\') => 92,
            Some('\'') => 39,
            Some(c) => c as i64,
            None => 0,
        },
        Some(c) => c as i64,
        None => 0,
    }
}
