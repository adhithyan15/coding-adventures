//! The lowering pass from `coding_adventures_wolfram_parser`'s generic
//! [`GrammarASTNode`] CST → [`semantic_ir::Module`], **v0.1.0**.
//!
//! This is the first frontend to target
//! [SIR23](../../../specs/SIR23-symbolic-pattern-semantic-ir.md), the
//! symbolic-expression/pattern-matching domain extension of the SIR10
//! narrow-waist IR (Stream B of
//! [`HML01`](../../../specs/HML01-math-to-semantic-ir.md)).
//!
//! # "Everything is data" — this frontend's one design decision
//!
//! Wolfram's defining idea (MA04 §1) is that *every* fragment is one
//! expression tree `head[arg, …]`, manipulable as **data** even before (or
//! instead of) being evaluated — `2 + 3` is `Add(2, 3)`, `{1, 2}` is
//! `List(1, 2)`, `x = 5` is `Set(x, 5)`. The existing native
//! `wolfram-runtime` crate lowers to `symbolic_ir::IRNode` with exactly this
//! shape and then *evaluates* it against a live environment (variable
//! bindings, held heads, a pattern-matching VM). This crate lowers to the
//! same shape in `semantic_ir::Expr`'s SIR23 vocabulary
//! (`SymSymbol`/`SymApply`/pattern/rule nodes) but stops there: **every**
//! Wolfram construct — arithmetic, comparisons, lists, function
//! application, even `=`/`:=` assignment — becomes symbolic *data*, never a
//! host-language `Expr::VarRef`/`Stmt::Assign`. There is no environment, no
//! binding, no evaluation at lowering time.
//!
//! This is not a simplification of convenience; it is the only choice that
//! matches the SIR23 spec's stated fidelity goal: representing an
//! **uncomputed** Wolfram function body (`f[x_] := x + 1`, which
//! pattern-matches at call time, not at definition time) requires `x + 1`
//! to already be data *before* `x` has any value. Lowering ordinary
//! arithmetic straight to a host addition would make that impossible; by
//! lowering everything uniformly to `SymApply`, the same code path handles
//! both "this expression happens to be computable now" and "this
//! expression is a pattern-matched template" without special-casing.
//! Evaluating that data (binding `Set`/`SetDelayed`, running the matcher
//! for `SymReplaceAll`) is deliberately left to a **backend runtime**
//! library (`sir-runtime-symbolic`, not yet built — SIR23 spec, "Backend
//! impact"; Stream B rollout item 6) — mirroring the SIR23 spec's own
//! explicit "session state isn't SIR" boundary (the same boundary that lets
//! Macsyma's `assume`/`kill` stay out of the IR entirely).
//!
//! # Scope (v0.1.0)
//!
//! Because everything reduces to the same small SIR23 vocabulary, this
//! crate covers the **full** grammar `wolfram-parser` accepts — there is no
//! scalar/array-style ambiguity here forcing a narrower cut the way there
//! was for MATLAB. Concretely:
//!
//! - Literals: `NUMBER` (int- or float-shaped by lexeme, exactly as the
//!   native `wolfram-runtime` lowering decides), `STRING` → `IntLit`/
//!   `FloatLit`/`StrLit` (SIR10 nodes, reused directly per the SIR23 spec —
//!   no new literal node needed for these three).
//! - Bare symbols (`NAME`) → [`Expr::SymSymbol`] — always data, never a
//!   variable lookup (see above).
//! - Arithmetic (`+ - * / ^`, unary `-`/`+`) and their explicit-head forms
//!   (`Plus[…]`/`Times[…]`/`Power[…]`/`Subtract[…]`/`Divide[…]`/`Minus[…]`)
//!   → [`Expr::SymApply`] with the canonical head (`Add`/`Sub`/`Mul`/`Div`/
//!   `Pow`/`Neg`), bridged through the exact same surface→canonical table
//!   the native `wolfram-runtime::lower` uses, including the same
//!   associative n-ary left-fold for a 3-or-more-argument explicit head
//!   application (`Plus[1, 2, 3]` → `Add(Add(1, 2), 3)`, byte-identical to
//!   the infix `1 + 2 + 3`).
//! - Comparisons (`== != < > <= >=` and `Equal`/`Unequal`/`Less`/`Greater`/
//!   `LessEqual`/`GreaterEqual`), logic (`&& || !` and `And`/`Or`/`Not`),
//!   lists (`{…}` and `List[…]`) → `SymApply`.
//! - Function application `head[args]`, including a *computed* head
//!   (`f[x][y]`) and any unrecognised head (a user-defined function name
//!   passes through unchanged, exactly like the native runtime) →
//!   `SymApply`.
//! - `x = e` (`Set`) and `f[params] := body` (`SetDelayed`) → `SymApply`
//!   with head `Assign`/`Define` — pure data (see "Everything is data"
//!   above); no host-language binding is created or assumed.
//! - The W-6 sugar `/@` (`Map`), `@@` (`Apply`), `x[[i]]` (`Part`, folding
//!   a multi-index `x[[i, j]]` into nested `Part`s exactly as the native
//!   lowering does) → `SymApply` with those well-known heads.
//! - The W-11 pure-function forms `#`/`#n` (`Slot[n]`), `##`
//!   (`SlotSequence[1]`), `expr &` (`Function[expr]`), and the named
//!   long form `Function[params, body]` (params normalised to a `List`,
//!   mirroring the native lowering) → `SymApply`.
//! - The W-21 sugar `a | b` (`Alternatives`), `patt /; test` (`Condition`),
//!   `patt ? fn` (`PatternTest`) → `SymApply`.
//! - Pattern blanks `_`/`_h` → [`Expr::SymPatternBlank`]; named `x_`/`x_h`
//!   → [`Expr::SymPatternNamed`].
//! - Rules `a -> b` (eager) / `a :> b` (delayed) → [`Expr::SymRule`],
//!   including the same pattern-name-to-reference rewriting on the RHS the
//!   native lowering performs (see [`bind_pattern_refs`]) so a later
//!   `sir-runtime-symbolic` matcher can substitute bindings the same way
//!   `cas-pattern-matching::substitute` does today.
//! - Replacement `expr /. rules` (one pass) / `expr //. rules` (fixed
//!   point) → [`Expr::SymReplaceAll`], flattening a `{…}` list of rules on
//!   the right-hand side into the node's `rules: Vec<Expr>` (a bare single
//!   rule, or any other computed expression, becomes a one-element `Vec`
//!   — the spec explicitly allows a non-`SymRule` element there as "a
//!   backend concern, not an IR shape").
//!
//! **Deliberately out of scope, disclosed rather than silently
//! mis-lowered:**
//! - Sequence patterns (`__`/`___`), `Repeated`/`Except`/`Longest`/
//!   `Shortest`, `Replace` level-specs — SIR23 tracks the native
//!   `wolfram-runtime`'s own still-open W-20 deferred-feature list exactly,
//!   and this grammar subset has no surface syntax for them regardless.
//! - `SymRational` is part of the SIR23 vocabulary but **unreachable**
//!   from this grammar's surface syntax: there is no dedicated rational
//!   *literal* token (`1/3` parses as `Div[1, 3]`, an ordinary `SymApply`,
//!   exactly as the native lowering treats it) — a future constant-folding
//!   pass could collapse that into a `SymRational` at lowering time, but
//!   this crate does not attempt constant folding.
//! - Evaluating anything: no environment, no binding, no pattern matching
//!   happens in this crate — see "Everything is data" above. This means
//!   **no** module this crate produces currently executes end-to-end
//!   through any backend (`sir-runtime-symbolic`, the JS/TS runtime
//!   library SIR23 codegen needs, does not exist yet — Stream B rollout
//!   item 6, after this frontend). This crate's tests verify structural
//!   correctness (exact `Expr` shapes) and the capability-rejection path
//!   (the validator confirms the manifest declares exactly the features
//!   used; every JS-backend check is expected to *reject*, not accept, a
//!   module using SIR23 nodes) — there is no e2e `node`-execution test,
//!   unlike `matlab-to-semantic-ir`'s purely-literal case, because no
//!   Wolfram program (not even bare literal arithmetic) can avoid emitting
//!   at least one SIR23 node under the "everything is data" design.
//!
//! # Recursion-depth hardening
//!
//! Per-construct chain-length checks (see [`MAX_EXPR_DEPTH`] and
//! [`Lowerer::check_chain_length`]/[`Lowerer::add_chain_depth`]) were
//! applied from day one rather than retrofitted, but two rounds of
//! security review still found real gaps in them: `postfix`/`amp` share
//! the same flat-repetition grammar shape without being on the "obviously
//! chain-shaped" list, and — even once every construct had its own
//! guard — those guards are each scoped to one grammar node, so chaining
//! several independently-in-bounds constructs across `(...)` boundaries
//! still composes past the cap (see [`measure_depth_iterative`], the
//! authoritative, construction-composition-independent check this crate
//! ultimately relies on; `CHANGELOG.md` has the full history of what each
//! round found).

use std::collections::HashSet;

use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span, Stmt,
};
use symbolic_ir::{
    ADD, AND, ASSIGN, DEFINE, DIV, EQUAL, GREATER, GREATER_EQUAL, LESS, LESS_EQUAL, LIST, MUL,
    NEG, NOT, NOT_EQUAL, OR, POW, SUB,
};

/// Maximum expression-nesting depth for *this crate's own* lowering
/// recursion — distinct from (and independent of) `wolfram-parser`'s own
/// `MAX_RULE_DEPTH` grammar-nesting guard, which bounds the CST this crate
/// walks. Mirrors every other SIR frontend's identically-named,
/// identically-justified guard: turns pathologically deep (but parseable)
/// input into a clean [`WolframLowerError`] instead of a native
/// (uncatchable) stack overflow.
///
/// This value (256) is comfortably above the ~98 real nesting levels
/// `wolfram-parser`'s own default `MAX_RULE_DEPTH` (2000) permits on the
/// enlarged-stack worker thread [`compile_source`] parses on (see that
/// function's doc comment) — so in practice this guard is a defense-in-depth
/// backstop against genuine bracket/`f[…]` nesting, which the parser's own
/// cap already bounds first. It is *not* a backstop against a flat,
/// unparenthesized operator chain — that risk is real and structurally
/// different (a long `1 + 1 + ... + 1` run never nests at the CST level at
/// all), and is handled separately by [`Lowerer::check_chain_length`].
const MAX_EXPR_DEPTH: usize = 256;

/// Synthetic file name used for all spans (the CST does not carry the
/// original path).
const FILE: &str = "<wolfram>";

// ---------------------------------------------------------------------------
// Surface-only head names (not exported by `symbolic_ir`, since these are
// synthetic heads a Wolfram-family frontend introduces, not part of the
// shared symbolic-IR vocabulary itself — mirrors the native
// `wolfram-runtime::lower`'s own identically-named local constants).
// ---------------------------------------------------------------------------

const ALTERNATIVES_HEAD: &str = "Alternatives";
const CONDITION_HEAD: &str = "Condition";
const PATTERN_TEST_HEAD: &str = "PatternTest";
const MAP_HEAD: &str = "Map";
const APPLY_HEAD: &str = "Apply";
const PART_HEAD: &str = "Part";
const SLOT_HEAD: &str = "Slot";
const SLOT_SEQUENCE_HEAD: &str = "SlotSequence";
const FUNCTION_HEAD: &str = "Function";

// ---------------------------------------------------------------------------
// Public error type
// ---------------------------------------------------------------------------

/// An error encountered during Wolfram → SIR lowering.
///
/// Mirrors `MatlabLowerError`/`PythonLowerError`'s shape exactly
/// (`message` + 1-based `line`/`column`) so tooling can treat every SIR
/// frontend uniformly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WolframLowerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for WolframLowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "WolframLowerError at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for WolframLowerError {}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Lower a parsed Wolfram CST (rooted at the `program` rule) into a SIR
/// module.
///
/// This function does **not** itself guard against native stack overflow
/// on deeply-nested input beyond its own [`MAX_EXPR_DEPTH`] cap — it trusts
/// `tree` was already parsed under a suitable guard (see
/// [`compile_source`]'s doc comment for the hardened, recommended entry
/// point). This mirrors `matlab-to-semantic-ir::compile`'s identical
/// division of responsibility: `compile` is pure lowering over an
/// already-parsed tree, safe to call directly on an ordinary thread because
/// `MAX_EXPR_DEPTH` (256) alone is a modest, bare-stack-safe recursion
/// budget; `compile_source` is the guarded, parse-and-lower convenience
/// wrapper untrusted-input callers should prefer.
pub fn compile(tree: &GrammarASTNode, module_name: &str) -> Result<Module, WolframLowerError> {
    Lowerer::new(module_name).lower_file(tree)
}

// ---------------------------------------------------------------------------
// The lowerer
// ---------------------------------------------------------------------------

/// The lowering pass's only mutable state: the module name (fixed at
/// construction) and the set of SIR features observed while lowering (used
/// to build the manifest so it declares *exactly* what the module emits —
/// see `semantic-ir/src/validator.rs`'s `check_expr` for the ground truth
/// this must match node-kind-for-node-kind).
///
/// Unlike `matlab-to-semantic-ir`'s `Lowerer`, there is no per-function
/// name-resolution context (`FunctionCtx`, a `locals`/`params` set) here at
/// all — under the "everything is data" design (see the module doc
/// comment) there are no host variables to resolve, so this lowerer is a
/// near-stateless recursive descent over the CST.
struct Lowerer {
    module_name: String,
    observed: FeatureManifest,
}

impl Lowerer {
    fn new(module_name: &str) -> Self {
        Self {
            module_name: module_name.to_string(),
            observed: FeatureManifest::new(),
        }
    }

    // -------------------------------------------------------------------
    // top level: `program = { statement_line }`
    // -------------------------------------------------------------------

    fn lower_file(&mut self, program: &GrammarASTNode) -> Result<Module, WolframLowerError> {
        if program.rule_name != "program" {
            return Err(self.err_at(
                program,
                format!("expected `program` root, got `{}`", program.rule_name),
            ));
        }

        let mut stmts: Vec<Stmt> = Vec::new();
        for line in child_nodes(program) {
            if line.rule_name != "statement_line" {
                continue;
            }
            // A statement_line is `statement (NEWLINE|SEMI) | statement |
            // NEWLINE | SEMI`; only the first two carry an inner `statement`
            // node -- a bare terminator (blank line) contributes nothing.
            let Some(stmt) = first_child_named(line, "statement") else {
                continue;
            };
            let expr = self.lower_node(stmt, 0)?;
            if measure_depth_iterative(&expr).is_none() {
                let err = self.err_at(
                    stmt,
                    format!("expression tree too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
                );
                drop_iterative(expr);
                return Err(err);
            }
            let span = expr.span().clone();
            stmts.push(Stmt::ExprStmt { expr, span });
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
            .with_source_language("wolfram")
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

    // -------------------------------------------------------------------
    // Dispatch
    // -------------------------------------------------------------------

    /// Lower a single arbitrary node, one level deeper than `depth`.
    ///
    /// Most grammar rules are "transparent wrappers" — a precedence level
    /// that did not apply its own operator still emits its own node with a
    /// single child. [`unwrap_single`] peels those away so we dispatch on
    /// the first rule that genuinely shapes the tree.
    fn lower_node(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, WolframLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            ));
        }
        match unwrap_single(node) {
            Unwrapped::Token(token) => self.lower_token(token),
            Unwrapped::Node(node) => match node.rule_name.as_str() {
                "program" => Err(self.err_at(node, "nested program node is not an expression".to_string())),
                "statement_line" | "statement" | "expr" => self.lower_first_node(node, depth),
                "assignment" => self.lower_assignment(node, depth),
                "replaceall" => self.lower_replaceall(node, depth),
                "rule" => self.lower_rule(node, depth),
                "condition" => self.lower_condition(node, depth),
                "alternatives" => self.lower_alternatives(node, depth),
                "patterntest" => self.lower_patterntest(node, depth),
                "logical_or" => self.lower_logical_chain(node, depth, OR),
                "logical_and" => self.lower_logical_chain(node, depth, AND),
                "logical_not" => self.lower_logical_not(node, depth),
                "comparison" => self.lower_comparison(node, depth),
                "additive" | "multiplicative" => self.lower_binary_chain(node, depth),
                "unary" => self.lower_unary(node, depth),
                "power" => self.lower_power(node, depth),
                "amp" => self.lower_amp(node, depth),
                "mapapply" => self.lower_mapapply(node, depth),
                "postfix" => self.lower_postfix(node, depth),
                "atom" => self.lower_atom(node, depth),
                "slot" => self.lower_slot(node),
                "list" => self.lower_list(node, depth),
                "group" => self.lower_group(node, depth),
                "arglist" => Err(self.err_at(
                    node,
                    "an arglist cannot be lowered as a scalar expression".to_string(),
                )),
                other => Err(self.err_at(node, format!("no lowering for rule `{other}`"))),
            },
        }
    }

    /// Lower a raw token (a literal or a bare symbol).
    fn lower_token(&mut self, token: &Token) -> Result<Expr, WolframLowerError> {
        let span = self.token_span(token);
        match token_type(token) {
            "NUMBER" => Ok(self.number_literal_expr(&token.value, span)),
            "NAME" => Ok(self.sym_symbol(token.value.clone(), span)),
            "STRING" => Ok(Expr::StrLit {
                value: strip_quotes(&token.value).to_string(),
                span,
            }),
            // A lone `_` is `Blank()` -- see `lower_atom`'s doc comment for
            // why the token-level arm is the right place to interpret it.
            "BLANK" => Ok(self.pattern_blank(None, span)),
            "HASH" => Ok(self.slot_apply(1, span)),
            "SLOTSEQ" => Ok(self.slot_sequence_apply(span)),
            other => Err(WolframLowerError {
                message: format!("unexpected token `{other}` = {:?}", token.value),
                line: token.line,
                column: token.column,
            }),
        }
    }

    /// `assignment = replaceall [ ( SET | SETDELAYED ) assignment ]`.
    ///
    /// `x = e` lowers to `SymApply{head: Assign, args: [x, e]}`; `f[x_] :=
    /// e` to `SymApply{head: Define, args: [f[x_], e]}` -- both pure data
    /// (see the module doc comment's "Everything is data"); no host
    /// binding is created.
    fn lower_assignment(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, WolframLowerError> {
        let Some(op_index) = node
            .children
            .iter()
            .position(|c| as_token(c).is_some_and(|t| matches!(token_type(t), "SET" | "SETDELAYED")))
        else {
            return self.lower_first_node(node, depth);
        };
        if op_index == 0 || op_index + 1 >= node.children.len() {
            return Err(self.err_at(node, "malformed assignment node".to_string()));
        }
        let lhs = self.lower_child(&node.children[op_index - 1], depth + 1)?;
        let rhs = self.lower_child(&node.children[op_index + 1], depth + 1)?;
        let op = token_type(as_token(&node.children[op_index]).unwrap());
        let span = self.span_of(node);
        let head = if op == "SETDELAYED" { DEFINE } else { ASSIGN };
        Ok(self.sym_apply(self.sym_symbol_bare(head, span.clone()), vec![lhs, rhs], span))
    }

    /// `replaceall = rule { ( REPLACEALL | REPLACEREPEATED ) rule }` --
    /// left-associative `/.` and `//.`.
    ///
    /// `e /. r1 /. r2` folds left into nested `SymReplaceAll`s, exactly as
    /// the native lowering folds nested `ReplaceAll` applies. The RHS of
    /// each step is flattened into `rules: Vec<Expr>` -- a `{…}` list
    /// literal's elements become the vec directly; anything else becomes a
    /// single-element vec (the spec explicitly allows a non-`SymRule`
    /// element there as a runtime concern).
    fn lower_replaceall(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, WolframLowerError> {
        self.check_chain_length(node)?;
        if node.children.len() == 1 {
            return self.lower_child(&node.children[0], depth + 1);
        }
        let mut children = node.children.iter();
        let first = children
            .next()
            .ok_or_else(|| self.err_at(node, "empty replaceall node".to_string()))?;
        let mut result = self.lower_child(first, depth + 1)?;
        while let Some(op_child) = children.next() {
            let repeated = match as_token(op_child).map(token_type) {
                Some("REPLACEALL") => false,
                Some("REPLACEREPEATED") => true,
                _ => return Err(self.err_at(node, "expected a `/.` or `//.` operator".to_string())),
            };
            let rhs_child = children
                .next()
                .ok_or_else(|| self.err_at(node, "`/.`/`//.` with no right operand".to_string()))?;
            let rules = self.lower_rules_operand(rhs_child, depth + 1)?;
            let span = self.span_of(node);
            self.observed.add(Feature::PatternMatching);
            result = Expr::SymReplaceAll {
                expr: Box::new(result),
                rules,
                repeated,
                span,
            };
        }
        Ok(result)
    }

    /// Flatten the right-hand operand of `/.`/`//.` into a `Vec<Expr>` of
    /// rules: a `{…}` list literal's elements unpack directly; anything
    /// else (a single rule, a variable, a computed expression) becomes a
    /// one-element vec.
    fn lower_rules_operand(
        &mut self,
        child: &ASTNodeOrToken,
        depth: usize,
    ) -> Result<Vec<Expr>, WolframLowerError> {
        if let ASTNodeOrToken::Node(node) = child {
            if unwrap_single(node).is_list() {
                if let Unwrapped::Node(list_node) = unwrap_single(node) {
                    return self.lower_list_elements(list_node, depth);
                }
            }
        }
        Ok(vec![self.lower_child(child, depth)?])
    }

    /// `rule = logical_or [ ( RULE | RULEDELAYED ) rule ]` --
    /// right-associative.
    fn lower_rule(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, WolframLowerError> {
        let Some(op_index) = node
            .children
            .iter()
            .position(|c| as_token(c).is_some_and(|t| matches!(token_type(t), "RULE" | "RULEDELAYED")))
        else {
            return self.lower_first_node(node, depth);
        };
        if op_index == 0 || op_index + 1 >= node.children.len() {
            return Err(self.err_at(node, "malformed rule node".to_string()));
        }
        let lhs = self.lower_child(&node.children[op_index - 1], depth + 1)?;
        let rhs = self.lower_child(&node.children[op_index + 1], depth + 1)?;
        let delayed = token_type(as_token(&node.children[op_index]).unwrap()) == "RULEDELAYED";
        let span = self.span_of(node);

        // `collect_pattern_names`/`bind_pattern_refs` below recurse over
        // `lhs`/`rhs` with no depth cap of their own (see their doc
        // comments -- they assume whatever tree they're handed is already
        // bounded). That assumption does not hold in general: `depth`
        // alone only bounds *this crate's own* CST-walking recursion, not
        // the true depth of a tree a flat-chain fold (postfix/amp/etc, see
        // `add_chain_depth`) may have built -- and per-construct chain
        // budgets don't compose across nested grammar boundaries (a
        // security review found chaining several independently-capped
        // constructs, e.g. through `(...)` boundaries, can still build a
        // tree far deeper than any single guard's own limit). Measure the
        // TRUE depth authoritatively and iteratively (never recursively,
        // so this check itself can never crash regardless of how deep the
        // tree already is -- building a deep `Box`-based tree costs heap,
        // not stack, so it's always safe to measure after the fact) before
        // handing `lhs`/`rhs` to either unguarded recursive helper.
        if measure_depth_iterative(&lhs).is_none() || measure_depth_iterative(&rhs).is_none() {
            let err = self.err_at(
                node,
                format!("expression tree too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            );
            // Tear both down iteratively before returning -- letting either
            // fall out of scope normally here would recurse through the
            // ordinary derived `Drop` glue on exactly the pathologically
            // deep tree we just detected (see `drop_iterative`'s doc
            // comment).
            drop_iterative(lhs);
            drop_iterative(rhs);
            return Err(err);
        }

        // Wolfram binds a pattern name on the LHS (`t_`) and refers to it as
        // a *bare* symbol on the RHS (`-> t`); a later matcher only fills in
        // `SymPatternNamed` reference nodes, so we rewrite every RHS
        // occurrence of a name bound on the LHS into that reference shape --
        // see `bind_pattern_refs`'s doc comment.
        let mut bound = HashSet::new();
        collect_pattern_names(&lhs, &mut bound);
        let rhs = bind_pattern_refs(rhs, &bound);

        self.observed.add(Feature::PatternMatching);
        Ok(Expr::SymRule {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
            delayed,
            span,
        })
    }

    /// `condition = alternatives [ CONDITION condition ]` -- the `/;`
    /// operator, right-associative. Lowers to an ordinary `SymApply` with
    /// head `Condition` (no dedicated SIR23 node; see the module doc
    /// comment). Unlike `lower_rule`, the test keeps its bare
    /// named-symbol references -- we do NOT run `bind_pattern_refs` here,
    /// matching the native lowering's identical reasoning (a future
    /// `sir-runtime-symbolic` `Condition` evaluator substitutes bindings
    /// into the test at match time, not at lowering time).
    fn lower_condition(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, WolframLowerError> {
        let Some(op_index) = node
            .children
            .iter()
            .position(|c| as_token(c).is_some_and(|t| token_type(t) == "CONDITION"))
        else {
            return self.lower_first_node(node, depth);
        };
        if op_index == 0 || op_index + 1 >= node.children.len() {
            return Err(self.err_at(node, "malformed condition node".to_string()));
        }
        let patt = self.lower_child(&node.children[op_index - 1], depth + 1)?;
        let test = self.lower_child(&node.children[op_index + 1], depth + 1)?;
        let span = self.span_of(node);
        Ok(self.sym_apply(self.sym_symbol_bare(CONDITION_HEAD, span.clone()), vec![patt, test], span))
    }

    /// `alternatives = logical_or { ALTERNATIVES logical_or }` -- the `|`
    /// operator. Folds into one n-ary `SymApply{head: Alternatives, …}`.
    fn lower_alternatives(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, WolframLowerError> {
        self.check_chain_length(node)?;
        let operands = self.lower_child_nodes(node, depth + 1)?;
        match operands.len() {
            0 => Err(self.err_at(node, "empty alternatives node".to_string())),
            1 => Ok(operands.into_iter().next().unwrap()),
            _ => {
                let span = self.span_of(node);
                Ok(self.sym_apply(self.sym_symbol_bare(ALTERNATIVES_HEAD, span.clone()), operands, span))
            }
        }
    }

    /// `patterntest = postfix { PATTERNTEST postfix }` -- the `?` operator,
    /// left-associative.
    fn lower_patterntest(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, WolframLowerError> {
        self.check_chain_length(node)?;
        if node.children.len() == 1 {
            return self.lower_child(&node.children[0], depth + 1);
        }
        let mut children = node.children.iter();
        let first = children
            .next()
            .ok_or_else(|| self.err_at(node, "empty patterntest node".to_string()))?;
        let mut result = self.lower_child(first, depth + 1)?;
        while let Some(op_child) = children.next() {
            if as_token(op_child).map(token_type) != Some("PATTERNTEST") {
                return Err(self.err_at(node, "expected a `?` operator".to_string()));
            }
            let rhs = children
                .next()
                .ok_or_else(|| self.err_at(node, "`?` with no right operand".to_string()))?;
            let rhs_expr = self.lower_child(rhs, depth + 1)?;
            let span = self.span_of(node);
            result = self.sym_apply(
                self.sym_symbol_bare(PATTERN_TEST_HEAD, span.clone()),
                vec![result, rhs_expr],
                span,
            );
        }
        Ok(result)
    }

    /// `logical_or`/`logical_and` -- fold operands into an n-ary
    /// `And`/`Or` `SymApply`.
    fn lower_logical_chain(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
        head: &str,
    ) -> Result<Expr, WolframLowerError> {
        self.check_chain_length(node)?;
        let operands = self.lower_child_nodes(node, depth + 1)?;
        match operands.len() {
            0 => Err(self.err_at(node, "empty logical chain".to_string())),
            1 => Ok(operands.into_iter().next().unwrap()),
            _ => {
                let span = self.span_of(node);
                Ok(self.sym_apply(self.sym_symbol_bare(head, span.clone()), operands, span))
            }
        }
    }

    /// `logical_not = NOT logical_not | comparison`.
    fn lower_logical_not(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, WolframLowerError> {
        let has_not = node
            .children
            .iter()
            .any(|c| as_token(c).is_some_and(|t| token_type(t) == "NOT"));
        if !has_not {
            return self.lower_first_node(node, depth);
        }
        let inner = child_nodes(node)
            .into_iter()
            .next()
            .ok_or_else(|| self.err_at(node, "`!` with no operand".to_string()))?;
        let operand = self.lower_node(inner, depth + 1)?;
        let span = self.span_of(node);
        Ok(self.sym_apply(self.sym_symbol_bare(NOT, span.clone()), vec![operand], span))
    }

    /// `comparison = additive [ op additive ]` -- a single, non-chained
    /// comparison (the grammar does not flatten a chain here, so no
    /// `check_chain_length` call is needed).
    fn lower_comparison(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, WolframLowerError> {
        let Some(op_index) = node
            .children
            .iter()
            .position(|c| as_token(c).is_some_and(|t| comparison_head(token_type(t)).is_some()))
        else {
            return self.lower_first_node(node, depth);
        };
        if op_index == 0 || op_index + 1 >= node.children.len() {
            return Err(self.err_at(node, "malformed comparison node".to_string()));
        }
        let head = comparison_head(token_type(as_token(&node.children[op_index]).unwrap())).unwrap();
        let lhs = self.lower_child(&node.children[op_index - 1], depth + 1)?;
        let rhs = self.lower_child(&node.children[op_index + 1], depth + 1)?;
        let span = self.span_of(node);
        Ok(self.sym_apply(self.sym_symbol_bare(head, span.clone()), vec![lhs, rhs], span))
    }

    /// `additive`/`multiplicative` -- a left-associative chain of
    /// `+`/`-`/`*`/`/`.
    ///
    /// The Wolfram grammar, like MATLAB's, collapses a flat run of
    /// same-precedence operators into ONE CST node with many children
    /// rather than nesting through parens -- see [`Self::check_chain_length`]
    /// for why this specifically needs its own cap independent of
    /// `MAX_EXPR_DEPTH`.
    fn lower_binary_chain(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, WolframLowerError> {
        self.check_chain_length(node)?;
        let mut children = node.children.iter();
        let first = children
            .next()
            .ok_or_else(|| self.err_at(node, "empty binary chain".to_string()))?;
        let mut result = self.lower_child(first, depth + 1)?;
        while let Some(op_child) = children.next() {
            let head = as_token(op_child)
                .and_then(|t| binary_head(token_type(t)))
                .ok_or_else(|| self.err_at(node, "expected a binary operator".to_string()))?;
            let rhs = children
                .next()
                .ok_or_else(|| self.err_at(node, "binary operator with no right operand".to_string()))?;
            let rhs_expr = self.lower_child(rhs, depth + 1)?;
            let span = self.span_of(node);
            result = self.sym_apply(self.sym_symbol_bare(head, span.clone()), vec![result, rhs_expr], span);
        }
        Ok(result)
    }

    /// `unary = ( MINUS | PLUS ) unary | power`.
    fn lower_unary(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, WolframLowerError> {
        if node.children.len() == 1 {
            return self.lower_child(&node.children[0], depth + 1);
        }
        let op = token_type(
            as_token(&node.children[0])
                .ok_or_else(|| self.err_at(node, "unary op must be a token".to_string()))?,
        );
        let operand = self.lower_child(
            node.children
                .get(1)
                .ok_or_else(|| self.err_at(node, "unary op with no operand".to_string()))?,
            depth + 1,
        )?;
        if op == "MINUS" {
            let span = self.span_of(node);
            Ok(self.sym_apply(self.sym_symbol_bare(NEG, span.clone()), vec![operand], span))
        } else {
            Ok(operand) // unary plus is a no-op
        }
    }

    /// `power = postfix [ POWER unary ]` -- right-associative `^`.
    fn lower_power(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, WolframLowerError> {
        match node.children.len() {
            1 => self.lower_child(&node.children[0], depth + 1),
            3 => {
                let lhs = self.lower_child(&node.children[0], depth + 1)?;
                let rhs = self.lower_child(&node.children[2], depth + 1)?;
                let span = self.span_of(node);
                Ok(self.sym_apply(self.sym_symbol_bare(POW, span.clone()), vec![lhs, rhs], span))
            }
            _ => Err(self.err_at(node, "malformed power node".to_string())),
        }
    }

    /// `mapapply = postfix { ( MAP | APPLY ) postfix }` -- the `/@`/`@@`
    /// operator sugar, infix and left-associative.
    fn lower_mapapply(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, WolframLowerError> {
        self.check_chain_length(node)?;
        if node.children.len() == 1 {
            return self.lower_child(&node.children[0], depth + 1);
        }
        let mut children = node.children.iter();
        let first = children
            .next()
            .ok_or_else(|| self.err_at(node, "empty mapapply node".to_string()))?;
        let mut result = self.lower_child(first, depth + 1)?;
        while let Some(op_child) = children.next() {
            let head = match as_token(op_child).map(token_type) {
                Some("MAP") => MAP_HEAD,
                Some("APPLY") => APPLY_HEAD,
                _ => return Err(self.err_at(node, "expected a `/@` or `@@` operator".to_string())),
            };
            let rhs = children
                .next()
                .ok_or_else(|| self.err_at(node, "`/@`/`@@` with no right operand".to_string()))?;
            let rhs_expr = self.lower_child(rhs, depth + 1)?;
            let span = self.span_of(node);
            result = self.sym_apply(self.sym_symbol_bare(head, span.clone()), vec![result, rhs_expr], span);
        }
        Ok(result)
    }

    /// `postfix = atom { LBRACKET [ arglist ] RBRACKET | LDBRACKET arglist
    /// RBRACKET RBRACKET }` -- function application and the `[[ … ]]` part
    /// sugar, both postfix, left-associative and chainable.
    ///
    /// Tracks a *cumulative* nesting-depth budget (`chain_depth`) across the
    /// whole chain rather than separately capping "how many bracket groups"
    /// and "how many indices/args per group": those two axes multiply, not
    /// add -- an `LDBRACKET` group folds one `Part` per index, so N chained
    /// groups each carrying M indices builds N×M levels of nesting, not N.
    /// A prior version of this guard counted groups only (bounding N to
    /// `MAX_EXPR_DEPTH`) and relied on `check_apply_arg_count` to separately
    /// bound M per group to `MAX_EXPR_DEPTH` -- a security review found this
    /// still permits up to `MAX_EXPR_DEPTH`² levels of real nesting (256×256
    /// far exceeds the intended cap), confirmed to reproduce the same class
    /// of native stack overflow `check_chain_length`'s own doc comment
    /// describes. See [`Self::add_chain_depth`] for the shared budget this
    /// function and [`Self::lower_amp_apply`] both draw from.
    fn lower_postfix(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, WolframLowerError> {
        let mut result = self.lower_child(
            node.children
                .first()
                .ok_or_else(|| self.err_at(node, "postfix has no base".to_string()))?,
            depth + 1,
        )?;

        let mut chain_depth: usize = 0;
        let mut i = 1;
        while i < node.children.len() {
            let Some(token) = as_token(&node.children[i]) else {
                i += 1;
                continue;
            };
            match token_type(token) {
                "LBRACKET" => {
                    let args = node
                        .children
                        .get(i + 1)
                        .and_then(as_node)
                        .filter(|n| n.rule_name == "arglist")
                        .map(|n| self.lower_arglist(n, depth + 1))
                        .transpose()?
                        .unwrap_or_default();
                    self.check_apply_arg_count(node, args.len())?;
                    chain_depth = self.add_chain_depth(node, chain_depth, args.len().max(1))?;
                    result = self.build_application(result, args, node);
                }
                "LDBRACKET" => {
                    let indices = node
                        .children
                        .get(i + 1)
                        .and_then(as_node)
                        .filter(|n| n.rule_name == "arglist")
                        .map(|n| self.lower_arglist(n, depth + 1))
                        .transpose()?
                        .unwrap_or_default();
                    self.check_apply_arg_count(node, indices.len())?;
                    chain_depth = self.add_chain_depth(node, chain_depth, indices.len().max(1))?;
                    for index in indices {
                        let span = self.span_of(node);
                        result = self.sym_apply(
                            self.sym_symbol_bare(PART_HEAD, span.clone()),
                            vec![result, index],
                            span,
                        );
                    }
                }
                _ => {}
            }
            i += 1;
        }
        Ok(result)
    }

    /// Apply `head` to `args`, bridging built-in surface heads to their
    /// canonical IR head, and left-folding a 3-or-more-argument
    /// associative application (`Plus`/`Times`/`And`/`Or`) into a binary
    /// chain -- mirrors the native `wolfram-runtime::lower::build_application`
    /// exactly, so `Plus[1, 2, 3]` and `1 + 2 + 3` produce byte-identical
    /// IR.
    fn build_application(&mut self, head: Expr, args: Vec<Expr>, node: &GrammarASTNode) -> Expr {
        let span = self.span_of(node);
        let canonical_head = match &head {
            Expr::SymSymbol { name, span } => surface_head_to_ir(name)
                .map(|c| self.sym_symbol_bare(c, span.clone()))
                .unwrap_or_else(|| head.clone()),
            _ => head,
        };
        if let Expr::SymSymbol { name, .. } = &canonical_head {
            if name == FUNCTION_HEAD && args.len() == 2 && !is_list_apply(&args[0]) {
                let mut it = args.into_iter();
                let param = it.next().unwrap();
                let body = it.next().unwrap();
                let list_span = self.span_of(node);
                let params = self.sym_apply(
                    self.sym_symbol_bare(LIST, list_span.clone()),
                    vec![param],
                    list_span,
                );
                return self.sym_apply(canonical_head, vec![params, body], span);
            }
            if matches!(name.as_str(), ADD | MUL | AND | OR) && args.len() > 2 {
                let mut iter = args.into_iter();
                let mut acc = iter.next().unwrap();
                for next in iter {
                    let step_span = self.span_of(node);
                    acc = self.sym_apply(
                        self.sym_symbol_bare(name, step_span.clone()),
                        vec![acc, next],
                        step_span,
                    );
                }
                return acc;
            }
        }
        self.sym_apply(canonical_head, args, span)
    }

    /// `arglist = expr { COMMA expr }` -- lower each comma-separated
    /// argument. An arglist is a flat `Vec`, not a folded tree, so it has
    /// no stack-recursion risk analogous to the binary-chain rules --
    /// [`Self::check_apply_arg_count`] still bounds its raw length as a
    /// modest defense-in-depth cap on allocation size.
    fn lower_arglist(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Vec<Expr>, WolframLowerError> {
        self.lower_child_nodes(node, depth)
    }

    /// `atom = NUMBER | STRING | NAME [ BLANK [ NAME ] ] | BLANK [ NAME ] |
    /// list | group | slot`.
    fn lower_atom(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, WolframLowerError> {
        if let Some(child) = child_nodes(node).into_iter().next() {
            if matches!(child.rule_name.as_str(), "list" | "group" | "slot") {
                return self.lower_node(child, depth + 1);
            }
        }
        let tokens: Vec<&Token> = node.children.iter().filter_map(as_token).collect();
        let span = self.span_of(node);
        match tokens.as_slice() {
            [b, rest @ ..] if token_type(b) == "BLANK" => Ok(self.blank_from_tokens(rest, span)),
            [name, b, rest @ ..] if token_type(name) == "NAME" && token_type(b) == "BLANK" => {
                let inner = self.blank_from_tokens(rest, span.clone());
                self.observed.add(Feature::PatternMatching);
                Ok(Expr::SymPatternNamed {
                    name: name.value.clone(),
                    pattern: Box::new(inner),
                    span,
                })
            }
            [single] => self.lower_token(single),
            _ => Err(WolframLowerError {
                message: format!(
                    "unrecognised atom token shape: {:?}",
                    tokens.iter().map(|t| &t.value).collect::<Vec<_>>()
                ),
                line: span.start_line,
                column: span.start_col,
            }),
        }
    }

    /// `slot = HASH [ NUMBER ] | SLOTSEQ`.
    fn lower_slot(&mut self, node: &GrammarASTNode) -> Result<Expr, WolframLowerError> {
        let tokens: Vec<&Token> = node.children.iter().filter_map(as_token).collect();
        let span = self.span_of(node);
        match tokens.as_slice() {
            [s] if token_type(s) == "SLOTSEQ" => Ok(self.slot_sequence_apply(span)),
            [h] if token_type(h) == "HASH" => Ok(self.slot_apply(1, span)),
            [h, n] if token_type(h) == "HASH" && token_type(n) == "NUMBER" => {
                let idx = n.value.parse::<i64>().map_err(|e| WolframLowerError {
                    message: format!("invalid slot number {:?}: {e}", n.value),
                    line: n.line,
                    column: n.column,
                })?;
                if idx < 1 {
                    return Err(WolframLowerError {
                        message: format!("slot number must be >= 1, got {idx}"),
                        line: n.line,
                        column: n.column,
                    });
                }
                Ok(self.slot_apply(idx, span))
            }
            _ => Err(WolframLowerError {
                message: format!(
                    "unrecognised slot token shape: {:?}",
                    tokens.iter().map(|t| &t.value).collect::<Vec<_>>()
                ),
                line: span.start_line,
                column: span.start_col,
            }),
        }
    }

    /// `amp = power AMP { AMP } { amp_apply } | power` -- the `&`
    /// pure-function postfix and any immediate trailing application.
    ///
    /// Both repetitions here (`{ AMP }` and `{ amp_apply }`) are flat,
    /// token-or-node runs the grammar folds into ONE `amp` node's children
    /// rather than nesting. `amp_count` levels of `Function`-wrapping is
    /// exact (each `&` adds exactly one level, no multiplication risk), but
    /// each trailing `amp_apply` suffix can itself carry a multi-index
    /// `[[…]]` Part-fold or a multi-arg associative-head call -- see
    /// [`Self::lower_postfix`]'s doc comment for why a per-suffix-count cap
    /// alone is insufficient (the same multiplicative gap applies here), so
    /// the `&`-run and every suffix share one cumulative
    /// [`Self::add_chain_depth`] budget.
    fn lower_amp(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, WolframLowerError> {
        let amp_count = node
            .children
            .iter()
            .filter(|c| as_token(c).is_some_and(|t| token_type(t) == "AMP"))
            .count();
        if amp_count == 0 {
            return self.lower_first_node(node, depth);
        }
        let mut chain_depth = self.add_chain_depth(node, 0, amp_count)?;
        let body = self.lower_first_node(node, depth + 1)?;
        let mut result = body;
        for _ in 0..amp_count {
            let span = self.span_of(node);
            result = self.sym_apply(self.sym_symbol_bare(FUNCTION_HEAD, span.clone()), vec![result], span);
        }
        for suffix in child_nodes(node) {
            if suffix.rule_name == "amp_apply" {
                let (new_result, new_depth) =
                    self.lower_amp_apply(result, suffix, depth + 1, chain_depth)?;
                result = new_result;
                chain_depth = new_depth;
            }
        }
        Ok(result)
    }

    /// Apply one `amp_apply` suffix (`[args]` or `[[i]]`) to an
    /// already-built pure function, threading and returning the updated
    /// cumulative chain-depth budget (see [`Self::lower_amp`]'s doc
    /// comment).
    fn lower_amp_apply(
        &mut self,
        func: Expr,
        suffix: &GrammarASTNode,
        depth: usize,
        chain_depth: usize,
    ) -> Result<(Expr, usize), WolframLowerError> {
        let is_part = suffix
            .children
            .iter()
            .any(|c| as_token(c).is_some_and(|t| token_type(t) == "LDBRACKET"));
        let args = match child_nodes(suffix).into_iter().find(|n| n.rule_name == "arglist") {
            Some(n) => self.lower_arglist(n, depth)?,
            None => vec![],
        };
        self.check_apply_arg_count(suffix, args.len())?;
        let chain_depth = self.add_chain_depth(suffix, chain_depth, args.len().max(1))?;
        if is_part {
            let mut result = func;
            for index in args {
                let span = self.span_of(suffix);
                result = self.sym_apply(self.sym_symbol_bare(PART_HEAD, span.clone()), vec![result, index], span);
            }
            Ok((result, chain_depth))
        } else {
            Ok((self.build_application(func, args, suffix), chain_depth))
        }
    }

    /// `list = LBRACE [ arglist ] RBRACE` → `SymApply{head: List, …}`.
    fn lower_list(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, WolframLowerError> {
        let args = self.lower_list_elements(node, depth)?;
        let span = self.span_of(node);
        Ok(self.sym_apply(self.sym_symbol_bare(LIST, span.clone()), args, span))
    }

    /// The raw element list of a `list` node, without wrapping in `List`
    /// (used both by `lower_list` and by `lower_rules_operand`, which needs
    /// the bare elements rather than a `List` `SymApply`).
    fn lower_list_elements(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Vec<Expr>, WolframLowerError> {
        let mut args = Vec::new();
        for child in child_nodes(node) {
            if child.rule_name == "arglist" {
                args.extend(self.lower_arglist(child, depth + 1)?);
            }
        }
        self.check_apply_arg_count(node, args.len())?;
        Ok(args)
    }

    /// `group = LPAREN expr RPAREN` -- grouping only.
    fn lower_group(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, WolframLowerError> {
        let inner = child_nodes(node)
            .into_iter()
            .next()
            .ok_or_else(|| self.err_at(node, "empty group `( )`".to_string()))?;
        self.lower_node(inner, depth + 1)
    }

    // -------------------------------------------------------------------
    // Small constructors (each observes the feature the validator's own
    // `check_expr` requires for that node kind -- see `semantic-ir/src/
    // validator.rs`, the ground truth this must match exactly).
    // -------------------------------------------------------------------

    fn sym_symbol(&mut self, name: String, span: Span) -> Expr {
        self.observed.add(Feature::SymbolicExpr);
        Expr::SymSymbol { name, span }
    }

    /// Build a `SymSymbol` for a *head* name (identical shape to
    /// [`Self::sym_symbol`] -- named separately only so call sites that
    /// build a head make their intent legible).
    fn sym_symbol_bare(&self, name: impl Into<String>, span: Span) -> Expr {
        Expr::SymSymbol {
            name: name.into(),
            span,
        }
    }

    fn sym_apply(&mut self, head: Expr, args: Vec<Expr>, span: Span) -> Expr {
        self.observed.add(Feature::SymbolicExpr);
        Expr::SymApply {
            head: Box::new(head),
            args,
            span,
        }
    }

    fn pattern_blank(&mut self, head: Option<Box<Expr>>, span: Span) -> Expr {
        self.observed.add(Feature::PatternMatching);
        Expr::SymPatternBlank { head, span }
    }

    fn blank_from_tokens(&mut self, rest: &[&Token], span: Span) -> Expr {
        match rest.first() {
            Some(head) if token_type(head) == "NAME" => {
                let head_span = self.token_span(head);
                self.pattern_blank(Some(Box::new(self.sym_symbol_bare(head.value.clone(), head_span))), span)
            }
            _ => self.pattern_blank(None, span),
        }
    }

    fn slot_apply(&mut self, n: i64, span: Span) -> Expr {
        let int_span = span.clone();
        self.sym_apply(
            self.sym_symbol_bare(SLOT_HEAD, span.clone()),
            vec![Expr::IntLit { value: n, span: int_span }],
            span,
        )
    }

    fn slot_sequence_apply(&mut self, span: Span) -> Expr {
        let int_span = span.clone();
        self.sym_apply(
            self.sym_symbol_bare(SLOT_SEQUENCE_HEAD, span.clone()),
            vec![Expr::IntLit { value: 1, span: int_span }],
            span,
        )
    }

    // -------------------------------------------------------------------
    // Guards
    // -------------------------------------------------------------------

    /// Reject a same-precedence operator chain (`additive`/
    /// `multiplicative`/`logical_or`/`logical_and`/`alternatives`/
    /// `mapapply`/`patterntest`/`replaceall`) with more than
    /// `MAX_EXPR_DEPTH` operands.
    ///
    /// The Wolfram grammar collapses a flat run of same-precedence
    /// operators into ONE CST node with many children rather than nesting
    /// through parens, so a long unparenthesized chain (`1 + 1 + ... + 1`,
    /// tens of thousands of terms) never trips the ordinary grammar-nesting
    /// depth guard (`wolfram-parser`'s `MAX_RULE_DEPTH`, which counts
    /// *nesting*, not the length of one flat repetition). But folding N
    /// operands left-associatively still builds an N-deep *binary* `Expr`
    /// tree, and that tree's own depth is what every later recursive pass
    /// over it pays for (the validator, any backend's emit pass, even plain
    /// `Drop`) regardless of how cheaply each fold step was. This is the
    /// exact bug class `matlab-to-semantic-ir` discovered the hard way
    /// during its own security review (a 60,000-term chain overflowed the
    /// native stack even after fixing that crate's O(1)-per-step
    /// construction cost, because the *structure* was still 60,000 levels
    /// deep) -- applied here from day one instead of being retrofitted.
    fn check_chain_length(&self, node: &GrammarASTNode) -> Result<(), WolframLowerError> {
        let operand_count = node
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(_)))
            .count();
        if operand_count > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!(
                    "expression chain too long ({operand_count} operands, exceeds \
                     {MAX_EXPR_DEPTH})"
                ),
            ));
        }
        Ok(())
    }

    /// Cap the argument count of a single `f[…]`/`{…}`/`x[[…]]` application
    /// or list literal. Unlike [`Self::check_chain_length`], an arglist does
    /// not fold into a nested tree (it stays a flat `Vec<Expr>`), so this is
    /// not a stack-recursion guard -- it is a modest defense-in-depth cap on
    /// a single allocation's size, using the same `MAX_EXPR_DEPTH` bound for
    /// consistency rather than inventing a second unrelated constant.
    fn check_apply_arg_count(&self, node: &GrammarASTNode, count: usize) -> Result<(), WolframLowerError> {
        if count > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("too many arguments ({count}, exceeds {MAX_EXPR_DEPTH})"),
            ));
        }
        Ok(())
    }

    /// Add `delta` to a running cumulative nesting-depth budget shared
    /// across an entire postfix/amp-apply chain, rejecting once the total
    /// exceeds `MAX_EXPR_DEPTH`.
    ///
    /// Used by [`Self::lower_postfix`] and [`Self::lower_amp`]/
    /// [`Self::lower_amp_apply`], whose bracket/part/pure-function-apply
    /// chains are iterative loops that rebuild a result across many
    /// grammar-flattened repetitions -- never recursing through the
    /// depth-capped `lower_node` -- so nothing else bounds how deep the
    /// resulting tree gets.
    ///
    /// This must be a single *cumulative* budget, not independent per-axis
    /// counts: a security review found that separately capping "how many
    /// bracket groups" (to `MAX_EXPR_DEPTH`) and "how many indices/args per
    /// group" (also to `MAX_EXPR_DEPTH`, via [`Self::check_apply_arg_count`])
    /// still permits up to `MAX_EXPR_DEPTH`² levels of real nesting, since
    /// an `LDBRACKET` group folds one `Part` per index and those two axes
    /// multiply rather than add. Every call site therefore threads the same
    /// running total through the whole chain and charges each group's own
    /// contribution (`args.len()`/`indices.len()`, floored at 1) against
    /// it, so the *cumulative* depth the entire chain can add is bounded to
    /// `MAX_EXPR_DEPTH` regardless of how it's distributed across groups.
    fn add_chain_depth(
        &self,
        node: &GrammarASTNode,
        current: usize,
        delta: usize,
    ) -> Result<usize, WolframLowerError> {
        let next = current.saturating_add(delta);
        if next > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!(
                    "chained application/part/pure-function nesting too deep ({next}, exceeds \
                     {MAX_EXPR_DEPTH})"
                ),
            ));
        }
        Ok(next)
    }

    // -------------------------------------------------------------------
    // Small helpers
    // -------------------------------------------------------------------

    fn lower_first_node(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, WolframLowerError> {
        let child = child_nodes(node)
            .into_iter()
            .next()
            .ok_or_else(|| self.err_at(node, format!("`{}` has no expression child", node.rule_name)))?;
        self.lower_node(child, depth + 1)
    }

    fn lower_child(&mut self, child: &ASTNodeOrToken, depth: usize) -> Result<Expr, WolframLowerError> {
        match child {
            ASTNodeOrToken::Node(node) => self.lower_node(node, depth),
            ASTNodeOrToken::Token(token) => self.lower_token(token),
        }
    }

    fn lower_child_nodes(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Vec<Expr>, WolframLowerError> {
        child_nodes(node)
            .into_iter()
            .map(|n| self.lower_node(n, depth))
            .collect()
    }

    fn span_of(&self, node: &GrammarASTNode) -> Span {
        Span::new(
            FILE,
            node.start_line.unwrap_or(1),
            node.start_column.unwrap_or(1),
            node.end_line.unwrap_or(node.start_line.unwrap_or(1)),
            node.end_column.unwrap_or(node.start_column.unwrap_or(1)),
        )
    }

    fn token_span(&self, token: &Token) -> Span {
        Span::point(FILE, token.line, token.column)
    }

    fn err_at(&self, node: &GrammarASTNode, message: String) -> WolframLowerError {
        WolframLowerError {
            message,
            line: node.start_line.unwrap_or(1),
            column: node.start_column.unwrap_or(1),
        }
    }

    /// Parse a `NUMBER` lexeme into an `IntLit` or `FloatLit`, matching the
    /// native `wolfram-runtime::lower::lower_number`'s identical rule (a `.`,
    /// `e`, or `E` means a real; otherwise an integer). An integer lexeme too
    /// large for `i64` falls back to a float rather than silently truncating.
    ///
    /// **Must** be an instance method, not a free function: every branch
    /// that constructs a `FloatLit` calls `self.observed.add(Feature::
    /// Floats)` immediately. Previously this was a free function with no
    /// access to `observed`, so a float-literal-only module never declared
    /// the feature even though `semantic-ir/src/validator.rs`'s `check_expr`
    /// requires it for every `Expr::FloatLit` node — a confirmed, live bug
    /// (any Wolfram program with a float literal failed `semantic_ir::
    /// validate()`), found while implementing `macsyma-to-semantic-ir` and
    /// fixed here.
    fn number_literal_expr(&mut self, text: &str, span: Span) -> Expr {
        if text.contains('.') || text.contains('e') || text.contains('E') {
            self.observed.add(Feature::Floats);
            Expr::FloatLit {
                value: text.parse::<f64>().unwrap_or(0.0),
                span,
            }
        } else {
            match text.parse::<i64>() {
                Ok(v) => Expr::IntLit { value: v, span },
                Err(_) => {
                    self.observed.add(Feature::Floats);
                    Expr::FloatLit {
                        value: text.parse::<f64>().unwrap_or(0.0),
                        span,
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Free helpers (no `&mut self` needed)
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

/// The first *node* child of `node` whose `rule_name == name`.
fn first_child_named<'a>(node: &'a GrammarASTNode, name: &str) -> Option<&'a GrammarASTNode> {
    child_nodes(node).into_iter().find(|n| n.rule_name == name)
}

fn as_node(child: &ASTNodeOrToken) -> Option<&GrammarASTNode> {
    match child {
        ASTNodeOrToken::Node(node) => Some(node),
        ASTNodeOrToken::Token(_) => None,
    }
}

fn as_token(child: &ASTNodeOrToken) -> Option<&Token> {
    match child {
        ASTNodeOrToken::Token(token) => Some(token),
        ASTNodeOrToken::Node(_) => None,
    }
}

fn token_type(token: &Token) -> &str {
    token.effective_type_name()
}

/// Strip the surrounding double-quotes from a `STRING` lexeme (mirrors the
/// native lowering's identical helper).
fn strip_quotes(text: &str) -> &str {
    text.strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(text)
}

/// Map an arithmetic/separator token type to its canonical IR head.
fn binary_head(token_type: &str) -> Option<&'static str> {
    match token_type {
        "PLUS" => Some(ADD),
        "MINUS" => Some(SUB),
        "TIMES" => Some(MUL),
        "SLASH" => Some(DIV),
        _ => None,
    }
}

/// Map a comparison token type to its canonical IR head.
fn comparison_head(token_type: &str) -> Option<&'static str> {
    match token_type {
        "EQUAL" => Some(EQUAL),
        "UNEQUAL" => Some(NOT_EQUAL),
        "LESS" => Some(LESS),
        "GREATER" => Some(GREATER),
        "LE" => Some(LESS_EQUAL),
        "GE" => Some(GREATER_EQUAL),
        _ => None,
    }
}

/// Bridge a Wolfram *surface* head to the canonical IR head for built-ins
/// (`Plus`→`Add`, …). Returns `None` for anything else (already-canonical
/// heads like `Sin`, and any user-defined head, pass through unchanged) --
/// mirrors the native `wolfram-runtime::lower::surface_head_to_ir` table
/// exactly.
fn surface_head_to_ir(name: &str) -> Option<&'static str> {
    Some(match name {
        "Plus" => ADD,
        "Subtract" => SUB,
        "Times" => MUL,
        "Divide" => DIV,
        "Power" => POW,
        "Minus" => NEG,
        "Equal" => EQUAL,
        "Unequal" => NOT_EQUAL,
        "Less" => LESS,
        "Greater" => GREATER,
        "LessEqual" => LESS_EQUAL,
        "GreaterEqual" => GREATER_EQUAL,
        "And" => AND,
        "Or" => OR,
        "Not" => NOT,
        "List" => LIST,
        "Set" => ASSIGN,
        "SetDelayed" => DEFINE,
        _ => return None,
    })
}

/// True if `expr` is a `SymApply{head: List, ..}` -- used to detect an
/// already-list `Function` parameter list (`Function[{x, y}, body]`) vs. a
/// single-symbol one (`Function[x, body]`).
fn is_list_apply(expr: &Expr) -> bool {
    matches!(expr, Expr::SymApply { head, .. } if matches!(head.as_ref(), Expr::SymSymbol { name, .. } if name == LIST))
}

/// Measure `expr`'s true tree depth **iteratively**, using an explicit
/// heap-allocated work stack rather than native recursion, so calling this
/// can never itself overflow the stack no matter how deep `expr` already
/// is -- unlike a naive recursive depth check, which would have exactly
/// the same crash risk as the thing it's trying to detect. Building a
/// deeply-nested `Box`-based tree only costs heap space (each construction
/// step is O(1) stack), so it is always safe to measure a tree's depth
/// *after* it has been fully built; the risk this guards against is only
/// in *walking* it recursively afterward.
///
/// Returns `None` (bailing out early, without doing more than
/// `O(MAX_EXPR_DEPTH * branching factor)` work) as soon as the depth is
/// certain to exceed `MAX_EXPR_DEPTH`, `Some(depth)` otherwise.
///
/// This is the authoritative depth check every other guard in this file
/// (`MAX_EXPR_DEPTH`'s recursion-depth parameter, [`Lowerer::add_chain_depth`])
/// is only an early, cheap approximation of. A security review found that
/// per-construct chain budgets do not compose across nested grammar
/// boundaries -- chaining several independently-capped constructs (e.g.
/// through `(...)` boundaries) can still build a tree far deeper than any
/// single guard's own limit, since each guard only sees its own local
/// slice of the construction. This function is called on the fully-built
/// result before anything recurses over it without its own cap (see
/// [`Lowerer::lower_rule`]'s use before `collect_pattern_names`/
/// `bind_pattern_refs`) and once per top-level statement in
/// [`Lowerer::lower_file`], so no tree this crate hands to a caller (or
/// recurses over internally) can ever actually exceed `MAX_EXPR_DEPTH`,
/// regardless of how its construction was composed.
fn measure_depth_iterative(expr: &Expr) -> Option<usize> {
    // The root itself starts at 0 wraps deep (matching the counting
    // convention `Lowerer::add_chain_depth`/`check_apply_arg_count` already
    // use elsewhere -- a chain of exactly `MAX_EXPR_DEPTH` nested wraps
    // atop a leaf is "at the cap", not one past it).
    let mut stack: Vec<(&Expr, usize)> = vec![(expr, 0)];
    let mut max_depth = 0;
    while let Some((node, d)) = stack.pop() {
        if d > MAX_EXPR_DEPTH {
            return None;
        }
        max_depth = max_depth.max(d);
        match node {
            Expr::SymApply { head, args, .. } => {
                stack.push((head, d + 1));
                for a in args {
                    stack.push((a, d + 1));
                }
            }
            Expr::SymPatternBlank { head: Some(h), .. } => stack.push((h, d + 1)),
            Expr::SymPatternNamed { pattern, .. } => stack.push((pattern, d + 1)),
            Expr::SymRule { lhs, rhs, .. } => {
                stack.push((lhs, d + 1));
                stack.push((rhs, d + 1));
            }
            Expr::SymReplaceAll { expr, rules, .. } => {
                stack.push((expr, d + 1));
                for r in rules {
                    stack.push((r, d + 1));
                }
            }
            _ => {}
        }
    }
    Some(max_depth)
}

/// Tear down a rejected, pathologically-deep `Expr` tree **iteratively**,
/// so freeing it can never itself overflow the stack -- unlike simply
/// letting `expr` fall out of scope, which invokes `Expr`/`Box<Expr>`'s
/// ordinary *recursive* compiler-derived `Drop` glue (`semantic_ir::Expr`
/// has no custom `Drop` impl of its own). A security review confirmed
/// (empirically, via an isolated subprocess: `compile()` called directly
/// on a bare default-stack thread with a rejected ~23,000-level-deep tree)
/// that this ordinary drop is a real, exploitable crash -- moving a
/// pathologically deep tree past [`measure_depth_iterative`]'s detection
/// only to then let it drop normally just relocates the same native stack
/// overflow from "walking the tree forward" to "walking it backward",
/// which none of the prior fixes in this file's history examined.
///
/// The technique: take ownership of `expr`, and for every node with a
/// nested `Expr` field, *move* that field out via the match (not borrow
/// it), pushing it onto our own explicit heap-allocated work stack instead
/// of leaving it in place to be dropped as part of the outer match's
/// scrutinee. Each loop iteration therefore drops only one node's own
/// non-recursive fields (strings, spans, flags) -- moving a child out via
/// `*head` / `Vec` iteration prevents the *outer* value's default drop
/// glue from ever recursing into it. This mirrors the standard
/// iterative-drop pattern for a boxed recursive structure (the same
/// technique a hand-written `impl Drop for List` uses to avoid overflowing
/// on a long linked list), generalised from a list to a tree.
fn drop_iterative(expr: Expr) {
    let mut stack: Vec<Expr> = vec![expr];
    while let Some(node) = stack.pop() {
        match node {
            Expr::SymApply { head, args, .. } => {
                stack.push(*head);
                stack.extend(args);
            }
            Expr::SymPatternBlank { head: Some(h), .. } => stack.push(*h),
            Expr::SymPatternNamed { pattern, .. } => stack.push(*pattern),
            Expr::SymRule { lhs, rhs, .. } => {
                stack.push(*lhs);
                stack.push(*rhs);
            }
            Expr::SymReplaceAll { expr, rules, .. } => {
                stack.push(*expr);
                stack.extend(rules);
            }
            _ => {}
        }
        // `node`'s own shell drops here -- shallowly, since every nested
        // `Expr` field it had was already moved out onto `stack` above.
    }
}

/// Gather the names captured by every [`Expr::SymPatternNamed`] anywhere in
/// `expr` -- used by `lower_rule` to know which RHS bare-symbol references
/// need rewriting into pattern-reference form. Recurses without its own
/// depth cap: safe because its only caller ([`Lowerer::lower_rule`])
/// verifies `expr`'s true depth via [`measure_depth_iterative`] first,
/// which is the authoritative bound -- see that function's doc comment for
/// why a purely construction-time cap (`MAX_EXPR_DEPTH`/`add_chain_depth`)
/// is not sufficient on its own.
fn collect_pattern_names(expr: &Expr, names: &mut HashSet<String>) {
    match expr {
        Expr::SymPatternNamed { name, pattern, .. } => {
            names.insert(name.clone());
            collect_pattern_names(pattern, names);
        }
        Expr::SymPatternBlank { head: Some(h), .. } => collect_pattern_names(h, names),
        Expr::SymApply { head, args, .. } => {
            collect_pattern_names(head, names);
            for a in args {
                collect_pattern_names(a, names);
            }
        }
        Expr::SymRule { lhs, rhs, .. } => {
            collect_pattern_names(lhs, names);
            collect_pattern_names(rhs, names);
        }
        Expr::SymReplaceAll { expr, rules, .. } => {
            collect_pattern_names(expr, names);
            for r in rules {
                collect_pattern_names(r, names);
            }
        }
        _ => {}
    }
}

/// Rewrite bare `SymSymbol(name)` references that are bound LHS pattern
/// names into `SymPatternNamed{name, pattern: SymPatternBlank{None}}`
/// reference nodes -- the same shape a fresh `x_` occurrence lowers to (see
/// the SIR23 spec) -- so a later matcher's substitution step fills them in.
/// Symbols not in `bound`, and all literals, pass through unchanged.
/// Recurses without its own depth cap for the same reason
/// [`collect_pattern_names`] does -- see that function's doc comment.
fn bind_pattern_refs(expr: Expr, bound: &HashSet<String>) -> Expr {
    match expr {
        Expr::SymSymbol { name, span } if bound.contains(&name) => Expr::SymPatternNamed {
            name: name.clone(),
            pattern: Box::new(Expr::SymPatternBlank {
                head: None,
                span: span.clone(),
            }),
            span,
        },
        Expr::SymApply { head, args, span } => Expr::SymApply {
            head: Box::new(bind_pattern_refs(*head, bound)),
            args: args.into_iter().map(|a| bind_pattern_refs(a, bound)).collect(),
            span,
        },
        other => other,
    }
}

enum Unwrapped<'a> {
    Node(&'a GrammarASTNode),
    Token(&'a Token),
}

impl<'a> Unwrapped<'a> {
    fn is_list(&self) -> bool {
        matches!(self, Unwrapped::Node(n) if n.rule_name == "list")
    }
}

/// Peel away single-child wrapper nodes until we reach a node with
/// structure (or a leaf token). A precedence-cascade rule that did not
/// apply its operator still emits its own node with exactly one child --
/// this skips straight to the rule that actually matters.
fn unwrap_single(mut node: &GrammarASTNode) -> Unwrapped<'_> {
    loop {
        if node.children.len() != 1 {
            return Unwrapped::Node(node);
        }
        match &node.children[0] {
            ASTNodeOrToken::Node(child) => node = child,
            ASTNodeOrToken::Token(token) => return Unwrapped::Token(token),
        }
    }
}
