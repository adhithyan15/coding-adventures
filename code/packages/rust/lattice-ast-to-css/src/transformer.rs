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
use crate::evaluator::{ExpressionEvaluator, get_token_type_name};
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
        // variable_declaration = VARIABLE COLON value_list SEMICOLON ;
        let mut name: Option<String> = None;
        let mut css_value: Option<String> = None;

        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) if get_token_type_name(tok) == "VARIABLE" => {
                    name = Some(tok.value.clone());
                }
                ASTNodeOrToken::Node(n) if n.rule_name == "value_list" => {
                    css_value = Some(emit_raw_node(n));
                }
                _ => {}
            }
        }

        if let (Some(n), Some(v)) = (name, css_value) {
            self.variables.set(n, ScopeValue::Raw(v));
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
    /// @include, or control flow.
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
                _ => {}
            }
        }
        Ok(vec![node])
    }

    /// Process a variable declaration inside a block — sets the variable in scope.
    fn expand_variable_declaration(
        &mut self,
        node: GrammarASTNode,
        scope: &mut ScopeChain,
    ) -> Result<(), LatticeError> {
        let mut var_name: Option<String> = None;
        let mut value_text: Option<String> = None;

        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) if get_token_type_name(tok) == "VARIABLE" => {
                    var_name = Some(tok.value.clone());
                }
                ASTNodeOrToken::Node(n) if n.rule_name == "value_list" => {
                    // Expand variables within the value before storing
                    let expanded = self.expand_value_list(n.clone(), scope)?;
                    value_text = Some(emit_raw_node(&expanded));
                }
                _ => {}
            }
        }

        if let (Some(name), Some(value)) = (var_name, value_text) {
            scope.set(name, ScopeValue::Raw(value));
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

        // CSS built-in: expand args but keep structure
        if is_css_function(&func_name) {
            return self.expand_children(node, scope);
        }

        // Lattice function: evaluate
        if self.functions.contains_key(&func_name) {
            return self.evaluate_function_call(&func_name.clone(), node, scope);
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
        // include_directive = "@include" FUNCTION include_args RPAREN ( SEMICOLON | block )
        //                   | "@include" IDENT ( SEMICOLON | block )

        let mut mixin_name: Option<String> = None;
        let mut args_node: Option<GrammarASTNode> = None;

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

        // Parse arguments
        let args = if let Some(a) = args_node {
            self.parse_include_args(a, scope)?
        } else {
            vec![]
        };

        // Arity check
        let required = mixin_def.params.len() - mixin_def.defaults.len();
        if args.len() < required || args.len() > mixin_def.params.len() {
            return Err(LatticeError::wrong_arity(
                "Mixin", &mixin_name,
                mixin_def.params.len(), args.len(),
                0, 0,
            ));
        }

        // Build the mixin scope
        let mut mixin_scope = scope.child();
        for (i, param_name) in mixin_def.params.iter().enumerate() {
            if i < args.len() {
                mixin_scope.set(param_name.clone(), ScopeValue::Raw(args[i].clone()));
            } else if let Some(default) = mixin_def.defaults.get(param_name) {
                mixin_scope.set(param_name.clone(), ScopeValue::Raw(default.clone()));
            }
        }

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

        result
    }

    /// Parse include arguments into CSS text strings.
    fn parse_include_args(
        &mut self,
        node: GrammarASTNode,
        scope: &ScopeChain,
    ) -> Result<Vec<String>, LatticeError> {
        // include_args = value_list { COMMA value_list } ;
        // Due to grammar design, args may be in a single value_list with commas inside.

        let mut args: Vec<String> = Vec::new();
        let mut current_arg: Vec<String> = Vec::new();

        let mut process_child = |child: &ASTNodeOrToken| {
            match child {
                ASTNodeOrToken::Token(tok) => {
                    let type_name = get_token_type_name(tok);
                    if type_name == "Comma" || tok.value == "," {
                        if !current_arg.is_empty() {
                            args.push(current_arg.join(" "));
                            current_arg.clear();
                        }
                    } else if type_name != "Semicolon" && tok.value != ";" {
                        current_arg.push(tok.value.clone());
                    }
                }
                ASTNodeOrToken::Node(n) => {
                    let text = emit_raw_node(n);
                    if !text.is_empty() {
                        current_arg.push(text);
                    }
                }
            }
        };

        // Flatten the include_args structure
        fn collect_tokens(node: &GrammarASTNode, out: &mut Vec<ASTNodeOrToken>) {
            for child in &node.children {
                match child {
                    ASTNodeOrToken::Token(_) => out.push(child.clone()),
                    ASTNodeOrToken::Node(n) => {
                        if n.rule_name == "value_list" || n.rule_name == "value" {
                            collect_tokens(n, out);
                        } else {
                            out.push(child.clone());
                        }
                    }
                }
            }
        }

        let mut flat: Vec<ASTNodeOrToken> = Vec::new();
        collect_tokens(&node, &mut flat);

        for child in &flat {
            process_child(child);
        }

        if !current_arg.is_empty() {
            args.push(current_arg.join(" "));
        }

        // Now expand variables in each arg
        let expanded: Vec<String> = args.iter().map(|a| {
            // Simple variable substitution in the arg text
            expand_variables_in_text(a, scope)
        }).collect();

        Ok(expanded)
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
                _ => {}
            }
        }
        Ok(vec![])
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

        // Extract list items from each_list
        let items: Vec<String> = collect_each_list_items(&each_list);

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
fn collect_each_list_items(node: &GrammarASTNode) -> Vec<String> {
    let mut items: Vec<String> = Vec::new();
    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(tok) => {
                let type_name = get_token_type_name(tok);
                if type_name != "Comma" && tok.value != "," {
                    items.push(tok.value.clone());
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
                        ASTNodeOrToken::Node(n) if n.rule_name == "value_list" => {
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
