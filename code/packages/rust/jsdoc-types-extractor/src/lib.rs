//! JSDoc → type-sidecar extractor.
//!
//! Walks the [`GrammarASTNode`] tree produced by
//! [`coding_adventures_jsdoc_parser::parse_jsdoc`] and emits a
//! [`Sidecar`] containing one [`Record`] per anchored JS node, per
//! [CLOC05 §"jsdoc-types-extractor"](../../../specs/CLOC05-jsdoc-sub-pipeline.md).
//!
//! # Scope (v1)
//!
//! - **One Record per call**, keyed by the caller-supplied
//!   `anchor_cv`. The full per-anchor flow (driven by
//!   `jsdoc-comment-extractor`'s `BlockComment` list mapped to AST
//!   nodes) is a follow-up; v1 takes a single JSDoc body string and
//!   a single anchor.
//! - **Primitive-only type lowering.** The `type-sidecar` crate's
//!   `Type` lattice ships primitives only at v1 (CLOC04 Phase 1), so
//!   anything richer — `Foo`, `Foo[]`, `?Foo`, `function(...): T`,
//!   `{ k: T }` — collapses to [`Type::Opaque`] with the
//!   reconstructed original text in `raw`. The richer lowering lands
//!   when the lattice expands.
//! - **`@type` / `@param` / `@returns` only.** The full tag set from
//!   CLOC05's mapping table comes online once we have richer
//!   `Attributes` and `Type::Function` to lower into. v1 stashes
//!   `@param` / `@returns` payloads into
//!   `Attributes.extension["params"]` / `["returns"]` as JSON blobs
//!   to preserve information until the proper slots arrive.
//! - **Unknown tags survive.** Anything else (`@throws`, `@template`,
//!   …) is parsed as `unknown_tag` by `jsdoc-parser` and ignored here
//!   without erroring.
//!
//! # What it does for you
//!
//! ```text
//! /** @type {number} */
//! ```
//!
//! becomes a `Sidecar` with one record at `anchor_cv` whose
//! `ty = Some(Type::Number)` and provenance `producer = "jsdoc"`.
//!
//! ```text
//! /** @param {string} name */
//! /** @returns {boolean} */
//! ```
//!
//! becomes a single record whose `attributes.extension` contains a
//! `"params"` array (`[{"type":"string","name":"name"}]`) and a
//! `"returns"` object (`{"type":"boolean"}`).

use coding_adventures_jsdoc_parser::parse_jsdoc;
use coding_adventures_type_sidecar::{
    Attributes, EvidenceStep, ProducerId, Provenance, Record, Sidecar, Type,
};
use parser::grammar_parser::{find_nodes, ASTNodeOrToken, GrammarASTNode};

const PRODUCER_NAME: &str = "jsdoc";
const PRODUCER_VERSION: &str = "0.1.0";

/// Extract JSDoc types from `source` (the cleaned interior of a single
/// `/** ... */` block, as produced by
/// `coding-adventures-jsdoc-comment-extractor`) into a [`Sidecar`].
///
/// `anchor_cv` is the correlation-vector ID of the JS AST node the
/// comment annotates. v1 produces at most one [`Record`] keyed by that
/// ID; if no `@type` / `@param` / `@returns` tag is present, the
/// returned sidecar is empty.
///
/// Returns `Err` only if `jsdoc-parser` fails outright. Parse errors
/// inside individual tags (e.g. unrecognised type expressions)
/// degrade gracefully into [`Type::Opaque`].
pub fn extract_types(source: &str, anchor_cv: &str) -> Result<Sidecar, String> {
    let ast = parse_jsdoc(source)?;
    Ok(extract_from_ast(&ast, anchor_cv))
}

/// Like [`extract_types`] but takes a pre-parsed [`GrammarASTNode`].
/// Useful when the caller has already invoked `parse_jsdoc` for other
/// reasons.
pub fn extract_from_ast(ast: &GrammarASTNode, anchor_cv: &str) -> Sidecar {
    let mut sidecar = Sidecar::new();
    let mut builder = RecordBuilder::new(anchor_cv);

    // The JSDoc grammar disambiguates `type_tag` / `param_tag` /
    // `returns_tag` by **structure**, not by the `@name` token. So a
    // `@returns {boolean}` gets parsed as `type_tag` (because it has
    // the same shape) and a `@throws {Error} bad` as `param_tag`. We
    // dispatch on the actual AT_TAG value here.
    for tag_kind in &["type_tag", "param_tag", "returns_tag"] {
        for tag_node in find_nodes(ast, tag_kind) {
            let at_tag_name = first_at_tag(&tag_node).unwrap_or_default();
            match at_tag_name.as_str() {
                "@type" => {
                    if let Some(ty) = lower_type_expression(&tag_node) {
                        builder.set_type(ty);
                    }
                }
                "@param" | "@arg" | "@argument" => {
                    if let Some(entry) = lower_param_tag(&tag_node) {
                        builder.add_param(entry);
                    }
                }
                "@returns" | "@return" => {
                    if let Some(entry) = lower_returns_tag(&tag_node) {
                        builder.set_returns(entry);
                    }
                }
                _ => {
                    // Anything else slipped through grammar structurally
                    // but we don't recognise it semantically yet — drop
                    // it. The CLOC05 "unknown tags survive" contract
                    // still holds at the parse layer; this extractor
                    // just doesn't emit them in v1.
                }
            }
        }
    }

    if let Some(record) = builder.build() {
        sidecar.insert(record);
    }
    sidecar
}

/// The text of the first `AT_TAG` token directly under `node`, e.g.
/// `"@type"`, `"@param"`, `"@returns"`. Returns `None` if no AT_TAG
/// token exists at the top level (shouldn't happen for any tag node).
fn first_at_tag(node: &GrammarASTNode) -> Option<String> {
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            if t.type_name.as_deref() == Some("AT_TAG") {
                return Some(t.value.clone());
            }
        }
    }
    None
}

// ============================================================================
// RecordBuilder — coalesces tags into a single Record at anchor_cv
// ============================================================================

struct RecordBuilder {
    cv: String,
    ty: Option<Type>,
    params: Vec<serde_json::Value>,
    returns: Option<serde_json::Value>,
    had_anything: bool,
}

impl RecordBuilder {
    fn new(anchor_cv: &str) -> Self {
        Self {
            cv: anchor_cv.to_string(),
            ty: None,
            params: Vec::new(),
            returns: None,
            had_anything: false,
        }
    }

    fn set_type(&mut self, ty: Type) {
        self.ty = Some(ty);
        self.had_anything = true;
    }

    fn add_param(&mut self, entry: serde_json::Value) {
        self.params.push(entry);
        self.had_anything = true;
    }

    fn set_returns(&mut self, entry: serde_json::Value) {
        self.returns = Some(entry);
        self.had_anything = true;
    }

    fn build(self) -> Option<Record> {
        if !self.had_anything {
            return None;
        }
        let mut attributes = Attributes::default();
        if !self.params.is_empty() {
            attributes
                .extension
                .insert("params".to_string(), serde_json::Value::Array(self.params));
        }
        if let Some(r) = self.returns {
            attributes.extension.insert("returns".to_string(), r);
        }
        Some(Record {
            cv: self.cv,
            ty: self.ty,
            attributes,
            provenance: Provenance {
                producer: ProducerId::new(PRODUCER_NAME),
                producer_version: PRODUCER_VERSION.to_string(),
                source_file: None,
                source_location: Some("extractor".to_string()),
                generated_at: None,
                evidence: vec![EvidenceStep {
                    stage: "extract".to_string(),
                    note: "jsdoc-types-extractor v1".to_string(),
                    at: None,
                }],
            },
        })
    }
}

// ============================================================================
// Tag lowerings
// ============================================================================

fn lower_type_expression(type_tag_node: &GrammarASTNode) -> Option<Type> {
    // `type_tag` shape: AT_TAG type_expression NEWLINE
    // `type_expression` shape: LBRACE type RBRACE
    // We just want the textual content of the `type` node and lower it.
    let type_expr = find_first_child_node(type_tag_node, "type_expression")?;
    let inner_type = find_first_child_node(&type_expr, "type")?;
    Some(lower_type_node(&inner_type))
}

fn lower_param_tag(param_tag_node: &GrammarASTNode) -> Option<serde_json::Value> {
    // `param_tag` shape:
    //   AT_TAG type_expression name_path [ description ] NEWLINE
    let type_expr = find_first_child_node(param_tag_node, "type_expression")?;
    let inner_type = find_first_child_node(&type_expr, "type")?;
    let ty_string = type_node_text(&inner_type);

    let name_path = find_first_child_node(param_tag_node, "name_path")
        .map(|n| flatten_text(&n))
        .unwrap_or_default();

    Some(serde_json::json!({
        "type": ty_string,
        "name": name_path,
    }))
}

fn lower_returns_tag(returns_tag_node: &GrammarASTNode) -> Option<serde_json::Value> {
    // `returns_tag` shape: AT_TAG type_expression [ description ] NEWLINE
    let type_expr = find_first_child_node(returns_tag_node, "type_expression")?;
    let inner_type = find_first_child_node(&type_expr, "type")?;
    let ty_string = type_node_text(&inner_type);
    Some(serde_json::json!({ "type": ty_string }))
}

/// Map a `type` node to a [`Type`] value, preferring primitive variants
/// for the small set the type-sidecar lattice currently models, and
/// falling back to [`Type::Opaque`] with the reconstructed source text
/// for everything else.
fn lower_type_node(type_node: &GrammarASTNode) -> Type {
    let raw = type_node_text(type_node);
    primitive_from_name(&raw).unwrap_or(Type::Opaque { raw })
}

/// The text shape of a `type` subtree, reconstructed by joining its
/// token values with spaces stripped. Good enough for the primitive
/// match and for stashing into `Opaque.raw`.
fn type_node_text(type_node: &GrammarASTNode) -> String {
    flatten_text(type_node)
}

fn primitive_from_name(name: &str) -> Option<Type> {
    match name.trim() {
        "number" => Some(Type::Number),
        "string" => Some(Type::String),
        "boolean" => Some(Type::Boolean),
        "null" => Some(Type::Null),
        "undefined" => Some(Type::Undefined),
        "void" => Some(Type::Undefined), // JSDoc convention: void = undefined
        "any" => Some(Type::Any),
        "unknown" => Some(Type::Unknown),
        "never" => Some(Type::Never),
        "bigint" => Some(Type::BigInt),
        "symbol" => Some(Type::Symbol),
        _ => None,
    }
}

// ============================================================================
// AST walking helpers
// ============================================================================

fn find_first_child_node(node: &GrammarASTNode, rule_name: &str) -> Option<GrammarASTNode> {
    for c in &node.children {
        if let ASTNodeOrToken::Node(n) = c {
            if n.rule_name == rule_name {
                return Some(n.clone());
            }
        }
    }
    // Fall back to a deep search — handles cases where the rule is
    // nested through intermediate alternatives.
    find_nodes(node, rule_name).into_iter().next()
}

/// Flatten all token text in a subtree into a single string, separating
/// adjacent token values with no extra whitespace. Good enough for the
/// v1 primitive-vs-opaque distinction and Opaque.raw.
fn flatten_text(node: &GrammarASTNode) -> String {
    let mut out = String::new();
    flatten_into(node, &mut out);
    out
}

fn flatten_into(node: &GrammarASTNode, out: &mut String) {
    for c in &node.children {
        match c {
            ASTNodeOrToken::Token(tok) => out.push_str(&tok.value),
            ASTNodeOrToken::Node(child) => flatten_into(child, out),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_source_yields_empty_sidecar() {
        let sidecar = extract_types("", "anchor.1").unwrap();
        assert!(sidecar.records.is_empty());
    }

    #[test]
    fn type_number_lowers_to_number_primitive() {
        let sidecar = extract_types("@type {number}\n", "anchor.1").unwrap();
        let record = sidecar.get(&"anchor.1".to_string()).expect("record present");
        assert_eq!(record.ty, Some(Type::Number));
    }

    #[test]
    fn type_string_lowers_to_string_primitive() {
        let sidecar = extract_types("@type {string}\n", "x").unwrap();
        let record = sidecar.get(&"x".to_string()).unwrap();
        assert_eq!(record.ty, Some(Type::String));
    }

    #[test]
    fn type_boolean_lowers_to_boolean_primitive() {
        let sidecar = extract_types("@type {boolean}\n", "x").unwrap();
        assert_eq!(sidecar.ty(&"x".to_string()), Some(&Type::Boolean));
    }

    #[test]
    fn type_void_maps_to_undefined() {
        // Convention: JSDoc `void` carries the same meaning as `undefined`.
        let sidecar = extract_types("@type {void}\n", "x").unwrap();
        assert_eq!(sidecar.ty(&"x".to_string()), Some(&Type::Undefined));
    }

    #[test]
    fn type_foo_lowers_to_opaque_with_raw_text() {
        // `Foo` isn't a primitive in the current lattice — falls back
        // to Opaque carrying the original raw text.
        let sidecar = extract_types("@type {Foo}\n", "x").unwrap();
        let ty = sidecar.ty(&"x".to_string()).unwrap();
        assert!(matches!(ty, Type::Opaque { raw } if raw == "Foo"));
    }

    #[test]
    fn type_dotted_lowers_to_opaque() {
        let sidecar = extract_types("@type {Foo.Bar}\n", "x").unwrap();
        let ty = sidecar.ty(&"x".to_string()).unwrap();
        match ty {
            Type::Opaque { raw } => {
                assert!(raw.contains("Foo"));
                assert!(raw.contains("Bar"));
            }
            _ => panic!("expected Opaque, got {:?}", ty),
        }
    }

    #[test]
    fn param_tag_appears_in_extension_params_array() {
        let sidecar = extract_types("@param {string} name\n", "x").unwrap();
        let record = sidecar.get(&"x".to_string()).expect("record present");
        let params = record
            .attributes
            .extension
            .get("params")
            .and_then(|v| v.as_array())
            .expect("params array");
        assert_eq!(params.len(), 1);
        assert_eq!(params[0]["type"], "string");
        assert_eq!(params[0]["name"], "name");
    }

    #[test]
    fn returns_tag_appears_in_extension_returns_object() {
        let sidecar = extract_types("@returns {boolean}\n", "x").unwrap();
        let record = sidecar.get(&"x".to_string()).unwrap();
        let returns = record
            .attributes
            .extension
            .get("returns")
            .expect("returns object");
        assert_eq!(returns["type"], "boolean");
    }

    #[test]
    fn multiple_tags_coalesce_into_one_record() {
        let src = "@param {string} name\n@param {number} count\n@returns {boolean}\n";
        let sidecar = extract_types(src, "x").unwrap();
        assert_eq!(sidecar.records.len(), 1);
        let record = sidecar.get(&"x".to_string()).unwrap();
        let params = record
            .attributes
            .extension
            .get("params")
            .and_then(|v| v.as_array())
            .unwrap();
        assert_eq!(params.len(), 2);
        assert!(record.attributes.extension.contains_key("returns"));
    }

    #[test]
    fn provenance_records_jsdoc_producer() {
        let sidecar = extract_types("@type {number}\n", "x").unwrap();
        let record = sidecar.get(&"x".to_string()).unwrap();
        assert_eq!(record.provenance.producer, ProducerId::new("jsdoc"));
        assert_eq!(record.provenance.producer_version, "0.1.0");
        assert_eq!(
            record.provenance.source_location.as_deref(),
            Some("extractor")
        );
        // At least the "extract" evidence step.
        assert!(record
            .provenance
            .evidence
            .iter()
            .any(|step| step.stage == "extract"));
    }

    #[test]
    fn unknown_tag_is_ignored_silently() {
        // `@throws` isn't in v1's recognised set — but jsdoc-parser
        // catches it via unknown_tag, and we just don't emit anything.
        let sidecar = extract_types("@throws {Error} bad\n", "x").unwrap();
        assert!(
            sidecar.records.is_empty(),
            "expected no records, got {:?}",
            sidecar.records
        );
    }

    #[test]
    fn type_and_param_both_present_keeps_ty_and_extension() {
        // Reading a `@type {Foo}` and `@param {string} x` together
        // should preserve both the structural type and the param.
        let src = "@type {number}\n@param {string} x\n";
        let sidecar = extract_types(src, "anchor").unwrap();
        let record = sidecar.get(&"anchor".to_string()).unwrap();
        assert_eq!(record.ty, Some(Type::Number));
        assert!(record.attributes.extension.contains_key("params"));
    }

    #[test]
    fn extract_from_ast_works_with_preparsed_node() {
        let ast = parse_jsdoc("@type {string}\n").unwrap();
        let sidecar = extract_from_ast(&ast, "z");
        assert_eq!(sidecar.ty(&"z".to_string()), Some(&Type::String));
    }
}
