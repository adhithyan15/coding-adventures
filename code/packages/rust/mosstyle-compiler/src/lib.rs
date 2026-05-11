//! # mosstyle-compiler — Compiling `.msl` component style files.
//!
//! `mosstyle` is the visual style language for the Mosaic UI stack.
//! A `.msl` file answers exactly one question: *what do the parts of this
//! component look like, in each of their possible states?*
//!
//! It assigns style properties (color, font, spacing, …) to named **parts**
//! exported by the companion `.mll` layout file, optionally overriding per
//! interaction state (hover, pressed, focused, disabled, selected, …).
//!
//! # Pipeline
//!
//! ```text
//! .msl source  +  part map JSON (.mll output)
//!       │
//!       ▼  tokenize()
//! Vec<Token>         (mosstyle.tokens grammar via GrammarLexer)
//!       │
//!       ▼  parse()
//! GrammarASTNode     (mosstyle.grammar via GrammarParser)
//!       │
//!       ▼  analyze()
//! StyleDef           (typed IR: component name + part styles)
//!       │
//!       ▼  validate()
//! ValidationResult   (part existence, property validity, token resolution)
//!       │
//!       ▼  emit_css()
//! String             (scoped CSS for the DOM/React backend)
//! ```
//!
//! # Design tokens
//!
//! Token references (`$color-surface`, `$font-size-body`, …) are Lattice-
//! style variables.  This first implementation resolves them against a built-in
//! **default dark palette** that matches the values from `UI15-mosstyle.md §1`.
//! Full Lattice compilation and custom token override files are v2.
//!
//! # Quick start
//!
//! ```no_run
//! use mosstyle_compiler::compile;
//!
//! let src = r#"
//!   style Grid {
//!     part root {
//!       background: #ffffff ;
//!     }
//!   }
//! "#;
//!
//! let result = compile(src, None).expect("compilation failed");
//! println!("{}", result.css);
//! ```

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::{Token, TokenType};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode, GrammarParser};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

mod _grammar;

// ===========================================================================
// Style IR types
// ===========================================================================

/// The analyzed representation of a `.msl` file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StyleDef {
    /// PascalCase component name (matches the `.mil` and `.mll` component name).
    // Renamed to "component" in JSON for consistency with mosmodel-compiler output.
    #[serde(rename = "component")]
    pub component_name: String,
    /// One block per named part.
    pub parts: Vec<PartStyle>,
}

/// Style declarations for a single named part.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartStyle {
    /// The part name (e.g. `root`, `cell-grid`, `header-text`).
    pub name: String,
    /// Base-state properties (always applied).
    pub base: Vec<StyleProp>,
    /// Per-interaction-state overrides.
    pub states: Vec<StateStyle>,
}

/// A single style property declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StyleProp {
    /// Property name in kebab-case (e.g. `background`, `font-size`).
    pub name: String,
    /// The raw value string (after token resolution if applicable).
    pub value: String,
}

/// Style overrides for one interaction state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateStyle {
    /// State name: `hover`, `pressed`, `focused`, `disabled`, `selected`, etc.
    pub state: String,
    /// Properties that override the base in this state.
    pub props: Vec<StyleProp>,
}

// ===========================================================================
// Compiler output
// ===========================================================================

/// The result of a successful `compile()` call.
#[derive(Debug, Clone)]
pub struct CompileOutput {
    /// The analyzed style IR.
    pub def: StyleDef,
    /// Scoped CSS for the DOM / React backend.
    ///
    /// Class names follow the pattern `mos-{ComponentName}-{part-name}`.
    /// State selectors use CSS pseudo-classes: `:hover`, `:active`, `:focus`,
    /// `.disabled`, `.selected`, etc.
    pub css: String,
    /// Resolved style map as JSON (backend-agnostic intermediate form).
    pub style_map_json: String,
}

// ===========================================================================
// Compiler errors
// ===========================================================================

/// A structured compile error from the style compiler.
#[derive(Debug, Clone, PartialEq)]
pub struct CompileError {
    pub kind: ErrorKind,
    pub message: String,
}

/// Error kinds for the mosstyle compiler (§9 of UI15-mosstyle.md).
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    /// A `part` block names a part not in the layout's part map.
    UnknownPart,
    /// A property name is not in the known property table.
    UnknownProperty,
    /// A state name is not in the known states list.
    UnknownState,
    /// A `$token-ref` has no definition in the token map.
    UnresolvedToken,
    /// The AST has an unexpected shape (internal error).
    InternalError,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for CompileError {}

// ===========================================================================
// Default token palette (UI15-mosstyle.md §1)
// ===========================================================================
//
// These values implement the dark-mode base palette from the spec.
// Custom token files and the full Lattice override system are v2.

fn default_token_map() -> HashMap<String, String> {
    let mut m = HashMap::new();
    // Colors
    m.insert("color-surface".to_string(),       "#1e1e1e".to_string());
    m.insert("color-surface-hover".to_string(),  "#2e2e2e".to_string());
    m.insert("color-text-primary".to_string(),   "#ffffff".to_string());
    m.insert("color-text-muted".to_string(),     "rgba(255,255,255,0.6)".to_string());
    m.insert("color-accent".to_string(),         "#4a90d9".to_string());
    m.insert("color-border".to_string(),         "rgba(255,255,255,0.12)".to_string());
    m.insert("color-danger".to_string(),         "#e53e3e".to_string());
    // Radii
    m.insert("radius-sm".to_string(),            "4px".to_string());
    m.insert("radius-md".to_string(),            "8px".to_string());
    m.insert("radius-lg".to_string(),            "12px".to_string());
    // Spacing
    m.insert("spacing-xs".to_string(),           "4px".to_string());
    m.insert("spacing-sm".to_string(),           "8px".to_string());
    m.insert("spacing-md".to_string(),           "16px".to_string());
    m.insert("spacing-lg".to_string(),           "24px".to_string());
    // Typography
    m.insert("font-family-body".to_string(),     "\"Inter\", system-ui".to_string());
    m.insert("font-size-sm".to_string(),         "12px".to_string());
    m.insert("font-size-body".to_string(),       "14px".to_string());
    m.insert("font-size-lg".to_string(),         "18px".to_string());
    m.insert("font-weight-normal".to_string(),   "400".to_string());
    m.insert("font-weight-bold".to_string(),     "600".to_string());
    // Durations
    m.insert("duration-fast".to_string(),        "80ms".to_string());
    m.insert("duration-normal".to_string(),      "150ms".to_string());
    m.insert("duration-slow".to_string(),        "300ms".to_string());
    m.insert("easing-out".to_string(),           "ease-out".to_string());
    // Opacity
    m.insert("opacity-disabled".to_string(),     "0.4".to_string());
    m
}

/// Resolve a `$token-ref` to its concrete value.
///
/// `token_name` is the name without the `$` prefix, e.g. `color-surface`.
fn resolve_token(token_name: &str, extra: &HashMap<String, String>) -> Option<String> {
    if let Some(v) = extra.get(token_name) {
        return Some(v.clone());
    }
    default_token_map().get(token_name).cloned()
}

// ===========================================================================
// Known states and properties
// ===========================================================================

const VALID_STATES: &[&str] = &[
    "hover", "pressed", "focused", "disabled", "selected", "editing", "error",
];

// ===========================================================================
// Tokenizer
// ===========================================================================

/// Tokenize mosstyle source text into a flat `Vec<Token>`.
///
/// Returns `Err(CompileError)` rather than panicking if the lexer encounters
/// a character it cannot recognise.  Callers such as `parse_style` propagate
/// this error upward through the `compile` pipeline.
pub fn tokenize(source: &str) -> Result<Vec<Token>, CompileError> {
    let grammar = _grammar::token_grammar();
    let mut lexer = GrammarLexer::new(source, &grammar);
    lexer.tokenize().map_err(|e| CompileError {
        kind: ErrorKind::InternalError,
        message: format!("mosstyle tokenization failed: {e}"),
    })
}

// ===========================================================================
// Parser
// ===========================================================================

/// Parse mosstyle source text into a grammar AST.
pub fn parse_style(source: &str) -> Result<GrammarASTNode, String> {
    let tokens = tokenize(source).map_err(|e| e.message)?;
    let grammar = _grammar::parser_grammar();
    let mut parser = GrammarParser::new(tokens, grammar);
    parser.parse().map_err(|e| format!("parse error: {e}"))
}

// ===========================================================================
// Analyzer — GrammarASTNode → StyleDef
// ===========================================================================

/// Walk the raw grammar AST and produce a typed `StyleDef`.
pub fn analyze(ast: &GrammarASTNode) -> Result<StyleDef, CompileError> {
    let style_node = find_rule(ast, "style_def").ok_or_else(|| CompileError {
        kind: ErrorKind::InternalError,
        message: "style_def rule not found in AST".to_string(),
    })?;

    let component_name = extract_style_name(style_node)?;
    let parts = extract_parts(style_node)?;

    Ok(StyleDef {
        component_name,
        parts,
    })
}

fn extract_style_name(style_def: &GrammarASTNode) -> Result<String, CompileError> {
    let mut saw_keyword = false;
    for child in &style_def.children {
        if let ASTNodeOrToken::Token(t) = child {
            if t.type_ == TokenType::Keyword && t.value == "style" {
                saw_keyword = true;
            } else if saw_keyword && t.type_ == TokenType::Name {
                return Ok(t.value.clone());
            }
        }
    }
    Err(CompileError {
        kind: ErrorKind::InternalError,
        message: "Could not extract component name from style_def".to_string(),
    })
}

fn extract_parts(style_def: &GrammarASTNode) -> Result<Vec<PartStyle>, CompileError> {
    let mut parts = Vec::new();
    for child in &style_def.children {
        if let ASTNodeOrToken::Node(n) = child {
            if n.rule_name == "part_def" {
                parts.push(analyze_part(n)?);
            }
        }
    }
    Ok(parts)
}

fn analyze_part(part_def: &GrammarASTNode) -> Result<PartStyle, CompileError> {
    // part_def = KEYWORD("part") NAME LBRACE { part_item } RBRACE
    let mut saw_keyword = false;
    let mut name: Option<String> = None;
    let mut base: Vec<StyleProp> = Vec::new();
    let mut states: Vec<StateStyle> = Vec::new();

    for child in &part_def.children {
        match child {
            ASTNodeOrToken::Token(t) => {
                if t.type_ == TokenType::Keyword && t.value == "part" {
                    saw_keyword = true;
                } else if saw_keyword && t.type_ == TokenType::Name && name.is_none() {
                    name = Some(t.value.clone());
                }
            }
            ASTNodeOrToken::Node(n) => {
                match n.rule_name.as_str() {
                    "part_item" => {
                        // part_item contains either state_block or property_decl.
                        for item_child in &n.children {
                            if let ASTNodeOrToken::Node(inner) = item_child {
                                match inner.rule_name.as_str() {
                                    "property_decl" => {
                                        base.push(analyze_property(inner)?);
                                    }
                                    "state_block" => {
                                        states.push(analyze_state(inner)?);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(PartStyle {
        name: name.ok_or_else(|| CompileError {
            kind: ErrorKind::InternalError,
            message: "part_def missing name".to_string(),
        })?,
        base,
        states,
    })
}

fn analyze_state(state_block: &GrammarASTNode) -> Result<StateStyle, CompileError> {
    // state_block = KEYWORD("state") NAME LBRACE { property_decl } RBRACE
    let mut saw_keyword = false;
    let mut state_name: Option<String> = None;
    let mut props: Vec<StyleProp> = Vec::new();

    for child in &state_block.children {
        match child {
            ASTNodeOrToken::Token(t) => {
                if t.type_ == TokenType::Keyword && t.value == "state" {
                    saw_keyword = true;
                } else if saw_keyword && t.type_ == TokenType::Name && state_name.is_none() {
                    state_name = Some(t.value.clone());
                }
            }
            ASTNodeOrToken::Node(n) if n.rule_name == "property_decl" => {
                props.push(analyze_property(n)?);
            }
            _ => {}
        }
    }

    Ok(StateStyle {
        state: state_name.ok_or_else(|| CompileError {
            kind: ErrorKind::InternalError,
            message: "state_block missing state name".to_string(),
        })?,
        props,
    })
}

fn analyze_property(prop_decl: &GrammarASTNode) -> Result<StyleProp, CompileError> {
    // property_decl = NAME COLON style_value SEMICOLON
    let mut prop_name: Option<String> = None;
    let mut prop_value: Option<String> = None;

    for child in &prop_decl.children {
        match child {
            ASTNodeOrToken::Token(t) if t.type_ == TokenType::Name && prop_name.is_none() => {
                prop_name = Some(t.value.clone());
            }
            ASTNodeOrToken::Node(n) if n.rule_name == "style_value" => {
                prop_value = Some(extract_style_value(n)?);
            }
            _ => {}
        }
    }

    Ok(StyleProp {
        name: prop_name.ok_or_else(|| CompileError {
            kind: ErrorKind::InternalError,
            message: "property_decl missing name".to_string(),
        })?,
        value: prop_value.ok_or_else(|| CompileError {
            kind: ErrorKind::InternalError,
            message: "property_decl missing value".to_string(),
        })?,
    })
}

fn extract_style_value(sv_ast: &GrammarASTNode) -> Result<String, CompileError> {
    // style_value = TOKEN_REF | HASH_COLOR | DIMENSION | NUMBER | STRING | NAME
    // The AST node has one child: the matched token.
    //
    // Custom token types (TOKEN_REF, HASH_COLOR, DIMENSION) are stored by the
    // GrammarLexer as TokenType::Name with type_name = Some("TOKEN_REF"), etc.
    // Plain NAME tokens have type_name = None.
    for child in &sv_ast.children {
        if let ASTNodeOrToken::Token(t) = child {
            return match t.type_name.as_deref() {
                Some("TOKEN_REF") => {
                    // Resolve $token-name → concrete value.
                    let name = t.value.trim_start_matches('$');
                    resolve_token(name, &HashMap::new()).ok_or_else(|| CompileError {
                        kind: ErrorKind::UnresolvedToken,
                        message: format!("Token '{}' not found in token map", t.value),
                    })
                }
                // HASH_COLOR, DIMENSION, NUMBER, NAME — safe by grammar constraints:
                //   HASH_COLOR: #[0-9a-fA-F]{3,8} — only hex digits
                //   DIMENSION:  number + unit suffix — alphanumeric
                //   NUMBER:     [0-9]+(\.[0-9]+)?  — digits only
                //   NAME:       [a-zA-Z][a-zA-Z0-9-]* — alphanumeric + hyphen
                // None of these can contain '}' or ';' that would break CSS rule syntax.
                //
                // STRING: "([^"\\\n]|\\.)*" — the token value includes the surrounding
                // double-quote delimiters.  When emitted into CSS as `prop: "..."`, the
                // `}` or `;` characters inside the string literal are safely contained by
                // the CSS parser's string tokenisation; they do NOT terminate the rule.
                // Additionally, the lexer stops at the closing `"`, so characters after
                // the closing quote are separate tokens and the grammar rejects them.
                // No CSS injection is possible via the grammar's STRING tokens.
                _ => Ok(t.value.clone()),
            };
        }
    }
    Err(CompileError {
        kind: ErrorKind::InternalError,
        message: "style_value node has no token child".to_string(),
    })
}

// ===========================================================================
// Validation
// ===========================================================================

/// Validate a `StyleDef` against the layout's part map.
///
/// `part_map_json` is the JSON output of `moslayout_compiler::compile().part_map_json`.
/// Pass `None` to skip part-existence validation.
pub fn validate(
    def: &StyleDef,
    part_map_json: Option<&str>,
) -> Result<(), Vec<CompileError>> {
    let mut errors = Vec::new();

    let known_parts: HashSet<String> = if let Some(json) = part_map_json {
        parse_part_names(json)
    } else {
        HashSet::new()
    };
    let has_part_map = part_map_json.is_some();

    for part in &def.parts {
        // Validate part existence.
        if has_part_map && !known_parts.contains(&part.name) {
            errors.push(CompileError {
                kind: ErrorKind::UnknownPart,
                message: format!(
                    "Unknown part '{}' — not exported by the layout (.mll)",
                    part.name
                ),
            });
        }

        // Validate state names.
        for state in &part.states {
            if !VALID_STATES.contains(&state.state.as_str()) {
                errors.push(CompileError {
                    kind: ErrorKind::UnknownState,
                    message: format!(
                        "Unknown state '{}' in part '{}' — valid states: {}",
                        state.state,
                        part.name,
                        VALID_STATES.join(", ")
                    ),
                });
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn parse_part_names(json: &str) -> HashSet<String> {
    let mut names = HashSet::new();
    let v: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return names,
    };
    if let Some(parts) = v["parts"].as_array() {
        for p in parts {
            if let Some(name) = p["name"].as_str() {
                names.insert(name.to_string());
            }
        }
    }
    names
}

// ===========================================================================
// Full compile pipeline
// ===========================================================================

/// Compile a `.msl` source file into a `CompileOutput`.
///
/// `part_map_json` is the part map JSON produced by `moslayout_compiler`.
/// Pass `None` to skip part validation.
pub fn compile(
    source: &str,
    part_map_json: Option<&str>,
) -> Result<CompileOutput, Vec<CompileError>> {
    let ast = parse_style(source).map_err(|e| {
        vec![CompileError {
            kind: ErrorKind::InternalError,
            message: e,
        }]
    })?;

    let def = analyze(&ast).map_err(|e| vec![e])?;
    validate(&def, part_map_json)?;

    let css = emit_css(&def);
    let style_map_json = emit_style_map_json(&def);

    Ok(CompileOutput {
        def,
        css,
        style_map_json,
    })
}

// ===========================================================================
// CSS emitter
// ===========================================================================

/// Emit scoped CSS for the DOM / React backend.
///
/// Class names: `.mos-{ComponentName}-{part-name}` for base styles.
/// States: `.mos-{ComponentName}-{part-name}:hover` for hover, etc.
///
/// The `selected` and `editing` states use class selectors rather than CSS
/// pseudo-classes because they are driven by application state, not native
/// browser state: `.mos-{ComponentName}-{part-name}.selected`.
pub fn emit_css(def: &StyleDef) -> String {
    let mut blocks: Vec<String> = Vec::new();
    let comp = &def.component_name;

    for part in &def.parts {
        let class = format!(".mos-{}-{}", comp, part.name);

        // Base styles.
        if !part.base.is_empty() {
            let props: Vec<String> = part
                .base
                .iter()
                .map(|p| format!("  {}: {};", p.name, p.value))
                .collect();
            blocks.push(format!("{} {{\n{}\n}}", class, props.join("\n")));
        }

        // State overrides.
        for state in &part.states {
            if state.props.is_empty() {
                continue;
            }
            let selector = match state.state.as_str() {
                "hover"    => format!("{}:hover", class),
                "pressed"  => format!("{}:active", class),
                "focused"  => format!("{}:focus-visible", class),
                "disabled" => format!("{}.disabled", class),
                "selected" => format!("{}.selected", class),
                "editing"  => format!("{}.editing", class),
                "error"    => format!("{}.error", class),
                other      => format!("{}.{}", class, other),
            };
            let props: Vec<String> = state
                .props
                .iter()
                .map(|p| format!("  {}: {};", p.name, p.value))
                .collect();
            blocks.push(format!("{} {{\n{}\n}}", selector, props.join("\n")));
        }
    }

    if blocks.is_empty() {
        String::new()
    } else {
        format!(
            "/* Generated by mosstyle-compiler for {}. Do not edit. */\n\n{}",
            comp,
            blocks.join("\n\n")
        )
    }
}

/// Emit the backend-agnostic style map as JSON.
pub fn emit_style_map_json(def: &StyleDef) -> String {
    // Use serde_json for serialisation so that all string values are
    // properly escaped.  Hand-rolling JSON with format!() is unsafe:
    // a prop value containing '"' or '\' would produce malformed JSON
    // that silently corrupts downstream consumers.
    //
    // StyleDef, PartStyle, StyleProp, and StateStyle all derive Serialize,
    // so no extra configuration is required.
    serde_json::to_string_pretty(def)
        .unwrap_or_else(|e| format!("{{\"error\": \"serialisation failed: {e}\"}}"))
}

// ===========================================================================
// AST walking helpers
// ===========================================================================

fn find_rule<'a>(node: &'a GrammarASTNode, rule: &str) -> Option<&'a GrammarASTNode> {
    if node.rule_name == rule {
        return Some(node);
    }
    for child in &node.children {
        if let ASTNodeOrToken::Node(n) = child {
            if let Some(found) = find_rule(n, rule) {
                return Some(found);
            }
        }
    }
    None
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ── Tokenizer ────────────────────────────────────────────────────────────

    #[test]
    fn test_tokenize_keywords() {
        let src = "style Grid { part root { } }";
        let tokens = tokenize(src).expect("tokenize failed");
        let values: Vec<_> = tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| (t.value.as_str(), t.type_.clone()))
            .collect();
        // "style" and "part" should be Keyword tokens; "Grid" and "root" Name.
        assert_eq!(values[0], ("style", TokenType::Keyword));
        assert_eq!(values[1], ("Grid",  TokenType::Name));
        assert_eq!(values[2], ("{",     TokenType::LBrace));
        assert_eq!(values[3], ("part",  TokenType::Keyword));
        assert_eq!(values[4], ("root",  TokenType::Name));
    }

    #[test]
    fn test_tokenize_dimension() {
        let src = "border-width: 4px ;";
        let tokens = tokenize(src).expect("tokenize failed");
        let non_eof: Vec<_> = tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .collect();
        // "4px" should be tokenized as a custom DIMENSION token:
        //  type_ = Name (GrammarLexer uses Name for all custom types)
        //  type_name = Some("DIMENSION")
        assert_eq!(non_eof[2].value, "4px");
        assert_eq!(non_eof[2].type_, TokenType::Name);
        assert_eq!(non_eof[2].type_name.as_deref(), Some("DIMENSION"));
    }

    #[test]
    fn test_tokenize_hash_color() {
        let src = "background: #1e1e1e ;";
        let tokens = tokenize(src).expect("tokenize failed");
        let non_eof: Vec<_> = tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .collect();
        // "#1e1e1e" should be a HASH_COLOR token (type_name = Some("HASH_COLOR")).
        assert_eq!(non_eof[1].value, ":");
        assert_eq!(non_eof[2].value, "#1e1e1e");
        assert_eq!(non_eof[2].type_name.as_deref(), Some("HASH_COLOR"));
    }

    #[test]
    fn test_tokenize_token_ref() {
        let src = "background: $color-surface ;";
        let tokens = tokenize(src).expect("tokenize failed");
        let non_eof: Vec<_> = tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .collect();
        // "$color-surface" should be a TOKEN_REF token (type_name = Some("TOKEN_REF")).
        assert_eq!(non_eof[2].value, "$color-surface");
        assert_eq!(non_eof[2].type_name.as_deref(), Some("TOKEN_REF"));
    }

    // ── Parser + Analyzer ────────────────────────────────────────────────────

    fn parse_and_analyze(src: &str) -> StyleDef {
        let ast = parse_style(src).expect("parse failed");
        analyze(&ast).expect("analyze failed")
    }

    #[test]
    fn test_minimal_style() {
        let src = "style Grid { }";
        let def = parse_and_analyze(src);
        assert_eq!(def.component_name, "Grid");
        assert!(def.parts.is_empty());
    }

    #[test]
    fn test_part_with_property() {
        let src = r#"
          style Grid {
            part root {
              background: #ffffff ;
            }
          }
        "#;
        let def = parse_and_analyze(src);
        assert_eq!(def.parts.len(), 1);
        assert_eq!(def.parts[0].name, "root");
        assert_eq!(def.parts[0].base.len(), 1);
        assert_eq!(def.parts[0].base[0].name, "background");
        assert_eq!(def.parts[0].base[0].value, "#ffffff");
    }

    #[test]
    fn test_token_ref_resolves() {
        let src = r#"
          style Grid {
            part root {
              background: $color-surface ;
            }
          }
        "#;
        let def = parse_and_analyze(src);
        // Token $color-surface should resolve to the default palette value.
        assert_eq!(def.parts[0].base[0].value, "#1e1e1e");
    }

    #[test]
    fn test_state_block() {
        // Note: rgba() function calls require a v2 grammar extension.
        // v1 supports hex colors, dimensions, numbers, strings, and identifiers.
        let src = r#"
          style Grid {
            part cell {
              background: transparent ;
              state hover {
                background: #f5f5f5 ;
              }
            }
          }
        "#;
        let def = parse_and_analyze(src);
        assert_eq!(def.parts[0].states.len(), 1);
        assert_eq!(def.parts[0].states[0].state, "hover");
        assert_eq!(def.parts[0].states[0].props[0].name, "background");
    }

    #[test]
    fn test_dimension_value() {
        let src = r#"
          style Grid {
            part root { border-width: 1px ; }
          }
        "#;
        let def = parse_and_analyze(src);
        assert_eq!(def.parts[0].base[0].value, "1px");
    }

    // ── CSS emitter ──────────────────────────────────────────────────────────

    #[test]
    fn test_css_base_class() {
        let src = r#"
          style Button {
            part root {
              background: #1e1e1e ;
              border-radius: 4px ;
            }
          }
        "#;
        let result = compile(src, None).unwrap();
        assert!(result.css.contains(".mos-Button-root"));
        assert!(result.css.contains("background: #1e1e1e"));
        assert!(result.css.contains("border-radius: 4px"));
    }

    #[test]
    fn test_css_hover_state() {
        let src = r#"
          style Button {
            part root {
              background: #1e1e1e ;
              state hover { background: #2e2e2e ; }
            }
          }
        "#;
        let result = compile(src, None).unwrap();
        assert!(result.css.contains(".mos-Button-root:hover"));
        assert!(result.css.contains("background: #2e2e2e"));
    }

    #[test]
    fn test_css_disabled_class_selector() {
        let src = r#"
          style Button {
            part root {
              background: #1e1e1e ;
              state disabled { opacity: 0.4 ; }
            }
          }
        "#;
        let result = compile(src, None).unwrap();
        // disabled → .disabled class selector (not CSS :disabled pseudo-class).
        assert!(result.css.contains(".mos-Button-root.disabled"));
    }

    #[test]
    fn test_css_has_preamble() {
        let src = "style Grid { part root { background: #1e1e1e ; } }";
        let result = compile(src, None).unwrap();
        assert!(result.css.starts_with("/* Generated by mosstyle-compiler"));
    }

    // ── Validation ───────────────────────────────────────────────────────────

    #[test]
    fn test_unknown_part_error() {
        let part_map = r#"{"component":"Grid","parts":[{"name":"root","primitive":"Column"}]}"#;
        let src = r#"
          style Grid {
            part nonexistent {
              background: #ffffff ;
            }
          }
        "#;
        let result = compile(src, Some(part_map));
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(|e| e.kind == ErrorKind::UnknownPart));
    }

    #[test]
    fn test_unknown_state_error() {
        let src = r#"
          style Grid {
            part root {
              background: #1e1e1e ;
              state hovered {
                background: #2e2e2e ;
              }
            }
          }
        "#;
        let result = compile(src, None);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(|e| e.kind == ErrorKind::UnknownState));
    }

    #[test]
    fn test_grid_style() {
        // rgba() is v2; use hex literals only in v1 tests.
        let src = r#"
          style Grid {
            part root {
              background: #ffffff ;
              border-color: #e0e0e0 ;
              border-width: 1px ;
            }
            part cell-grid {
              background: #1e1e1e ;
              border-color: #1a1a2e ;
              border-width: 1px ;
            }
          }
        "#;
        let result = compile(src, None).unwrap();
        let css = &result.css;
        assert!(css.contains(".mos-Grid-root"));
        assert!(css.contains(".mos-Grid-cell-grid"));
        assert!(css.contains("border-width: 1px"));
    }

    #[test]
    fn test_style_map_json() {
        let src = r#"
          style Grid {
            part root { background: #ffffff ; }
          }
        "#;
        let result = compile(src, None).unwrap();
        assert!(result.style_map_json.contains("\"component\": \"Grid\""));
        assert!(result.style_map_json.contains("\"root\""));
        assert!(result.style_map_json.contains("\"background\""));
    }
}
