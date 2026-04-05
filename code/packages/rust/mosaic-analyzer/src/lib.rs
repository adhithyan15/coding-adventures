//! # mosaic-analyzer — Validating the Mosaic AST and producing a typed `MosaicComponent`.
//!
//! The analyzer is the third stage of the Mosaic compiler pipeline:
//!
//! ```text
//! Source text → Lexer → Tokens → Parser → ASTNode → **Analyzer** → MosaicComponent
//! ```
//!
//! ## What the analyzer does
//!
//! The raw AST from the parser is an untyped tree of rule matches and tokens.
//! The analyzer walks this tree and produces a strongly-typed IR:
//!
//! 1. **Strip syntax noise** — keywords, semicolons, braces are discarded.
//! 2. **Resolve slot types** — keyword strings `"text"`, `"bool"`, etc. become
//!    `MosaicType` enum variants.
//! 3. **Normalize values** — `"16dp"` → `MosaicValue::Dimension(16.0, "dp")`,
//!    `"#2563eb"` → `MosaicValue::Color(r,g,b,a)`, etc.
//! 4. **Classify nodes** — nodes whose names are in the primitive set get
//!    `is_primitive = true`; all others (imported component types) get `false`.
//!
//! ## Public types
//!
//! All IR types are exported from this crate so `mosaic-vm` and backends can
//! depend only on `mosaic-analyzer` (not the lexer/parser).
//!
//! ## Design note
//!
//! This analyzer is **permissive by design**: it does not enforce that slot
//! types match property usages or that all required slots are filled. Those
//! checks belong in a stricter validation pass. The goal here is to produce
//! a valid IR for any syntactically correct `.mosaic` file.

use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use mosaic_parser::parse;

// ===========================================================================
// Primitive node set
// ===========================================================================

/// The built-in layout/display elements. Any other name is a component type.
///
/// Primitives: Row, Column, Box, Stack, Text, Image, Icon, Spacer, Divider, Scroll.
fn is_primitive_node(tag: &str) -> bool {
    matches!(
        tag,
        "Row" | "Column" | "Box" | "Stack"
            | "Text" | "Image" | "Icon"
            | "Spacer" | "Divider" | "Scroll"
    )
}

// ===========================================================================
// Public IR types
// ===========================================================================

/// The analyzed representation of a `.mosaic` file.
///
/// Contains the single component declaration plus all `import` statements.
#[derive(Debug, Clone, PartialEq)]
pub struct MosaicFile {
    /// The component declared in this file.
    pub component: MosaicComponent,
    /// All `import X from "..."` declarations.
    pub imports: Vec<MosaicImport>,
}

/// A Mosaic component — name, slots, and root visual tree.
#[derive(Debug, Clone, PartialEq)]
pub struct MosaicComponent {
    /// PascalCase name, e.g. `ProfileCard`.
    pub name: String,
    /// Typed slot declarations (the component's data inputs).
    pub slots: Vec<MosaicSlot>,
    /// Root node of the visual hierarchy.
    pub root: MosaicNode,
}

/// An `import X from "..."` declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct MosaicImport {
    /// The exported component name (the `X` in `import X from …`).
    pub name: String,
    /// Optional local alias (`Y` in `import X as Y from …`).
    pub alias: Option<String>,
    /// Import path string (e.g. `"./button.mosaic"`).
    pub path: String,
}

/// A typed slot declaration.
///
/// Example: `slot padding: number = 0;`
/// → `MosaicSlot { name: "padding", slot_type: MosaicType::Primitive("number"),
///                 default_value: Some(MosaicValue::Number(0.0, None)) }`
#[derive(Debug, Clone, PartialEq)]
pub struct MosaicSlot {
    /// Slot name in kebab-case (e.g. `avatar-url`).
    pub name: String,
    /// The slot's declared type.
    pub slot_type: MosaicType,
    /// Optional default value (present when `= value` appears in the source).
    pub default_value: Option<MosaicValue>,
    /// `true` if the slot has no default — the host must provide a value.
    pub required: bool,
}

/// The type system for Mosaic slot declarations.
///
/// Primitive types are `text`, `number`, `bool`, `image`, `color`, `node`.
/// Component types reference imported or self-referencing component names.
/// List types hold an element type.
#[derive(Debug, Clone, PartialEq)]
pub enum MosaicType {
    /// A built-in primitive type keyword. Values: `"text"`, `"number"`,
    /// `"bool"`, `"image"`, `"color"`, `"node"`.
    Primitive(String),
    /// A named component type (from an import or self-reference).
    Component(String),
    /// A parameterized list type: `list<ElementType>`.
    List(Box<MosaicType>),
}

/// A visual node in the component tree.
#[derive(Debug, Clone, PartialEq)]
pub struct MosaicNode {
    /// Element type name (e.g. `Row`, `Column`, `Text`, `Button`).
    pub node_type: String,
    /// Whether this is a Mosaic primitive node (Row, Column, Text, etc.).
    pub is_primitive: bool,
    /// Property assignments (`name: value` pairs).
    pub properties: Vec<MosaicProperty>,
    /// Direct children.
    pub children: Vec<MosaicChild>,
}

/// A property assignment on a node (`name: value`).
#[derive(Debug, Clone, PartialEq)]
pub struct MosaicProperty {
    pub name: String,
    pub value: MosaicValue,
}

/// A child of a node — one of four forms.
#[derive(Debug, Clone, PartialEq)]
pub enum MosaicChild {
    /// A nested node element.
    Node(MosaicNode),
    /// A slot reference used as a child: `@header;`.
    SlotRef(String),
    /// Conditional subtree: `when @show { ... }`.
    When {
        slot: String,
        body: Vec<MosaicChild>,
    },
    /// Iterating subtree: `each @items as item { ... }`.
    Each {
        slot: String,
        item_name: String,
        body: Vec<MosaicChild>,
    },
}

/// A property value or slot default value.
///
/// | Source text    | Variant                               |
/// |----------------|---------------------------------------|
/// | `@title`       | `SlotRef("title")`                    |
/// | `"hello"`      | `Literal("hello")`                    |
/// | `42`, `-3.14`  | `Number(42.0, None)`                  |
/// | `16dp`         | `Number(16.0, Some("dp"))`            |
/// | `#2563eb`      | `Color(0x25, 0x63, 0xeb, 0xff)`       |
/// | `true`/`false` | `Bool(true)` / `Bool(false)`          |
/// | `center`       | `Ident("center")`                     |
/// | `align.center` | `Enum("align", "center")`             |
#[derive(Debug, Clone, PartialEq)]
pub enum MosaicValue {
    /// A slot reference: `@slot_name`.
    SlotRef(String),
    /// A string literal (quotes stripped).
    Literal(String),
    /// A number, optionally with a unit suffix (dimensions).
    Number(f64, Option<String>),
    /// A hex color parsed to RGBA bytes.
    Color(u8, u8, u8, u8),
    /// A boolean keyword.
    Bool(bool),
    /// A bare identifier used as a value (e.g. `center`, `auto`).
    Ident(String),
    /// A dotted namespace.member reference (e.g. `heading.large`).
    Enum(String, String),
}

// ===========================================================================
// Errors
// ===========================================================================

/// Error produced by the analyzer when the AST has unexpected structure.
///
/// These are "should not happen" errors that indicate either a parser bug or
/// a malformed AST. Syntactic errors are caught earlier by the parser.
#[derive(Debug, Clone, PartialEq)]
pub struct AnalyzeError(pub String);

impl std::fmt::Display for AnalyzeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AnalyzeError: {}", self.0)
    }
}

impl std::error::Error for AnalyzeError {}

// ===========================================================================
// Public API
// ===========================================================================

/// Analyze Mosaic source text and return a typed `MosaicFile`.
///
/// # Errors
///
/// Returns `AnalyzeError` if the AST has unexpected structure.
/// Returns a panic if parsing/lexing fails (use `parse()` directly for more
/// control over parse errors).
///
/// # Example
///
/// ```no_run
/// use mosaic_analyzer::analyze;
///
/// let file = analyze(r#"
///   component Label {
///     slot text: text;
///     Text { content: @text; }
///   }
/// "#).unwrap();
/// assert_eq!(file.component.name, "Label");
/// assert_eq!(file.component.slots.len(), 1);
/// ```
pub fn analyze(source: &str) -> Result<MosaicFile, AnalyzeError> {
    let ast = parse(source);
    analyze_ast(&ast)
}

/// Analyze a pre-parsed AST node.
pub fn analyze_ast(ast: &GrammarASTNode) -> Result<MosaicFile, AnalyzeError> {
    if ast.rule_name != "file" {
        return Err(AnalyzeError(format!(
            "Expected root rule 'file', got '{}'",
            ast.rule_name
        )));
    }
    analyze_file(ast)
}

// ===========================================================================
// File-level analysis
// ===========================================================================

fn analyze_file(ast: &GrammarASTNode) -> Result<MosaicFile, AnalyzeError> {
    let mut imports = Vec::new();
    let mut component_decl: Option<&GrammarASTNode> = None;

    for child in &ast.children {
        if let ASTNodeOrToken::Node(n) = child {
            match n.rule_name.as_str() {
                "import_decl" => imports.push(analyze_import(n)?),
                "component_decl" => component_decl = Some(n),
                _ => {}
            }
        }
    }

    let component_decl =
        component_decl.ok_or_else(|| AnalyzeError("No component declaration found".into()))?;

    let component = analyze_component(component_decl)?;
    Ok(MosaicFile { component, imports })
}

// ===========================================================================
// Import analysis
// ===========================================================================

fn analyze_import(node: &GrammarASTNode) -> Result<MosaicImport, AnalyzeError> {
    // import_decl = KEYWORD NAME [ KEYWORD NAME ] KEYWORD STRING SEMICOLON ;
    // Tokens in sequence: "import", NAME(component), ["as", NAME(alias)], "from", STRING(path), ";"
    let names: Vec<String> = direct_token_values(node, "NAME");
    let strings: Vec<String> = direct_token_values(node, "STRING");

    if names.is_empty() {
        return Err(AnalyzeError("import_decl missing component name".into()));
    }
    if strings.is_empty() {
        return Err(AnalyzeError("import_decl missing path".into()));
    }

    let name = names[0].clone();
    let alias = if names.len() >= 2 {
        Some(names[1].clone())
    } else {
        None
    };
    let path = strings[0].clone();

    Ok(MosaicImport { name, alias, path })
}

// ===========================================================================
// Component analysis
// ===========================================================================

fn analyze_component(node: &GrammarASTNode) -> Result<MosaicComponent, AnalyzeError> {
    // component_decl = KEYWORD NAME LBRACE { slot_decl } node_tree RBRACE ;
    let names = direct_token_values(node, "NAME");
    if names.is_empty() {
        return Err(AnalyzeError("component_decl missing name".into()));
    }
    let name = names[0].clone();

    let mut slots = Vec::new();
    let mut tree_node: Option<&GrammarASTNode> = None;

    for child in &node.children {
        if let ASTNodeOrToken::Node(n) = child {
            match n.rule_name.as_str() {
                "slot_decl" => slots.push(analyze_slot(n)?),
                "node_tree" => tree_node = Some(n),
                _ => {}
            }
        }
    }

    let tree_node =
        tree_node.ok_or_else(|| AnalyzeError(format!("component '{name}' has no node tree")))?;
    let root = analyze_node_tree(tree_node)?;

    Ok(MosaicComponent { name, slots, root })
}

// ===========================================================================
// Slot analysis
// ===========================================================================

fn analyze_slot(node: &GrammarASTNode) -> Result<MosaicSlot, AnalyzeError> {
    // slot_decl = KEYWORD NAME COLON slot_type [ EQUALS default_value ] SEMICOLON ;
    let names = direct_token_values(node, "NAME");
    if names.is_empty() {
        return Err(AnalyzeError("slot_decl missing name".into()));
    }
    let name = names[0].clone();

    let slot_type_node = find_child(node, "slot_type")
        .ok_or_else(|| AnalyzeError(format!("slot '{name}' missing type")))?;
    let slot_type = analyze_slot_type(slot_type_node)?;

    let default_value = if let Some(dv_node) = find_child(node, "default_value") {
        Some(analyze_default_value(dv_node)?)
    } else {
        None
    };

    let required = default_value.is_none();

    Ok(MosaicSlot {
        name,
        slot_type,
        default_value,
        required,
    })
}

fn analyze_slot_type(node: &GrammarASTNode) -> Result<MosaicType, AnalyzeError> {
    // slot_type = KEYWORD | NAME | list_type
    if let Some(list_node) = find_child(node, "list_type") {
        return analyze_list_type(list_node);
    }

    // Direct tokens
    if let Some(kw) = first_token_value(node, "KEYWORD") {
        return Ok(MosaicType::Primitive(kw));
    }
    if let Some(name) = first_token_value(node, "NAME") {
        return Ok(MosaicType::Component(name));
    }

    Err(AnalyzeError("slot_type has no recognizable content".into()))
}

fn analyze_list_type(node: &GrammarASTNode) -> Result<MosaicType, AnalyzeError> {
    // list_type = KEYWORD LANGLE slot_type RANGLE
    let elem_node = find_child(node, "slot_type")
        .ok_or_else(|| AnalyzeError("list_type missing element type".into()))?;
    let elem_type = analyze_slot_type(elem_node)?;
    Ok(MosaicType::List(Box::new(elem_type)))
}

fn analyze_default_value(node: &GrammarASTNode) -> Result<MosaicValue, AnalyzeError> {
    // default_value = STRING | NUMBER | DIMENSION | COLOR_HEX | KEYWORD
    if let Some(s) = first_token_value(node, "STRING") {
        return Ok(MosaicValue::Literal(s));
    }
    if let Some(dim) = first_token_value(node, "DIMENSION") {
        return parse_dimension(&dim);
    }
    if let Some(num) = first_token_value(node, "NUMBER") {
        let v: f64 = num.parse().map_err(|_| AnalyzeError(format!("Invalid number: {num}")))?;
        return Ok(MosaicValue::Number(v, None));
    }
    if let Some(color) = first_token_value(node, "COLOR_HEX") {
        return parse_color(&color);
    }
    if let Some(kw) = first_token_value(node, "KEYWORD") {
        return match kw.as_str() {
            "true" => Ok(MosaicValue::Bool(true)),
            "false" => Ok(MosaicValue::Bool(false)),
            other => Ok(MosaicValue::Ident(other.to_string())),
        };
    }
    Err(AnalyzeError("default_value has no recognizable content".into()))
}

// ===========================================================================
// Node tree analysis
// ===========================================================================

fn analyze_node_tree(node: &GrammarASTNode) -> Result<MosaicNode, AnalyzeError> {
    // node_tree = node_element
    let elem = find_child(node, "node_element")
        .ok_or_else(|| AnalyzeError("node_tree missing node_element".into()))?;
    analyze_node_element(elem)
}

fn analyze_node_element(node: &GrammarASTNode) -> Result<MosaicNode, AnalyzeError> {
    // node_element = NAME LBRACE { node_content } RBRACE
    let node_type = first_token_value(node, "NAME")
        .ok_or_else(|| AnalyzeError("node_element missing tag name".into()))?;
    let is_primitive = is_primitive_node(&node_type);

    let mut properties = Vec::new();
    let mut children = Vec::new();

    for child in &node.children {
        if let ASTNodeOrToken::Node(n) = child {
            if n.rule_name == "node_content" {
                let (prop, child_item) = analyze_node_content(n)?;
                if let Some(p) = prop {
                    properties.push(p);
                }
                if let Some(c) = child_item {
                    children.push(c);
                }
            }
        }
    }

    Ok(MosaicNode {
        node_type,
        is_primitive,
        properties,
        children,
    })
}

fn analyze_node_content(
    node: &GrammarASTNode,
) -> Result<(Option<MosaicProperty>, Option<MosaicChild>), AnalyzeError> {
    // node_content = property_assignment | child_node | slot_reference | when_block | each_block
    for child in &node.children {
        if let ASTNodeOrToken::Node(n) = child {
            match n.rule_name.as_str() {
                "property_assignment" => {
                    return Ok((Some(analyze_property_assignment(n)?), None));
                }
                "child_node" => {
                    if let Some(elem) = find_child(n, "node_element") {
                        let child_node = analyze_node_element(elem)?;
                        return Ok((None, Some(MosaicChild::Node(child_node))));
                    }
                }
                "slot_reference" => {
                    if let Some(name) = first_token_value(n, "NAME") {
                        return Ok((None, Some(MosaicChild::SlotRef(name))));
                    }
                }
                "when_block" => {
                    return Ok((None, Some(analyze_when_block(n)?)));
                }
                "each_block" => {
                    return Ok((None, Some(analyze_each_block(n)?)));
                }
                _ => {}
            }
        }
    }
    Ok((None, None))
}

// ===========================================================================
// Property analysis
// ===========================================================================

fn analyze_property_assignment(node: &GrammarASTNode) -> Result<MosaicProperty, AnalyzeError> {
    // property_assignment = (NAME | KEYWORD) COLON property_value SEMICOLON
    // Property names may be NAME or KEYWORD tokens.
    let name = first_token_value(node, "NAME")
        .or_else(|| first_token_value(node, "KEYWORD"))
        .ok_or_else(|| AnalyzeError("property_assignment missing name".into()))?;

    let value_node = find_child(node, "property_value")
        .ok_or_else(|| AnalyzeError(format!("property '{name}' missing value")))?;
    let value = analyze_property_value(value_node)?;

    Ok(MosaicProperty { name, value })
}

fn analyze_property_value(node: &GrammarASTNode) -> Result<MosaicValue, AnalyzeError> {
    // property_value = slot_ref | STRING | NUMBER | DIMENSION | COLOR_HEX | KEYWORD | enum_value | NAME
    // Check child rule nodes first.
    for child in &node.children {
        if let ASTNodeOrToken::Node(n) = child {
            match n.rule_name.as_str() {
                "slot_ref" => {
                    if let Some(name) = first_token_value(n, "NAME") {
                        return Ok(MosaicValue::SlotRef(name));
                    }
                }
                "enum_value" => {
                    let names = direct_token_values(n, "NAME");
                    if names.len() >= 2 {
                        return Ok(MosaicValue::Enum(names[0].clone(), names[1].clone()));
                    }
                }
                _ => {}
            }
        }
    }

    // Leaf tokens.
    if let Some(s) = first_token_value(node, "STRING") {
        return Ok(MosaicValue::Literal(s));
    }
    if let Some(dim) = first_token_value(node, "DIMENSION") {
        return parse_dimension(&dim);
    }
    if let Some(num) = first_token_value(node, "NUMBER") {
        let v: f64 = num.parse().map_err(|_| AnalyzeError(format!("Invalid number: {num}")))?;
        return Ok(MosaicValue::Number(v, None));
    }
    if let Some(color) = first_token_value(node, "COLOR_HEX") {
        return parse_color(&color);
    }
    if let Some(kw) = first_token_value(node, "KEYWORD") {
        return match kw.as_str() {
            "true" => Ok(MosaicValue::Bool(true)),
            "false" => Ok(MosaicValue::Bool(false)),
            other => Ok(MosaicValue::Ident(other.to_string())),
        };
    }
    if let Some(ident) = first_token_value(node, "NAME") {
        return Ok(MosaicValue::Ident(ident));
    }

    Err(AnalyzeError("property_value has no recognizable content".into()))
}

// ===========================================================================
// When / each block analysis
// ===========================================================================

fn analyze_when_block(node: &GrammarASTNode) -> Result<MosaicChild, AnalyzeError> {
    // when_block = KEYWORD slot_ref LBRACE { node_content } RBRACE
    let slot_ref_node = find_child(node, "slot_ref")
        .ok_or_else(|| AnalyzeError("when_block missing slot_ref".into()))?;
    let slot = first_token_value(slot_ref_node, "NAME")
        .ok_or_else(|| AnalyzeError("when_block slot_ref missing name".into()))?;

    let body = collect_node_contents(node)?;
    Ok(MosaicChild::When { slot, body })
}

fn analyze_each_block(node: &GrammarASTNode) -> Result<MosaicChild, AnalyzeError> {
    // each_block = KEYWORD slot_ref KEYWORD NAME LBRACE { node_content } RBRACE
    let slot_ref_node = find_child(node, "slot_ref")
        .ok_or_else(|| AnalyzeError("each_block missing slot_ref".into()))?;
    let slot = first_token_value(slot_ref_node, "NAME")
        .ok_or_else(|| AnalyzeError("each_block slot_ref missing name".into()))?;

    // The loop variable is the NAME token that is a DIRECT child of each_block,
    // not inside the slot_ref sub-tree. We scan for a NAME after an "as" keyword.
    let item_name = find_loop_variable(node)?
        .ok_or_else(|| AnalyzeError("each_block missing loop variable".into()))?;

    let body = collect_node_contents(node)?;
    Ok(MosaicChild::Each {
        slot,
        item_name,
        body,
    })
}

/// Find the loop variable in an each_block.
///
/// The structure is:
///   KEYWORD(each)  slot_ref  KEYWORD(as)  NAME(item)  LBRACE  …  RBRACE
///
/// We look for the NAME token that immediately follows the "as" keyword.
fn find_loop_variable(each_block: &GrammarASTNode) -> Result<Option<String>, AnalyzeError> {
    let mut after_as = false;
    // Scan direct children (not recursing into sub-rules).
    for child in &each_block.children {
        match child {
            ASTNodeOrToken::Node(n) => {
                // Skip the slot_ref and node_content subtrees.
                if n.rule_name == "slot_ref" || n.rule_name == "node_content" {
                    continue;
                }
            }
            ASTNodeOrToken::Token(tok) => {
                if tok.value == "as" {
                    after_as = true;
                    continue;
                }
                if after_as && tok.effective_type_name() == "NAME" {
                    return Ok(Some(tok.value.clone()));
                }
            }
        }
    }
    Ok(None)
}

/// Collect all node_content children from a block.
fn collect_node_contents(node: &GrammarASTNode) -> Result<Vec<MosaicChild>, AnalyzeError> {
    let mut children = Vec::new();
    for child in &node.children {
        if let ASTNodeOrToken::Node(n) = child {
            if n.rule_name == "node_content" {
                let (_, child_item) = analyze_node_content(n)?;
                if let Some(c) = child_item {
                    children.push(c);
                }
            }
        }
    }
    Ok(children)
}

// ===========================================================================
// Value parsing helpers
// ===========================================================================

/// Parse a DIMENSION token like `"16dp"` into `MosaicValue::Number(16.0, Some("dp"))`.
fn parse_dimension(raw: &str) -> Result<MosaicValue, AnalyzeError> {
    // Split at the first alphabetic or '%' character.
    let split_pos = raw
        .char_indices()
        .find(|(_, c)| c.is_alphabetic() || *c == '%')
        .map(|(i, _)| i);

    if let Some(pos) = split_pos {
        let num_part = &raw[..pos];
        let unit_part = &raw[pos..];
        let v: f64 = num_part
            .parse()
            .map_err(|_| AnalyzeError(format!("Invalid DIMENSION number part: '{num_part}'")))?;
        Ok(MosaicValue::Number(v, Some(unit_part.to_string())))
    } else {
        Err(AnalyzeError(format!("Cannot parse DIMENSION: '{raw}'")))
    }
}

/// Parse a COLOR_HEX token like `"#2563eb"` into `MosaicValue::Color(r, g, b, a)`.
///
/// Expansion rules:
///   - `#rgb`     → each digit doubled, alpha = 255.
///   - `#rrggbb`  → alpha = 255.
///   - `#rrggbbaa`→ all four channels explicit.
fn parse_color(hex: &str) -> Result<MosaicValue, AnalyzeError> {
    let h = if hex.starts_with('#') { &hex[1..] } else { hex };
    match h.len() {
        3 => {
            let r = u8::from_str_radix(&h[0..1].repeat(2), 16)
                .map_err(|_| AnalyzeError(format!("Bad color: {hex}")))?;
            let g = u8::from_str_radix(&h[1..2].repeat(2), 16)
                .map_err(|_| AnalyzeError(format!("Bad color: {hex}")))?;
            let b = u8::from_str_radix(&h[2..3].repeat(2), 16)
                .map_err(|_| AnalyzeError(format!("Bad color: {hex}")))?;
            Ok(MosaicValue::Color(r, g, b, 255))
        }
        6 => {
            let r = u8::from_str_radix(&h[0..2], 16)
                .map_err(|_| AnalyzeError(format!("Bad color: {hex}")))?;
            let g = u8::from_str_radix(&h[2..4], 16)
                .map_err(|_| AnalyzeError(format!("Bad color: {hex}")))?;
            let b = u8::from_str_radix(&h[4..6], 16)
                .map_err(|_| AnalyzeError(format!("Bad color: {hex}")))?;
            Ok(MosaicValue::Color(r, g, b, 255))
        }
        8 => {
            let r = u8::from_str_radix(&h[0..2], 16)
                .map_err(|_| AnalyzeError(format!("Bad color: {hex}")))?;
            let g = u8::from_str_radix(&h[2..4], 16)
                .map_err(|_| AnalyzeError(format!("Bad color: {hex}")))?;
            let b = u8::from_str_radix(&h[4..6], 16)
                .map_err(|_| AnalyzeError(format!("Bad color: {hex}")))?;
            let a = u8::from_str_radix(&h[6..8], 16)
                .map_err(|_| AnalyzeError(format!("Bad color: {hex}")))?;
            Ok(MosaicValue::Color(r, g, b, a))
        }
        _ => Err(AnalyzeError(format!("Invalid color hex length: {hex}"))),
    }
}

// ===========================================================================
// AST traversal helpers
// ===========================================================================

/// Find the first direct child node with the given rule name.
fn find_child<'a>(node: &'a GrammarASTNode, rule: &str) -> Option<&'a GrammarASTNode> {
    for child in &node.children {
        if let ASTNodeOrToken::Node(n) = child {
            if n.rule_name == rule {
                return Some(n);
            }
        }
    }
    None
}

/// Collect all direct-child token values with the given token type name.
///
/// Uses `effective_type_name()` which returns the canonical uppercase grammar name
/// (e.g. `"NAME"`, `"KEYWORD"`, `"STRING"`, `"NUMBER"`, `"DIMENSION"`, etc.).
fn direct_token_values(node: &GrammarASTNode, token_type: &str) -> Vec<String> {
    let mut result = Vec::new();
    for child in &node.children {
        if let ASTNodeOrToken::Token(tok) = child {
            if tok.effective_type_name() == token_type {
                result.push(tok.value.clone());
            }
        }
    }
    result
}

/// Get the first direct-child token value with the given token type name.
fn first_token_value(node: &GrammarASTNode, token_type: &str) -> Option<String> {
    for child in &node.children {
        if let ASTNodeOrToken::Token(tok) = child {
            if tok.effective_type_name() == token_type {
                return Some(tok.value.clone());
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

    // -----------------------------------------------------------------------
    // Test 1: Minimal component
    // -----------------------------------------------------------------------

    #[test]
    fn test_analyze_minimal_component() {
        let src = r#"component Empty { Box { } }"#;
        let file = analyze(src).expect("analyze failed");
        assert_eq!(file.component.name, "Empty");
        assert!(file.component.slots.is_empty());
        assert_eq!(file.component.root.node_type, "Box");
        assert!(file.component.root.is_primitive);
    }

    // -----------------------------------------------------------------------
    // Test 2: Single slot with text type (slot name must not be a keyword)
    // -----------------------------------------------------------------------

    #[test]
    fn test_analyze_text_slot() {
        // Slot names must not be reserved keywords — use "title" not "text".
        let src = r#"component Label { slot title: text; Text { content: @title; } }"#;
        let file = analyze(src).expect("analyze failed");
        assert_eq!(file.component.slots.len(), 1);
        assert_eq!(file.component.slots[0].name, "title");
        assert_eq!(
            file.component.slots[0].slot_type,
            MosaicType::Primitive("text".into())
        );
        assert!(file.component.slots[0].required);
    }

    // -----------------------------------------------------------------------
    // Test 3: Slot with default value (bool = true)
    // -----------------------------------------------------------------------

    #[test]
    fn test_analyze_slot_with_default() {
        let src = r#"component Toggle { slot visible: bool = true; Box { } }"#;
        let file = analyze(src).expect("analyze failed");
        let slot = &file.component.slots[0];
        assert_eq!(slot.name, "visible");
        assert!(!slot.required);
        assert_eq!(slot.default_value, Some(MosaicValue::Bool(true)));
    }

    // -----------------------------------------------------------------------
    // Test 4: List type slot
    // -----------------------------------------------------------------------
    // NOTE: list<T> syntax fails to parse in the Rust GrammarParser because
    // the packrat memo resolves `list` as KEYWORD before trying list_type.
    // This is a known difference from the TypeScript parser.

    #[test]
    #[ignore = "Rust GrammarParser resolves 'list' as KEYWORD before list_type"]
    fn test_analyze_list_slot() {
        let src = r#"component ItemList { slot entries: list<text>; Column { } }"#;
        let file = analyze(src).expect("analyze failed");
        let slot = &file.component.slots[0];
        assert_eq!(slot.name, "entries");
        assert_eq!(
            slot.slot_type,
            MosaicType::List(Box::new(MosaicType::Primitive("text".into())))
        );
    }

    // -----------------------------------------------------------------------
    // Test 5: Color property
    // -----------------------------------------------------------------------

    #[test]
    fn test_analyze_color_property() {
        let src = r#"component Colored { Box { background: #ff0000; } }"#;
        let file = analyze(src).expect("analyze failed");
        let props = &file.component.root.properties;
        assert_eq!(props.len(), 1);
        assert_eq!(props[0].name, "background");
        assert_eq!(props[0].value, MosaicValue::Color(0xff, 0x00, 0x00, 0xff));
    }

    // -----------------------------------------------------------------------
    // Test 6: Dimension property
    // -----------------------------------------------------------------------

    #[test]
    fn test_analyze_dimension_property() {
        let src = r#"component Padded { Box { padding: 16dp; } }"#;
        let file = analyze(src).expect("analyze failed");
        let props = &file.component.root.properties;
        assert_eq!(props.len(), 1);
        assert_eq!(
            props[0].value,
            MosaicValue::Number(16.0, Some("dp".into()))
        );
    }

    // -----------------------------------------------------------------------
    // Test 7: Slot reference as property value
    // -----------------------------------------------------------------------

    #[test]
    fn test_analyze_slot_ref_property() {
        let src = r#"component Label { slot title: text; Text { content: @title; } }"#;
        let file = analyze(src).expect("analyze failed");
        let props = &file.component.root.properties;
        assert_eq!(props.len(), 1);
        assert_eq!(props[0].value, MosaicValue::SlotRef("title".into()));
    }

    // -----------------------------------------------------------------------
    // Test 8: Nested child nodes
    // -----------------------------------------------------------------------

    #[test]
    fn test_analyze_nested_nodes() {
        let src = r#"component Layout { Column { Row { Text { content: "hi"; } } } }"#;
        let file = analyze(src).expect("analyze failed");
        let root = &file.component.root;
        assert_eq!(root.node_type, "Column");
        assert_eq!(root.children.len(), 1);

        if let MosaicChild::Node(row) = &root.children[0] {
            assert_eq!(row.node_type, "Row");
            assert_eq!(row.children.len(), 1);
            if let MosaicChild::Node(text) = &row.children[0] {
                assert_eq!(text.node_type, "Text");
            } else {
                panic!("Expected Text node child");
            }
        } else {
            panic!("Expected Row node child");
        }
    }

    // -----------------------------------------------------------------------
    // Test 9: when block
    // -----------------------------------------------------------------------

    #[test]
    fn test_analyze_when_block() {
        let src = r#"
          component Conditional {
            slot show: bool;
            Column {
              when @show {
                Text { content: "Visible"; }
              }
            }
          }
        "#;
        let file = analyze(src).expect("analyze failed");
        let root = &file.component.root;
        assert_eq!(root.children.len(), 1);

        if let MosaicChild::When { slot, body } = &root.children[0] {
            assert_eq!(slot, "show");
            assert_eq!(body.len(), 1);
        } else {
            panic!("Expected When child");
        }
    }

    // -----------------------------------------------------------------------
    // Test 10: each block
    // -----------------------------------------------------------------------
    // NOTE: Uses list<node> which fails to parse in the Rust GrammarParser.
    // Marked ignore pending grammar/parser fix.

    #[test]
    #[ignore = "Rust GrammarParser resolves 'list' as KEYWORD before list_type"]
    fn test_analyze_each_block() {
        let src = r#"
          component ItemList {
            slot items: list<text>;
            Column {
              each @items as item {
                Text { content: @item; }
              }
            }
          }
        "#;
        let file = analyze(src).expect("analyze failed");
        let root = &file.component.root;
        assert_eq!(root.children.len(), 1);

        if let MosaicChild::Each {
            slot,
            item_name,
            body,
        } = &root.children[0]
        {
            assert_eq!(slot, "items");
            assert_eq!(item_name, "item");
            assert_eq!(body.len(), 1);
        } else {
            panic!("Expected Each child");
        }
    }

    // -----------------------------------------------------------------------
    // Test 11: Import declaration
    // -----------------------------------------------------------------------

    #[test]
    fn test_analyze_import() {
        let src = r#"
          import Button from "./button.mosaic";
          component Card { Box { } }
        "#;
        let file = analyze(src).expect("analyze failed");
        assert_eq!(file.imports.len(), 1);
        assert_eq!(file.imports[0].name, "Button");
        assert_eq!(file.imports[0].alias, None);
        assert!(file.imports[0].path.contains("button.mosaic"));
    }

    // -----------------------------------------------------------------------
    // Test 12: Import with alias
    // -----------------------------------------------------------------------

    #[test]
    fn test_analyze_import_with_alias() {
        let src = r#"
          import Card as InfoCard from "./cards.mosaic";
          component Page { Box { } }
        "#;
        let file = analyze(src).expect("analyze failed");
        assert_eq!(file.imports.len(), 1);
        assert_eq!(file.imports[0].name, "Card");
        assert_eq!(file.imports[0].alias, Some("InfoCard".into()));
    }

    // -----------------------------------------------------------------------
    // Test 13: Three-digit color shorthand
    // -----------------------------------------------------------------------

    #[test]
    fn test_analyze_color_shorthand() {
        let src = r#"component X { Box { background: #fff; } }"#;
        let file = analyze(src).expect("analyze failed");
        assert_eq!(
            file.component.root.properties[0].value,
            MosaicValue::Color(0xff, 0xff, 0xff, 0xff)
        );
    }

    // -----------------------------------------------------------------------
    // Test 14: Non-primitive node type
    // -----------------------------------------------------------------------

    #[test]
    fn test_analyze_non_primitive_node() {
        let src = r#"
          import Button from "./button.mosaic";
          component Page { Button { } }
        "#;
        let file = analyze(src).expect("analyze failed");
        let root = &file.component.root;
        assert_eq!(root.node_type, "Button");
        assert!(!root.is_primitive, "Button should not be primitive");
    }

    // -----------------------------------------------------------------------
    // Test 15: Slot reference as child
    // -----------------------------------------------------------------------

    #[test]
    fn test_analyze_slot_ref_child() {
        let src = r#"
          component Container {
            slot header: node;
            Column {
              @header;
              Box { }
            }
          }
        "#;
        let file = analyze(src).expect("analyze failed");
        let root = &file.component.root;
        // First child should be the slot reference @header.
        assert!(
            matches!(&root.children[0], MosaicChild::SlotRef(n) if n == "header"),
            "Expected SlotRef(header), got {:?}",
            &root.children[0]
        );
    }
}
