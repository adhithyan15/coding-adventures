//! The lowering pass from `coding_adventures_java_parser`'s generic
//! [`GrammarASTNode`] CST → [`semantic_ir::Module`], **v0.2.0 (JV02
//! milestone M1)**.
//!
//! # Scope
//!
//! Java requires an explicit `class`/`main`-method wrapper at the source
//! level — this milestone recognizes exactly one shape and returns a clean
//! [`JavaLowerError`] for anything else, rather than silently
//! mis-lowering:
//!
//! **Supported (M0, unchanged):**
//! - Exactly one top-level `class` declaration, containing exactly one
//!   `public static void main(String[] args) { ... }` method.
//! - Literal expressions: integer (`42`), floating-point (`3.14`), boolean
//!   (`true`/`false`), `null`, and string (`"str"`) literals.
//!
//! **Supported (M1, new):**
//! - Local variable declarations with an explicit primitive type
//!   (`int`/`long`/`short`/`byte`/`char`/`float`/`double`/`boolean`) or
//!   `String`, each requiring an initializer (`int x = 1;`).
//! - `var` type inference (Java 10+ — see "The `var` ambiguity" below),
//!   inferring the declared kind from the initializer.
//! - Re-assignment of an already-declared local (`x = 2;`), plain `=`
//!   only.
//! - Arithmetic (`+ - * / %`), relational (`< > <= >=`), equality
//!   (`== !=`), and logical (`&& || !`) operators, plus unary `+`/`-`.
//! - String concatenation via `+` when either operand is `String`
//!   (lowers to [`Expr::StrConcat`], which auto-stringifies non-string
//!   parts — see that node's own doc comment — matching Java's own `+`
//!   semantics for mixed-type concatenation, e.g. `"n=" + 5`).
//! - Parenthesized sub-expressions.
//!
//! **Deliberately out of scope for v0.2.0** (each rejected with an
//! explicit [`JavaLowerError`], tracked in
//! [JV02](../../../specs/JV02-java-to-semantic-ir.md)'s own milestone
//! table as M2 onward): control flow, method calls, field/array access,
//! lambdas, casts, `instanceof`, the ternary conditional, bitwise
//! operators (`& | ^ ~ << >> >>>`), compound assignment (`+=` etc.),
//! increment/decrement (`++`/`--`), uninitialized declarations, multiple
//! declarators per statement, C-style array-bracket declarators, array
//! initializers, and reference types other than `String`.
//!
//! ## The `var` ambiguity
//!
//! `local_var_type = type | "var"` is an ordered PEG choice with `type`
//! tried first. Since `type` can itself resolve to a bare `class_type`
//! (`qualified_name` of one segment), the grammar parses `var x = 1;` as
//! `type -> class_type -> qualified_name -> NAME "var"` — the literal
//! `"var"` alternative is *never actually reached* for real source
//! (confirmed by direct inspection of the parser's own output, not
//! assumed from reading the grammar). This lowerer therefore detects
//! `var` by its resolved shape (a single-segment class type literally
//! named `var`) rather than by which grammar alternative matched. This is
//! not a heuristic: the JLS reserves `var` as a type name, so no real
//! Java source can ever declare a class actually named `var` — the two
//! cases are truly unambiguous, just not distinguished by which grammar
//! rule fired.

use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Scope, Span, Stmt,
};
use std::collections::HashMap;

/// Maximum descent depth through the Java grammar's expression-precedence
/// chain (`assignment_expression` → `conditional_expression` → … →
/// `literal`, one grammar rule per precedence level — see this module's
/// own doc comment and `java-parser`'s own `MAX_RULE_DEPTH`). Mirrors
/// every other SIR frontend's identically-named, identically-justified
/// guard: turns pathologically deep (but parseable) input into a clean
/// [`JavaLowerError`] instead of a native (uncatchable) stack overflow.
/// The parser's own `MAX_RULE_DEPTH` (180) already bounds this in
/// practice; this is this frontend's own independent guard, not a
/// reliance on that upstream cap. Every mutually-recursive expression
/// lowering helper in this module (`lower_expr` and its callees) takes
/// and threads a `depth` parameter checked against this constant, exactly
/// like M0's `descend_to_literal` did.
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

/// A lightweight, lowering-time-only classification of a Java local
/// variable's/expression's static type — just enough to select the
/// correct SIR operator (`div_trunc` vs `div_true`, `StrConcat` vs
/// numeric `+`) and to reject nonsensical operand combinations (`"a" -
/// "b"`, `1 && 2`). This is *not* a real type checker: it assumes the
/// input is already valid, type-correct Java (as every other SIR
/// frontend assumes about its own input) and exists purely to recover
/// the handful of type-directed decisions Java's own compiler would make
/// implicitly. `Null` exists only transiently, as the kind of a bare
/// `null` literal — a variable's own tracked kind is always its
/// *declared* kind (`Str` for `String x = null;`), never `Null` itself
/// (see `lower_local_var_decl`'s handling of the `var x = null;` case,
/// which Java itself also rejects).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Int,
    Float,
    Bool,
    Str,
    Null,
}

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
/// supported subset (JV02 milestone M1).
pub fn compile(tree: &GrammarASTNode, module_name: &str) -> Result<Module, JavaLowerError> {
    Lowerer::new(module_name).lower_program(tree)
}

struct Lowerer {
    module_name: String,
    /// Features observed while lowering, used to build the manifest so it
    /// declares *exactly* what the module emits (mirrors every other SIR
    /// frontend's own `observed` accumulator).
    observed: FeatureManifest,
    /// Declared kind of every local variable seen so far in the `main`
    /// method body. M1's `main` body is still a flat statement list (no
    /// nested blocks/control flow yet — that lands in M2, which is also
    /// where real lexical scoping becomes necessary), so a single flat
    /// map is sufficient and correct for this milestone.
    locals: HashMap<String, Kind>,
}

impl Lowerer {
    fn new(module_name: &str) -> Self {
        Self {
            module_name: module_name.to_string(),
            observed: FeatureManifest::new(),
            locals: HashMap::new(),
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
    /// Depth-guarded like the expression-lowering chain below: `compile()`
    /// is a public entry point that accepts a raw `GrammarASTNode`
    /// directly, not only one produced by `parse_java`'s depth-capped
    /// parser — a caller could hand it a tree built some other, unbounded
    /// way. Without its own cap this recursive walk over a class body
    /// would be a CWE-674 uncontrolled-recursion DoS on adversarially
    /// deep input; found by `/security-review` before this crate shipped.
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

    // ── statement-level lowering ────────────────────────────────────

    /// Lower one `block_statement`. `block_statement = var_declaration |
    /// class_declaration | statement`, and (a grammar quirk) `statement`
    /// itself *also* lists `var_declaration` as one of its own
    /// alternatives — both positions are checked so a local variable
    /// declaration is recognized regardless of which alternative the
    /// parser actually took.
    fn lower_block_statement(
        &mut self,
        block_stmt: &GrammarASTNode,
    ) -> Result<Stmt, JavaLowerError> {
        if let Some(var_decl) = self.first_child_named(block_stmt, "var_declaration") {
            return self.lower_var_declaration_node(var_decl);
        }
        let statement = self.first_child_named(block_stmt, "statement").ok_or_else(|| {
            self.err_at(
                block_stmt,
                "unsupported statement kind (JV02 M1 supports only variable declarations, assignment, and bare expression statements — control flow is deferred to JV02 M2)"
                    .to_string(),
            )
        })?;
        if let Some(var_decl) = self.first_child_named(statement, "var_declaration") {
            return self.lower_var_declaration_node(var_decl);
        }
        let expr_stmt = self.first_child_named(statement, "expression_statement").ok_or_else(|| {
            self.err_at(
                statement,
                "unsupported statement kind (JV02 M1 supports only variable declarations, assignment, and bare expression statements — control flow is deferred to JV02 M2)"
                    .to_string(),
            )
        })?;
        let expression = self
            .first_child_named(expr_stmt, "expression")
            .ok_or_else(|| {
                self.err_at(
                    expr_stmt,
                    "expression statement has no expression".to_string(),
                )
            })?;
        self.lower_expr_statement(expression)
    }

    fn lower_var_declaration_node(
        &mut self,
        var_decl: &GrammarASTNode,
    ) -> Result<Stmt, JavaLowerError> {
        let lvds = self
            .first_child_named(var_decl, "local_variable_declaration_statement")
            .ok_or_else(|| self.err_at(var_decl, "malformed variable declaration".to_string()))?;
        self.lower_local_var_decl(lvds)
    }

    /// Lower `local_variable_declaration_statement` (`{annotation}
    /// ["final"] local_var_type variable_declarators SEMICOLON`) into a
    /// `Stmt::LetBinding`. See this module's own doc comment for the
    /// exact supported subset (single declarator, initializer required,
    /// no array-bracket declarator suffix).
    fn lower_local_var_decl(&mut self, lvds: &GrammarASTNode) -> Result<Stmt, JavaLowerError> {
        let lvt = self
            .first_child_named(lvds, "local_var_type")
            .ok_or_else(|| {
                self.err_at(
                    lvds,
                    "malformed local variable declaration (missing type)".to_string(),
                )
            })?;
        let declared_kind = self.declared_kind_of_local_var_type(lvt)?;

        let declarators = self
            .first_child_named(lvds, "variable_declarators")
            .ok_or_else(|| {
                self.err_at(
                    lvds,
                    "malformed local variable declaration (missing declarators)".to_string(),
                )
            })?;
        let decls: Vec<&GrammarASTNode> = child_nodes(declarators)
            .into_iter()
            .filter(|n| n.rule_name == "variable_declarator")
            .collect();
        let declarator = match decls.as_slice() {
            [only] => *only,
            _ => return Err(self.err_at(
                declarators,
                "multiple variable declarators in one statement are not supported yet (deferred; declare each variable in its own statement)".to_string(),
            )),
        };

        let has_array_brackets = declarator
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "["));
        if has_array_brackets {
            return Err(self.err_at(
                declarator,
                "C-style array declarator brackets (`int x[]`) are not supported yet (deferred to JV02 M4)".to_string(),
            ));
        }
        let name_tok = declarator
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Token(t) if t.type_ == lexer::token::TokenType::Name => Some(t),
                _ => None,
            })
            .ok_or_else(|| {
                self.err_at(
                    declarator,
                    "malformed variable declarator (missing name)".to_string(),
                )
            })?;
        let name = name_tok.value.clone();

        let initializer = self.first_child_named(declarator, "variable_initializer").ok_or_else(|| {
            self.err_at(
                declarator,
                "uninitialized local variable declarations are not supported yet (JV02 M1 requires an initializer)".to_string(),
            )
        })?;
        let init_expr_node = match initializer.children.as_slice() {
            [ASTNodeOrToken::Node(n)] if n.rule_name == "expression" => n,
            _ => {
                return Err(self.err_at(
                    initializer,
                    "array initializers are not supported yet (deferred to JV02 M4)".to_string(),
                ))
            }
        };
        let (value, value_kind) = self.lower_expr(init_expr_node, 0)?;

        let kind = match declared_kind {
            Some(k) => k,
            None => {
                if value_kind == Kind::Null {
                    return Err(self.err_at(
                        init_expr_node,
                        "cannot infer `var`'s type from a `null` initializer".to_string(),
                    ));
                }
                value_kind
            }
        };
        self.locals.insert(name.clone(), kind);

        // `Stmt::LetStarBinding`, not `LetBinding`: Java's local
        // declarations are strictly sequential — `int x = 1; int y = x +
        // 1;` requires `y`'s initializer to see `x`. `LetBinding` has
        // *parallel*-let semantics instead (consecutive bindings evaluate
        // outside each other's scope — see that variant's own doc
        // comment), which would make every declaration but the first
        // reference an "unknown name" per `semantic_ir::validate()`.
        let span = self.span_of(lvds);
        Ok(Stmt::LetStarBinding {
            name,
            sir_type: None,
            value,
            span,
        })
    }

    /// Resolve `local_var_type`'s declared kind, or `None` for `var`
    /// (type inferred from the initializer by the caller). See this
    /// module's own doc comment ("The `var` ambiguity") for why `var` is
    /// detected by resolved shape rather than by grammar alternative.
    fn declared_kind_of_local_var_type(
        &self,
        lvt: &GrammarASTNode,
    ) -> Result<Option<Kind>, JavaLowerError> {
        match lvt.children.as_slice() {
            // The literal `"var"` grammar alternative — dead in practice
            // (see the module doc comment) but handled defensively in
            // case a future grammar revision changes the ordering.
            [ASTNodeOrToken::Token(t)] if t.value == "var" => Ok(None),
            [ASTNodeOrToken::Node(type_node)] => {
                if single_segment_class_type_name(type_node) == Some("var") {
                    return Ok(None);
                }
                self.kind_of_type_node(type_node).map(Some)
            }
            _ => Err(self.err_at(lvt, "malformed local variable type".to_string())),
        }
    }

    /// Resolve a `type` node (`{annotation} primitive_type
    /// {LBRACKET RBRACKET} | {annotation} class_type {LBRACKET RBRACKET}`)
    /// to a [`Kind`]. Only `String` is accepted among reference types —
    /// every other class type (including any user-defined class, since
    /// M1 has no class-declaration lowering yet) is out of scope.
    fn kind_of_type_node(&self, type_node: &GrammarASTNode) -> Result<Kind, JavaLowerError> {
        let has_array_brackets = type_node
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "["));
        if has_array_brackets {
            return Err(self.err_at(
                type_node,
                "array types are not supported yet (deferred to JV02 M4)".to_string(),
            ));
        }
        if let Some(prim) = self.first_child_named(type_node, "primitive_type") {
            let tok = prim
                .children
                .iter()
                .find_map(|c| match c {
                    ASTNodeOrToken::Token(t) => Some(t),
                    ASTNodeOrToken::Node(_) => None,
                })
                .ok_or_else(|| self.err_at(prim, "malformed primitive type".to_string()))?;
            return match tok.value.as_str() {
                "boolean" => Ok(Kind::Bool),
                "byte" | "short" | "int" | "long" | "char" => Ok(Kind::Int),
                "float" | "double" => Ok(Kind::Float),
                other => Err(self.err_at(prim, format!("unsupported primitive type `{other}`"))),
            };
        }
        if let Some(class_type) = self.first_child_named(type_node, "class_type") {
            return match single_segment_class_type_name(type_node) {
                Some("String") => Ok(Kind::Str),
                _ => {
                    let name =
                        qualified_name_text(class_type).unwrap_or_else(|| "<unknown>".to_string());
                    Err(self.err_at(
                        class_type,
                        format!("unsupported reference type `{name}` (JV02 M1 supports only `String` and primitive types)"),
                    ))
                }
            };
        }
        Err(self.err_at(type_node, "malformed type node".to_string()))
    }

    /// Lower a full expression-statement's `expression` node: either a
    /// bare-name-target assignment (`x = <rhs>;` → `Stmt::Assign`) or an
    /// ordinary value expression evaluated for effect (→ `Stmt::ExprStmt`,
    /// matching M0's existing behavior for e.g. `42;`).
    fn lower_expr_statement(
        &mut self,
        expression: &GrammarASTNode,
    ) -> Result<Stmt, JavaLowerError> {
        let inner = match expression.children.as_slice() {
            [ASTNodeOrToken::Node(n)] => n,
            _ => return Err(self.err_at(expression, "malformed `expression` node".to_string())),
        };
        if inner.rule_name == "lambda_expression" {
            return Err(self.err_at(
                inner,
                "lambda expressions are not supported yet (deferred to JV02 M3)".to_string(),
            ));
        }
        if inner.rule_name == "assignment_expression" {
            if let [ASTNodeOrToken::Node(lvalue_node), ASTNodeOrToken::Node(op_node), ASTNodeOrToken::Node(rhs_node)] =
                inner.children.as_slice()
            {
                let op_tok = op_node
                    .children
                    .iter()
                    .find_map(|c| match c {
                        ASTNodeOrToken::Token(t) => Some(t),
                        ASTNodeOrToken::Node(_) => None,
                    })
                    .ok_or_else(|| {
                        self.err_at(op_node, "malformed assignment operator".to_string())
                    })?;
                if op_tok.value != "=" {
                    return Err(self.err_at(
                        op_node,
                        format!(
                            "compound assignment operator `{}` is not supported yet (deferred; write it as a plain `=` with the operator spelled out on the right-hand side)",
                            op_tok.value
                        ),
                    ));
                }
                let name = self.extract_bare_name(lvalue_node, 0)?;
                if !self.locals.contains_key(&name) {
                    return Err(self.err_at(
                        lvalue_node,
                        format!("assignment to undeclared local variable `{name}`"),
                    ));
                }
                let (value, _kind) = self.lower_expr(rhs_node, 0)?;
                self.observed.add(Feature::MutableBindings);
                let span = self.span_of(inner);
                return Ok(Stmt::Assign {
                    name,
                    scope: Scope::Local,
                    value,
                    span,
                });
            }
        }
        let (expr, _kind) = self.lower_expr(inner, 0)?;
        let span = self.span_of(expression);
        Ok(Stmt::ExprStmt { expr, span })
    }

    /// Walk an assignment target's `unary_expression` chain down to its
    /// `primary`, requiring it to be a bare `NAME` — `foo.bar = x`,
    /// `arr[0] = x`, and any other non-simple target are out of scope for
    /// M1 (rejected here rather than mis-lowered).
    fn extract_bare_name(
        &self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<String, JavaLowerError> {
        if depth >= MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting exceeds {MAX_EXPR_DEPTH} levels"),
            ));
        }
        if node.rule_name == "primary" {
            return match node.children.as_slice() {
                [ASTNodeOrToken::Token(t)] if t.type_ == lexer::token::TokenType::Name => Ok(t.value.clone()),
                _ => Err(self.err_at(
                    node,
                    "assignment target must be a simple local variable (JV02 M1 does not support field or indexed assignment targets)".to_string(),
                )),
            };
        }
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(only)] => self.extract_bare_name(only, depth + 1),
            _ => Err(self.err_at(
                node,
                "assignment target must be a simple local variable (JV02 M1 does not support field or indexed assignment targets)".to_string(),
            )),
        }
    }

    // ── expression-level lowering ───────────────────────────────────
    //
    // Each Java grammar precedence level gets its own dispatch arm below,
    // mirroring the grammar's own explicit rule-per-level structure (see
    // `java21.grammar`'s "Assignment" through "Primary Expressions"
    // sections). Every level that has no real operator present in a
    // given tree is a single-child wrapper — that case always just
    // recurses into the one child. A level with more than one child means
    // a real operator is present, which is either lowered (if in M1's
    // scope) or rejected with a clear "deferred" error.

    /// Dispatch on `node.rule_name` to the right precedence-level lowering
    /// helper. Returns the lowered [`Expr`] together with its inferred
    /// [`Kind`] (needed by callers up the chain to pick the right SIR
    /// operator — `div_trunc` vs `div_true`, `StrConcat` vs numeric `+`
    /// — and to reject ill-typed operand combinations).
    fn lower_expr(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        if depth >= MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting exceeds {MAX_EXPR_DEPTH} levels"),
            ));
        }
        match node.rule_name.as_str() {
            "expression" => self.lower_expression_rule(node, depth),
            "assignment_expression" => self.lower_assignment_expression_as_value(node, depth),
            "conditional_expression" => self.lower_conditional_expression(node, depth),
            "logical_or_expression" => self.lower_logical_chain(node, depth, "||", false),
            "logical_and_expression" => self.lower_logical_chain(node, depth, "&&", true),
            "bitwise_or_expression" | "bitwise_xor_expression" | "bitwise_and_expression" => {
                self.lower_single_child_only(node, depth, "bitwise operators")
            }
            "equality_expression" => self.lower_equality(node, depth),
            "relational_expression" => self.lower_relational(node, depth),
            "shift_expression" => self.lower_single_child_only(node, depth, "shift operators"),
            "additive_expression" => self.lower_additive(node, depth),
            "multiplicative_expression" => self.lower_multiplicative(node, depth),
            "unary_expression" => self.lower_unary(node, depth),
            "unary_expression_not_plus_minus" => self.lower_unary_not_plus_minus(node, depth),
            "postfix_expression" => self.lower_postfix(node, depth),
            "primary_expression" => self.lower_primary_expression(node, depth),
            "primary" => self.lower_primary(node, depth),
            other => Err(self.err_at(
                node,
                format!(
                    "unsupported expression construct `{other}` (JV02 M1 does not lower this yet)"
                ),
            )),
        }
    }

    /// `expression = lambda_expression | assignment_expression ;`
    fn lower_expression_rule(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(only)] if only.rule_name == "lambda_expression" => Err(self
                .err_at(
                    only,
                    "lambda expressions are not supported yet (deferred to JV02 M3)".to_string(),
                )),
            [ASTNodeOrToken::Node(only)] => self.lower_expr(only, depth + 1),
            _ => Err(self.err_at(node, "malformed `expression` node".to_string())),
        }
    }

    /// `assignment_expression = unary_expression assignment_operator
    /// assignment_expression | conditional_expression ;` — reached here
    /// only for a *value* position (statement-top assignment is peeled
    /// off earlier by `lower_expr_statement`), so the 3-child real-
    /// assignment shape means a *nested* assignment expression, which
    /// M1 does not support (SIR's `Assign` is a statement, not an
    /// expression — see `Stmt::Assign`'s own doc comment).
    fn lower_assignment_expression_as_value(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(only)] => self.lower_expr(only, depth + 1),
            [ASTNodeOrToken::Node(_), ASTNodeOrToken::Node(_), ASTNodeOrToken::Node(_)] => Err(self.err_at(
                node,
                "nested assignment expressions are not supported (JV02 M1 supports assignment only as a full statement)".to_string(),
            )),
            _ => Err(self.err_at(node, "malformed `assignment_expression` node".to_string())),
        }
    }

    /// `conditional_expression = logical_or_expression [ QUESTION
    /// assignment_expression COLON assignment_expression ] ;`
    fn lower_conditional_expression(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(only)] => self.lower_expr(only, depth + 1),
            _ => Err(self.err_at(
                node,
                "the ternary conditional operator (`?:`) is not supported yet (deferred to a later JV02 milestone)".to_string(),
            )),
        }
    }

    /// Shared fold for `logical_or_expression` (`{ OR_OR
    /// logical_and_expression }`) and `logical_and_expression` (`{
    /// AND_AND bitwise_or_expression }`) — both require every operand to
    /// be `Kind::Bool` and produce `Kind::Bool`.
    fn lower_logical_chain(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
        op_value: &str,
        is_and: bool,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        let mut acc: Option<(Expr, Kind)> = None;
        for child in &node.children {
            if let ASTNodeOrToken::Node(n) = child {
                let (expr, kind) = self.lower_expr(n, depth + 1)?;
                acc = Some(match acc.take() {
                    // Pure passthrough (no real operator at this level) —
                    // do NOT validate `kind` here. Every expression flows
                    // through this precedence level regardless of its
                    // type; only an *actual* `&&`/`||` combination
                    // requires boolean operands.
                    None => (expr, kind),
                    Some((lhs, lhs_kind)) => {
                        if lhs_kind != Kind::Bool || kind != Kind::Bool {
                            return Err(
                                self.err_at(n, format!("`{op_value}` requires boolean operands"))
                            );
                        }
                        let span = lhs.span().clone();
                        let combined = if is_and {
                            Expr::LogicalAnd {
                                lhs: Box::new(lhs),
                                rhs: Box::new(expr),
                                span,
                            }
                        } else {
                            Expr::LogicalOr {
                                lhs: Box::new(lhs),
                                rhs: Box::new(expr),
                                span,
                            }
                        };
                        self.observed.add(Feature::ShortCircuit);
                        (combined, Kind::Bool)
                    }
                });
            }
        }
        acc.ok_or_else(|| self.err_at(node, format!("empty `{}` expression", node.rule_name)))
    }

    /// A precedence level M1 does not touch (bitwise/shift): pass through
    /// when the grammar produced no real operator (single child), reject
    /// with a clear "deferred" error otherwise.
    fn lower_single_child_only(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
        what: &str,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(only)] => self.lower_expr(only, depth + 1),
            _ => Err(self.err_at(
                node,
                format!("{what} are not supported yet (deferred to a later JV02 milestone)"),
            )),
        }
    }

    /// `equality_expression = relational_expression { (EQUALS_EQUALS |
    /// NOT_EQUALS) relational_expression } ;`. Restricted to numeric/
    /// boolean operands — Java's `==`/`!=` on `String` is *reference*
    /// equality (a well-known Java gotcha, since it silently diverges
    /// from `.equals()`), a fundamentally different operation from every
    /// other SIR frontend's `=`/`!=` builtin (value equality); lowering
    /// it as value equality would be a silent correctness bug, so it is
    /// rejected instead (string equality needs `.equals()`, which is
    /// method-call surface — JV02 M4+).
    fn lower_equality(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        let mut acc: Option<(Expr, Kind)> = None;
        let mut pending_op: Option<&'static str> = None;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(t) if t.value == "==" || t.value == "!=" => {
                    pending_op = Some(if t.value == "==" { "=" } else { "!=" });
                }
                ASTNodeOrToken::Node(n) => {
                    let (expr, kind) = self.lower_expr(n, depth + 1)?;
                    acc = Some(match (acc.take(), pending_op.take()) {
                        // Pure passthrough (no real `==`/`!=` at this
                        // level) — do NOT validate `kind` here; only an
                        // actual comparison requires numeric/boolean
                        // operands.
                        (None, _) => (expr, kind),
                        (Some((lhs, lhs_kind)), Some(op)) => {
                            if kind == Kind::Str || lhs_kind == Kind::Str {
                                return Err(self.err_at(
                                    n,
                                    "`==`/`!=` on `String` is Java reference equality, not value equality, and is not supported (use `.equals()` — deferred to a later JV02 milestone)".to_string(),
                                ));
                            }
                            if !kinds_compatible_for_compare(lhs_kind, kind) {
                                return Err(self.err_at(
                                    node,
                                    "equality comparison requires both operands to be the same general kind (both numeric, or both boolean)".to_string(),
                                ));
                            }
                            let span = lhs.span().clone();
                            (
                                Expr::BuiltinCall {
                                    name: op.to_string(),
                                    args: vec![lhs, expr],
                                    effects: EffectSet::PURE,
                                    span,
                                },
                                Kind::Bool,
                            )
                        }
                        (Some(_), None) => {
                            return Err(
                                self.err_at(node, "malformed equality expression".to_string())
                            )
                        }
                    });
                }
                ASTNodeOrToken::Token(_) => {}
            }
        }
        acc.ok_or_else(|| self.err_at(node, "empty equality expression".to_string()))
    }

    /// `relational_expression = shift_expression { (LESS_THAN |
    /// GREATER_THAN | LESS_EQUALS | GREATER_EQUALS) shift_expression |
    /// "instanceof" instanceof_target } ;`
    fn lower_relational(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        let has_instanceof = node
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "instanceof"));
        if has_instanceof {
            return Err(self.err_at(
                node,
                "`instanceof` is not supported yet (deferred to a later JV02 milestone)"
                    .to_string(),
            ));
        }
        let mut acc: Option<(Expr, Kind)> = None;
        let mut pending_op: Option<&'static str> = None;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(t) if matches!(t.value.as_str(), "<" | ">" | "<=" | ">=") => {
                    pending_op = Some(match t.value.as_str() {
                        "<" => "<",
                        ">" => ">",
                        "<=" => "<=",
                        ">=" => ">=",
                        _ => unreachable!(),
                    });
                }
                ASTNodeOrToken::Node(n) => {
                    let (expr, kind) = self.lower_expr(n, depth + 1)?;
                    acc = Some(match (acc.take(), pending_op.take()) {
                        // Pure passthrough (no real relational operator at
                        // this level) — do NOT validate `kind` here; only
                        // an actual comparison requires numeric operands.
                        (None, _) => (expr, kind),
                        (Some((lhs, lhs_kind)), Some(op)) => {
                            if !matches!(lhs_kind, Kind::Int | Kind::Float)
                                || !matches!(kind, Kind::Int | Kind::Float)
                            {
                                return Err(self.err_at(
                                    n,
                                    "relational comparison requires numeric operands".to_string(),
                                ));
                            }
                            let span = lhs.span().clone();
                            (
                                Expr::BuiltinCall {
                                    name: op.to_string(),
                                    args: vec![lhs, expr],
                                    effects: EffectSet::PURE,
                                    span,
                                },
                                Kind::Bool,
                            )
                        }
                        (Some(_), None) => {
                            return Err(
                                self.err_at(node, "malformed relational expression".to_string())
                            )
                        }
                    });
                }
                ASTNodeOrToken::Token(_) => {}
            }
        }
        acc.ok_or_else(|| self.err_at(node, "empty relational expression".to_string()))
    }

    /// `additive_expression = multiplicative_expression { (PLUS | MINUS)
    /// multiplicative_expression } ;`. `+` routes to string concatenation
    /// when either operand is `Kind::Str` (see `combine_additive`);
    /// everything else requires numeric operands.
    fn lower_additive(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        let mut acc: Option<(Expr, Kind)> = None;
        let mut pending_op: Option<char> = None;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(t) if t.value == "+" || t.value == "-" => {
                    pending_op = Some(t.value.chars().next().expect("non-empty operator token"));
                }
                ASTNodeOrToken::Node(n) => {
                    let (rhs, rhs_kind) = self.lower_expr(n, depth + 1)?;
                    acc = Some(match (acc.take(), pending_op.take()) {
                        (None, _) => (rhs, rhs_kind),
                        (Some((lhs, lhs_kind)), Some(op)) => {
                            self.combine_additive(lhs, lhs_kind, rhs, rhs_kind, op, node)?
                        }
                        (Some(_), None) => {
                            return Err(
                                self.err_at(node, "malformed additive expression".to_string())
                            )
                        }
                    });
                }
                ASTNodeOrToken::Token(_) => {}
            }
        }
        acc.ok_or_else(|| self.err_at(node, "empty additive expression".to_string()))
    }

    fn combine_additive(
        &mut self,
        lhs: Expr,
        lhs_kind: Kind,
        rhs: Expr,
        rhs_kind: Kind,
        op: char,
        node: &GrammarASTNode,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        if op == '+' && (lhs_kind == Kind::Str || rhs_kind == Kind::Str) {
            self.observed.add(Feature::StringInterpolation);
            let span = lhs.span().clone();
            let parts = match lhs {
                Expr::StrConcat { parts, .. } => {
                    let mut parts = parts;
                    parts.push(rhs);
                    parts
                }
                other => vec![other, rhs],
            };
            return Ok((Expr::StrConcat { parts, span }, Kind::Str));
        }
        if !matches!(lhs_kind, Kind::Int | Kind::Float)
            || !matches!(rhs_kind, Kind::Int | Kind::Float)
        {
            return Err(self.err_at(
                node,
                format!(
                    "`{op}` requires numeric operands (or, for `+`, at least one `String` operand)"
                ),
            ));
        }
        let result_kind = if lhs_kind == Kind::Float || rhs_kind == Kind::Float {
            Kind::Float
        } else {
            Kind::Int
        };
        let span = lhs.span().clone();
        Ok((
            Expr::BuiltinCall {
                name: op.to_string(),
                args: vec![lhs, rhs],
                effects: EffectSet::PURE,
                span,
            },
            result_kind,
        ))
    }

    /// `multiplicative_expression = unary_expression { (STAR | SLASH |
    /// PERCENT) unary_expression } ;`. `/` selects `div_trunc` (both
    /// operands integral — Java truncates toward zero, same as Rust/C)
    /// or `div_true` (either operand `float`/`double`) per SIR21 T3b-2's
    /// op-name convention (see `c-to-semantic-ir`'s identically-reasoned
    /// selection). Java's primitive numeric types are all signed, so
    /// `udiv_trunc` never applies here.
    fn lower_multiplicative(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        let mut acc: Option<(Expr, Kind)> = None;
        let mut pending_op: Option<char> = None;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(t) if matches!(t.value.as_str(), "*" | "/" | "%") => {
                    pending_op = Some(t.value.chars().next().expect("non-empty operator token"));
                }
                ASTNodeOrToken::Node(n) => {
                    let (rhs, rhs_kind) = self.lower_expr(n, depth + 1)?;
                    acc = Some(match (acc.take(), pending_op.take()) {
                        (None, _) => (rhs, rhs_kind),
                        (Some((lhs, lhs_kind)), Some(op)) => {
                            self.combine_multiplicative(lhs, lhs_kind, rhs, rhs_kind, op, node)?
                        }
                        (Some(_), None) => {
                            return Err(self
                                .err_at(node, "malformed multiplicative expression".to_string()))
                        }
                    });
                }
                ASTNodeOrToken::Token(_) => {}
            }
        }
        acc.ok_or_else(|| self.err_at(node, "empty multiplicative expression".to_string()))
    }

    fn combine_multiplicative(
        &mut self,
        lhs: Expr,
        lhs_kind: Kind,
        rhs: Expr,
        rhs_kind: Kind,
        op: char,
        node: &GrammarASTNode,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        if !matches!(lhs_kind, Kind::Int | Kind::Float)
            || !matches!(rhs_kind, Kind::Int | Kind::Float)
        {
            return Err(self.err_at(node, format!("`{op}` requires numeric operands")));
        }
        let is_float = lhs_kind == Kind::Float || rhs_kind == Kind::Float;
        let result_kind = if is_float { Kind::Float } else { Kind::Int };
        let name = match op {
            '*' => "*".to_string(),
            '%' => "%".to_string(),
            '/' => {
                if is_float {
                    "div_true".to_string()
                } else {
                    "div_trunc".to_string()
                }
            }
            _ => unreachable!("combine_multiplicative called with an unrecognized operator"),
        };
        let span = lhs.span().clone();
        Ok((
            Expr::BuiltinCall {
                name,
                args: vec![lhs, rhs],
                effects: EffectSet::PURE,
                span,
            },
            result_kind,
        ))
    }

    /// `unary_expression = PLUS_PLUS unary_expression | MINUS_MINUS
    /// unary_expression | PLUS unary_expression | MINUS unary_expression
    /// | unary_expression_not_plus_minus ;`
    fn lower_unary(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(only)] => self.lower_expr(only, depth + 1),
            [ASTNodeOrToken::Token(t), ASTNodeOrToken::Node(inner)] => match t.value.as_str() {
                "++" | "--" => Err(self.err_at(
                    node,
                    "prefix increment/decrement operators are not supported yet (deferred to a later JV02 milestone)".to_string(),
                )),
                "+" => {
                    let (expr, kind) = self.lower_expr(inner, depth + 1)?;
                    if !matches!(kind, Kind::Int | Kind::Float) {
                        return Err(self.err_at(inner, "unary `+` requires a numeric operand".to_string()));
                    }
                    Ok((expr, kind))
                }
                "-" => {
                    let (expr, kind) = self.lower_expr(inner, depth + 1)?;
                    if !matches!(kind, Kind::Int | Kind::Float) {
                        return Err(self.err_at(inner, "unary `-` requires a numeric operand".to_string()));
                    }
                    let negated = match expr {
                        Expr::IntLit { value, span } => Expr::IntLit {
                            value: value.wrapping_neg(),
                            span,
                        },
                        Expr::FloatLit { value, span } => Expr::FloatLit { value: -value, span },
                        other => {
                            let span = other.span().clone();
                            Expr::BuiltinCall {
                                name: "neg".to_string(),
                                args: vec![other],
                                effects: EffectSet::PURE,
                                span,
                            }
                        }
                    };
                    Ok((negated, kind))
                }
                other => Err(self.err_at(node, format!("unsupported unary operator `{other}`"))),
            },
            _ => Err(self.err_at(node, "malformed `unary_expression` node".to_string())),
        }
    }

    /// `unary_expression_not_plus_minus = TILDE unary_expression | BANG
    /// unary_expression | cast_expression | postfix_expression ;`. The
    /// single-child case covers *both* remaining alternatives —
    /// `cast_expression` is naturally rejected by `lower_expr`'s own
    /// catch-all (it has no dispatch arm), so no special-case is needed
    /// here to distinguish it from `postfix_expression`.
    fn lower_unary_not_plus_minus(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(only)] => self.lower_expr(only, depth + 1),
            [ASTNodeOrToken::Token(t), ASTNodeOrToken::Node(inner)] => match t.value.as_str() {
                "~" => Err(self.err_at(
                    node,
                    "bitwise complement (`~`) is not supported yet (deferred to a later JV02 milestone)".to_string(),
                )),
                "!" => {
                    let (expr, kind) = self.lower_expr(inner, depth + 1)?;
                    if kind != Kind::Bool {
                        return Err(self.err_at(inner, "unary `!` requires a boolean operand".to_string()));
                    }
                    let span = expr.span().clone();
                    Ok((
                        Expr::BuiltinCall {
                            name: "not".to_string(),
                            args: vec![expr],
                            effects: EffectSet::PURE,
                            span,
                        },
                        Kind::Bool,
                    ))
                }
                other => Err(self.err_at(node, format!("unsupported unary operator `{other}`"))),
            },
            _ => Err(self.err_at(node, "malformed `unary_expression_not_plus_minus` node".to_string())),
        }
    }

    /// `postfix_expression = primary_expression { PLUS_PLUS | MINUS_MINUS
    /// } ;`
    fn lower_postfix(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        let has_incr_decr =
            node.children.iter().skip(1).any(
                |c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "++" || t.value == "--"),
            );
        if has_incr_decr {
            return Err(self.err_at(
                node,
                "postfix increment/decrement operators are not supported yet (deferred to a later JV02 milestone)".to_string(),
            ));
        }
        match node.children.first() {
            Some(ASTNodeOrToken::Node(primary_expr)) => self.lower_expr(primary_expr, depth + 1),
            _ => Err(self.err_at(node, "malformed `postfix_expression` node".to_string())),
        }
    }

    /// `primary_expression = primary { primary_suffix } ;` — any suffix
    /// (`.field`, `.method(...)`, `::ref`, etc.) is field/method-access
    /// surface, out of scope until JV02 M3+.
    fn lower_primary_expression(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        if node.children.len() > 1 {
            return Err(self.err_at(
                node,
                "field access, method calls, and other primary suffixes are not supported yet (deferred to a later JV02 milestone)".to_string(),
            ));
        }
        match node.children.first() {
            Some(ASTNodeOrToken::Node(primary)) => self.lower_expr(primary, depth + 1),
            _ => Err(self.err_at(node, "malformed `primary_expression` node".to_string())),
        }
    }

    /// `primary = literal | "this" | ... | LPAREN expression RPAREN |
    /// NAME ;` — M1 supports exactly three of these alternatives:
    /// literals, parenthesized sub-expressions, and bare variable
    /// references. Everything else (`this`, `super`, `switch`
    /// expressions, object/array construction) is out of scope.
    fn lower_primary(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(n)] if n.rule_name == "literal" => {
                let expr = self.lower_literal(n)?;
                let kind = kind_of_literal_expr(&expr);
                Ok((expr, kind))
            }
            // Bare `NAME` — a variable reference. This position is
            // reached only when the grammar matched the `NAME` terminal
            // specifically (not an operator token — the shared,
            // cross-language `TokenType` enum tags several operators
            // without their own dedicated variant as `Name` too, but
            // `primary`'s own grammar production never places one of
            // those there), so the token is always a genuine identifier
            // lexeme here.
            [ASTNodeOrToken::Token(t)] if t.type_ == lexer::token::TokenType::Name => {
                let name = t.value.clone();
                let kind = *self.locals.get(&name).ok_or_else(|| {
                    self.err_at(node, format!("reference to undeclared local variable `{name}`"))
                })?;
                let span = self.span_of(node);
                Ok((Expr::VarRef { name, scope: Scope::Local, span }, kind))
            }
            [ASTNodeOrToken::Token(open), ASTNodeOrToken::Node(inner), ASTNodeOrToken::Token(_close)]
                if open.value == "(" =>
            {
                self.lower_expr(inner, depth + 1)
            }
            _ => Err(self.err_at(
                node,
                "unsupported primary expression (JV02 M1 supports only literals, bare variable references, and parenthesized expressions)".to_string(),
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
                format!(
                    "unsupported literal token `{}` (`{}`)",
                    tok.value, tok.type_
                ),
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

/// Two kinds are compatible operands for `==`/`!=` if they're both
/// numeric (Java allows mixed `int`/`double` comparison via numeric
/// promotion) or both boolean — never one of each.
fn kinds_compatible_for_compare(a: Kind, b: Kind) -> bool {
    matches!(
        (a, b),
        (Kind::Bool, Kind::Bool) | (Kind::Int | Kind::Float, Kind::Int | Kind::Float)
    )
}

/// The [`Kind`] of an `Expr` freshly produced by `lower_literal` — only
/// ever one of these five variants, since that is the entirety of what
/// `lower_literal` can construct.
fn kind_of_literal_expr(expr: &Expr) -> Kind {
    match expr {
        Expr::IntLit { .. } => Kind::Int,
        Expr::FloatLit { .. } => Kind::Float,
        Expr::BoolLit { .. } => Kind::Bool,
        Expr::StrLit { .. } => Kind::Str,
        Expr::NilLit { .. } => Kind::Null,
        other => unreachable!("lower_literal produced an unexpected expr shape: {other:?}"),
    }
}

/// If `type_node` is `class_type { LBRACKET RBRACKET }` with no array
/// brackets and a single-segment `qualified_name`, return that one
/// segment's text (e.g. `"String"`, or `"var"` — see this module's own
/// doc comment on the `var` ambiguity). Returns `None` for a primitive
/// type, a multi-segment qualified name (`java.lang.String`), or an
/// array type — none of those are the shape this helper exists to detect.
fn single_segment_class_type_name(type_node: &GrammarASTNode) -> Option<&str> {
    let class_type = type_node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(n) if n.rule_name == "class_type" => Some(n),
        _ => None,
    })?;
    let qualified = class_type.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(n) if n.rule_name == "qualified_name" => Some(n),
        _ => None,
    })?;
    let names: Vec<&str> = qualified
        .children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.type_ == lexer::token::TokenType::Name => {
                Some(t.value.as_str())
            }
            _ => None,
        })
        .collect();
    match names.as_slice() {
        [only] => Some(only),
        _ => None,
    }
}

/// Render a `class_type` node's `qualified_name` back to dotted text, for
/// error messages only (e.g. `"java.util.List"`).
fn qualified_name_text(class_type: &GrammarASTNode) -> Option<String> {
    let qualified = class_type.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(n) if n.rule_name == "qualified_name" => Some(n),
        _ => None,
    })?;
    let names: Vec<&str> = qualified
        .children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.type_ == lexer::token::TokenType::Name => {
                Some(t.value.as_str())
            }
            _ => None,
        })
        .collect();
    if names.is_empty() {
        None
    } else {
        Some(names.join("."))
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
