//! The [`LanguageProfile`] trait — language-specific tree navigation for the
//! generic type checker.
//!
//! The `grammar-type-checker` knows *how* to type-check (scope tracking, kind
//! inference, arity/exhaustiveness checks) but not *which grammar rules play
//! which semantic roles*.  Each language provides a small struct that
//! implements [`LanguageProfile`], mapping its grammar rule names to the
//! semantic roles the checker understands.
//!
//! ## Implementing `LanguageProfile` for a new language
//!
//! 1. Inspect the language's `.grammar` file to find the rule names for:
//!    - Integer / bool / nil / symbol literals
//!    - Variable references
//!    - Function application
//!    - Let bindings
//!    - Lambda / function definitions
//!    - Match / case expressions
//!    - Begin / sequence blocks
//! 2. Implement each method by matching on `node.rule_name`.
//! 3. For "transparent wrapper" rules (e.g., Twig's `expr` / `compound`),
//!    return `None` from all role methods and list the single meaningful child
//!    in [`child_exprs`](LanguageProfile::child_exprs) so the checker recurses
//!    through it automatically.

use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use type_declarations::KindDecl;

// ---------------------------------------------------------------------------
// Helper return types
// ---------------------------------------------------------------------------

/// Extracted information about a function application node.
pub struct AppInfo<'a> {
    /// The function or callee expression.
    pub callee: &'a GrammarASTNode,
    /// The argument expressions, in call order.
    pub args: Vec<&'a GrammarASTNode>,
}

/// Discriminated binder information — distinguishes let-style from
/// lambda/function-style binding forms.
pub enum BinderKind<'a> {
    /// A `let`-style binder: each binding has a name and an expression whose
    /// kind is evaluated and bound.
    ///
    /// Scheme semantics: all RHS expressions are evaluated in the *outer*
    /// scope before any name is visible in the inner scope.
    Let {
        /// `(name, rhs_expr)` pairs.
        bindings: Vec<(String, &'a GrammarASTNode)>,
        /// Body expressions — the last one's kind is the result kind.
        body: Vec<&'a GrammarASTNode>,
    },

    /// A `lambda`-style binder: parameters are names with optional kind
    /// annotations; the result kind is `KindDecl::Function { arity }`.
    ///
    /// Parameters are bound in the body scope (not the outer scope).
    Lambda {
        /// `(param_name, param_kind)` pairs.  Kind comes from the source
        /// type annotation if present, otherwise [`KindDecl::Any`].
        params: Vec<(String, KindDecl)>,
        /// Body expressions.
        body: Vec<&'a GrammarASTNode>,
    },
}

/// Information extracted from a binder node.
pub struct BinderInfo<'a> {
    /// Whether this is a global binder (top-level define) or local
    /// (let / anonymous lambda).
    pub is_global: bool,
    /// The actual binding structure.
    pub kind: BinderKind<'a>,
}

/// Extracted information about a match / case expression.
pub struct MatchInfo<'a> {
    /// The expression being matched.
    pub scrutinee: &'a GrammarASTNode,
    /// The match arms in source order.
    pub arms: Vec<MatchArmInfo<'a>>,
}

/// One arm of a match expression.
pub struct MatchArmInfo<'a> {
    /// The pattern that this arm matches.
    pub pattern: ArmPattern,
    /// The body expression of this arm.
    pub body: &'a GrammarASTNode,
}

/// A pattern in a match arm.
#[derive(Debug, Clone)]
pub enum ArmPattern {
    /// `(VariantName binding0 binding1 ...)` — matches a specific union variant
    /// and binds its fields.
    Variant {
        /// Name of the union variant constructor.
        name: String,
        /// Names to bind to each variant field.
        bindings: Vec<String>,
    },
    /// `_` — matches anything, binds nothing.
    Wildcard,
    /// `varname` — matches anything, binds the scrutinee to `varname`.
    Binding(String),
}

// ---------------------------------------------------------------------------
// LanguageProfile trait
// ---------------------------------------------------------------------------

/// Language-specific tree navigation for the generic type checker.
///
/// The generic checker calls these methods on every `GrammarASTNode` it
/// encounters.  Each method should:
///
/// - Return `Some(info)` when the node plays that semantic role.
/// - Return `None` when the node is something else (the checker will try the
///   next method or fall back to [`child_exprs`](Self::child_exprs)).
///
/// The methods are tried in this priority order inside [`crate::check`]:
///
/// 1. `literal_kind`
/// 2. `as_var_ref`
/// 3. `as_apply`
/// 4. `as_binder`
/// 5. `as_match`
/// 6. `as_begin`
/// 7. `child_exprs` (fallback)
pub trait LanguageProfile: Send + Sync {
    /// Return the literal kind produced by this node, if it is a literal.
    ///
    /// # Examples
    ///
    /// | Grammar rule | Token(s) | Returns |
    /// |---|---|---|
    /// | `atom` + INTEGER | `42` | `Some(KindDecl::Int)` |
    /// | `atom` + BOOL_TRUE | `#t` | `Some(KindDecl::Bool)` |
    /// | `quoted` | `'foo` | `Some(KindDecl::Symbol)` |
    /// | `atom` + NAME | `my-var` | `None` (variable reference) |
    fn literal_kind(&self, node: &GrammarASTNode) -> Option<KindDecl>;

    /// If this node is a variable reference, return the variable name.
    ///
    /// The checker uses the name to look up the kind in the scope stack and
    /// in `TypeDeclarations::globals`.  Returns `None` for all non-reference
    /// nodes.
    fn as_var_ref<'a>(&self, node: &'a GrammarASTNode) -> Option<&'a str>;

    /// If this node is a function application, extract the callee and
    /// argument sub-expressions.
    ///
    /// The checker infers the callee's kind, checks arity if it is
    /// `Function { arity }`, then infers each argument.
    fn as_apply<'a>(&self, node: &'a GrammarASTNode) -> Option<AppInfo<'a>>;

    /// If this node introduces new bindings (let / lambda / define), extract
    /// them.
    ///
    /// The checker uses the returned [`BinderInfo`] to extend the scope stack
    /// and infer the body's kind.
    fn as_binder<'a>(&self, node: &'a GrammarASTNode) -> Option<BinderInfo<'a>>;

    /// If this node is a match / case expression, extract its scrutinee and
    /// arms.
    ///
    /// The checker infers the scrutinee's kind, checks exhaustiveness if it
    /// is a known union, then infers each arm body.
    fn as_match<'a>(&self, node: &'a GrammarASTNode) -> Option<MatchInfo<'a>>;

    /// If this node is a sequential block (e.g., `begin`), return its
    /// sub-expressions in order.
    ///
    /// The checker infers all expressions and returns the last one's kind.
    fn as_begin<'a>(&self, node: &'a GrammarASTNode) -> Option<Vec<&'a GrammarASTNode>>;

    /// Return the child expression nodes to recurse into as a fallback.
    ///
    /// Used when none of the above role methods match.  For "transparent
    /// wrapper" rules (e.g., Twig's `expr`, `compound`, `form`) this should
    /// return the single meaningful child.  For structural expressions like
    /// `if_form`, return the condition + branches.
    fn child_exprs<'a>(&self, node: &'a GrammarASTNode) -> Vec<&'a GrammarASTNode>;

    /// Return the source position `(line, column)` of the node for error
    /// reporting.
    ///
    /// The default implementation reads `start_line` / `start_column` from
    /// the grammar node.
    fn position(&self, node: &GrammarASTNode) -> (usize, usize) {
        (
            node.start_line.unwrap_or(0),
            node.start_column.unwrap_or(0),
        )
    }
}

// ---------------------------------------------------------------------------
// Helpers shared across profile implementations
// ---------------------------------------------------------------------------

/// Collect all child `GrammarASTNode`s (dropping raw token children).
pub fn ast_node_children(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

/// Collect all child `GrammarASTNode`s whose `rule_name` matches `rule`.
pub fn ast_nodes_named<'a>(node: &'a GrammarASTNode, rule: &str) -> Vec<&'a GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == rule => Some(n),
            _ => None,
        })
        .collect()
}

/// Return the text of the first token child whose effective type name matches
/// `token_type`, or `None` if not found.
pub fn first_token_value<'a>(node: &'a GrammarASTNode, token_type: &str) -> Option<&'a str> {
    node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Token(t) if t.effective_type_name() == token_type => {
            Some(t.value.as_str())
        }
        _ => None,
    })
}

/// True if the node has at least one token child of the given effective type.
pub fn has_token_type(node: &GrammarASTNode, token_type: &str) -> bool {
    node.children.iter().any(|c| match c {
        ASTNodeOrToken::Token(t) => t.effective_type_name() == token_type,
        _ => false,
    })
}
