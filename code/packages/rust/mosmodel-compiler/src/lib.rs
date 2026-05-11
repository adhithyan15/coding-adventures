//! # mosmodel-compiler — Compiling `.mil` component interface files.
//!
//! `mosmodel` is the component interface language for the Mosaic UI stack.
//! A `.mil` file answers exactly one question: *what does the outside
//! world need to know to use this component?*
//!
//! It answers with two constructs:
//!
//! - **slots** — named, typed data values the host pushes *in* to the component
//! - **emits** — named, typed events the component fires *out* to the host
//!
//! Nothing else is permitted; the compiler rejects any other construct.
//!
//! # Pipeline
//!
//! ```text
//! .mil source
//!       │
//!       ▼  tokenize()
//! Vec<Token>       (mosmodel.tokens grammar via GrammarLexer)
//!       │
//!       ▼  parse()
//! GrammarASTNode   (mosmodel.grammar via GrammarParser)
//!       │
//!       ▼  analyze()
//! MosmodelComponent  (typed IR: slots + emits)
//!       │
//!       ▼  validate()
//! ValidationResult   (uniqueness, type resolution, default compatibility)
//!       │
//!       ▼  emit_json()
//! String             (interface descriptor JSON consumed by moslayout + mosstyle)
//! ```
//!
//! # Quick start
//!
//! ```no_run
//! use mosmodel_compiler::compile;
//!
//! let src = r#"
//!   component Button {
//!     slot label   : text ;
//!     slot disabled : bool = false ;
//!     emit onClick ;
//!     emit onLongPress ;
//!   }
//! "#;
//!
//! let result = compile(src).expect("compilation failed");
//! println!("{}", result.descriptor_json);
//! println!("{}", result.rust_binding);
//! ```

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::{Token, TokenType};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode, GrammarParser};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

mod _grammar;

// ===========================================================================
// Public output types
// ===========================================================================

/// The result of a successful `compile()` call.
#[derive(Debug, Clone)]
pub struct CompileOutput {
    /// The analyzed component IR.
    pub component: MosmodelComponent,
    /// Interface descriptor as a JSON string.  Consumed by moslayout + mosstyle.
    pub descriptor_json: String,
    /// Rust struct binding source text.
    pub rust_binding: String,
}

// ===========================================================================
// Typed IR
// ===========================================================================

/// The compiled representation of a single `.mil` file.
///
/// A mosmodel file declares exactly one component.  The compiler maps the
/// raw syntax tree into this strongly-typed struct before validation and
/// code generation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MosmodelComponent {
    /// PascalCase name, e.g. `Button`, `Grid`, `FormulaBar`.
    pub component: String,
    /// Slot declarations — typed data inputs the host sets.
    pub slots: Vec<SlotDecl>,
    /// Emit declarations — typed events the component fires.
    pub emits: Vec<EmitDecl>,
}

/// A single slot declaration.
///
/// ```text
/// slot label    : text ;
/// slot disabled : bool = false ;
/// slot headers  : list<text> ;
/// slot cell     : CellAddress ;
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlotDecl {
    /// kebab-case name, e.g. `label`, `total-rows`, `active-cell`.
    pub name: String,
    /// The resolved type.
    pub r#type: SlotType,
    /// Whether the host must supply a value.  Slots with a default are optional.
    pub required: bool,
    /// The default value, if the slot has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<SlotDefault>,
}

/// A single emit declaration.
///
/// ```text
/// emit onClick ;
/// emit onNavigate ( row : number , col : number ) ;
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmitDecl {
    /// camelCase name with `on` prefix, e.g. `onClick`, `onNavigate`.
    pub name: String,
    /// Payload parameters.  Empty vec = void emit.
    pub params: Vec<EmitParam>,
}

/// A parameter carried by an emit's payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmitParam {
    /// kebab-case parameter name, e.g. `row`, `start-col`.
    pub name: String,
    /// The parameter type.
    pub r#type: EmitPayloadType,
}

// ---------------------------------------------------------------------------
// Type system
// ---------------------------------------------------------------------------

/// All valid types for a slot declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SlotType {
    Text,
    Number,
    Bool,
    Image,
    Color,
    Node,
    /// `list<T>` — homogeneous ordered list.
    List(Box<ListInnerType>),
    /// Named component type resolved from the component library.
    Component(String),
}

/// The inner type of a `list<T>` slot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ListInnerType {
    Text,
    Number,
    Bool,
    Image,
    Color,
    Node,
    Component(String),
}

/// Valid types for an emit payload parameter.
///
/// `image` and `node` are excluded — events carry data, not rendered subtrees.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EmitPayloadType {
    Text,
    Number,
    Bool,
    Color,
    /// Named component type (data-only component types as payload).
    Component(String),
}

/// The inline default value for an optional slot.
///
/// Only `text`, `number`, and `bool` slots may have inline defaults.
/// `image`, `color`, `list`, and component types require the host to
/// supply an explicit value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "value")]
pub enum SlotDefault {
    Text(String),
    Number(f64),
    Bool(bool),
}

// ===========================================================================
// Compiler errors
// ===========================================================================

/// A structured compile error with source location.
#[derive(Debug, Clone, PartialEq)]
pub struct CompileError {
    pub kind: ErrorKind,
    pub message: String,
}

/// The seven error kinds defined by the mosmodel spec §6.
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    /// Two slots or two emits share a name.
    DuplicateName,
    /// A slot and an emit share a name.
    NameConflict,
    /// A type name does not resolve to a known scalar or component.
    UnknownType,
    /// The default value is type-incompatible with the slot type.
    InvalidDefault,
    /// A default was provided for a type that cannot have one (image, color, list, component).
    NoDefaultForType,
    /// Something other than a slot or emit appeared inside the component body.
    UnknownConstruct,
    /// A named component type was not found in the component library.
    MissingComponent,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for CompileError {}

// ===========================================================================
// Lexer
// ===========================================================================

/// Create a `GrammarLexer` configured for mosmodel source text.
///
/// Prefer [`tokenize`] for the common case; use this when you need the
/// lexer object directly for position tracking or custom error handling.
pub fn create_mosmodel_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    GrammarLexer::new(source, &grammar)
}

/// Tokenize mosmodel source text into a flat `Vec<Token>`.
///
/// The returned vector always ends with an `EOF` token.
///
/// # Panics
///
/// Panics on unexpected characters.  Well-formed `.mil` source never
/// triggers this; callers that handle arbitrary user input should use
/// `create_mosmodel_lexer` and check the result.
pub fn tokenize(source: &str) -> Vec<Token> {
    let mut lexer = create_mosmodel_lexer(source);
    lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("mosmodel tokenization failed: {e}"))
}

// ===========================================================================
// Parser helpers
// ===========================================================================

/// Collect the non-EOF token values from a flat token stream.
/// Used in tests to assert on token sequences without comparing full Token structs.
pub fn token_values(tokens: &[Token]) -> Vec<String> {
    tokens
        .iter()
        .filter(|t| t.type_ != TokenType::Eof)
        .map(|t| t.value.clone())
        .collect()
}

// ===========================================================================
// Analyzer — ASTNode → MosmodelComponent
// ===========================================================================

/// Walk the raw grammar AST and produce a typed `MosmodelComponent`.
///
/// The `GrammarParser` produces an untyped tree of rule matches and tokens.
/// This function extracts the structure the mosmodel grammar defines and maps
/// it to strongly-typed Rust values.
///
/// # Errors
///
/// Returns a `CompileError` if the AST has an unexpected shape (which would
/// indicate a bug in the grammar or parser).
pub fn analyze(ast: &GrammarASTNode) -> Result<MosmodelComponent, CompileError> {
    // The top-level rule is `file = component_def ;`
    // component_def = KEYWORD NAME LBRACE { member } RBRACE ;
    let comp_name = extract_component_name(ast)?;
    let members = extract_members(ast)?;

    let mut slots = Vec::new();
    let mut emits = Vec::new();

    for member in members {
        match member {
            MemberNode::Slot(s) => slots.push(s),
            MemberNode::Emit(e) => emits.push(e),
        }
    }

    Ok(MosmodelComponent {
        component: comp_name,
        slots,
        emits,
    })
}

// Internal representation while walking the AST
enum MemberNode {
    Slot(SlotDecl),
    Emit(EmitDecl),
}

/// Extract the component name (the NAME token after the `component` keyword).
///
/// From the debug AST: NAME tokens have `type_ = Name` with `type_name = None`.
/// Keywords have `type_ = Keyword` with `type_name = Some("KEYWORD")`.
/// The component name is the first Name token whose value starts with uppercase.
fn extract_component_name(file_node: &GrammarASTNode) -> Result<String, CompileError> {
    // Walk: file → component_def → [KEYWORD("component"), NAME("Button"), LBRACE, …]
    for child in flatten_ast(file_node) {
        if let ASTNodeOrToken::Token(t) = &child {
            // NAME tokens: type_ = Name, type_name = None
            if t.type_ == TokenType::Name {
                if let Some(ch) = t.value.chars().next() {
                    if ch.is_uppercase() {
                        return Ok(t.value.clone());
                    }
                }
            }
        }
    }
    Err(CompileError {
        kind: ErrorKind::UnknownConstruct,
        message: "Could not find component name in AST".to_string(),
    })
}

/// Return true if a token is a Name-kind token (identifier, not keyword or punctuation).
fn is_name_token(t: &Token) -> bool {
    t.type_ == TokenType::Name
}


/// Flatten the AST depth-first, yielding all tokens and nodes.
fn flatten_ast(node: &GrammarASTNode) -> Vec<ASTNodeOrToken> {
    let mut result = Vec::new();
    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(t) => result.push(ASTNodeOrToken::Token(t.clone())),
            ASTNodeOrToken::Node(n) => {
                result.push(ASTNodeOrToken::Node(n.clone()));
                result.extend(flatten_ast(n));
            }
        }
    }
    result
}

/// Find all `member` rule nodes inside the component_def.
fn extract_members(file_node: &GrammarASTNode) -> Result<Vec<MemberNode>, CompileError> {
    let mut members = Vec::new();
    extract_members_recursive(file_node, &mut members)?;
    Ok(members)
}

fn extract_members_recursive(
    node: &GrammarASTNode,
    out: &mut Vec<MemberNode>,
) -> Result<(), CompileError> {
    match node.rule_name.as_str() {
        "slot_decl" => {
            out.push(MemberNode::Slot(parse_slot_decl(node)?));
        }
        "emit_decl" => {
            out.push(MemberNode::Emit(parse_emit_decl(node)?));
        }
        _ => {
            for child in &node.children {
                if let ASTNodeOrToken::Node(n) = child {
                    extract_members_recursive(n, out)?;
                }
            }
        }
    }
    Ok(())
}

/// Parse a `slot_decl` AST node:
///   KEYWORD(slot) NAME COLON slot_type [ EQUALS slot_default ] SEMICOLON
fn parse_slot_decl(node: &GrammarASTNode) -> Result<SlotDecl, CompileError> {
    // Collect the tokens and child-nodes in order, skipping structural punctuation.
    let mut name: Option<String> = None;
    let mut slot_type: Option<SlotType> = None;
    let mut default: Option<SlotDefault> = None;
    let mut after_equals = false;

    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(t) => {
                let val = t.value.as_str();
                match val {
                    "slot" | ":" | ";" => continue,
                    "=" => {
                        after_equals = true;
                    }
                    "true" => {
                        if after_equals {
                            default = Some(SlotDefault::Bool(true));
                        }
                    }
                    "false" => {
                        if after_equals {
                            default = Some(SlotDefault::Bool(false));
                        }
                    }
                    _ => {
                        if name.is_none() && is_name_token(t) {
                            name = Some(val.to_string());
                        } else if after_equals {
                            // Default value token
                            match t.type_ {
                                TokenType::String => {
                                    // Lexer already strips surrounding quotes
                                    default = Some(SlotDefault::Text(val.to_string()));
                                }
                                TokenType::Number => {
                                    if let Ok(n) = val.parse::<f64>() {
                                        default = Some(SlotDefault::Number(n));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            ASTNodeOrToken::Node(n) => {
                if n.rule_name == "slot_type" || n.rule_name == "scalar_type" || n.rule_name == "list_type" {
                    slot_type = Some(parse_slot_type(n)?);
                } else if n.rule_name == "slot_default" {
                    default = Some(parse_slot_default(n)?);
                }
            }
        }
    }

    let name = name.ok_or_else(|| CompileError {
        kind: ErrorKind::UnknownConstruct,
        message: "slot_decl missing name".to_string(),
    })?;
    let r#type = slot_type.ok_or_else(|| CompileError {
        kind: ErrorKind::UnknownConstruct,
        message: format!("slot '{name}' missing type"),
    })?;
    let required = default.is_none();

    Ok(SlotDecl {
        name,
        r#type,
        required,
        default,
    })
}

/// Parse a `slot_type` / `scalar_type` / `list_type` AST node.
fn parse_slot_type(node: &GrammarASTNode) -> Result<SlotType, CompileError> {
    match node.rule_name.as_str() {
        "scalar_type" => parse_scalar_slot_type(node),
        "slot_type" => {
            // slot_type = scalar_type | list_type | NAME
            for child in &node.children {
                match child {
                    ASTNodeOrToken::Node(n) => return parse_slot_type(n),
                    ASTNodeOrToken::Token(t)
                        if is_name_token(t) =>
                    {
                        return Ok(SlotType::Component(t.value.clone()));
                    }
                    _ => {}
                }
            }
            Err(CompileError {
                kind: ErrorKind::UnknownType,
                message: "empty slot_type".to_string(),
            })
        }
        "list_type" => {
            // list_type = KEYWORD("list") LANGLE inner_type RANGLE
            for child in &node.children {
                if let ASTNodeOrToken::Node(n) = child {
                    if n.rule_name == "inner_type" {
                        return Ok(SlotType::List(Box::new(parse_list_inner_type(n)?)));
                    }
                }
            }
            Err(CompileError {
                kind: ErrorKind::UnknownType,
                message: "list_type missing inner_type".to_string(),
            })
        }
        _ => {
            // Fallback: look for a NAME or scalar keyword token inside this node
            for child in flatten_ast(node) {
                if let ASTNodeOrToken::Token(t) = &child {
                    if let Some(ty) = keyword_to_slot_type(&t.value) {
                        return Ok(ty);
                    } else if is_name_token(t) {
                        return Ok(SlotType::Component(t.value.clone()));
                    }
                }
            }
            Err(CompileError {
                kind: ErrorKind::UnknownType,
                message: format!("unrecognized slot type rule: {}", node.rule_name),
            })
        }
    }
}

fn parse_scalar_slot_type(node: &GrammarASTNode) -> Result<SlotType, CompileError> {
    for child in &node.children {
        if let ASTNodeOrToken::Token(t) = child {
            if let Some(ty) = keyword_to_slot_type(&t.value) {
                return Ok(ty);
            }
        }
    }
    Err(CompileError {
        kind: ErrorKind::UnknownType,
        message: "scalar_type node has no recognizable type keyword".to_string(),
    })
}

fn parse_list_inner_type(node: &GrammarASTNode) -> Result<ListInnerType, CompileError> {
    // inner_type = scalar_type | NAME
    for child in &node.children {
        match child {
            ASTNodeOrToken::Node(n) if n.rule_name == "scalar_type" => {
                return parse_scalar_list_inner(n);
            }
            ASTNodeOrToken::Token(t) if is_name_token(t) => {
                return Ok(ListInnerType::Component(t.value.clone()));
            }
            ASTNodeOrToken::Token(t) => {
                if let Some(inner) = keyword_to_list_inner(&t.value) {
                    return Ok(inner);
                }
            }
            _ => {}
        }
    }
    Err(CompileError {
        kind: ErrorKind::UnknownType,
        message: "list inner_type is empty".to_string(),
    })
}

fn parse_scalar_list_inner(node: &GrammarASTNode) -> Result<ListInnerType, CompileError> {
    for child in &node.children {
        if let ASTNodeOrToken::Token(t) = child {
            if let Some(inner) = keyword_to_list_inner(&t.value) {
                return Ok(inner);
            }
        }
    }
    Err(CompileError {
        kind: ErrorKind::UnknownType,
        message: "scalar list inner_type has no recognizable keyword".to_string(),
    })
}

fn keyword_to_slot_type(kw: &str) -> Option<SlotType> {
    match kw {
        "text" => Some(SlotType::Text),
        "number" => Some(SlotType::Number),
        "bool" => Some(SlotType::Bool),
        "image" => Some(SlotType::Image),
        "color" => Some(SlotType::Color),
        "node" => Some(SlotType::Node),
        _ => None,
    }
}

fn keyword_to_list_inner(kw: &str) -> Option<ListInnerType> {
    match kw {
        "text" => Some(ListInnerType::Text),
        "number" => Some(ListInnerType::Number),
        "bool" => Some(ListInnerType::Bool),
        "image" => Some(ListInnerType::Image),
        "color" => Some(ListInnerType::Color),
        "node" => Some(ListInnerType::Node),
        _ => None,
    }
}

/// Parse a `slot_default` AST node: STRING | NUMBER | KEYWORD(true|false)
fn parse_slot_default(node: &GrammarASTNode) -> Result<SlotDefault, CompileError> {
    for child in &node.children {
        if let ASTNodeOrToken::Token(t) = child {
            match t.type_ {
                TokenType::String => {
                    // The GrammarLexer strips the surrounding quotes from string tokens.
                    return Ok(SlotDefault::Text(t.value.clone()));
                }
                TokenType::Number => {
                    let n = t.value.parse::<f64>().map_err(|_| CompileError {
                        kind: ErrorKind::InvalidDefault,
                        message: format!("cannot parse number default '{}'", t.value),
                    })?;
                    return Ok(SlotDefault::Number(n));
                }
                _ => match t.value.as_str() {
                    "true" => return Ok(SlotDefault::Bool(true)),
                    "false" => return Ok(SlotDefault::Bool(false)),
                    _ => {}
                },
            }
        }
    }
    Err(CompileError {
        kind: ErrorKind::InvalidDefault,
        message: "slot_default node is empty".to_string(),
    })
}

/// Parse an `emit_decl` AST node:
///   KEYWORD(emit) NAME [ LPAREN emit_param_list RPAREN ] SEMICOLON
fn parse_emit_decl(node: &GrammarASTNode) -> Result<EmitDecl, CompileError> {
    let mut name: Option<String> = None;
    let mut params: Vec<EmitParam> = Vec::new();

    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(t) => {
                match t.value.as_str() {
                    "emit" | "(" | ")" | ";" => continue,
                    _ => {
                        if name.is_none() && is_name_token(t) {
                            name = Some(t.value.clone());
                        }
                    }
                }
            }
            ASTNodeOrToken::Node(n) => {
                if n.rule_name == "emit_param_list" {
                    params = parse_emit_param_list(n)?;
                }
            }
        }
    }

    let name = name.ok_or_else(|| CompileError {
        kind: ErrorKind::UnknownConstruct,
        message: "emit_decl missing name".to_string(),
    })?;

    Ok(EmitDecl { name, params })
}

/// Parse `emit_param_list = emit_param { COMMA emit_param }`.
fn parse_emit_param_list(node: &GrammarASTNode) -> Result<Vec<EmitParam>, CompileError> {
    let mut params = Vec::new();
    for child in &node.children {
        if let ASTNodeOrToken::Node(n) = child {
            if n.rule_name == "emit_param" {
                params.push(parse_emit_param(n)?);
            }
        }
    }
    Ok(params)
}

/// Parse `emit_param = NAME COLON emit_payload_type`.
fn parse_emit_param(node: &GrammarASTNode) -> Result<EmitParam, CompileError> {
    let mut name: Option<String> = None;
    let mut payload_type: Option<EmitPayloadType> = None;

    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(t) => match t.value.as_str() {
                ":" => continue,
                _ => {
                    if name.is_none() && is_name_token(t) {
                        name = Some(t.value.clone());
                    }
                }
            },
            ASTNodeOrToken::Node(n) if n.rule_name == "emit_payload_type" => {
                payload_type = Some(parse_emit_payload_type(n)?);
            }
            _ => {}
        }
    }

    let name = name.ok_or_else(|| CompileError {
        kind: ErrorKind::UnknownConstruct,
        message: "emit_param missing name".to_string(),
    })?;
    let r#type = payload_type.ok_or_else(|| CompileError {
        kind: ErrorKind::UnknownType,
        message: format!("emit param '{name}' missing type"),
    })?;

    Ok(EmitParam { name, r#type })
}

/// Parse `emit_payload_type = KEYWORD | NAME`.
fn parse_emit_payload_type(node: &GrammarASTNode) -> Result<EmitPayloadType, CompileError> {
    for child in &node.children {
        if let ASTNodeOrToken::Token(t) = child {
            match t.value.as_str() {
                "text" => return Ok(EmitPayloadType::Text),
                "number" => return Ok(EmitPayloadType::Number),
                "bool" => return Ok(EmitPayloadType::Bool),
                "color" => return Ok(EmitPayloadType::Color),
                _ if is_name_token(t) => {
                    return Ok(EmitPayloadType::Component(t.value.clone()));
                }
                _ => {}
            }
        }
    }
    Err(CompileError {
        kind: ErrorKind::UnknownType,
        message: "emit_payload_type node has no recognizable type".to_string(),
    })
}

// ===========================================================================
// Validator
// ===========================================================================

/// Validate a `MosmodelComponent` for semantic correctness.
///
/// Checks (from the spec §5):
///
/// 1. Unique slot names.
/// 2. Unique emit names.
/// 3. No slot and emit share a name.
/// 4. Default values are type-compatible.
/// 5. Types that cannot have defaults (`image`, `color`, `list`, component) don't.
/// 6. Payload types exclude `image` and `node`.
pub fn validate(component: &MosmodelComponent) -> Result<(), Vec<CompileError>> {
    let mut errors = Vec::new();

    // --- 1. Unique slot names ---
    let mut slot_names: HashSet<&str> = HashSet::new();
    for slot in &component.slots {
        if !slot_names.insert(&slot.name) {
            errors.push(CompileError {
                kind: ErrorKind::DuplicateName,
                message: format!("Duplicate slot name '{}'", slot.name),
            });
        }
    }

    // --- 2. Unique emit names ---
    let mut emit_names: HashSet<&str> = HashSet::new();
    for emit in &component.emits {
        if !emit_names.insert(&emit.name) {
            errors.push(CompileError {
                kind: ErrorKind::DuplicateName,
                message: format!("Duplicate emit name '{}'", emit.name),
            });
        }
    }

    // --- 3. No name shared between a slot and an emit ---
    for slot in &component.slots {
        if emit_names.contains(slot.name.as_str()) {
            errors.push(CompileError {
                kind: ErrorKind::NameConflict,
                message: format!(
                    "'{}' is declared as both a slot and an emit",
                    slot.name
                ),
            });
        }
    }

    // --- 4 + 5. Default value compatibility ---
    for slot in &component.slots {
        validate_slot_default(slot, &mut errors);
    }

    // --- 6. Emit payload types must not be image or node ---
    for emit in &component.emits {
        for param in &emit.params {
            match &param.r#type {
                // All current EmitPayloadType variants are valid — image/node are excluded
                // by the type system itself (they're not variants of EmitPayloadType).
                EmitPayloadType::Text
                | EmitPayloadType::Number
                | EmitPayloadType::Bool
                | EmitPayloadType::Color
                | EmitPayloadType::Component(_) => {}
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_slot_default(slot: &SlotDecl, errors: &mut Vec<CompileError>) {
    // Types that cannot have inline defaults.
    let no_default_type = matches!(
        slot.r#type,
        SlotType::Image | SlotType::Color | SlotType::List(_) | SlotType::Component(_)
    );

    if let Some(default) = &slot.default {
        if no_default_type {
            errors.push(CompileError {
                kind: ErrorKind::NoDefaultForType,
                message: format!(
                    "Slots of type {:?} cannot have inline defaults (slot '{}')",
                    slot.r#type, slot.name
                ),
            });
            return;
        }

        // Verify the default value matches the declared type.
        let compatible = match (&slot.r#type, default) {
            (SlotType::Text, SlotDefault::Text(_)) => true,
            (SlotType::Number, SlotDefault::Number(_)) => true,
            (SlotType::Bool, SlotDefault::Bool(_)) => true,
            _ => false,
        };

        if !compatible {
            errors.push(CompileError {
                kind: ErrorKind::InvalidDefault,
                message: format!(
                    "Slot '{}' has type {:?} but the default value has an incompatible type",
                    slot.name, slot.r#type
                ),
            });
        }
    }
}

// ===========================================================================
// Emitters
// ===========================================================================

// ---------------------------------------------------------------------------
// JSON descriptor
// ---------------------------------------------------------------------------

/// Serialize the component to the interface descriptor JSON string.
///
/// The descriptor is the format consumed by the `moslayout` and `mosstyle`
/// compilers and by each backend emitter.
pub fn emit_descriptor_json(component: &MosmodelComponent) -> Result<String, CompileError> {
    serde_json::to_string_pretty(component).map_err(|e| CompileError {
        kind: ErrorKind::UnknownConstruct,
        message: format!("JSON serialization failed: {e}"),
    })
}

// ---------------------------------------------------------------------------
// Rust binding
// ---------------------------------------------------------------------------

/// Generate a Rust struct binding for the Metal / paint-vm backend.
///
/// Produces a builder-pattern struct following the pattern defined in the
/// mosmodel spec §5.  Each slot becomes a public field and a builder method;
/// each emit becomes an `Option<Box<dyn Fn(…)>>` field and a builder method.
pub fn emit_rust_binding(component: &MosmodelComponent) -> String {
    let name = &component.component;
    let mut out = String::new();

    // ---- struct fields ----
    out.push_str(&format!("/// Auto-generated binding for the `{name}` mosmodel interface.\n"));
    out.push_str("#[derive(Default)]\n");
    out.push_str(&format!("pub struct {name} {{\n"));

    for slot in &component.slots {
        let field = kebab_to_snake(&slot.name);
        let ty = slot_type_to_rust(&slot.r#type);
        out.push_str(&format!("    pub {field}: {ty},\n"));
    }
    for emit in &component.emits {
        let field = camel_to_snake(&emit.name);
        let fn_ty = emit_fn_type(&emit.params);
        out.push_str(&format!("    pub {field}: Option<Box<dyn {fn_ty}>>,\n"));
    }

    out.push_str("}\n\n");

    // ---- impl block with builder methods ----
    out.push_str(&format!("impl {name} {{\n"));
    out.push_str(&format!("    pub fn new() -> Self {{ Self::default() }}\n\n"));

    for slot in &component.slots {
        let field = kebab_to_snake(&slot.name);
        let ty = slot_type_to_rust(&slot.r#type);
        out.push_str(&format!(
            "    pub fn {field}(mut self, v: {ty}) -> Self {{ self.{field} = v; self }}\n"
        ));
    }
    for emit in &component.emits {
        let field = camel_to_snake(&emit.name);
        let param_types: Vec<String> = emit
            .params
            .iter()
            .map(|p| emit_payload_type_to_rust(&p.r#type))
            .collect();
        let closure_ty = if param_types.is_empty() {
            "impl Fn() + 'static".to_string()
        } else {
            format!("impl Fn({}) + 'static", param_types.join(", "))
        };
        out.push_str(&format!(
            "    pub fn {field}(mut self, f: {closure_ty}) -> Self {{ self.{field} = Some(Box::new(f)); self }}\n"
        ));
    }

    out.push_str("}\n");
    out
}

// --- Type mapping helpers ---

fn slot_type_to_rust(ty: &SlotType) -> String {
    match ty {
        SlotType::Text => "String".to_string(),
        SlotType::Number => "f64".to_string(),
        SlotType::Bool => "bool".to_string(),
        SlotType::Image => "ImageHandle".to_string(),
        SlotType::Color => "[f32; 4]".to_string(),
        SlotType::Node => "Box<dyn AnyNode>".to_string(),
        SlotType::List(inner) => format!("Vec<{}>", list_inner_to_rust(inner)),
        SlotType::Component(n) => n.clone(),
    }
}

fn list_inner_to_rust(inner: &ListInnerType) -> &'static str {
    match inner {
        ListInnerType::Text => "String",
        ListInnerType::Number => "f64",
        ListInnerType::Bool => "bool",
        ListInnerType::Image => "ImageHandle",
        ListInnerType::Color => "[f32; 4]",
        ListInnerType::Node => "Box<dyn AnyNode>",
        ListInnerType::Component(_) => "Box<dyn AnyNode>",
    }
}

fn emit_payload_type_to_rust(ty: &EmitPayloadType) -> String {
    match ty {
        EmitPayloadType::Text => "String".to_string(),
        EmitPayloadType::Number => "f64".to_string(),
        EmitPayloadType::Bool => "bool".to_string(),
        EmitPayloadType::Color => "[f32; 4]".to_string(),
        EmitPayloadType::Component(n) => n.clone(),
    }
}

fn emit_fn_type(params: &[EmitParam]) -> String {
    if params.is_empty() {
        return "Fn()".to_string();
    }
    let args: Vec<String> = params
        .iter()
        .map(|p| emit_payload_type_to_rust(&p.r#type))
        .collect();
    format!("Fn({})", args.join(", "))
}

// --- Name-conversion helpers ---

/// `total-rows` → `total_rows`
fn kebab_to_snake(s: &str) -> String {
    s.replace('-', "_")
}

/// `onClick` → `on_click`, `onEditCommit` → `on_edit_commit`
fn camel_to_snake(s: &str) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(ch.to_lowercase().next().unwrap());
    }
    out
}

// ===========================================================================
// Top-level compile entry point
// ===========================================================================

/// Compile a `.mil` source string.
///
/// Runs the full pipeline: tokenize → parse → analyze → validate → emit.
///
/// # Errors
///
/// Returns `Err(Vec<CompileError>)` if tokenization, parsing, analysis, or
/// validation fails.  Multiple errors may be returned from the validation pass.
pub fn compile(source: &str) -> Result<CompileOutput, Vec<CompileError>> {
    // Tokenize
    let tokens = tokenize(source);

    // Parse
    let grammar = _grammar::parser_grammar();
    let mut parser = GrammarParser::new(tokens, grammar);
    let ast = parser.parse().map_err(|e| {
        vec![CompileError {
            kind: ErrorKind::UnknownConstruct,
            message: format!("Parse error: {e}"),
        }]
    })?;

    // Analyze
    let component = analyze(&ast).map_err(|e| vec![e])?;

    // Validate
    validate(&component)?;

    // Emit
    let descriptor_json = emit_descriptor_json(&component)
        .map_err(|e| vec![e])?;
    let rust_binding = emit_rust_binding(&component);

    Ok(CompileOutput {
        component,
        descriptor_json,
        rust_binding,
    })
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Lexer tests
    // -----------------------------------------------------------------------

    /// `component` must lex as a keyword.
    #[test]
    fn lex_component_keyword() {
        let toks = tokenize("component");
        assert_eq!(token_values(&toks), vec!["component"]);
    }

    /// `slot` must lex as a keyword.
    #[test]
    fn lex_slot_keyword() {
        let toks = tokenize("slot");
        assert_eq!(token_values(&toks), vec!["slot"]);
    }

    /// `emit` must lex as a keyword.
    #[test]
    fn lex_emit_keyword() {
        let toks = tokenize("emit");
        assert_eq!(token_values(&toks), vec!["emit"]);
    }

    /// All eight scalar type keywords are recognized.
    #[test]
    fn lex_scalar_type_keywords() {
        let vals = token_values(&tokenize("text number bool image color node list"));
        assert!(vals.contains(&"text".to_string()));
        assert!(vals.contains(&"number".to_string()));
        assert!(vals.contains(&"bool".to_string()));
        assert!(vals.contains(&"image".to_string()));
        assert!(vals.contains(&"color".to_string()));
        assert!(vals.contains(&"node".to_string()));
        assert!(vals.contains(&"list".to_string()));
    }

    /// PascalCase component names lex as NAME tokens.
    #[test]
    fn lex_pascal_name() {
        let vals = token_values(&tokenize("Button"));
        assert_eq!(vals, vec!["Button"]);
    }

    /// kebab-case slot names lex as a single NAME token.
    #[test]
    fn lex_kebab_name() {
        let vals = token_values(&tokenize("total-rows"));
        assert_eq!(vals, vec!["total-rows"]);
    }

    /// Hyphen-free slot names are still NAME.
    #[test]
    fn lex_simple_name() {
        let vals = token_values(&tokenize("label"));
        assert_eq!(vals, vec!["label"]);
    }

    /// `true` and `false` lex as keywords.
    #[test]
    fn lex_bool_literals() {
        let vals = token_values(&tokenize("true false"));
        assert_eq!(vals, vec!["true", "false"]);
    }

    /// A double-quoted string is a single STRING token.
    /// The GrammarLexer strips the surrounding quotes from string token values.
    #[test]
    fn lex_string_literal() {
        let vals = token_values(&tokenize("\"hello\""));
        assert_eq!(vals, vec!["hello"]);
    }

    /// Numbers parse correctly.
    #[test]
    fn lex_number_literal() {
        let vals = token_values(&tokenize("42 3.14 0"));
        assert_eq!(vals, vec!["42", "3.14", "0"]);
    }

    /// All punctuation tokens are recognized.
    #[test]
    fn lex_punctuation() {
        let vals = token_values(&tokenize("{ } ( ) < > : ; , ="));
        assert_eq!(
            vals,
            vec!["{", "}", "(", ")", "<", ">", ":", ";", ",", "="]
        );
    }

    /// Line comments are skipped.
    #[test]
    fn lex_line_comment_skipped() {
        let vals = token_values(&tokenize("// comment\ncomponent"));
        assert_eq!(vals, vec!["component"]);
    }

    /// Block comments are skipped.
    #[test]
    fn lex_block_comment_skipped() {
        let vals = token_values(&tokenize("/* block */emit"));
        assert_eq!(vals, vec!["emit"]);
    }

    /// Whitespace is skipped; compact and spaced source produce identical tokens.
    #[test]
    fn lex_whitespace_skipped() {
        let compact = token_values(&tokenize("slot x:text;"));
        let spaced = token_values(&tokenize("slot x : text ;"));
        assert_eq!(compact, spaced);
    }

    /// A complete minimal component declaration tokenizes without error.
    #[test]
    fn lex_minimal_component() {
        let src = "component Button { slot label : text ; emit onClick ; }";
        let toks = tokenize(src);
        // Must end with EOF.
        assert_eq!(toks.last().unwrap().type_, TokenType::Eof);
        // More than 5 meaningful tokens.
        assert!(toks.len() > 5);
    }

    // -----------------------------------------------------------------------
    // Compile tests — happy path
    // -----------------------------------------------------------------------

    fn button_src() -> &'static str {
        r#"
        component Button {
          slot label    : text ;
          slot disabled : bool = false ;
          emit onClick ;
          emit onLongPress ;
        }
        "#
    }

    fn grid_src() -> &'static str {
        r#"
        component Grid {
          slot column-headers  : list<text> ;
          slot total-rows      : number ;
          slot viewport-offset : number = 0 ;
          slot selected-row    : number = 0 ;
          slot selected-col    : number = 0 ;
          emit onNavigate ( row : number , col : number ) ;
          emit onEditCommit ( value : text ) ;
          emit onEditCancel ;
          emit onScroll ( offset : number ) ;
        }
        "#
    }

    /// Button component compiles without errors.
    #[test]
    fn compile_button() {
        let out = compile(button_src()).expect("Button should compile");
        assert_eq!(out.component.component, "Button");
        assert_eq!(out.component.slots.len(), 2);
        assert_eq!(out.component.emits.len(), 2);
    }

    /// Grid component compiles without errors.
    #[test]
    fn compile_grid() {
        let out = compile(grid_src()).expect("Grid should compile");
        assert_eq!(out.component.component, "Grid");
        assert_eq!(out.component.slots.len(), 5);
        assert_eq!(out.component.emits.len(), 4);
    }

    /// `label` slot has type text and is required (no default).
    #[test]
    fn slot_label_is_required_text() {
        let out = compile(button_src()).unwrap();
        let label = out.component.slots.iter().find(|s| s.name == "label").unwrap();
        assert_eq!(label.r#type, SlotType::Text);
        assert!(label.required);
        assert!(label.default.is_none());
    }

    /// `disabled` slot has type bool, is optional, and defaults to false.
    #[test]
    fn slot_disabled_is_optional_bool() {
        let out = compile(button_src()).unwrap();
        let disabled = out
            .component
            .slots
            .iter()
            .find(|s| s.name == "disabled")
            .unwrap();
        assert_eq!(disabled.r#type, SlotType::Bool);
        assert!(!disabled.required);
        assert_eq!(disabled.default, Some(SlotDefault::Bool(false)));
    }

    /// `column-headers` slot has type list<text>.
    #[test]
    fn slot_list_type() {
        let out = compile(grid_src()).unwrap();
        let headers = out
            .component
            .slots
            .iter()
            .find(|s| s.name == "column-headers")
            .unwrap();
        assert_eq!(headers.r#type, SlotType::List(Box::new(ListInnerType::Text)));
        assert!(headers.required);
    }

    /// `viewport-offset` slot defaults to 0.
    #[test]
    fn slot_number_default() {
        let out = compile(grid_src()).unwrap();
        let vp = out
            .component
            .slots
            .iter()
            .find(|s| s.name == "viewport-offset")
            .unwrap();
        assert_eq!(vp.r#type, SlotType::Number);
        assert_eq!(vp.default, Some(SlotDefault::Number(0.0)));
    }

    /// `onClick` emit has no params.
    #[test]
    fn emit_void() {
        let out = compile(button_src()).unwrap();
        let onclick = out
            .component
            .emits
            .iter()
            .find(|e| e.name == "onClick")
            .unwrap();
        assert!(onclick.params.is_empty());
    }

    /// `onNavigate` emit carries row and col number params.
    #[test]
    fn emit_with_params() {
        let out = compile(grid_src()).unwrap();
        let nav = out
            .component
            .emits
            .iter()
            .find(|e| e.name == "onNavigate")
            .unwrap();
        assert_eq!(nav.params.len(), 2);
        assert_eq!(nav.params[0].name, "row");
        assert_eq!(nav.params[0].r#type, EmitPayloadType::Number);
        assert_eq!(nav.params[1].name, "col");
        assert_eq!(nav.params[1].r#type, EmitPayloadType::Number);
    }

    /// `onEditCommit` emit carries a text param named `value`.
    #[test]
    fn emit_text_param() {
        let out = compile(grid_src()).unwrap();
        let commit = out
            .component
            .emits
            .iter()
            .find(|e| e.name == "onEditCommit")
            .unwrap();
        assert_eq!(commit.params.len(), 1);
        assert_eq!(commit.params[0].name, "value");
        assert_eq!(commit.params[0].r#type, EmitPayloadType::Text);
    }

    /// The descriptor JSON is valid JSON and contains the component name.
    #[test]
    fn descriptor_json_is_valid() {
        let out = compile(button_src()).unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(&out.descriptor_json).expect("descriptor must be valid JSON");
        assert_eq!(parsed["component"], "Button");
    }

    /// The Rust binding contains the struct name and builder methods.
    #[test]
    fn rust_binding_contains_struct() {
        let out = compile(button_src()).unwrap();
        assert!(out.rust_binding.contains("pub struct Button"));
        assert!(out.rust_binding.contains("pub fn label"));
        assert!(out.rust_binding.contains("pub fn on_click"));
    }

    // -----------------------------------------------------------------------
    // Validation tests — error cases
    // -----------------------------------------------------------------------

    /// Duplicate slot names produce a DuplicateName error.
    #[test]
    fn validate_duplicate_slot() {
        let src = r#"component Foo { slot x : text ; slot x : number ; }"#;
        let result = compile(src);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(|e| e.kind == ErrorKind::DuplicateName));
    }

    /// Duplicate emit names produce a DuplicateName error.
    #[test]
    fn validate_duplicate_emit() {
        let src = r#"component Foo { emit onClick ; emit onClick ; }"#;
        let result = compile(src);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(|e| e.kind == ErrorKind::DuplicateName));
    }

    /// A slot and emit sharing a name produce a NameConflict error.
    #[test]
    fn validate_slot_emit_name_conflict() {
        let src = r#"component Foo { slot label : text ; emit label ; }"#;
        let result = compile(src);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(|e| e.kind == ErrorKind::NameConflict));
    }

    /// A bool slot with a string default produces an InvalidDefault error.
    #[test]
    fn validate_incompatible_default() {
        let src = r#"component Foo { slot flag : bool = "yes" ; }"#;
        let result = compile(src);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs
            .iter()
            .any(|e| e.kind == ErrorKind::InvalidDefault || e.kind == ErrorKind::NoDefaultForType));
    }

    // -----------------------------------------------------------------------
    // Utility tests
    // -----------------------------------------------------------------------

    /// `camel_to_snake` converts camelCase emit names.
    #[test]
    fn camel_to_snake_conversions() {
        assert_eq!(camel_to_snake("onClick"), "on_click");
        assert_eq!(camel_to_snake("onEditCommit"), "on_edit_commit");
        assert_eq!(camel_to_snake("onNavigate"), "on_navigate");
    }

    /// `kebab_to_snake` converts kebab-case slot names.
    #[test]
    fn kebab_to_snake_conversions() {
        assert_eq!(kebab_to_snake("total-rows"), "total_rows");
        assert_eq!(kebab_to_snake("selected-col"), "selected_col");
        assert_eq!(kebab_to_snake("label"), "label");
    }

    /// An empty component (no slots, no emits) compiles successfully.
    #[test]
    fn compile_empty_component() {
        let src = r#"component Empty { }"#;
        let out = compile(src).expect("empty component should compile");
        assert_eq!(out.component.component, "Empty");
        assert!(out.component.slots.is_empty());
        assert!(out.component.emits.is_empty());
    }

    /// FormulaBar from the spec compiles correctly.
    #[test]
    fn compile_formula_bar() {
        let src = r#"
        component FormulaBar {
          slot cell-address : text ;
          slot formula      : text ;
          slot read-only    : bool = false ;
          emit onFormulaChange ( formula : text ) ;
          emit onCommit ;
          emit onCancel ;
        }
        "#;
        let out = compile(src).expect("FormulaBar should compile");
        assert_eq!(out.component.component, "FormulaBar");
        assert_eq!(out.component.slots.len(), 3);
        assert_eq!(out.component.emits.len(), 3);

        let formula_slot = out
            .component
            .slots
            .iter()
            .find(|s| s.name == "formula")
            .unwrap();
        assert_eq!(formula_slot.r#type, SlotType::Text);
        assert!(formula_slot.required);

        let change_emit = out
            .component
            .emits
            .iter()
            .find(|e| e.name == "onFormulaChange")
            .unwrap();
        assert_eq!(change_emit.params.len(), 1);
    }
}

