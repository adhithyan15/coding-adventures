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
//! | `For`   | yes       | `each: <expr>` (slot ref or expression), `as: <NAME>`, `index: <NAME>?` (UI29 §3.1, §3.3) |
//! | `If`    | yes       | `when: <expr>` (slot ref or expression) (UI29 §3.2, §3.3) |
//! | `Else`  | yes       | no props; must follow an `If` sibling (UI29 §3.2) |
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
use std::collections::HashSet;

mod _grammar;

// ===========================================================================
// The valid primitive node names — validated semantically, not in the grammar.
// ===========================================================================

/// The set of built-in layout primitives.
///
/// Everything else at a node position is either a component reference (upper-
/// case first letter) or a compile error (unknown identifier).
///
/// UI29 §2.1 froze the kernel at 15 primitives. `For` is one of the two
/// meta-primitives in that set (the other is `If`, landing in U29-G2).
///
/// **U29-X1 milestone (PR landing this comment block):** the legacy
/// `Grid` built-in primitive has been removed. It survived through
/// UI31-L10 as a backwards-compat shim for VisiCalc's pre-UI28-1
/// Grid.{desktop,touch}.mll, and through UI28-1 v0.2.0 as a "just
/// in case some demo we forgot still uses it" safety net. Both of
/// those reasons are gone now:
///
///   - mosaic-pkg-grid v0.2.0 (#4408) ships the userland Grid
///     composition that proves Cell-and-Column userland composition
///     end-to-end.
///   - VisiCalc's Grid.{desktop,touch}.mll (#4411) was rewired to
///     use the v0.2.0 composition shape directly (inlined until the
///     cross-package resolver lands).
///   - A grep across the entire repo confirms ZERO `.mll` files use
///     `Grid` as a tag — the primitive is dead in the source layer.
///
/// Per-emitter `"Grid" => ...` special-case dispatch is now dead
/// code; a follow-up PR will sweep it. This PR removes the
/// registration so any future use of `Grid` resolves as a userland
/// component reference (matching the userland v0.2.0 package).
#[allow(dead_code)] // retained as API surface / scaffolding
const PRIMITIVES: &[&str] = &[
    "Box", "Row", "Column", "Text", "Image", "Spacer",
    // Extended set from earlier specs (kept for completeness):
    "Scroll", "Divider", "Stack", "Icon",
    // U29-G1 — control-flow meta-primitive. See `validate_for_node` for the
    // prop contract (`each:`, `as:`, optional `index:`).
    "For",
    // U29-G2 — conditional meta-primitives. `If` carries the `when:` prop;
    // an `Else` sibling (no props) immediately following an `If` declares
    // the negative branch. Pairing into a single IR node is a follow-up
    // (currently both nodes coexist as siblings in `children`).
    "If",
    "Else",
    // UI29 §2.1 / UI29-1 / UI29-2 / UI29-4 — host primitives. Each
    // lowers to the host platform's native widget (DOM <input>,
    // SwiftUI TextField, Qt TextInput, etc.). HostDialog (UI29-1) is
    // the 16th kernel primitive, added after mosaic-pkg-dialog v0.1.0
    // exposed the need for a native dialog primitive with modal/focus/
    // top-layer/accessibility semantics that composition cannot
    // provide.
    // HostCheckbox and HostRadio (UI29-2) are the 17th and 18th,
    // added after mosaic-pkg-toolkit's Checkbox/Radio were found to
    // be fake HostButton wrappers — losing native a11y role,
    // checked-state visuals (tri-state, focus ring), and keyboard
    // semantics that only the platform's real checkbox/radio widget
    // provides.
    // HostLink, HostTooltip, and HostNumberInput (UI29-4) are the
    // 19th, 20th, and 21st. HostLink closes the only remaining fake-
    // X pattern in the toolkit (Breadcrumb + Nav fake `<a>` via
    // HostButton, losing role="link", Ctrl-click new-tab, visited-
    // state). HostTooltip wraps a single child with a platform-native
    // hover/long-press tooltip with proper aria-describedby wiring.
    // HostNumberInput exposes numeric-only entry with mobile numeric
    // keyboard, ± stepper buttons (Qt SpinBox / WinUI NumberBox), and
    // min/max validation. See code/specs/UI29-4-form-and-nav-
    // candidates-survey.md for the full inclusion-criteria audit.
    "HostInput", "HostButton", "HostTable", "HostScroll", "HostDialog",
    // Typed native composition boundary for host-supplied `node` slots.
    "HostSurface",
    "HostCheckbox", "HostRadio",
    "HostLink", "HostTooltip", "HostNumberInput",
    // UI31 — `HostTable` sibling primitives. Recognised by the React
    // emitter's HostTable dispatcher pre-UI31; promoting to PRIMITIVES
    // here so the parser stops routing them through the unknown-
    // component-reference fallback path and so future backends'
    // emitters can match on them directly. See
    // `code/specs/UI31-host-table.md` §2 for the structural shape.
    "HostTableColGroup", "HostTableHead", "HostTableBody", "HostTableFoot",
    "Col",
    // UI35 — the drag-and-drop family. Before this the kernel had **no**
    // drag primitive of any kind, so "drag a card to another column" —
    // the defining gesture of board software — could not be expressed in
    // a `.mll` at all. Composition cannot supply it: each platform has
    // its own native drag system (HTML pointer events, SwiftUI
    // `.draggable`/`.dropDestination`, Compose dragAndDropSource/Target,
    // QDrag, Flutter Draggable/DragTarget, WinUI CanDrag/Drop), and the
    // keyboard-equivalent path, screen-reader announcements, and touch
    // support that make dragging usable are per-platform concerns an
    // author cannot re-derive. Two primitives because a drag has two
    // ends — a card is typically both. See
    // `code/specs/UI35-host-drag-drop.md`.
    "HostDraggable", "HostDropTarget",
];

#[allow(dead_code)] // retained as API surface / scaffolding
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
    /// Element type name.
    ///
    /// Two forms (UI34):
    ///
    /// * **Unqualified** — a single PascalCase component or kernel
    ///   primitive name: `"Column"`, `"Grid"`, `"HostTable"`.  This
    ///   is the legacy form used by every pre-UI34 `.mll` file.
    /// * **Qualified** — the canonical UI34 cross-package reference:
    ///   `"pkg::mosaic-pkg-grid::Grid"`.  The string is the
    ///   author-written source syntax verbatim, so the AST round-
    ///   trips to the same `.mll` text.
    ///
    /// Callers that care about the structure should prefer the
    /// helper methods [`LayoutNode::package_ref`] and
    /// [`LayoutNode::component`] over splitting the string by hand.
    ///
    /// **Resolver invariant.**  After the `mosaic-compile`
    /// package-resolver has run (UI34 §5), no `tag` may start with
    /// `pkg::` — every qualified reference must have been
    /// substituted with the resolved sub-tree.  Each backend emitter
    /// asserts this with a `debug_assert!` in its entry point.
    pub tag: String,
    /// Optional part name for mosstyle targeting, e.g. `root`, `cell-grid`.
    pub part_name: Option<String>,
    /// Structural properties (direction, align, slot bindings, etc.).
    pub props: Vec<LayoutProp>,
    /// Child nodes (containers only; leaf nodes like `Grid` have no children).
    pub children: Vec<LayoutNode>,
}

impl LayoutNode {
    /// UI34 — return `Some((package_name, component_name))` when
    /// `tag` is a qualified `pkg::P::C` reference; `None`
    /// otherwise.
    ///
    /// ```
    /// # use moslayout_compiler::LayoutNode;
    /// let q = LayoutNode {
    ///     tag: "pkg::mosaic-pkg-grid::Grid".to_string(),
    ///     part_name: None, props: vec![], children: vec![],
    /// };
    /// assert_eq!(q.package_ref(), Some(("mosaic-pkg-grid", "Grid")));
    ///
    /// let p = LayoutNode {
    ///     tag: "HostTable".to_string(),
    ///     part_name: None, props: vec![], children: vec![],
    /// };
    /// assert_eq!(p.package_ref(), None);
    /// ```
    pub fn package_ref(&self) -> Option<(&str, &str)> {
        let rest = self.tag.strip_prefix("pkg::")?;
        let (pkg, comp) = rest.split_once("::")?;
        // Defensive: a well-formed qualified tag has exactly
        // `pkg::P::C`.  If the parser ever emits a malformed shape
        // (extra `::`) we return `None` rather than guessing — the
        // validator will surface a clean error.
        if pkg.is_empty() || comp.is_empty() || comp.contains("::") {
            return None;
        }
        Some((pkg, comp))
    }

    /// The component name portion of `tag` — strips the
    /// `pkg::P::` prefix when present, otherwise returns `tag`
    /// unchanged.
    ///
    /// Use this in emitter switch statements where the package is
    /// irrelevant (e.g. counting node depth) but care must be
    /// taken to compare the bare component name.
    ///
    /// ```
    /// # use moslayout_compiler::LayoutNode;
    /// let q = LayoutNode {
    ///     tag: "pkg::mosaic-pkg-grid::Grid".to_string(),
    ///     part_name: None, props: vec![], children: vec![],
    /// };
    /// assert_eq!(q.component(), "Grid");
    ///
    /// let p = LayoutNode {
    ///     tag: "HostTable".to_string(),
    ///     part_name: None, props: vec![], children: vec![],
    /// };
    /// assert_eq!(p.component(), "HostTable");
    /// ```
    pub fn component(&self) -> &str {
        match self.package_ref() {
            Some((_, comp)) => comp,
            None => &self.tag,
        }
    }
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
    /// A double-quoted string literal: `"Enter formula"`, `"system-ui"`.
    ///
    /// Stored *unquoted and unescaped*: the lexer emits the source text
    /// including the surrounding `"` characters, and `extract_prop_value`
    /// strips them and resolves `\n`, `\t`, `\\`, `\"`, etc. so downstream
    /// emitters can treat the value as the literal text the author meant.
    String(String),
    /// An expression in the moslayout `expr` non-terminal (UI29 §3.3).
    ///
    /// Stored as the reconstructed source substring (tokens joined with
    /// spaces). Backends parse it themselves for now — a future PR can
    /// lower this to a typed expression AST.
    ///
    /// Only constructed when the parsed `expr` contains at least one
    /// operator (`==`, `!=`, `<`, `<=`, `>`, `>=`, `&&`, `||`, `!`),
    /// member access (`.`), index access (`[...]`), or grouping (`(...)`).
    /// The four legacy primary forms (slot ref, NAME, NUMBER, STRING)
    /// still come back as `SlotRef`/`EmitRef`/`Keyword`/`Number`/`String`
    /// so all G1/G2 tests and downstream backends keep working unchanged.
    Expr(String),
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
    /// A primitive is used with an invalid or incomplete prop set. Used for
    /// kernel meta-primitives (`For`, `If`) where prop validity is structural
    /// rather than per-slot-type. UI29 §3.1 / §3.2.
    InvalidPrimitiveUsage,
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
/// Tokenize moslayout source text into a flat `Vec<Token>`.
///
/// Returns `Err(CompileError)` rather than panicking if the lexer encounters
/// a character it cannot recognise.  Callers such as `parse_layout` propagate
/// this error upward through the `compile` pipeline.
pub fn tokenize(source: &str) -> Result<Vec<Token>, CompileError> {
    let grammar = _grammar::token_grammar();
    let mut lexer = GrammarLexer::new(source, &grammar);
    lexer.tokenize().map_err(|e| CompileError {
        kind: ErrorKind::InternalError,
        message: format!("moslayout tokenization failed: {e}"),
    })
}

// ===========================================================================
// Parser
// ===========================================================================

/// Recursion-depth cap for the moslayout [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] and
/// [`parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH`] for why the underlying
/// guard exists at all (deep recursion through `parse_rule` can overflow
/// the *native* thread stack — an uncatchable process abort — before this
/// crate's own `Result`-returning entry points ever get a chance to report
/// anything). `moslayout-compiler` is reachable via the `mosaic` CLI on
/// arbitrary `.mll` files, a real attack surface.
///
/// # Three independent recursive shapes
///
/// This grammar has three *independent* recursion paths that must all be
/// measured, since a single `MAX_RULE_DEPTH` bounds the parser's internal
/// rule-invocation counter for any of them:
///
/// - **Node-tree nesting** — `node = qualified_name [...] [LBRACE {node}
///   RBRACE]`, direct self-recursion (1 rule-frame per real nesting level).
/// - **`!` (NOT) chain** — `unary = NOT unary | postfix`, direct
///   self-recursion (1 rule-frame per real nesting level), independent of
///   the expression cycle below.
/// - **Expression re-entry cycle** — `primary -> expr -> or_expr ->
///   and_expr -> eq_expr -> rel_expr -> unary -> postfix -> primary`,
///   reached by two distinct concrete constructs with different
///   rule-frames-per-level: parenthesised nesting (8 hops/level) and
///   bracket-index nesting (7 hops/level, since `postfix`'s bracket form
///   calls `expr` directly inside its own loop, skipping a `primary`
///   frame).
///
/// Measured (binary search, uncapped parser, on the true default-stack
/// per-test worker thread — no `RUST_MIN_STACK` override and no explicit
/// `Builder::stack_size`, matching what `cargo test` and a production
/// caller both actually get — debug build, adversarial 5000-level input):
/// node-tree nesting (the *binding*, lower floor) safe through 145
/// rule-frames, crashes at 146; NOT-chain safe through 210, crashes at 220;
/// bracket-index nesting safe through 260, crashes at 270; parenthesised
/// nesting safe through 280, crashes at 290.
///
/// `MAX_RULE_DEPTH` is set to **100** — about 31% below the binding
/// 145-rule-frame floor (comparable margin to sibling crates' 25-45%
/// convention), independently confirmed not to crash a default-stack
/// thread even thousands of rule-frames past the cap for any of the
/// shapes (see this crate's tests). Measured real-nesting headroom at 100
/// (capped parser, so no crash risk): node-tree nesting parses cleanly up
/// to 97 levels (98 trips the cap), NOT-chain up to 86 levels (87 trips
/// the cap), bracket-index nesting up to 12 levels (13 trips the cap),
/// parenthesised nesting up to 10 levels (11 trips the cap) — comfortably
/// past any hand-written moslayout expression's real nesting.
const MAX_RULE_DEPTH: usize = 100;

/// Parse moslayout source text into a grammar AST.
///
/// The AST mirrors the grammar rules exactly; call `analyze` to convert it
/// to a strongly-typed `LayoutDef`.
pub fn parse_layout(source: &str) -> Result<GrammarASTNode, String> {
    let tokens = tokenize(source).map_err(|e| e.message)?;
    let grammar = _grammar::parser_grammar();
    let mut parser = GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH);
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

    // UI29 §3.4 — start with an empty loop-binding scope at the layout
    // root. Each `For` node we walk into pushes its `as:` (and optional
    // `index:`) bindings onto the stack for the duration of its
    // children walk. The stack is per-call rather than per-tree-level
    // — siblings never see each other's bindings, only ancestors'.
    let loop_bindings: HashSet<String> = HashSet::new();

    validate_node(
        &def.root,
        &known_slots,
        &known_emits,
        has_interface,
        &loop_bindings,
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

#[allow(clippy::too_many_arguments)] // threaded validation context; signature kept as-is
fn validate_node(
    node: &LayoutNode,
    known_slots: &HashSet<String>,
    known_emits: &HashSet<String>,
    has_interface: bool,
    loop_bindings: &HashSet<String>,
    parts: &mut Vec<PartEntry>,
    part_names: &mut HashSet<String>,
    errors: &mut Vec<CompileError>,
) {
    // U29-G1 / U29-G2 — kernel meta-primitive structural validation.
    //
    // `For`, `If`, `Else` carry control-flow semantics, not visual ones;
    // their prop sets are fixed and they have no part_name. Validation runs
    // first because a malformed control-flow node should report
    // `InvalidPrimitiveUsage` before the rest of the prop-walking surfaces
    // `UnknownSlot` for `each:` / `when:`-referenced slots the user already
    // mistyped.
    //
    // UI29 §3.4 — `For` validation now consults `loop_bindings` to allow
    // a NAME in `each:` to resolve to an enclosing For's binding. Without
    // the scope, `For (each: row, as: cell) { ... }` (nested-For-over-
    // outer-binding) would be rejected as "must be a slot reference or
    // expression" because the parser parses bare NAMEs as Keyword, which
    // pre-§3.4 the validator unconditionally rejected.
    match node.tag.as_str() {
        "For" => validate_for_node(node, loop_bindings, errors),
        "If" => validate_if_node(node, errors),
        "Else" => validate_else_node(node, errors),
        _ => {}
    }

    // U29-G2 — `Else` orphan check. An Else node must immediately follow an
    // If sibling. Children are walked in order below, but the orphan check
    // needs the surrounding sibling list, so we run it once per parent (see
    // the post-loop hook at the end of this function for the child walk).

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
            LayoutPropValue::EmitRef(emit_name)
                if has_interface && !known_emits.contains(emit_name) => {
                    errors.push(CompileError {
                        kind: ErrorKind::UnknownEmit,
                        message: format!(
                            "Unknown emit '{}' referenced in layout — not declared in .mil",
                            emit_name
                        ),
                    });
                }
            _ => {}
        }
    }

    // U29-G2 — sibling-context check for `Else`. We walk the children list
    // with a windowed view so each `Else` is verified to immediately follow
    // an `If`. Doing this here (not inside `validate_else_node`) is the
    // simplest way to access the previous sibling without restructuring the
    // IR. A later pass can collapse the If+Else pair into a single typed
    // node; for now they coexist as siblings.
    let mut prev_tag: Option<&str> = None;
    for child in &node.children {
        if child.tag == "Else" && prev_tag != Some("If") {
            errors.push(CompileError {
                kind: ErrorKind::InvalidPrimitiveUsage,
                message: "`Else` must immediately follow an `If` sibling".to_string(),
            });
        }
        prev_tag = Some(child.tag.as_str());
    }

    // UI29 §3.4 — extend the loop-binding scope when descending into a
    // `For`'s body. The For's `as:` and (optional) `index:` bindings are
    // visible only to descendants in this subtree; siblings of the For
    // do NOT see them (the per-call scope set is rebuilt per recursion).
    //
    // Shadowing: an inner For's `as:` with the same NAME as an outer's
    // is silently allowed per UI29 §3.4 ("Nested `For`s shadow"). The
    // shadow-warning case (For binding shadowing a slot) is also
    // accepted today — see the open question in §3.4 ("Slots win? `For`
    // wins?") — defaulting to "For wins" by construction since the
    // loop_bindings scope is checked before the slot-known set in any
    // emitter's name-resolution.
    let child_loop_bindings: HashSet<String> = if node.tag == "For" {
        let mut extended = loop_bindings.clone();
        if let Some(as_name) = node
            .props
            .iter()
            .find(|p| p.name == "as")
            .and_then(|p| match &p.value {
                LayoutPropValue::Keyword(s) => Some(s.clone()),
                _ => None,
            })
        {
            extended.insert(as_name);
        }
        if let Some(idx_name) = node
            .props
            .iter()
            .find(|p| p.name == "index")
            .and_then(|p| match &p.value {
                LayoutPropValue::Keyword(s) => Some(s.clone()),
                _ => None,
            })
        {
            extended.insert(idx_name);
        }
        extended
    } else {
        loop_bindings.clone()
    };

    // Recurse into children.
    for child in &node.children {
        validate_node(
            child,
            known_slots,
            known_emits,
            has_interface,
            &child_loop_bindings,
            parts,
            part_names,
            errors,
        );
    }
}

// ===========================================================================
// U29-G1 — `For` meta-primitive validation
// ===========================================================================
//
// Per UI29 §3.1 a `For` node carries exactly three structural props:
//
//   For (each: slot: <name>, as: <name>, index: <name>?) { ...children... }
//
// • `each:` — required. Must be a SlotRef pointing at a list-typed slot.
//   (Type-of-list checking belongs in a later pass that has access to the
//   .mil descriptor's slot types; here we only enforce the slot-ref shape.)
// • `as:`   — required. Must be a NAME-keyword (parsed as `Keyword(...)`).
//   This is the per-element binding visible to children.
// • `index:` — optional. Same shape as `as:`.
//
// A `For` node may carry children — that is the whole point — but it should
// not carry a `part_name` because it produces no visual element of its own
// (control-flow primitives are styled through their children). We surface
// that as an `InvalidPrimitiveUsage` rather than silently accepting and
// dropping the part_name.
//
// What this function deliberately does NOT validate:
//
// • The `as:` / `index:` bindings are usable inside the children's expression
//   contexts. That requires `expr` (U29-G3) and a real lexical-scope walker;
//   it cannot be checked at the `LayoutNode` level today.
// • The `each:` slot's element type. Needs interface-aware analysis.
fn validate_for_node(
    node: &LayoutNode,
    loop_bindings: &HashSet<String>,
    errors: &mut Vec<CompileError>,
) {
    // Part_name on a control-flow primitive is a category error.
    if let Some(part) = &node.part_name {
        errors.push(CompileError {
            kind: ErrorKind::InvalidPrimitiveUsage,
            message: format!(
                "`For` is a control-flow primitive and cannot declare a part name (got '{}'); \
                 style its children instead",
                part
            ),
        });
    }

    let mut saw_each = false;
    let mut saw_as = false;
    let mut saw_index = false;

    for prop in &node.props {
        match prop.name.as_str() {
            "each" => {
                saw_each = true;
                // Pre-G3 this was SlotRef-only. U29-G3 lifted the restriction
                // so `each:` can be either a slot reference (`each: slot: rows`)
                // or any expression that evaluates to a list at runtime
                // (`each: cols.visible`, `each: row.cells`). U29 §3.4 further
                // lifts the restriction to accept a bare NAME (parsed as
                // Keyword) when the NAME shadows an enclosing `For`'s `as:`
                // or `index:` binding — the nested-For-over-outer-binding
                // case that mosaic-pkg-grid's Grid.mll needs:
                //
                //   For (each: slot: rows, as: row) {       ← outer
                //     For (each: row, as: cell) { … }       ← inner: each: row
                //   }                                          resolves to
                //                                              outer's `as:`
                //
                // Type-of-list checking still belongs in a later pass with
                // .mil-aware analysis; here we only enforce that the value
                // shape is one of {SlotRef, Expr, Keyword-in-loop-scope}.
                let accepted = match &prop.value {
                    LayoutPropValue::SlotRef(_) | LayoutPropValue::Expr(_) => true,
                    LayoutPropValue::Keyword(name) => loop_bindings.contains(name),
                    _ => false,
                };
                if !accepted {
                    // The error message tailors to the bad shape: a Keyword
                    // that's NOT in scope means the author wrote a bare
                    // NAME that's neither a loop binding nor a slot — the
                    // most useful hint is "did you mean `slot: NAME`?".
                    let msg = match &prop.value {
                        LayoutPropValue::Keyword(name) => format!(
                            "`For` prop `each:` references bare name `{}`, but it isn't \
                             an enclosing `For`'s `as:`/`index:` binding. Did you mean \
                             `each: slot: {}`? (UI29 §3.4)",
                            name, name
                        ),
                        _ => "`For` prop `each:` must be a slot reference, an enclosing \
                              `For` binding, or an expression (e.g. `each: slot: rows`, \
                              `each: row` where `row` is bound by an outer `For`, or \
                              `each: cols.visible`)"
                            .to_string(),
                    };
                    errors.push(CompileError {
                        kind: ErrorKind::InvalidPrimitiveUsage,
                        message: msg,
                    });
                }
            }
            "as" => {
                saw_as = true;
                if !matches!(prop.value, LayoutPropValue::Keyword(_)) {
                    errors.push(CompileError {
                        kind: ErrorKind::InvalidPrimitiveUsage,
                        message: "`For` prop `as:` must be a NAME (e.g. `as: row`)".to_string(),
                    });
                }
            }
            "index" => {
                saw_index = true;
                if !matches!(prop.value, LayoutPropValue::Keyword(_)) {
                    errors.push(CompileError {
                        kind: ErrorKind::InvalidPrimitiveUsage,
                        message: "`For` prop `index:` must be a NAME (e.g. `index: r`)".to_string(),
                    });
                }
            }
            other => {
                errors.push(CompileError {
                    kind: ErrorKind::InvalidPrimitiveUsage,
                    message: format!(
                        "`For` does not accept prop `{}`; allowed props are `each:`, `as:`, \
                         and optional `index:`",
                        other
                    ),
                });
            }
        }
    }

    if !saw_each {
        errors.push(CompileError {
            kind: ErrorKind::InvalidPrimitiveUsage,
            message: "`For` is missing required prop `each:` (a slot reference to iterate over)"
                .to_string(),
        });
    }
    if !saw_as {
        errors.push(CompileError {
            kind: ErrorKind::InvalidPrimitiveUsage,
            message: "`For` is missing required prop `as:` (the per-element binding name)"
                .to_string(),
        });
    }
    // `index:` is optional — no missing check.
    let _ = saw_index;
}

// ===========================================================================
// U29-G2 — `If` / `Else` meta-primitive validation
// ===========================================================================
//
// Per UI29 §3.2:
//
//   If ( when: <expr> ) { ...then-children... }
//   Else { ...else-children... }            // optional, must follow an If
//
// Until U29-G3 lands, `when:` accepts a single SlotRef (a boolean-typed
// slot). Once `expr` arrives, `when: <expr>` will accept the full
// boolean-expression grammar; the existing tests will still pass because
// a single `slot: x` reference is a valid trivial `expr`.
//
// `Else` exists as a separate primitive (rather than nested syntax under
// If) so the grammar didn't have to grow a new production. The orphan
// check — Else must immediately follow an If — runs in the parent's
// child-walk loop because the prev-sibling relation is needed.
//
// Neither `If` nor `Else` may carry a part_name; like `For`, they have no
// visual surface of their own.
fn validate_if_node(node: &LayoutNode, errors: &mut Vec<CompileError>) {
    if let Some(part) = &node.part_name {
        errors.push(CompileError {
            kind: ErrorKind::InvalidPrimitiveUsage,
            message: format!(
                "`If` is a control-flow primitive and cannot declare a part name (got '{}'); \
                 style its children instead",
                part
            ),
        });
    }

    let mut saw_when = false;
    for prop in &node.props {
        match prop.name.as_str() {
            "when" => {
                saw_when = true;
                // U29-G3 broadened the accepted shape: a boolean slot ref
                // (`when: slot: editing`) OR any expression (`when: r == editRow`,
                // `when: editing && !readonly`, `when: !disabled`). Bare NAME
                // values (parsed as `Keyword(...)`) are still rejected — those
                // mean a name in the *property-value enum* sense (`row`,
                // `true`, `false`, `center`), not a bound-name reference.
                // Once scoping (UI29 §3.4) is implemented we can let those in
                // too, but for now require either a slot ref or an explicit
                // expression with at least one operator / `.` / `[]` / `(`.
                if !matches!(
                    prop.value,
                    LayoutPropValue::SlotRef(_) | LayoutPropValue::Expr(_)
                ) {
                    errors.push(CompileError {
                        kind: ErrorKind::InvalidPrimitiveUsage,
                        message:
                            "`If` prop `when:` must be a slot reference or expression \
                             (e.g. `when: slot: editing` or `when: r == editRow`)"
                                .to_string(),
                    });
                }
            }
            other => {
                errors.push(CompileError {
                    kind: ErrorKind::InvalidPrimitiveUsage,
                    message: format!(
                        "`If` does not accept prop `{}`; allowed props are `when:` only",
                        other
                    ),
                });
            }
        }
    }

    if !saw_when {
        errors.push(CompileError {
            kind: ErrorKind::InvalidPrimitiveUsage,
            message:
                "`If` is missing required prop `when:` (the boolean slot to branch on)"
                    .to_string(),
        });
    }
}

fn validate_else_node(node: &LayoutNode, errors: &mut Vec<CompileError>) {
    if let Some(part) = &node.part_name {
        errors.push(CompileError {
            kind: ErrorKind::InvalidPrimitiveUsage,
            message: format!(
                "`Else` is a control-flow primitive and cannot declare a part name (got '{}')",
                part
            ),
        });
    }
    if !node.props.is_empty() {
        errors.push(CompileError {
            kind: ErrorKind::InvalidPrimitiveUsage,
            message: format!(
                "`Else` does not accept any props (got {})",
                node.props.len()
            ),
        });
    }
    // Orphan check (Else must follow an If sibling) runs in the parent's
    // child-walk loop in `validate_node` because that's where the prev-
    // sibling relation is observable.
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
    // Use serde_json so that component_name, part names, and primitive names
    // are always properly escaped — preventing JSON injection if any value
    // were to contain '"' or '\' (e.g. from a future grammar extension or
    // direct API call with arbitrary input).
    //
    // PartEntry already derives Serialize, so this is zero extra boilerplate.
    let json = serde_json::json!({
        "component": component_name,
        "parts": parts,
    });
    serde_json::to_string_pretty(&json)
        .unwrap_or_else(|e| format!("{{\"error\": \"serialisation failed: {e}\"}}"))
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
/// Grammar (UI34): `node = qualified_name [ part_name ] [ LPAREN prop_list RPAREN ] [ LBRACE { node } RBRACE ]`
///
/// where `qualified_name = NAME | KEYWORD(pkg) DOUBLE_COLON NAME DOUBLE_COLON NAME`.
///
/// The children of a `node` ASTNode contain (in order):
/// - ASTNode("qualified_name")            — the type-name reference (always present)
/// - ASTNode("part_name")                 — optional
/// - Token(LPAREN), ASTNode("prop_list"), Token(RPAREN)  — optional
/// - Token(LBRACE), ASTNode("node")*, Token(RBRACE)      — optional
fn analyze_node(node_ast: &GrammarASTNode) -> Result<LayoutNode, CompileError> {
    let children = &node_ast.children;
    let mut idx = 0;

    // ── TAG ──────────────────────────────────────────────────────────────────
    // First child must be the `qualified_name` AST node (UI34).  It either
    // wraps a single NAME token (legacy form) or a five-token sequence
    // `KEYWORD(pkg) :: NAME :: NAME` (qualified form).  We encode the
    // qualified form into the tag string verbatim (`"pkg::P::C"`); see the
    // doc-comment on [`LayoutNode::tag`].
    let tag = match children.get(idx) {
        Some(ASTNodeOrToken::Node(n)) if n.rule_name == "qualified_name" => {
            idx += 1;
            extract_qualified_name(n)?
        }
        // Fallback for the (currently impossible) case where the grammar
        // hands us a bare NAME token at the head of `node`.  Treating it
        // as an unqualified tag keeps the analyzer robust to grammar
        // evolution without silently producing the wrong AST.
        Some(ASTNodeOrToken::Token(t)) if t.type_ == TokenType::Name => {
            idx += 1;
            t.value.clone()
        }
        _ => {
            return Err(CompileError {
                kind: ErrorKind::InternalError,
                message: format!(
                    "Expected qualified_name AST node at start of node, got {:?}",
                    children.first()
                ),
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
                    // RBRACE closes the child block; `idx` is not read after
                    // the loop, so no need to advance it here.
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

/// Extract a node's type-name from a `qualified_name` AST node (UI34).
///
/// Grammar:
///     qualified_name = NAME
///                    | KEYWORD(pkg) DOUBLE_COLON NAME DOUBLE_COLON NAME
///
/// Unqualified form returns the single NAME verbatim (`"Grid"`).  Qualified
/// form returns the canonical source-syntax string (`"pkg::P::C"`) — see
/// the doc-comment on [`LayoutNode::tag`] for the rationale.  Storing the
/// reference encoded into the existing `tag` field keeps the ≈ 350 in-repo
/// `LayoutNode { … }` literal constructions in emitter test code compiling
/// unchanged (UI34 §4.1).
fn extract_qualified_name(qn_ast: &GrammarASTNode) -> Result<String, CompileError> {
    // Walk the children.  We accept either:
    //   • A single NAME token → unqualified.
    //   • A KEYWORD(pkg) followed by `::`, NAME, `::`, NAME → qualified.
    // Anything else is an InternalError because the grammar should never
    // produce a `qualified_name` of a different shape.
    let children = &qn_ast.children;

    // Unqualified shape — single NAME token.
    if children.len() == 1 {
        if let Some(ASTNodeOrToken::Token(t)) = children.first() {
            if t.type_ == TokenType::Name {
                return Ok(t.value.clone());
            }
        }
    }

    // Qualified shape — `pkg :: P :: C` is exactly five tokens.
    if children.len() == 5 {
        // We don't need to inspect each token's `type_` exhaustively
        // (the grammar already constrained the shape).  Pull the raw
        // string values and re-glue them with `::`.
        let parts: Vec<&str> = children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Token(t) => Some(t.value.as_str()),
                _ => None,
            })
            .collect();
        if parts.len() == 5 && parts[1] == "::" && parts[3] == "::" {
            // The result string is `pkg::P::C` — the verbatim source
            // syntax the author wrote.  Storing the canonical form
            // means a future round-trip emitter can reconstitute the
            // original `.mll` text without consulting any side table.
            return Ok(format!("{}::{}::{}", parts[0], parts[2], parts[4]));
        }
    }

    Err(CompileError {
        kind: ErrorKind::InternalError,
        message: format!(
            "Could not extract qualified_name (got {} children)",
            children.len()
        ),
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
/// # Grammar context (UI29 §3.3, U29-G3)
///
/// `prop_value` is now an expression — `prop_value = expr` — and `expr`
/// reaches down through `or_expr → and_expr → eq_expr → rel_expr → unary
/// → postfix → primary`. A `primary` is one of:
///
/// * `KEYWORD COLON NAME` — slot/emit binding
/// * `NAME`               — keyword value (`row`, `true`, `center`)
/// * `NUMBER`             — numeric literal
/// * `STRING`             — quoted string literal
/// * `LPAREN expr RPAREN` — parenthesised sub-expression
///
/// # Strategy
///
/// We start at `prop_value` and descend through any chain of single-child
/// rule references. If we land at a `primary` whose only content is one of
/// the four legacy forms, we emit the same variant as pre-G3 — `SlotRef`,
/// `EmitRef`, `Keyword`, `Number`, or `String`. That keeps every G1/G2
/// test and every downstream backend (mosaic-emit-react, mosaic-vm, etc.)
/// working unchanged.
///
/// If anywhere along the descent we hit a node whose body is *more* than
/// a single rule reference (i.e. it actually used an operator: `||`,
/// `&&`, `==`, `!=`, comparison, `!`, postfix `.` / `[]`, or parenthesised
/// grouping), we treat the value as an opaque expression and emit
/// `Expr(text)`, where `text` is the reconstructed source substring
/// (tokens joined with spaces).
fn extract_prop_value(pv_ast: &GrammarASTNode) -> Result<LayoutPropValue, CompileError> {
    // ── Descend the chain rules (prop_value → expr → or_expr → and_expr →
    //    eq_expr → rel_expr → unary → postfix) while each level has exactly
    //    one structural child (a single nested rule node). Stop when we
    //    either reach `primary` or hit a node that carries an actual operator.
    let mut cur = pv_ast;
    loop {
        // A node is "transparent" when its only child is one nested rule node.
        // That happens for `prop_value`, `expr`, and for any of the precedence
        // wrappers whose RHS produced no operator at this level.
        if cur.children.len() == 1 {
            if let ASTNodeOrToken::Node(inner) = &cur.children[0] {
                cur = inner;
                continue;
            }
        }
        break;
    }

    // ── If we're at `primary`, try the four legacy shapes first. The
    //    primary rule's children are direct token references (no nesting),
    //    except for the parenthesised form which contains an `expr` child.
    if cur.rule_name == "primary" {
        match cur.children.as_slice() {
            // KEYWORD COLON NAME — slot: column-headers OR emit: onNavigate
            [
                ASTNodeOrToken::Token(kw),
                ASTNodeOrToken::Token(_colon),
                ASTNodeOrToken::Token(name_tok),
            ] if kw.type_ == TokenType::Keyword => {
                let ref_name = name_tok.value.clone();
                if kw.value == "slot" {
                    return Ok(LayoutPropValue::SlotRef(ref_name));
                } else if kw.value == "emit" {
                    return Ok(LayoutPropValue::EmitRef(ref_name));
                } else {
                    return Err(CompileError {
                        kind: ErrorKind::InternalError,
                        message: format!(
                            "Unknown binding keyword '{}' in primary (expected 'slot' or 'emit')",
                            kw.value
                        ),
                    });
                }
            }
            // NAME — keyword value: row, column, true, false, center, …
            [ASTNodeOrToken::Token(t)] if t.type_ == TokenType::Name => {
                return Ok(LayoutPropValue::Keyword(t.value.clone()));
            }
            // NUMBER — numeric value: 1.5, 0, 2
            [ASTNodeOrToken::Token(t)] if t.type_ == TokenType::Number => {
                let n = t.value.parse::<f64>().map_err(|_| CompileError {
                    kind: ErrorKind::InternalError,
                    message: format!("Invalid number literal '{}'", t.value),
                })?;
                return Ok(LayoutPropValue::Number(n));
            }
            // STRING — quoted literal: "Enter formula", "system-ui", …
            [ASTNodeOrToken::Token(t)] if t.type_ == TokenType::String => {
                return Ok(LayoutPropValue::String(unescape_string_literal(&t.value)?));
            }
            // LPAREN expr RPAREN — parenthesised expression. Fall through to
            // the expression-reconstruction path below; even `(x)` carries
            // meaningful grouping semantics worth preserving as `Expr`.
            _ => {}
        }
    }

    // ── Otherwise reconstruct the source substring from the tokens under
    //    this subtree. The grammar guarantees this only happens when the
    //    author actually wrote an operator, member access, index access,
    //    or grouping — i.e. when they meant an expression.
    let text = reconstruct_expr_text(pv_ast);
    Ok(LayoutPropValue::Expr(text))
}

/// Flatten an AST subtree to its underlying source-token string.
///
/// Tokens are joined with single spaces. This is lossy with respect to the
/// original formatting but preserves every token, which is what the backend
/// needs to re-parse the expression. A future PR can replace this with a
/// proper typed expression AST.
fn reconstruct_expr_text(node: &GrammarASTNode) -> String {
    let mut parts: Vec<String> = Vec::new();
    collect_tokens(node, &mut parts);
    parts.join(" ")
}

fn collect_tokens(node: &GrammarASTNode, out: &mut Vec<String>) {
    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(t) => out.push(token_source_text(t)),
            ASTNodeOrToken::Node(n) => collect_tokens(n, out),
        }
    }
}

/// A token's text as it must appear in *reconstructed source*.
///
/// For every token type but one this is just the token's value. The exception is
/// `String`: the lexer strips a string literal's surrounding quotes and resolves its
/// escapes before storing the value (see `TokenType::String`), so pushing that value
/// verbatim silently rewrites the author's expression —
///
/// ```text
/// ( status == "done" )   reconstructs as   status == done
/// ```
///
/// — turning a string comparison into a comparison against an undefined identifier.
/// Re-quoting restores the author's meaning. It also means a string's contents can no
/// longer contribute structural characters (`}`, `,`, a bare quote) to the emitted
/// source, which is what kept expression text from being able to break out of the
/// construct it was interpolated into. See `code/specs/UI36-data-driven-sizing.md` §6.
fn token_source_text(t: &Token) -> String {
    if t.type_ != TokenType::String {
        return t.value.clone();
    }
    let mut out = String::with_capacity(t.value.len() + 2);
    out.push('"');
    for c in t.value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Resolve the standard `\`-escapes (`\n`, `\t`, `\\`, `\"`, `\r`, `\0`)
/// in a STRING token's value. The lexer has already stripped the
/// surrounding double quotes for us, so this function operates on the
/// inner text only. Any other `\x` sequence is preserved literally — the
/// grammar's STRING regex already rejects unescaped newlines and
/// unmatched quotes, so this function only needs to handle the
/// well-formed cases.
fn unescape_string_literal(raw: &str) -> Result<String, CompileError> {
    // Defensive: if a future lexer change starts shipping quotes, strip them.
    let inner: &str = raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(raw);

    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n')  => out.push('\n'),
                Some('t')  => out.push('\t'),
                Some('r')  => out.push('\r'),
                Some('0')  => out.push('\0'),
                Some('\\') => out.push('\\'),
                Some('"')  => out.push('"'),
                Some(other) => {
                    // Preserve unknown escapes verbatim (e.g. `\u` follow-up
                    // could land later as a separate feature).
                    out.push('\\');
                    out.push(other);
                }
                None => {
                    return Err(CompileError {
                        kind: ErrorKind::InternalError,
                        message: format!("Trailing backslash in string literal {:?}", raw),
                    });
                }
            }
        } else {
            out.push(c);
        }
    }
    Ok(out)
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
        let tokens = tokenize(src).expect("tokenize failed");
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
        let tokens = tokenize(src).expect("tokenize failed");
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
        let tokens = tokenize(src).expect("tokenize failed");
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
        let tokens = tokenize(src).expect("tokenize failed");
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

    /// String-literal prop value: `placeholder: "Enter formula"`. The
    /// surrounding double quotes are stripped at compile time so downstream
    /// emitters see the literal text the author meant.
    #[test]
    fn test_string_literal_prop() {
        let src = r#"
          layout Bar {
            Input [ field ] ( placeholder: "Enter formula" ) { }
          }
        "#;
        let def = parse_and_analyze(src);
        let root = &def.root;
        assert_eq!(root.tag, "Input");
        assert_eq!(root.props.len(), 1);
        assert_eq!(root.props[0].name, "placeholder");
        assert_eq!(
            root.props[0].value,
            LayoutPropValue::String("Enter formula".to_string())
        );
    }

    /// Standard `\`-escapes inside a string literal are resolved at compile
    /// time: `\n`, `\t`, `\"`, `\\`. Unknown escapes are preserved verbatim.
    #[test]
    fn test_string_literal_escapes() {
        let src = r#"
          layout Bar {
            Input [ field ] ( placeholder: "line\none\ttwo\"three" ) { }
          }
        "#;
        let def = parse_and_analyze(src);
        let root = &def.root;
        assert_eq!(
            root.props[0].value,
            LayoutPropValue::String("line\none\ttwo\"three".to_string())
        );
    }

    /// Empty string is a valid literal — useful for clearing a previous
    /// placeholder when composing styles, or for sentinel values.
    #[test]
    fn test_empty_string_literal_prop() {
        let src = r#"
          layout Bar {
            Input [ field ] ( placeholder: "" ) { }
          }
        "#;
        let def = parse_and_analyze(src);
        assert_eq!(
            def.root.props[0].value,
            LayoutPropValue::String(String::new())
        );
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

    // =====================================================================
    // U29-G1 — `For` meta-primitive validation tests
    // =====================================================================
    //
    // These tests pin down the `For` prop contract in UI29 §3.1:
    //   `For (each: slot: <name>, as: <NAME>, index: <NAME>?) { ... }`
    //
    // They run through `compile(src, None)` (no interface) so the only
    // validation surface in play is the structural one in
    // `validate_for_node`. Adding the interface (and slot/type checks for
    // `each:`) is a follow-up once G3 / R2 land — see UI29 §6.

    /// Helper: assert at least one error of the given kind matches `needle`.
    fn assert_error(
        errors: &[CompileError],
        kind: ErrorKind,
        needle: &str,
    ) {
        let matched = errors
            .iter()
            .any(|e| e.kind == kind && e.message.contains(needle));
        assert!(
            matched,
            "expected error with kind {:?} containing '{}', got: {:?}",
            kind, needle, errors
        );
    }

    #[test]
    fn for_each_as_index_well_formed_compiles_clean() {
        let src = r#"
          layout L {
            Column [ root ] {
              For ( each: slot: rows, as: row, index: r ) {
                Text ( slot: row )
              }
            }
          }
        "#;
        let result = compile(src, None).expect("well-formed For should compile");
        // Root is the Column; first child is the For node.
        let for_node = &result.def.root.children[0];
        assert_eq!(for_node.tag, "For");
        assert_eq!(for_node.props.len(), 3, "For should carry three props");
        // The For body still gets parsed and lives in `children`.
        assert_eq!(for_node.children.len(), 1);
        assert_eq!(for_node.children[0].tag, "Text");
    }

    #[test]
    fn for_without_index_is_allowed() {
        let src = r#"
          layout L {
            Column [ root ] {
              For ( each: slot: rows, as: row ) {
                Text ( slot: row )
              }
            }
          }
        "#;
        let result = compile(src, None).expect("`index:` is optional");
        assert_eq!(result.def.root.children[0].props.len(), 2);
    }

    #[test]
    fn for_missing_each_errors() {
        let src = r#"
          layout L {
            Column [ root ] {
              For ( as: row ) {
                Text ( slot: row )
              }
            }
          }
        "#;
        let errors = compile(src, None).expect_err("missing each: should reject");
        assert_error(&errors, ErrorKind::InvalidPrimitiveUsage, "missing required prop `each:`");
    }

    #[test]
    fn for_missing_as_errors() {
        let src = r#"
          layout L {
            Column [ root ] {
              For ( each: slot: rows ) {
                Text ( slot: rows )
              }
            }
          }
        "#;
        let errors = compile(src, None).expect_err("missing as: should reject");
        assert_error(&errors, ErrorKind::InvalidPrimitiveUsage, "missing required prop `as:`");
    }

    #[test]
    fn for_each_must_be_slot_ref() {
        // `each: 5` is a number — wrong shape for a list-binding prop.
        let src = r#"
          layout L {
            Column [ root ] {
              For ( each: 5 , as: row ) {
                Text ( slot: rows )
              }
            }
          }
        "#;
        let errors = compile(src, None).expect_err("each: must be a slot ref");
        assert_error(
            &errors,
            ErrorKind::InvalidPrimitiveUsage,
            "must be a slot reference",
        );
    }

    #[test]
    fn for_part_name_is_rejected() {
        // Control-flow primitives have no visual surface to style.
        let src = r#"
          layout L {
            Column [ root ] {
              For [ loop ] ( each: slot: rows, as: row ) {
                Text ( slot: row )
              }
            }
          }
        "#;
        let errors =
            compile(src, None).expect_err("part_name on a For should reject");
        assert_error(
            &errors,
            ErrorKind::InvalidPrimitiveUsage,
            "cannot declare a part name",
        );
    }

    #[test]
    fn for_unknown_prop_rejected() {
        let src = r#"
          layout L {
            Column [ root ] {
              For ( each: slot: rows, as: row, limit: 10 ) {
                Text ( slot: row )
              }
            }
          }
        "#;
        let errors = compile(src, None).expect_err("limit: is not allowed");
        assert_error(
            &errors,
            ErrorKind::InvalidPrimitiveUsage,
            "does not accept prop `limit`",
        );
    }

    #[test]
    fn for_nested_compiles_clean() {
        // The visicalc 2-D loop shape: row loop wraps a column loop.
        let src = r#"
          layout L {
            Column [ root ] {
              For ( each: slot: viewport-rows, as: row, index: r ) {
                Row {
                  For ( each: slot: columns, as: col, index: c ) {
                    Text ( slot: row )
                  }
                }
              }
            }
          }
        "#;
        let result = compile(src, None).expect("nested For should compile");
        let outer = &result.def.root.children[0];
        assert_eq!(outer.tag, "For");
        let inner_row = &outer.children[0];
        assert_eq!(inner_row.tag, "Row");
        let inner_for = &inner_row.children[0];
        assert_eq!(inner_for.tag, "For");
    }

    // =====================================================================
    // U29-G2 — `If` / `Else` meta-primitive validation tests
    // =====================================================================

    #[test]
    fn if_with_when_compiles_clean() {
        let src = r#"
          layout L {
            Column [ root ] {
              If ( when: slot: editing ) {
                Text ( slot: label )
              }
            }
          }
        "#;
        let result = compile(src, None).expect("well-formed If should compile");
        let if_node = &result.def.root.children[0];
        assert_eq!(if_node.tag, "If");
        assert_eq!(if_node.props.len(), 1);
        assert_eq!(if_node.children.len(), 1);
    }

    #[test]
    fn if_with_else_sibling_compiles_clean() {
        let src = r#"
          layout L {
            Column [ root ] {
              If ( when: slot: editing ) {
                Text ( slot: edit-buf )
              }
              Else {
                Text ( slot: label )
              }
            }
          }
        "#;
        let result =
            compile(src, None).expect("If followed by Else should compile clean");
        let children = &result.def.root.children;
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].tag, "If");
        assert_eq!(children[1].tag, "Else");
        assert_eq!(children[1].children.len(), 1);
    }

    #[test]
    fn if_missing_when_errors() {
        let src = r#"
          layout L {
            Column [ root ] {
              If {
                Text ( slot: label )
              }
            }
          }
        "#;
        let errors = compile(src, None).expect_err("missing when: should reject");
        assert_error(
            &errors,
            ErrorKind::InvalidPrimitiveUsage,
            "missing required prop `when:`",
        );
    }

    #[test]
    fn if_when_bare_name_rejected() {
        // `when: true` parses as a bare NAME → `Keyword("true")`. After G3,
        // `when:` accepts SlotRef or Expr only — bare NAMEs without any
        // operator/access are still rejected. (Once UI29 §3.4 scoping is in
        // and bound-name resolution exists, this can be revisited.) This
        // test guards that "must be a slot reference or expression" still
        // fires for the no-operator NAME case.
        let src = r#"
          layout L {
            Column [ root ] {
              If ( when: true ) {
                Text ( slot: label )
              }
            }
          }
        "#;
        let errors = compile(src, None).expect_err("bare-name when: rejected");
        assert_error(
            &errors,
            ErrorKind::InvalidPrimitiveUsage,
            "must be a slot reference or expression",
        );
    }

    #[test]
    fn if_extra_prop_rejected() {
        let src = r#"
          layout L {
            Column [ root ] {
              If ( when: slot: editing, mode: row ) {
                Text ( slot: label )
              }
            }
          }
        "#;
        let errors = compile(src, None).expect_err("If only accepts when:");
        assert_error(
            &errors,
            ErrorKind::InvalidPrimitiveUsage,
            "does not accept prop `mode`",
        );
    }

    #[test]
    fn if_part_name_rejected() {
        let src = r#"
          layout L {
            Column [ root ] {
              If [ cond ] ( when: slot: editing ) {
                Text ( slot: label )
              }
            }
          }
        "#;
        let errors = compile(src, None).expect_err("If has no visual surface");
        assert_error(
            &errors,
            ErrorKind::InvalidPrimitiveUsage,
            "cannot declare a part name",
        );
    }

    #[test]
    fn orphan_else_rejected() {
        // Else without a preceding If sibling.
        let src = r#"
          layout L {
            Column [ root ] {
              Text ( slot: label )
              Else {
                Text ( slot: alt )
              }
            }
          }
        "#;
        let errors = compile(src, None).expect_err("orphan Else rejected");
        assert_error(
            &errors,
            ErrorKind::InvalidPrimitiveUsage,
            "must immediately follow an `If` sibling",
        );
    }

    #[test]
    fn else_first_child_rejected() {
        // Else as the very first child has no possible If sibling.
        let src = r#"
          layout L {
            Column [ root ] {
              Else {
                Text ( slot: alt )
              }
            }
          }
        "#;
        let errors = compile(src, None).expect_err("first-child Else rejected");
        assert_error(
            &errors,
            ErrorKind::InvalidPrimitiveUsage,
            "must immediately follow an `If` sibling",
        );
    }

    #[test]
    fn else_with_props_rejected() {
        let src = r#"
          layout L {
            Column [ root ] {
              If ( when: slot: editing ) {
                Text ( slot: label )
              }
              Else ( mode: row ) {
                Text ( slot: alt )
              }
            }
          }
        "#;
        let errors = compile(src, None).expect_err("Else takes no props");
        assert_error(
            &errors,
            ErrorKind::InvalidPrimitiveUsage,
            "Else` does not accept any props",
        );
    }

    #[test]
    fn else_part_name_rejected() {
        let src = r#"
          layout L {
            Column [ root ] {
              If ( when: slot: editing ) {
                Text ( slot: label )
              }
              Else [ neg ] {
                Text ( slot: alt )
              }
            }
          }
        "#;
        let errors = compile(src, None).expect_err("Else has no visual surface");
        assert_error(
            &errors,
            ErrorKind::InvalidPrimitiveUsage,
            "Else` is a control-flow primitive and cannot declare a part name",
        );
    }

    #[test]
    fn if_and_else_are_in_primitives() {
        assert!(PRIMITIVES.contains(&"If"));
        assert!(PRIMITIVES.contains(&"Else"));
    }

    /// UI29-1 — HostDialog joined the kernel as the 16th primitive after
    /// `mosaic-pkg-dialog` v0.1.0 demonstrated that composed dialogs lose
    /// modal/focus/top-layer/accessibility semantics. PRIMITIVES is the
    /// canonical roster every backend looks at; pin the entry so future
    /// refactors that mistakenly drop it are caught at test time.
    ///
    /// UI29-2 — `HostCheckbox` and `HostRadio` joined as primitives 17 and
    /// 18 after `mosaic-pkg-toolkit`'s Checkbox/Radio were found to be
    /// fake `HostButton` wrappers.
    ///
    /// UI29-4 — `HostLink`, `HostTooltip`, and `HostNumberInput` joined as
    /// primitives 19, 20, and 21 after the post-UI29-2 toolkit audit
    /// identified Breadcrumb and Nav as the only remaining fake-X
    /// patterns (both faking `<a>` via `HostButton`). HostTooltip and
    /// HostNumberInput were promoted in the same batch because their
    /// per-backend native-widget shape varies enough (e.g. Qt's `SpinBox`
    /// with built-in ± buttons, mobile platforms' `inputmode="numeric"`
    /// keyboard) that userland composition couldn't reach parity.
    /// The kernel now stands at 21 primitives.
    /// UI35 — `HostDraggable` and `HostDropTarget` joined the kernel as the
    /// drag-and-drop family. Before them the kernel had no drag primitive of
    /// any kind, so a board's defining gesture was inexpressible in a `.mll`.
    /// Pinned for the same reason as the rest: PRIMITIVES is the roster every
    /// backend matches against, and a refactor that silently drops an entry
    /// sends the tag down the unknown-component fallback instead of failing.
    #[test]
    fn drag_and_drop_family_in_primitives() {
        assert!(
            PRIMITIVES.contains(&"HostDraggable"),
            "UI35 added HostDraggable (the drag source)"
        );
        assert!(
            PRIMITIVES.contains(&"HostDropTarget"),
            "UI35 added HostDropTarget (the drop sink)"
        );
    }

    /// Registration only matters if a real layout can use it: a board card is a
    /// drop target wrapping a draggable, the shape every kanban lowering emits.
    /// Compiling proves the tags resolve as primitives rather than falling
    /// through to the unknown-component-reference path.
    #[test]
    fn a_draggable_card_inside_a_drop_target_compiles() {
        let src = r#"
          layout Board {
            Column [ board ] {
              HostDropTarget [ column ] {
                HostDraggable [ card ] {
                  Text ( content: "Write spec" )
                }
              }
            }
          }
        "#;
        compile(src, None).expect("UI35 drag/drop primitives must compile in a layout");
    }

    #[test]
    fn host_dialog_and_friends_in_primitives() {
        assert!(PRIMITIVES.contains(&"HostInput"));
        assert!(PRIMITIVES.contains(&"HostButton"));
        assert!(PRIMITIVES.contains(&"HostTable"));
        assert!(PRIMITIVES.contains(&"HostScroll"));
        assert!(PRIMITIVES.contains(&"HostSurface"));
        assert!(
            PRIMITIVES.contains(&"HostDialog"),
            "UI29-1 added HostDialog as the 16th kernel primitive"
        );
        assert!(
            PRIMITIVES.contains(&"HostCheckbox"),
            "UI29-2 added HostCheckbox as the 17th kernel primitive"
        );
        assert!(
            PRIMITIVES.contains(&"HostRadio"),
            "UI29-2 added HostRadio as the 18th kernel primitive"
        );
        assert!(
            PRIMITIVES.contains(&"HostLink"),
            "UI29-4 added HostLink as the 19th kernel primitive"
        );
        assert!(
            PRIMITIVES.contains(&"HostTooltip"),
            "UI29-4 added HostTooltip as the 20th kernel primitive"
        );
        assert!(
            PRIMITIVES.contains(&"HostNumberInput"),
            "UI29-4 added HostNumberInput as the 21st kernel primitive"
        );
        // UI31 — HostTable family structural sub-tags.
        assert!(
            PRIMITIVES.contains(&"HostTableColGroup"),
            "UI31 added HostTableColGroup as a HostTable structural sub-tag"
        );
        assert!(
            PRIMITIVES.contains(&"HostTableHead"),
            "UI31 added HostTableHead as a HostTable structural sub-tag"
        );
        assert!(
            PRIMITIVES.contains(&"HostTableBody"),
            "UI31 added HostTableBody as a HostTable structural sub-tag"
        );
        assert!(
            PRIMITIVES.contains(&"HostTableFoot"),
            "UI31 added HostTableFoot as a HostTable structural sub-tag"
        );
        assert!(
            PRIMITIVES.contains(&"Col"),
            "UI31 added Col as the cell-definition sub-tag inside HostTableColGroup"
        );
    }

    #[test]
    fn host_surface_with_node_slot_compiles() {
        let interface = mosmodel_compiler::compile(
            "component Browser { slot content-surface : node ; }",
        )
        .expect("compile node-slot interface");
        let source = r#"
          layout Browser {
            Column [ shell ] {
              HostSurface [ content-surface ] (
                content : slot: content-surface
              )
            }
          }
        "#;
        compile(source, Some(&interface.descriptor_json))
            .expect("HostSurface must accept a host-supplied node slot");
    }

    // =====================================================================
    // U29-G3 — `expr` non-terminal parse/compile tests
    // =====================================================================
    //
    // These pin down the new expression grammar (UI29 §3.3). The tests live
    // at the `compile()` level so the whole pipeline (lex → parse → analyze)
    // is exercised end-to-end. The shape we're checking is always the
    // resulting `LayoutPropValue`: for the four legacy primary forms it must
    // *still* be `SlotRef`/`Keyword`/`Number`/`String`; for anything that
    // uses an operator, member access, index access, or grouping it must
    // come back as `Expr(text)` with all the source tokens preserved.
    //
    // Operators excluded by UI29 §3.3 (arithmetic, ternary, function calls,
    // string concatenation) are NOT tested for parse-error here — they
    // simply never made it into the tokens file, so the lexer fails earlier
    // and that failure mode is not the contract this PR is pinning.

    /// Helper: pull the first prop value off the first child of the root.
    fn first_prop_value(src: &str) -> LayoutPropValue {
        let result = compile(src, None).expect("compile should succeed");
        let child = &result.def.root.children[0];
        child.props[0].value.clone()
    }

    /// G3-1: A comparison expression (`r == editRow`) round-trips through
    /// the parser into a `LayoutPropValue::Expr` carrying every source token.
    #[test]
    fn g3_comparison_expr_parses_as_expr() {
        let src = r#"
          layout L {
            Column [ root ] {
              If ( when: r == editRow ) {
                Text ( slot: label )
              }
            }
          }
        "#;
        match first_prop_value(src) {
            LayoutPropValue::Expr(text) => {
                assert!(text.contains("r"), "Expr text should include LHS: {text:?}");
                assert!(text.contains("=="), "Expr text should include ==: {text:?}");
                assert!(
                    text.contains("editRow"),
                    "Expr text should include RHS: {text:?}"
                );
            }
            other => panic!("expected Expr, got {other:?}"),
        }
    }

    /// G3-2: A logical-AND expression (`editing && readonly`) lands as an Expr.
    #[test]
    fn g3_logical_and_expr_parses_as_expr() {
        let src = r#"
          layout L {
            Column [ root ] {
              If ( when: editing && readonly ) {
                Text ( slot: label )
              }
            }
          }
        "#;
        match first_prop_value(src) {
            LayoutPropValue::Expr(text) => {
                assert!(text.contains("&&"), "Expr should include &&: {text:?}");
                assert!(text.contains("editing") && text.contains("readonly"));
            }
            other => panic!("expected Expr, got {other:?}"),
        }
    }

    /// G3-3: Field access (`col.editable`) lands as an Expr — the postfix
    /// `DOT NAME` form makes this non-trivial even with no top-level operator.
    #[test]
    fn g3_field_access_parses_as_expr() {
        let src = r#"
          layout L {
            Column [ root ] {
              If ( when: col.editable ) {
                Text ( slot: label )
              }
            }
          }
        "#;
        match first_prop_value(src) {
            LayoutPropValue::Expr(text) => {
                assert!(text.contains("col"), "Expr should mention base: {text:?}");
                assert!(text.contains("."), "Expr should include DOT: {text:?}");
                assert!(
                    text.contains("editable"),
                    "Expr should include field name: {text:?}"
                );
            }
            other => panic!("expected Expr, got {other:?}"),
        }
    }

    /// A string literal inside an expression must keep its quotes.
    ///
    /// The lexer strips them (and resolves escapes) before storing a STRING token's
    /// value, so reconstructing from `token.value` verbatim silently rewrote
    /// `status == "done"` into `status == done` — a comparison against an undefined
    /// identifier. Every backend interpolates this text into generated source, so the
    /// bug reached all of them.
    #[test]
    fn expr_string_literal_keeps_its_quotes() {
        let src = r#"
          layout L {
            Column [ root ] {
              If ( when: ( status == "done" ) ) {
                Text ( slot: label )
              }
            }
          }
        "#;
        match first_prop_value(src) {
            LayoutPropValue::Expr(text) => assert!(
                text.contains("\"done\""),
                "the string literal lost its quotes: {text:?}"
            ),
            other => panic!("expected Expr, got {other:?}"),
        }
    }

    /// A quote inside the string must come back escaped, so the reconstructed text
    /// can't terminate the literal early — which is also what stops a string's
    /// contents from contributing structural characters to the emitted source.
    #[test]
    fn expr_string_literal_reescapes_an_inner_quote() {
        let src = r#"
          layout L {
            Column [ root ] {
              If ( when: ( label == "he said \"hi\"" ) ) {
                Text ( slot: label )
              }
            }
          }
        "#;
        match first_prop_value(src) {
            LayoutPropValue::Expr(text) => {
                assert!(text.contains("\\\""), "inner quote not re-escaped: {text:?}");
                // Exactly two unescaped quotes: the ones that delimit the literal.
                let bare = text
                    .char_indices()
                    .filter(|(i, c)| {
                        *c == '"' && (*i == 0 || text.as_bytes()[i - 1] != b'\\')
                    })
                    .count();
                assert_eq!(bare, 2, "unbalanced delimiters in {text:?}");
            }
            other => panic!("expected Expr, got {other:?}"),
        }
    }

    /// A non-string token is untouched — the fix must not start quoting identifiers.
    #[test]
    fn expr_non_string_tokens_are_unquoted() {
        let src = r#"
          layout L {
            Column [ root ] {
              If ( when: ( count > 3 ) ) {
                Text ( slot: label )
              }
            }
          }
        "#;
        match first_prop_value(src) {
            LayoutPropValue::Expr(text) => {
                assert!(!text.contains('"'), "identifiers got quoted: {text:?}");
                assert!(text.contains("count") && text.contains('3'), "{text:?}");
            }
            other => panic!("expected Expr, got {other:?}"),
        }
    }

    /// G3-4: Index access (`row[c]`) lands as an Expr.
    #[test]
    fn g3_index_access_parses_as_expr() {
        let src = r#"
          layout L {
            Column [ root ] {
              If ( when: row[c] ) {
                Text ( slot: label )
              }
            }
          }
        "#;
        match first_prop_value(src) {
            LayoutPropValue::Expr(text) => {
                assert!(text.contains("row"), "Expr should include base: {text:?}");
                assert!(text.contains("["), "Expr should include LBRACKET: {text:?}");
                assert!(text.contains("c"), "Expr should include index: {text:?}");
                assert!(text.contains("]"), "Expr should include RBRACKET: {text:?}");
            }
            other => panic!("expected Expr, got {other:?}"),
        }
    }

    /// G3-5: A nested expression mixing comparison + logical-AND parses cleanly.
    /// This is the canonical visicalc "this cell is the editing cell" predicate.
    #[test]
    fn g3_nested_expr_parses() {
        let src = r#"
          layout L {
            Column [ root ] {
              If ( when: r == editRow && c == editCol ) {
                Text ( slot: label )
              }
            }
          }
        "#;
        match first_prop_value(src) {
            LayoutPropValue::Expr(text) => {
                assert!(text.contains("=="), "should contain ==: {text:?}");
                assert!(text.contains("&&"), "should contain &&: {text:?}");
                assert!(text.contains("editRow"));
                assert!(text.contains("editCol"));
            }
            other => panic!("expected Expr, got {other:?}"),
        }
    }

    /// G3-6: A parenthesised expression (`(a && b)`) parses as an Expr. The
    /// LPAREN ... RPAREN form is the `primary` alternative that always
    /// produces an Expr regardless of inner shape — even `(x)` is grouping
    /// and worth preserving as such.
    #[test]
    fn g3_parenthesised_expr_parses_as_expr() {
        let src = r#"
          layout L {
            Column [ root ] {
              If ( when: (a && b) ) {
                Text ( slot: label )
              }
            }
          }
        "#;
        match first_prop_value(src) {
            LayoutPropValue::Expr(text) => {
                assert!(text.contains("("), "should include LPAREN: {text:?}");
                assert!(text.contains("&&"));
                assert!(text.contains(")"), "should include RPAREN: {text:?}");
            }
            other => panic!("expected Expr, got {other:?}"),
        }
    }

    /// G3-7: Prefix NOT (`!editing`) parses as an Expr.
    #[test]
    fn g3_not_operator_parses_as_expr() {
        let src = r#"
          layout L {
            Column [ root ] {
              If ( when: !editing ) {
                Text ( slot: label )
              }
            }
          }
        "#;
        match first_prop_value(src) {
            LayoutPropValue::Expr(text) => {
                assert!(text.contains("!"), "Expr should include NOT: {text:?}");
                assert!(text.contains("editing"));
            }
            other => panic!("expected Expr, got {other:?}"),
        }
    }

    /// G3-8: `For` accepts an expression for `each:` — the canonical
    /// "iterate only the visible columns" use case from UI29 §3.1 examples.
    #[test]
    fn g3_for_each_accepts_expr() {
        let src = r#"
          layout L {
            Column [ root ] {
              For ( each: cols.visible, as: col ) {
                Text ( slot: col )
              }
            }
          }
        "#;
        let result = compile(src, None).expect("For with expr `each:` should compile");
        let for_node = &result.def.root.children[0];
        assert_eq!(for_node.tag, "For");
        assert_eq!(for_node.props[0].name, "each");
        assert!(
            matches!(for_node.props[0].value, LayoutPropValue::Expr(_)),
            "each: expression should land as Expr, got {:?}",
            for_node.props[0].value
        );
    }

    /// G3-9: `If` accepts an expression for `when:` — the comparison
    /// `r == editRow` is what U29-G3 was created to enable.
    #[test]
    fn g3_if_when_accepts_expr() {
        let src = r#"
          layout L {
            Column [ root ] {
              If ( when: r == editRow ) {
                Text ( slot: label )
              }
            }
          }
        "#;
        let result = compile(src, None).expect("If with expr `when:` should compile");
        let if_node = &result.def.root.children[0];
        assert_eq!(if_node.tag, "If");
        assert_eq!(if_node.props[0].name, "when");
        assert!(
            matches!(if_node.props[0].value, LayoutPropValue::Expr(_)),
            "when: expression should land as Expr, got {:?}",
            if_node.props[0].value
        );
    }

    /// G3-10: Regression guard for G2 — `when: slot: editing` STILL parses
    /// as a `SlotRef`, not as an `Expr`. The grammar change must not break
    /// the legacy primary form.
    #[test]
    fn g3_if_when_slot_ref_still_slot_ref() {
        let src = r#"
          layout L {
            Column [ root ] {
              If ( when: slot: editing ) {
                Text ( slot: label )
              }
            }
          }
        "#;
        let result = compile(src, None).expect("legacy SlotRef `when:` still compiles");
        let if_node = &result.def.root.children[0];
        assert_eq!(
            if_node.props[0].value,
            LayoutPropValue::SlotRef("editing".to_string()),
            "when: slot: editing must still come back as SlotRef, not Expr"
        );
    }

    /// G3-11: Regression guard for G1 — `each: slot: rows` STILL parses
    /// as a `SlotRef`, not as an `Expr`.
    #[test]
    fn g3_for_each_slot_ref_still_slot_ref() {
        let src = r#"
          layout L {
            Column [ root ] {
              For ( each: slot: rows, as: row ) {
                Text ( slot: row )
              }
            }
          }
        "#;
        let result = compile(src, None).expect("legacy SlotRef `each:` still compiles");
        let for_node = &result.def.root.children[0];
        assert_eq!(
            for_node.props[0].value,
            LayoutPropValue::SlotRef("rows".to_string()),
            "each: slot: rows must still come back as SlotRef, not Expr"
        );
    }

    /// G3-12: Regression guard — bare NAME prop values (`direction: row`)
    /// must STILL parse as `Keyword("row")` and NOT as `Expr("row")`. The
    /// chain-rule descent must collapse cleanly through the precedence
    /// levels to `primary → NAME` for the no-operator case.
    #[test]
    fn g3_bare_name_prop_still_keyword() {
        let src = r#"
          layout L {
            Box [ root ] ( direction: row ) { }
          }
        "#;
        let result = compile(src, None).expect("direction: row still compiles");
        let root = &result.def.root;
        assert_eq!(root.props[0].name, "direction");
        assert_eq!(
            root.props[0].value,
            LayoutPropValue::Keyword("row".to_string()),
            "bare NAME values must stay as Keyword, not become Expr"
        );
    }

    // =====================================================================
    // UI29 §3.4 — For-loop binding scope
    //
    // These tests pin the scope-walker behaviour:
    //   1. Nested For with `each: <outer-as-binding>` is accepted
    //   2. Bare NAME that isn't bound anywhere is rejected with a helpful
    //      hint ("did you mean `slot: NAME`?")
    //   3. Outer For's `as:` is in scope for descendants
    //   4. Inner For's `as:` shadows outer's
    //   5. Sibling For's bindings don't leak across
    //   6. `index:` bindings are also in scope (not just `as:`)
    //
    // Required by mosaic-pkg-grid v0.2.0's Grid.mll, which nests
    // `For (each: slot: viewport-rows, as: row, index: r) {
    //    Row {
    //      For (each: row, as: cell, index: c) { Cell(...) }
    //    }
    //  }`.
    // =====================================================================

    #[test]
    fn g34_nested_for_with_outer_as_binding_compiles() {
        let src = r#"
          layout L {
            Column [ root ] {
              For ( each: slot: rows, as: row ) {
                Row {
                  For ( each: row, as: cell ) {
                    Text ( content: slot: cell )
                  }
                }
              }
            }
          }
        "#;
        // Should compile cleanly — the inner `each: row` resolves to
        // the outer For's `as: row` binding per UI29 §3.4.
        compile(src, None).expect("nested For over outer binding must compile");
    }

    #[test]
    fn g34_nested_for_with_outer_index_binding_compiles() {
        let src = r#"
          layout L {
            Column [ root ] {
              For ( each: slot: rows, as: r-data, index: r-idx ) {
                For ( each: r-idx, as: x ) {
                  Text ( content: slot: x )
                }
              }
            }
          }
        "#;
        // `each: r-idx` resolves to the outer's `index:` binding.
        // Inner For's `as: x` is also a Keyword that the inner For
        // happily declares.
        compile(src, None).expect("nested For over outer index binding must compile");
    }

    #[test]
    fn g34_unbound_bare_name_in_for_each_is_rejected_with_hint() {
        let src = r#"
          layout L {
            Column [ root ] {
              For ( each: rows, as: row ) {
                Text ( content: slot: row )
              }
            }
          }
        "#;
        // `rows` is a bare NAME with no enclosing For binding — should
        // be rejected with a hint to use `slot: rows`.
        let err = compile(src, None).expect_err("bare unbound NAME in each: must be rejected");
        let msg = format!("{:?}", err);
        assert!(
            msg.contains("rows") && msg.contains("slot:"),
            "expected error message to suggest `slot:` for bare NAME `rows`, got: {}",
            msg
        );
    }

    #[test]
    fn g34_sibling_for_bindings_do_not_leak() {
        // Two sibling Fors. The second cannot see the first's `as:`.
        let src = r#"
          layout L {
            Column [ root ] {
              For ( each: slot: rows, as: row ) { Text ( content: slot: row ) }
              For ( each: row, as: other ) { Text ( content: slot: other ) }
            }
          }
        "#;
        let err = compile(src, None)
            .expect_err("sibling For cannot reference earlier sibling's binding");
        let msg = format!("{:?}", err);
        assert!(
            msg.contains("row"),
            "expected error message to mention the unbound `row`, got: {}",
            msg
        );
    }

    #[test]
    fn g34_three_deep_nested_for_resolves_through_multiple_scopes() {
        // Innermost For sees ALL enclosing bindings, not just immediate parent.
        let src = r#"
          layout L {
            Column [ root ] {
              For ( each: slot: outer, as: a ) {
                For ( each: a, as: b ) {
                  For ( each: b, as: c ) {
                    Text ( content: slot: c )
                  }
                }
              }
            }
          }
        "#;
        compile(src, None).expect("three-deep nested For must compile");
    }

    #[test]
    fn g34_inner_for_can_still_shadow_outer_as_binding() {
        // Both Fors use `as: x`. Per UI29 §3.4 ("Nested Fors shadow"),
        // this is silently allowed. The inner's `each: x` resolves to
        // whichever For most-recently bound `x` — but since x is in
        // scope at all, the validator accepts.
        let src = r#"
          layout L {
            Column [ root ] {
              For ( each: slot: rows, as: x ) {
                For ( each: x, as: x ) {
                  Text ( content: slot: x )
                }
              }
            }
          }
        "#;
        compile(src, None).expect("shadowing inner `as: x` must compile per §3.4");
    }

    #[test]
    fn g34_for_each_slot_ref_still_works_after_scope_change() {
        // Regression guard — the §3.4 changes must NOT break the
        // pre-§3.4 happy path (each: slot: rows). Without this guard,
        // a future refactor of the validator could lose the SlotRef
        // branch silently.
        let src = r#"
          layout L {
            Column [ root ] {
              For ( each: slot: rows, as: row ) {
                Text ( content: slot: row )
              }
            }
          }
        "#;
        compile(src, None).expect("plain slot ref each: still works after §3.4");
    }

    #[test]
    fn g34_for_each_expr_still_works_after_scope_change() {
        // Same regression guard for the Expr branch.
        let src = r#"
          layout L {
            Column [ root ] {
              For ( each: cols.visible, as: col ) {
                Text ( content: slot: col )
              }
            }
          }
        "#;
        compile(src, None).expect("Expr each: still works after §3.4");
    }

    #[test]
    fn for_appears_in_the_primitive_list() {
        // Pins the `For` entry in PRIMITIVES so a future refactor that
        // accidentally drops it is caught at test time. (The validator
        // currently doesn't fire `UnknownPrimitive` at all — `is_primitive`
        // is wired up by a follow-up — but PRIMITIVES is the canonical
        // place backends and tooling look for the kernel surface.)
        assert!(
            PRIMITIVES.contains(&"For"),
            "`For` must be in PRIMITIVES (UI29 §2.1 kernel meta-primitive)"
        );
        let src = r#"
          layout L {
            Column [ root ] {
              For ( each: slot: rows, as: row ) {
                Text ( slot: row )
              }
            }
          }
        "#;
        let result = compile(src, None).expect("For must compile end-to-end");
        fn contains_for(n: &LayoutNode) -> bool {
            n.tag == "For" || n.children.iter().any(contains_for)
        }
        assert!(contains_for(&result.def.root));
    }

    // ── UI34 — pkg::P::C qualified references ─────────────────────────────

    /// `pkg::package-name::Component` parses and the analyzer stores it
    /// verbatim in the child node's `tag` field.
    #[test]
    fn ui34_qualified_tag_round_trips_into_tag_field() {
        let src = r#"
            layout Demo {
                Box [ root ] {
                    pkg::mosaic-pkg-grid::Grid { }
                }
            }
        "#;
        let def = parse_and_analyze(src);
        let child = &def.root.children[0];
        assert_eq!(child.tag, "pkg::mosaic-pkg-grid::Grid");
        assert_eq!(
            child.package_ref(),
            Some(("mosaic-pkg-grid", "Grid")),
            "package_ref() must split the canonical pkg::P::C form"
        );
        assert_eq!(child.component(), "Grid");
    }

    /// Unqualified tags don't accidentally look like qualified ones.
    #[test]
    fn ui34_unqualified_tag_has_no_package_ref() {
        let src = "layout Demo { Box [ root ] { HostTable { } } }";
        let def = parse_and_analyze(src);
        let child = &def.root.children[0];
        assert_eq!(child.tag, "HostTable");
        assert_eq!(child.package_ref(), None);
        assert_eq!(child.component(), "HostTable");
    }

    /// Qualified tag at the layout root works too — Grid.desktop.mll
    /// in the VisiCalc demo will use this exact shape once UI34 lands.
    #[test]
    fn ui34_qualified_root_node_with_props() {
        let src = r#"
            layout Grid {
                pkg::mosaic-pkg-grid::Grid (
                    viewport-rows:  slot: viewport-rows ,
                    column-headers: slot: column-headers ,
                    onNavigate:     emit: onNavigate
                )
            }
        "#;
        let def = parse_and_analyze(src);
        assert_eq!(def.root.tag, "pkg::mosaic-pkg-grid::Grid");
        let pkg = def.root.package_ref().expect("must be qualified");
        assert_eq!(pkg.0, "mosaic-pkg-grid");
        assert_eq!(pkg.1, "Grid");
        // Props pass through unchanged.
        assert_eq!(def.root.props.len(), 3);
        assert_eq!(def.root.props[0].name, "viewport-rows");
    }

    /// `package_ref()` is robust to malformed tags — it returns `None`
    /// rather than panicking or guessing.
    #[test]
    fn ui34_package_ref_rejects_malformed_tags() {
        // Missing component segment.
        let n = LayoutNode {
            tag: "pkg::mosaic-pkg-grid".to_string(),
            part_name: None,
            props: vec![],
            children: vec![],
        };
        assert_eq!(n.package_ref(), None);

        // Empty package.
        let n = LayoutNode {
            tag: "pkg::::Grid".to_string(),
            part_name: None,
            props: vec![],
            children: vec![],
        };
        assert_eq!(n.package_ref(), None);

        // Three segments after `pkg::` — the `Some` arm requires exactly
        // two — extra `::` rejected to keep round-trip exact.
        let n = LayoutNode {
            tag: "pkg::a::b::c".to_string(),
            part_name: None,
            props: vec![],
            children: vec![],
        };
        assert_eq!(n.package_ref(), None);
    }

    /// A qualified tag can be a child of another qualified tag —
    /// `mosaic-pkg-grid`'s Grid composes Cell, both qualified inside
    /// the package's own layout files.
    #[test]
    fn ui34_qualified_tags_nest() {
        let src = r#"
            layout Demo {
                pkg::mosaic-pkg-grid::Grid {
                    pkg::mosaic-pkg-grid::Cell ( slot: value )
                }
            }
        "#;
        let def = parse_and_analyze(src);
        assert_eq!(def.root.tag, "pkg::mosaic-pkg-grid::Grid");
        assert_eq!(def.root.children.len(), 1);
        assert_eq!(
            def.root.children[0].tag,
            "pkg::mosaic-pkg-grid::Cell"
        );
    }
}

/// Regression tests for [`MAX_RULE_DEPTH`], one triple per independent
/// recursive shape (see that constant's doc comment).
#[cfg(test)]
mod depth_guard_tests {
    fn nested_node_source(n: usize) -> String {
        format!("layout L {{ {}{} }}", "N{".repeat(n), "}".repeat(n))
    }

    fn nested_not_source(n: usize) -> String {
        format!("layout L {{ N(f: {}x) }}", "!".repeat(n))
    }

    fn nested_paren_source(n: usize) -> String {
        format!("layout L {{ N(f: {}x{}) }}", "(".repeat(n), ")".repeat(n))
    }

    fn nested_bracket_source(n: usize) -> String {
        format!("layout L {{ N(f: x{}{}) }}", "[x".repeat(n), "]".repeat(n))
    }

    macro_rules! depth_guard_triple {
        ($mod_name:ident, $source_fn:ident, $up_to_cap:expr, $one_past_cap:expr) => {
            mod $mod_name {
                use super::$source_fn as nested_source;

                /// Deeply-nested input must produce a recoverable error, not
                /// overflow the native stack. Parses 5000 levels — far past
                /// `MAX_RULE_DEPTH` — on a worker thread with a generous
                /// 32 MiB stack, so the *guard* is what stops the
                /// recursion, not the stack running out.
                #[test]
                fn test_deeply_nested_input_returns_error_not_overflow() {
                    let handle = std::thread::Builder::new()
                        .name(
                            concat!(
                                "moslayout-depth-guard-",
                                stringify!($mod_name),
                                "-regression"
                            )
                            .to_string(),
                        )
                        .stack_size(32 * 1024 * 1024)
                        .spawn(|| {
                            let result = super::super::parse_layout(&nested_source(5000));
                            assert!(
                                result.is_err(),
                                "deeply-nested input must fail with an error, not parse or crash"
                            );
                        })
                        .expect("failed to spawn worker thread");
                    handle
                        .join()
                        .expect("depth guard must keep the worker thread from crashing");
                }

                /// Input that nests *exactly up to* `MAX_RULE_DEPTH` still
                /// parses cleanly, and one layer deeper cleanly trips the
                /// guard. These exact boundary counts were found
                /// empirically by binary-searching against increasing
                /// nesting counts at the production cap — see
                /// `MAX_RULE_DEPTH`'s doc comment.
                #[test]
                fn test_nesting_up_to_cap_still_parses() {
                    assert!(
                        super::super::parse_layout(&nested_source($up_to_cap)).is_ok(),
                        "{} levels must stay under the cap",
                        $up_to_cap
                    );
                    assert!(
                        super::super::parse_layout(&nested_source($one_past_cap)).is_err(),
                        "one nesting level past the cap's measured limit must fail"
                    );
                }

                /// A caller relying on `MAX_RULE_DEPTH` must have the guard
                /// trip *before* the native stack overflows on a
                /// default-stack thread — otherwise a production caller
                /// (e.g. the `mosaic` CLI, or `cargo test`'s own per-test
                /// thread) would still crash. Parses far-too-deep input on
                /// a worker thread with **no** `stack_size` override (the
                /// same default a thread gets in this environment,
                /// unmodified by any `RUST_MIN_STACK` override). A clean
                /// `Err` (not a `join()` failure from a crashed thread)
                /// proves `MAX_RULE_DEPTH` sits safely below the native
                /// overflow point on the default stack.
                #[test]
                fn test_opt_in_cap_trips_before_overflow_on_default_stack() {
                    let handle = std::thread::spawn(|| {
                        let result = super::super::parse_layout(&nested_source(5000));
                        assert!(result.is_err(), "deeply-nested input must error, not crash");
                    });
                    handle.join().expect(
                        "MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack",
                    );
                }
            }
        };
    }

    depth_guard_triple!(node_shape, nested_node_source, 97, 98);
    depth_guard_triple!(not_shape, nested_not_source, 86, 87);
    depth_guard_triple!(paren_shape, nested_paren_source, 10, 11);
    depth_guard_triple!(bracket_shape, nested_bracket_source, 12, 13);
}
