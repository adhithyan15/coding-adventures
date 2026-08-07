//! The lowering pass from `coding_adventures_axiom_parser`'s generic
//! [`GrammarASTNode`] CST → [`semantic_ir::Module`], **v0.1.0** (MA-13e).
//!
//! This is the **sixth** frontend to target
//! [SIR23](../../../specs/SIR23-symbolic-pattern-semantic-ir.md), the
//! symbolic-expression/pattern-matching domain extension of the SIR10
//! narrow-waist IR (Stream B of
//! [`HML01`](../../../specs/HML01-math-to-semantic-ir.md)) — sibling to
//! `wolfram-to-semantic-ir`, `macsyma-to-semantic-ir`, `derive-to-semantic-ir`,
//! `reduce-to-semantic-ir`, and `maple-to-semantic-ir` (the closest structural
//! templates: Axiom's arithmetic/comparison/assignment/definition/if/block
//! core is exactly the same "surface operators + `head(args)` calls, no
//! pattern/rewrite-rule vocabulary" shape those three already lower, per
//! [`MA13`](../../../specs/MA13-axiom-language.md) §5). This crate's own
//! genuinely new territory — not shared with any prior SIR23 frontend — is
//! Axiom's declaration (`:`), coercion (`::`), and category-membership query
//! (`has`), fixed by MA13 §3/§4 and resolved by this crate's own central
//! design decision (below).
//!
//! # Retargeting `axiom-runtime`, not starting from scratch
//!
//! `axiom-runtime` already walks this exact CST (see that crate's
//! `src/eval.rs`) — but, unlike Derive/Reduce/Maple's own two-phase
//! "lower once, evaluate once" runtimes, it is a single-phase **tree-walking
//! interpreter** (its own module doc comment explains why: `::`/`:`/`has`
//! have no `IRNode` representation at all, so a clean lower-then-evaluate
//! split does not exist natively). This crate is nonetheless a **direct
//! structural retarget** of that same rule-name dispatch — `eval_expr`'s own
//! `match node.rule_name.as_str() { "if_expr" => ..., "declared_define" =>
//! ..., ... }` is this module's [`Lowerer::lower_node`], node-for-node, just
//! building [`semantic_ir::Expr`] data instead of evaluating. Every
//! arithmetic/comparison operator lowers to the exact same canonical head
//! name `axiom-runtime`'s own `additive_head`/`multiplicative_head`/
//! `comparison_head` tables already use (`ADD`/`SUB`/`MUL`/`DIV`/`POW`/`NEG`/
//! `EQUAL`/`NOT_EQUAL`/`LESS`/`GREATER`/`LESS_EQUAL`/`GREATER_EQUAL`) — the
//! same heads Derive/Reduce/Maple already lower to, so the shared JS backend's
//! `evalTerm` dispatch (SIR23's addendum, already shipped) folds Axiom's
//! ordinary arithmetic exactly the way it folds every sibling language's.
//!
//! # `program` is a SINGLE expression, not a repeated multi-statement worksheet
//!
//! Unlike every prior SIR23 frontend (`derive.grammar`/`reduce.grammar`/
//! `maple.grammar` each parse `program = { statement_line }`, a whole
//! worksheet file in one call), `axiom.grammar`'s own `program = expr` parses
//! **exactly one** expression per call — see that grammar file's own "WHY
//! `program` IS A SINGLE EXPRESSION" header comment: Axiom is modeled here as
//! a numbered, per-line interactive session (`axiom-repl` tracks its own step
//! counter, MA13 §5), not a batch file, and `axiom.tokens` gives top-level
//! inputs no separator at all. This crate's [`compile`]/[`compile_source`]
//! therefore lower exactly one top-level statement into `main`'s body — a
//! real, disclosed structural difference from `maple-to-semantic-ir`/
//! `reduce-to-semantic-ir`'s own multi-statement `lower_file` loops, not an
//! oversight. A caller wanting a multi-line Axiom *session* compiled as one
//! SIR module would call this crate once per line and concatenate the
//! resulting `main` bodies — a REPL-層 concern this crate does not need to
//! solve, exactly as `axiom.grammar` itself leaves it to `axiom-repl`.
//!
//! # No logical operators, no `elif` — a genuinely smaller grammar than Maple's
//!
//! `axiom.grammar` has no `logical_or`/`logical_and`/`logical_not` production
//! at all (MA13 §4's own table lists no `and`/`or`/`not` surface), and its
//! `if_expr = "if" expr "then" expr "else" expr` is a plain ternary — no
//! `elif`/`elseif` repetition (`else` is even **mandatory** in this cut, MA13
//! §4's own disclosed narrowing) — so, unlike `maple-to-semantic-ir`/
//! `macsyma-to-semantic-ir`, this crate needs no `check_elif_chain_length`-
//! style guard at all: the risk it would guard against is structurally
//! absent from this grammar, not merely bounded by a cap.
//!
//! # The central design decision: how `:` / `::` / `has` lower to SIR23
//!
//! MA13 §2's own finding is that `symbolic_ir::IRNode` — and, verified
//! directly against [SIR23](../../../specs/SIR23-symbolic-pattern-semantic-ir.md)
//! by this crate's own research, `semantic_ir::Expr`/`SirType`/`Feature` too —
//! has **no concept of a domain, a category, or a per-value type tag
//! anywhere**. SIR23 adds `SymExpr`/`Rational`/`Complex` types and
//! `SymApply`/`SymPatternBlank`/`SymPatternNamed`/`SymRule`/`SymReplaceAll`
//! node kinds — nothing resembling a coercion or a category-membership query.
//! So the question MA13 §5 explicitly left open for this item ("depends on
//! what the shared IR already has by the time the frontend starts") resolves
//! the same way MA13 §2 itself resolved the identical question for
//! `axiom-runtime`: **no core-IR change is needed or justified** for three
//! constructs used by exactly one language in this repo.
//!
//! **Decision: lower `:`, `::`, and `has` as ordinary [`Expr::SymApply`]
//! nodes with three new, locally-defined (NOT added to shared `semantic-ir`
//! or `symbolic-ir`) reserved head-name constants** — [`AXIOM_DECLARE`],
//! [`AXIOM_COERCE`], [`AXIOM_HAS`] — exactly the same "new construct, no
//! shared-crate change, a local `pub const` head name" pattern this repo's
//! own SIR23 family already established twice: `reduce-to-semantic-ir`'s
//! [`COMPOUND_EXPRESSION`]/`Cons`/`First`/… (for Reduce-specific list/block
//! surface `symbolic-vm` has no handler for) and `maple-to-semantic-ir`'s
//! `Set` (for Maple's second bracketed-aggregate literal). This crate's own
//! `;`-sequenced block (below) reuses [`COMPOUND_EXPRESSION`] — spelled
//! identically to Reduce's own constant — for the exact same reason Reduce
//! needed it: MA13 §4's parenthesised, semicolon-separated block, "value is
//! the last expression's value," is structurally identical to Reduce's
//! `<< s1; s2; ... >>`.
//!
//! Concretely:
//!
//! - **`a : T` / `(a, b, c) : T`** (declaration) lowers to
//!   `Apply(__axiom_declare, [List(SymSymbol(name)...), type_expr])` — the
//!   declared name(s) always wrapped in a `List` (even a single name), so a
//!   future runtime evaluator can iterate uniformly regardless of whether the
//!   plain-`NAME` or tuple `decl_target` form was used.
//! - **`e :: T`** (coercion) lowers to `Apply(__axiom_coerce, [e, type_expr])`
//!   — `e` is an ordinary, arbitrary expression (MA13 §3 confirms
//!   `(a + b) :: Float` is legal), so it lowers exactly like any other
//!   `additive` operand; the type-expression argument is never evaluated as
//!   an ordinary variable reference (see "Type positions" below).
//! - **`D has C`** (category query) lowers to
//!   `Apply(__axiom_has, [domain_type_expr, category_type_expr])`.
//!
//! **Why this is the correct, narrowest fix — not merely the cheapest one.**
//! Per this repo's own established discipline (see `HML01` §4's "new runtime
//! behavior ships as new npm packages, gated on the module's manifest," and
//! the SIR23 spec's own addendum, which repeatedly chooses "port the
//! reference architecture with a small, disclosed extension" over "invent a
//! new mechanism"), extending a shared, `Backend`-agnostic enum
//! (`semantic_ir::Expr`/`SirType`/`Feature`) for a construct exactly one
//! language in this repo currently uses would be the disproportionate
//! response — every existing frontend and every existing backend `match`
//! would need a new, permanently-unreachable arm. An ordinary `SymApply` with
//! a reserved head name costs nothing extra anywhere else in the pipeline: it
//! is exactly as valid, structurally-checkable SIR23 data as any other
//! `SymApply`, `semantic_ir::validate` accepts it with no special-casing (it
//! only inspects `Expr` variant shape, never head-name spelling), and
//! `semantic-ir-to-javascript`'s SIR23 codegen already handles **any**
//! `SymApply`/`SymSymbol` shape uniformly regardless of head spelling
//! (confirmed directly, the same fact `maple-to-semantic-ir`'s own `Set` and
//! `reduce-to-semantic-ir`'s own `CompoundExpression`/`Cons`/… already lean
//! on). No `Feature` flag beyond the pre-existing `Feature::SymbolicExpr` (and
//! `Feature::Floats`/`Feature::Strings` for literals) is ever observed for
//! these three constructs — they need no new capability declaration, only a
//! head name a runtime *may* one day choose to interpret specially.
//!
//! **Runtime-shim status: UPDATE (Wave 7 close-out) — now wired, in the
//! follow-on oracle-testing item this section originally deferred to.**
//! This crate itself is unchanged (it still "never evaluates anything" —
//! the same "everything is data" design every SIR23 frontend shares — so
//! it never *needed* a working evaluator to emit correct, well-formed SIR),
//! but the deferred half of the story below is no longer accurate as a
//! description of the CURRENT state of the pipeline, only of this crate's
//! own PR at the time it shipped. Verified directly against the actual
//! shipped architecture: the JS backend's real SIR23 evaluator
//! (`Symbolic.evalTerm`, `HELD_HEADS`, the whole held-form/arithmetic
//! dispatch the SIR23 spec's addendum describes) lives **inline**, inside
//! `semantic-ir-to-javascript/src/runtime.rs`'s own emitted `RUNTIME` string
//! blob — confirmed by reading that file directly. `HELD_HEADS` now also
//! lists `__axiom_declare`/`__axiom_coerce`/`__axiom_has` (alongside
//! `Assign`/`Define`/`If`), each with its own handler
//! (`axiomDeclareHandler`/`axiomCoerceHandler`/`axiomHasHandler`) — a
//! JS-side port of `axiom-runtime::domains`'s fixed
//! `AxiomDomain`/`AxiomCategory` table, described in that file's own
//! "Axiom domain/category table + reserved-head handlers" section. The
//! published npm package `@coding-adventures/sir-runtime-symbolic`
//! (`code/packages/typescript/sir-runtime-symbolic/`) still has no
//! evaluator at all and still only backs the TypeScript backend (confirmed
//! unchanged) — the JS runtime addition above did not touch it, matching
//! this crate's own original finding that extending that package would not
//! reach the path this repo's `node`-execution oracle tests actually use.
//!
//! `axiom-to-semantic-ir/tests/oracle.rs` is the real end-to-end proof:
//! the same Axiom source run through `axiom-runtime` (ground truth) and
//! through this crate → `semantic-ir-to-javascript` → `node` (compiled),
//! diffed, covering both a passing and a failing `:` declaration, a
//! passing and a failing `::` coercion, and the book's own two confirmed
//! `has` examples (`Polynomial(Integer) has Ring` → `true`,
//! `List(Integer) has Ring` → `false`) — see that file's own module doc
//! for the full corpus and any disclosed native-vs-compiled differences
//! found while building it.
//!
//! The historical record of the ORIGINAL deferral decision, kept below
//! verbatim for context (every prior "new reserved head with no runtime
//! evaluator yet" precedent in this exact family — `Set`,
//! `CompoundExpression`, `Cons`, `First`, `Second`, `Third`, `Rest`, `Part`,
//! `Append`, `Reverse` — shipped its *frontend* first and left the
//! evaluator as later, separate work; this crate followed that identical,
//! already-proven sequence): ship the lowering now (this PR), and leave
//! "teach a JS/TS runtime the fixed `AxiomDomain`/`AxiomCategory` table
//! `axiom-runtime::domains` already has" as the natural first step of the
//! follow-on oracle-testing task — which is exactly what has now happened.
//!
//! # Type positions (`type_expr`) are ordinary symbolic data too — no new
//! representation needed
//!
//! `axiom.grammar`'s own `type_expr` rule comment observes that a type
//! position (`Polynomial(Integer)`, `Fraction Integer`, `List(Float)`) is
//! structurally just "a NAME, optionally applied to further arguments" — the
//! exact same shape an ordinary function call already has. [`Lowerer::
//! lower_type_expr`] exploits this directly: a bare `type_expr` with no
//! `type_ctor_args` lowers to a plain [`Expr::SymSymbol`] (`Integer` →
//! `SymSymbol("Integer")`); a parameterized one lowers to an ordinary
//! [`Expr::SymApply`] (`Fraction(Integer)` →
//! `SymApply(SymSymbol("Fraction"), [SymSymbol("Integer")])`) — **the exact
//! same node shapes an ordinary call already produces**, just constructed by
//! a dedicated function so a `type_expr` position is never confused with an
//! ordinary value-producing `postfix` call at the Rust level (mirroring
//! `axiom-runtime::builtins::parse_type_spec`'s own identical "separate
//! function, same underlying shape" design, and `idl.grammar`'s established
//! "give a semantically distinct position its own rule name even where the
//! shape overlaps" discipline). The paren-optional shorthand
//! (`Fraction Integer`) is restricted to a single bare `NAME` argument,
//! never a further-nested `type_expr` — mirroring `axiom.grammar`'s own
//! `type_ctor_args`'s documented restriction (and `axiom-runtime::builtins::
//! parse_type_ctor_args`'s identical reading) exactly, so this crate invents
//! no syntax the grammar does not already accept.
//!
//! Because `__axiom_declare`/`__axiom_coerce`/`__axiom_has`'s type-expression
//! arguments are never meant to be evaluated as ordinary variable references
//! (a domain name like `Integer` is not a bound variable — mirroring how
//! `axiom-runtime::eval_coercion`/`eval_has_query`/`eval_declaration` never
//! evaluate a `type_expr` node as an ordinary expression either, they walk it
//! structurally via `parse_type_spec`), a future runtime evaluator for these
//! three heads would need to treat them as **held** arguments (not
//! evaluated before dispatch) exactly the way `Assign`'s left-hand-side name
//! is held today — a design note for the deferred follow-on item, not
//! something this frontend itself needs to encode (this frontend never
//! evaluates anything, so "held" vs. "evaluated" is moot at lowering time;
//! the data shape is identical either way).
//!
//! # A disclosed widening relative to `axiom-runtime`'s own function bodies
//!
//! `axiom-runtime::eval`'s own `lower_pure_body` rejects `:=`/`:`/`::`/`has`/
//! a `;`-sequenced block inside a held function body, because none of those
//! constructs have **any** representation in that crate's own reduced
//! `IRNode` value model (its own module doc comment explains this at length —
//! it is a consequence of that crate being a single-phase, eagerly-evaluating
//! interpreter needing a *representable* body to substitute-and-re-evaluate
//! at call time). This crate imposes **no equivalent restriction**: because
//! "everything is data" here (this is the same design every SIR23 frontend
//! shares — see `wolfram-to-semantic-ir::lower`'s own module doc comment for
//! why this is necessary, not just convenient, for an uncomputed function
//! body), `:=`/`:`/`::`/`has`/a block ALL already have an ordinary `SymApply`
//! representation (per the design decision above), so a function body
//! containing any of them lowers exactly the same way a top-level statement
//! would — [`Lowerer::lower_declared_define`]/[`Lowerer::
//! lower_undeclared_define`] call the SAME [`Lowerer::lower_node`] on a
//! body that a top-level statement would use, no restricted variant. This is
//! a real, disclosed WIDENING relative to `axiom-runtime`'s own current
//! scope, not a bug: the native runtime's restriction is an artifact of ITS
//! OWN two-phase-incompatible evaluation design (explained in its own module
//! doc comment), not a limitation of Axiom-the-language or of this SIR
//! target.
//!
//! # Declared function definitions: type annotations are dropped, not
//! validated, at lowering time
//!
//! `declared_define = NAME LPAREN [ typed_param_list ] RPAREN COLON type_expr
//! DEFINE expr` carries a typed parameter list AND a return-type annotation —
//! but [`Lowerer::lower_declared_define`] extracts **only** each parameter's
//! bare NAME (via [`Lowerer::lower_typed_param_list`]), dropping every
//! `type_expr` annotation entirely (including the return type, which this
//! function never even locates), producing the exact same 3-argument
//! `Define(name, List(params...), body)` shape Derive's/Reduce's/Maple's own
//! (differently-spelled) general-definition idioms already use. This is a
//! real, disclosed narrowing: `axiom-runtime::eval_declared_define` DOES
//! resolve each annotation against the fixed domain table at
//! definition-evaluation time (rejecting an invalid type name, e.g.
//! `f(x: Matrix): Integer == x`, with a `DomainError`) — but never enforces
//! it against call arguments (MA13 §4: "duck-typed... evaluated rather than
//! 'recompiled'"). Reproducing that definition-time-only check here would
//! mean either (a) duplicating `axiom-runtime::domains`'s fixed table a
//! second time in this frontend for a validation this crate's own design
//! otherwise never performs (every sibling SIR23 frontend is a **pure**
//! syntactic retarget — none of them validate semantic well-formedness at
//! lowering time), or (b) routing the annotations through the same
//! `__axiom_declare`-shaped machinery and inserting them as held statements
//! ahead of the body, which would change `Define`'s own established 3-arg
//! shape every sibling frontend relies on. Both are real scope expansions
//! beyond "lower the syntax that's there"; this crate takes the narrower,
//! more consistent path and drops the annotations, exactly mirroring how
//! every sibling `Define` never carried argument types to begin with.

use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span, Stmt,
};
use symbolic_ir::{
    ADD, ASSIGN, DEFINE, DIV, EQUAL, GREATER, GREATER_EQUAL, IF, LESS, LESS_EQUAL, LIST, MUL, NEG,
    NOT_EQUAL, POW, SUB,
};

/// `a : T` / `(a, b, c) : T` — declaration (MA13 §3/§4). See the module doc
/// comment's central design-decision section for the full reasoning. Not
/// exported by `symbolic-ir`/`semantic-ir` — defined locally, since this is
/// not a `Backend`-agnostic canonical head any other language in this repo
/// currently needs.
pub const AXIOM_DECLARE: &str = "__axiom_declare";

/// `e :: T` — coercion (MA13 §3/§4).
pub const AXIOM_COERCE: &str = "__axiom_coerce";

/// `D has C` — category-membership query (MA13 §3/§4).
pub const AXIOM_HAS: &str = "__axiom_has";

/// The canonical head for a parenthesised, `;`-separated block (MA13 §4:
/// "value is the last expression's value") — spelled identically to
/// `reduce-to-semantic-ir`'s/`reduce-runtime`'s own `COMPOUND_EXPRESSION`
/// constant (`reduce-runtime`'s `<< s1; s2; ... >>`), since Axiom's block is
/// structurally the identical construct under different surface syntax.
/// Reusing the SAME spelling (rather than inventing a new one) means a
/// future shared runtime evaluator only needs one `CompoundExpression`
/// handler to cover both languages, not two differently-named ones.
pub const COMPOUND_EXPRESSION: &str = "CompoundExpression";

/// Maximum expression-nesting depth for *this crate's own* lowering
/// recursion — distinct from (and independent of) `axiom-parser`'s own
/// `MAX_RULE_DEPTH` (140) grammar-nesting guard, which bounds the CST this
/// crate walks. Kept at 256 for consistency with every sibling SIR23
/// frontend's identically-named, identically-valued guard.
const MAX_EXPR_DEPTH: usize = 256;

/// Synthetic file name used for all spans (the CST does not carry the
/// original path).
const FILE: &str = "<axiom>";

// ---------------------------------------------------------------------------
// Public error type
// ---------------------------------------------------------------------------

/// An error encountered during Axiom → SIR lowering.
///
/// Mirrors `MapleLowerError`/`ReduceLowerError`/`DeriveLowerError`'s shape
/// exactly (`message` + 1-based `line`/`column`) so tooling can treat every
/// SIR frontend uniformly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxiomLowerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for AxiomLowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AxiomLowerError at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for AxiomLowerError {}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Lower a parsed Axiom CST (rooted at the `program` rule) into a SIR
/// module.
///
/// Unlike `maple-to-semantic-ir::compile`/`reduce-to-semantic-ir::compile`
/// (which lower a repeated `{ statement_line }` worksheet), `program` here is
/// exactly ONE expression (see the module doc comment) — the returned
/// module's `main` function body therefore always has exactly one
/// `Stmt::ExprStmt`.
///
/// This function does **not** itself guard against native stack overflow on
/// deeply-nested input beyond its own [`MAX_EXPR_DEPTH`] cap — it trusts
/// `tree` was already parsed under a suitable guard (`axiom-parser`'s own
/// `MAX_RULE_DEPTH`). See `src/lib.rs`'s `compile_source` doc comment for why
/// no worker-thread stack enlargement is needed here.
pub fn compile(tree: &GrammarASTNode, module_name: &str) -> Result<Module, AxiomLowerError> {
    Lowerer::new(module_name).lower_program(tree)
}

// ---------------------------------------------------------------------------
// The lowerer
// ---------------------------------------------------------------------------

/// The lowering pass's only mutable state: the module name (fixed at
/// construction) and the set of SIR features observed while lowering (used
/// to build the manifest so it declares *exactly* what the module emits —
/// see `semantic-ir/src/validator.rs`'s `check_expr`, the ground truth this
/// must match node-kind-for-node-kind).
///
/// Like every sibling SIR23 frontend's `Lowerer`, there is no per-function
/// name-resolution context here at all: under the "everything is data"
/// design (see the module doc comment), there are no host variables,
/// parameters, or scopes to resolve. This lowerer is a near-stateless
/// recursive descent over the CST.
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
    // top level: `program = expr ;` -- a SINGLE expression, see the module
    // doc comment's "program is a SINGLE expression" section.
    // -------------------------------------------------------------------

    fn lower_program(&mut self, program: &GrammarASTNode) -> Result<Module, AxiomLowerError> {
        if program.rule_name != "program" {
            return Err(self.err_at(
                program,
                format!("expected `program` root, got `{}`", program.rule_name),
            ));
        }

        // `lower_node` peels `program`'s (and every intermediate transparent
        // wrapper rule's) own single child away via `unwrap_single` before
        // dispatching -- safe to call directly on the root, exactly mirroring
        // `axiom-runtime::eval::eval_expr`'s own documented "safe to call on
        // any node in the tree, not just program/expr" contract.
        let expr = self.lower_node(program, 0)?;
        if measure_depth_iterative(&expr).is_none() {
            let err = self.err_at(
                program,
                format!("expression tree too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            );
            drop_iterative(expr);
            return Err(err);
        }

        let span = expr.span().clone();
        let stmts = vec![Stmt::ExprStmt {
            expr,
            span: span.clone(),
        }];

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
            .with_source_language("axiom")
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
    // Dispatch -- a direct structural retarget of
    // `axiom-runtime::eval::eval_expr`'s own rule-name `match`.
    // -------------------------------------------------------------------

    fn lower_node(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, AxiomLowerError> {
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
                "if_expr" => self.lower_if(node, depth),
                "declared_define" => self.lower_declared_define(node, depth),
                "undeclared_define" => self.lower_undeclared_define(node, depth),
                "assignment" => self.lower_assignment(node, depth),
                "declaration" => self.lower_declaration(node, depth),
                "has_query" => self.lower_has_query(node, depth),
                "comparison" => self.lower_comparison(node, depth),
                "coercion" => self.lower_coercion(node, depth),
                "additive" | "multiplicative" => self.lower_binary_chain(node, depth),
                "unary" => self.lower_unary(node, depth),
                "power" => self.lower_power(node, depth),
                "postfix" => self.lower_postfix(node, depth),
                "atom" => self.lower_atom(node, depth),
                "list_literal" => self.lower_list_literal(node, depth),
                "group" => self.lower_group(node, depth),
                "call_args" => Err(self.err_at(node, "`call_args` cannot be lowered as a standalone expression".to_string())),
                "arglist" => Err(self.err_at(node, "an arglist cannot be lowered as a scalar expression".to_string())),
                "elem_list" => Err(self.err_at(node, "an elem_list cannot be lowered as a scalar expression".to_string())),
                "type_expr" => Err(self.err_at(node, "a bare type expression is not a value-producing expression".to_string())),
                other => Err(self.err_at(node, format!("no lowering for rule `{other}`"))),
            },
        }
    }

    /// Lower a raw token (a literal or a bare symbol). Mirrors
    /// `axiom-runtime::eval::eval_token`'s own token-type dispatch.
    fn lower_token(&mut self, token: &Token) -> Result<Expr, AxiomLowerError> {
        let span = self.token_span(token);
        match token_type(token) {
            "NUMBER" => Ok(self.number_literal_expr(&token.value, span)),
            "STRING" => Ok(self.str_lit(token.value.clone(), span)),
            "NAME" => Ok(self.sym_symbol(token.value.clone(), span)),
            other => Err(AxiomLowerError {
                message: format!("unexpected token `{other}` = {:?}", token.value),
                line: token.line,
                column: token.column,
            }),
        }
    }

    /// `if_expr = "if" expr "then" expr "else" expr` — a plain ternary,
    /// `else` mandatory in this cut (MA13 §4). No `elif` repetition exists in
    /// this grammar (see the module doc comment), so there is no
    /// right-fold and no analogous chain-length guard to write.
    fn lower_if(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, AxiomLowerError> {
        let exprs: Vec<&GrammarASTNode> = child_nodes(node).filter(|n| n.rule_name == "expr").collect();
        if exprs.len() != 3 {
            return Err(self.err_at(node, "malformed `if` node".to_string()));
        }
        let cond = self.lower_node(exprs[0], depth + 1)?;
        let then_branch = self.lower_node(exprs[1], depth + 1)?;
        let else_branch = self.lower_node(exprs[2], depth + 1)?;
        let span = self.span_of(node);
        Ok(self.sym_apply(
            self.sym_symbol_bare(IF, span.clone()),
            vec![cond, then_branch, else_branch],
            span,
        ))
    }

    // -------------------------------------------------------------------
    // Function definition -- `==`, held-body (MA13 §4). See the module doc
    // comment's own sections on the type-annotation-dropping and the
    // function-body-widening design decisions.
    // -------------------------------------------------------------------

    /// `declared_define = NAME LPAREN [ typed_param_list ] RPAREN COLON
    /// type_expr DEFINE expr`.
    fn lower_declared_define(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, AxiomLowerError> {
        let name = first_token_value(node, "NAME")
            .ok_or_else(|| self.err_at(node, "malformed function definition: missing name".to_string()))?;

        let params = match child_nodes(node).find(|n| n.rule_name == "typed_param_list") {
            Some(list_node) => self.lower_typed_param_list(list_node)?,
            None => vec![],
        };

        // The return-type annotation (a direct `type_expr` child) is
        // deliberately never located or inspected -- see the module doc
        // comment's "type annotations are dropped, not validated" section.
        let body_node = child_nodes(node)
            .find(|n| n.rule_name == "expr")
            .ok_or_else(|| self.err_at(node, "malformed function definition: missing body".to_string()))?;
        let body = self.lower_node(body_node, depth + 1)?;

        let span = self.span_of(node);
        let params_list = self.sym_apply(self.sym_symbol_bare(LIST, span.clone()), params, span.clone());
        Ok(self.sym_apply(
            self.sym_symbol_bare(DEFINE, span.clone()),
            vec![self.sym_symbol_bare(name, span.clone()), params_list, body],
            span,
        ))
    }

    /// `typed_param_list = typed_param { COMMA typed_param } ; typed_param =
    /// NAME COLON type_expr`. Extracts only each parameter's bare NAME,
    /// dropping its `type_expr` annotation entirely (see the module doc
    /// comment). A flat `Vec`, not a folded tree, so
    /// [`Self::check_apply_arg_count`] bounds it as an allocation-size
    /// backstop only.
    fn lower_typed_param_list(&mut self, node: &GrammarASTNode) -> Result<Vec<Expr>, AxiomLowerError> {
        let params: Vec<&GrammarASTNode> = child_nodes(node).filter(|n| n.rule_name == "typed_param").collect();
        self.check_apply_arg_count(node, params.len())?;
        let mut result = Vec::with_capacity(params.len());
        for p in params {
            let name = first_token_value(p, "NAME")
                .ok_or_else(|| self.err_at(p, "malformed parameter: missing name".to_string()))?;
            let span = self.span_of(p);
            result.push(self.sym_symbol(name, span));
        }
        Ok(result)
    }

    /// `undeclared_define = NAME NAME DEFINE expr` — the paren-optional
    /// single-parameter, duck-typed form (MA13 §4: `f x == x * x`).
    fn lower_undeclared_define(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, AxiomLowerError> {
        let names: Vec<String> = node
            .children
            .iter()
            .filter_map(as_token)
            .filter(|t| token_type(t) == "NAME")
            .map(|t| t.value.clone())
            .collect();
        let [name, param] = names.as_slice() else {
            return Err(self.err_at(node, "malformed undeclared function definition".to_string()));
        };
        let body_node = child_nodes(node)
            .find(|n| n.rule_name == "expr")
            .ok_or_else(|| self.err_at(node, "malformed function definition: missing body".to_string()))?;
        let body = self.lower_node(body_node, depth + 1)?;

        let span = self.span_of(node);
        let param_expr = self.sym_symbol(param.clone(), span.clone());
        let params_list = self.sym_apply(self.sym_symbol_bare(LIST, span.clone()), vec![param_expr], span.clone());
        Ok(self.sym_apply(
            self.sym_symbol_bare(DEFINE, span.clone()),
            vec![self.sym_symbol_bare(name.clone(), span.clone()), params_list, body],
            span,
        ))
    }

    // -------------------------------------------------------------------
    // x := e -- immediate assignment (MA13 §4)
    // -------------------------------------------------------------------

    fn lower_assignment(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, AxiomLowerError> {
        let name = first_token_value(node, "NAME")
            .ok_or_else(|| self.err_at(node, "malformed assignment: missing name".to_string()))?;
        let rhs_node = child_nodes(node)
            .find(|n| n.rule_name == "expr")
            .ok_or_else(|| self.err_at(node, "malformed assignment: missing right-hand side".to_string()))?;
        let rhs = self.lower_node(rhs_node, depth + 1)?;
        let span = self.span_of(node);
        Ok(self.sym_apply(
            self.sym_symbol_bare(ASSIGN, span.clone()),
            vec![self.sym_symbol_bare(name, span.clone()), rhs],
            span,
        ))
    }

    // -------------------------------------------------------------------
    // a : T / (a, b, c) : T -- declaration (MA13 §3/§4). See the module doc
    // comment's central design-decision section.
    // -------------------------------------------------------------------

    fn lower_declaration(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, AxiomLowerError> {
        let decl_target = child_nodes(node)
            .find(|n| n.rule_name == "decl_target")
            .ok_or_else(|| self.err_at(node, "malformed declaration: missing target".to_string()))?;
        let type_expr_node = child_nodes(node)
            .find(|n| n.rule_name == "type_expr")
            .ok_or_else(|| self.err_at(node, "malformed declaration: missing type".to_string()))?;

        let names = decl_target_names(decl_target);
        self.check_apply_arg_count(decl_target, names.len())?;

        let span = self.span_of(node);
        let name_exprs: Vec<Expr> = names.into_iter().map(|n| self.sym_symbol(n, span.clone())).collect();
        let names_list = self.sym_apply(self.sym_symbol_bare(LIST, span.clone()), name_exprs, span.clone());
        let target_type = self.lower_type_expr(type_expr_node, depth + 1)?;

        Ok(self.sym_apply(
            self.sym_symbol_bare(AXIOM_DECLARE, span.clone()),
            vec![names_list, target_type],
            span,
        ))
    }

    // -------------------------------------------------------------------
    // D has C -- category-membership query (MA13 §3/§4).
    // -------------------------------------------------------------------

    fn lower_has_query(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, AxiomLowerError> {
        let type_exprs: Vec<&GrammarASTNode> = child_nodes(node).filter(|n| n.rule_name == "type_expr").collect();
        if type_exprs.len() != 2 {
            return Err(self.err_at(node, "malformed `has` query".to_string()));
        }
        let domain = self.lower_type_expr(type_exprs[0], depth + 1)?;
        let category = self.lower_type_expr(type_exprs[1], depth + 1)?;
        let span = self.span_of(node);
        Ok(self.sym_apply(self.sym_symbol_bare(AXIOM_HAS, span.clone()), vec![domain, category], span))
    }

    // -------------------------------------------------------------------
    // comparison = coercion [ (EQ|NE|LE|LESS|GREATER|GE) coercion ]
    // -------------------------------------------------------------------

    /// Non-chaining (one optional suffix), mirroring `axiom-runtime::
    /// eval::eval_comparison`'s identical shape and comparison-head table.
    fn lower_comparison(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, AxiomLowerError> {
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

    // -------------------------------------------------------------------
    // coercion = additive [ COERCE type_expr ]
    // -------------------------------------------------------------------

    fn lower_coercion(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, AxiomLowerError> {
        let Some(additive_node) = child_nodes(node).find(|n| n.rule_name == "additive") else {
            return self.lower_first_node(node, depth);
        };
        let type_expr_node = child_nodes(node)
            .find(|n| n.rule_name == "type_expr")
            .ok_or_else(|| self.err_at(node, "malformed coercion: missing target type".to_string()))?;

        let lhs = self.lower_node(additive_node, depth + 1)?;
        let target = self.lower_type_expr(type_expr_node, depth + 1)?;
        let span = self.span_of(node);
        Ok(self.sym_apply(self.sym_symbol_bare(AXIOM_COERCE, span.clone()), vec![lhs, target], span))
    }

    // -------------------------------------------------------------------
    // additive / multiplicative -- iterative left-associative fold
    // -------------------------------------------------------------------

    /// Mirrors `axiom-runtime::eval::eval_binary_chain`'s identical fold,
    /// building `Expr` data instead of evaluating. [`Self::
    /// check_chain_length`] guards the fold (a flat EBNF repetition folds
    /// into an N-deep binary tree).
    fn lower_binary_chain(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, AxiomLowerError> {
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

    // -------------------------------------------------------------------
    // unary = MINUS unary | power
    // -------------------------------------------------------------------

    fn lower_unary(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, AxiomLowerError> {
        match node.children.len() {
            1 => self.lower_child(&node.children[0], depth + 1),
            2 => {
                let operand = self.lower_child(&node.children[1], depth + 1)?;
                let span = self.span_of(node);
                Ok(self.sym_apply(self.sym_symbol_bare(NEG, span.clone()), vec![operand], span))
            }
            _ => Err(self.err_at(node, "malformed unary node".to_string())),
        }
    }

    // -------------------------------------------------------------------
    // power = postfix [ (CARET|POW) unary ]
    // -------------------------------------------------------------------

    /// `^` and `**` are the SAME operator (MA13 §4), both collapsed onto
    /// `POW` here, mirroring `axiom-runtime::eval::eval_power`'s identical
    /// acceptance of either token type.
    fn lower_power(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, AxiomLowerError> {
        match node.children.len() {
            1 => self.lower_child(&node.children[0], depth + 1),
            3 => {
                let is_pow_op = as_token(&node.children[1]).is_some_and(|t| matches!(token_type(t), "CARET" | "POW"));
                if !is_pow_op {
                    return Err(self.err_at(node, "malformed power node: expected CARET or POW".to_string()));
                }
                let lhs = self.lower_child(&node.children[0], depth + 1)?;
                let rhs = self.lower_child(&node.children[2], depth + 1)?;
                let span = self.span_of(node);
                Ok(self.sym_apply(self.sym_symbol_bare(POW, span.clone()), vec![lhs, rhs], span))
            }
            _ => Err(self.err_at(node, "malformed power node".to_string())),
        }
    }

    // -------------------------------------------------------------------
    // postfix = atom [ call_args ] -- function application, `f(a,b)` AND
    // paren-optional `f a` (MA13 §4)
    // -------------------------------------------------------------------

    /// Unlike Reduce's/Derive's own `postfix` (a REPEATED call suffix),
    /// `axiom.grammar`'s own `postfix = atom [ call_args ]` allows at most
    /// ONE call suffix (`f(x)(y)` is not valid Axiom syntax in this subset) —
    /// so, mirroring `maple-to-semantic-ir`'s identical finding, there is no
    /// `check_postfix_chain_length`-equivalent guard anywhere in this
    /// function: the axis it would guard is structurally impossible here.
    ///
    /// Unlike `maple-to-semantic-ir`/`reduce-to-semantic-ir`, this crate
    /// bridges no surface builtin-call name to a canonical head at all — MA13
    /// §4's scope table names no calculus/list-accessor surface bridge for
    /// Axiom (unlike Maple's `diff`/`int` or Reduce's `first`/`append`), so
    /// every call's head lowers exactly as written, whatever it is.
    fn lower_postfix(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, AxiomLowerError> {
        let atom_node = child_nodes(node)
            .next()
            .ok_or_else(|| self.err_at(node, "postfix has no base".to_string()))?;
        let base = self.lower_node(atom_node, depth + 1)?;

        let Some(call_args_node) = child_nodes(node).find(|n| n.rule_name == "call_args") else {
            return Ok(base);
        };

        let arg_nodes = call_args_exprs(call_args_node);
        self.check_apply_arg_count(call_args_node, arg_nodes.len())?;
        let mut args = Vec::with_capacity(arg_nodes.len());
        for a in arg_nodes {
            args.push(self.lower_node(a, depth + 1)?);
        }
        let span = self.span_of(node);
        Ok(self.sym_apply(base, args, span))
    }

    // -------------------------------------------------------------------
    // atom = NUMBER | STRING | NAME | list_literal | group
    // -------------------------------------------------------------------

    /// In practice [`unwrap_single`] already dissolves a single-child `atom`
    /// node before `lower_node`'s dispatch ever sees rule_name `"atom"`
    /// (every alternative here matches to exactly one child) — this function
    /// mirrors `axiom-runtime::eval`'s identical defensive shape rather than
    /// being load-bearing for the common case.
    fn lower_atom(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, AxiomLowerError> {
        if let Some(child) = child_nodes(node).next() {
            if matches!(child.rule_name.as_str(), "list_literal" | "group") {
                return self.lower_node(child, depth + 1);
            }
        }
        let tokens: Vec<&Token> = node.children.iter().filter_map(as_token).collect();
        match tokens.as_slice() {
            [single] => self.lower_token(single),
            _ => Err(self.err_at(
                node,
                format!(
                    "unrecognised atom token shape: {:?}",
                    tokens.iter().map(|t| &t.value).collect::<Vec<_>>()
                ),
            )),
        }
    }

    // -------------------------------------------------------------------
    // list_literal = LBRACKET [ elem_list ] RBRACKET
    // -------------------------------------------------------------------

    /// `[a, b, c]` (MA13 §4) — lowers to the shared, already-handled `List`
    /// head every CAS-family sibling in this repo reuses.
    fn lower_list_literal(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, AxiomLowerError> {
        let args = match child_nodes(node).find(|n| n.rule_name == "elem_list") {
            Some(elem_list) => {
                let elems: Vec<&GrammarASTNode> = child_nodes(elem_list).filter(|n| n.rule_name == "expr").collect();
                self.check_apply_arg_count(elem_list, elems.len())?;
                elems
                    .into_iter()
                    .map(|e| self.lower_node(e, depth + 1))
                    .collect::<Result<Vec<_>, _>>()?
            }
            None => vec![],
        };
        let span = self.span_of(node);
        Ok(self.sym_apply(self.sym_symbol_bare(LIST, span.clone()), args, span))
    }

    // -------------------------------------------------------------------
    // group = LPAREN expr { SEMI expr } RPAREN -- grouping OR a `;`-block
    // -------------------------------------------------------------------

    /// A `group` with exactly ONE `expr` child is ordinary grouping (returns
    /// the inner expression unchanged, no wrapper node at all); a `group`
    /// with TWO OR MORE `expr` children (joined by `;`) is a block, lowered
    /// to [`COMPOUND_EXPRESSION`] (MA13 §4: "value is the last expression's
    /// value" — matching `reduce-to-semantic-ir`'s identical
    /// `CompoundExpression[s1, s2, ...]` shape). `axiom.grammar`'s own
    /// `{ SEMI expr }` repetition is a flat EBNF shape (zero native parser
    /// stack cost regardless of width, the same established fact every
    /// sibling frontend's own chain-length guards rely on) and
    /// [`Self::sym_apply`] builds one FLAT n-ary apply here, not a folded
    /// pairwise tree — so [`Self::check_apply_arg_count`] (an
    /// allocation-size backstop) is the correct guard, not a chain-length
    /// one.
    fn lower_group(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, AxiomLowerError> {
        let exprs: Vec<&GrammarASTNode> = child_nodes(node).filter(|n| n.rule_name == "expr").collect();
        if exprs.is_empty() {
            return Err(self.err_at(node, "empty group `( )`".to_string()));
        }
        self.check_apply_arg_count(node, exprs.len())?;
        if exprs.len() == 1 {
            return self.lower_node(exprs[0], depth + 1);
        }
        let lowered = exprs
            .into_iter()
            .map(|e| self.lower_node(e, depth + 1))
            .collect::<Result<Vec<_>, _>>()?;
        let span = self.span_of(node);
        Ok(self.sym_apply(self.sym_symbol_bare(COMPOUND_EXPRESSION, span.clone()), lowered, span))
    }

    // -------------------------------------------------------------------
    // type_expr = NAME [ type_ctor_args ] -- domain/category-shaped
    // expressions, used by declaration/coercion/has_query/function-header
    // type-annotation positions. See the module doc comment's "Type
    // positions" section.
    // -------------------------------------------------------------------

    fn lower_type_expr(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, AxiomLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("type expression nesting too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            ));
        }
        let name = node
            .children
            .iter()
            .find_map(as_token)
            .filter(|t| token_type(t) == "NAME")
            .map(|t| t.value.clone())
            .ok_or_else(|| self.err_at(node, "malformed type expression: missing name".to_string()))?;
        let span = self.span_of(node);

        let ctor_args_node = node
            .children
            .iter()
            .find_map(as_node)
            .filter(|n| n.rule_name == "type_ctor_args");
        let Some(ctor_args_node) = ctor_args_node else {
            return Ok(self.sym_symbol(name, span));
        };

        let args = self.lower_type_ctor_args(ctor_args_node, depth + 1)?;
        Ok(self.sym_apply(self.sym_symbol_bare(name, span.clone()), args, span))
    }

    /// `type_ctor_args = LPAREN [ type_expr_list ] RPAREN | NAME` — the
    /// second alternative is the paren-optional shorthand (`Fraction
    /// Integer`), restricted by the grammar to a single bare NAME argument
    /// only, never a further-nested `type_expr` (see the module doc
    /// comment).
    fn lower_type_ctor_args(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Vec<Expr>, AxiomLowerError> {
        let has_lparen = node
            .children
            .iter()
            .any(|c| as_token(c).is_some_and(|t| token_type(t) == "LPAREN"));

        if !has_lparen {
            let name = node
                .children
                .iter()
                .find_map(as_token)
                .filter(|t| token_type(t) == "NAME")
                .map(|t| t.value.clone())
                .ok_or_else(|| self.err_at(node, "malformed paren-optional type argument".to_string()))?;
            let span = self.span_of(node);
            return Ok(vec![self.sym_symbol(name, span)]);
        }

        match child_nodes(node).find(|n| n.rule_name == "type_expr_list") {
            Some(list_node) => {
                let items: Vec<&GrammarASTNode> = child_nodes(list_node).filter(|n| n.rule_name == "type_expr").collect();
                self.check_apply_arg_count(list_node, items.len())?;
                items.into_iter().map(|te| self.lower_type_expr(te, depth + 1)).collect()
            }
            None => Ok(vec![]),
        }
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

    /// Build a `SymSymbol` for a *head* name, or any other
    /// internally-constructed symbol that is always immediately wrapped in a
    /// [`Self::sym_apply`] call — which itself observes the feature — so
    /// this helper does not need to (identical shape to [`Self::
    /// sym_symbol`], named separately only so call sites make their intent
    /// legible; mirrors every sibling SIR23 frontend's identically-named
    /// helper).
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

    /// `"hello"` (MA13 §4) — domain `String`. The FIRST SIR23 frontend in
    /// this repo's CAS family to construct `Expr::StrLit` at all (Derive's/
    /// Reduce's/Maple's own grammars have no `STRING` token) — `StrLit`
    /// requires `Feature::Strings` per `semantic-ir/src/validator.rs`'s own
    /// `check_expr`, observed here immediately for the same reason
    /// [`Self::number_literal_expr`] observes `Feature::Floats` immediately:
    /// a free function with no access to `self.observed` was a confirmed,
    /// previously-shipped bug class in `matlab-to-semantic-ir`/
    /// `wolfram-to-semantic-ir`.
    fn str_lit(&mut self, value: String, span: Span) -> Expr {
        self.observed.add(Feature::Strings);
        Expr::StrLit { value, span }
    }

    /// Parse a `NUMBER` lexeme into an `IntLit` or `FloatLit` (a `.`, `e`, or
    /// `E` means a real; otherwise an integer), matching
    /// `axiom-runtime::eval::parse_number`'s identical rule. An integer
    /// lexeme too large for `i64` falls back to a float rather than silently
    /// truncating.
    ///
    /// **Must** be an instance method, not a free function — see
    /// [`Self::str_lit`]'s doc comment for why.
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

    // -------------------------------------------------------------------
    // Guards
    // -------------------------------------------------------------------

    /// Reject a same-precedence operator chain (`additive`/`multiplicative`)
    /// with more than `MAX_EXPR_DEPTH` operands. `axiom.grammar`, like every
    /// sibling CAS-family grammar in this repo, collapses a flat run of
    /// same-precedence operators into ONE CST node with many children rather
    /// than nesting through parens, so a long unparenthesized chain never
    /// trips `axiom-parser`'s own `MAX_RULE_DEPTH` (which counts *nesting*,
    /// not the length of one flat repetition) — but folding N operands
    /// left-associatively still builds an N-deep binary `Expr` tree.
    fn check_chain_length(&self, node: &GrammarASTNode) -> Result<(), AxiomLowerError> {
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

    /// Cap the argument count of a single `f(…)` application, the element
    /// count of a `[…]` list literal, a `typed_param_list`/`name_list`/
    /// `type_expr_list`'s element count, or a `;`-block's statement count.
    /// None of these fold into a nested tree (all stay a flat `Vec<Expr>`),
    /// so this is not a stack-recursion guard — it is a modest
    /// defense-in-depth cap on a single allocation's size, using the same
    /// `MAX_EXPR_DEPTH` bound for consistency rather than inventing new
    /// constants per call site (mirrors `reduce-to-semantic-ir`'s/
    /// `maple-to-semantic-ir`'s identical reuse across multiple flat-`Vec`
    /// productions).
    fn check_apply_arg_count(&self, node: &GrammarASTNode, count: usize) -> Result<(), AxiomLowerError> {
        if count > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("too many arguments ({count}, exceeds {MAX_EXPR_DEPTH})"),
            ));
        }
        Ok(())
    }

    // -------------------------------------------------------------------
    // Small helpers
    // -------------------------------------------------------------------

    fn lower_first_node(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, AxiomLowerError> {
        let child = child_nodes(node)
            .next()
            .ok_or_else(|| self.err_at(node, format!("`{}` has no expression child", node.rule_name)))?;
        self.lower_node(child, depth + 1)
    }

    fn lower_child(&mut self, child: &ASTNodeOrToken, depth: usize) -> Result<Expr, AxiomLowerError> {
        match child {
            ASTNodeOrToken::Node(node) => self.lower_node(node, depth),
            ASTNodeOrToken::Token(token) => self.lower_token(token),
        }
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

    fn err_at(&self, node: &GrammarASTNode, message: String) -> AxiomLowerError {
        AxiomLowerError {
            message,
            line: node.start_line.unwrap_or(1),
            column: node.start_column.unwrap_or(1),
        }
    }
}

// ---------------------------------------------------------------------------
// Free helpers (no `&mut self` needed)
// ---------------------------------------------------------------------------

/// Collect the *node* children of `node` (dropping tokens).
fn child_nodes(node: &GrammarASTNode) -> impl Iterator<Item = &GrammarASTNode> {
    node.children.iter().filter_map(as_node)
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

/// Find the first direct `NAME` token among `node`'s IMMEDIATE children only
/// (never recursing into child *nodes*) — used for a rule whose grammar
/// guarantees its own name token is a direct child (`declared_define`,
/// `assignment`, `typed_param`). Mirrors `axiom-runtime::eval::
/// first_token_value` exactly.
fn first_token_value(node: &GrammarASTNode, token_ty: &str) -> Option<String> {
    node.children
        .iter()
        .filter_map(as_token)
        .find(|t| token_type(t) == token_ty)
        .map(|t| t.value.clone())
}

/// `decl_target = NAME | LPAREN name_list RPAREN ; name_list = NAME { COMMA
/// NAME }`. Mirrors `axiom-runtime::eval::decl_target_names` exactly.
fn decl_target_names(node: &GrammarASTNode) -> Vec<String> {
    if let Some(name_list) = child_nodes(node).find(|n| n.rule_name == "name_list") {
        return name_list
            .children
            .iter()
            .filter_map(as_token)
            .filter(|t| token_type(t) == "NAME")
            .map(|t| t.value.clone())
            .collect();
    }
    node.children
        .iter()
        .filter_map(as_token)
        .filter(|t| token_type(t) == "NAME")
        .map(|t| t.value.clone())
        .collect()
}

/// Map an arithmetic token type to its canonical IR head — the exact heads
/// `symbolic_vm::handlers::build_handler_table` wires and `axiom-runtime`
/// itself already uses (`additive_head`/`multiplicative_head`, combined
/// here since both tables are disjoint on token type).
fn binary_head(token_type: &str) -> Option<&'static str> {
    match token_type {
        "PLUS" => Some(ADD),
        "MINUS" => Some(SUB),
        "TIMES" => Some(MUL),
        "SLASH" => Some(DIV),
        _ => None,
    }
}

/// Map a comparison token to its canonical IR head — mirrors
/// `axiom-runtime::eval::comparison_head`'s identical table exactly (`~=` is
/// tokenized as `NE`, not Maple's `NEQ`/Reduce's `neq` keyword — MA13 §4's
/// own confirmed not-equal spelling).
fn comparison_head(token_type: &str) -> Option<&'static str> {
    match token_type {
        "EQ" => Some(EQUAL),
        "NE" => Some(NOT_EQUAL),
        "LE" => Some(LESS_EQUAL),
        "LESS" => Some(LESS),
        "GREATER" => Some(GREATER),
        "GE" => Some(GREATER_EQUAL),
        _ => None,
    }
}

/// `call_args = LPAREN [ arglist ] RPAREN | atom` — returns the argument
/// expression nodes to lower, uniformly for both the explicit-parens form
/// and the paren-optional single-bare-atom form (`f a`). Mirrors
/// `axiom-runtime::eval::call_args_exprs` exactly.
fn call_args_exprs(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    if let Some(arglist) = child_nodes(node).find(|n| n.rule_name == "arglist") {
        return child_nodes(arglist).filter(|n| n.rule_name == "expr").collect();
    }
    let has_lparen = node
        .children
        .iter()
        .any(|c| as_token(c).is_some_and(|t| token_type(t) == "LPAREN"));
    if has_lparen {
        Vec::new() // f() -- explicit, empty argument list
    } else {
        // f a -- the paren-optional single bare atom IS the one argument.
        child_nodes(node).filter(|n| n.rule_name == "atom").collect()
    }
}

/// Measure `expr`'s true tree depth **iteratively**, using an explicit
/// heap-allocated work stack rather than native recursion, so calling this
/// can never itself overflow the stack no matter how deep `expr` already is.
///
/// Returns `None` as soon as the depth is certain to exceed
/// `MAX_EXPR_DEPTH`, `Some(depth)` otherwise.
///
/// Only needs a match arm for [`Expr::SymApply`] (recursing into `head` and
/// `args`) — every other `Expr` variant is a leaf for this crate's purposes,
/// since this crate never constructs a `SymPatternBlank`/`SymPatternNamed`/
/// `SymRule`/`SymReplaceAll` node (Axiom's grammar has no pattern-matching or
/// rewrite-rule syntax at all). `If`/`Assign`/`Define`/`__axiom_declare`/
/// `__axiom_coerce`/`__axiom_has`/`CompoundExpression` are all `SymApply`
/// with a different head symbol, not new `Expr` variants, so this one match
/// arm already covers them.
///
/// Called once per top-level statement in [`Lowerer::lower_program`], so no
/// tree this crate hands to a caller can ever actually exceed
/// `MAX_EXPR_DEPTH`, regardless of how its construction was composed —
/// mirrors every sibling SIR23 frontend's identical authoritative,
/// construction-composition-independent depth check.
fn measure_depth_iterative(expr: &Expr) -> Option<usize> {
    let mut stack: Vec<(&Expr, usize)> = vec![(expr, 0)];
    let mut max_depth = 0;
    while let Some((node, d)) = stack.pop() {
        if d > MAX_EXPR_DEPTH {
            return None;
        }
        max_depth = max_depth.max(d);
        if let Expr::SymApply { head, args, .. } = node {
            stack.push((head, d + 1));
            for a in args {
                stack.push((a, d + 1));
            }
        }
    }
    Some(max_depth)
}

/// Tear down a rejected, pathologically-deep `Expr` tree **iteratively**, so
/// freeing it can never itself overflow the stack — see
/// `wolfram-to-semantic-ir`'s security-review history (referenced by every
/// sibling SIR23 frontend) for why letting a detected-oversized tree simply
/// fall out of scope (recursive `Drop` glue) just relocates the same native
/// stack overflow from "walking forward" to "walking backward".
fn drop_iterative(expr: Expr) {
    let mut stack: Vec<Expr> = vec![expr];
    while let Some(node) = stack.pop() {
        if let Expr::SymApply { head, args, .. } = node {
            stack.push(*head);
            stack.extend(args);
        }
    }
}

enum Unwrapped<'a> {
    Node(&'a GrammarASTNode),
    Token(&'a Token),
}

/// Peel away single-child wrapper nodes until we reach a node with structure
/// (or a leaf token). A precedence-cascade rule that did not apply its own
/// operator still emits its own node with exactly one child — this skips
/// straight to the rule that actually matters. Mirrors `axiom-runtime::
/// eval::unwrap_single`/every sibling SIR23 frontend's identically-named
/// helper exactly — the shared `parser::GrammarParser` engine's node shape
/// is identical across every grammar built on it.
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
