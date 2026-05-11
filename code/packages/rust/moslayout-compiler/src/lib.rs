//! # moslayout-compiler — Compiling `.mll` component layout files.
//!
//! `moslayout` is the structural layout language for the Mosaic UI stack.
//! A `.mll` file answers exactly one question: *how are a component's
//! primitives arranged in space, and how do they wire to the component's
//! interface?*
//!
//! It does this by connecting `mosmodel` slot and emit names to a closed
//! vocabulary of layout primitives: `Box`, `Row`, `Column`, `Text`,
//! `Image`, `Spacer`, `Grid`.
//!
//! # Pipeline
//!
//! ```text
//! .mll source  +  interface descriptor JSON (.mil output)
//!       │
//!       ▼  tokenize()
//! Vec<Token>          (moslayout.tokens grammar via GrammarLexer)
//!       │
//!       ▼  parse()
//! GrammarASTNode      (moslayout.grammar via GrammarParser)
//!       │
//!       ▼  analyze()
//! LayoutDef           (typed IR: component name + node tree)
//!       │
//!       ▼  validate()
//! ValidationResult    (slot refs, emit refs, part uniqueness)
//!       │
//!       ▼  emit_part_map_json()
//! String              (part map JSON consumed by mosstyle)
//! ```
//!
//! # Primitives
//!
//! | Name    | Children? | Props                                      |
//! |---------|-----------|---------------------------------------------|
//! | `Box`   | yes       | direction, align, justify, wrap, grow, etc. |
//! | `Row`   | yes       | same as Box (direction fixed to row)        |
//! | `Column`| yes       | same as Box (direction fixed to column)     |
//! | `Text`  | no        | `slot: <name>` (must be text-typed)         |
//! | `Image` | no        | `slot: <name>` (must be image-typed)        |
//! | `Spacer`| no        | optional `grow: <number>`                   |
//! | `Grid`  | no        | `headers: slot: <name>`, `rows: slot: <name>`, … |
//!
//! # Quick start
//!
//! ```no_run
//! use moslayout_compiler::compile;
//!
//! let layout_src = r#"
//!   layout Grid {
//!     Column [ root ] {
//!       Grid [ cell-grid ] (
//!         headers: slot: column-headers ,
//!         rows:    slot: viewport-rows
//!       )
//!     }
//!   }
//! "#;
//!
//! let result = compile(layout_src, None).expect("compilation failed");
//! println!("{}", result.part_map_json);
//! ```

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::{Token, TokenType};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode, GrammarParser};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

mod _grammar;

// ===========================================================================
// The valid primitive node names — validated semantically, not in the grammar.
// ===========================================================================

/// The set of built-in layout primitives.
///
/// Everything else at a node position is either a component reference (upper-
/// case first letter) or a compile error (unknown identifier).
const PRIMITIVES: &[&str] = &[
    "Box", "Row", "Column", "Text", "Image", "Spacer", "Grid",
    // Extended set from the spec (included for completeness):
    "Scroll", "Divider", "Stack", "Icon",
];

fn is_primitive(tag: &str) -> bool {
    PRIMITIVES.contains(&tag)
}

// ===========================================================================
// Public output types
// ===========================================================================

/// The result of a successful `compile()` call.
#[derive(Debug, Clone)]
pub struct CompileOutput {
    /// The analyzed layout IR.
    pub def: LayoutDef,
    /// The list of named parts exported by this layout.
    pub parts: Vec<PartEntry>,
    /// The part map as a JSON string (consumed by mosstyle-compiler).
    pub part_map_json: String,
}

// ===========================================================================
// Layout IR types
// ===========================================================================

/// The analyzed representation of a `.mll` file.
///
/// Produced by `analyze()` from the grammar AST.  Used by `mosaic-driver` to
/// assemble a `MosaicFile` IR for feeding into `MosaicVM`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutDef {
    /// PascalCase component name (matches the `.mil` component name).
    pub component_name: String,
    /// The root node of the layout tree.
    ///
    /// A well-formed `.mll` file has exactly one root node. The grammar
    /// allows multiple top-level nodes; the compiler validates there is one.
    pub root: LayoutNode,
}

/// A node in the layout tree.
///
/// Every node has a tag (the primitive or component name), an optional part
/// name for mosstyle targeting, optional structural properties, and optional
/// child nodes.
///
/// # Examples
///
/// `Column [ root ] { ... }` becomes:
/// ```text
/// LayoutNode { tag: "Column", part_name: Some("root"), props: [], children: [...] }
/// ```
///
/// `Grid [ cell-grid ] ( headers: slot: column-headers , rows: slot: viewport-rows )` becomes:
/// ```text
/// LayoutNode {
///   tag: "Grid",
///   part_name: Some("cell-grid"),
///   props: [
///     LayoutProp { name: "headers", value: SlotRef("column-headers") },
///     LayoutProp { name: "rows",    value: SlotRef("viewport-rows") },
///   ],
///   children: [],
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutNode {
    /// Element type name, e.g. `Column`, `Grid`, `Text`.
    pub tag: String,
    /// Optional part name for mosstyle targeting, e.g. `root`, `cell-grid`.
    pub part_name: Option<String>,
    /// Structural properties (direction, align, slot bindings, etc.).
    pub props: Vec<LayoutProp>,
    /// Child nodes (containers only; leaf nodes like `Grid` have no children).
    pub children: Vec<LayoutNode>,
}

/// A structural property on a layout node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutProp {
    /// Property name in kebab-case, e.g. `headers`, `direction`, `grow`.
    pub name: String,
    /// The property value.
    pub value: LayoutPropValue,
}

/// The value of a structural property.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum LayoutPropValue {
    /// A slot reference: `slot: column-headers`.
    SlotRef(String),
    /// An emit reference: `emit: onNavigate`.
    EmitRef(String),
    /// A keyword value: `row`, `column`, `true`, `false`, `center`, etc.
    Keyword(String),
    /// A numeric value: `1.5`, `0`, `2`.
    Number(f64),
}

/// A named part exported by this layout (consumed by the mosstyle compiler).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartEntry {
    /// The part name, e.g. `root`, `cell-grid`, `header-text`.
    pub name: String,
    /// The primitive tag this part wraps, e.g. `Column`, `Grid`, `Text`.
    pub primitive: String,
}

// ===========================================================================
// Compiler errors
// ===========================================================================

/// A structured compile error from the layout compiler.
#[derive(Debug, Clone, PartialEq)]
pub struct CompileError {
    pub kind: ErrorKind,
    pub message: String,
}

/// Error kinds for the moslayout compiler (§9 of UI14-moslayout.md).
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    /// A slot reference names a slot not declared in the interface descriptor.
    UnknownSlot,
    /// An emit reference names an emit not declared in the interface descriptor.
    UnknownEmit,
    /// Two parts share the same name.
    DuplicatePart,
    /// An unknown identifier appears in node position (not a primitive or component).
    UnknownPrimitive,
    /// The layout body has zero or more than one root nodes.
    BadRootCount,
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
// Tokenizer
// ===========================================================================

/// Tokenize moslayout source text into a flat `Vec<Token>`.
///
/// Whitespace and comments are skipped.  The returned vector ends with EOF.
pub fn tokenize(source: &str) -> Vec<Token> {
    let grammar = _grammar::token_grammar();
    let mut lexer = GrammarLexer::new(source, &grammar);
    lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("moslayout tokenization failed: {e}"))
}

// ===========================================================================
// Parser
// ===========================================================================

/// Parse moslayout source text into a grammar AST.
///
/// The AST mirrors the grammar rules exactly; call `analyze` to convert it
/// to a strongly-typed `LayoutDef`.
pub fn parse_layout(source: &str) -> Result<GrammarASTNode, String> {
    let tokens = tokenize(source);
    let grammar = _grammar::parser_grammar();
    let mut parser = GrammarParser::new(tokens, grammar);
    parser.parse().map_err(|e| format!("parse error: {e}"))
}

// ===========================================================================
// Analyzer — GrammarASTNode → LayoutDef
// ===========================================================================

/// Walk the raw grammar AST and produce a typed `LayoutDef`.
pub fn analyze(ast: &GrammarASTNode) -> Result<LayoutDef, CompileError> {
    // AST root is `file` which contains `layout_def`.
    // layout_def: KEYWORD("layout") NAME LBRACE { node } RBRACE
    let layout_node = find_rule(ast, "layout_def").ok_or_else(|| CompileError {
        kind: ErrorKind::InternalError,
        message: "layout_def rule not found in AST".to_string(),
    })?;

    let component_name = extract_layout_name(layout_node)?;
    let child_nodes = extract_child_nodes(layout_node)?;

    if child_nodes.len() != 1 {
        return Err(CompileError {
            kind: ErrorKind::BadRootCount,
            message: format!(
                "layout '{}' must have exactly one root node, found {}",
                component_name,
                child_nodes.len()
            ),
        });
    }

    Ok(LayoutDef {
        component_name,
        root: child_nodes.into_iter().next().unwrap(),
    })
}

/// Validate a `LayoutDef` against the interface descriptor.
///
/// `interface_json` is the output of `mosmodel_compiler::compile().descriptor_json`.
/// Pass `None` to skip interface validation (useful during development).
pub fn validate(
    def: &LayoutDef,
    interface_json: Option<&str>,
) -> Result<Vec<PartEntry>, Vec<CompileError>> {
    let mut errors = Vec::new();

    // Build known slot/emit name sets from interface descriptor.
    let (known_slots, known_emits) = if let Some(json) = interface_json {
        parse_interface_sets(json)
    } else {
        (HashSet::new(), HashSet::new())
    };
    let has_interface = interface_json.is_some();

    // Collect all parts and validate references.
    let mut parts = Vec::new();
    let mut part_names: HashSet<String> = HashSet::new();

    validate_node(
        &def.root,
        &known_slots,
        &known_emits,
        has_interface,
        &mut parts,
        &mut part_names,
        &mut errors,
    );

    if errors.is_empty() {
        Ok(parts)
    } else {
        Err(errors)
    }
}

fn validate_node(
    node: &LayoutNode,
    known_slots: &HashSet<String>,
    known_emits: &HashSet<String>,
    has_interface: bool,
    parts: &mut Vec<PartEntry>,
    part_names: &mut HashSet<String>,
    errors: &mut Vec<CompileError>,
) {
    // Collect part name.
    if let Some(part) = &node.part_name {
        if part_names.contains(part) {
            errors.push(CompileError {
                kind: ErrorKind::DuplicatePart,
                message: format!("Duplicate part name '{}' in layout", part),
            });
        } else {
            part_names.insert(part.clone());
            parts.push(PartEntry {
                name: part.clone(),
                primitive: node.tag.clone(),
            });
        }
    }

    // Validate slot/emit references in props.
    for prop in &node.props {
        match &prop.value {
            LayoutPropValue::SlotRef(slot_name) => {
                if has_interface && !known_slots.contains(slot_name) {
                    errors.push(CompileError {
                        kind: ErrorKind::UnknownSlot,
                        message: format!(
                            "Unknown slot '{}' referenced in layout — not declared in .mil",
                            slot_name
                        ),
                    });
                }
            }
            LayoutPropValue::EmitRef(emit_name) => {
                if has_interface && !known_emits.contains(emit_name) {
                    errors.push(CompileError {
                        kind: ErrorKind::UnknownEmit,
                        message: format!(
                            "Unknown emit '{}' referenced in layout — not declared in .mil",
                            emit_name
                        ),
                    });
                }
            }
            _ => {}
        }
    }

    // Recurse into children.
    for child in &node.children {
        validate_node(
            child,
            known_slots,
            known_emits,
            has_interface,
            parts,
            part_names,
            errors,
        );
    }
}

// ===========================================================================
// Full compile pipeline
// ===========================================================================

/// Compile a `.mll` source file into a `CompileOutput`.
///
/// `interface_json` is the descriptor JSON produced by `mosmodel_compiler`.
/// Pass `None` to skip interface validation.
///
/// # Errors
///
/// Returns `Err(Vec<CompileError>)` if tokenization, parsing, analysis, or
/// validation fails.
///
/// # Example
///
/// ```no_run
/// use moslayout_compiler::compile;
///
/// let src = r#"
///   layout Grid {
///     Column [ root ] {
///       Grid [ cell-grid ] (
///         headers: slot: column-headers ,
///         rows:    slot: viewport-rows
///       )
///     }
///   }
/// "#;
///
/// let result = compile(src, None).unwrap();
/// println!("Component: {}", result.def.component_name);
/// println!("Parts: {:?}", result.parts);
/// ```
pub fn compile(
    source: &str,
    interface_json: Option<&str>,
) -> Result<CompileOutput, Vec<CompileError>> {
    // Parse.
    let ast = parse_layout(source).map_err(|e| {
        vec![CompileError {
            kind: ErrorKind::InternalError,
            message: e,
        }]
    })?;

    // Analyze.
    let def = analyze(&ast).map_err(|e| vec![e])?;

    // Validate.
    let parts = validate(&def, interface_json)?;

    // Emit part map JSON.
    let part_map_json = emit_part_map_json(&def.component_name, &parts);

    Ok(CompileOutput {
        def,
        parts,
        part_map_json,
    })
}

// ===========================================================================
// Part map JSON emitter
// ===========================================================================

/// Serialize the part map to JSON (consumed by mosstyle-compiler).
///
/// ```json
/// {
///   "component": "Grid",
///   "parts": [
///     { "name": "root",      "primitive": "Column" },
///     { "name": "cell-grid", "primitive": "Grid"   }
///   ]
/// }
/// ```
pub fn emit_part_map_json(component_name: &str, parts: &[PartEntry]) -> String {
    // Hand-roll the JSON to avoid needing serde_json feature complexity.
    let parts_json: Vec<String> = parts
        .iter()
        .map(|p| {
            format!(
                r#"    {{ "name": "{}", "primitive": "{}" }}"#,
                p.name, p.primitive
            )
        })
        .collect();

    format!(
        "{{\n  \"component\": \"{}\",\n  \"parts\": [\n{}\n  ]\n}}",
        component_name,
        parts_json.join(",\n")
    )
}

// ===========================================================================
// AST walking helpers
// ===========================================================================

/// Find the first node with the given `rule_name` anywhere in the AST (DFS).
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

/// Extract the component name from a `layout_def` AST node.
///
/// layout_def = KEYWORD("layout") NAME LBRACE { node } RBRACE
/// The NAME immediately after the KEYWORD is the component name.
fn extract_layout_name(layout_def: &GrammarASTNode) -> Result<String, CompileError> {
    let mut saw_keyword = false;
    for child in &layout_def.children {
        if let ASTNodeOrToken::Token(t) = child {
            if t.type_ == TokenType::Keyword && t.value == "layout" {
                saw_keyword = true;
            } else if saw_keyword && t.type_ == TokenType::Name {
                return Ok(t.value.clone());
            }
        }
    }
    Err(CompileError {
        kind: ErrorKind::InternalError,
        message: "Could not extract component name from layout_def".to_string(),
    })
}

/// Extract all top-level `node` child rules from a `layout_def`.
///
/// The `{ node }` repetition inside `layout_def` creates a sequence of `node`
/// ASTNodes as direct children of `layout_def`.
fn extract_child_nodes(layout_def: &GrammarASTNode) -> Result<Vec<LayoutNode>, CompileError> {
    let mut nodes = Vec::new();
    for child in &layout_def.children {
        if let ASTNodeOrToken::Node(n) = child {
            if n.rule_name == "node" {
                nodes.push(analyze_node(n)?);
            }
        }
    }
    Ok(nodes)
}

/// Analyze a `node` AST node into a `LayoutNode`.
///
/// Grammar: `node = NAME [ part_name ] [ LPAREN prop_list RPAREN ] [ LBRACE { node } RBRACE ]`
///
/// The children of a `node` ASTNode may contain (in order):
/// - Token(NAME)                         — the primitive tag
/// - ASTNode("part_name")               — optional
/// - Token(LPAREN), ASTNode("prop_list"), Token(RPAREN) — optional
/// - Token(LBRACE), ASTNode("node")*, Token(RBRACE)    — optional
fn analyze_node(node_ast: &GrammarASTNode) -> Result<LayoutNode, CompileError> {
    let children = &node_ast.children;
    let mut idx = 0;

    // ── TAG ──────────────────────────────────────────────────────────────────
    // First child must be the NAME token for the primitive tag.
    let tag = match children.get(idx) {
        Some(ASTNodeOrToken::Token(t)) if t.type_ == TokenType::Name => {
            idx += 1;
            t.value.clone()
        }
        _ => {
            return Err(CompileError {
                kind: ErrorKind::InternalError,
                message: format!("Expected NAME token at start of node, got {:?}", children.get(0)),
            });
        }
    };

    // ── PART NAME (optional) ────────────────────────────────────────────────
    let part_name = if let Some(ASTNodeOrToken::Node(n)) = children.get(idx) {
        if n.rule_name == "part_name" {
            idx += 1;
            Some(extract_part_name(n)?)
        } else {
            None
        }
    } else {
        None
    };

    // ── PROPS (optional) ────────────────────────────────────────────────────
    // Signals: Token(LPAREN) at current position.
    let props = if matches!(children.get(idx), Some(ASTNodeOrToken::Token(t)) if t.value == "(") {
        idx += 1; // skip LPAREN
        let prop_list_node = match children.get(idx) {
            Some(ASTNodeOrToken::Node(n)) if n.rule_name == "prop_list" => {
                idx += 1;
                n
            }
            _ => {
                return Err(CompileError {
                    kind: ErrorKind::InternalError,
                    message: "Expected prop_list after LPAREN in node".to_string(),
                });
            }
        };
        let props = extract_prop_list(prop_list_node)?;
        // Skip RPAREN.
        if matches!(children.get(idx), Some(ASTNodeOrToken::Token(t)) if t.value == ")") {
            idx += 1;
        }
        props
    } else {
        Vec::new()
    };

    // ── CHILDREN (optional) ─────────────────────────────────────────────────
    // Signals: Token(LBRACE) at current position.
    let child_nodes = if matches!(children.get(idx), Some(ASTNodeOrToken::Token(t)) if t.value == "{") {
        idx += 1; // skip LBRACE
        let mut nodes = Vec::new();
        while let Some(child) = children.get(idx) {
            match child {
                ASTNodeOrToken::Node(n) if n.rule_name == "node" => {
                    nodes.push(analyze_node(n)?);
                    idx += 1;
                }
                ASTNodeOrToken::Token(t) if t.value == "}" => {
                    idx += 1; // skip RBRACE
                    break;
                }
                _ => {
                    idx += 1; // skip unexpected (RBRACE usually)
                }
            }
        }
        nodes
    } else {
        Vec::new()
    };

    Ok(LayoutNode {
        tag,
        part_name,
        props,
        children: child_nodes,
    })
}

/// Extract the part name from a `part_name` AST node.
///
/// Grammar: `part_name = LBRACKET NAME RBRACKET`
fn extract_part_name(part_name_ast: &GrammarASTNode) -> Result<String, CompileError> {
    for child in &part_name_ast.children {
        if let ASTNodeOrToken::Token(t) = child {
            if t.type_ == TokenType::Name {
                return Ok(t.value.clone());
            }
        }
    }
    Err(CompileError {
        kind: ErrorKind::InternalError,
        message: "Could not extract name from part_name node".to_string(),
    })
}

/// Extract all props from a `prop_list` AST node.
///
/// Grammar: `prop_list = prop { COMMA prop }`
fn extract_prop_list(prop_list_ast: &GrammarASTNode) -> Result<Vec<LayoutProp>, CompileError> {
    let mut props = Vec::new();
    for child in &prop_list_ast.children {
        match child {
            ASTNodeOrToken::Node(n) if n.rule_name == "prop" => {
                props.push(extract_prop(n)?);
            }
            // COMMA tokens are structural noise — skip.
            _ => {}
        }
    }
    Ok(props)
}

/// Extract a single `prop` from a `prop` AST node.
///
/// The grammar supports two alternatives:
///
/// **Named form** — `NAME COLON prop_value`
///
/// ```text
/// direction: row
/// headers:   slot: column-headers
/// grow:      1.5
/// ```
///
/// **Shorthand form** — `KEYWORD COLON NAME`
///
/// ```text
/// slot: label         →  prop name = "slot",  value = SlotRef("label")
/// emit: onNavigate    →  prop name = "emit",  value = EmitRef("onNavigate")
/// ```
///
/// The shorthand is sugar for single-slot leaf nodes (Text, Image) where
/// the binding target is unambiguous and writing `content: slot: label` is
/// unnecessarily verbose.
fn extract_prop(prop_ast: &GrammarASTNode) -> Result<LayoutProp, CompileError> {
    let children = &prop_ast.children;

    // ── Shorthand detection ─────────────────────────────────────────────────
    // If the first token is a KEYWORD (slot/emit), this is the shorthand form.
    // Children: Token(KEYWORD) Token(COLON) Token(NAME)
    if let Some(ASTNodeOrToken::Token(first)) = children.first() {
        if first.type_ == TokenType::Keyword {
            // Shorthand: KEYWORD COLON NAME
            // Prop name is the keyword itself ("slot" or "emit").
            // Prop value is derived from the NAME token that follows.
            let prop_name = first.value.clone();
            let slot_name = children
                .iter()
                .filter_map(|c| {
                    if let ASTNodeOrToken::Token(t) = c {
                        if t.type_ == TokenType::Name { Some(t.value.clone()) } else { None }
                    } else {
                        None
                    }
                })
                .next()
                .ok_or_else(|| CompileError {
                    kind: ErrorKind::InternalError,
                    message: format!(
                        "Shorthand prop '{}:' missing target name",
                        prop_name
                    ),
                })?;

            let value = if prop_name == "slot" {
                LayoutPropValue::SlotRef(slot_name)
            } else if prop_name == "emit" {
                LayoutPropValue::EmitRef(slot_name)
            } else {
                return Err(CompileError {
                    kind: ErrorKind::InternalError,
                    message: format!(
                        "Unknown shorthand keyword '{}' (expected 'slot' or 'emit')",
                        prop_name
                    ),
                });
            };

            return Ok(LayoutProp { name: prop_name, value });
        }
    }

    // ── Named form ──────────────────────────────────────────────────────────
    // Children: Token(NAME) Token(COLON) ASTNode("prop_value")
    let mut name: Option<String> = None;
    let mut value: Option<LayoutPropValue> = None;

    for child in children {
        match child {
            ASTNodeOrToken::Token(t) if t.type_ == TokenType::Name && name.is_none() => {
                name = Some(t.value.clone());
            }
            ASTNodeOrToken::Token(t) if t.value == ":" => {}
            ASTNodeOrToken::Node(n) if n.rule_name == "prop_value" => {
                value = Some(extract_prop_value(n)?);
            }
            _ => {}
        }
    }

    Ok(LayoutProp {
        name: name.ok_or_else(|| CompileError {
            kind: ErrorKind::InternalError,
            message: "prop missing name".to_string(),
        })?,
        value: value.ok_or_else(|| CompileError {
            kind: ErrorKind::InternalError,
            message: "prop missing value".to_string(),
        })?,
    })
}

/// Extract a `prop_value` from its AST node.
///
/// Grammar: `prop_value = KEYWORD COLON NAME | NAME | NUMBER`
///
/// The three alternatives are distinguished by the first child token's type:
/// - Keyword → slot/emit binding
/// - Name    → keyword value
/// - Number  → numeric value
fn extract_prop_value(pv_ast: &GrammarASTNode) -> Result<LayoutPropValue, CompileError> {
    let children = &pv_ast.children;

    match children.as_slice() {
        // KEYWORD COLON NAME — slot: column-headers OR emit: onNavigate
        [
            ASTNodeOrToken::Token(kw),
            ASTNodeOrToken::Token(_colon),
            ASTNodeOrToken::Token(name_tok),
        ] if kw.type_ == TokenType::Keyword => {
            let ref_name = name_tok.value.clone();
            if kw.value == "slot" {
                Ok(LayoutPropValue::SlotRef(ref_name))
            } else if kw.value == "emit" {
                Ok(LayoutPropValue::EmitRef(ref_name))
            } else {
                Err(CompileError {
                    kind: ErrorKind::InternalError,
                    message: format!(
                        "Unknown binding keyword '{}' in prop_value (expected 'slot' or 'emit')",
                        kw.value
                    ),
                })
            }
        }
        // NAME — keyword value: row, column, true, false, center, …
        [ASTNodeOrToken::Token(t)] if t.type_ == TokenType::Name => {
            Ok(LayoutPropValue::Keyword(t.value.clone()))
        }
        // NUMBER — numeric value: 1.5, 0, 2
        [ASTNodeOrToken::Token(t)] if t.type_ == TokenType::Number => {
            let n = t.value.parse::<f64>().map_err(|_| CompileError {
                kind: ErrorKind::InternalError,
                message: format!("Invalid number literal '{}'", t.value),
            })?;
            Ok(LayoutPropValue::Number(n))
        }
        other => Err(CompileError {
            kind: ErrorKind::InternalError,
            message: format!("Unexpected prop_value shape: {:?}", other.len()),
        }),
    }
}

// ===========================================================================
// Interface descriptor parsing (for validation)
// ===========================================================================

/// Parse slot names and emit names from an interface descriptor JSON.
///
/// The descriptor JSON is produced by `mosmodel_compiler::compile()`.
/// This is a minimal parser — we only need name sets, not full types.
fn parse_interface_sets(json: &str) -> (HashSet<String>, HashSet<String>) {
    let mut slots = HashSet::new();
    let mut emits = HashSet::new();

    // Simple string scanning: look for "name": "..." inside slot/emit arrays.
    // Works for the JSON format produced by mosmodel-compiler.
    let v: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return (slots, emits),
    };

    if let Some(slot_arr) = v["slots"].as_array() {
        for s in slot_arr {
            if let Some(name) = s["name"].as_str() {
                slots.insert(name.to_string());
            }
        }
    }
    if let Some(emit_arr) = v["emits"].as_array() {
        for e in emit_arr {
            if let Some(name) = e["name"].as_str() {
                emits.insert(name.to_string());
            }
        }
    }

    (slots, emits)
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
        let src = "layout Grid { }";
        let tokens = tokenize(src);
        let non_eof: Vec<_> = tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .collect();
        // "layout" → Keyword, "Grid" → Name, "{" → Lbrace, "}" → Rbrace
        assert_eq!(non_eof[0].value, "layout");
        assert_eq!(non_eof[0].type_, TokenType::Keyword);
        assert_eq!(non_eof[1].value, "Grid");
        assert_eq!(non_eof[1].type_, TokenType::Name);
    }

    #[test]
    fn test_tokenize_slot_keyword() {
        let src = "slot column-headers";
        let tokens = tokenize(src);
        let non_eof: Vec<_> = tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .collect();
        assert_eq!(non_eof[0].value, "slot");
        assert_eq!(non_eof[0].type_, TokenType::Keyword);
        assert_eq!(non_eof[1].value, "column-headers");
        assert_eq!(non_eof[1].type_, TokenType::Name);
    }

    #[test]
    fn test_tokenize_brackets() {
        let src = "Column [ root ]";
        let tokens = tokenize(src);
        let values: Vec<_> = tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(values, &["Column", "[", "root", "]"]);
    }

    #[test]
    fn test_tokenize_number() {
        let src = "grow: 1.5";
        let tokens = tokenize(src);
        let non_eof: Vec<_> = tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .collect();
        assert_eq!(non_eof[2].value, "1.5");
        assert_eq!(non_eof[2].type_, TokenType::Number);
    }

    // ── Parser + Analyzer ────────────────────────────────────────────────────

    fn parse_and_analyze(src: &str) -> LayoutDef {
        let ast = parse_layout(src).expect("parse failed");
        analyze(&ast).expect("analyze failed")
    }

    #[test]
    fn test_minimal_layout() {
        let src = "layout Button { Box { } }";
        let def = parse_and_analyze(src);
        assert_eq!(def.component_name, "Button");
        assert_eq!(def.root.tag, "Box");
        assert!(def.root.children.is_empty());
    }

    #[test]
    fn test_layout_with_part_name() {
        let src = "layout Button { Box [ root ] { } }";
        let def = parse_and_analyze(src);
        assert_eq!(def.root.part_name, Some("root".to_string()));
    }

    #[test]
    fn test_layout_nested_children() {
        let src = r#"
          layout FormulaBar {
            Row [ root ] {
              Text [ address ] ( slot: cell-address )
              Text [ formula ] ( slot: formula )
            }
          }
        "#;
        let def = parse_and_analyze(src);
        assert_eq!(def.component_name, "FormulaBar");
        assert_eq!(def.root.tag, "Row");
        assert_eq!(def.root.children.len(), 2);
        assert_eq!(def.root.children[0].tag, "Text");
        assert_eq!(def.root.children[0].part_name, Some("address".to_string()));
        assert_eq!(def.root.children[1].tag, "Text");
    }

    #[test]
    fn test_slot_binding_prop() {
        let src = r#"
          layout Grid {
            Column [ root ] {
              Grid [ cell-grid ] (
                headers: slot: column-headers ,
                rows:    slot: viewport-rows
              )
            }
          }
        "#;
        let def = parse_and_analyze(src);
        assert_eq!(def.component_name, "Grid");
        assert_eq!(def.root.tag, "Column");
        assert_eq!(def.root.children.len(), 1);

        let grid = &def.root.children[0];
        assert_eq!(grid.tag, "Grid");
        assert_eq!(grid.part_name, Some("cell-grid".to_string()));
        assert_eq!(grid.props.len(), 2);

        assert_eq!(grid.props[0].name, "headers");
        assert_eq!(
            grid.props[0].value,
            LayoutPropValue::SlotRef("column-headers".to_string())
        );
        assert_eq!(grid.props[1].name, "rows");
        assert_eq!(
            grid.props[1].value,
            LayoutPropValue::SlotRef("viewport-rows".to_string())
        );
    }

    #[test]
    fn test_keyword_value_prop() {
        let src = r#"
          layout Button {
            Box [ root ] ( direction: row ) { }
          }
        "#;
        let def = parse_and_analyze(src);
        let root = &def.root;
        assert_eq!(root.tag, "Box");
        assert_eq!(root.props.len(), 1);
        assert_eq!(root.props[0].name, "direction");
        assert_eq!(root.props[0].value, LayoutPropValue::Keyword("row".to_string()));
    }

    #[test]
    fn test_numeric_prop() {
        let src = "layout Spacer { Spacer ( grow: 2 ) }";
        let def = parse_and_analyze(src);
        let root = &def.root;
        assert_eq!(root.tag, "Spacer");
        assert_eq!(root.props.len(), 1);
        assert_eq!(root.props[0].name, "grow");
        assert_eq!(root.props[0].value, LayoutPropValue::Number(2.0));
    }

    #[test]
    fn test_emit_binding_prop() {
        let src = r#"
          layout Button {
            Box [ root ] ( focusable: true , connects: onClick ) { }
          }
        "#;
        let def = parse_and_analyze(src);
        // connects: onClick → but "onClick" is a NAME not a KEYWORD COLON NAME form.
        // The connects property uses `emit:` keyword for formal emit wiring.
        // For now, bare identifier like "onClick" → Keyword("onClick").
        let root = &def.root;
        assert_eq!(root.props.len(), 2);
        assert_eq!(root.props[0].name, "focusable");
        assert_eq!(root.props[0].value, LayoutPropValue::Keyword("true".to_string()));
        assert_eq!(root.props[1].name, "connects");
        assert_eq!(root.props[1].value, LayoutPropValue::Keyword("onClick".to_string()));
    }

    // ── Validation ───────────────────────────────────────────────────────────

    #[test]
    fn test_part_map_collected() {
        let src = r#"
          layout Grid {
            Column [ root ] {
              Grid [ cell-grid ] (
                headers: slot: column-headers ,
                rows:    slot: viewport-rows
              )
            }
          }
        "#;
        let result = compile(src, None).unwrap();
        assert_eq!(result.parts.len(), 2);
        let names: Vec<_> = result.parts.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"root"));
        assert!(names.contains(&"cell-grid"));
    }

    #[test]
    fn test_duplicate_part_error() {
        let src = r#"
          layout Grid {
            Column [ root ] {
              Grid [ root ] (
                headers: slot: column-headers ,
                rows:    slot: viewport-rows
              )
            }
          }
        "#;
        let result = compile(src, None);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(|e| e.kind == ErrorKind::DuplicatePart));
    }

    #[test]
    fn test_part_map_json_format() {
        let src = "layout Button { Box [ root ] { } }";
        let result = compile(src, None).unwrap();
        let json = &result.part_map_json;
        assert!(json.contains("\"component\": \"Button\""));
        assert!(json.contains("\"name\": \"root\""));
        assert!(json.contains("\"primitive\": \"Box\""));
    }

    #[test]
    fn test_interface_validation_unknown_slot() {
        let interface_json = r#"{
            "component": "Grid",
            "slots": [{ "name": "column-headers", "type": "list" }],
            "emits": []
        }"#;
        let src = r#"
          layout Grid {
            Column [ root ] {
              Grid [ cell-grid ] (
                headers: slot: column-headers ,
                rows:    slot: nonexistent-slot
              )
            }
          }
        "#;
        let result = compile(src, Some(interface_json));
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(|e| e.kind == ErrorKind::UnknownSlot));
    }

    #[test]
    fn test_interface_validation_passes() {
        let interface_json = r#"{
            "component": "Grid",
            "slots": [
                { "name": "column-headers", "type": "list" },
                { "name": "viewport-rows", "type": "list" }
            ],
            "emits": []
        }"#;
        let src = r#"
          layout Grid {
            Column [ root ] {
              Grid [ cell-grid ] (
                headers: slot: column-headers ,
                rows:    slot: viewport-rows
              )
            }
          }
        "#;
        let result = compile(src, Some(interface_json));
        assert!(result.is_ok());
    }

    #[test]
    fn test_formula_bar_layout() {
        let src = r#"
          layout FormulaBar {
            Row [ root ] {
              Text [ address ] ( slot: cell-address )
              Box  [ divider ] { }
              Text [ formula ] ( slot: formula )
            }
          }
        "#;
        let result = compile(src, None).unwrap();
        assert_eq!(result.def.component_name, "FormulaBar");
        assert_eq!(result.parts.len(), 4); // root, address, divider, formula
    }

    #[test]
    fn test_single_root_required() {
        // Zero root nodes.
        let src = "layout Empty { }";
        let result = compile(src, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_button_layout() {
        let src = r#"
          layout Button {
            Box [ root ] ( direction: row ) {
              Text [ label ] ( slot: label )
            }
          }
        "#;
        let result = compile(src, None).unwrap();
        assert_eq!(result.def.component_name, "Button");
        assert_eq!(result.def.root.tag, "Box");
        assert_eq!(result.def.root.children.len(), 1);
        assert_eq!(result.def.root.children[0].tag, "Text");
    }
}
