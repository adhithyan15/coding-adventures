//! # CSS Emitter — reconstructs CSS text from a clean AST.
//!
//! After the transformer has expanded all Lattice constructs, the AST
//! contains only pure CSS nodes:
//!
//! | Rule                  | CSS construct                           |
//! |-----------------------|-----------------------------------------|
//! | `stylesheet`          | The root — a sequence of rules          |
//! | `qualified_rule`      | `h1 { color: red; }`                    |
//! | `at_rule`             | `@media screen { ... }`                 |
//! | `selector_list`       | `h1, h2, h3`                            |
//! | `complex_selector`    | `div > span`                            |
//! | `compound_selector`   | `.foo.bar#baz`                          |
//! | `block`               | `{ declarations }`                      |
//! | `declaration`         | `color: red;`                           |
//! | `value_list`          | `16px sans-serif`                       |
//! | `function_call`       | `rgb(255, 0, 0)`                        |
//! | `priority`            | `!important`                            |
//!
//! # Two Modes
//!
//! - **Pretty-print** (default): 2-space indentation, newlines between
//!   declarations, blank lines between rules.
//!
//! - **Minified**: No unnecessary whitespace. Every token is emitted with
//!   no spaces except where semantically required.
//!
//! # Design
//!
//! The emitter dispatches on `rule_name`. Each rule has a handler method
//! that formats that particular CSS construct. Unknown rules fall through to
//! the default handler which recurses into children.

use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use lexer::token::Token;

use crate::evaluator::get_token_type_name;

// ===========================================================================
// CSSEmitter
// ===========================================================================

/// Emits CSS text from a clean (Lattice-free) AST.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_lattice_ast_to_css::emitter::CSSEmitter;
///
/// let emitter = CSSEmitter::new("  ", false);
/// // let css = emitter.emit(&clean_ast);
/// ```
pub struct CSSEmitter {
    /// The indentation string per nesting level (default: `"  "` — two spaces)
    indent: String,
    /// If true, emit minified CSS (no unnecessary whitespace)
    minified: bool,
}

impl CSSEmitter {
    /// Create a new emitter.
    ///
    /// # Arguments
    ///
    /// - `indent`: Indentation per level (typically `"  "` or `"\t"`)
    /// - `minified`: If `true`, emit compact CSS with no whitespace
    pub fn new(indent: &str, minified: bool) -> Self {
        CSSEmitter {
            indent: indent.to_string(),
            minified,
        }
    }

    /// Emit CSS text from the root AST node.
    ///
    /// Pass the `stylesheet` node returned by the transformer. Returns a
    /// CSS string ending with a newline (for pretty mode) or without (minified).
    pub fn emit(&self, node: &GrammarASTNode) -> String {
        let result = self.emit_node(node, 0);
        let trimmed = result.trim().to_string();
        if trimmed.is_empty() {
            String::new()
        } else {
            format!("{}\n", trimmed)
        }
    }

    /// Dispatch to the appropriate handler based on `rule_name`.
    fn emit_node(&self, node: &GrammarASTNode, depth: usize) -> String {
        match node.rule_name.as_str() {
            "stylesheet" => self.emit_stylesheet(node, depth),
            "rule" => self.emit_rule(node, depth),
            "qualified_rule" => self.emit_qualified_rule(node, depth),
            "at_rule" => self.emit_at_rule(node, depth),
            "at_prelude" | "at_prelude_token" | "at_prelude_tokens" => {
                self.emit_at_prelude(node, depth)
            }
            "function_in_prelude" => self.emit_function_in_prelude(node, depth),
            "paren_block" => self.emit_paren_block(node, depth),
            "selector_list" => self.emit_selector_list(node, depth),
            "complex_selector" => self.emit_complex_selector(node, depth),
            "combinator" => self.emit_combinator(node, depth),
            "compound_selector" => self.emit_compound_selector(node, depth),
            "simple_selector" | "subclass_selector" => self.emit_single_child(node, depth),
            "class_selector" => self.emit_class_selector(node, depth),
            "id_selector" => self.emit_single_child(node, depth),
            "attribute_selector" => self.emit_attribute_selector(node, depth),
            "attr_matcher" | "attr_value" => self.emit_single_child(node, depth),
            "pseudo_class" => self.emit_pseudo_class(node, depth),
            "pseudo_class_args" | "pseudo_class_arg" => self.emit_default(node, depth),
            "pseudo_element" => self.emit_pseudo_element(node, depth),
            "block" => self.emit_block(node, depth),
            "block_contents" => self.emit_block_contents(node, depth),
            "block_item" | "declaration_or_nested" => self.emit_single_child(node, depth),
            "declaration" => self.emit_declaration(node, depth),
            "property" => self.emit_single_child(node, depth),
            "priority" => String::from("!important"),
            "value_list" => self.emit_value_list(node, depth),
            "value" => self.emit_value(node, depth),
            "function_call" => self.emit_function_call(node, depth),
            "function_args" => self.emit_function_args(node, depth),
            "function_arg" => self.emit_function_arg(node, depth),
            _ => {
                // Lattice nodes that somehow survive — silently skip
                if is_lattice_node(&node.rule_name) {
                    return String::new();
                }
                self.emit_default(node, depth)
            }
        }
    }

    /// Emit a raw token's text value.
    fn emit_token(&self, token: &Token) -> String {
        let type_name = get_token_type_name(token);
        if type_name == "String" || type_name == "STRING" {
            format!("\"{}\"", token.value)
        } else {
            token.value.clone()
        }
    }

    // -----------------------------------------------------------------------
    // Top-level structure
    // -----------------------------------------------------------------------

    fn emit_stylesheet(&self, node: &GrammarASTNode, depth: usize) -> String {
        let mut parts: Vec<String> = Vec::new();

        for child in &node.children {
            let text = match child {
                ASTNodeOrToken::Node(n) => self.emit_node(n, depth),
                ASTNodeOrToken::Token(t) => self.emit_token(t),
            };
            let trimmed = text.trim().to_string();
            if !trimmed.is_empty() {
                parts.push(trimmed);
            }
        }

        if self.minified {
            parts.join("")
        } else {
            parts.join("\n\n")
        }
    }

    fn emit_rule(&self, node: &GrammarASTNode, depth: usize) -> String {
        self.emit_single_child(node, depth)
    }

    // -----------------------------------------------------------------------
    // Qualified rules (selector + block)
    // -----------------------------------------------------------------------

    fn emit_qualified_rule(&self, node: &GrammarASTNode, depth: usize) -> String {
        let mut selector = String::new();
        let mut block = String::new();

        for child in &node.children {
            if let ASTNodeOrToken::Node(n) = child {
                match n.rule_name.as_str() {
                    "selector_list" => selector = self.emit_selector_list(n, depth),
                    "block" => block = self.emit_block(n, depth),
                    _ => {}
                }
            }
        }

        if self.minified {
            format!("{}{}", selector, block)
        } else {
            format!("{} {}", selector, block)
        }
    }

    // -----------------------------------------------------------------------
    // At-rules
    // -----------------------------------------------------------------------

    fn emit_at_rule(&self, node: &GrammarASTNode, depth: usize) -> String {
        let mut keyword = String::new();
        let mut prelude = String::new();
        let mut block_text = String::new();
        let mut has_semicolon = false;

        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) => {
                    let type_name = get_token_type_name(tok);
                    match type_name.as_str() {
                        "AT_KEYWORD" => keyword = tok.value.clone(),
                        "Semicolon" | "SEMICOLON" => has_semicolon = true,
                        _ => {}
                    }
                }
                ASTNodeOrToken::Node(n) => {
                    match n.rule_name.as_str() {
                        "at_prelude" => prelude = self.emit_at_prelude(n, depth),
                        "block" => block_text = self.emit_block(n, depth),
                        _ => {}
                    }
                }
            }
        }

        if self.minified {
            if has_semicolon {
                format!("{}{};", keyword, prelude)
            } else {
                format!("{}{}{}", keyword, prelude, block_text)
            }
        } else {
            let prelude_part = if prelude.trim().is_empty() {
                String::new()
            } else {
                format!(" {}", prelude.trim())
            };
            if has_semicolon {
                format!("{}{};", keyword, prelude_part)
            } else {
                format!("{}{} {}", keyword, prelude_part, block_text)
            }
        }
    }

    fn emit_at_prelude(&self, node: &GrammarASTNode, depth: usize) -> String {
        let mut parts: Vec<String> = Vec::new();
        for child in &node.children {
            let text = match child {
                ASTNodeOrToken::Node(n) => self.emit_node(n, depth),
                ASTNodeOrToken::Token(t) => self.emit_token(t),
            };
            if !text.is_empty() {
                parts.push(text);
            }
        }
        parts.join(" ")
    }

    fn emit_function_in_prelude(&self, node: &GrammarASTNode, depth: usize) -> String {
        let mut parts: Vec<String> = Vec::new();
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) => {
                    let type_name = get_token_type_name(tok);
                    if type_name == "RParen" || tok.value == ")" {
                        parts.push(")".to_string());
                    } else {
                        parts.push(tok.value.clone());
                    }
                }
                ASTNodeOrToken::Node(n) => parts.push(self.emit_node(n, depth)),
            }
        }
        parts.join("")
    }

    fn emit_paren_block(&self, node: &GrammarASTNode, depth: usize) -> String {
        let mut parts: Vec<String> = Vec::new();
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) => {
                    let type_name = get_token_type_name(tok);
                    match type_name.as_str() {
                        "LParen" => parts.push("(".to_string()),
                        "RParen" => parts.push(")".to_string()),
                        _ => parts.push(tok.value.clone()),
                    }
                }
                ASTNodeOrToken::Node(n) => parts.push(self.emit_node(n, depth)),
            }
        }
        parts.join("")
    }

    // -----------------------------------------------------------------------
    // Selectors
    // -----------------------------------------------------------------------

    fn emit_selector_list(&self, node: &GrammarASTNode, depth: usize) -> String {
        let mut selectors: Vec<String> = Vec::new();

        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) => {
                    let type_name = get_token_type_name(tok);
                    if type_name == "Comma" || tok.value == "," {
                        // Separator — handled by join below
                        continue;
                    }
                }
                ASTNodeOrToken::Node(n) => {
                    let text = self.emit_node(n, depth);
                    if !text.trim().is_empty() {
                        selectors.push(text.trim().to_string());
                    }
                }
            }
        }

        let sep = if self.minified { "," } else { ", " };
        selectors.join(sep)
    }

    fn emit_complex_selector(&self, node: &GrammarASTNode, depth: usize) -> String {
        let mut parts: Vec<String> = Vec::new();
        for child in &node.children {
            let text = match child {
                ASTNodeOrToken::Node(n) => self.emit_node(n, depth),
                ASTNodeOrToken::Token(t) => self.emit_token(t),
            };
            if !text.trim().is_empty() {
                parts.push(text.trim().to_string());
            }
        }
        parts.join(" ")
    }

    fn emit_combinator(&self, node: &GrammarASTNode, _depth: usize) -> String {
        if let Some(ASTNodeOrToken::Token(tok)) = node.children.first() {
            return tok.value.clone();
        }
        String::new()
    }

    fn emit_compound_selector(&self, node: &GrammarASTNode, depth: usize) -> String {
        // Compound selectors concatenate without spaces: .foo.bar#baz:hover
        let mut parts: Vec<String> = Vec::new();
        for child in &node.children {
            let text = match child {
                ASTNodeOrToken::Node(n) => self.emit_node(n, depth),
                ASTNodeOrToken::Token(t) => self.emit_token(t),
            };
            if !text.is_empty() {
                parts.push(text);
            }
        }
        parts.join("")
    }

    fn emit_class_selector(&self, node: &GrammarASTNode, _depth: usize) -> String {
        // DOT IDENT → ".classname"
        let parts: Vec<String> = node.children.iter()
            .filter_map(|c| {
                if let ASTNodeOrToken::Token(tok) = c { Some(tok.value.clone()) } else { None }
            })
            .collect();
        parts.join("")
    }

    fn emit_attribute_selector(&self, node: &GrammarASTNode, depth: usize) -> String {
        let mut parts: Vec<String> = Vec::new();
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) => {
                    let type_name = get_token_type_name(tok);
                    match type_name.as_str() {
                        "LBracket" => parts.push("[".to_string()),
                        "RBracket" => parts.push("]".to_string()),
                        _ => parts.push(tok.value.clone()),
                    }
                }
                ASTNodeOrToken::Node(n) => parts.push(self.emit_node(n, depth)),
            }
        }
        parts.join("")
    }

    fn emit_pseudo_class(&self, node: &GrammarASTNode, depth: usize) -> String {
        let mut parts: Vec<String> = Vec::new();
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) => {
                    let type_name = get_token_type_name(tok);
                    match type_name.as_str() {
                        "Colon" => parts.push(":".to_string()),
                        "RParen" => parts.push(")".to_string()),
                        _ => parts.push(tok.value.clone()),
                    }
                }
                ASTNodeOrToken::Node(n) => parts.push(self.emit_node(n, depth)),
            }
        }
        parts.join("")
    }

    fn emit_pseudo_element(&self, node: &GrammarASTNode, _depth: usize) -> String {
        let mut parts: Vec<String> = Vec::new();
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) => {
                    let type_name = get_token_type_name(tok);
                    match type_name.as_str() {
                        "COLON_COLON" => parts.push("::".to_string()),
                        _ => parts.push(tok.value.clone()),
                    }
                }
                _ => {}
            }
        }
        parts.join("")
    }

    // -----------------------------------------------------------------------
    // Blocks and declarations
    // -----------------------------------------------------------------------

    fn emit_block(&self, node: &GrammarASTNode, depth: usize) -> String {
        // Find block_contents
        let contents = node.children.iter().find_map(|c| {
            if let ASTNodeOrToken::Node(n) = c {
                if n.rule_name == "block_contents" { return Some(n); }
            }
            None
        });

        if self.minified {
            let inner = contents.map(|c| self.emit_block_contents(c, depth + 1))
                .unwrap_or_default();
            format!("{{{}}}", inner)
        } else {
            match contents {
                None => {
                    format!("{{\n{}}}", " ".repeat(self.indent.len() * depth))
                }
                Some(c) => {
                    let inner = self.emit_block_contents(c, depth + 1);
                    if inner.trim().is_empty() {
                        format!("{{\n{}}}", " ".repeat(self.indent.len() * depth))
                    } else {
                        format!("{{\n{}\n{}}}", inner, self.indent.repeat(depth))
                    }
                }
            }
        }
    }

    fn emit_block_contents(&self, node: &GrammarASTNode, depth: usize) -> String {
        let mut parts: Vec<String> = Vec::new();

        for child in &node.children {
            let text = match child {
                ASTNodeOrToken::Node(n) => self.emit_node(n, depth),
                ASTNodeOrToken::Token(t) => self.emit_token(t),
            };
            let trimmed = text.trim().to_string();
            if !trimmed.is_empty() {
                parts.push(trimmed);
            }
        }

        if self.minified {
            parts.join("")
        } else {
            let prefix = self.indent.repeat(depth);
            parts.iter().map(|p| format!("{}{}", prefix, p)).collect::<Vec<_>>().join("\n")
        }
    }

    fn emit_declaration(&self, node: &GrammarASTNode, depth: usize) -> String {
        let mut prop = String::new();
        let mut value = String::new();
        let mut priority = String::new();

        for child in &node.children {
            if let ASTNodeOrToken::Node(n) = child {
                match n.rule_name.as_str() {
                    "property" => prop = self.emit_single_child(n, depth),
                    "value_list" => value = self.emit_value_list(n, depth),
                    "priority" => priority = " !important".to_string(),
                    _ => {}
                }
            }
        }

        if self.minified {
            format!("{}:{}{};", prop, value, priority)
        } else {
            format!("{}: {}{};", prop, value, priority)
        }
    }

    // -----------------------------------------------------------------------
    // Values
    // -----------------------------------------------------------------------

    fn emit_value_list(&self, node: &GrammarASTNode, depth: usize) -> String {
        let mut parts: Vec<String> = Vec::new();

        for child in &node.children {
            let text = match child {
                ASTNodeOrToken::Node(n) => self.emit_node(n, depth),
                ASTNodeOrToken::Token(t) => self.emit_token(t),
            };
            if !text.is_empty() {
                parts.push(text);
            }
        }

        // Join parts with spaces, then collapse " , " → ", "
        let result = parts.join(" ");
        result.replace(" , ", ", ").replace(" ,", ",")
    }

    fn emit_value(&self, node: &GrammarASTNode, depth: usize) -> String {
        if node.children.len() == 1 {
            return match &node.children[0] {
                ASTNodeOrToken::Node(n) => self.emit_node(n, depth),
                ASTNodeOrToken::Token(t) => self.emit_token(t),
            };
        }
        self.emit_default(node, depth)
    }

    fn emit_function_call(&self, node: &GrammarASTNode, depth: usize) -> String {
        if node.children.len() == 1 {
            // URL_TOKEN case
            if let ASTNodeOrToken::Token(tok) = &node.children[0] {
                return tok.value.clone();
            }
        }

        let mut parts: Vec<String> = Vec::new();
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) => {
                    let type_name = get_token_type_name(tok);
                    match type_name.as_str() {
                        "FUNCTION" => parts.push(tok.value.clone()), // Includes "("
                        "RParen" => parts.push(")".to_string()),
                        _ => parts.push(tok.value.clone()),
                    }
                }
                ASTNodeOrToken::Node(n) => parts.push(self.emit_node(n, depth)),
            }
        }
        parts.join("")
    }

    fn emit_function_args(&self, node: &GrammarASTNode, depth: usize) -> String {
        let mut parts: Vec<String> = Vec::new();
        for child in &node.children {
            let text = match child {
                ASTNodeOrToken::Node(n) => self.emit_node(n, depth),
                ASTNodeOrToken::Token(t) => self.emit_token(t),
            };
            parts.push(text);
        }
        let result = parts.join(" ");
        result.replace(" , ", ", ").replace(" ,", ",")
    }

    fn emit_function_arg(&self, node: &GrammarASTNode, depth: usize) -> String {
        if node.children.len() == 1 {
            return match &node.children[0] {
                ASTNodeOrToken::Node(n) => self.emit_node(n, depth),
                ASTNodeOrToken::Token(t) => self.emit_token(t),
            };
        }

        // Bug #5: when a function_arg contains a nested function call (FUNCTION token,
        // function_args node, RParen token), concatenate the parts WITHOUT a separator.
        // Previously emit_default joined all children with " ", producing e.g. "rgb( 255,0,0 )"
        // instead of the correct "rgb(255,0,0)".
        let has_function_token = node.children.iter().any(|c| {
            matches!(c, ASTNodeOrToken::Token(t) if get_token_type_name(t) == "FUNCTION")
        });
        let has_rparen = node.children.iter().any(|c| {
            matches!(c, ASTNodeOrToken::Token(t) if {
                let tn = get_token_type_name(t);
                tn == "RParen" || t.value == ")"
            })
        });

        if has_function_token && has_rparen {
            // Nested function call — concatenate with no separator
            let parts: Vec<String> = node.children.iter()
                .map(|c| match c {
                    ASTNodeOrToken::Node(n) => self.emit_node(n, depth),
                    ASTNodeOrToken::Token(t) => {
                        let tn = get_token_type_name(t);
                        if tn == "RParen" || t.value == ")" {
                            ")".to_string()
                        } else {
                            self.emit_token(t)
                        }
                    }
                })
                .collect();
            return parts.concat();
        }

        self.emit_default(node, depth)
    }

    // -----------------------------------------------------------------------
    // Default handlers
    // -----------------------------------------------------------------------

    /// Emit a node with exactly one child — just emit the child.
    fn emit_single_child(&self, node: &GrammarASTNode, depth: usize) -> String {
        match node.children.first() {
            Some(ASTNodeOrToken::Node(n)) => self.emit_node(n, depth),
            Some(ASTNodeOrToken::Token(t)) => self.emit_token(t),
            None => String::new(),
        }
    }

    /// Default: concatenate all children with spaces.
    fn emit_default(&self, node: &GrammarASTNode, depth: usize) -> String {
        let parts: Vec<String> = node.children.iter()
            .map(|c| match c {
                ASTNodeOrToken::Node(n) => self.emit_node(n, depth),
                ASTNodeOrToken::Token(t) => self.emit_token(t),
            })
            .collect();
        parts.join(" ")
    }
}

impl Default for CSSEmitter {
    fn default() -> Self {
        CSSEmitter::new("  ", false)
    }
}

/// Check if a rule name is a Lattice-specific node that should be skipped.
fn is_lattice_node(rule_name: &str) -> bool {
    matches!(rule_name,
        "variable_declaration" | "mixin_definition" | "function_definition"
        | "use_directive" | "include_directive" | "lattice_rule"
        | "lattice_block_item" | "lattice_control" | "if_directive"
        | "for_directive" | "each_directive" | "return_directive"
        | "function_body" | "function_body_item" | "mixin_params"
        | "mixin_param" | "lattice_expression" | "lattice_or_expr"
        | "lattice_and_expr" | "lattice_comparison" | "comparison_op"
        | "lattice_additive" | "lattice_multiplicative" | "lattice_unary"
        | "lattice_primary" | "include_args" | "each_list"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_token_node(rule: &str, children: Vec<ASTNodeOrToken>) -> GrammarASTNode {
        GrammarASTNode { rule_name: rule.to_string(), children }
    }

    fn make_ident_token(value: &str) -> ASTNodeOrToken {
        ASTNodeOrToken::Token(Token {
            type_: lexer::token::TokenType::Name,
            type_name: None,
            value: value.to_string(),
            line: 0,
            column: 0,
        })
    }

    fn make_named_token(type_name: &str, value: &str) -> ASTNodeOrToken {
        ASTNodeOrToken::Token(Token {
            type_: lexer::token::TokenType::Name,
            type_name: Some(type_name.to_string()),
            value: value.to_string(),
            line: 0,
            column: 0,
        })
    }

    #[test]
    fn test_emit_empty_stylesheet() {
        let emitter = CSSEmitter::default();
        let root = GrammarASTNode {
            rule_name: "stylesheet".to_string(),
            children: vec![],
        };
        let result = emitter.emit(&root);
        assert_eq!(result, "");
    }

    #[test]
    fn test_emit_simple_declaration() {
        let emitter = CSSEmitter::default();

        // Build: declaration = property ":" value_list ";"
        let prop = make_token_node("property", vec![make_ident_token("color")]);
        let val_token = ASTNodeOrToken::Token(Token {
            type_: lexer::token::TokenType::Name,
            type_name: None,
            value: "red".to_string(),
            line: 0, column: 0,
        });
        let value = make_token_node("value", vec![val_token]);
        let value_list = make_token_node("value_list", vec![ASTNodeOrToken::Node(value)]);

        let decl = make_token_node("declaration", vec![
            ASTNodeOrToken::Node(prop),
            ASTNodeOrToken::Node(value_list),
        ]);

        let result = emitter.emit_declaration(&decl, 0);
        assert!(result.contains("color"), "Expected 'color' in: {result}");
        assert!(result.contains("red"), "Expected 'red' in: {result}");
        assert!(result.ends_with(';'), "Expected semicolon in: {result}");
    }

    #[test]
    fn test_minified_declaration() {
        let emitter = CSSEmitter::new("  ", true);

        let prop = make_token_node("property", vec![make_ident_token("color")]);
        let val_token = ASTNodeOrToken::Token(Token {
            type_: lexer::token::TokenType::Name,
            type_name: None,
            value: "red".to_string(),
            line: 0, column: 0,
        });
        let value = make_token_node("value", vec![val_token]);
        let value_list = make_token_node("value_list", vec![ASTNodeOrToken::Node(value)]);

        let decl = make_token_node("declaration", vec![
            ASTNodeOrToken::Node(prop),
            ASTNodeOrToken::Node(value_list),
        ]);

        let result = emitter.emit_declaration(&decl, 0);
        // Minified: "color:red;" — no space after colon
        assert_eq!(result, "color:red;");
    }

    #[test]
    fn test_emit_class_selector() {
        let emitter = CSSEmitter::default();

        // class_selector = DOT IDENT
        let dot = ASTNodeOrToken::Token(Token {
            type_: lexer::token::TokenType::Dot,
            type_name: None,
            value: ".".to_string(),
            line: 0, column: 0,
        });
        let ident = make_ident_token("btn");
        let selector = make_token_node("class_selector", vec![dot, ident]);

        let result = emitter.emit_class_selector(&selector, 0);
        assert_eq!(result, ".btn");
    }

    #[test]
    fn test_lattice_node_skipped() {
        let emitter = CSSEmitter::default();
        // A lattice node should return empty string
        let node = make_token_node("variable_declaration", vec![]);
        let result = emitter.emit_node(&node, 0);
        assert_eq!(result, "");
    }

    #[test]
    fn test_emit_at_rule_with_semicolon() {
        let emitter = CSSEmitter::default();

        // @import url("style.css");
        let at_node = ASTNodeOrToken::Token(Token {
            type_: lexer::token::TokenType::Name,
            type_name: Some("AT_KEYWORD".to_string()),
            value: "@import".to_string(),
            line: 0, column: 0,
        });
        let url = make_ident_token("url(style.css)");
        let prelude_token = make_token_node("at_prelude_token", vec![url]);
        let prelude = make_token_node("at_prelude", vec![ASTNodeOrToken::Node(prelude_token)]);
        let semicolon = ASTNodeOrToken::Token(Token {
            type_: lexer::token::TokenType::Semicolon,
            type_name: None,
            value: ";".to_string(),
            line: 0, column: 0,
        });

        let at_rule = make_token_node("at_rule", vec![
            at_node,
            ASTNodeOrToken::Node(prelude),
            semicolon,
        ]);

        let result = emitter.emit_at_rule(&at_rule, 0);
        assert!(result.starts_with("@import"), "Expected @import: {result}");
        assert!(result.ends_with(';'), "Expected semicolon: {result}");
    }
}
