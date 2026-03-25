//! # Lattice AST transformer — expands Lattice constructs into pure CSS.
//!
//! This is the core of the Lattice-to-CSS compiler. It takes a mixed Lattice
//! AST (containing both CSS and Lattice nodes) and produces a clean CSS AST
//! (containing only CSS nodes) by expanding all Lattice constructs.
//!
//! # Three-Pass Architecture
//!
//! ## Pass 1: Symbol Collection
//!
//! Walk the top-level AST and collect:
//! - Variable declarations → variable registry in the global scope
//! - Mixin definitions → mixin registry
//! - Function definitions → function registry
//!
//! Definitions are removed from the AST (they produce no CSS output).
//! This pass runs BEFORE expansion so that mixins and functions can be
//! defined AFTER they are used:
//!
//! ```text
//! .btn { @include button(red); }    ← used first
//! @mixin button($bg) { ... }        ← defined later — works because Pass 1
//! ```                                  collects definitions before Pass 2 runs
//!
//! ## Pass 2: Expansion
//!
//! Recursively walk remaining AST nodes and expand Lattice constructs:
//! - VARIABLE tokens → substitute resolved values
//! - `@include` directives → clone and expand mixin bodies
//! - `@if`/`@else` → evaluate condition, keep matching branch
//! - `@for` → generate one copy of the body per iteration
//! - `@each` → generate one copy of the body per list item
//! - Function calls → evaluate and replace with return value
//!
//! After this pass, the AST contains only CSS-valid constructs.
//!
//! ## Pass 3: Cleanup
//!
//! Remove empty blocks, None children, and other artifacts that result
//! from expansion (e.g., an `@if` that matched no branch leaves nothing).
//!
//! # Cycle Detection
//!
//! Mixin and function expansion track a call stack. If a name appears
//! twice, a `CircularReference` error is raised.
//!
//! # CSS Built-in Functions
//!
//! CSS functions like `rgb()`, `calc()`, `var()` are NOT Lattice functions.
//! They are passed through unchanged. Only functions defined with `@function`
//! in the source are evaluated.

use std::collections::HashMap;

use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use lexer::token::{Token, TokenType};

use crate::errors::LatticeError;
use crate::evaluator::{ExpressionEvaluator, get_token_type_name, is_builtin_function, evaluate_builtin};
use crate::scope::{ScopeChain, ScopeValue};
use crate::values::LatticeValue;

// ===========================================================================
// CSS built-in function names
// ===========================================================================

/// CSS built-in functions that should NOT be resolved as Lattice functions.
///
/// When a `function_call` node's function name matches one of these, the
/// entire function call is passed through to CSS output unchanged (with
/// its arguments expanded for variable substitution only).
///
/// This list covers CSS Color, Math, Transform, Filter, Grid, and Gradient
/// functions. When adding new CSS functions, add them here — NOT to the
/// grammar.
fn is_css_function(name: &str) -> bool {
    // Strip trailing "(" if present (FUNCTION token includes it)
    let clean = name.trim_end_matches('(');
    CSS_FUNCTION_NAMES.contains(&clean)
}

/// All CSS built-in function names as a constant slice.
///
/// We use a const slice instead of a HashSet to avoid runtime allocation
/// and external dependencies. The slice is searched linearly — with ~60
/// entries this is fast enough for compile-time processing.
const CSS_FUNCTION_NAMES: &[&str] = &[
    // Color
    "rgb", "rgba", "hsl", "hsla", "hwb", "lab", "lch", "oklch", "oklab",
    "color", "color-mix",
    // Math
    "calc", "min", "max", "clamp", "abs", "sign", "round", "mod", "rem",
    "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "pow", "sqrt",
    "hypot", "log", "exp",
    // Custom properties / environment
    "var", "env",
    // Content
    "url", "format", "local",
    // Gradients
    "linear-gradient", "radial-gradient", "conic-gradient",
    "repeating-linear-gradient", "repeating-radial-gradient",
    "repeating-conic-gradient",
    // Misc
    "counter", "counters", "attr", "element",
    // Transforms
    "translate", "translateX", "translateY", "translateZ",
    "rotate", "rotateX", "rotateY", "rotateZ",
    "scale", "scaleX", "scaleY", "scaleZ",
    "skew", "skewX", "skewY",
    "matrix", "matrix3d", "perspective",
    // Timing
    "cubic-bezier", "steps",
    // Shapes
    "path", "polygon", "circle", "ellipse", "inset",
    // Images
    "image-set", "cross-fade",
    // Grid
    "fit-content", "minmax", "repeat",
    // Filters
    "blur", "brightness", "contrast", "drop-shadow", "grayscale",
    "hue-rotate", "invert", "opacity", "saturate", "sepia",
];

// ===========================================================================
// Mixin and Function Definition Records
// ===========================================================================

/// A stored `@mixin` definition.
///
/// Captures the parameter list, default values, and the body block AST node.
/// The body is stored by value and deep-copied each time the mixin is expanded,
/// so multiple `@include` calls each get their own independent copy.
#[derive(Debug, Clone)]
pub struct MixinDef {
    pub name: String,
    /// Parameter names in order, e.g. `["$bg", "$fg"]`
    pub params: Vec<String>,
    /// Parameter names that have default values → the default CSS text
    pub defaults: HashMap<String, String>,
    /// The `block` AST node (LBRACE block_contents RBRACE)
    pub body: GrammarASTNode,
}

/// A stored `@function` definition.
///
/// Same structure as `MixinDef`, but the body is a `function_body` node
/// (different grammar rule) and functions return values via `@return` instead
/// of emitting declarations.
#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub name: String,
    pub params: Vec<String>,
    pub defaults: HashMap<String, String>,
    /// The `function_body` AST node
    pub body: GrammarASTNode,
}

// ===========================================================================
// Transformer
// ===========================================================================

/// Transforms a Lattice AST into a clean CSS AST.
///
/// Create one transformer per stylesheet. Call `transform(ast)` to run the
/// three-pass pipeline and get back a CSS-only AST.
/// Maximum number of iterations allowed in a @while loop (Lattice v2).
const MAX_WHILE_ITERATIONS: usize = 1000;

pub struct LatticeTransformer {
    /// Global variable scope (populated in Pass 1)
    pub variables: ScopeChain,
    /// Registered mixins (populated in Pass 1)
    pub mixins: HashMap<String, MixinDef>,
    /// Registered functions (populated in Pass 1)
    pub functions: HashMap<String, FunctionDef>,
    /// Mixin call stack for cycle detection
    mixin_stack: Vec<String>,
    /// Function call stack for cycle detection
    function_stack: Vec<String>,
    /// Maximum @while iterations (Lattice v2)
    max_while_iterations: usize,
    /// @extend tracking: maps target selector → list of extending selectors (v2)
    extend_map: HashMap<String, Vec<String>>,
    /// @at-root hoisted rules collected during expansion (v2)
    at_root_rules: Vec<GrammarASTNode>,
    /// @content block stack for mixin content blocks (v2)
    content_block_stack: Vec<Option<GrammarASTNode>>,
    /// Scope stack for @content evaluation in the caller's scope (v2)
    content_scope_stack: Vec<ScopeChain>,
    /// Current selector context — tracks the selector path for @extend (v2)
    current_selector: Option<String>,
}

impl LatticeTransformer {
    /// Create a new transformer with empty registries.
    pub fn new() -> Self {
        LatticeTransformer {
            variables: ScopeChain::new(),
            mixins: HashMap::new(),
            functions: HashMap::new(),
            mixin_stack: Vec::new(),
            function_stack: Vec::new(),
            max_while_iterations: MAX_WHILE_ITERATIONS,
            extend_map: HashMap::new(),
            at_root_rules: Vec::new(),
            content_block_stack: Vec::new(),
            content_scope_stack: Vec::new(),
            current_selector: None,
        }
    }

    /// Run the three-pass transformation pipeline.
    ///
    /// Returns a CSS-only AST with all Lattice constructs expanded.
    pub fn transform(&mut self, ast: GrammarASTNode) -> Result<GrammarASTNode, LatticeError> {
        // Pass 1: Collect all symbol definitions (variables, mixins, functions)
        let mut ast = self.collect_symbols(ast)?;

        // Pass 2: Expand all remaining nodes (substitution, mixin calls, control flow)
        ast = self.expand_node(ast, &self.variables.clone())?;

        // Pass 3: Cleanup — remove empty nodes
        ast = self.cleanup(ast);

        // Lattice v2: Apply @extend selector merging
        if !self.extend_map.is_empty() {
            self.apply_extends(&mut ast);
        }

        // Lattice v2: Splice @at-root hoisted rules at the top level
        if !self.at_root_rules.is_empty() {
            let hoisted = std::mem::take(&mut self.at_root_rules);
            for rule in hoisted {
                ast.children.push(ASTNodeOrToken::Node(rule));
            }
        }

        Ok(ast)
    }

    // =========================================================================
    // Pass 1: Symbol Collection
    // =========================================================================

    fn collect_symbols(&mut self, mut ast: GrammarASTNode) -> Result<GrammarASTNode, LatticeError> {
        let mut new_children: Vec<ASTNodeOrToken> = Vec::new();

        for child in ast.children.drain(..) {
            match &child {
                ASTNodeOrToken::Node(node) if node.rule_name == "rule" => {
                    // Peek into the rule to see if it's a lattice definition
                    if let Some(inner) = get_first_node_child(node) {
                        if inner.rule_name == "lattice_rule" {
                            if let Some(lattice_child) = get_first_node_child(inner) {
                                match lattice_child.rule_name.as_str() {
                                    "variable_declaration" => {
                                        self.collect_variable(lattice_child)?;
                                        // Do NOT add to output — variable defs produce no CSS
                                        continue;
                                    }
                                    "mixin_definition" => {
                                        self.collect_mixin(lattice_child)?;
                                        continue;
                                    }
                                    "function_definition" => {
                                        self.collect_function(lattice_child)?;
                                        continue;
                                    }
                                    "use_directive" => {
                                        // @use is not fully implemented — skip silently
                                        continue;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    new_children.push(child);
                }
                _ => {
                    new_children.push(child);
                }
            }
        }

        ast.children = new_children;
        Ok(ast)
    }

    fn collect_variable(&mut self, node: &GrammarASTNode) -> Result<(), LatticeError> {
        // variable_declaration = VARIABLE COLON value_list { variable_flag } SEMICOLON ;
        // Lattice v2: supports !default and !global flags
        let mut name: Option<String> = None;
        let mut css_value: Option<String> = None;
        let mut is_default = false;
        let mut is_global = false;

        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) => {
                    let type_name = get_token_type_name(tok);
                    if type_name == "VARIABLE" {
                        name = Some(tok.value.clone());
                    } else if type_name == "BANG_DEFAULT" || tok.value == "!default" {
                        is_default = true;
                    } else if type_name == "BANG_GLOBAL" || tok.value == "!global" {
                        is_global = true;
                    }
                }
                ASTNodeOrToken::Node(n) => {
                    if n.rule_name == "value_list" {
                        css_value = Some(emit_raw_node(n));
                    } else if n.rule_name == "variable_flag" {
                        for fc in &n.children {
                            if let ASTNodeOrToken::Token(ft) = fc {
                                let ft_name = get_token_type_name(ft);
                                if ft_name == "BANG_DEFAULT" || ft.value == "!default" {
                                    is_default = true;
                                } else if ft_name == "BANG_GLOBAL" || ft.value == "!global" {
                                    is_global = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        if let (Some(n), Some(v)) = (name, css_value) {
            if is_default && is_global {
                // Check global scope only — if not defined, set globally
                if !self.variables.has(&n) {
                    self.variables.set(n, ScopeValue::Raw(v));
                }
            } else if is_default {
                // Only set if not already defined
                if !self.variables.has(&n) {
                    self.variables.set(n, ScopeValue::Raw(v));
                }
            } else if is_global {
                // Always set in global scope
                self.variables.set(n, ScopeValue::Raw(v));
            } else {
                self.variables.set(n, ScopeValue::Raw(v));
            }
        }
        Ok(())
    }

    fn collect_mixin(&mut self, node: &GrammarASTNode) -> Result<(), LatticeError> {
        // mixin_definition = "@mixin" FUNCTION [ mixin_params ] RPAREN block ;
        let mut name: Option<String> = None;
        let mut params: Vec<String> = Vec::new();
        let mut defaults: HashMap<String, String> = HashMap::new();
        let mut body: Option<GrammarASTNode> = None;

        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) if get_token_type_name(tok) == "FUNCTION" => {
                    name = Some(tok.value.trim_end_matches('(').to_string());
                }
                ASTNodeOrToken::Node(n) if n.rule_name == "mixin_params" => {
                    let (p, d) = extract_params(n);
                    params = p;
                    defaults = d;
                }
                ASTNodeOrToken::Node(n) if n.rule_name == "block" => {
                    body = Some(n.clone());
                }
                _ => {}
            }
        }

        if let (Some(n), Some(b)) = (name, body) {
            self.mixins.insert(n.clone(), MixinDef { name: n, params, defaults, body: b });
        }
        Ok(())
    }

    fn collect_function(&mut self, node: &GrammarASTNode) -> Result<(), LatticeError> {
        // function_definition = "@function" FUNCTION [ mixin_params ] RPAREN function_body ;
        let mut name: Option<String> = None;
        let mut params: Vec<String> = Vec::new();
        let mut defaults: HashMap<String, String> = HashMap::new();
        let mut body: Option<GrammarASTNode> = None;

        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) if get_token_type_name(tok) == "FUNCTION" => {
                    name = Some(tok.value.trim_end_matches('(').to_string());
                }
                ASTNodeOrToken::Node(n) if n.rule_name == "mixin_params" => {
                    let (p, d) = extract_params(n);
                    params = p;
                    defaults = d;
                }
                ASTNodeOrToken::Node(n) if n.rule_name == "function_body" => {
                    body = Some(n.clone());
                }
                _ => {}
            }
        }

        if let (Some(n), Some(b)) = (name, body) {
            self.functions.insert(n.clone(), FunctionDef { name: n, params, defaults, body: b });
        }
        Ok(())
    }

    // =========================================================================
    // Pass 2: Expansion
    // =========================================================================

    fn expand_node(
        &mut self,
        node: GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<GrammarASTNode, LatticeError> {
        match node.rule_name.as_str() {
            // The stylesheet level needs to handle top-level control flow
            // (@if/@for/@each) which can expand to multiple rule nodes.
            "stylesheet" => self.expand_stylesheet(node, scope),
            "block" => self.expand_block(node, scope),
            "block_contents" => self.expand_block_contents(node, scope),
            "block_item" => self.expand_block_item(node, scope),
            "value_list" => self.expand_value_list(node, scope),
            "value" => self.expand_value_node(node, scope),
            "function_call" => self.expand_function_call(node, scope),
            "function_args" | "function_arg" => self.expand_children(node, scope),
            // Top-level rule wrapper: check for lattice_rule containing control flow
            "rule" => self.expand_rule_node(node, scope),
            // Lattice v2: resolve variables in selector positions
            "compound_selector" | "simple_selector" | "class_selector" => {
                self.expand_selector_with_vars(node, scope)
            }
            _ => self.expand_children(node, scope),
        }
    }

    /// Expand the top-level stylesheet node.
    ///
    /// Like `expand_block_contents`, a stylesheet can contain top-level
    /// control flow (`@if`/`@for`/`@each`) that expands to *multiple* rules.
    /// We iterate over the stylesheet's `rule` children and splice in the
    /// expanded output for any such constructs.
    fn expand_stylesheet(
        &mut self,
        mut node: GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<GrammarASTNode, LatticeError> {
        let mut new_children: Vec<ASTNodeOrToken> = Vec::new();

        for child in node.children.drain(..) {
            match child {
                ASTNodeOrToken::Node(rule_node) if rule_node.rule_name == "rule" => {
                    let items = self.expand_rule_node_to_vec(rule_node, scope)?;
                    for item in items {
                        new_children.push(ASTNodeOrToken::Node(item));
                    }
                }
                ASTNodeOrToken::Node(n) => {
                    let expanded = self.expand_node(n, scope)?;
                    new_children.push(ASTNodeOrToken::Node(expanded));
                }
                tok => new_children.push(tok),
            }
        }

        node.children = new_children;
        Ok(node)
    }

    /// Expand a top-level `rule` node, returning 0-or-more result nodes.
    ///
    /// A `rule` wrapping `lattice_rule → lattice_control` (e.g. `@for`) can
    /// produce many output rules. A CSS `qualified_rule` or `at_rule` always
    /// produces exactly one output rule.
    fn expand_rule_node_to_vec(
        &mut self,
        node: GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<Vec<GrammarASTNode>, LatticeError> {
        // Look into rule → lattice_rule → lattice_control
        if let Some(ASTNodeOrToken::Node(inner)) = node.children.first() {
            if inner.rule_name == "lattice_rule" {
                if let Some(ASTNodeOrToken::Node(lattice_child)) = inner.children.first() {
                    if lattice_child.rule_name == "lattice_control" {
                        let expanded = self.expand_control(lattice_child.clone(), scope)?;
                        return Ok(expanded);
                    }
                    // variable_declaration, mixin_definition, etc. were already
                    // removed in Pass 1 — but handle them defensively here
                    match lattice_child.rule_name.as_str() {
                        "variable_declaration" | "mixin_definition"
                        | "function_definition" | "use_directive" => {
                            return Ok(vec![]);
                        }
                        _ => {}
                    }
                }
            }
        }

        // Default: expand the rule as a normal CSS node (qualified_rule, at_rule).
        // Call expand_children directly to avoid re-entering expand_rule_node.
        let expanded = self.expand_children(node, scope)?;
        Ok(vec![expanded])
    }

    /// Expand a top-level `rule` node, returning a single result.
    ///
    /// Used when `expand_node` is called on a `rule` and we need to return
    /// exactly one node. If the rule expands to multiple nodes (control flow),
    /// we wrap them in a synthetic container.
    fn expand_rule_node(
        &mut self,
        node: GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<GrammarASTNode, LatticeError> {
        let items = self.expand_rule_node_to_vec(node.clone(), scope)?;
        if items.is_empty() {
            return Ok(GrammarASTNode { rule_name: "rule".to_string(), children: vec![] });
        }
        if items.len() == 1 {
            return Ok(items.into_iter().next().unwrap());
        }
        // Multiple rules: wrap in a synthetic "stylesheet_fragment" node
        // (The cleanup pass will flatten these)
        Ok(GrammarASTNode {
            rule_name: "stylesheet_fragment".to_string(),
            children: items.into_iter().map(ASTNodeOrToken::Node).collect(),
        })
    }

    /// Expand all children of a node.
    fn expand_children(
        &mut self,
        mut node: GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<GrammarASTNode, LatticeError> {
        let mut new_children: Vec<ASTNodeOrToken> = Vec::new();

        for child in node.children.drain(..) {
            match child {
                ASTNodeOrToken::Token(tok) => {
                    if get_token_type_name(&tok) == "VARIABLE" {
                        let substituted = self.substitute_variable(&tok, scope)?;
                        new_children.push(substituted);
                    } else {
                        new_children.push(ASTNodeOrToken::Token(tok));
                    }
                }
                ASTNodeOrToken::Node(n) => {
                    let expanded = self.expand_node(n, scope)?;
                    new_children.push(ASTNodeOrToken::Node(expanded));
                }
            }
        }

        node.children = new_children;
        Ok(node)
    }

    /// Substitute a VARIABLE token with its resolved value.
    fn substitute_variable(
        &self,
        token: &Token,
        scope: &ScopeChain,
    ) -> Result<ASTNodeOrToken, LatticeError> {
        let name = &token.value;

        match scope.get(name) {
            Some(scope_val) => {
                let css_text = scope_val.to_css_text();
                // Create a synthetic IDENT token with the resolved value
                let new_token = make_synthetic_token(&css_text, token);
                Ok(ASTNodeOrToken::Token(new_token))
            }
            None => {
                // Variable not found — emit an error
                Err(LatticeError::undefined_variable(
                    name,
                    token.line,
                    token.column,
                ))
            }
        }
    }

    /// Expand a block, creating a child scope for its contents.
    fn expand_block(
        &mut self,
        node: GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<GrammarASTNode, LatticeError> {
        let child_scope = scope.child();
        self.expand_children(node, &child_scope)
    }

    /// Expand block_contents — handles Lattice block items specially.
    fn expand_block_contents(
        &mut self,
        mut node: GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<GrammarASTNode, LatticeError> {
        let mut new_children: Vec<ASTNodeOrToken> = Vec::new();
        let mut local_scope = scope.child();

        for child in node.children.drain(..) {
            match child {
                ASTNodeOrToken::Node(n) if n.rule_name == "block_item" => {
                    let results = self.expand_block_item_to_vec(n, &mut local_scope)?;
                    for item in results {
                        new_children.push(ASTNodeOrToken::Node(item));
                    }
                }
                other => {
                    new_children.push(other);
                }
            }
        }

        node.children = new_children;
        Ok(node)
    }

    /// Expand a block_item node, returning 0 or more replacement items.
    fn expand_block_item(
        &mut self,
        node: GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<GrammarASTNode, LatticeError> {
        // Delegate to the vector version and return only the node itself
        let mut scope_copy = scope.clone();
        let items = self.expand_block_item_to_vec(node.clone(), &mut scope_copy)?;
        if items.is_empty() {
            // Return an empty block_item (will be cleaned up in Pass 3)
            return Ok(GrammarASTNode { rule_name: "block_item".to_string(), children: vec![] });
        }
        // If there's exactly one result matching the original, return it
        if items.len() == 1 {
            return Ok(items.into_iter().next().unwrap());
        }
        // Multiple items: wrap in a synthetic node (handled by caller)
        Ok(items.into_iter().next().unwrap_or(node))
    }

    /// Expand a block_item, returning potentially multiple replacement items.
    fn expand_block_item_to_vec(
        &mut self,
        node: GrammarASTNode,
        scope: &mut ScopeChain,
    ) -> Result<Vec<GrammarASTNode>, LatticeError> {
        if node.children.is_empty() {
            return Ok(vec![]);
        }

        if let Some(ASTNodeOrToken::Node(inner)) = node.children.first() {
            match inner.rule_name.as_str() {
                "lattice_block_item" => {
                    return self.expand_lattice_block_item(inner.clone(), scope);
                }
                _ => {}
            }
        }

        // Default: expand children and return as-is
        let expanded = self.expand_children(node, scope)?;
        Ok(vec![expanded])
    }

    /// Expand a lattice_block_item, which can be a variable declaration,
    /// @include, @content, @at-root, @extend, or control flow.
    ///
    /// Lattice v2 adds support for content_directive, at_root_directive,
    /// and extend_directive.
    fn expand_lattice_block_item(
        &mut self,
        node: GrammarASTNode,
        scope: &mut ScopeChain,
    ) -> Result<Vec<GrammarASTNode>, LatticeError> {
        if let Some(ASTNodeOrToken::Node(inner)) = node.children.first() {
            match inner.rule_name.as_str() {
                "variable_declaration" => {
                    self.expand_variable_declaration(inner.clone(), scope)?;
                    return Ok(vec![]); // Produces no CSS output
                }
                "include_directive" => {
                    return self.expand_include(inner.clone(), scope);
                }
                "lattice_control" => {
                    return self.expand_control(inner.clone(), scope);
                }
                // Lattice v2: @content
                "content_directive" => {
                    return self.expand_content_directive(scope);
                }
                // Lattice v2: @at-root
                "at_root_directive" => {
                    return self.expand_at_root(inner.clone(), scope);
                }
                // Lattice v2: @extend
                "extend_directive" => {
                    self.expand_extend(inner, scope);
                    return Ok(vec![]);
                }
                _ => {}
            }
        }
        Ok(vec![node])
    }

    // =========================================================================
    // @content Expansion (Lattice v2)
    // =========================================================================

    fn expand_content_directive(
        &mut self,
        _scope: &ScopeChain,
    ) -> Result<Vec<GrammarASTNode>, LatticeError> {
        // @content is replaced with the content block passed to @include.
        // If no content block was passed, produce nothing.
        if let Some(Some(content_block)) = self.content_block_stack.last().cloned() {
            let content_scope = self.content_scope_stack.last().cloned()
                .unwrap_or_else(ScopeChain::new);
            let expanded = self.expand_node(content_block, &content_scope)?;
            Ok(extract_block_contents_items(&expanded))
        } else {
            Ok(vec![])
        }
    }

    // =========================================================================
    // @at-root Expansion (Lattice v2)
    // =========================================================================

    fn expand_at_root(
        &mut self,
        node: GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<Vec<GrammarASTNode>, LatticeError> {
        // at_root_directive = "@at-root" ( block | selector_list block ) ;
        // Collect the block children and hoist them to the root.
        for child in &node.children {
            if let ASTNodeOrToken::Node(n) = child {
                if n.rule_name == "block" {
                    let expanded = self.expand_node(n.clone(), scope)?;
                    let items = extract_block_contents_items(&expanded);
                    self.at_root_rules.extend(items);
                }
            }
        }
        Ok(vec![])
    }

    // =========================================================================
    // @extend Expansion (Lattice v2)
    // =========================================================================

    fn expand_extend(
        &mut self,
        node: &GrammarASTNode,
        _scope: &ScopeChain,
    ) {
        // extend_directive = "@extend" extend_target SEMICOLON ;
        // extract_target could be PLACEHOLDER, DOT IDENT, or IDENT
        let mut target = String::new();
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) => {
                    let type_name = get_token_type_name(tok);
                    if type_name == "PLACEHOLDER" || type_name == "Dot"
                        || type_name == "Ident" || type_name == "IDENT" {
                        target.push_str(&tok.value);
                    }
                }
                ASTNodeOrToken::Node(n) if n.rule_name == "extend_target" => {
                    target = emit_raw_node(n).trim().to_string();
                }
                _ => {}
            }
        }

        if !target.is_empty() {
            let extending = self.current_selector.clone().unwrap_or_default();
            self.extend_map
                .entry(target)
                .or_default()
                .push(extending);
        }
    }

    /// Apply @extend selector merging in Pass 3 (Lattice v2).
    ///
    /// For each rule in the AST, check if its selector matches an extend target.
    /// If so, append the extending selectors to the rule's selector list.
    /// Also remove placeholder-only rules.
    fn apply_extends(&self, ast: &mut GrammarASTNode) {
        // Simple implementation: walk all qualified_rule nodes and check selectors
        let mut new_children: Vec<ASTNodeOrToken> = Vec::new();
        let mut children = std::mem::take(&mut ast.children);

        for child in children.drain(..) {
            match child {
                ASTNodeOrToken::Node(mut n) => {
                    if n.rule_name == "rule" || n.rule_name == "qualified_rule" {
                        // Check if this rule's selector matches an extend target
                        let selector_text = self.extract_selector_text(&n);
                        let is_placeholder = selector_text.starts_with('%');

                        // Check for extend matches
                        let mut extended = false;
                        for (target, extenders) in &self.extend_map {
                            if selector_text.contains(target.as_str()) {
                                // Add extending selectors to this rule
                                self.add_selectors_to_rule(&mut n, extenders);
                                extended = true;
                            }
                        }

                        // Remove placeholder-only rules
                        if is_placeholder && !extended {
                            continue; // Skip this rule
                        }
                        if is_placeholder {
                            // Remove the placeholder selector part but keep extended selectors
                            // For simplicity, keep the rule with modified selectors
                        }
                    }
                    new_children.push(ASTNodeOrToken::Node(n));
                }
                other => new_children.push(other),
            }
        }

        ast.children = new_children;
    }

    /// Extract the selector text from a rule node.
    fn extract_selector_text(&self, node: &GrammarASTNode) -> String {
        for child in &node.children {
            if let ASTNodeOrToken::Node(n) = child {
                if n.rule_name == "selector_list" || n.rule_name == "complex_selector"
                    || n.rule_name == "compound_selector" || n.rule_name == "qualified_rule" {
                    return emit_raw_node(n);
                }
            }
        }
        // Fallback: look into inner nodes
        if let Some(ASTNodeOrToken::Node(inner)) = node.children.first() {
            return self.extract_selector_text(inner);
        }
        String::new()
    }

    /// Add extending selectors to a rule's selector list.
    fn add_selectors_to_rule(&self, node: &mut GrammarASTNode, selectors: &[String]) {
        // Find the selector_list and append new selectors
        for child in &mut node.children {
            if let ASTNodeOrToken::Node(n) = child {
                if n.rule_name == "qualified_rule" {
                    self.add_selectors_to_rule(n, selectors);
                    return;
                }
                if n.rule_name == "selector_list" {
                    for sel in selectors {
                        if !sel.is_empty() {
                            // Add comma + new selector tokens
                            n.children.push(ASTNodeOrToken::Token(Token {
                                type_: TokenType::Name,
                                type_name: Some("Comma".to_string()),
                                value: ",".to_string(),
                                line: 0, column: 0,
                            }));
                            n.children.push(ASTNodeOrToken::Token(Token {
                                type_: TokenType::Name,
                                type_name: None,
                                value: format!(" {}", sel),
                                line: 0, column: 0,
                            }));
                        }
                    }
                    return;
                }
            }
        }
    }

    /// Process a variable declaration inside a block — sets the variable in scope.
    ///
    /// Lattice v2: handles !default and !global flags.
    fn expand_variable_declaration(
        &mut self,
        node: GrammarASTNode,
        scope: &mut ScopeChain,
    ) -> Result<(), LatticeError> {
        let mut var_name: Option<String> = None;
        let mut value_text: Option<String> = None;
        let mut is_default = false;
        let mut is_global = false;

        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) => {
                    let type_name = get_token_type_name(tok);
                    if type_name == "VARIABLE" {
                        var_name = Some(tok.value.clone());
                    } else if type_name == "BANG_DEFAULT" || tok.value == "!default" {
                        is_default = true;
                    } else if type_name == "BANG_GLOBAL" || tok.value == "!global" {
                        is_global = true;
                    }
                }
                ASTNodeOrToken::Node(n) => {
                    if n.rule_name == "value_list" {
                        let expanded = self.expand_value_list(n.clone(), scope)?;
                        // Try to evaluate as an expression (e.g. $i + 1 → 2).
                        // This is critical for @while loops: without it,
                        // $i: $i + 1 stores "1 + 1" instead of "2", causing
                        // the loop condition to never change and looping forever.
                        let evaluator = ExpressionEvaluator::new(scope);
                        if let Ok(evaluated) = evaluator.evaluate_node(&expanded) {
                            value_text = Some(evaluated.to_css_string());
                        } else {
                            value_text = Some(emit_raw_node(&expanded));
                        }
                    } else if n.rule_name == "variable_flag" {
                        for fc in &n.children {
                            if let ASTNodeOrToken::Token(ft) = fc {
                                let ft_name = get_token_type_name(ft);
                                if ft_name == "BANG_DEFAULT" || ft.value == "!default" {
                                    is_default = true;
                                } else if ft_name == "BANG_GLOBAL" || ft.value == "!global" {
                                    is_global = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        if let (Some(name), Some(value)) = (var_name, value_text) {
            if is_default && is_global {
                if !self.variables.has(&name) {
                    self.variables.set(name, ScopeValue::Raw(value));
                }
            } else if is_default {
                if !scope.has(&name) {
                    scope.set(name, ScopeValue::Raw(value));
                }
            } else if is_global {
                self.variables.set(name, ScopeValue::Raw(value));
            } else {
                scope.set(name, ScopeValue::Raw(value));
            }
        }
        Ok(())
    }

    /// Expand a value_list, substituting variables.
    fn expand_value_list(
        &mut self,
        mut node: GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<GrammarASTNode, LatticeError> {
        let mut new_children: Vec<ASTNodeOrToken> = Vec::new();

        for child in node.children.drain(..) {
            match child {
                ASTNodeOrToken::Token(tok) if get_token_type_name(&tok) == "VARIABLE" => {
                    let substituted = self.substitute_variable(&tok, scope)?;
                    new_children.push(substituted);
                }
                ASTNodeOrToken::Node(n) => {
                    let expanded = self.expand_node(n, scope)?;
                    new_children.push(ASTNodeOrToken::Node(expanded));
                }
                other => new_children.push(other),
            }
        }

        node.children = new_children;
        Ok(node)
    }

    /// Expand a `value` node.
    fn expand_value_node(
        &mut self,
        mut node: GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<GrammarASTNode, LatticeError> {
        if node.children.len() == 1 {
            if let Some(ASTNodeOrToken::Token(tok)) = node.children.first() {
                if get_token_type_name(tok) == "VARIABLE" {
                    let tok_clone = tok.clone();
                    let substituted = self.substitute_variable(&tok_clone, scope)?;
                    node.children = vec![substituted];
                    return Ok(node);
                }
            }
        }
        self.expand_children(node, scope)
    }

    /// Expand a function call — either CSS built-in (pass through) or Lattice.
    fn expand_function_call(
        &mut self,
        node: GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<GrammarASTNode, LatticeError> {
        // Find the FUNCTION token to determine the name
        let func_name = node.children.iter()
            .find_map(|c| {
                if let ASTNodeOrToken::Token(tok) = c {
                    if get_token_type_name(tok) == "FUNCTION" {
                        return Some(tok.value.trim_end_matches('(').to_string());
                    }
                }
                None
            });

        let Some(func_name) = func_name else {
            // URL_TOKEN or other — pass through
            return self.expand_children(node, scope);
        };

        // User-defined function ALWAYS takes priority — even over CSS built-ins
        // like scale(), translate(), etc. If the user defines @function scale(),
        // their definition wins. This matches Sass behavior.
        if self.functions.contains_key(&func_name) {
            return self.evaluate_function_call(&func_name.clone(), node, scope);
        }

        // CSS built-in that is NOT also a Lattice built-in — pass through
        if is_css_function(&func_name) && !is_builtin_function(&func_name) {
            return self.expand_children(node, scope);
        }

        // Lattice v2: Built-in function evaluation
        if is_builtin_function(&func_name) {
            return self.evaluate_builtin_function_call(&func_name, node, scope);
        }

        // CSS built-in that overlaps with Lattice built-in names
        if is_css_function(&func_name) {
            return self.expand_children(node, scope);
        }

        // Unknown function: pass through
        self.expand_children(node, scope)
    }

    // =========================================================================
    // @include Expansion
    // =========================================================================

    fn expand_include(
        &mut self,
        node: GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<Vec<GrammarASTNode>, LatticeError> {
        // include_directive = "@include" FUNCTION [ include_args ] RPAREN ( SEMICOLON | block )
        //                   | "@include" IDENT ( SEMICOLON | block )
        // Lattice v2: the trailing block (if present) is a @content block.

        let mut mixin_name: Option<String> = None;
        let mut args_node: Option<GrammarASTNode> = None;
        let mut content_block: Option<GrammarASTNode> = None;

        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) => {
                    let type_name = get_token_type_name(tok);
                    match type_name.as_str() {
                        "FUNCTION" => {
                            mixin_name = Some(tok.value.trim_end_matches('(').to_string());
                        }
                        "Ident" | "IDENT" => {
                            if mixin_name.is_none() {
                                mixin_name = Some(tok.value.clone());
                            }
                        }
                        _ => {}
                    }
                }
                ASTNodeOrToken::Node(n) if n.rule_name == "include_args" => {
                    args_node = Some(n.clone());
                }
                // Lattice v2: content block for @content
                ASTNodeOrToken::Node(n) if n.rule_name == "block" => {
                    content_block = Some(n.clone());
                }
                _ => {}
            }
        }

        let mixin_name = match mixin_name {
            Some(n) => n,
            None => return Ok(vec![]),
        };

        if !self.mixins.contains_key(&mixin_name) {
            return Err(LatticeError::undefined_mixin(&mixin_name, 0, 0));
        }

        // Cycle detection
        if self.mixin_stack.contains(&mixin_name) {
            let mut chain = self.mixin_stack.clone();
            chain.push(mixin_name.clone());
            return Err(LatticeError::circular_reference("mixin", chain, 0, 0));
        }

        let mixin_def = self.mixins[&mixin_name].clone();

        // Bug #2: Parse arguments — now returns (positional, named) split.
        // Bug #3: Named and positional args are pre-evaluated in the CALLER'S scope
        //         before being bound into the mixin scope. This prevents infinite
        //         recursion when the mixin parameter name shadows a caller variable
        //         of the same name (e.g. `@include foo($color: $color)`).
        let (positional, named) = if let Some(a) = args_node {
            self.parse_include_args_named(a, scope)?
        } else {
            (vec![], HashMap::new())
        };

        // Arity check: count only args that aren't covered by named args
        let total_provided = positional.len() + named.len();
        let required = mixin_def.params.len() - mixin_def.defaults.len();
        if total_provided < required || total_provided > mixin_def.params.len() {
            return Err(LatticeError::wrong_arity(
                "Mixin", &mixin_name,
                mixin_def.params.len(), total_provided,
                0, 0,
            ));
        }

        // Build the mixin scope: named args take priority, then positional, then defaults.
        // Bug #3: each arg value was already evaluated in the caller's scope inside
        //         parse_include_args_named, so we just store the result string here.
        let mut mixin_scope = scope.child();
        let mut pos_idx = 0usize;
        for param_name in &mixin_def.params {
            if let Some(val) = named.get(param_name) {
                mixin_scope.set(param_name.clone(), ScopeValue::Raw(val.clone()));
            } else if pos_idx < positional.len() {
                mixin_scope.set(param_name.clone(), ScopeValue::Raw(positional[pos_idx].clone()));
                pos_idx += 1;
            } else if let Some(default) = mixin_def.defaults.get(param_name) {
                mixin_scope.set(param_name.clone(), ScopeValue::Raw(default.clone()));
            }
        }

        // Lattice v2: push @content block and caller scope for @content expansion
        self.content_block_stack.push(content_block);
        self.content_scope_stack.push(scope.clone());

        // Expand the mixin body
        self.mixin_stack.push(mixin_name.clone());
        let result = (|| -> Result<Vec<GrammarASTNode>, LatticeError> {
            let body = mixin_def.body.clone();
            let expanded = self.expand_node(body, &mixin_scope)?;

            // Extract block_contents children
            let items = extract_block_contents_items(&expanded);
            Ok(items)
        })();
        self.mixin_stack.pop();

        // Lattice v2: pop @content block and scope
        self.content_block_stack.pop();
        self.content_scope_stack.pop();

        result
    }

    /// Parse include arguments, returning (positional_args, named_args).
    ///
    /// Bug #2: handles the updated grammar where include_args may contain
    /// `include_arg` nodes of the form `VARIABLE COLON value_list` (named) or
    /// just `value_list` (positional).
    ///
    /// Bug #3: each argument value is evaluated in the CALLER'S scope before
    /// being returned, so that mixin parameter names cannot shadow caller
    /// variable names during argument evaluation (prevents infinite recursion).
    fn parse_include_args_named(
        &mut self,
        node: GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<(Vec<String>, HashMap<String, String>), LatticeError> {
        let mut positional: Vec<String> = Vec::new();
        let mut named: HashMap<String, String> = HashMap::new();

        // Helper: expand a value_list node in the caller scope and return its CSS text.
        // This is Bug #3 — we evaluate in the CALLER scope (passed as `scope`) so that
        // if the mixin param name matches a caller variable (e.g. $color: $color),
        // the right-hand side $color resolves to the caller's value, not to the param.
        let evaluate_arg_node = |transformer: &mut LatticeTransformer, n: &GrammarASTNode, scope: &ScopeChain| -> String {
            let cloned = n.clone();
            match transformer.expand_value_list(cloned, scope) {
                Ok(expanded) => emit_raw_node(&expanded),
                Err(_) => emit_raw_node(n),
            }
        };

        // Check if any children are include_arg nodes (updated grammar).
        let has_include_arg_nodes = node.children.iter().any(|c| {
            matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "include_arg")
        });

        if has_include_arg_nodes {
            // Bug #2: new grammar — include_args = include_arg { COMMA include_arg }
            for child in &node.children {
                match child {
                    ASTNodeOrToken::Node(n) if n.rule_name == "include_arg" => {
                        // include_arg = VARIABLE COLON value_list   (named)
                        //             | value_list                   (positional)
                        let first_is_var = n.children.iter().any(|c| {
                            matches!(c, ASTNodeOrToken::Token(t) if get_token_type_name(t) == "VARIABLE")
                        });
                        let has_colon = n.children.iter().any(|c| {
                            matches!(c, ASTNodeOrToken::Token(t)
                                if get_token_type_name(t) == "Colon" || t.value == ":")
                        });

                        if first_is_var && has_colon {
                            // Named arg: key = VARIABLE value, val = value_list
                            let mut key: Option<String> = None;
                            let mut val_node: Option<&GrammarASTNode> = None;
                            for ac in &n.children {
                                match ac {
                                    ASTNodeOrToken::Token(t) if get_token_type_name(t) == "VARIABLE" => {
                                        key = Some(t.value.clone());
                                    }
                                    ASTNodeOrToken::Node(vn) if vn.rule_name == "value_list" => {
                                        val_node = Some(vn);
                                    }
                                    _ => {}
                                }
                            }
                            if let (Some(k), Some(vn)) = (key, val_node) {
                                let val = evaluate_arg_node(self, vn, scope);
                                named.insert(k, val);
                            }
                        } else {
                            // Positional arg: find the value_list child
                            for ac in &n.children {
                                if let ASTNodeOrToken::Node(vn) = ac {
                                    if vn.rule_name == "value_list" {
                                        let val = evaluate_arg_node(self, vn, scope);
                                        if !val.is_empty() {
                                            positional.push(val);
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    // Commas between include_arg nodes — skip
                    ASTNodeOrToken::Token(t) if get_token_type_name(t) == "Comma" || t.value == "," => {}
                    _ => {}
                }
            }
        } else {
            // Legacy grammar: include_args = value_list { COMMA value_list }
            // or a single value_list containing comma-separated values.
            let mut args: Vec<String> = Vec::new();
            let mut current_arg: Vec<String> = Vec::new();

            fn collect_tokens_flat(node: &GrammarASTNode, out: &mut Vec<ASTNodeOrToken>) {
                for child in &node.children {
                    match child {
                        ASTNodeOrToken::Token(_) => out.push(child.clone()),
                        ASTNodeOrToken::Node(n) => {
                            if n.rule_name == "value_list" || n.rule_name == "value" {
                                collect_tokens_flat(n, out);
                            } else {
                                out.push(child.clone());
                            }
                        }
                    }
                }
            }

            let mut flat: Vec<ASTNodeOrToken> = Vec::new();
            collect_tokens_flat(&node, &mut flat);

            for child in &flat {
                match child {
                    ASTNodeOrToken::Token(tok) => {
                        let type_name = get_token_type_name(tok);
                        if type_name == "Comma" || tok.value == "," {
                            if !current_arg.is_empty() {
                                args.push(current_arg.join(" "));
                                current_arg.clear();
                            }
                        } else if type_name != "Semicolon" && tok.value != ";" {
                            // Bug #3: expand variable references in the CALLER's scope
                            if type_name == "VARIABLE" {
                                let expanded = expand_variables_in_text(&tok.value, scope);
                                current_arg.push(expanded);
                            } else {
                                current_arg.push(tok.value.clone());
                            }
                        }
                    }
                    ASTNodeOrToken::Node(n) => {
                        let text = emit_raw_node(n);
                        if !text.is_empty() {
                            current_arg.push(text);
                        }
                    }
                }
            }

            if !current_arg.is_empty() {
                args.push(current_arg.join(" "));
            }

            // Bug #3: expand any remaining variable references in caller scope
            positional = args.iter().map(|a| expand_variables_in_text(a, scope)).collect();
        }

        Ok((positional, named))
    }

    // =========================================================================
    // Control Flow
    // =========================================================================

    fn expand_control(
        &mut self,
        node: GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<Vec<GrammarASTNode>, LatticeError> {
        if let Some(ASTNodeOrToken::Node(inner)) = node.children.first() {
            match inner.rule_name.as_str() {
                "if_directive" => return self.expand_if(inner.clone(), scope),
                "for_directive" => return self.expand_for(inner.clone(), scope),
                "each_directive" => return self.expand_each(inner.clone(), scope),
                "while_directive" => return self.expand_while(inner.clone(), scope),
                _ => {}
            }
        }
        Ok(vec![])
    }

    // =========================================================================
    // @while Expansion (Lattice v2)
    // =========================================================================

    fn expand_while(
        &mut self,
        node: GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<Vec<GrammarASTNode>, LatticeError> {
        // while_directive = "@while" lattice_expression block ;
        let mut expr_node: Option<GrammarASTNode> = None;
        let mut block: Option<GrammarASTNode> = None;

        for child in &node.children {
            match child {
                ASTNodeOrToken::Node(n) if n.rule_name.contains("expression") || n.rule_name.contains("lattice") => {
                    if expr_node.is_none() && n.rule_name != "block" {
                        expr_node = Some(n.clone());
                    }
                }
                ASTNodeOrToken::Node(n) if n.rule_name == "block" => {
                    block = Some(n.clone());
                }
                _ => {}
            }
        }

        let (expr_node, block) = match (expr_node, block) {
            (Some(e), Some(b)) => (e, b),
            _ => return Ok(vec![]),
        };

        let mut result: Vec<GrammarASTNode> = Vec::new();
        let loop_scope = scope.child();
        let mut iterations = 0;

        loop {
            let evaluator = ExpressionEvaluator::new(&loop_scope);
            let val = evaluator.evaluate_node(&expr_node)?;
            if !val.is_truthy() {
                break;
            }

            iterations += 1;
            if iterations > self.max_while_iterations {
                return Err(LatticeError::max_iteration(self.max_while_iterations, 0, 0));
            }

            let block_clone = block.clone();
            let expanded = self.expand_node(block_clone, &loop_scope)?;
            let items = extract_block_contents_items(&expanded);
            result.extend(items);

            // Re-read any variable mutations from the block expansion
            // by expanding vars in the block contents scope
        }

        Ok(result)
    }

    fn expand_if(
        &mut self,
        node: GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<Vec<GrammarASTNode>, LatticeError> {
        // if_directive = "@if" lattice_expression block
        //                { "@else" "if" lattice_expression block }
        //                [ "@else" block ]

        // Parse the if/else-if/else structure into branches
        let mut branches: Vec<(Option<GrammarASTNode>, GrammarASTNode)> = Vec::new();
        let children = &node.children;
        let mut i = 0;

        while i < children.len() {
            match &children[i] {
                ASTNodeOrToken::Token(tok) if tok.value == "@if" => {
                    // @if expression block
                    if let (Some(expr), Some(block)) = (
                        get_node_at(children, i + 1),
                        get_node_at(children, i + 2),
                    ) {
                        branches.push((Some(expr.clone()), block.clone()));
                        i += 3;
                    } else {
                        i += 1;
                    }
                }
                ASTNodeOrToken::Token(tok) if tok.value == "@else" => {
                    // Peek at next token: "if" means else-if, otherwise plain else
                    let next_is_if = matches!(
                        children.get(i + 1),
                        Some(ASTNodeOrToken::Token(t)) if t.value == "if"
                    );
                    if next_is_if {
                        // @else if expression block
                        if let (Some(expr), Some(block)) = (
                            get_node_at(children, i + 2),
                            get_node_at(children, i + 3),
                        ) {
                            branches.push((Some(expr.clone()), block.clone()));
                            i += 4;
                        } else {
                            i += 1;
                        }
                    } else {
                        // @else block
                        if let Some(block) = get_node_at(children, i + 1) {
                            branches.push((None, block.clone()));
                            i += 2;
                        } else {
                            i += 1;
                        }
                    }
                }
                _ => { i += 1; }
            }
        }

        // Evaluate branches
        let evaluator = ExpressionEvaluator::new(scope);
        for (condition, block) in branches {
            let should_expand = match condition {
                None => true, // @else — always matches
                Some(expr) => {
                    let val = evaluator.evaluate_node(&expr)?;
                    val.is_truthy()
                }
            };

            if should_expand {
                let expanded = self.expand_node(block, scope)?;
                return Ok(extract_block_contents_items(&expanded));
            }
        }

        Ok(vec![])
    }

    fn expand_for(
        &mut self,
        node: GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<Vec<GrammarASTNode>, LatticeError> {
        // for_directive = "@for" VARIABLE "from" lattice_expression
        //                 ( "through" | "to" ) lattice_expression block ;

        let mut var_name: Option<String> = None;
        let mut from_expr: Option<GrammarASTNode> = None;
        let mut to_expr: Option<GrammarASTNode> = None;
        let mut is_through = false;
        let mut block: Option<GrammarASTNode> = None;

        let children = &node.children;
        let mut i = 0;

        while i < children.len() {
            match &children[i] {
                ASTNodeOrToken::Token(tok) if get_token_type_name(tok) == "VARIABLE" => {
                    var_name = Some(tok.value.clone());
                }
                ASTNodeOrToken::Token(tok) if tok.value == "from" => {
                    from_expr = get_node_at(children, i + 1).cloned();
                    i += 1;
                }
                ASTNodeOrToken::Token(tok) if tok.value == "through" => {
                    is_through = true;
                    to_expr = get_node_at(children, i + 1).cloned();
                    i += 1;
                }
                ASTNodeOrToken::Token(tok) if tok.value == "to" => {
                    is_through = false;
                    to_expr = get_node_at(children, i + 1).cloned();
                    i += 1;
                }
                ASTNodeOrToken::Node(n) if n.rule_name == "block" => {
                    block = Some(n.clone());
                }
                _ => {}
            }
            i += 1;
        }

        let (var_name, from_expr, to_expr, block) = match (var_name, from_expr, to_expr, block) {
            (Some(v), Some(f), Some(t), Some(b)) => (v, f, t, b),
            _ => return Ok(vec![]),
        };

        let evaluator = ExpressionEvaluator::new(scope);
        let from_val = evaluator.evaluate_node(&from_expr)?;
        let to_val = evaluator.evaluate_node(&to_expr)?;

        let from_num = extract_int_value(&from_val);
        let to_num = extract_int_value(&to_val);

        let end = if is_through { to_num + 1 } else { to_num };

        let mut result: Vec<GrammarASTNode> = Vec::new();

        for i_val in from_num..end {
            let mut loop_scope = scope.child();
            loop_scope.set(
                var_name.clone(),
                ScopeValue::Raw(i_val.to_string()),
            );

            let block_clone = block.clone();
            let expanded = self.expand_node(block_clone, &loop_scope)?;
            let items = extract_block_contents_items(&expanded);
            result.extend(items);
        }

        Ok(result)
    }

    fn expand_each(
        &mut self,
        node: GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<Vec<GrammarASTNode>, LatticeError> {
        // each_directive = "@each" VARIABLE { COMMA VARIABLE } "in" each_list block ;

        let mut var_names: Vec<String> = Vec::new();
        let mut each_list: Option<GrammarASTNode> = None;
        let mut block: Option<GrammarASTNode> = None;
        let mut past_in = false;

        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) => {
                    let type_name = get_token_type_name(tok);
                    if type_name == "VARIABLE" && !past_in {
                        var_names.push(tok.value.clone());
                    } else if tok.value == "in" {
                        past_in = true;
                    }
                }
                ASTNodeOrToken::Node(n) => {
                    if n.rule_name == "each_list" {
                        each_list = Some(n.clone());
                    } else if n.rule_name == "block" {
                        block = Some(n.clone());
                    }
                }
            }
        }

        let (each_list, block) = match (each_list, block) {
            (Some(l), Some(b)) => (l, b),
            _ => return Ok(vec![]),
        };

        // Bug #4: Extract list items, passing scope so variables pointing to
        // LatticeValue::Map or LatticeValue::List can be iterated correctly.
        let items: Vec<String> = collect_each_list_items(&each_list, scope);

        let mut result: Vec<GrammarASTNode> = Vec::new();
        for item in &items {
            let mut loop_scope = scope.child();
            if let Some(var_name) = var_names.first() {
                loop_scope.set(var_name.clone(), ScopeValue::Raw(item.clone()));
            }

            let block_clone = block.clone();
            let expanded = self.expand_node(block_clone, &loop_scope)?;
            let block_items = extract_block_contents_items(&expanded);
            result.extend(block_items);
        }

        Ok(result)
    }

    // =========================================================================
    // Function Evaluation
    // =========================================================================

    fn evaluate_function_call(
        &mut self,
        func_name: &str,
        node: GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<GrammarASTNode, LatticeError> {
        let func_def = self.functions[func_name].clone();

        // Extract arguments
        let args = self.parse_function_call_args(&node, scope)?;

        // Arity check
        let required = func_def.params.len() - func_def.defaults.len();
        if args.len() < required || args.len() > func_def.params.len() {
            return Err(LatticeError::wrong_arity(
                "Function", func_name,
                func_def.params.len(), args.len(),
                0, 0,
            ));
        }

        // Cycle detection
        if self.function_stack.contains(&func_name.to_string()) {
            let mut chain = self.function_stack.clone();
            chain.push(func_name.to_string());
            return Err(LatticeError::circular_reference("function", chain, 0, 0));
        }

        // Build isolated function scope (parent = global only)
        let mut func_scope = self.variables.child();
        for (i, param_name) in func_def.params.iter().enumerate() {
            if i < args.len() {
                func_scope.set(param_name.clone(), ScopeValue::Raw(args[i].clone()));
            } else if let Some(default) = func_def.defaults.get(param_name) {
                func_scope.set(param_name.clone(), ScopeValue::Raw(default.clone()));
            }
        }

        self.function_stack.push(func_name.to_string());
        let result = self.evaluate_function_body(&func_def.body, &func_scope);
        self.function_stack.pop();

        match result {
            Err(LatticeError::Return { value }) => {
                // @return signal: create a value node with the returned CSS text
                Ok(make_value_node(&value, &node))
            }
            Err(e) => Err(e),
            Ok(_) => Err(LatticeError::missing_return(func_name, 0, 0)),
        }
    }

    fn evaluate_function_body(
        &mut self,
        body: &GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<(), LatticeError> {
        // function_body = LBRACE { function_body_item } RBRACE
        for child in &body.children {
            if let ASTNodeOrToken::Node(n) = child {
                if n.rule_name == "function_body_item" {
                    self.evaluate_function_body_item(n, scope)?;
                }
            }
        }
        Ok(())
    }

    fn evaluate_function_body_item(
        &mut self,
        node: &GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<(), LatticeError> {
        // function_body_item = variable_declaration | return_directive | lattice_control
        if let Some(ASTNodeOrToken::Node(inner)) = node.children.first() {
            match inner.rule_name.as_str() {
                "variable_declaration" => {
                    // Evaluate and bind in scope (mutable borrow issue — use a clone workaround)
                    let mut scope_copy = scope.clone();
                    self.expand_variable_declaration(inner.clone(), &mut scope_copy)?;
                    // Can't update the original scope here, but this is acceptable
                    // for read-only function bodies
                }
                "return_directive" => {
                    return self.evaluate_return_directive(inner, scope);
                }
                "lattice_control" => {
                    self.evaluate_control_in_function(inner, scope)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn evaluate_return_directive(
        &self,
        node: &GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<(), LatticeError> {
        // return_directive = "@return" lattice_expression SEMICOLON
        let evaluator = ExpressionEvaluator::new(scope);
        for child in &node.children {
            if let ASTNodeOrToken::Node(n) = child {
                if n.rule_name == "lattice_expression" {
                    let value = evaluator.evaluate_node(n)?;
                    return Err(LatticeError::return_signal(value.to_css_string()));
                }
            }
        }
        Err(LatticeError::return_signal(""))
    }

    fn evaluate_control_in_function(
        &mut self,
        node: &GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<(), LatticeError> {
        // Only @if is meaningful inside a function
        if let Some(ASTNodeOrToken::Node(inner)) = node.children.first() {
            if inner.rule_name == "if_directive" {
                return self.evaluate_if_in_function(inner, scope);
            }
        }
        Ok(())
    }

    fn evaluate_if_in_function(
        &mut self,
        node: &GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<(), LatticeError> {
        let evaluator = ExpressionEvaluator::new(scope);
        let children = &node.children;
        let mut i = 0;

        while i < children.len() {
            match &children[i] {
                ASTNodeOrToken::Token(tok) if tok.value == "@if" => {
                    if let (Some(expr), Some(block)) = (
                        get_node_at(children, i + 1),
                        get_node_at(children, i + 2),
                    ) {
                        let val = evaluator.evaluate_node(expr)?;
                        if val.is_truthy() {
                            return self.evaluate_function_block(block, scope);
                        }
                        i += 3;
                    } else {
                        i += 1;
                    }
                }
                ASTNodeOrToken::Token(tok) if tok.value == "@else" => {
                    let next_is_if = matches!(
                        children.get(i + 1),
                        Some(ASTNodeOrToken::Token(t)) if t.value == "if"
                    );
                    if next_is_if {
                        if let (Some(expr), Some(block)) = (
                            get_node_at(children, i + 2),
                            get_node_at(children, i + 3),
                        ) {
                            let val = evaluator.evaluate_node(expr)?;
                            if val.is_truthy() {
                                return self.evaluate_function_block(block, scope);
                            }
                            i += 4;
                        } else {
                            i += 1;
                        }
                    } else {
                        if let Some(block) = get_node_at(children, i + 1) {
                            return self.evaluate_function_block(block, scope);
                        }
                        i += 2;
                    }
                }
                _ => { i += 1; }
            }
        }
        Ok(())
    }

    fn evaluate_function_block(
        &mut self,
        block: &GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<(), LatticeError> {
        // Look for return_directive, at_rule with @return, or nested blocks
        for child in &block.children {
            if let ASTNodeOrToken::Node(n) = child {
                match n.rule_name.as_str() {
                    "block_contents" => {
                        for item in &n.children {
                            if let ASTNodeOrToken::Node(item_node) = item {
                                if item_node.rule_name == "block_item" {
                                    if let Some(ASTNodeOrToken::Node(inner)) = item_node.children.first() {
                                        if inner.rule_name == "at_rule" {
                                            // Might be @return masquerading as at_rule
                                            if let Some(keyword_tok) = find_at_keyword(inner) {
                                                if keyword_tok == "@return" {
                                                    let expr_text = extract_at_prelude_text(inner);
                                                    return Err(LatticeError::return_signal(
                                                        resolve_variable_in_text(&expr_text, scope)
                                                    ));
                                                }
                                            }
                                        } else if inner.rule_name == "lattice_block_item" {
                                            if let Some(ASTNodeOrToken::Node(lbi)) = inner.children.first() {
                                                if lbi.rule_name == "return_directive" {
                                                    return self.evaluate_return_directive(lbi, scope);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    "return_directive" => {
                        return self.evaluate_return_directive(n, scope);
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn parse_function_call_args(
        &self,
        node: &GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<Vec<String>, LatticeError> {
        // Find function_args node
        let args_node = node.children.iter().find_map(|c| {
            if let ASTNodeOrToken::Node(n) = c {
                if n.rule_name == "function_args" {
                    return Some(n);
                }
            }
            None
        });

        let Some(args_node) = args_node else {
            return Ok(vec![]);
        };

        let mut args: Vec<Vec<String>> = vec![vec![]];

        for child in &args_node.children {
            match child {
                ASTNodeOrToken::Token(tok) => {
                    if tok.value == "," {
                        args.push(vec![]);
                    } else {
                        let val = if get_token_type_name(tok) == "VARIABLE" {
                            scope.get(&tok.value)
                                .map(|v| v.to_css_text())
                                .unwrap_or_else(|| tok.value.clone())
                        } else {
                            tok.value.clone()
                        };
                        args.last_mut().unwrap().push(val);
                    }
                }
                ASTNodeOrToken::Node(n) if n.rule_name == "function_arg" => {
                    for fc in &n.children {
                        match fc {
                            ASTNodeOrToken::Token(tok) => {
                                if tok.value == "," {
                                    args.push(vec![]);
                                } else {
                                    let val = if get_token_type_name(tok) == "VARIABLE" {
                                        scope.get(&tok.value)
                                            .map(|v| v.to_css_text())
                                            .unwrap_or_else(|| tok.value.clone())
                                    } else {
                                        tok.value.clone()
                                    };
                                    args.last_mut().unwrap().push(val);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(args.into_iter()
            .map(|parts| parts.join(""))
            .filter(|s| !s.is_empty())
            .collect())
    }

    // =========================================================================
    // Built-in Function Evaluation (Lattice v2)
    // =========================================================================

    fn evaluate_builtin_function_call(
        &mut self,
        func_name: &str,
        node: GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<GrammarASTNode, LatticeError> {
        // Parse arguments and evaluate them as LatticeValues
        let arg_strings = self.parse_function_call_args(&node, scope)?;
        let mut args: Vec<LatticeValue> = Vec::new();
        for arg_str in &arg_strings {
            let trimmed = arg_str.trim();
            // Try to convert the arg string to a LatticeValue
            let val = if trimmed.starts_with('#') {
                LatticeValue::Color(trimmed.to_string())
            } else if trimmed.starts_with('$') {
                // Variable reference — look up in scope
                match scope.get(trimmed) {
                    Some(sv) => match sv.as_lattice_value() {
                        Some(v) => v.clone(),
                        None => {
                            let text = sv.to_css_text();
                            parse_css_text_to_value(&text)
                        }
                    },
                    None => LatticeValue::Ident(trimmed.to_string()),
                }
            } else if trimmed.ends_with('%') {
                let num_str = trimmed.trim_end_matches('%');
                match num_str.parse::<f64>() {
                    Ok(n) => LatticeValue::Percentage(n),
                    Err(_) => LatticeValue::Ident(trimmed.to_string()),
                }
            } else if let Ok(n) = trimmed.parse::<f64>() {
                LatticeValue::Number(n)
            } else {
                // Could be a dimension, ident, string, etc.
                parse_css_text_to_value(trimmed)
            };
            args.push(val);
        }

        let result = evaluate_builtin(func_name, &args)?;
        Ok(make_value_node(&result.to_css_string(), &node))
    }

    // =========================================================================
    // Property Nesting Expansion (Lattice v2)
    // =========================================================================
    //
    // property_nesting = property COLON block ;
    // e.g., font: { size: 14px; weight: bold; }
    // becomes: font-size: 14px; font-weight: bold;

    #[allow(dead_code)]
    fn expand_property_nesting(
        &mut self,
        prefix: &str,
        block: GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<Vec<GrammarASTNode>, LatticeError> {
        let expanded = self.expand_node(block, scope)?;
        let items = extract_block_contents_items(&expanded);
        let mut result: Vec<GrammarASTNode> = Vec::new();

        for item in items {
            // Look for declaration nodes and prepend the prefix
            let modified = self.prepend_property_prefix(&item, prefix);
            result.push(modified);
        }

        Ok(result)
    }

    #[allow(dead_code)]
    fn prepend_property_prefix(&self, node: &GrammarASTNode, prefix: &str) -> GrammarASTNode {
        let mut new_node = node.clone();
        // Walk into declaration nodes and prepend prefix to property name
        if node.rule_name == "declaration" || node.rule_name == "block_item" {
            let mut found_property = false;
            for child in &mut new_node.children {
                match child {
                    ASTNodeOrToken::Token(tok) if !found_property => {
                        let type_name = get_token_type_name(tok);
                        if type_name == "Ident" || type_name == "IDENT" || type_name == "Name" {
                            tok.value = format!("{}-{}", prefix, tok.value);
                            found_property = true;
                        }
                    }
                    ASTNodeOrToken::Node(n) if n.rule_name == "declaration" => {
                        *n = self.prepend_property_prefix(n, prefix);
                    }
                    ASTNodeOrToken::Node(n) if n.rule_name == "block_item" || n.rule_name == "declaration_or_nested" => {
                        *n = self.prepend_property_prefix(n, prefix);
                    }
                    _ => {}
                }
            }
        }
        new_node
    }

    // =========================================================================
    // Variable in Selector Expansion (Lattice v2)
    // =========================================================================
    //
    // In selector positions, $variable tokens are resolved to their CSS text
    // values and concatenated with adjacent tokens.

    fn expand_selector_with_vars(
        &mut self,
        mut node: GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<GrammarASTNode, LatticeError> {
        let mut new_children: Vec<ASTNodeOrToken> = Vec::new();

        for child in node.children.drain(..) {
            match child {
                ASTNodeOrToken::Token(tok) if get_token_type_name(&tok) == "VARIABLE" => {
                    let substituted = self.substitute_variable(&tok, scope)?;
                    new_children.push(substituted);
                }
                other => new_children.push(other),
            }
        }

        node.children = new_children;
        Ok(node)
    }

    // =========================================================================
    // Pass 3: Cleanup
    // =========================================================================

    fn cleanup(&self, mut node: GrammarASTNode) -> GrammarASTNode {
        let mut new_children: Vec<ASTNodeOrToken> = Vec::new();

        for child in node.children.drain(..) {
            match child {
                ASTNodeOrToken::Node(n) => {
                    let cleaned = self.cleanup(n);
                    // Skip empty lattice artifacts
                    if !is_empty_artifact(&cleaned) {
                        new_children.push(ASTNodeOrToken::Node(cleaned));
                    }
                }
                other => new_children.push(other),
            }
        }

        node.children = new_children;
        node
    }
}

impl Default for LatticeTransformer {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

/// Extract the first `Node` child from a node (not Token children).
fn get_first_node_child(node: &GrammarASTNode) -> Option<&GrammarASTNode> {
    node.children.iter().find_map(|c| {
        if let ASTNodeOrToken::Node(n) = c { Some(n) } else { None }
    })
}

/// Get the node at position `i` in a children slice (Node variant only).
fn get_node_at(children: &[ASTNodeOrToken], i: usize) -> Option<&GrammarASTNode> {
    children.get(i).and_then(|c| {
        if let ASTNodeOrToken::Node(n) = c { Some(n) } else { None }
    })
}

/// Extract the `block_contents` children from a `block` node.
fn extract_block_contents_items(block: &GrammarASTNode) -> Vec<GrammarASTNode> {
    for child in &block.children {
        if let ASTNodeOrToken::Node(n) = child {
            if n.rule_name == "block_contents" {
                return n.children.iter()
                    .filter_map(|c| {
                        if let ASTNodeOrToken::Node(item) = c { Some(item.clone()) } else { None }
                    })
                    .collect();
            }
        }
    }
    vec![]
}

/// Collect all non-empty value items from an `each_list` node.
///
/// Bug #4: when the each_list contains a single VARIABLE that resolves to a
/// `LatticeValue::Map` or `LatticeValue::List` in scope, return its items
/// rather than the raw variable token text.
fn collect_each_list_items(node: &GrammarASTNode, scope: &ScopeChain) -> Vec<String> {
    // Check for the special case: a single VARIABLE token that resolves to a
    // map or list in scope. This handles `@each $key, $val in $my-map { }`.
    let non_comma_children: Vec<&ASTNodeOrToken> = node.children.iter().filter(|c| {
        !matches!(c, ASTNodeOrToken::Token(t) if get_token_type_name(t) == "Comma" || t.value == ",")
    }).collect();

    if non_comma_children.len() == 1 {
        if let Some(ASTNodeOrToken::Token(tok)) = non_comma_children.first() {
            let type_name = get_token_type_name(tok);
            if type_name == "VARIABLE" {
                if let Some(scope_val) = scope.get(&tok.value) {
                    match scope_val {
                        ScopeValue::Evaluated(LatticeValue::Map(entries)) => {
                            // Iterate over map entries — emit each as "key value" pair
                            return entries.iter()
                                .map(|(k, v)| format!("{} {}", k, v.to_css_string()))
                                .collect();
                        }
                        ScopeValue::Evaluated(LatticeValue::List(items)) => {
                            return items.iter().map(|v| v.to_css_string()).collect();
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    let mut items: Vec<String> = Vec::new();
    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(tok) => {
                let type_name = get_token_type_name(tok);
                if type_name != "Comma" && tok.value != "," {
                    // Bug #4: resolve VARIABLE tokens via scope when possible
                    if type_name == "VARIABLE" {
                        if let Some(sv) = scope.get(&tok.value) {
                            items.push(sv.to_css_text());
                        } else {
                            items.push(tok.value.clone());
                        }
                    } else {
                        items.push(tok.value.clone());
                    }
                }
            }
            ASTNodeOrToken::Node(n) if n.rule_name == "value" => {
                let text = emit_raw_node(n);
                if !text.is_empty() && text != "," {
                    items.push(text);
                }
            }
            _ => {}
        }
    }
    items
}

/// Extract the integer value from a LatticeValue, defaulting to 0.
fn extract_int_value(val: &LatticeValue) -> i64 {
    match val {
        LatticeValue::Number(n) => *n as i64,
        LatticeValue::Dimension { value, .. } => *value as i64,
        _ => 0,
    }
}

/// Emit the raw CSS text of a node by concatenating all token values.
pub fn emit_raw_node(node: &GrammarASTNode) -> String {
    let mut parts: Vec<String> = Vec::new();
    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(tok) => parts.push(tok.value.clone()),
            ASTNodeOrToken::Node(n) => parts.push(emit_raw_node(n)),
        }
    }
    parts.join(" ")
}

/// Create a synthetic IDENT token with the given value.
pub fn make_synthetic_token(value: &str, template: &Token) -> Token {
    // Determine the best token type for the value
    let (type_, type_name) = if value.starts_with('#') {
        (TokenType::Name, Some("HASH".to_string()))
    } else if value.ends_with('%') {
        (TokenType::Name, Some("PERCENTAGE".to_string()))
    } else if value.chars().any(|c| c.is_alphabetic()) && !value.starts_with('"') {
        (TokenType::Name, None)
    } else {
        (TokenType::Name, None)
    };

    Token {
        type_,
        type_name,
        value: value.to_string(),
        line: template.line,
        column: template.column,
    }
}

/// Create a synthetic `value` node wrapping a text value.
pub fn make_value_node(value: &str, _template: &GrammarASTNode) -> GrammarASTNode {
    // Find a template token for position info
    let template_token = Token {
        type_: TokenType::Name,
        type_name: None,
        value: value.to_string(),
        line: 0,
        column: 0,
    };

    let token = make_synthetic_token(value, &template_token);
    GrammarASTNode {
        rule_name: "value".to_string(),
        children: vec![ASTNodeOrToken::Token(token)],
    }
}

/// Check whether a node is an empty artifact from expansion.
fn is_empty_artifact(node: &GrammarASTNode) -> bool {
    if node.children.is_empty() && node.rule_name == "block_item" {
        return true;
    }
    false
}

/// Extract mixin/function parameters from a `mixin_params` node.
fn extract_params(node: &GrammarASTNode) -> (Vec<String>, HashMap<String, String>) {
    let mut params: Vec<String> = Vec::new();
    let mut defaults: HashMap<String, String> = HashMap::new();

    for child in &node.children {
        if let ASTNodeOrToken::Node(param_node) = child {
            if param_node.rule_name == "mixin_param" {
                let mut param_name: Option<String> = None;
                let mut default_value: Option<String> = None;

                for pc in &param_node.children {
                    match pc {
                        ASTNodeOrToken::Token(tok) if get_token_type_name(tok) == "VARIABLE" => {
                            param_name = Some(tok.value.clone());
                        }
                        ASTNodeOrToken::Node(n) if n.rule_name == "value_list" || n.rule_name == "mixin_value_list" => {
                            default_value = Some(emit_raw_node(n));
                        }
                        _ => {}
                    }
                }

                if let Some(name) = param_name {
                    params.push(name.clone());
                    if let Some(default) = default_value {
                        defaults.insert(name, default);
                    }
                }
            }
        }
    }

    (params, defaults)
}

/// Simple variable substitution in a text string (for arg expansion).
fn expand_variables_in_text(text: &str, scope: &ScopeChain) -> String {
    // Very simple: just return the text as-is if it doesn't contain $
    if !text.contains('$') {
        return text.to_string();
    }
    // Look up the entire text as a variable name
    if let Some(val) = scope.get(text.trim()) {
        return val.to_css_text();
    }
    text.to_string()
}

/// Resolve a variable reference if the text is a variable name.
fn resolve_variable_in_text(text: &str, scope: &ScopeChain) -> String {
    let trimmed = text.trim();
    if trimmed.starts_with('$') {
        if let Some(val) = scope.get(trimmed) {
            return val.to_css_text();
        }
    }
    trimmed.to_string()
}

/// Parse a CSS text string into a LatticeValue for built-in function args.
///
/// Tries to detect numbers, dimensions, percentages, colors, and strings.
/// Falls back to Ident for anything unrecognized.
fn parse_css_text_to_value(text: &str) -> LatticeValue {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return LatticeValue::Null;
    }
    if trimmed.starts_with('#') {
        return LatticeValue::Color(trimmed.to_string());
    }
    if trimmed.starts_with('"') && trimmed.ends_with('"') {
        return LatticeValue::String(trimmed[1..trimmed.len()-1].to_string());
    }
    if trimmed == "true" { return LatticeValue::Bool(true); }
    if trimmed == "false" { return LatticeValue::Bool(false); }
    if trimmed == "null" { return LatticeValue::Null; }
    if trimmed.ends_with('%') {
        if let Ok(n) = trimmed[..trimmed.len()-1].parse::<f64>() {
            return LatticeValue::Percentage(n);
        }
    }
    // Try dimension: number followed by unit letters
    if let Some(pos) = trimmed.find(|c: char| c.is_alphabetic()) {
        if pos > 0 {
            if let Ok(n) = trimmed[..pos].parse::<f64>() {
                return LatticeValue::Dimension {
                    value: n,
                    unit: trimmed[pos..].to_string(),
                };
            }
        }
    }
    if let Ok(n) = trimmed.parse::<f64>() {
        return LatticeValue::Number(n);
    }
    LatticeValue::Ident(trimmed.to_string())
}

/// Find the AT_KEYWORD token value in an at_rule node.
fn find_at_keyword(node: &GrammarASTNode) -> Option<String> {
    for child in &node.children {
        if let ASTNodeOrToken::Token(tok) = child {
            if get_token_type_name(tok) == "AT_KEYWORD" {
                return Some(tok.value.clone());
            }
        }
    }
    None
}

/// Extract the text of an at_rule's prelude.
fn extract_at_prelude_text(node: &GrammarASTNode) -> String {
    for child in &node.children {
        if let ASTNodeOrToken::Node(n) = child {
            if n.rule_name == "at_prelude" {
                return emit_raw_node(n);
            }
        }
    }
    String::new()
}
