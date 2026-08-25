//! The lowering pass from `coding_adventures_java_parser`'s generic
//! [`GrammarASTNode`] CST → [`semantic_ir::Module`], **v0.1.0 (JV02
//! milestone M0)**.
//!
//! # Scope
//!
//! Java requires an explicit `class`/`main`-method wrapper at the source
//! level — this milestone recognizes exactly one shape and returns a clean
//! [`JavaLowerError`] for anything else, rather than silently
//! mis-lowering:
//!
//! **Supported:**
//! - Exactly one top-level `class` declaration, containing exactly one
//!   `public static void main(String[] args) { ... }` method.
//! - The method body is a flat sequence of literal expression statements:
//!   integer (`42;`), floating-point (`3.14;`), boolean (`true;`/
//!   `false;`), `null;`, and string (`"str";`) literals.
//!
//! **Deliberately out of scope for v0.1.0** (each rejected with an
//! explicit [`JavaLowerError`], tracked in
//! [JV02](../../../specs/JV02-java-to-semantic-ir.md)'s own milestone
//! table as M1 onward): variable references and assignment, every
//! operator (including unary `-`/`+`/`!`), control flow, method calls,
//! additional classes/methods/fields, and every SIR29 construct
//! (`NominalClassDef`/`InterfaceDef`/`MethodDef`/`VirtualCall`) — this
//! milestone lowers the `main` method's own statements directly into the
//! synthesized SIR `main` [`Function`], it does not yet lower the *class*
//! declaration itself into a `Stmt::NominalClassDef` at all.

use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span, Stmt,
};

/// Maximum descent depth through the Java grammar's expression-precedence
/// chain (`assignment_expression` → `conditional_expression` → … →
/// `literal`, one grammar rule per precedence level — see this module's
/// own doc comment and `java-parser`'s own `MAX_RULE_DEPTH`). Mirrors
/// every other SIR frontend's identically-named, identically-justified
/// guard: turns pathologically deep (but parseable) input into a clean
/// [`JavaLowerError`] instead of a native (uncatchable) stack overflow.
/// The parser's own `MAX_RULE_DEPTH` (180) already bounds this in
/// practice; this is this frontend's own independent guard, not a
/// reliance on that upstream cap.
const MAX_EXPR_DEPTH: usize = 64;

/// Maximum recursion depth for `find_main_method`'s class-body search — a
/// separate budget from [`MAX_EXPR_DEPTH`] (that one bounds the
/// expression-precedence chain specifically; this one bounds an arbitrary
/// class-body tree walk, a conceptually different traversal even though
/// both currently use the same numeric value). Exists for the same
/// reason: `compile()` is a public entry point accepting a raw
/// `GrammarASTNode`, not guaranteed to have come from a depth-capped
/// parser.
const MAX_TREE_DEPTH: usize = 64;

/// Synthetic file name used for all spans (the CST does not carry the
/// original path).
const FILE: &str = "<java>";

/// An error encountered during Java → SIR lowering.
///
/// Mirrors `MatlabLowerError`/`PythonLowerError`/`TwigLowerError`'s shape
/// exactly (`message` + 1-based `line`/`column`) so tooling can treat
/// every SIR frontend uniformly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaLowerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

/// Lower a parsed Java `program` CST into a [`Module`] named
/// `module_name`. See this module's own doc comment for the exact
/// supported subset (JV02 milestone M0).
pub fn compile(tree: &GrammarASTNode, module_name: &str) -> Result<Module, JavaLowerError> {
    Lowerer::new(module_name).lower_program(tree)
}

struct Lowerer {
    module_name: String,
    /// Features observed while lowering, used to build the manifest so it
    /// declares *exactly* what the module emits (mirrors every other SIR
    /// frontend's own `observed` accumulator).
    observed: FeatureManifest,
}

impl Lowerer {
    fn new(module_name: &str) -> Self {
        Self {
            module_name: module_name.to_string(),
            observed: FeatureManifest::new(),
        }
    }

    fn lower_program(&mut self, program: &GrammarASTNode) -> Result<Module, JavaLowerError> {
        if program.rule_name != "program" {
            return Err(self.err_at(
                program,
                format!("expected `program` root, got `{}`", program.rule_name),
            ));
        }

        let mut class_decls = Vec::new();
        collect_bounded(program, "class_declaration", 0, self, &mut class_decls)?;
        let class_decl = match class_decls.as_slice() {
            [only] => *only,
            [] => return Err(self.err_at(program, "expected one top-level class declaration, found none (JV02 M0 supports exactly one)".to_string())),
            _ => return Err(self.err_at(program, format!("expected exactly one top-level class declaration, found {} (JV02 M0 supports exactly one)", class_decls.len()))),
        };

        let main_method = self.find_main_method(class_decl)?;
        let method_body = self
            .first_child_named(main_method, "method_body")
            .ok_or_else(|| self.err_at(main_method, "main method has no body".to_string()))?;
        let block = self
            .first_child_named(method_body, "block")
            .ok_or_else(|| self.err_at(method_body, "main method body has no block".to_string()))?;

        let mut stmts = Vec::new();
        for block_stmt in child_nodes(block) {
            if block_stmt.rule_name != "block_statement" {
                continue;
            }
            stmts.push(self.lower_block_statement(block_stmt)?);
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
            .with_source_language("java")
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

    /// Find the single `public static void main(String[] args)` method
    /// inside `class_decl`'s body. Any other method-name/modifier shape
    /// (including a genuinely absent `main`) is rejected — JV02 M0's
    /// scope is exactly this one entry-point shape.
    ///
    /// Hand-written recursive search rather than
    /// `parser::grammar_parser::find_nodes` (that shared helper is
    /// unguarded — see [`collect_bounded`]'s own doc comment for why this
    /// crate cannot use it on a raw, possibly-adversarial tree — and
    /// returns owned `GrammarASTNode` clones besides, which can't be
    /// borrowed back into `class_decl`'s own tree): this search instead
    /// walks `class_decl` directly, depth-guarded, and returns a real
    /// borrow.
    ///
    /// Depth-guarded like `descend_to_literal` (see that method's own doc
    /// comment for why this crate doesn't rely on the upstream parser's
    /// own `MAX_RULE_DEPTH` cap): `compile()` is a public entry point that
    /// accepts a raw `GrammarASTNode` directly, not only one produced by
    /// `parse_java`'s depth-capped parser — a caller could hand it a tree
    /// built some other, unbounded way. Without its own cap this
    /// recursive walk over a class body would be a CWE-674 uncontrolled-
    /// recursion DoS on adversarially deep input; found by
    /// `/security-review` before this crate shipped.
    fn find_main_method<'a>(
        &self,
        class_decl: &'a GrammarASTNode,
    ) -> Result<&'a GrammarASTNode, JavaLowerError> {
        fn search<'a>(
            node: &'a GrammarASTNode,
            lowerer: &Lowerer,
            depth: usize,
        ) -> Result<Option<&'a GrammarASTNode>, JavaLowerError> {
            if depth >= MAX_TREE_DEPTH {
                return Err(lowerer.err_at(
                    node,
                    format!("class body nesting exceeds {MAX_TREE_DEPTH} levels"),
                ));
            }
            if node.rule_name == "method_declaration"
                && lowerer.method_name(node).as_deref() == Some("main")
            {
                return Ok(Some(node));
            }
            for child in &node.children {
                if let ASTNodeOrToken::Node(n) = child {
                    if let Some(found) = search(n, lowerer, depth + 1)? {
                        return Ok(Some(found));
                    }
                }
            }
            Ok(None)
        }
        search(class_decl, self, 0)?.ok_or_else(|| {
            self.err_at(
                class_decl,
                "expected a `main` method (JV02 M0 requires `public static void main(String[] args)`)"
                    .to_string(),
            )
        })
    }

    fn method_name(&self, method_decl: &GrammarASTNode) -> Option<String> {
        let declarator = self.first_child_named(method_decl, "method_declarator")?;
        for child in &declarator.children {
            if let ASTNodeOrToken::Token(t) = child {
                if t.type_ == lexer::token::TokenType::Name {
                    return Some(t.value.clone());
                }
            }
        }
        None
    }

    fn lower_block_statement(&mut self, block_stmt: &GrammarASTNode) -> Result<Stmt, JavaLowerError> {
        let statement = self
            .first_child_named(block_stmt, "statement")
            .ok_or_else(|| self.err_at(block_stmt, "expected a `statement` (JV02 M0 supports only literal expression statements)".to_string()))?;
        let expr_stmt = self.first_child_named(statement, "expression_statement").ok_or_else(|| {
            self.err_at(
                statement,
                "unsupported statement kind (JV02 M0 supports only literal expression statements)"
                    .to_string(),
            )
        })?;
        let expression = self
            .first_child_named(expr_stmt, "expression")
            .ok_or_else(|| self.err_at(expr_stmt, "expression statement has no expression".to_string()))?;
        let literal = self.descend_to_literal(expression, 0)?;
        let expr = self.lower_literal(literal)?;
        let span = self.span_of(expression);
        Ok(Stmt::ExprStmt { expr, span })
    }

    /// Walk down the Java grammar's expression-precedence chain
    /// (`assignment_expression` → `conditional_expression` → … →
    /// `primary` → `literal`) to the `literal` rule at the bottom.
    ///
    /// Every level in this chain has exactly one child when the source
    /// used no actual operator at that precedence — the parser still
    /// builds the whole chain of single-child wrapper nodes, since the
    /// grammar has no "elide an empty level" shortcut. A node with more
    /// than one child (or with a `unary_expression`'s optional leading
    /// `+`/`-`/`!` token) means a real operator is present, which is out
    /// of scope for M0 and rejected with a clear error rather than
    /// mis-lowered.
    fn descend_to_literal<'a>(
        &self,
        node: &'a GrammarASTNode,
        depth: usize,
    ) -> Result<&'a GrammarASTNode, JavaLowerError> {
        if depth >= MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting exceeds {MAX_EXPR_DEPTH} levels"),
            ));
        }
        if node.rule_name == "literal" {
            return Ok(node);
        }
        // Check the RAW children (`node.children`), not `child_nodes(node)`
        // (which filters out `ASTNodeOrToken::Token` entries) — a real
        // unary `-`/`+`/`!` shows up as an extra *token* sibling alongside
        // the nested `unary_expression` node (e.g. `unary_expression { TOKEN
        // Minus "-", unary_expression { ... } }`, confirmed by direct
        // probing of the parser's output for `-7`). Checking only the
        // Node-filtered list would silently accept that shape as "one
        // wrapper level" and drop the minus sign entirely — a confirmed,
        // caught-by-its-own-test regression during development (a genuine
        // "never trade loud for silent" case): `-7;` must be REJECTED as
        // an unsupported operator, not silently lowered to `IntLit(7)`.
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(only)] => self.descend_to_literal(only, depth + 1),
            _ => Err(self.err_at(
                node,
                format!(
                    "unsupported expression (JV02 M0 supports only bare literals, found a `{}` with an operator)",
                    node.rule_name
                ),
            )),
        }
    }

    /// Lower a `literal` node's single token child to an [`Expr`].
    fn lower_literal(&mut self, literal: &GrammarASTNode) -> Result<Expr, JavaLowerError> {
        let span = self.span_of(literal);
        let tok = match literal.children.as_slice() {
            [ASTNodeOrToken::Token(t)] => t,
            _ => return Err(self.err_at(literal, "malformed literal node".to_string())),
        };
        match (tok.type_, tok.value.as_str()) {
            (_, "true") => Ok(Expr::BoolLit { value: true, span }),
            (_, "false") => Ok(Expr::BoolLit { value: false, span }),
            (_, "null") => Ok(Expr::NilLit { span }),
            (lexer::token::TokenType::Number, text) => Ok(self.number_literal_expr(text, span)),
            (lexer::token::TokenType::String, text) => {
                self.observed.add(Feature::Strings);
                Ok(Expr::StrLit {
                    value: text.to_string(),
                    span,
                })
            }
            _ => Err(self.err_at(
                literal,
                format!("unsupported literal token `{}` (`{}`)", tok.value, tok.type_),
            )),
        }
    }

    /// A Java `NUMBER` lexeme is a float if it has a decimal point or
    /// exponent (or an `f`/`F`/`d`/`D` suffix, stripped before parsing —
    /// M0 does not distinguish Java's `float` vs. `double`, both lower to
    /// `Expr::FloatLit`), otherwise an int; an integer lexeme too large
    /// for `i64` falls back to a float rather than silently truncating or
    /// erroring. Mirrors `matlab-to-semantic-ir`'s identically-reasoned
    /// `number_literal_expr` — including its own hard-won lesson that
    /// `Feature::Floats` must be observed on every `FloatLit` branch, not
    /// just the "has a dot" one, or a float-literal module fails
    /// `semantic_ir::validate()`.
    fn number_literal_expr(&mut self, text: &str, span: Span) -> Expr {
        let trimmed = text.trim_end_matches(['f', 'F', 'd', 'D']);
        if trimmed.contains('.') || trimmed.contains('e') || trimmed.contains('E') {
            self.observed.add(Feature::Floats);
            Expr::FloatLit {
                value: trimmed.parse::<f64>().unwrap_or(0.0),
                span,
            }
        } else {
            match trimmed.parse::<i64>() {
                Ok(v) => Expr::IntLit { value: v, span },
                Err(_) => {
                    self.observed.add(Feature::Floats);
                    Expr::FloatLit {
                        value: trimmed.parse::<f64>().unwrap_or(0.0),
                        span,
                    }
                }
            }
        }
    }

    fn first_child_named<'a>(
        &self,
        node: &'a GrammarASTNode,
        kind: &str,
    ) -> Option<&'a GrammarASTNode> {
        child_nodes(node).into_iter().find(|n| n.rule_name == kind)
    }

    fn span_of(&self, node: &GrammarASTNode) -> Span {
        Span::point(
            FILE,
            node.start_line.unwrap_or(1),
            node.start_column.unwrap_or(1),
        )
    }

    fn err_at(&self, node: &GrammarASTNode, message: String) -> JavaLowerError {
        JavaLowerError {
            message,
            line: node.start_line.unwrap_or(1),
            column: node.start_column.unwrap_or(1),
        }
    }
}

fn child_nodes(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

/// Depth-guarded pre-order collection of every node named `rule_name`
/// under `node` (inclusive). Deliberately hand-written rather than using
/// the shared `parser::grammar_parser::find_nodes` helper: that function
/// has no depth cap of its own, and `compile()` — the caller that
/// ultimately reaches this — is a public entry point accepting a raw
/// `GrammarASTNode` directly, not only one produced by `parse_java`'s own
/// `MAX_RULE_DEPTH`-capped parser. Calling the unguarded shared helper on
/// a possibly-adversarial tree would reintroduce the exact CWE-674
/// uncontrolled-recursion DoS this crate's own `MAX_TREE_DEPTH` guard
/// (see `find_main_method`'s identically-reasoned `search` helper) exists
/// to prevent — found by `/security-review` as a second, earlier-executing
/// instance of the same gap before this crate shipped.
fn collect_bounded<'a>(
    node: &'a GrammarASTNode,
    rule_name: &str,
    depth: usize,
    lowerer: &Lowerer,
    out: &mut Vec<&'a GrammarASTNode>,
) -> Result<(), JavaLowerError> {
    if depth >= MAX_TREE_DEPTH {
        return Err(lowerer.err_at(
            node,
            format!("tree nesting exceeds {MAX_TREE_DEPTH} levels"),
        ));
    }
    if node.rule_name == rule_name {
        out.push(node);
    }
    for child in &node.children {
        if let ASTNodeOrToken::Node(n) = child {
            collect_bounded(n, rule_name, depth + 1, lowerer, out)?;
        }
    }
    Ok(())
}
