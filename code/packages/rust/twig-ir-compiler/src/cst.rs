//! Borrowed views over Twig's parse tree (LANG84 decision 6).
//!
//! The IIR compiler's only input is the generic [`GrammarASTNode`] tree
//! that `GrammarParser` builds from `code/grammars/twig/twig.grammar`.
//! This module is how the compiler *reads* that tree.  Nothing here owns
//! or builds a tree: every type is a `Copy` handle holding a `&'a`
//! reference into the CST, and every method looks at **one node's
//! children** and answers one question about it.
//!
//! ## Why views, and not a typed AST?
//!
//! LANG84's rule is *"the IIR compiler walks the parse tree"*: no
//! hand-written code may convert the CST into an owned, language-specific
//! AST before IIR compilation.  `twig_parser::Program` is exactly such an
//! AST (built by `twig_parser::ast_extract`), so up to v0.45 this crate
//! broke the rule.  The views below are the borrowed alternative — in the
//! style of rust-analyzer's typed nodes over a lossless syntax tree.  A
//! view never outlives the CST, never copies a subtree, and asking for a
//! child returns another unclassified handle; the child is only looked at
//! when the compiler itself descends into it.
//!
//! ## The grammar, as the compiler sees it
//!
//! Every non-terminal produces a node whose `rule_name` is the rule; tokens
//! are leaves.  The rules the compiler cares about (punctuation omitted):
//!
//! | rule                | children the views read                                        |
//! |---------------------|----------------------------------------------------------------|
//! | `program`           | optional `module_form` node, then `form` nodes                 |
//! | `module_form`       | `NAME` (module name), `module_clause` nodes                    |
//! | `module_clause`     | one `export_clause` / `import_clause` / `typed_clause` node    |
//! | `form`              | one `define` / `type_alias` / `record_def` / `union_def` / `expr` node |
//! | `define`            | `name_or_signature` node, then ≥ 1 `expr` nodes                |
//! | `name_or_signature` | `NAME [: type_annotation]` **or** `( NAME typed_param* [-> type_annotation] )` |
//! | `typed_param`       | `NAME` **or** `( NAME : type_annotation )`                     |
//! | `expr`              | one `atom` / `quoted` / `compound` node                        |
//! | `atom`              | one token: `INTEGER`, `STRING`, `BOOL_TRUE`, `BOOL_FALSE`, `KEYWORD "nil"`, `NAME` |
//! | `quoted`            | `'` then `NAME`                                                |
//! | `compound`          | one `if_form` / `let_form` / `let_star_form` / `begin_form` / `lambda_form` / `quote_form` / `match_form` / `apply` node |
//! | `if_form`           | three `expr` nodes                                             |
//! | `let_form`, `let_star_form` | `binding` nodes, then ≥ 1 `expr` nodes                 |
//! | `binding`           | `NAME`, one `expr` node                                        |
//! | `begin_form`        | ≥ 1 `expr` nodes                                               |
//! | `lambda_form`       | `NAME` params, then ≥ 1 `expr` nodes                           |
//! | `quote_form`        | `NAME`                                                         |
//! | `match_form`        | one `expr` node (the scrutinee), then ≥ 1 `match_arm` nodes    |
//! | `match_arm`         | one `match_pat` node, then ≥ 1 `expr` nodes                    |
//! | `match_pat`         | `NAME` **or** `( NAME NAME* )`                                 |
//! | `apply`             | ≥ 1 `expr` nodes: the function position, then the arguments    |
//!
//! A small example — `(define (sq x) (* x x))` — drawn in full (nodes in
//! lower case, tokens as `TYPE "text"`):
//!
//! ```text
//! program
//! └─ form
//!    └─ define
//!       ├─ LPAREN "("   KEYWORD "define"
//!       ├─ name_or_signature
//!       │  ├─ LPAREN "("   NAME "sq"
//!       │  ├─ typed_param ─ NAME "x"
//!       │  └─ RPAREN ")"
//!       ├─ expr
//!       │  └─ compound
//!       │     └─ apply
//!       │        ├─ LPAREN "("
//!       │        ├─ expr ─ atom ─ NAME "*"
//!       │        ├─ expr ─ atom ─ NAME "x"
//!       │        ├─ expr ─ atom ─ NAME "x"
//!       │        └─ RPAREN ")"
//!       └─ RPAREN ")"
//! ```
//!
//! ## Sugar the views expand on the fly
//!
//! Two pieces of surface syntax are only abbreviations, and the compiler
//! treats each pair the same way without building anything:
//!
//! * `'foo`, `(quote foo)` — both read as [`ExprKind::SymLit`].
//! * `(define (f x) body…)`, `(define f (lambda (x) body…))` — both read
//!   as a define whose right-hand side is a [`LambdaView`]: the former as
//!   [`LambdaView::Sugar`] (params and annotations come from the
//!   signature, the body from the `define` node), the latter as
//!   [`LambdaView::Anon`].
//!
//! ## Validation happens first
//!
//! Views assume the tree has passed [`crate::check_cst::check_cst`], which
//! rejects exactly what the old typed-AST extractor rejected (bad integer
//! literals, malformed nodes, nesting deeper than
//! [`twig_parser::MAX_AST_DEPTH`], …).  After that pass every shape the
//! views rely on is guaranteed, so the views are infallible.  Where a
//! shape is nonetheless missing (a caller skipped validation), a view
//! degrades to a harmless default instead of panicking.
//!
//! ## Source positions
//!
//! IIR source maps must stay byte-identical to the old pipeline, so each
//! view reports the same `(line, column)` the old typed AST stored:
//!
//! | construct                         | position taken from                         |
//! |-----------------------------------|---------------------------------------------|
//! | literal / name atom               | the atom's token                            |
//! | `'foo`                            | the `quoted` node                           |
//! | keyword compound form, `apply`    | the inner form node (`if_form`, `apply`, …) |
//! | `(define …)`                      | the `define` node                           |
//! | function-sugar lambda             | the function-name `NAME` token              |
//!
//! Node positions fall back to `(1, 1)` when the parser recorded none,
//! as the extractor did.

use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

// ===========================================================================
// One-node helpers
// ===========================================================================

/// The nested-node children of `node`, skipping bare tokens.
pub(crate) fn node_children(node: &GrammarASTNode) -> impl Iterator<Item = &GrammarASTNode> + Clone {
    node.children.iter().filter_map(|c| match c {
        ASTNodeOrToken::Node(n) => Some(n),
        ASTNodeOrToken::Token(_) => None,
    })
}

/// The first nested-node child of `node`, if any.
pub(crate) fn first_node(node: &GrammarASTNode) -> Option<&GrammarASTNode> {
    node_children(node).next()
}

/// The nested-node children of `node` whose rule is `rule`.
pub(crate) fn nodes_named<'a>(
    node: &'a GrammarASTNode,
    rule: &'a str,
) -> impl Iterator<Item = &'a GrammarASTNode> + Clone + 'a {
    node_children(node).filter(move |n| n.rule_name == rule)
}

/// The direct child tokens of `node` whose grammar name is `name`.
pub(crate) fn tokens_named<'a>(
    node: &'a GrammarASTNode,
    name: &'a str,
) -> impl Iterator<Item = &'a Token> + 'a {
    node.children.iter().filter_map(move |c| match c {
        ASTNodeOrToken::Token(t) if t.effective_type_name() == name => Some(t),
        _ => None,
    })
}

/// The first direct child token of `node` named `name`.
pub(crate) fn first_token_named<'a>(node: &'a GrammarASTNode, name: &'a str) -> Option<&'a Token> {
    tokens_named(node, name).next()
}

/// True iff `node` has a direct child token named `name`.
pub(crate) fn has_token(node: &GrammarASTNode, name: &str) -> bool {
    node.children
        .iter()
        .any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.effective_type_name() == name))
}

/// A node's start position, `(1, 1)` when the parser recorded none.
pub(crate) fn pos(node: &GrammarASTNode) -> (usize, usize) {
    (node.start_line.unwrap_or(1), node.start_column.unwrap_or(1))
}

/// The text of a `NAME`-like token, or `""` for a missing one (only
/// reachable on a tree that skipped [`crate::check_cst::check_cst`]).
fn text(t: Option<&Token>) -> &str {
    t.map(|t| t.value.as_str()).unwrap_or("")
}

// ===========================================================================
// Program-level views
// ===========================================================================

/// The `program` root: an optional module form plus top-level forms.
#[derive(Clone, Copy)]
pub(crate) struct ProgramView<'a>(pub &'a GrammarASTNode);

impl<'a> ProgramView<'a> {
    /// The top-level forms in source order.
    pub fn forms(self) -> impl Iterator<Item = FormKind<'a>> + 'a {
        nodes_named(self.0, "form").filter_map(FormKind::of)
    }

    /// The `(module …)` header.  The grammar allows at most one; like the
    /// old extractor, the *last* one wins should a hand-built tree carry
    /// several.
    pub fn module(self) -> Option<ModuleView<'a>> {
        nodes_named(self.0, "module_form").last().map(ModuleView)
    }
}

/// A `module_form` node — `(module name (export …) (import …) (typed …))`.
#[derive(Clone, Copy)]
pub(crate) struct ModuleView<'a>(&'a GrammarASTNode);

impl<'a> ModuleView<'a> {
    /// The inner clause nodes (`export_clause`, `import_clause`,
    /// `typed_clause`), each unwrapped from its `module_clause` wrapper.
    fn clauses(self) -> impl Iterator<Item = &'a GrammarASTNode> + 'a {
        nodes_named(self.0, "module_clause").flat_map(node_children)
    }

    /// Every exported name, in source order, across all `(export …)`
    /// clauses (duplicates kept — exactly as the extractor kept them).
    pub fn exports(self) -> impl Iterator<Item = &'a str> + 'a {
        self.clauses()
            .filter(|c| c.rule_name == "export_clause")
            .flat_map(|c| tokens_named(c, "NAME").map(|t| t.value.as_str()))
    }

    /// The `(typed …)` mode text (`"strict"`, `"lenient"`, `"off"`), last
    /// clause wins.  [`crate::check_cst::check_cst`] has already rejected
    /// any other word.
    pub fn typed_mode(self) -> Option<&'a str> {
        self.clauses()
            .filter(|c| c.rule_name == "typed_clause")
            .filter_map(|c| first_token_named(c, "NAME"))
            .map(|t| t.value.as_str())
            .last()
    }
}

/// One top-level form, classified one level deep.
#[derive(Clone, Copy)]
pub(crate) enum FormKind<'a> {
    /// `(define …)` in either shape.
    Define(DefineView<'a>),
    /// `(type Name t)` — compile-time only, erased.
    TypeAlias,
    /// `(record Name (field : T) …)`.
    RecordDef(RecordView<'a>),
    /// `(union Name (Variant (field : T) …) …)`.
    UnionDef(UnionView<'a>),
    /// A bare top-level expression.
    Expr(ExprNode<'a>),
}

impl<'a> FormKind<'a> {
    /// Classify a `form` node by its single child.
    fn of(form: &'a GrammarASTNode) -> Option<Self> {
        let inner = first_node(form)?;
        Some(match inner.rule_name.as_str() {
            "define" => FormKind::Define(DefineView::of(inner)?),
            "type_alias" => FormKind::TypeAlias,
            "record_def" => FormKind::RecordDef(RecordView(inner)),
            "union_def" => FormKind::UnionDef(UnionView(inner)),
            "expr" => FormKind::Expr(ExprNode(inner)),
            _ => return None,
        })
    }
}

// ===========================================================================
// define
// ===========================================================================

/// A `define` node.
///
/// ```text
/// (define name expr)                       ── value define
/// (define name : T expr)                   ── annotated value define
/// (define (name p (q : T) …) -> R body…)   ── function sugar
/// ```
///
/// The two shapes are told apart the way the extractor told them apart:
/// the function form's `name_or_signature` node has a direct `LPAREN`
/// child.
#[derive(Clone, Copy)]
pub(crate) struct DefineView<'a> {
    node: &'a GrammarASTNode,
    sig: &'a GrammarASTNode,
}

impl<'a> DefineView<'a> {
    fn of(node: &'a GrammarASTNode) -> Option<Self> {
        let sig = first_node(node)?;
        Some(DefineView { node, sig })
    }

    /// The defined name (the first `NAME` token of the signature).
    pub fn name(self) -> &'a str {
        text(first_token_named(self.sig, "NAME"))
    }

    /// The `define` node's position.
    pub fn pos(self) -> (usize, usize) {
        pos(self.node)
    }

    /// True for `(define (f …) …)` function sugar.
    fn is_sugar(self) -> bool {
        has_token(self.sig, "LPAREN")
    }

    /// The right-hand side when it is a lambda — either function sugar or
    /// a literal `(lambda …)` expression.  `None` for a value define.
    pub fn lambda(self) -> Option<LambdaView<'a>> {
        if self.is_sugar() {
            return Some(LambdaView::Sugar { define: self.node, sig: self.sig });
        }
        match self.value()?.kind() {
            ExprKind::Lambda(l) => Some(l),
            _ => None,
        }
    }

    /// The right-hand-side expression of a *value* define (`None` for
    /// function sugar, whose body is a list — see [`Self::lambda`]).
    pub fn value(self) -> Option<ExprNode<'a>> {
        if self.is_sugar() {
            return None;
        }
        Exprs::of(self.node).first()
    }
}

// ===========================================================================
// record / union
// ===========================================================================

/// A `record_def` node — `(record Name (field : T) …)`.
#[derive(Clone, Copy)]
pub(crate) struct RecordView<'a>(&'a GrammarASTNode);

impl<'a> RecordView<'a> {
    /// The record type's name.
    pub fn name(self) -> &'a str {
        text(first_token_named(self.0, "NAME"))
    }

    /// The field names in declaration order.
    pub fn fields(self) -> impl Iterator<Item = &'a str> + Clone + 'a {
        field_names(self.0)
    }
}

/// A `union_def` node — `(union Name (Variant (field : T) …) …)`.
#[derive(Clone, Copy)]
pub(crate) struct UnionView<'a>(&'a GrammarASTNode);

impl<'a> UnionView<'a> {
    /// The variants in declaration order (their index is their tag).
    pub fn variants(self) -> impl Iterator<Item = VariantView<'a>> + 'a {
        nodes_named(self.0, "union_variant").map(VariantView)
    }
}

/// A `union_variant` node — `(Variant (field : T) …)`.
#[derive(Clone, Copy)]
pub(crate) struct VariantView<'a>(&'a GrammarASTNode);

impl<'a> VariantView<'a> {
    /// The variant (constructor) name.
    pub fn name(self) -> &'a str {
        text(first_token_named(self.0, "NAME"))
    }

    /// The field names in declaration order.
    pub fn fields(self) -> impl Iterator<Item = &'a str> + Clone + 'a {
        field_names(self.0)
    }
}

/// The names of the `record_field` children of a record or variant.
fn field_names<'a>(node: &'a GrammarASTNode) -> impl Iterator<Item = &'a str> + Clone + 'a {
    node.children.iter().filter_map(|c| match c {
        ASTNodeOrToken::Node(f) if f.rule_name == "record_field" => {
            Some(text(first_token_named(f, "NAME")))
        }
        _ => None,
    })
}

// ===========================================================================
// Expressions
// ===========================================================================

/// A handle on one `expr` node.  Cheap to copy; classify it with
/// [`ExprNode::kind`] when the compiler needs to know what it is.
#[derive(Clone, Copy)]
pub(crate) struct ExprNode<'a>(pub(crate) &'a GrammarASTNode);

/// What an `expr` node *is*, one level deep.  Sub-expressions stay
/// unclassified [`ExprNode`] handles.
#[derive(Clone, Copy)]
pub(crate) enum ExprKind<'a> {
    /// `42`, `-7` — already range-checked by `check_cst`.
    IntLit(i64),
    /// `#t` / `#f`.
    BoolLit(bool),
    /// `nil`.
    NilLit,
    /// `'foo` or `(quote foo)` — the symbol's name.
    SymLit(&'a str),
    /// `"text"` — the lexer has already stripped quotes and decoded escapes.
    StrLit(&'a str),
    /// A bare name.
    VarRef(&'a str),
    /// `(if c t e)`.
    If(IfView<'a>),
    /// `(let ((x e) …) body…)`.
    Let(LetView<'a>),
    /// `(let* ((x e) …) body…)` (LANG52).
    LetStar(LetView<'a>),
    /// `(begin e…)` — the expressions.
    Begin(Exprs<'a>),
    /// `(lambda (p…) body…)`.
    Lambda(LambdaView<'a>),
    /// `(f a…)`.
    Apply(ApplyView<'a>),
    /// `(match e arm…)` (LANG48).
    Match(MatchView<'a>),
}

impl<'a> ExprNode<'a> {
    /// The single child of the `expr` node (`atom`, `quoted` or
    /// `compound`), and — for `compound` — the keyword-form node inside it.
    fn inner(self) -> Option<&'a GrammarASTNode> {
        let inner = first_node(self.0)?;
        if inner.rule_name == "compound" {
            first_node(inner)
        } else {
            Some(inner)
        }
    }

    /// The atom's token, when this expression is an atom.
    fn atom_token(self) -> Option<&'a Token> {
        let atom = first_node(self.0).filter(|n| n.rule_name == "atom")?;
        atom.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) => Some(t),
            ASTNodeOrToken::Node(_) => None,
        })
    }

    /// Classify this expression one level deep.
    pub fn kind(self) -> ExprKind<'a> {
        if let Some(t) = self.atom_token() {
            return match t.effective_type_name() {
                // `check_cst` proved the literal fits in i64.
                "INTEGER" => ExprKind::IntLit(t.value.parse().unwrap_or(0)),
                "BOOL_TRUE" => ExprKind::BoolLit(true),
                "BOOL_FALSE" => ExprKind::BoolLit(false),
                "STRING" => ExprKind::StrLit(t.value.as_str()),
                "KEYWORD" if t.value == "nil" => ExprKind::NilLit,
                _ => ExprKind::VarRef(t.value.as_str()),
            };
        }
        let Some(inner) = self.inner() else { return ExprKind::NilLit };
        match inner.rule_name.as_str() {
            "quoted" | "quote_form" => ExprKind::SymLit(text(first_token_named(inner, "NAME"))),
            "if_form" => {
                let mut e = Exprs::of(inner).iter();
                let (cond, then_branch, else_branch) = match (e.next(), e.next(), e.next()) {
                    (Some(c), Some(t), Some(f)) => (c, t, f),
                    _ => return ExprKind::NilLit,
                };
                ExprKind::If(IfView { cond, then_branch, else_branch, pos: pos(inner) })
            }
            "let_form" => ExprKind::Let(LetView(inner)),
            "let_star_form" => ExprKind::LetStar(LetView(inner)),
            "begin_form" => ExprKind::Begin(Exprs::of(inner)),
            "lambda_form" => ExprKind::Lambda(LambdaView::Anon(inner)),
            "match_form" => ExprKind::Match(MatchView(inner)),
            _ => ExprKind::Apply(ApplyView(inner)),
        }
    }

    /// The source position the old typed AST stored for this expression
    /// (see the module docs' position table).
    pub fn pos(self) -> (usize, usize) {
        if let Some(t) = self.atom_token() {
            return (t.line, t.column);
        }
        match self.inner() {
            Some(inner) => pos(inner),
            None => pos(self.0),
        }
    }

    /// The name, when this expression is a bare name reference.
    pub fn as_var(self) -> Option<&'a str> {
        match self.kind() {
            ExprKind::VarRef(name) => Some(name),
            _ => None,
        }
    }
}

/// A lazy list of the `expr` children of one node, optionally skipping a
/// leading prefix (an `apply`'s function position, say).
///
/// It is a cursor, not a collection: `iter`, `len` and `get` re-scan the
/// parent's children each time they are called.
#[derive(Clone, Copy)]
pub(crate) struct Exprs<'a> {
    parent: &'a GrammarASTNode,
    skip: usize,
}

impl<'a> Exprs<'a> {
    /// Every `expr` child of `parent`.
    pub fn of(parent: &'a GrammarASTNode) -> Self {
        Exprs { parent, skip: 0 }
    }

    /// The expressions, in source order.
    pub fn iter(self) -> impl Iterator<Item = ExprNode<'a>> + Clone + 'a {
        nodes_named(self.parent, "expr").skip(self.skip).map(ExprNode)
    }

    /// How many expressions there are.
    pub fn len(self) -> usize {
        self.iter().count()
    }

    /// The `i`-th expression.
    pub fn get(self, i: usize) -> Option<ExprNode<'a>> {
        self.iter().nth(i)
    }

    /// The first expression.
    pub fn first(self) -> Option<ExprNode<'a>> {
        self.iter().next()
    }

    /// The list without its first expression (`rest @ ..`).
    pub fn rest(self) -> Self {
        Exprs { parent: self.parent, skip: self.skip + 1 }
    }
}

/// `(if cond then else)` — the three branches, already split out.
#[derive(Clone, Copy)]
pub(crate) struct IfView<'a> {
    pub cond: ExprNode<'a>,
    pub then_branch: ExprNode<'a>,
    pub else_branch: ExprNode<'a>,
    /// The `if_form` node's position.
    pub pos: (usize, usize),
}

/// A `let_form` or `let_star_form` node.
#[derive(Clone, Copy)]
pub(crate) struct LetView<'a>(&'a GrammarASTNode);

impl<'a> LetView<'a> {
    /// The `(name rhs)` bindings in source order.
    pub fn bindings(self) -> impl Iterator<Item = (&'a str, ExprNode<'a>)> + Clone + 'a {
        nodes_named(self.0, "binding").filter_map(|b| {
            let name = first_token_named(b, "NAME")?.value.as_str();
            Some((name, Exprs::of(b).first()?))
        })
    }

    /// The body expressions.
    pub fn body(self) -> Exprs<'a> {
        Exprs::of(self.0)
    }

    /// The form node's position.
    pub fn pos(self) -> (usize, usize) {
        pos(self.0)
    }
}

/// An `apply` node — `(f a…)`.
#[derive(Clone, Copy)]
pub(crate) struct ApplyView<'a>(&'a GrammarASTNode);

impl<'a> ApplyView<'a> {
    /// The function-position expression.
    pub fn fn_expr(self) -> Option<ExprNode<'a>> {
        Exprs::of(self.0).first()
    }

    /// The function position's name, when it is a bare name.
    pub fn fn_name(self) -> Option<&'a str> {
        self.fn_expr()?.as_var()
    }

    /// The argument expressions.
    pub fn args(self) -> Exprs<'a> {
        Exprs::of(self.0).rest()
    }

    /// The `apply` node's position.
    pub fn pos(self) -> (usize, usize) {
        pos(self.0)
    }
}

// ===========================================================================
// Lambdas and type annotations
// ===========================================================================

/// A lambda, from either of its two spellings.
#[derive(Clone, Copy)]
pub(crate) enum LambdaView<'a> {
    /// `(lambda (p…) body…)` — a `lambda_form` node.  Anonymous lambdas
    /// carry no type annotations.
    Anon(&'a GrammarASTNode),
    /// `(define (f p…) body…)` — the `define` node (whose `expr` children
    /// are the body) and its `name_or_signature` node (params, annotations).
    Sugar {
        define: &'a GrammarASTNode,
        sig: &'a GrammarASTNode,
    },
}

/// One lambda parameter: its name and, for function sugar, its optional
/// `type_annotation` node.
#[derive(Clone, Copy)]
pub(crate) struct Param<'a> {
    pub name: &'a str,
    pub annotation: Option<&'a GrammarASTNode>,
}

impl<'a> LambdaView<'a> {
    /// The parameters in order.
    ///
    /// * `Anon` — the `NAME` tokens that precede the first body `expr`.
    /// * `Sugar` — after the function-name `NAME`: each bare `NAME` token
    ///   and each `typed_param` node (`p` or `(p : T)`), in order.
    pub fn params(self) -> impl Iterator<Item = Param<'a>> + Clone + 'a {
        let (node, sugar) = match self {
            LambdaView::Anon(n) => (n, false),
            LambdaView::Sugar { sig, .. } => (sig, true),
        };
        let mut seen_name = false;
        let mut seen_expr = false;
        node.children.iter().filter_map(move |c| match c {
            ASTNodeOrToken::Token(t) if t.effective_type_name() == "NAME" => {
                if sugar && !seen_name {
                    // The function's own name, not a parameter.
                    seen_name = true;
                    return None;
                }
                (!seen_expr).then_some(Param { name: t.value.as_str(), annotation: None })
            }
            ASTNodeOrToken::Node(n) if sugar && n.rule_name == "typed_param" => Some(Param {
                name: text(first_token_named(n, "NAME")),
                annotation: nodes_named(n, "type_annotation").next(),
            }),
            ASTNodeOrToken::Node(n) if !sugar && n.rule_name == "expr" => {
                seen_expr = true;
                None
            }
            _ => None,
        })
    }

    /// The `-> T` return annotation (function sugar only): the
    /// `type_annotation` node that directly follows the `ARROW` token.
    /// (The grammar allows one arrow; on a hand-built tree with several,
    /// the last one wins, as it did in the extractor.)
    pub fn return_annotation(self) -> Option<&'a GrammarASTNode> {
        let LambdaView::Sugar { sig, .. } = self else { return None };
        let mut after_arrow = false;
        let mut found = None;
        for c in &sig.children {
            match c {
                ASTNodeOrToken::Token(t) if t.effective_type_name() == "ARROW" => after_arrow = true,
                ASTNodeOrToken::Node(n) if after_arrow && n.rule_name == "type_annotation" => {
                    found = Some(n);
                    after_arrow = false;
                }
                _ => after_arrow = false,
            }
        }
        found
    }

    /// The body expressions.
    pub fn body(self) -> Exprs<'a> {
        match self {
            LambdaView::Anon(n) => Exprs::of(n),
            LambdaView::Sugar { define, .. } => Exprs::of(define),
        }
    }

    /// The position the old typed AST gave the lambda: the `lambda_form`
    /// node, or the function-name token of a sugared define.
    pub fn pos(self) -> (usize, usize) {
        match self {
            LambdaView::Anon(n) => pos(n),
            LambdaView::Sugar { sig, .. } => first_token_named(sig, "NAME")
                .map(|t| (t.line, t.column))
                .unwrap_or_else(|| pos(sig)),
        }
    }
}

/// One element of a type expression, as the annotation classifier sees it.
///
/// `type_annotation = LPAREN { type_annotation } RPAREN | NAME | INTEGER`,
/// so a type expression is a name, an integer, or a parenthesised list
/// whose items are themselves type expressions.
#[derive(Clone, Copy)]
pub(crate) enum TyItem<'a> {
    Name(&'a str),
    Int(i64),
    /// A parenthesised `type_annotation` node; read its items with
    /// [`ty_list_items`].
    List(&'a GrammarASTNode),
}

/// Read one `type_annotation` node as a [`TyItem`].
///
/// Without a direct `LPAREN` child the node is a bare `NAME` or
/// `INTEGER` (the first such token wins, as in the extractor).
pub(crate) fn ty_item(node: &GrammarASTNode) -> TyItem<'_> {
    if has_token(node, "LPAREN") {
        return TyItem::List(node);
    }
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            match t.effective_type_name() {
                "NAME" => return TyItem::Name(t.value.as_str()),
                "INTEGER" => return TyItem::Int(t.value.parse().unwrap_or(0)),
                _ => {}
            }
        }
    }
    TyItem::Name("")
}

/// The items of a parenthesised `type_annotation` node, in order:
/// nested `type_annotation` nodes, plus any bare `NAME` / `INTEGER`
/// tokens a future grammar might inline.  Brackets are skipped.
pub(crate) fn ty_list_items(node: &GrammarASTNode) -> impl Iterator<Item = TyItem<'_>> {
    node.children.iter().filter_map(|c| match c {
        ASTNodeOrToken::Token(t) => match t.effective_type_name() {
            "NAME" => Some(TyItem::Name(t.value.as_str())),
            "INTEGER" => Some(TyItem::Int(t.value.parse().unwrap_or(0))),
            _ => None,
        },
        ASTNodeOrToken::Node(sub) if sub.rule_name == "type_annotation" => Some(ty_item(sub)),
        ASTNodeOrToken::Node(_) => None,
    })
}

/// A type annotation, classified the way LANG23 PR 23-E reads it.
///
/// | annotation                    | class                    |
/// |-------------------------------|--------------------------|
/// | `int`                         | `UnrefinedInt`           |
/// | `any`                         | `Any`                    |
/// | `bool`                        | `UnrefinedBool`          |
/// | `(Int lo hi)` (two integers)  | `RangeInt { lo, hi }`    |
/// | `(Member int (v…))` (integers)| `MembershipInt(values)`  |
/// | anything else                 | `Opaque(node)`           |
pub(crate) enum Annotation<'a> {
    UnrefinedInt,
    Any,
    UnrefinedBool,
    RangeInt { lo: i128, hi: i128 },
    MembershipInt(Vec<i128>),
    /// A TW05 type expression the IIR erases to `any`; the node is kept so
    /// a caller can still recognise a bare name such as `str`.
    Opaque(&'a GrammarASTNode),
}

/// Classify a `type_annotation` node.
pub(crate) fn classify_annotation(node: &GrammarASTNode) -> Annotation<'_> {
    match ty_item(node) {
        TyItem::Name("int") => Annotation::UnrefinedInt,
        TyItem::Name("any") => Annotation::Any,
        TyItem::Name("bool") => Annotation::UnrefinedBool,
        TyItem::Name(_) | TyItem::Int(_) => Annotation::Opaque(node),
        TyItem::List(list) => {
            let mut items = ty_list_items(list);
            let parts = (items.next(), items.next(), items.next(), items.next());
            match parts {
                (Some(TyItem::Name("Int")), Some(TyItem::Int(lo)), Some(TyItem::Int(hi)), None) => {
                    Annotation::RangeInt { lo: lo as i128, hi: hi as i128 }
                }
                (
                    Some(TyItem::Name("Member")),
                    Some(TyItem::Name("int")),
                    Some(TyItem::List(values)),
                    None,
                ) => {
                    let ints: Option<Vec<i128>> = ty_list_items(values)
                        .map(|v| match v {
                            TyItem::Int(i) => Some(i as i128),
                            _ => None,
                        })
                        .collect();
                    match ints {
                        Some(values) => Annotation::MembershipInt(values),
                        None => Annotation::Opaque(node),
                    }
                }
                _ => Annotation::Opaque(node),
            }
        }
    }
}

// ===========================================================================
// match
// ===========================================================================

/// A `match_form` node — `(match scrutinee arm…)`.
#[derive(Clone, Copy)]
pub(crate) struct MatchView<'a>(&'a GrammarASTNode);

impl<'a> MatchView<'a> {
    /// The scrutinee (the first `expr` child).
    pub fn scrutinee(self) -> Option<ExprNode<'a>> {
        Exprs::of(self.0).first()
    }

    /// The arms in order.
    pub fn arms(self) -> impl Iterator<Item = ArmView<'a>> + 'a {
        nodes_named(self.0, "match_arm").map(ArmView)
    }

    /// The `match_form` node's position.
    pub fn pos(self) -> (usize, usize) {
        pos(self.0)
    }
}

/// A `match_arm` node — `(pattern body…)`.
#[derive(Clone, Copy)]
pub(crate) struct ArmView<'a>(&'a GrammarASTNode);

/// A match pattern, classified the way the extractor classified it.
#[derive(Clone, Copy)]
pub(crate) enum Pat<'a> {
    /// `(Name b…)` — the constructor name and the binder tokens' node.
    Variant { name: &'a str, node: &'a GrammarASTNode },
    /// `_`.
    Wildcard,
    /// Any other bare name.
    Binding(&'a str),
}

impl<'a> Pat<'a> {
    /// The names the pattern binds in its arm's body.
    pub fn bound_names(self) -> impl Iterator<Item = &'a str> + 'a {
        let (node, single) = match self {
            Pat::Variant { node, .. } => (Some(node), None),
            Pat::Binding(n) => (None, Some(n)),
            Pat::Wildcard => (None, None),
        };
        node.into_iter()
            .flat_map(|n| tokens_named(n, "NAME").skip(1).map(|t| t.value.as_str()))
            .chain(single)
    }
}

impl<'a> ArmView<'a> {
    /// The arm's pattern.
    pub fn pat(self) -> Pat<'a> {
        let Some(pat) = nodes_named(self.0, "match_pat").next() else { return Pat::Wildcard };
        let name = text(first_token_named(pat, "NAME"));
        if has_token(pat, "LPAREN") {
            Pat::Variant { name, node: pat }
        } else if name == "_" {
            Pat::Wildcard
        } else {
            Pat::Binding(name)
        }
    }

    /// The arm's body expressions.
    pub fn body(self) -> Exprs<'a> {
        Exprs::of(self.0)
    }
}
