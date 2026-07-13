//! The lowering pass from `coding_adventures_apl_parser`'s generic
//! [`GrammarASTNode`] CST → [`semantic_ir::Module`], **v0.1.0**.
//!
//! # APL's CST shape (a quick refresher, confirmed against `apl-parser`'s
//! own grammar and the tree-walk `apl-runtime::eval` already does over it)
//!
//! ```text
//! program    = { line }
//! line       = [ statement ]           -- no `statement` child: blank/comment line, skip
//! statement  = assignment              -- pure passthrough, always 1 child
//! assignment = NAME ARROW assignment   -- 3 children: chained/actual assignment
//!            | value_expr              -- 1 child: base case
//! value_expr = term                                    -- 1 child: bare term
//!            | function_expr value_expr                -- 2 children: monadic
//!            | term function_expr value_expr           -- 3 children: dyadic (right-recursive)
//! term       = NUMBER { NUMBER }        -- 1+ stranded numbers (1 = scalar, 2+ = vector)
//!            | NAME
//!            | LPAREN value_expr RPAREN
//! function_expr  = function_atom [ REDUCE | SCAN ]   -- operator trails the atom
//!                | OUTER function_atom               -- operator precedes the atom
//! function_atom  = one of the 15 primitive glyph tokens
//! ```
//!
//! # Scope
//!
//! **Supported** (every construct `apl-parser`'s grammar can produce):
//! - Number literals (`NUMBER`, high-minus `¯` negative sign), stranded
//!   literals (`1 2 3` → one rank-1 [`Expr::ArrayLit`]), variables (`NAME`),
//!   parenthesised grouping.
//! - Assignment (`←`), including right-associative chained assignment
//!   (`A←B←3`) — see "Chained assignment" below.
//! - All 12 scalar dyadic atoms (`+ - × ÷ ⌈ ⌊ = ≠ < ≤ ≥ >`), unconditionally
//!   lowered to [`Expr::ElementwiseOp`] — see "No scalar/array
//!   disambiguation" below.
//! - The 6 scalar atoms that have a monadic meaning (`+ - × ÷ ⌈ ⌊`), each
//!   mapped onto a well-known [`Expr::BuiltinCall`] name (or, for `+`, no
//!   wrapping at all — see point 2 in this file's PR description / the
//!   `apply_monadic_scalar` doc comment).
//! - `⍴`/`⍳`/`,` (shape-reshape, index-generator-index-of,
//!   ravel-catenate), monadic and dyadic.
//! - `/` (reduce) and `\` (scan), monadic-only, over any of the 12 scalar
//!   atoms.
//! - `∘.` (outer product), dyadic-only, over any of the 12 scalar atoms.
//! - Auto-print of a bare top-level value expression (mapped onto the same
//!   `"print"` [`Expr::BuiltinCall`] every SIR backend already implements).
//!
//! **Deliberately rejected** with a clean [`AplLowerError`] (each is
//! syntactically constructible by `apl-parser`'s grammar but semantically
//! invalid — the grammar alone cannot rule these out, exactly as
//! `apl-runtime::eval`'s own evaluator discovers at *runtime*, and this
//! frontend discovers at *lowering time* instead):
//! - The 6 comparison atoms (`= ≠ < ≤ ≥ >`) used monadically — they have no
//!   monadic meaning in APL (see `apply_monadic_scalar`).
//! - A reduce- or scan-decorated `function_expr` used dyadically (`3+/4`) —
//!   both operators are inherently monadic (`+/A` takes exactly one
//!   operand).
//! - An outer-product-decorated `function_expr` used monadically (`∘.×1`) —
//!   outer product is inherently dyadic (`A∘.×B` needs both operands).
//! - `⍴`/`⍳`/`,` decorated with `/`/`\`/`∘.` (e.g. `⍴/A`) — these three
//!   primitives are not "scalar dyadic functions" the way the other 12
//!   atoms are, so stacking an operator on one of them is rejected, mirroring
//!   `apl-runtime::eval::require_scalar_binop`'s identical restriction.
//!
//! **Not applicable** (the grammar `apl-parser` compiles literally cannot
//! produce these — boxing/nested arrays, the rank conjunction,
//! user-defined functions/dfns, control flow — so there is no rejection
//! code to write for them; they simply have no CST shape to reach this
//! lowerer at all).
//!
//! # No scalar/array disambiguation (unlike `matlab-to-semantic-ir`)
//!
//! MATLAB's `matlab-to-semantic-ir` frontend has to guess, per operator use,
//! whether `*`/`/`/`\` mean scalar arithmetic or a matrix operation (`*` is
//! matrix multiply between two non-scalars, elementwise between two provable
//! scalars) — that guess is the entire reason `expr_is_known_scalar` exists
//! in that crate. **None of APL's 12 scalar dyadic atoms have a second,
//! non-elementwise reading** — `+ - × ÷ ⌈ ⌊ = ≠ < ≤ ≥ >` are always
//! elementwise/broadcast, full stop, whether the operands are two scalars or
//! two arrays (this mirrors `array_runtime::BinOp`'s own single meaning per
//! variant, and `apl-runtime::eval::AplFn::Atom`'s own doc comment making the
//! same point). So `3+4` lowers to the *exact same* node shape as `A+B`:
//! `Expr::ElementwiseOp { op: Add, lhs: IntLit(3), rhs: IntLit(4), .. }` —
//! there is deliberately no "fold two known-scalar literals into a bare
//! `BuiltinCall`" optimisation here the way MATLAB's frontend has. This is a
//! genuine simplification, not a missed optimisation: writing one would only
//! reintroduce, for zero semantic benefit, the exact kind of syntactic
//! scalar-provability heuristic APL's own semantics make unnecessary.
//!
//! # Chained assignment (`A←B←3`)
//!
//! `assignment`'s right-recursive `NAME ARROW assignment` production means
//! `A←B←3` parses as `A ← (B ← 3)`, one CST node nested inside another. This
//! lowerer's [`Lowerer::lower_assignment_chain`] mirrors that recursive
//! shape: it lowers the *inner* `assignment` first (producing the
//! statements it emits, plus an `Expr` for "the value that was just bound"),
//! then appends **one more** statement binding the outer `NAME` to that same
//! value. Two design points worth spelling out:
//!
//! - The value threaded back up is a [`Expr::VarRef`] to the name **just
//!   bound**, not a re-lowered copy of the RHS expression tree. For
//!   `A←B←3`, `A` receives a `VarRef("B")`, not a duplicated `IntLit(3)`.
//!   This avoids re-lowering (and potentially duplicating, in the emitted
//!   IR, a large) sub-expression tree, and is observationally identical
//!   here since this cut has no side-effecting expressions between the two
//!   assignments (`apl-runtime::eval::eval_assignment` — the reference
//!   evaluator — does something equivalent at the *value* level: it inserts
//!   into `self.vars` and returns the same `Array` clone up the recursion).
//! - Statements come out in the CORRECT dependency order despite the
//!   recursion going "outer calls inner first": `lower_assignment_chain`
//!   only appends the outer binding's own statement **after** the
//!   recursive call to the inner one returns, so for `A←B←3` the emitted
//!   sequence is `[LetStarBinding(B, 3), LetStarBinding(A, VarRef(B))]` —
//!   `B` is always bound before anything references it.
//!
//! # Auto-print, not MATLAB-style suppression
//!
//! A bare top-level `value_expr` (an `assignment` node that never reaches
//! the `NAME ARROW …` production) is wrapped in
//! `Expr::BuiltinCall { name: "print", .. }` — reusing the *exact* builtin
//! name `matlab-to-semantic-ir` already maps its own `disp(x)` call onto, so
//! every backend that already implements `"print"` needs no APL-specific
//! change. This is a deliberately different design decision from MATLAB's
//! own frontend, which does not model `;`-suppression as a language
//! semantic at all (MATLAB scripts have no auto-print concept to preserve).
//! APL's auto-print, by contrast, *is* a real language semantic (MA05 §4:
//! "assignment is silent, a bare value_expr result auto-prints" — the same
//! rule `apl-runtime::eval::run`'s own doc comment states), so this frontend
//! models it explicitly rather than leaving it as a REPL nicety the IR
//! doesn't know about.

use std::collections::HashSet;

use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{
    Block, EffectSet, ElementwiseOpKind, Expr, Feature, FeatureManifest, Function, Metadata,
    Module, Scope, Span, Stmt,
};

/// Maximum expression-nesting depth. Mirrors every other SIR frontend's
/// identically-named, identically-justified guard ("defense in depth,
/// exactly like every other runtime crate in this repo" —
/// `apl-runtime::eval::MAX_DEPTH`'s own doc comment states the same
/// rationale): `apl-parser`'s own `MAX_RULE_DEPTH` (100) already bounds how
/// deep a CST built from untrusted source can possibly be, so this bound can
/// never actually trip on a tree that came from `try_parse_apl` — it exists
/// purely so a *hand-built* `GrammarASTNode` (or a future change to
/// `apl-parser`'s own cap) can't turn a deep-but-technically-parseable input
/// into an uncatchable native stack overflow while walking it here. This
/// cut has no blocks or control-flow constructs at all (MA05 §4), so unlike
/// `matlab-to-semantic-ir` there is no separate `MAX_BLOCK_DEPTH` to define.
const MAX_EXPR_DEPTH: usize = 256;

/// Synthetic file name used for all spans (the CST does not carry the
/// original path).
const FILE: &str = "<apl>";

// ---------------------------------------------------------------------------
// Public error type
// ---------------------------------------------------------------------------

/// An error encountered during APL → SIR lowering.
///
/// Mirrors `MatlabLowerError`/`WolframLowerError`'s shape exactly (a
/// `message` plus 1-based `line`/`column`) so tooling can treat every SIR
/// frontend uniformly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AplLowerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for AplLowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AplLowerError at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for AplLowerError {}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Lower a parsed APL CST (rooted at the `program` rule) into a SIR module.
pub fn compile(tree: &GrammarASTNode, module_name: &str) -> Result<Module, AplLowerError> {
    Lowerer::new(module_name).lower_file(tree)
}

// ---------------------------------------------------------------------------
// Function-expression representation
// ---------------------------------------------------------------------------

/// A glyph naming one of `apl.tokens`' three "bespoke" primitives, kept
/// around so error messages can name the actual glyph (`"⍴"`, not `"RHO"`) —
/// mirrors `apl-runtime::eval::NonScalarAtom` exactly.
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

/// This lowerer's own representation of a `function_expr`: "which function,
/// and with which operator (if any) applied" — mirrors
/// `apl-runtime::eval::AplFn` exactly (that crate's evaluator and this
/// crate's lowerer both dispatch on the identical CST shape, so it is not a
/// coincidence the two enums line up one-for-one).
enum FnKind {
    /// One of the 12 atoms that map onto [`ElementwiseOpKind`] (`+ - × ÷ ⌈ ⌊
    /// = ≠ < ≤ ≥ >`). There is exactly one glyph per `ElementwiseOpKind`
    /// variant this frontend ever constructs, so the kind alone is enough to
    /// recover which glyph this was for monadic dispatch.
    Atom(ElementwiseOpKind),
    /// `⍴`/`⍳`/`,` — bespoke monadic+dyadic logic that does not fit "an
    /// operator over a scalar dyadic function" at all, so it never plugs
    /// into reduce/scan/outer-product.
    NonScalar(NonScalarAtom),
    /// An atom decorated with `/` (reduce) — inherently monadic.
    Reduce(ElementwiseOpKind),
    /// An atom decorated with `\` (scan) — inherently monadic.
    Scan(ElementwiseOpKind),
    /// An atom decorated with `∘.` (outer product) — inherently dyadic.
    Outer(ElementwiseOpKind),
}

// ---------------------------------------------------------------------------
// The lowerer
// ---------------------------------------------------------------------------

/// APL scopes every variable to the *whole program* (there are no blocks,
/// loops, or functions in this cut — MA05 §4) — so, unlike
/// `matlab-to-semantic-ir`'s per-function `FunctionCtx`, this lowerer needs
/// only ONE flat set of bound names for its entire lifetime, never rewound.
struct Lowerer {
    module_name: String,
    /// Features observed while lowering, used to build the manifest so it
    /// declares *exactly* what the module emits.
    observed: FeatureManifest,
    /// Every name assigned so far, in program order. Used to decide
    /// first-occurrence (`LetStarBinding`) vs. re-assignment (`Assign`) for
    /// an assignment target, and to reject a `NAME` term reference to a
    /// variable that has never been assigned (mirroring
    /// `apl-runtime::eval::eval_term`'s own "undefined variable" runtime
    /// error, but caught at lowering time instead).
    locals: HashSet<String>,
}

impl Lowerer {
    fn new(module_name: &str) -> Self {
        Self {
            module_name: module_name.to_string(),
            observed: FeatureManifest::new(),
            locals: HashSet::new(),
        }
    }

    // -------------------------------------------------------------------
    // top level: `program` → one `main` function
    // -------------------------------------------------------------------

    fn lower_file(&mut self, program: &GrammarASTNode) -> Result<Module, AplLowerError> {
        if program.rule_name != "program" {
            return Err(self.err_at(
                program,
                format!("expected `program` root, got `{}`", program.rule_name),
            ));
        }

        // Every APL value carries no static type declaration -- this is
        // itself what the validator's own ground truth treats as "using"
        // dynamic typing (see `semantic-ir/src/validator.rs`'s comment to
        // that effect, and `matlab-to-semantic-ir`'s identical observation),
        // so it is unconditionally true for every module this crate emits.
        self.observed.add(Feature::DynamicTyping);

        let mut stmts: Vec<Stmt> = Vec::new();
        for line in child_nodes(program) {
            if line.rule_name != "line" {
                continue;
            }
            // A `line` with no `statement` child (a blank line, or a
            // comment-only line -- `⍝` comments are already stripped by the
            // lexer's skip pattern) is a bare NEWLINE production; skip it,
            // don't error.
            let Some(stmt_node) = first_child_named(line, "statement") else {
                continue;
            };
            // `statement = assignment` is a pure passthrough rule (always
            // exactly one child).
            let assignment_node = only_node(stmt_node)
                .ok_or_else(|| self.err_at(stmt_node, "malformed statement".to_string()))?;
            let mut new_stmts = self.lower_top_level_statement(assignment_node, 0)?;
            stmts.append(&mut new_stmts);
        }

        let span = Span::point(FILE, 1, 1);
        let main = Function {
            name: "main".to_string(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts,
                value: Expr::NilLit { span: span.clone() },
                span: span.clone(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: span.clone(),
        };

        let metadata = Metadata::new()
            .with_source_language("apl")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION);

        Ok(Module {
            name: self.module_name.clone(),
            manifest: self.observed.clone(),
            imports: vec![],
            exports: vec![],
            functions: vec![main],
            globals: vec![],
            metadata,
            span,
        })
    }

    /// Dispatch one top-level `assignment` node (a `statement`'s sole
    /// child) into zero or more [`Stmt`]s.
    fn lower_top_level_statement(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Vec<Stmt>, AplLowerError> {
        self.check_depth(node, depth)?;
        match node.children.len() {
            // Base case: a bare `value_expr`, not an assignment. Real APL
            // auto-print session semantics (MA05 §4) -- see this file's
            // module doc comment's "Auto-print" section.
            1 => {
                let value_expr_node = only_node(node)
                    .ok_or_else(|| self.err_at(node, "malformed value_expr statement".to_string()))?;
                let v = self.lower_value_expr(value_expr_node, depth + 1)?;
                let span = v.span().clone();
                Ok(vec![Stmt::ExprStmt {
                    expr: Expr::BuiltinCall {
                        name: "print".to_string(),
                        args: vec![v],
                        effects: EffectSet::PURE,
                        span: span.clone(),
                    },
                    span,
                }])
            }
            // `NAME ARROW assignment` -- an actual assignment (possibly
            // chained). Assignment is silent (MA05 §4): emit every
            // statement the chain unrolled into, and nothing else.
            3 => {
                let (stmts, _final_value) = self.lower_assignment_chain(node, depth)?;
                Ok(stmts)
            }
            n => Err(self.err_at(node, format!("malformed assignment with {n} children"))),
        }
    }

    // -------------------------------------------------------------------
    // assignment (including chained assignment)
    // -------------------------------------------------------------------

    /// Recursively lower an `assignment` node. Returns the statements the
    /// chain unrolled into (in dependency order) and an [`Expr`] the OUTER
    /// caller can reuse to reference "the value just bound" -- see this
    /// file's module doc comment's "Chained assignment" section for the full
    /// design rationale.
    fn lower_assignment_chain(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Vec<Stmt>, Expr), AplLowerError> {
        self.check_depth(node, depth)?;
        match node.children.len() {
            // Base case: `value_expr`, nothing to bind.
            1 => {
                let value_expr_node = only_node(node).ok_or_else(|| {
                    self.err_at(node, "malformed value_expr in assignment".to_string())
                })?;
                let v = self.lower_value_expr(value_expr_node, depth + 1)?;
                Ok((vec![], v))
            }
            // Recursive case: `NAME ARROW assignment`.
            3 => {
                let name = self.assignment_target_name(node)?;
                let inner = only_node(node).ok_or_else(|| {
                    self.err_at(node, "malformed assignment: no nested assignment".to_string())
                })?;
                let (mut stmts, inner_value) = self.lower_assignment_chain(inner, depth + 1)?;
                let span = self.span_of(node);
                // First occurrence of `name` in this (whole-program) scope
                // -> `LetStarBinding`; already-seen -> `Assign` (and observe
                // `Feature::MutableBindings`), mirroring every other SIR
                // frontend's first-occurrence-vs-reassignment convention.
                if self.locals.insert(name.clone()) {
                    stmts.push(Stmt::LetStarBinding {
                        name: name.clone(),
                        sir_type: None,
                        value: inner_value,
                        span: span.clone(),
                    });
                } else {
                    self.observed.add(Feature::MutableBindings);
                    stmts.push(Stmt::Assign {
                        name: name.clone(),
                        scope: Scope::Local,
                        value: inner_value,
                        span: span.clone(),
                    });
                }
                Ok((stmts, Expr::VarRef { name, scope: Scope::Local, span }))
            }
            n => Err(self.err_at(node, format!("malformed assignment with {n} children"))),
        }
    }

    /// The `NAME` token of an actual assignment's target -- the first child
    /// of a 3-child `assignment` node (`[Token(NAME), Token(ARROW),
    /// Node(assignment)]`), mirroring
    /// `apl-runtime::eval::assignment_target_name` exactly.
    fn assignment_target_name(&self, node: &GrammarASTNode) -> Result<String, AplLowerError> {
        match node.children.first() {
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "NAME" => {
                Ok(t.value.clone())
            }
            _ => Err(self.err_at(node, "malformed assignment (missing target name)".to_string())),
        }
    }

    // -------------------------------------------------------------------
    // value_expr / term
    // -------------------------------------------------------------------

    /// `value_expr = term | function_expr value_expr | term function_expr
    /// value_expr` -- mirrors `apl-runtime::eval::eval_value_expr`'s own
    /// dispatch on child-count exactly (arity alone disambiguates the three
    /// productions; the parser has already resolved which alternative
    /// matched, so no further rule-name inspection of the children is
    /// needed).
    fn lower_value_expr(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, AplLowerError> {
        self.check_depth(node, depth)?;
        let span = self.span_of(node);
        let kids = child_nodes(node);
        match kids.as_slice() {
            [term] => self.lower_term(term, depth + 1),
            [fexpr, sub] => {
                let f = self.lower_function_expr(fexpr)?;
                let arg = self.lower_value_expr(sub, depth + 1)?;
                self.apply_monadic(f, arg, span)
            }
            [lhs_term, fexpr, sub] => {
                let lhs = self.lower_term(lhs_term, depth + 1)?;
                let f = self.lower_function_expr(fexpr)?;
                let rhs = self.lower_value_expr(sub, depth + 1)?;
                self.apply_dyadic(f, lhs, rhs, span)
            }
            other => Err(self.err_at(
                node,
                format!("malformed value_expr with {} children", other.len()),
            )),
        }
    }

    /// `term = NUMBER { NUMBER } | NAME | LPAREN value_expr RPAREN`.
    fn lower_term(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, AplLowerError> {
        self.check_depth(node, depth)?;
        match node.children.first() {
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "NUMBER" => {
                // "Stranding": one or more juxtaposed NUMBER tokens form a
                // single term -- `1 2 3` is one 3-element vector, a lone `5`
                // is a rank-0 scalar (MA05 §4).
                let numbers: Vec<&Token> = node
                    .children
                    .iter()
                    .filter_map(|c| match c {
                        ASTNodeOrToken::Token(tok) if tok.effective_type_name() == "NUMBER" => {
                            Some(tok)
                        }
                        _ => None,
                    })
                    .collect();
                if numbers.len() == 1 {
                    self.number_literal(numbers[0])
                } else {
                    // Per the SIR22 spec's own doc comment on `ArrayLit`,
                    // `rows.len() == 1` is precisely how a row/rank-1 vector
                    // is represented -- exactly APL's stranded-literal shape.
                    self.observed.add(Feature::NDArrays);
                    self.observed.add(Feature::ArrayColumnMajor);
                    let span = self.span_of(node);
                    let row: Vec<Expr> = numbers
                        .iter()
                        .map(|tok| self.number_literal(tok))
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(Expr::ArrayLit { rows: vec![row], span })
                }
            }
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "NAME" => {
                let span = self.span_of(node);
                if !self.locals.contains(&t.value) {
                    return Err(self.err_at(
                        node,
                        format!("undefined variable `{}` (not previously assigned)", t.value),
                    ));
                }
                Ok(Expr::VarRef {
                    name: t.value.clone(),
                    scope: Scope::Local,
                    span,
                })
            }
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "LPAREN" => {
                let inner = only_node(node)
                    .ok_or_else(|| self.err_at(node, "malformed parenthesised term".to_string()))?;
                self.lower_value_expr(inner, depth + 1)
            }
            _ => Err(self.err_at(node, "malformed term".to_string())),
        }
    }

    /// Convert one `NUMBER` token into an `Expr::IntLit`/`Expr::FloatLit`,
    /// observing `Feature::Floats` when it is the latter (the validator's
    /// own ground truth: `Expr::FloatLit` requires `Feature::Floats`
    /// declared -- see `semantic-ir/src/validator.rs`'s `FloatLit` arm).
    ///
    /// Returns a clean [`AplLowerError`] rather than silently substituting
    /// `0.0` if the token's lexeme somehow fails to parse as a number. Under
    /// the normal `compile_source` path this can never actually trigger --
    /// `apl-parser`'s own `NUMBER` lexer rule guarantees a parseable lexeme
    /// -- but `compile` is also a public entry point over a hand-built
    /// `GrammarASTNode`, and every other malformed-input case in this file
    /// is rejected explicitly rather than silently coerced to a default
    /// value, so this one should be no different (caught by security
    /// review).
    fn number_literal(&mut self, tok: &Token) -> Result<Expr, AplLowerError> {
        let span = Span::point(FILE, tok.line, tok.column);
        let expr = number_literal_expr(&tok.value, &span).map_err(|message| AplLowerError {
            message,
            line: tok.line,
            column: tok.column,
        })?;
        if matches!(expr, Expr::FloatLit { .. }) {
            self.observed.add(Feature::Floats);
        }
        Ok(expr)
    }

    // -------------------------------------------------------------------
    // function_expr / function_atom
    // -------------------------------------------------------------------

    /// `function_expr = function_atom [ REDUCE | SCAN ] | OUTER
    /// function_atom` -- mirrors `apl-runtime::eval::parse_function_expr`
    /// exactly, including the shape ambiguity note: both the
    /// reduce/scan-decorated and outer-decorated alternatives have exactly
    /// two children, so they are told apart by *which position* holds the
    /// bare token, not by length alone.
    fn lower_function_expr(&self, node: &GrammarASTNode) -> Result<FnKind, AplLowerError> {
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(atom)] => self.lower_function_atom(atom),
            [ASTNodeOrToken::Node(atom), ASTNodeOrToken::Token(op)] => match op.effective_type_name()
            {
                "REDUCE" => Ok(FnKind::Reduce(self.require_scalar_atom(atom, "reduce (/)")?)),
                "SCAN" => Ok(FnKind::Scan(self.require_scalar_atom(atom, "scan (\\)")?)),
                other => Err(self.err_at(node, format!("unexpected operator token `{other}`"))),
            },
            [ASTNodeOrToken::Token(_outer), ASTNodeOrToken::Node(atom)] => {
                Ok(FnKind::Outer(self.require_scalar_atom(atom, "outer product (∘.)")?))
            }
            _ => Err(self.err_at(node, "malformed function_expr".to_string())),
        }
    }

    /// `function_atom`: always exactly one child, a single token naming the
    /// primitive glyph -- mirrors `apl-runtime::eval::parse_function_atom`'s
    /// token-name-to-variant table exactly (confirmed against that
    /// function and `array_runtime::BinOp`'s own identical mapping).
    fn lower_function_atom(&self, node: &GrammarASTNode) -> Result<FnKind, AplLowerError> {
        let tok = match node.children.first() {
            Some(ASTNodeOrToken::Token(t)) => t,
            _ => return Err(self.err_at(node, "malformed function_atom".to_string())),
        };
        Ok(match tok.effective_type_name() {
            "PLUS" => FnKind::Atom(ElementwiseOpKind::Add),
            "MINUS" => FnKind::Atom(ElementwiseOpKind::Sub),
            "TIMES" => FnKind::Atom(ElementwiseOpKind::Mul),
            "DIVIDE" => FnKind::Atom(ElementwiseOpKind::Div),
            "CEILING" => FnKind::Atom(ElementwiseOpKind::Max),
            "FLOOR" => FnKind::Atom(ElementwiseOpKind::Min),
            "EQ" => FnKind::Atom(ElementwiseOpKind::Eq),
            "NE" => FnKind::Atom(ElementwiseOpKind::Ne),
            "LT" => FnKind::Atom(ElementwiseOpKind::Lt),
            "LE" => FnKind::Atom(ElementwiseOpKind::Le),
            "GE" => FnKind::Atom(ElementwiseOpKind::Ge),
            "GT" => FnKind::Atom(ElementwiseOpKind::Gt),
            "RHO" => FnKind::NonScalar(NonScalarAtom::Rho),
            "IOTA" => FnKind::NonScalar(NonScalarAtom::Iota),
            "RAVEL" => FnKind::NonScalar(NonScalarAtom::Ravel),
            other => return Err(self.err_at(node, format!("unknown function atom `{other}`"))),
        })
    }

    /// Reduce/scan/outer-product apply only to the 12 atoms that map onto an
    /// [`ElementwiseOpKind`] -- `⍴`/`⍳`/`,` are not "a scalar dyadic
    /// function" at all, so stacking an operator on one of them is a clean,
    /// explicit error, mirroring `apl-runtime::eval::require_scalar_binop`'s
    /// identical restriction (and its error-message style).
    fn require_scalar_atom(
        &self,
        atom: &GrammarASTNode,
        context: &str,
    ) -> Result<ElementwiseOpKind, AplLowerError> {
        match self.lower_function_atom(atom)? {
            FnKind::Atom(op) => Ok(op),
            FnKind::NonScalar(a) => Err(self.err_at(
                atom,
                format!(
                    "{} is not a scalar dyadic function and cannot take the {context} operator",
                    a.glyph()
                ),
            )),
            FnKind::Reduce(_) | FnKind::Scan(_) | FnKind::Outer(_) => {
                unreachable!("lower_function_atom never itself produces an operator-bearing FnKind")
            }
        }
    }

    // -------------------------------------------------------------------
    // monadic / dyadic application
    // -------------------------------------------------------------------

    /// Apply a monadic (one-argument) function-expression to `arg`.
    fn apply_monadic(&mut self, f: FnKind, arg: Expr, span: Span) -> Result<Expr, AplLowerError> {
        match f {
            FnKind::Atom(op) => self.apply_monadic_scalar(op, arg),
            FnKind::NonScalar(NonScalarAtom::Rho) => {
                self.observed.add(Feature::MatrixOps);
                self.observed.add(Feature::ArrayColumnMajor);
                Ok(Expr::Shape { target: Box::new(arg), span })
            }
            FnKind::NonScalar(NonScalarAtom::Iota) => {
                self.observed.add(Feature::NDArrays);
                Ok(Expr::IndexGenerator { count: Box::new(arg), span })
            }
            FnKind::NonScalar(NonScalarAtom::Ravel) => {
                self.observed.add(Feature::MatrixOps);
                self.observed.add(Feature::ArrayColumnMajor);
                Ok(Expr::Ravel { target: Box::new(arg), span })
            }
            FnKind::Reduce(op) => {
                self.observed.add(Feature::MatrixOps);
                self.observed.add(Feature::ArrayColumnMajor);
                Ok(Expr::Reduce { op, target: Box::new(arg), span })
            }
            FnKind::Scan(op) => {
                self.observed.add(Feature::MatrixOps);
                self.observed.add(Feature::ArrayColumnMajor);
                Ok(Expr::Scan { op, target: Box::new(arg), span })
            }
            FnKind::Outer(_) => Err(self.err_at_span(
                &span,
                "∘. (outer product) needs two operands, but was applied monadically".to_string(),
            )),
        }
    }

    /// Apply a dyadic (two-argument) function-expression to `lhs`/`rhs`.
    fn apply_dyadic(
        &mut self,
        f: FnKind,
        lhs: Expr,
        rhs: Expr,
        span: Span,
    ) -> Result<Expr, AplLowerError> {
        match f {
            FnKind::Atom(op) => {
                // Point 1 of this file's module doc comment: unconditional,
                // no scalar-vs-array disambiguation -- every one of the 12
                // scalar dyadic atoms is always elementwise in APL.
                self.observed.add(Feature::MatrixOps);
                self.observed.add(Feature::ArrayColumnMajor);
                Ok(Expr::ElementwiseOp {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span,
                })
            }
            FnKind::NonScalar(NonScalarAtom::Rho) => {
                // `apl-runtime::builtins::reshape(a, b)`: `a` is the shape
                // vector (APL's LHS), `b` is the data (APL's RHS) --
                // `Expr::Reshape`'s `shape`/`target` fields map to that same
                // A/B order.
                self.observed.add(Feature::MatrixOps);
                self.observed.add(Feature::ArrayColumnMajor);
                Ok(Expr::Reshape {
                    shape: Box::new(lhs),
                    target: Box::new(rhs),
                    span,
                })
            }
            FnKind::NonScalar(NonScalarAtom::Iota) => {
                // `apl-runtime::builtins::index_of(a, b)`: `a` is the
                // haystack, `b` is the needle.
                self.observed.add(Feature::NDArrays);
                Ok(Expr::IndexOf {
                    haystack: Box::new(lhs),
                    needle: Box::new(rhs),
                    span,
                })
            }
            FnKind::NonScalar(NonScalarAtom::Ravel) => {
                self.observed.add(Feature::MatrixOps);
                self.observed.add(Feature::ArrayColumnMajor);
                Ok(Expr::Catenate {
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span,
                })
            }
            FnKind::Outer(op) => {
                self.observed.add(Feature::MatrixOps);
                self.observed.add(Feature::ArrayColumnMajor);
                Ok(Expr::OuterProduct {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span,
                })
            }
            FnKind::Reduce(_) => Err(self.err_at_span(
                &span,
                "/ (reduce) takes exactly one operand, but was applied dyadically".to_string(),
            )),
            FnKind::Scan(_) => Err(self.err_at_span(
                &span,
                "\\ (scan) takes exactly one operand, but was applied dyadically".to_string(),
            )),
        }
    }

    /// Monadic meaning of the six atoms that have one (MA05 §4): `+`
    /// conjugate, `-` negate, `×` sign, `÷` reciprocal, `⌈` ceiling, `⌊`
    /// floor. The six comparisons have **no** monadic meaning -- a clean,
    /// explicit error rather than silently picking a behavior, mirroring
    /// `apl-runtime::eval::apply_monadic_scalar`'s identical restriction.
    ///
    /// `+` (conjugate) is the one case that emits the operand **unchanged**:
    /// this cut has no complex numbers, so conjugate is a genuine identity,
    /// and wrapping it in a synthetic no-op node would just be noise. Every
    /// other case wraps the operand in a well-known [`Expr::BuiltinCall`]:
    /// `"neg"` reuses the *exact* name `matlab-to-semantic-ir`'s own unary
    /// negate already established (confirmed by grepping that crate for
    /// `"neg"`) -- establishing the precedent that a `BuiltinCall` name is a
    /// generic operation any backend implements polymorphically over
    /// whatever value flows through it (SIR10's "types carry, don't
    /// verify"): it is fine and expected that `"neg"` here may receive an
    /// array-typed value even though MATLAB only ever proves it a scalar.
    /// `"sign"`/`"recip"`/`"ceil"`/`"floor"` are new well-known names this
    /// frontend introduces.
    fn apply_monadic_scalar(&mut self, op: ElementwiseOpKind, operand: Expr) -> Result<Expr, AplLowerError> {
        match op {
            ElementwiseOpKind::Add => Ok(operand),
            ElementwiseOpKind::Sub => Ok(wrap_builtin("neg", operand)),
            ElementwiseOpKind::Mul => Ok(wrap_builtin("sign", operand)),
            ElementwiseOpKind::Div => Ok(wrap_builtin("recip", operand)),
            ElementwiseOpKind::Max => Ok(wrap_builtin("ceil", operand)),
            ElementwiseOpKind::Min => Ok(wrap_builtin("floor", operand)),
            ElementwiseOpKind::Eq
            | ElementwiseOpKind::Ne
            | ElementwiseOpKind::Lt
            | ElementwiseOpKind::Le
            | ElementwiseOpKind::Ge
            | ElementwiseOpKind::Gt => {
                let span = operand.span().clone();
                Err(self.err_at_span(
                    &span,
                    format!(
                        "no monadic form for {} (comparison atoms are dyadic-only in APL)",
                        glyph_for_comparison(op)
                    ),
                ))
            }
            ElementwiseOpKind::Pow => unreachable!(
                "APL frontend never constructs ElementwiseOpKind::Pow -- there is no `^` atom \
                 among this cut's 12 scalar dyadic atoms (MA05 §4)"
            ),
        }
    }

    // -------------------------------------------------------------------
    // small helpers
    // -------------------------------------------------------------------

    fn span_of(&self, node: &GrammarASTNode) -> Span {
        Span::point(FILE, node.start_line.unwrap_or(1), node.start_column.unwrap_or(1))
    }

    fn err_at(&self, node: &GrammarASTNode, message: String) -> AplLowerError {
        AplLowerError {
            message,
            line: node.start_line.unwrap_or(1),
            column: node.start_column.unwrap_or(1),
        }
    }

    fn err_at_span(&self, span: &Span, message: String) -> AplLowerError {
        AplLowerError {
            message,
            line: span.start_line,
            column: span.start_col,
        }
    }

    fn check_depth(&self, node: &GrammarASTNode, depth: usize) -> Result<(), AplLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            ));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Free helpers
// ---------------------------------------------------------------------------

/// Collect the *node* children of `node` (dropping tokens).
fn child_nodes(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

/// The first *node* child named `kind`, or `None`.
fn first_child_named<'a>(node: &'a GrammarASTNode, kind: &str) -> Option<&'a GrammarASTNode> {
    child_nodes(node).into_iter().find(|n| n.rule_name == kind)
}

/// The first (and, for every rule this crate lowers, only) *node* child --
/// mirrors `apl-runtime::eval::only_node`'s exact "first Node child found"
/// behavior.
fn only_node(node: &GrammarASTNode) -> Option<&GrammarASTNode> {
    node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(n) => Some(n),
        ASTNodeOrToken::Token(_) => None,
    })
}

/// Wrap `operand` in a well-known, pure, single-argument
/// [`Expr::BuiltinCall`] named `name`, reusing `operand`'s own span.
fn wrap_builtin(name: &str, operand: Expr) -> Expr {
    let span = operand.span().clone();
    Expr::BuiltinCall {
        name: name.to_string(),
        args: vec![operand],
        effects: EffectSet::PURE,
        span,
    }
}

/// The glyph for one of the six comparison [`ElementwiseOpKind`]s -- used
/// only to name the offending atom in the "no monadic form" error message.
fn glyph_for_comparison(op: ElementwiseOpKind) -> &'static str {
    match op {
        ElementwiseOpKind::Eq => "=",
        ElementwiseOpKind::Ne => "≠",
        ElementwiseOpKind::Lt => "<",
        ElementwiseOpKind::Le => "≤",
        ElementwiseOpKind::Ge => "≥",
        ElementwiseOpKind::Gt => ">",
        other => unreachable!("glyph_for_comparison called with non-comparison op {other:?}"),
    }
}

/// Convert one `NUMBER` token's lexeme text into an `Expr::IntLit` (a whole
/// number that fits `i64`) or `Expr::FloatLit` (has a decimal point or
/// exponent, or is too large for `i64`) -- mirrors
/// `matlab-to-semantic-ir::lower::number_literal_expr`'s own lexeme-based
/// int-vs-float convention exactly, so every `-to-semantic-ir` frontend in
/// this repo picks the same literal-kind boundary.
///
/// APL's lexer spells a negative literal's sign with the historical "high
/// minus" `¯` (U+00AF) instead of ASCII `-` (`apl.tokens`'s `NUMBER` rule:
/// `-` is reserved, unambiguously, for the `MINUS` function token, so a
/// literal sign needs its own glyph). The `¯` is translated to `-` first,
/// exactly as `apl-runtime::eval::parse_apl_number` already does
/// (`s.replace('¯', "-")`), before either numeric parser ever sees the text.
///
/// Returns `Err` (rather than silently substituting `0.0`) if `raw_text`
/// fails to parse as a number at all -- unreachable via `compile_source`
/// (the lexer's own `NUMBER` rule guarantees a parseable lexeme), but
/// `compile` is also a public entry point over a hand-built
/// `GrammarASTNode`, and every other malformed-input case in this file is
/// rejected explicitly rather than silently coerced, so this one matches
/// that same discipline (caught by security review).
fn number_literal_expr(raw_text: &str, span: &Span) -> Result<Expr, String> {
    let text = raw_text.replace('¯', "-");
    let invalid = || format!("invalid number literal `{raw_text}`");
    if text.contains('.') || text.contains('e') || text.contains('E') {
        let value = text.parse::<f64>().map_err(|_| invalid())?;
        Ok(Expr::FloatLit { value, span: span.clone() })
    } else {
        match text.parse::<i64>() {
            Ok(v) => Ok(Expr::IntLit { value: v, span: span.clone() }),
            Err(_) => {
                let value = text.parse::<f64>().map_err(|_| invalid())?;
                Ok(Expr::FloatLit { value, span: span.clone() })
            }
        }
    }
}
