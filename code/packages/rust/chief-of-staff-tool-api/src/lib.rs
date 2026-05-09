//! # chief-of-staff-tool-api
//!
//! `chief-of-staff-tool-api` is the repository-owned contract between Chief of
//! Staff agents and the tools they can call.
//!
//! The D18D spec divides that contract into three stable pieces:
//!
//! ```text
//! ToolDefinition  -- what exists and what it is allowed to do
//! ToolInvocation  -- one requested call with arguments and provenance
//! ToolEvents      -- the observable execution stream and final result
//! ```
//!
//! This crate owns those nouns plus validation helpers. It deliberately does not
//! launch processes, talk to a model, or perform side effects. Runtime crates can
//! layer approval checks, sandbox execution, and built-in handlers on top of
//! these types without changing the model-facing shape.

#![forbid(unsafe_code)]

use coding_adventures_json_value::{JsonNumber, JsonValue};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Milliseconds since the Unix epoch.
pub type TimestampMs = u64;

// ============================================================================
// ToolDefinition
// ============================================================================

/// Repository-owned dotted identifier for a tool.
///
/// D18D uses lowercase dotted names such as `context.open_session` and
/// `artifact.create`. Each segment must start with an ASCII lowercase letter and
/// may then contain ASCII lowercase letters, digits, or underscores.
pub type ToolId = String;

/// How much side effect risk a tool carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolSideEffects {
    None,
    Read,
    Write,
    External,
}

impl ToolSideEffects {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Read => "read",
            Self::Write => "write",
            Self::External => "external",
        }
    }
}

impl Display for ToolSideEffects {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Whether a tool can be safely retried by a runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolIdempotency {
    Always,
    Conditional,
    Never,
}

impl ToolIdempotency {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::Conditional => "conditional",
            Self::Never => "never",
        }
    }
}

impl Display for ToolIdempotency {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Whether multiple calls to the same tool can run at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolConcurrency {
    Safe,
    Serialized,
}

impl ToolConcurrency {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Serialized => "serialized",
        }
    }
}

impl Display for ToolConcurrency {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Whether a tool emits only a final result or an event stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStreaming {
    None,
    Events,
}

impl ToolStreaming {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Events => "events",
        }
    }
}

impl Display for ToolStreaming {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stability level advertised by a tool definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStability {
    Experimental,
    Stable,
    Deprecated,
}

impl ToolStability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Experimental => "experimental",
            Self::Stable => "stable",
            Self::Deprecated => "deprecated",
        }
    }
}

impl Display for ToolStability {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Chief of Staff privilege tier required to call a tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrivilegeTier {
    Tier0,
    Tier1,
    Tier2,
    Tier3,
}

impl PrivilegeTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tier0 => "tier0",
            Self::Tier1 => "tier1",
            Self::Tier2 => "tier2",
            Self::Tier3 => "tier3",
        }
    }
}

impl Display for PrivilegeTier {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One model-facing tool definition.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolDefinition {
    pub tool_id: ToolId,
    pub display_name: String,
    pub description: String,
    pub input_schema: JsonSchema,
    pub output_schema: Option<JsonSchema>,
    pub side_effects: ToolSideEffects,
    pub idempotency: ToolIdempotency,
    pub concurrency: ToolConcurrency,
    pub streaming: ToolStreaming,
    pub required_tier: PrivilegeTier,
    pub required_capabilities: Vec<String>,
    pub preferred_lock_scope: Option<String>,
    pub timeout_seconds: Option<u32>,
    pub tags: Vec<String>,
    pub stability: ToolStability,
}

/// Schema-light catalog row for list/read tools that need to inspect tool
/// policy shape before fetching full schemas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDefinitionSummary {
    pub tool_id: ToolId,
    pub display_name: String,
    pub side_effects: ToolSideEffects,
    pub idempotency: ToolIdempotency,
    pub concurrency: ToolConcurrency,
    pub streaming: ToolStreaming,
    pub required_tier: PrivilegeTier,
    pub required_capabilities: Vec<String>,
    pub preferred_lock_scope: Option<String>,
    pub timeout_seconds: Option<u32>,
    pub tags: Vec<String>,
    pub stability: ToolStability,
    pub has_output_schema: bool,
}

impl ToolDefinitionSummary {
    pub fn from_definition(definition: &ToolDefinition) -> Self {
        Self {
            tool_id: definition.tool_id.clone(),
            display_name: definition.display_name.clone(),
            side_effects: definition.side_effects,
            idempotency: definition.idempotency,
            concurrency: definition.concurrency,
            streaming: definition.streaming,
            required_tier: definition.required_tier,
            required_capabilities: definition.required_capabilities.clone(),
            preferred_lock_scope: definition.preferred_lock_scope.clone(),
            timeout_seconds: definition.timeout_seconds,
            tags: definition.tags.clone(),
            stability: definition.stability,
            has_output_schema: definition.output_schema.is_some(),
        }
    }
}

impl ToolDefinition {
    /// Validate definition metadata and both schemas.
    pub fn validate(&self) -> ToolValidationReport {
        let mut report = ToolValidationReport::empty();
        push_id_issue(
            &mut report.errors,
            "tool_id",
            &validate_tool_id(&self.tool_id),
        );
        validate_non_empty("display_name", &self.display_name, &mut report.errors);
        validate_non_empty("description", &self.description, &mut report.errors);
        validate_labels(
            "required_capabilities",
            &self.required_capabilities,
            &mut report.errors,
        );
        validate_labels("tags", &self.tags, &mut report.errors);
        if let Some(scope) = &self.preferred_lock_scope {
            validate_non_empty("preferred_lock_scope", scope, &mut report.errors);
        }
        if self.timeout_seconds == Some(0) {
            report.errors.push(issue(
                "timeout_seconds",
                "timeout must be greater than zero",
            ));
        }
        self.input_schema
            .validate_schema("input_schema", &mut report.errors);
        if let Some(schema) = &self.output_schema {
            schema.validate_schema("output_schema", &mut report.errors);
        }
        report.ok = report.errors.is_empty();
        report
    }

    /// Project the input schema into a provider-neutral JSON Schema document.
    pub fn input_json_schema(&self) -> JsonValue {
        self.input_schema.to_json_schema_value()
    }

    /// Project the output schema into a provider-neutral JSON Schema document.
    pub fn output_json_schema(&self) -> Option<JsonValue> {
        self.output_schema
            .as_ref()
            .map(JsonSchema::to_json_schema_value)
    }

    /// Return the model-gateway-facing schema document for this tool.
    pub fn schema_document(&self) -> ToolSchemaDocument {
        ToolSchemaDocument {
            tool_id: self.tool_id.clone(),
            display_name: self.display_name.clone(),
            description: self.description.clone(),
            input_schema: self.input_json_schema(),
            output_schema: self.output_json_schema(),
        }
    }
}

/// Provider-neutral schema export for model gateway adapters.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolSchemaDocument {
    pub tool_id: ToolId,
    pub display_name: String,
    pub description: String,
    pub input_schema: JsonValue,
    pub output_schema: Option<JsonValue>,
}

// ============================================================================
// JSON schema subset
// ============================================================================

/// Small JSON-schema-like validator for tool inputs and outputs.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonSchema {
    Any,
    Null,
    Boolean,
    Integer,
    Number,
    String,
    Array {
        items: Box<JsonSchema>,
    },
    Object {
        properties: Vec<SchemaProperty>,
        required: Vec<String>,
        allow_unknown_fields: bool,
    },
    Enum {
        values: Vec<JsonValue>,
    },
}

impl JsonSchema {
    /// Project the repository-owned schema subset into JSON Schema shape.
    ///
    /// Provider adapters can translate this neutral document into OpenAI,
    /// Anthropic, local-model, or other concrete tool formats without reaching
    /// back into D18D internals.
    pub fn to_json_schema_value(&self) -> JsonValue {
        match self {
            Self::Any => JsonValue::Bool(true),
            Self::Null => json_schema_type("null"),
            Self::Boolean => json_schema_type("boolean"),
            Self::Integer => json_schema_type("integer"),
            Self::Number => json_schema_type("number"),
            Self::String => json_schema_type("string"),
            Self::Array { items } => JsonValue::Object(vec![
                ("type".to_string(), JsonValue::String("array".to_string())),
                ("items".to_string(), items.to_json_schema_value()),
            ]),
            Self::Object {
                properties,
                required,
                allow_unknown_fields,
            } => {
                let mut fields = vec![
                    ("type".to_string(), JsonValue::String("object".to_string())),
                    (
                        "properties".to_string(),
                        JsonValue::Object(
                            properties
                                .iter()
                                .map(|property| {
                                    (
                                        property.name.clone(),
                                        property.schema.to_json_schema_value(),
                                    )
                                })
                                .collect(),
                        ),
                    ),
                    (
                        "additionalProperties".to_string(),
                        JsonValue::Bool(*allow_unknown_fields),
                    ),
                ];
                if !required.is_empty() {
                    fields.push((
                        "required".to_string(),
                        JsonValue::Array(
                            required
                                .iter()
                                .map(|name| JsonValue::String(name.clone()))
                                .collect(),
                        ),
                    ));
                }
                JsonValue::Object(fields)
            }
            Self::Enum { values } => {
                JsonValue::Object(vec![("enum".to_string(), JsonValue::Array(values.clone()))])
            }
        }
    }

    /// Validate a value against this schema.
    pub fn validate_value(&self, value: &JsonValue) -> ToolValidationReport {
        let mut report = ToolValidationReport::ok(value.clone());
        self.validate_value_at(value, "$", &mut report.errors);
        report.ok = report.errors.is_empty();
        report
    }

    fn validate_schema(&self, path: &str, errors: &mut Vec<ToolValidationIssue>) {
        match self {
            Self::Array { items } => {
                items.validate_schema(&format!("{path}.items"), errors);
            }
            Self::Object {
                properties,
                required,
                ..
            } => {
                let mut names = BTreeMap::new();
                for property in properties {
                    validate_schema_key(&format!("{path}.properties"), &property.name, errors);
                    if names.insert(property.name.as_str(), ()).is_some() {
                        errors.push(issue(
                            format!("{path}.properties.{}", property.name),
                            "duplicate property",
                        ));
                    }
                    property
                        .schema
                        .validate_schema(&format!("{path}.properties.{}", property.name), errors);
                }
                for name in required {
                    validate_schema_key(&format!("{path}.required"), name, errors);
                    if !properties.iter().any(|property| property.name == *name) {
                        errors.push(issue(
                            format!("{path}.required.{name}"),
                            "required field is not declared as a property",
                        ));
                    }
                }
            }
            Self::Enum { values } if values.is_empty() => {
                errors.push(issue(path, "enum must contain at least one value"));
            }
            Self::Any
            | Self::Null
            | Self::Boolean
            | Self::Integer
            | Self::Number
            | Self::String
            | Self::Enum { .. } => {}
        }
    }

    fn validate_value_at(
        &self,
        value: &JsonValue,
        path: &str,
        errors: &mut Vec<ToolValidationIssue>,
    ) {
        match self {
            Self::Any => {}
            Self::Null => {
                if !matches!(value, JsonValue::Null) {
                    errors.push(type_issue(path, "null", value));
                }
            }
            Self::Boolean => {
                if !matches!(value, JsonValue::Bool(_)) {
                    errors.push(type_issue(path, "boolean", value));
                }
            }
            Self::Integer => {
                if !matches!(value, JsonValue::Number(JsonNumber::Integer(_))) {
                    errors.push(type_issue(path, "integer", value));
                }
            }
            Self::Number => match value {
                JsonValue::Number(JsonNumber::Integer(_)) => {}
                JsonValue::Number(JsonNumber::Float(number)) if number.is_finite() => {}
                JsonValue::Number(JsonNumber::Float(_)) => {
                    errors.push(issue(path, "number must be finite"));
                }
                _ => errors.push(type_issue(path, "number", value)),
            },
            Self::String => {
                if !matches!(value, JsonValue::String(_)) {
                    errors.push(type_issue(path, "string", value));
                }
            }
            Self::Array { items } => match value {
                JsonValue::Array(values) => {
                    for (index, item) in values.iter().enumerate() {
                        items.validate_value_at(item, &format!("{path}[{index}]"), errors);
                    }
                }
                _ => errors.push(type_issue(path, "array", value)),
            },
            Self::Object {
                properties,
                required,
                allow_unknown_fields,
            } => match value {
                JsonValue::Object(fields) => {
                    for required_name in required {
                        if !fields.iter().any(|(name, _)| name == required_name) {
                            errors.push(issue(
                                format!("{path}.{required_name}"),
                                "required field is missing",
                            ));
                        }
                    }
                    for (field_name, field_value) in fields {
                        if let Some(property) = properties
                            .iter()
                            .find(|property| property.name == *field_name)
                        {
                            property.schema.validate_value_at(
                                field_value,
                                &format!("{path}.{field_name}"),
                                errors,
                            );
                        } else if !allow_unknown_fields {
                            errors.push(issue(
                                format!("{path}.{field_name}"),
                                "unknown field is not allowed",
                            ));
                        }
                    }
                }
                _ => errors.push(type_issue(path, "object", value)),
            },
            Self::Enum { values } => {
                if !values.iter().any(|candidate| candidate == value) {
                    errors.push(issue(path, "value is not in enum"));
                }
            }
        }
    }
}

/// One named property inside a [`JsonSchema::Object`].
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaProperty {
    pub name: String,
    pub schema: JsonSchema,
}

impl SchemaProperty {
    pub fn new(name: impl Into<String>, schema: JsonSchema) -> Self {
        Self {
            name: name.into(),
            schema,
        }
    }
}

// ============================================================================
// Built-in tool catalog
// ============================================================================

/// Built-in D18D tool families backed by repository services.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BuiltinToolFamily {
    Context,
    Artifact,
    Skill,
    Memory,
    Job,
}

impl BuiltinToolFamily {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Context => "context",
            Self::Artifact => "artifact",
            Self::Skill => "skill",
            Self::Memory => "memory",
            Self::Job => "job",
        }
    }
}

impl Display for BuiltinToolFamily {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Query options for selecting model-facing tool definitions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolCatalogQuery {
    pub family: Option<BuiltinToolFamily>,
    pub side_effects: Option<ToolSideEffects>,
    pub max_tier: Option<PrivilegeTier>,
    pub required_capabilities: Vec<String>,
    pub tags: Vec<String>,
    pub stability: Option<ToolStability>,
    pub limit: Option<usize>,
}

impl ToolCatalogQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_family(mut self, family: BuiltinToolFamily) -> Self {
        self.family = Some(family);
        self
    }

    pub fn with_side_effects(mut self, side_effects: ToolSideEffects) -> Self {
        self.side_effects = Some(side_effects);
        self
    }

    pub fn with_max_tier(mut self, max_tier: PrivilegeTier) -> Self {
        self.max_tier = Some(max_tier);
        self
    }

    pub fn requiring_capability(mut self, capability: impl Into<String>) -> Self {
        self.required_capabilities.push(capability.into());
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn with_stability(mut self, stability: ToolStability) -> Self {
        self.stability = Some(stability);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Summary counts for a catalog export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCatalogSummary {
    pub total_tools: usize,
    pub by_family: BTreeMap<String, usize>,
    pub by_side_effects: BTreeMap<String, usize>,
    pub by_required_tier: BTreeMap<String, usize>,
    pub by_stability: BTreeMap<String, usize>,
    pub streaming_tools: usize,
    pub write_or_external_tools: usize,
}

impl ToolCatalogSummary {
    pub fn empty() -> Self {
        Self {
            total_tools: 0,
            by_family: BTreeMap::new(),
            by_side_effects: BTreeMap::new(),
            by_required_tier: BTreeMap::new(),
            by_stability: BTreeMap::new(),
            streaming_tools: 0,
            write_or_external_tools: 0,
        }
    }

    pub fn from_definitions<'a, I>(definitions: I) -> Self
    where
        I: IntoIterator<Item = &'a ToolDefinition>,
    {
        let mut summary = Self::empty();
        for definition in definitions {
            summary.total_tools += 1;
            increment_count(
                &mut summary.by_family,
                tool_family_label(&definition.tool_id),
            );
            increment_count(
                &mut summary.by_side_effects,
                definition.side_effects.as_str(),
            );
            increment_count(
                &mut summary.by_required_tier,
                definition.required_tier.as_str(),
            );
            increment_count(&mut summary.by_stability, definition.stability.as_str());
            if definition.streaming == ToolStreaming::Events {
                summary.streaming_tools += 1;
            }
            if matches!(
                definition.side_effects,
                ToolSideEffects::Write | ToolSideEffects::External
            ) {
                summary.write_or_external_tools += 1;
            }
        }
        summary
    }
}

/// Provider-facing catalog export for model gateways and portability checks.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCatalogExport {
    pub schema_documents: Vec<ToolSchemaDocument>,
    pub summary: ToolCatalogSummary,
    pub validation: ToolValidationReport,
}

impl ToolCatalogExport {
    pub fn from_definitions<'a, I>(definitions: I) -> Self
    where
        I: IntoIterator<Item = &'a ToolDefinition>,
    {
        let definitions: Vec<&ToolDefinition> = definitions.into_iter().collect();
        Self {
            schema_documents: definitions
                .iter()
                .map(|definition| definition.schema_document())
                .collect(),
            summary: ToolCatalogSummary::from_definitions(definitions.iter().copied()),
            validation: validate_tool_catalog(definitions.iter().copied()),
        }
    }

    pub fn ok(&self) -> bool {
        self.validation.ok
    }

    pub fn tool_ids(&self) -> Vec<&str> {
        self.schema_documents
            .iter()
            .map(|document| document.tool_id.as_str())
            .collect()
    }
}

/// Return the first-phase built-in store/job tool definitions from D18D.
pub fn builtin_tool_catalog() -> Vec<ToolDefinition> {
    [
        context_open_session_definition(),
        context_append_entry_definition(),
        context_read_entries_definition(),
        context_create_snapshot_definition(),
        context_compact_definition(),
        context_archive_session_definition(),
        artifact_create_definition(),
        artifact_write_revision_definition(),
        artifact_read_definition(),
        artifact_read_revision_definition(),
        artifact_list_definition(),
        artifact_tag_definition(),
        artifact_mark_retention_definition(),
        skill_list_definition(),
        skill_read_manifest_definition(),
        skill_read_asset_definition(),
        skill_install_definition(),
        skill_activate_definition(),
        skill_deactivate_definition(),
        skill_uninstall_definition(),
        memory_remember_definition(),
        memory_search_definition(),
        memory_list_by_class_definition(),
        memory_list_by_tag_definition(),
        memory_supersede_definition(),
        memory_expire_definition(),
        memory_tombstone_definition(),
        job_validate_definition(),
        job_install_definition(),
        job_uninstall_definition(),
        job_run_now_definition(),
        job_list_definition(),
        job_status_definition(),
    ]
    .into()
}

/// Return first-phase built-ins for one family.
pub fn builtin_tools_for_family(family: BuiltinToolFamily) -> Vec<ToolDefinition> {
    builtin_tools_matching(ToolCatalogQuery::new().for_family(family))
}

/// Query first-phase built-in definitions by catalog metadata.
pub fn builtin_tools_matching(query: ToolCatalogQuery) -> Vec<ToolDefinition> {
    if query.limit == Some(0) {
        return Vec::new();
    }

    let mut definitions = Vec::new();
    for definition in builtin_tool_catalog() {
        if !definition_matches_catalog_query(&definition, &query) {
            continue;
        }
        definitions.push(definition);
        if let Some(limit) = query.limit {
            if definitions.len() >= limit {
                break;
            }
        }
    }
    definitions
}

/// Export built-in catalog schemas, summary counts, and validation state.
pub fn builtin_tool_catalog_export(query: ToolCatalogQuery) -> ToolCatalogExport {
    let definitions = builtin_tools_matching(query);
    ToolCatalogExport::from_definitions(definitions.iter())
}

/// Export built-in schema documents for a model gateway adapter.
pub fn builtin_tool_schema_documents(query: ToolCatalogQuery) -> Vec<ToolSchemaDocument> {
    builtin_tool_catalog_export(query).schema_documents
}

/// Export schema-light built-in catalog summaries for read-side tools.
pub fn builtin_tool_definition_summaries(query: ToolCatalogQuery) -> Vec<ToolDefinitionSummary> {
    builtin_tools_matching(query)
        .iter()
        .map(ToolDefinitionSummary::from_definition)
        .collect()
}

/// Look up one first-phase built-in definition by id.
pub fn builtin_tool_definition(tool_id: &str) -> Option<ToolDefinition> {
    builtin_tool_catalog()
        .into_iter()
        .find(|definition| definition.tool_id == tool_id)
}

/// Validate catalog-wide invariants such as duplicate tool ids.
pub fn validate_tool_catalog<'a, I>(definitions: I) -> ToolValidationReport
where
    I: IntoIterator<Item = &'a ToolDefinition>,
{
    let mut report = ToolValidationReport::empty();
    let mut first_index_by_tool_id = BTreeMap::new();
    for (index, definition) in definitions.into_iter().enumerate() {
        let definition_report = definition.validate();
        report
            .errors
            .extend(definition_report.errors.into_iter().map(|error| {
                issue(
                    format!("tools[{index}].{}", error.path),
                    format!("{}: {}", definition.tool_id, error.message),
                )
            }));
        if let Some(first_index) = first_index_by_tool_id.insert(definition.tool_id.as_str(), index)
        {
            report.errors.push(issue(
                format!("tools[{index}].tool_id"),
                format!(
                    "duplicate tool id '{}' also appears at tools[{first_index}]",
                    definition.tool_id
                ),
            ));
        }
    }
    report.ok = report.errors.is_empty();
    report
}

fn definition_matches_catalog_query(definition: &ToolDefinition, query: &ToolCatalogQuery) -> bool {
    if let Some(family) = query.family {
        let family_prefix = format!("{}.", family.as_str());
        if !definition.tool_id.starts_with(&family_prefix) {
            return false;
        }
    }
    if let Some(side_effects) = query.side_effects {
        if definition.side_effects != side_effects {
            return false;
        }
    }
    if let Some(max_tier) = query.max_tier {
        if definition.required_tier > max_tier {
            return false;
        }
    }
    if let Some(stability) = query.stability {
        if definition.stability != stability {
            return false;
        }
    }
    query.required_capabilities.iter().all(|capability| {
        definition
            .required_capabilities
            .iter()
            .any(|candidate| candidate == capability)
    }) && query
        .tags
        .iter()
        .all(|tag| definition.tags.iter().any(|candidate| candidate == tag))
}

fn increment_count(counts: &mut BTreeMap<String, usize>, label: impl Into<String>) {
    *counts.entry(label.into()).or_insert(0) += 1;
}

fn tool_family_label(tool_id: &str) -> &str {
    tool_id
        .split_once('.')
        .map_or("unknown", |(family, _)| family)
}

fn context_open_session_definition() -> ToolDefinition {
    builtin_definition(
        "context.open_session",
        "Open context session",
        "Open a durable Chief of Staff context session by id.",
        object_schema(
            vec![
                SchemaProperty::new("session_id", JsonSchema::String),
                SchemaProperty::new("create_if_missing", JsonSchema::Boolean),
            ],
            vec!["session_id"],
            false,
        ),
        Some(object_schema(
            vec![
                SchemaProperty::new("session_id", JsonSchema::String),
                SchemaProperty::new("status", JsonSchema::String),
            ],
            vec!["session_id", "status"],
            false,
        )),
        ToolSideEffects::Read,
        ToolIdempotency::Always,
        ToolConcurrency::Safe,
        ToolStreaming::None,
        PrivilegeTier::Tier0,
        vec!["context:read"],
        None,
        vec!["context", "store"],
    )
}

fn context_append_entry_definition() -> ToolDefinition {
    builtin_definition(
        "context.append_entry",
        "Append context entry",
        "Append one ordered transcript entry to a context session.",
        object_schema(
            vec![
                SchemaProperty::new("session_id", JsonSchema::String),
                SchemaProperty::new("role", JsonSchema::String),
                SchemaProperty::new("content", JsonSchema::String),
                SchemaProperty::new("metadata", JsonSchema::Any),
            ],
            vec!["session_id", "role", "content"],
            false,
        ),
        Some(object_schema(
            vec![
                SchemaProperty::new("session_id", JsonSchema::String),
                SchemaProperty::new("entry_id", JsonSchema::String),
            ],
            vec!["session_id", "entry_id"],
            false,
        )),
        ToolSideEffects::Write,
        ToolIdempotency::Conditional,
        ToolConcurrency::Serialized,
        ToolStreaming::Events,
        PrivilegeTier::Tier0,
        vec!["context:write"],
        Some("context"),
        vec!["context", "store"],
    )
}

fn context_read_entries_definition() -> ToolDefinition {
    builtin_definition(
        "context.read_entries",
        "Read context entries",
        "Read ordered transcript entries from a durable context session.",
        object_schema(
            vec![
                SchemaProperty::new("session_id", JsonSchema::String),
                SchemaProperty::new("after_entry_id", JsonSchema::String),
                SchemaProperty::new("limit", JsonSchema::Integer),
            ],
            vec!["session_id"],
            false,
        ),
        Some(object_schema(
            vec![
                SchemaProperty::new("session_id", JsonSchema::String),
                SchemaProperty::new(
                    "entries",
                    JsonSchema::Array {
                        items: Box::new(JsonSchema::Any),
                    },
                ),
                SchemaProperty::new("next_after_entry_id", JsonSchema::String),
            ],
            vec!["session_id", "entries"],
            false,
        )),
        ToolSideEffects::Read,
        ToolIdempotency::Always,
        ToolConcurrency::Safe,
        ToolStreaming::None,
        PrivilegeTier::Tier0,
        vec!["context:read"],
        None,
        vec!["context", "store"],
    )
}

fn context_create_snapshot_definition() -> ToolDefinition {
    builtin_definition(
        "context.create_snapshot",
        "Create context snapshot",
        "Create a compact durable snapshot for a context session.",
        object_schema(
            vec![
                SchemaProperty::new("session_id", JsonSchema::String),
                SchemaProperty::new("basis_entry_id", JsonSchema::String),
                SchemaProperty::new("included_entry_ids", string_array_schema()),
                SchemaProperty::new("summary_refs", string_array_schema()),
                SchemaProperty::new("memory_refs", string_array_schema()),
                SchemaProperty::new("artifact_refs", string_array_schema()),
                SchemaProperty::new("token_estimate", JsonSchema::Integer),
            ],
            vec!["session_id", "basis_entry_id"],
            false,
        ),
        Some(object_schema(
            vec![
                SchemaProperty::new("session_id", JsonSchema::String),
                SchemaProperty::new("snapshot_id", JsonSchema::String),
            ],
            vec!["session_id", "snapshot_id"],
            false,
        )),
        ToolSideEffects::Write,
        ToolIdempotency::Conditional,
        ToolConcurrency::Serialized,
        ToolStreaming::Events,
        PrivilegeTier::Tier0,
        vec!["context:write"],
        Some("context"),
        vec!["context", "store"],
    )
}

fn context_compact_definition() -> ToolDefinition {
    builtin_definition(
        "context.compact",
        "Compact context",
        "Compact older context entries into a durable summary reference.",
        object_schema(
            vec![
                SchemaProperty::new("session_id", JsonSchema::String),
                SchemaProperty::new("before_entry_id", JsonSchema::String),
                SchemaProperty::new("summary_ref", JsonSchema::String),
            ],
            vec!["session_id", "before_entry_id", "summary_ref"],
            false,
        ),
        Some(object_schema(
            vec![
                SchemaProperty::new("session_id", JsonSchema::String),
                SchemaProperty::new("compacted_before_entry_id", JsonSchema::String),
                SchemaProperty::new("summary_ref", JsonSchema::String),
            ],
            vec!["session_id", "compacted_before_entry_id", "summary_ref"],
            false,
        )),
        ToolSideEffects::Write,
        ToolIdempotency::Conditional,
        ToolConcurrency::Serialized,
        ToolStreaming::Events,
        PrivilegeTier::Tier0,
        vec!["context:write"],
        Some("context"),
        vec!["context", "store"],
    )
}

fn context_archive_session_definition() -> ToolDefinition {
    builtin_definition(
        "context.archive_session",
        "Archive context session",
        "Archive a durable context session without deleting its entries.",
        object_schema(
            vec![SchemaProperty::new("session_id", JsonSchema::String)],
            vec!["session_id"],
            false,
        ),
        Some(object_schema(
            vec![
                SchemaProperty::new("session_id", JsonSchema::String),
                SchemaProperty::new("status", JsonSchema::String),
            ],
            vec!["session_id", "status"],
            false,
        )),
        ToolSideEffects::Write,
        ToolIdempotency::Always,
        ToolConcurrency::Serialized,
        ToolStreaming::Events,
        PrivilegeTier::Tier0,
        vec!["context:write"],
        Some("context"),
        vec!["context", "store"],
    )
}

fn artifact_create_definition() -> ToolDefinition {
    builtin_definition(
        "artifact.create",
        "Create artifact",
        "Create a durable artifact manifest and first revision.",
        object_schema(
            vec![
                SchemaProperty::new("collection", JsonSchema::String),
                SchemaProperty::new("name", JsonSchema::String),
                SchemaProperty::new("content_type", JsonSchema::String),
                SchemaProperty::new("body_base64", JsonSchema::String),
                SchemaProperty::new("labels", string_array_schema()),
            ],
            vec!["collection", "name", "content_type", "body_base64"],
            false,
        ),
        Some(object_schema(
            vec![
                SchemaProperty::new("artifact_id", JsonSchema::String),
                SchemaProperty::new("revision_id", JsonSchema::String),
            ],
            vec!["artifact_id", "revision_id"],
            false,
        )),
        ToolSideEffects::Write,
        ToolIdempotency::Conditional,
        ToolConcurrency::Safe,
        ToolStreaming::None,
        PrivilegeTier::Tier0,
        vec!["artifacts:create"],
        None,
        vec!["artifact", "store"],
    )
}

fn artifact_write_revision_definition() -> ToolDefinition {
    builtin_definition(
        "artifact.write_revision",
        "Write artifact revision",
        "Append a new opaque revision to a durable artifact.",
        object_schema(
            vec![
                SchemaProperty::new("artifact_id", JsonSchema::String),
                SchemaProperty::new("parent_revision_id", JsonSchema::String),
                SchemaProperty::new("content_type", JsonSchema::String),
                SchemaProperty::new("body_base64", JsonSchema::String),
                SchemaProperty::new("metadata", JsonSchema::Any),
            ],
            vec!["artifact_id", "content_type", "body_base64"],
            false,
        ),
        Some(object_schema(
            vec![
                SchemaProperty::new("artifact_id", JsonSchema::String),
                SchemaProperty::new("revision_id", JsonSchema::String),
            ],
            vec!["artifact_id", "revision_id"],
            false,
        )),
        ToolSideEffects::Write,
        ToolIdempotency::Conditional,
        ToolConcurrency::Serialized,
        ToolStreaming::Events,
        PrivilegeTier::Tier0,
        vec!["artifacts:write"],
        Some("artifact"),
        vec!["artifact", "store"],
    )
}

fn artifact_read_definition() -> ToolDefinition {
    builtin_definition(
        "artifact.read",
        "Read artifact",
        "Read a durable artifact manifest and latest revision reference.",
        object_schema(
            vec![SchemaProperty::new("artifact_id", JsonSchema::String)],
            vec!["artifact_id"],
            false,
        ),
        Some(object_schema(
            vec![
                SchemaProperty::new("artifact_id", JsonSchema::String),
                SchemaProperty::new("latest_revision_id", JsonSchema::String),
                SchemaProperty::new("content_type", JsonSchema::String),
                SchemaProperty::new("body_base64", JsonSchema::String),
            ],
            vec!["artifact_id"],
            false,
        )),
        ToolSideEffects::Read,
        ToolIdempotency::Always,
        ToolConcurrency::Safe,
        ToolStreaming::None,
        PrivilegeTier::Tier0,
        vec!["artifacts:read"],
        None,
        vec!["artifact", "store"],
    )
}

fn artifact_read_revision_definition() -> ToolDefinition {
    builtin_definition(
        "artifact.read_revision",
        "Read artifact revision",
        "Read one opaque artifact revision by id.",
        object_schema(
            vec![
                SchemaProperty::new("artifact_id", JsonSchema::String),
                SchemaProperty::new("revision_id", JsonSchema::String),
            ],
            vec!["artifact_id", "revision_id"],
            false,
        ),
        Some(object_schema(
            vec![
                SchemaProperty::new("artifact_id", JsonSchema::String),
                SchemaProperty::new("revision_id", JsonSchema::String),
                SchemaProperty::new("parent_revision_id", JsonSchema::String),
                SchemaProperty::new("content_type", JsonSchema::String),
                SchemaProperty::new("body_base64", JsonSchema::String),
            ],
            vec!["artifact_id", "revision_id", "content_type", "body_base64"],
            false,
        )),
        ToolSideEffects::Read,
        ToolIdempotency::Always,
        ToolConcurrency::Safe,
        ToolStreaming::None,
        PrivilegeTier::Tier0,
        vec!["artifacts:read"],
        None,
        vec!["artifact", "store"],
    )
}

fn artifact_list_definition() -> ToolDefinition {
    builtin_definition(
        "artifact.list",
        "List artifacts",
        "List durable artifacts by collection and labels.",
        object_schema(
            vec![
                SchemaProperty::new("collection", JsonSchema::String),
                SchemaProperty::new("labels", string_array_schema()),
                SchemaProperty::new("limit", JsonSchema::Integer),
            ],
            Vec::new(),
            false,
        ),
        Some(object_schema(
            vec![SchemaProperty::new(
                "artifacts",
                JsonSchema::Array {
                    items: Box::new(JsonSchema::Any),
                },
            )],
            vec!["artifacts"],
            false,
        )),
        ToolSideEffects::Read,
        ToolIdempotency::Always,
        ToolConcurrency::Safe,
        ToolStreaming::None,
        PrivilegeTier::Tier0,
        vec!["artifacts:list"],
        None,
        vec!["artifact", "store"],
    )
}

fn artifact_tag_definition() -> ToolDefinition {
    builtin_definition(
        "artifact.tag",
        "Tag artifact",
        "Attach labels to a durable artifact manifest.",
        object_schema(
            vec![
                SchemaProperty::new("artifact_id", JsonSchema::String),
                SchemaProperty::new("labels", string_array_schema()),
            ],
            vec!["artifact_id", "labels"],
            false,
        ),
        Some(object_schema(
            vec![
                SchemaProperty::new("artifact_id", JsonSchema::String),
                SchemaProperty::new("labels", string_array_schema()),
            ],
            vec!["artifact_id", "labels"],
            false,
        )),
        ToolSideEffects::Write,
        ToolIdempotency::Always,
        ToolConcurrency::Serialized,
        ToolStreaming::Events,
        PrivilegeTier::Tier0,
        vec!["artifacts:tag"],
        Some("artifact"),
        vec!["artifact", "store"],
    )
}

fn artifact_mark_retention_definition() -> ToolDefinition {
    builtin_definition(
        "artifact.mark_retention",
        "Mark artifact retention",
        "Mark a durable artifact as retained, temporary, or exported.",
        object_schema(
            vec![
                SchemaProperty::new("artifact_id", JsonSchema::String),
                SchemaProperty::new(
                    "retention",
                    string_enum(&["retained", "temporary", "exported"]),
                ),
            ],
            vec!["artifact_id", "retention"],
            false,
        ),
        Some(object_schema(
            vec![
                SchemaProperty::new("artifact_id", JsonSchema::String),
                SchemaProperty::new("retention", JsonSchema::String),
            ],
            vec!["artifact_id", "retention"],
            false,
        )),
        ToolSideEffects::Write,
        ToolIdempotency::Always,
        ToolConcurrency::Serialized,
        ToolStreaming::Events,
        PrivilegeTier::Tier0,
        vec!["artifacts:retention"],
        Some("artifact"),
        vec!["artifact", "store"],
    )
}

fn skill_list_definition() -> ToolDefinition {
    builtin_definition(
        "skill.list",
        "List skills",
        "List installed Chief of Staff skills.",
        object_schema(
            vec![SchemaProperty::new("active_only", JsonSchema::Boolean)],
            Vec::new(),
            false,
        ),
        Some(object_schema(
            vec![SchemaProperty::new(
                "skills",
                JsonSchema::Array {
                    items: Box::new(object_schema(
                        vec![
                            SchemaProperty::new("skill_id", JsonSchema::String),
                            SchemaProperty::new("version", JsonSchema::String),
                            SchemaProperty::new("name", JsonSchema::String),
                            SchemaProperty::new("active", JsonSchema::Boolean),
                        ],
                        vec!["skill_id", "version", "name", "active"],
                        false,
                    )),
                },
            )],
            vec!["skills"],
            false,
        )),
        ToolSideEffects::Read,
        ToolIdempotency::Always,
        ToolConcurrency::Safe,
        ToolStreaming::None,
        PrivilegeTier::Tier0,
        vec!["skills:read"],
        None,
        vec!["skill", "store"],
    )
}

fn skill_read_manifest_definition() -> ToolDefinition {
    builtin_definition(
        "skill.read_manifest",
        "Read skill manifest",
        "Read one installed Chief of Staff skill manifest.",
        object_schema(
            vec![
                SchemaProperty::new("skill_id", JsonSchema::String),
                SchemaProperty::new("version", JsonSchema::String),
            ],
            vec!["skill_id", "version"],
            false,
        ),
        Some(object_schema(
            vec![
                SchemaProperty::new("skill_id", JsonSchema::String),
                SchemaProperty::new("version", JsonSchema::String),
                SchemaProperty::new("manifest", JsonSchema::Any),
            ],
            vec!["skill_id", "version", "manifest"],
            false,
        )),
        ToolSideEffects::Read,
        ToolIdempotency::Always,
        ToolConcurrency::Safe,
        ToolStreaming::None,
        PrivilegeTier::Tier0,
        vec!["skills:read"],
        None,
        vec!["skill", "store"],
    )
}

fn skill_read_asset_definition() -> ToolDefinition {
    builtin_definition(
        "skill.read_asset",
        "Read skill asset",
        "Read one opaque skill asset by logical path.",
        object_schema(
            vec![
                SchemaProperty::new("skill_id", JsonSchema::String),
                SchemaProperty::new("version", JsonSchema::String),
                SchemaProperty::new("asset_path", JsonSchema::String),
            ],
            vec!["skill_id", "version", "asset_path"],
            false,
        ),
        Some(object_schema(
            vec![
                SchemaProperty::new("skill_id", JsonSchema::String),
                SchemaProperty::new("version", JsonSchema::String),
                SchemaProperty::new("asset_path", JsonSchema::String),
                SchemaProperty::new("content_type", JsonSchema::String),
                SchemaProperty::new("checksum_hex", JsonSchema::String),
                SchemaProperty::new("body_base64", JsonSchema::String),
            ],
            vec![
                "skill_id",
                "version",
                "asset_path",
                "content_type",
                "checksum_hex",
                "body_base64",
            ],
            false,
        )),
        ToolSideEffects::Read,
        ToolIdempotency::Always,
        ToolConcurrency::Safe,
        ToolStreaming::None,
        PrivilegeTier::Tier0,
        vec!["skills:read"],
        None,
        vec!["skill", "store"],
    )
}

fn skill_install_definition() -> ToolDefinition {
    builtin_definition(
        "skill.install",
        "Install skill",
        "Install a Chief of Staff skill manifest and bundled assets.",
        object_schema(
            vec![
                SchemaProperty::new("manifest", JsonSchema::Any),
                SchemaProperty::new(
                    "assets",
                    JsonSchema::Array {
                        items: Box::new(JsonSchema::Any),
                    },
                ),
                SchemaProperty::new("idempotency_key", JsonSchema::String),
            ],
            vec!["manifest", "assets"],
            false,
        ),
        Some(object_schema(
            vec![
                SchemaProperty::new("skill_id", JsonSchema::String),
                SchemaProperty::new("version", JsonSchema::String),
                SchemaProperty::new("installed", JsonSchema::Boolean),
            ],
            vec!["skill_id", "version", "installed"],
            false,
        )),
        ToolSideEffects::Write,
        ToolIdempotency::Conditional,
        ToolConcurrency::Serialized,
        ToolStreaming::Events,
        PrivilegeTier::Tier1,
        vec!["skills:install"],
        Some("skill"),
        vec!["skill", "store"],
    )
}

fn skill_activate_definition() -> ToolDefinition {
    builtin_definition(
        "skill.activate",
        "Activate skill",
        "Make one installed Chief of Staff skill version active.",
        skill_version_input_schema(),
        Some(skill_activation_output_schema()),
        ToolSideEffects::Write,
        ToolIdempotency::Conditional,
        ToolConcurrency::Serialized,
        ToolStreaming::Events,
        PrivilegeTier::Tier1,
        vec!["skills:activate"],
        Some("skill"),
        vec!["skill", "store"],
    )
}

fn skill_deactivate_definition() -> ToolDefinition {
    builtin_definition(
        "skill.deactivate",
        "Deactivate skill",
        "Deactivate one installed Chief of Staff skill version.",
        skill_version_input_schema(),
        Some(skill_activation_output_schema()),
        ToolSideEffects::Write,
        ToolIdempotency::Conditional,
        ToolConcurrency::Serialized,
        ToolStreaming::Events,
        PrivilegeTier::Tier1,
        vec!["skills:deactivate"],
        Some("skill"),
        vec!["skill", "store"],
    )
}

fn skill_uninstall_definition() -> ToolDefinition {
    builtin_definition(
        "skill.uninstall",
        "Uninstall skill",
        "Remove one installed Chief of Staff skill version and its assets.",
        skill_version_input_schema(),
        Some(object_schema(
            vec![
                SchemaProperty::new("skill_id", JsonSchema::String),
                SchemaProperty::new("version", JsonSchema::String),
                SchemaProperty::new("uninstalled", JsonSchema::Boolean),
            ],
            vec!["skill_id", "version", "uninstalled"],
            false,
        )),
        ToolSideEffects::Write,
        ToolIdempotency::Conditional,
        ToolConcurrency::Serialized,
        ToolStreaming::Events,
        PrivilegeTier::Tier1,
        vec!["skills:uninstall"],
        Some("skill"),
        vec!["skill", "store"],
    )
}

fn memory_remember_definition() -> ToolDefinition {
    builtin_definition(
        "memory.remember",
        "Remember",
        "Write one durable memory record.",
        object_schema(
            vec![
                SchemaProperty::new("class", JsonSchema::String),
                SchemaProperty::new("subject", JsonSchema::String),
                SchemaProperty::new("body", JsonSchema::String),
                SchemaProperty::new("tags", string_array_schema()),
                SchemaProperty::new("confidence", JsonSchema::Number),
            ],
            vec!["class", "body"],
            false,
        ),
        Some(object_schema(
            vec![SchemaProperty::new("memory_id", JsonSchema::String)],
            vec!["memory_id"],
            false,
        )),
        ToolSideEffects::Write,
        ToolIdempotency::Conditional,
        ToolConcurrency::Serialized,
        ToolStreaming::None,
        PrivilegeTier::Tier0,
        vec!["memory:write"],
        Some("memory"),
        vec!["memory", "store"],
    )
}

fn memory_search_definition() -> ToolDefinition {
    builtin_definition(
        "memory.search",
        "Search memory",
        "Search durable memory records.",
        object_schema(
            vec![
                SchemaProperty::new("query", JsonSchema::String),
                SchemaProperty::new("classes", string_array_schema()),
                SchemaProperty::new("tags", string_array_schema()),
                SchemaProperty::new("limit", JsonSchema::Integer),
            ],
            vec!["query"],
            false,
        ),
        Some(object_schema(
            vec![SchemaProperty::new(
                "matches",
                JsonSchema::Array {
                    items: Box::new(object_schema(
                        vec![
                            SchemaProperty::new("memory_id", JsonSchema::String),
                            SchemaProperty::new("score", JsonSchema::Number),
                        ],
                        vec!["memory_id"],
                        false,
                    )),
                },
            )],
            vec!["matches"],
            false,
        )),
        ToolSideEffects::Read,
        ToolIdempotency::Always,
        ToolConcurrency::Safe,
        ToolStreaming::None,
        PrivilegeTier::Tier0,
        vec!["memory:read"],
        None,
        vec!["memory", "store"],
    )
}

fn memory_list_by_class_definition() -> ToolDefinition {
    builtin_definition(
        "memory.list_by_class",
        "List memories by class",
        "List durable memory records by memory class.",
        object_schema(
            vec![
                SchemaProperty::new("class", memory_class_schema()),
                SchemaProperty::new("limit", JsonSchema::Integer),
            ],
            vec!["class"],
            false,
        ),
        Some(memory_matches_output_schema()),
        ToolSideEffects::Read,
        ToolIdempotency::Always,
        ToolConcurrency::Safe,
        ToolStreaming::None,
        PrivilegeTier::Tier0,
        vec!["memory:read"],
        None,
        vec!["memory", "store"],
    )
}

fn memory_list_by_tag_definition() -> ToolDefinition {
    builtin_definition(
        "memory.list_by_tag",
        "List memories by tag",
        "List durable memory records by tag.",
        object_schema(
            vec![
                SchemaProperty::new("tag", JsonSchema::String),
                SchemaProperty::new("limit", JsonSchema::Integer),
            ],
            vec!["tag"],
            false,
        ),
        Some(memory_matches_output_schema()),
        ToolSideEffects::Read,
        ToolIdempotency::Always,
        ToolConcurrency::Safe,
        ToolStreaming::None,
        PrivilegeTier::Tier0,
        vec!["memory:read"],
        None,
        vec!["memory", "store"],
    )
}

fn memory_supersede_definition() -> ToolDefinition {
    builtin_definition(
        "memory.supersede",
        "Supersede memory",
        "Record that one durable memory supersedes earlier memory records.",
        object_schema(
            vec![
                SchemaProperty::new("memory_id", JsonSchema::String),
                SchemaProperty::new("supersedes", string_array_schema()),
                SchemaProperty::new("reason", JsonSchema::String),
            ],
            vec!["memory_id", "supersedes"],
            false,
        ),
        Some(object_schema(
            vec![
                SchemaProperty::new("memory_id", JsonSchema::String),
                SchemaProperty::new("supersedes", string_array_schema()),
            ],
            vec!["memory_id", "supersedes"],
            false,
        )),
        ToolSideEffects::Write,
        ToolIdempotency::Always,
        ToolConcurrency::Serialized,
        ToolStreaming::Events,
        PrivilegeTier::Tier0,
        vec!["memory:write"],
        Some("memory"),
        vec!["memory", "store"],
    )
}

fn memory_expire_definition() -> ToolDefinition {
    builtin_definition(
        "memory.expire",
        "Expire memory",
        "Mark a durable memory record as expired.",
        object_schema(
            vec![
                SchemaProperty::new("memory_id", JsonSchema::String),
                SchemaProperty::new("expires_at_ms", JsonSchema::Integer),
                SchemaProperty::new("reason", JsonSchema::String),
            ],
            vec!["memory_id", "expires_at_ms"],
            false,
        ),
        Some(object_schema(
            vec![
                SchemaProperty::new("memory_id", JsonSchema::String),
                SchemaProperty::new("expired", JsonSchema::Boolean),
            ],
            vec!["memory_id", "expired"],
            false,
        )),
        ToolSideEffects::Write,
        ToolIdempotency::Always,
        ToolConcurrency::Serialized,
        ToolStreaming::Events,
        PrivilegeTier::Tier0,
        vec!["memory:expire"],
        Some("memory"),
        vec!["memory", "store"],
    )
}

fn memory_tombstone_definition() -> ToolDefinition {
    builtin_definition(
        "memory.tombstone",
        "Tombstone memory",
        "Forget a durable memory record by replacing it with a tombstone.",
        object_schema(
            vec![
                SchemaProperty::new("memory_id", JsonSchema::String),
                SchemaProperty::new("reason", JsonSchema::String),
            ],
            vec!["memory_id"],
            false,
        ),
        Some(object_schema(
            vec![
                SchemaProperty::new("memory_id", JsonSchema::String),
                SchemaProperty::new("tombstoned", JsonSchema::Boolean),
            ],
            vec!["memory_id", "tombstoned"],
            false,
        )),
        ToolSideEffects::Write,
        ToolIdempotency::Always,
        ToolConcurrency::Serialized,
        ToolStreaming::Events,
        PrivilegeTier::Tier1,
        vec!["memory:forget"],
        Some("memory"),
        vec!["memory", "store"],
    )
}

fn job_validate_definition() -> ToolDefinition {
    builtin_definition(
        "job.validate",
        "Validate job",
        "Validate a Chief of Staff portable job specification.",
        object_schema(
            vec![SchemaProperty::new("spec", JsonSchema::Any)],
            vec!["spec"],
            false,
        ),
        Some(object_schema(
            vec![
                SchemaProperty::new("valid", JsonSchema::Boolean),
                SchemaProperty::new("portable", JsonSchema::Boolean),
                SchemaProperty::new(
                    "issues",
                    JsonSchema::Array {
                        items: Box::new(JsonSchema::Any),
                    },
                ),
            ],
            vec!["valid", "portable", "issues"],
            false,
        )),
        ToolSideEffects::None,
        ToolIdempotency::Always,
        ToolConcurrency::Safe,
        ToolStreaming::None,
        PrivilegeTier::Tier0,
        vec!["jobs:validate"],
        None,
        vec!["job", "scheduler"],
    )
}

fn job_install_definition() -> ToolDefinition {
    builtin_definition(
        "job.install",
        "Install job",
        "Validate and install a portable Chief of Staff job.",
        object_schema(
            vec![
                SchemaProperty::new("spec", JsonSchema::Any),
                SchemaProperty::new("idempotency_key", JsonSchema::String),
            ],
            vec!["spec"],
            false,
        ),
        Some(object_schema(
            vec![
                SchemaProperty::new("job_id", JsonSchema::String),
                SchemaProperty::new("installed", JsonSchema::Boolean),
            ],
            vec!["job_id", "installed"],
            false,
        )),
        ToolSideEffects::Write,
        ToolIdempotency::Conditional,
        ToolConcurrency::Serialized,
        ToolStreaming::Events,
        PrivilegeTier::Tier1,
        vec!["jobs:install"],
        Some("job"),
        vec!["job", "scheduler"],
    )
}

fn job_uninstall_definition() -> ToolDefinition {
    builtin_definition(
        "job.uninstall",
        "Uninstall job",
        "Uninstall a portable Chief of Staff job.",
        object_schema(
            vec![SchemaProperty::new("job_id", JsonSchema::String)],
            vec!["job_id"],
            false,
        ),
        Some(object_schema(
            vec![
                SchemaProperty::new("job_id", JsonSchema::String),
                SchemaProperty::new("uninstalled", JsonSchema::Boolean),
            ],
            vec!["job_id", "uninstalled"],
            false,
        )),
        ToolSideEffects::Write,
        ToolIdempotency::Always,
        ToolConcurrency::Serialized,
        ToolStreaming::Events,
        PrivilegeTier::Tier1,
        vec!["jobs:uninstall"],
        Some("job"),
        vec!["job", "scheduler"],
    )
}

fn job_run_now_definition() -> ToolDefinition {
    builtin_definition(
        "job.run_now",
        "Run job now",
        "Request an immediate run for an installed Chief of Staff job.",
        object_schema(
            vec![
                SchemaProperty::new("job_id", JsonSchema::String),
                SchemaProperty::new("idempotency_key", JsonSchema::String),
            ],
            vec!["job_id"],
            false,
        ),
        Some(object_schema(
            vec![
                SchemaProperty::new("job_id", JsonSchema::String),
                SchemaProperty::new("run_id", JsonSchema::String),
                SchemaProperty::new("queued", JsonSchema::Boolean),
            ],
            vec!["job_id", "run_id", "queued"],
            false,
        )),
        ToolSideEffects::Write,
        ToolIdempotency::Conditional,
        ToolConcurrency::Serialized,
        ToolStreaming::Events,
        PrivilegeTier::Tier1,
        vec!["jobs:run"],
        Some("job"),
        vec!["job", "scheduler"],
    )
}

fn job_list_definition() -> ToolDefinition {
    builtin_definition(
        "job.list",
        "List jobs",
        "List installed Chief of Staff jobs.",
        object_schema(
            vec![
                SchemaProperty::new("status", JsonSchema::String),
                SchemaProperty::new("limit", JsonSchema::Integer),
            ],
            Vec::new(),
            false,
        ),
        Some(object_schema(
            vec![SchemaProperty::new(
                "jobs",
                JsonSchema::Array {
                    items: Box::new(JsonSchema::Any),
                },
            )],
            vec!["jobs"],
            false,
        )),
        ToolSideEffects::Read,
        ToolIdempotency::Always,
        ToolConcurrency::Safe,
        ToolStreaming::None,
        PrivilegeTier::Tier0,
        vec!["jobs:read"],
        None,
        vec!["job", "scheduler"],
    )
}

fn job_status_definition() -> ToolDefinition {
    builtin_definition(
        "job.status",
        "Read job status",
        "Read status and recent run metadata for an installed Chief of Staff job.",
        object_schema(
            vec![SchemaProperty::new("job_id", JsonSchema::String)],
            vec!["job_id"],
            false,
        ),
        Some(object_schema(
            vec![
                SchemaProperty::new("job_id", JsonSchema::String),
                SchemaProperty::new("status", JsonSchema::String),
                SchemaProperty::new("latest_run_id", JsonSchema::String),
            ],
            vec!["job_id", "status"],
            false,
        )),
        ToolSideEffects::Read,
        ToolIdempotency::Always,
        ToolConcurrency::Safe,
        ToolStreaming::None,
        PrivilegeTier::Tier0,
        vec!["jobs:read"],
        None,
        vec!["job", "scheduler"],
    )
}

#[allow(clippy::too_many_arguments)]
fn builtin_definition(
    tool_id: &str,
    display_name: &str,
    description: &str,
    input_schema: JsonSchema,
    output_schema: Option<JsonSchema>,
    side_effects: ToolSideEffects,
    idempotency: ToolIdempotency,
    concurrency: ToolConcurrency,
    streaming: ToolStreaming,
    required_tier: PrivilegeTier,
    required_capabilities: Vec<&str>,
    preferred_lock_scope: Option<&str>,
    tags: Vec<&str>,
) -> ToolDefinition {
    ToolDefinition {
        tool_id: tool_id.to_string(),
        display_name: display_name.to_string(),
        description: description.to_string(),
        input_schema,
        output_schema,
        side_effects,
        idempotency,
        concurrency,
        streaming,
        required_tier,
        required_capabilities: required_capabilities
            .into_iter()
            .map(str::to_string)
            .collect(),
        preferred_lock_scope: preferred_lock_scope.map(str::to_string),
        timeout_seconds: Some(30),
        tags: tags.into_iter().map(str::to_string).collect(),
        stability: ToolStability::Experimental,
    }
}

fn object_schema(
    properties: Vec<SchemaProperty>,
    required: Vec<&str>,
    allow_unknown_fields: bool,
) -> JsonSchema {
    JsonSchema::Object {
        properties,
        required: required.into_iter().map(str::to_string).collect(),
        allow_unknown_fields,
    }
}

fn string_array_schema() -> JsonSchema {
    JsonSchema::Array {
        items: Box::new(JsonSchema::String),
    }
}

fn string_enum(values: &[&str]) -> JsonSchema {
    JsonSchema::Enum {
        values: values
            .iter()
            .map(|value| JsonValue::String((*value).to_string()))
            .collect(),
    }
}

fn memory_class_schema() -> JsonSchema {
    string_enum(&["profile", "fact", "episodic", "procedure", "warning"])
}

fn memory_matches_output_schema() -> JsonSchema {
    object_schema(
        vec![SchemaProperty::new(
            "matches",
            JsonSchema::Array {
                items: Box::new(JsonSchema::Any),
            },
        )],
        vec!["matches"],
        false,
    )
}

fn skill_version_input_schema() -> JsonSchema {
    object_schema(
        vec![
            SchemaProperty::new("skill_id", JsonSchema::String),
            SchemaProperty::new("version", JsonSchema::String),
        ],
        vec!["skill_id", "version"],
        false,
    )
}

fn skill_activation_output_schema() -> JsonSchema {
    object_schema(
        vec![
            SchemaProperty::new("skill_id", JsonSchema::String),
            SchemaProperty::new("version", JsonSchema::String),
            SchemaProperty::new("active", JsonSchema::Boolean),
        ],
        vec!["skill_id", "version", "active"],
        false,
    )
}

// ============================================================================
// Invocation, events, and results
// ============================================================================

/// Origin category for a tool request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestedBy {
    User,
    Session,
    Job,
    Agent,
    System,
}

impl RequestedBy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Session => "session",
            Self::Job => "job",
            Self::Agent => "agent",
            Self::System => "system",
        }
    }
}

impl Display for RequestedBy {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One requested tool call.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolInvocationRequest {
    pub call_id: String,
    pub tool_id: ToolId,
    pub arguments: JsonValue,
    pub requested_by: RequestedBy,
    pub session_id: Option<String>,
    pub job_id: Option<String>,
    pub agent_id: Option<String>,
    pub user_id: Option<String>,
    pub requested_at: TimestampMs,
    pub deadline_at: Option<TimestampMs>,
    pub idempotency_key: Option<String>,
}

impl ToolInvocationRequest {
    /// Validate request metadata without resolving the tool definition.
    pub fn validate_metadata(&self) -> ToolValidationReport {
        let mut report = ToolValidationReport::ok(self.arguments.clone());
        validate_non_empty("call_id", &self.call_id, &mut report.errors);
        push_id_issue(
            &mut report.errors,
            "tool_id",
            &validate_tool_id(&self.tool_id),
        );
        validate_optional_id("session_id", &self.session_id, &mut report.errors);
        validate_optional_id("job_id", &self.job_id, &mut report.errors);
        validate_optional_id("agent_id", &self.agent_id, &mut report.errors);
        validate_optional_id("user_id", &self.user_id, &mut report.errors);
        validate_optional_id("idempotency_key", &self.idempotency_key, &mut report.errors);
        if self
            .deadline_at
            .is_some_and(|deadline| deadline < self.requested_at)
        {
            report.errors.push(issue(
                "deadline_at",
                "deadline cannot be before requested_at",
            ));
        }
        report.ok = report.errors.is_empty();
        report
    }
}

/// Streaming event kind for one call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolEventKind {
    Started,
    Progress,
    Output,
    Artifact,
    Memory,
    Warning,
    Completed,
    Failed,
    Cancelled,
}

impl ToolEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Progress => "progress",
            Self::Output => "output",
            Self::Artifact => "artifact",
            Self::Memory => "memory",
            Self::Warning => "warning",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

impl Display for ToolEventKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One event in the execution stream for a call.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolEvent {
    pub call_id: String,
    pub sequence: u64,
    pub at: TimestampMs,
    pub kind: ToolEventKind,
    pub payload: JsonValue,
}

impl ToolEvent {
    /// Return whether a terminal event is consistent with the final result.
    pub fn terminal_matches_result(&self, result: &ToolResult) -> bool {
        if self.call_id != result.call_id || !self.kind.is_terminal() {
            return false;
        }
        match self.kind {
            ToolEventKind::Completed => result.ok,
            ToolEventKind::Failed => !result.ok,
            ToolEventKind::Cancelled => {
                !result.ok
                    && result
                        .error
                        .as_ref()
                        .is_some_and(|error| error.kind == ToolErrorKind::ToolCancelled)
            }
            _ => false,
        }
    }
}

/// D18D error taxonomy for failed calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolErrorKind {
    ToolNotFound,
    ToolValidationError,
    ToolPermissionDenied,
    ToolTierDenied,
    ToolApprovalRequired,
    ToolApprovalDenied,
    ToolConflict,
    ToolTimeout,
    ToolCancelled,
    ToolExecutionError,
}

impl ToolErrorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ToolNotFound => "ToolNotFound",
            Self::ToolValidationError => "ToolValidationError",
            Self::ToolPermissionDenied => "ToolPermissionDenied",
            Self::ToolTierDenied => "ToolTierDenied",
            Self::ToolApprovalRequired => "ToolApprovalRequired",
            Self::ToolApprovalDenied => "ToolApprovalDenied",
            Self::ToolConflict => "ToolConflict",
            Self::ToolTimeout => "ToolTimeout",
            Self::ToolCancelled => "ToolCancelled",
            Self::ToolExecutionError => "ToolExecutionError",
        }
    }
}

impl Display for ToolErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Structured error returned by a failed tool call.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCallError {
    pub kind: ToolErrorKind,
    pub message: String,
    pub details: JsonValue,
}

impl ToolCallError {
    pub fn new(kind: ToolErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            details: JsonValue::Null,
        }
    }
}

/// Timing and byte metrics for one call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ToolMetrics {
    pub queued_ms: u64,
    pub run_ms: u64,
    pub validation_ms: u64,
    pub approval_ms: Option<u64>,
    pub bytes_in: Option<u64>,
    pub bytes_out: Option<u64>,
}

/// Final result for one tool call.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolResult {
    pub call_id: String,
    pub ok: bool,
    pub output: Option<JsonValue>,
    pub error: Option<ToolCallError>,
    pub artifact_refs: Vec<String>,
    pub memory_refs: Vec<String>,
    pub metrics: ToolMetrics,
}

impl ToolResult {
    pub fn completed(call_id: impl Into<String>, output: JsonValue) -> Self {
        Self {
            call_id: call_id.into(),
            ok: true,
            output: Some(output),
            error: None,
            artifact_refs: Vec::new(),
            memory_refs: Vec::new(),
            metrics: ToolMetrics::default(),
        }
    }

    pub fn failed(call_id: impl Into<String>, error: ToolCallError) -> Self {
        Self {
            call_id: call_id.into(),
            ok: false,
            output: None,
            error: Some(error),
            artifact_refs: Vec::new(),
            memory_refs: Vec::new(),
            metrics: ToolMetrics::default(),
        }
    }
}

// ============================================================================
// Read-side queries
// ============================================================================

/// Sort order for tool invocation request queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolInvocationSort {
    RequestedAtAsc,
    RequestedAtDesc,
    ToolIdThenRequestedAt,
    CallId,
}

impl Default for ToolInvocationSort {
    fn default() -> Self {
        Self::RequestedAtAsc
    }
}

/// Query options for selecting pending or persisted invocation requests.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolInvocationQuery {
    pub tool_id: Option<ToolId>,
    pub requested_by: Option<RequestedBy>,
    pub session_id: Option<String>,
    pub job_id: Option<String>,
    pub agent_id: Option<String>,
    pub user_id: Option<String>,
    pub requested_since: Option<TimestampMs>,
    pub requested_until: Option<TimestampMs>,
    pub deadline_before: Option<TimestampMs>,
    pub sort: ToolInvocationSort,
    pub limit: Option<usize>,
}

impl ToolInvocationQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_tool(mut self, tool_id: impl Into<String>) -> Self {
        self.tool_id = Some(tool_id.into());
        self
    }

    pub fn requested_by(mut self, requested_by: RequestedBy) -> Self {
        self.requested_by = Some(requested_by);
        self
    }

    pub fn in_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn for_job(mut self, job_id: impl Into<String>) -> Self {
        self.job_id = Some(job_id.into());
        self
    }

    pub fn for_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    pub fn for_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn requested_since(mut self, requested_since: TimestampMs) -> Self {
        self.requested_since = Some(requested_since);
        self
    }

    pub fn requested_until(mut self, requested_until: TimestampMs) -> Self {
        self.requested_until = Some(requested_until);
        self
    }

    pub fn deadline_before(mut self, deadline_before: TimestampMs) -> Self {
        self.deadline_before = Some(deadline_before);
        self
    }

    pub fn sorted_by(mut self, sort: ToolInvocationSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Query invocation requests without binding the contract to a storage backend.
pub fn query_tool_invocation_requests<'a, I>(
    requests: I,
    query: &ToolInvocationQuery,
) -> Vec<&'a ToolInvocationRequest>
where
    I: IntoIterator<Item = &'a ToolInvocationRequest>,
{
    if query.limit == Some(0) {
        return Vec::new();
    }

    let mut requests = requests
        .into_iter()
        .filter(|request| tool_invocation_matches_query(request, query))
        .collect::<Vec<_>>();
    sort_tool_invocation_requests(&mut requests, query.sort);
    apply_limit(&mut requests, query.limit);
    requests
}

/// Sort order for durable tool call record queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCallRecordSort {
    StartedAtAsc,
    StartedAtDesc,
    CompletedAtDesc,
    StatusThenToolId,
    CallId,
}

impl Default for ToolCallRecordSort {
    fn default() -> Self {
        Self::StartedAtAsc
    }
}

/// Query options for selecting durable tool call lifecycle records.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolCallRecordQuery {
    pub tool_id: Option<ToolId>,
    pub statuses: Vec<ToolCallStatus>,
    pub approval_states: Vec<ApprovalState>,
    pub lock_scope: Option<String>,
    pub started_since: Option<TimestampMs>,
    pub started_until: Option<TimestampMs>,
    pub completed_since: Option<TimestampMs>,
    pub completed_until: Option<TimestampMs>,
    pub active_only: bool,
    pub sort: ToolCallRecordSort,
    pub limit: Option<usize>,
}

impl ToolCallRecordQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_tool(mut self, tool_id: impl Into<String>) -> Self {
        self.tool_id = Some(tool_id.into());
        self
    }

    pub fn with_status(mut self, status: ToolCallStatus) -> Self {
        self.statuses.push(status);
        self
    }

    pub fn with_approval_state(mut self, approval_state: ApprovalState) -> Self {
        self.approval_states.push(approval_state);
        self
    }

    pub fn with_lock_scope(mut self, lock_scope: impl Into<String>) -> Self {
        self.lock_scope = Some(lock_scope.into());
        self
    }

    pub fn started_since(mut self, started_since: TimestampMs) -> Self {
        self.started_since = Some(started_since);
        self
    }

    pub fn started_until(mut self, started_until: TimestampMs) -> Self {
        self.started_until = Some(started_until);
        self
    }

    pub fn completed_since(mut self, completed_since: TimestampMs) -> Self {
        self.completed_since = Some(completed_since);
        self
    }

    pub fn completed_until(mut self, completed_until: TimestampMs) -> Self {
        self.completed_until = Some(completed_until);
        self
    }

    pub fn active_only(mut self) -> Self {
        self.active_only = true;
        self
    }

    pub fn sorted_by(mut self, sort: ToolCallRecordSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Query durable call records without binding the contract to a storage backend.
pub fn query_tool_call_records<'a, I>(
    records: I,
    query: &ToolCallRecordQuery,
) -> Vec<&'a ToolCallRecord>
where
    I: IntoIterator<Item = &'a ToolCallRecord>,
{
    if query.limit == Some(0) {
        return Vec::new();
    }

    let mut records = records
        .into_iter()
        .filter(|record| tool_call_record_matches_query(record, query))
        .collect::<Vec<_>>();
    sort_tool_call_records(&mut records, query.sort);
    apply_limit(&mut records, query.limit);
    records
}

/// Sort order for tool event stream queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolEventSort {
    TimeAsc,
    TimeDesc,
    SequenceAsc,
    SequenceDesc,
}

impl Default for ToolEventSort {
    fn default() -> Self {
        Self::TimeAsc
    }
}

/// Query options for selecting events from a tool execution stream.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolEventQuery {
    pub call_id: Option<String>,
    pub kinds: Vec<ToolEventKind>,
    pub terminal_only: bool,
    pub since: Option<TimestampMs>,
    pub until: Option<TimestampMs>,
    pub sequence_min: Option<u64>,
    pub sequence_max: Option<u64>,
    pub sort: ToolEventSort,
    pub limit: Option<usize>,
}

impl ToolEventQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_call(mut self, call_id: impl Into<String>) -> Self {
        self.call_id = Some(call_id.into());
        self
    }

    pub fn with_kind(mut self, kind: ToolEventKind) -> Self {
        self.kinds.push(kind);
        self
    }

    pub fn terminal_only(mut self) -> Self {
        self.terminal_only = true;
        self
    }

    pub fn since(mut self, since: TimestampMs) -> Self {
        self.since = Some(since);
        self
    }

    pub fn until(mut self, until: TimestampMs) -> Self {
        self.until = Some(until);
        self
    }

    pub fn sequence_min(mut self, sequence_min: u64) -> Self {
        self.sequence_min = Some(sequence_min);
        self
    }

    pub fn sequence_max(mut self, sequence_max: u64) -> Self {
        self.sequence_max = Some(sequence_max);
        self
    }

    pub fn sorted_by(mut self, sort: ToolEventSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Query tool events without binding the contract to an event-store backend.
pub fn query_tool_events<'a, I>(events: I, query: &ToolEventQuery) -> Vec<&'a ToolEvent>
where
    I: IntoIterator<Item = &'a ToolEvent>,
{
    if query.limit == Some(0) {
        return Vec::new();
    }

    let mut events = events
        .into_iter()
        .filter(|event| tool_event_matches_query(event, query))
        .collect::<Vec<_>>();
    sort_tool_events(&mut events, query.sort);
    apply_limit(&mut events, query.limit);
    events
}

/// Sort order for terminal tool result queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolResultSort {
    CallId,
    RunMsAsc,
    RunMsDesc,
}

impl Default for ToolResultSort {
    fn default() -> Self {
        Self::CallId
    }
}

/// Query options for selecting terminal tool results.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolResultQuery {
    pub call_id: Option<String>,
    pub ok: Option<bool>,
    pub error_kind: Option<ToolErrorKind>,
    pub has_artifact_refs: Option<bool>,
    pub has_memory_refs: Option<bool>,
    pub min_run_ms: Option<u64>,
    pub max_run_ms: Option<u64>,
    pub sort: ToolResultSort,
    pub limit: Option<usize>,
}

impl ToolResultQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_call(mut self, call_id: impl Into<String>) -> Self {
        self.call_id = Some(call_id.into());
        self
    }

    pub fn with_success(mut self, ok: bool) -> Self {
        self.ok = Some(ok);
        self
    }

    pub fn with_error_kind(mut self, error_kind: ToolErrorKind) -> Self {
        self.error_kind = Some(error_kind);
        self
    }

    pub fn with_artifact_refs(mut self, has_artifact_refs: bool) -> Self {
        self.has_artifact_refs = Some(has_artifact_refs);
        self
    }

    pub fn with_memory_refs(mut self, has_memory_refs: bool) -> Self {
        self.has_memory_refs = Some(has_memory_refs);
        self
    }

    pub fn min_run_ms(mut self, min_run_ms: u64) -> Self {
        self.min_run_ms = Some(min_run_ms);
        self
    }

    pub fn max_run_ms(mut self, max_run_ms: u64) -> Self {
        self.max_run_ms = Some(max_run_ms);
        self
    }

    pub fn sorted_by(mut self, sort: ToolResultSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Query terminal results without binding the contract to a result-store backend.
pub fn query_tool_results<'a, I>(results: I, query: &ToolResultQuery) -> Vec<&'a ToolResult>
where
    I: IntoIterator<Item = &'a ToolResult>,
{
    if query.limit == Some(0) {
        return Vec::new();
    }

    let mut results = results
        .into_iter()
        .filter(|result| tool_result_matches_query(result, query))
        .collect::<Vec<_>>();
    sort_tool_results(&mut results, query.sort);
    apply_limit(&mut results, query.limit);
    results
}

fn tool_invocation_matches_query(
    request: &ToolInvocationRequest,
    query: &ToolInvocationQuery,
) -> bool {
    query
        .tool_id
        .as_ref()
        .is_none_or(|tool_id| request.tool_id == *tool_id)
        && query
            .requested_by
            .is_none_or(|requested_by| request.requested_by == requested_by)
        && optional_string_matches(&request.session_id, &query.session_id)
        && optional_string_matches(&request.job_id, &query.job_id)
        && optional_string_matches(&request.agent_id, &query.agent_id)
        && optional_string_matches(&request.user_id, &query.user_id)
        && timestamp_matches_range(
            request.requested_at,
            query.requested_since,
            query.requested_until,
        )
        && query.deadline_before.is_none_or(|deadline_before| {
            request
                .deadline_at
                .is_some_and(|deadline_at| deadline_at <= deadline_before)
        })
}

fn sort_tool_invocation_requests(
    requests: &mut Vec<&ToolInvocationRequest>,
    sort: ToolInvocationSort,
) {
    match sort {
        ToolInvocationSort::RequestedAtAsc => requests.sort_by(|left, right| {
            left.requested_at
                .cmp(&right.requested_at)
                .then_with(|| left.call_id.cmp(&right.call_id))
        }),
        ToolInvocationSort::RequestedAtDesc => requests.sort_by(|left, right| {
            right
                .requested_at
                .cmp(&left.requested_at)
                .then_with(|| left.call_id.cmp(&right.call_id))
        }),
        ToolInvocationSort::ToolIdThenRequestedAt => requests.sort_by(|left, right| {
            left.tool_id
                .cmp(&right.tool_id)
                .then_with(|| left.requested_at.cmp(&right.requested_at))
                .then_with(|| left.call_id.cmp(&right.call_id))
        }),
        ToolInvocationSort::CallId => {
            requests.sort_by(|left, right| left.call_id.cmp(&right.call_id))
        }
    }
}

fn tool_call_record_matches_query(record: &ToolCallRecord, query: &ToolCallRecordQuery) -> bool {
    query
        .tool_id
        .as_ref()
        .is_none_or(|tool_id| record.tool_id == *tool_id)
        && (query.statuses.is_empty() || query.statuses.contains(&record.status))
        && (query.approval_states.is_empty()
            || query.approval_states.contains(&record.approval_state))
        && optional_string_matches(&record.lock_scope, &query.lock_scope)
        && optional_timestamp_matches_range(
            record.started_at,
            query.started_since,
            query.started_until,
        )
        && optional_timestamp_matches_range(
            record.completed_at,
            query.completed_since,
            query.completed_until,
        )
        && (!query.active_only || record.status.is_active())
}

fn sort_tool_call_records(records: &mut Vec<&ToolCallRecord>, sort: ToolCallRecordSort) {
    match sort {
        ToolCallRecordSort::StartedAtAsc => records.sort_by(|left, right| {
            left.started_at
                .unwrap_or(u64::MAX)
                .cmp(&right.started_at.unwrap_or(u64::MAX))
                .then_with(|| left.call_id.cmp(&right.call_id))
        }),
        ToolCallRecordSort::StartedAtDesc => records.sort_by(|left, right| {
            right
                .started_at
                .unwrap_or(0)
                .cmp(&left.started_at.unwrap_or(0))
                .then_with(|| left.call_id.cmp(&right.call_id))
        }),
        ToolCallRecordSort::CompletedAtDesc => records.sort_by(|left, right| {
            right
                .completed_at
                .unwrap_or(0)
                .cmp(&left.completed_at.unwrap_or(0))
                .then_with(|| left.call_id.cmp(&right.call_id))
        }),
        ToolCallRecordSort::StatusThenToolId => records.sort_by(|left, right| {
            left.status
                .as_str()
                .cmp(right.status.as_str())
                .then_with(|| left.tool_id.cmp(&right.tool_id))
                .then_with(|| left.call_id.cmp(&right.call_id))
        }),
        ToolCallRecordSort::CallId => {
            records.sort_by(|left, right| left.call_id.cmp(&right.call_id))
        }
    }
}

fn tool_event_matches_query(event: &ToolEvent, query: &ToolEventQuery) -> bool {
    query
        .call_id
        .as_ref()
        .is_none_or(|call_id| event.call_id == *call_id)
        && (query.kinds.is_empty() || query.kinds.contains(&event.kind))
        && (!query.terminal_only || event.kind.is_terminal())
        && timestamp_matches_range(event.at, query.since, query.until)
        && query
            .sequence_min
            .is_none_or(|sequence_min| event.sequence >= sequence_min)
        && query
            .sequence_max
            .is_none_or(|sequence_max| event.sequence <= sequence_max)
}

fn sort_tool_events(events: &mut Vec<&ToolEvent>, sort: ToolEventSort) {
    match sort {
        ToolEventSort::TimeAsc => events.sort_by(|left, right| {
            left.at
                .cmp(&right.at)
                .then_with(|| left.call_id.cmp(&right.call_id))
                .then_with(|| left.sequence.cmp(&right.sequence))
        }),
        ToolEventSort::TimeDesc => events.sort_by(|left, right| {
            right
                .at
                .cmp(&left.at)
                .then_with(|| left.call_id.cmp(&right.call_id))
                .then_with(|| left.sequence.cmp(&right.sequence))
        }),
        ToolEventSort::SequenceAsc => events.sort_by(|left, right| {
            left.call_id
                .cmp(&right.call_id)
                .then_with(|| left.sequence.cmp(&right.sequence))
        }),
        ToolEventSort::SequenceDesc => events.sort_by(|left, right| {
            left.call_id
                .cmp(&right.call_id)
                .then_with(|| right.sequence.cmp(&left.sequence))
        }),
    }
}

fn tool_result_matches_query(result: &ToolResult, query: &ToolResultQuery) -> bool {
    query
        .call_id
        .as_ref()
        .is_none_or(|call_id| result.call_id == *call_id)
        && query.ok.is_none_or(|ok| result.ok == ok)
        && query.error_kind.is_none_or(|error_kind| {
            result
                .error
                .as_ref()
                .is_some_and(|error| error.kind == error_kind)
        })
        && query
            .has_artifact_refs
            .is_none_or(|has_refs| result.artifact_refs.is_empty() != has_refs)
        && query
            .has_memory_refs
            .is_none_or(|has_refs| result.memory_refs.is_empty() != has_refs)
        && query
            .min_run_ms
            .is_none_or(|min_run_ms| result.metrics.run_ms >= min_run_ms)
        && query
            .max_run_ms
            .is_none_or(|max_run_ms| result.metrics.run_ms <= max_run_ms)
}

fn sort_tool_results(results: &mut Vec<&ToolResult>, sort: ToolResultSort) {
    match sort {
        ToolResultSort::CallId => results.sort_by(|left, right| left.call_id.cmp(&right.call_id)),
        ToolResultSort::RunMsAsc => results.sort_by(|left, right| {
            left.metrics
                .run_ms
                .cmp(&right.metrics.run_ms)
                .then_with(|| left.call_id.cmp(&right.call_id))
        }),
        ToolResultSort::RunMsDesc => results.sort_by(|left, right| {
            right
                .metrics
                .run_ms
                .cmp(&left.metrics.run_ms)
                .then_with(|| left.call_id.cmp(&right.call_id))
        }),
    }
}

fn optional_string_matches(candidate: &Option<String>, expected: &Option<String>) -> bool {
    expected
        .as_ref()
        .is_none_or(|expected| candidate.as_deref() == Some(expected.as_str()))
}

fn timestamp_matches_range(
    value: TimestampMs,
    since: Option<TimestampMs>,
    until: Option<TimestampMs>,
) -> bool {
    since.is_none_or(|since| value >= since) && until.is_none_or(|until| value <= until)
}

fn optional_timestamp_matches_range(
    value: Option<TimestampMs>,
    since: Option<TimestampMs>,
    until: Option<TimestampMs>,
) -> bool {
    if since.is_none() && until.is_none() {
        return true;
    }

    value.is_some_and(|value| timestamp_matches_range(value, since, until))
}

fn apply_limit<T>(items: &mut Vec<T>, limit: Option<usize>) {
    if let Some(limit) = limit {
        items.truncate(limit);
    }
}

// ============================================================================
// Registry
// ============================================================================

/// Deterministic in-memory registry for tool definitions.
#[derive(Debug, Clone, Default)]
pub struct InMemoryToolRegistry {
    definitions: BTreeMap<ToolId, ToolDefinition>,
}

impl InMemoryToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one definition.
    pub fn register(&mut self, definition: ToolDefinition) -> Result<(), ToolApiError> {
        let report = definition.validate();
        if !report.ok {
            return Err(ToolApiError::InvalidDefinition(report.errors));
        }
        if self.definitions.contains_key(&definition.tool_id) {
            return Err(ToolApiError::DuplicateToolId(definition.tool_id));
        }
        self.definitions
            .insert(definition.tool_id.clone(), definition);
        Ok(())
    }

    /// Fetch a definition by id.
    pub fn get(&self, tool_id: &str) -> Option<&ToolDefinition> {
        self.definitions.get(tool_id)
    }

    /// List definitions sorted by `tool_id`.
    pub fn list(&self) -> Vec<&ToolDefinition> {
        self.definitions.values().collect()
    }

    /// Query definitions sorted by `tool_id`.
    pub fn query(&self, query: &ToolCatalogQuery) -> Vec<&ToolDefinition> {
        if query.limit == Some(0) {
            return Vec::new();
        }

        let mut definitions = Vec::new();
        for definition in self.definitions.values() {
            if !definition_matches_catalog_query(definition, query) {
                continue;
            }
            definitions.push(definition);
            if let Some(limit) = query.limit {
                if definitions.len() >= limit {
                    break;
                }
            }
        }
        definitions
    }

    /// Export registered tool schemas, summary counts, and validation state.
    pub fn export(&self, query: &ToolCatalogQuery) -> ToolCatalogExport {
        ToolCatalogExport::from_definitions(self.query(query))
    }

    /// Export registered schema documents for a model gateway adapter.
    pub fn schema_documents(&self, query: &ToolCatalogQuery) -> Vec<ToolSchemaDocument> {
        self.export(query).schema_documents
    }

    /// Export schema-light registered catalog summaries.
    pub fn definition_summaries(&self, query: &ToolCatalogQuery) -> Vec<ToolDefinitionSummary> {
        self.query(query)
            .iter()
            .map(|definition| ToolDefinitionSummary::from_definition(definition))
            .collect()
    }

    /// Summarize all registered definitions.
    pub fn summary(&self) -> ToolCatalogSummary {
        ToolCatalogSummary::from_definitions(self.definitions.values())
    }

    /// Validate request metadata and arguments against the registered schema.
    pub fn validate_call(&self, request: &ToolInvocationRequest) -> ToolValidationReport {
        let mut report = request.validate_metadata();
        let Some(definition) = self.get(&request.tool_id) else {
            report.ok = false;
            report
                .errors
                .push(issue("tool_id", "tool is not registered"));
            return report;
        };

        let argument_report = definition.input_schema.validate_value(&request.arguments);
        report.errors.extend(argument_report.errors);
        report.warnings.extend(argument_report.warnings);
        report.ok = report.errors.is_empty();
        report.normalized_arguments = Some(request.arguments.clone());
        report
    }
}

// ============================================================================
// Runtime
// ============================================================================

/// Canonical call status used by runtime records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCallStatus {
    Queued,
    Validating,
    AwaitingApproval,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl ToolCallStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Validating => "validating",
            Self::AwaitingApproval => "awaiting_approval",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn is_active(self) -> bool {
        matches!(
            self,
            Self::Queued | Self::Validating | Self::AwaitingApproval | Self::Running
        )
    }
}

impl Display for ToolCallStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Approval state recorded for one invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalState {
    NotRequired,
    Pending,
    Granted,
    Denied,
    Expired,
}

impl ApprovalState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotRequired => "not_required",
            Self::Pending => "pending",
            Self::Granted => "granted",
            Self::Denied => "denied",
            Self::Expired => "expired",
        }
    }
}

impl Display for ApprovalState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Explicit approval token for replaying a previously blocked invocation.
///
/// The grant is intentionally scoped to one call id and one tool id. Policy
/// engines still decide whether approval is required; the runtime only accepts
/// this token for the `RequiresApproval` path and never lets it override
/// permission or tier denials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolApprovalGrant {
    pub call_id: String,
    pub tool_id: ToolId,
    pub granted_by: String,
    pub granted_at: TimestampMs,
    pub expires_at: Option<TimestampMs>,
}

impl ToolApprovalGrant {
    pub fn new(
        call_id: impl Into<String>,
        tool_id: impl Into<String>,
        granted_by: impl Into<String>,
        granted_at: TimestampMs,
    ) -> Self {
        Self {
            call_id: call_id.into(),
            tool_id: tool_id.into(),
            granted_by: granted_by.into(),
            granted_at,
            expires_at: None,
        }
    }

    pub fn with_expires_at(mut self, expires_at: TimestampMs) -> Self {
        self.expires_at = Some(expires_at);
        self
    }
}

/// Durable summary of one tool call lifecycle.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCallRecord {
    pub call_id: String,
    pub tool_id: ToolId,
    pub status: ToolCallStatus,
    pub started_at: Option<TimestampMs>,
    pub completed_at: Option<TimestampMs>,
    pub lock_scope: Option<String>,
    pub approval_state: ApprovalState,
    pub metrics: ToolMetrics,
}

impl ToolCallRecord {
    fn new(request: &ToolInvocationRequest, definition: Option<&ToolDefinition>) -> Self {
        Self {
            call_id: request.call_id.clone(),
            tool_id: request.tool_id.clone(),
            status: ToolCallStatus::Queued,
            started_at: None,
            completed_at: None,
            lock_scope: definition.and_then(|definition| definition.preferred_lock_scope.clone()),
            approval_state: ApprovalState::NotRequired,
            metrics: ToolMetrics::default(),
        }
    }
}

/// Cooperative cancellation handle passed into handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CancellationToken {
    cancelled: bool,
}

impl CancellationToken {
    pub fn active() -> Self {
        Self { cancelled: false }
    }

    pub fn cancelled() -> Self {
        Self { cancelled: true }
    }

    pub fn is_cancelled(self) -> bool {
        self.cancelled
    }
}

/// Explicit execution context passed to a handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolExecutionContext {
    pub call_id: String,
    pub tool_id: ToolId,
    pub requested_by: RequestedBy,
    pub session_id: Option<String>,
    pub job_id: Option<String>,
    pub agent_id: Option<String>,
    pub user_id: Option<String>,
    pub requested_at: TimestampMs,
    pub deadline_at: Option<TimestampMs>,
    pub cancellation_token: CancellationToken,
}

impl ToolExecutionContext {
    pub fn from_request(request: &ToolInvocationRequest) -> Self {
        Self {
            call_id: request.call_id.clone(),
            tool_id: request.tool_id.clone(),
            requested_by: request.requested_by,
            session_id: request.session_id.clone(),
            job_id: request.job_id.clone(),
            agent_id: request.agent_id.clone(),
            user_id: request.user_id.clone(),
            requested_at: request.requested_at,
            deadline_at: request.deadline_at,
            cancellation_token: CancellationToken::active(),
        }
    }
}

// ============================================================================
// Policy
// ============================================================================

/// Policy outcome for one validated tool call.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolPolicyOutcome {
    Allowed,
    Denied { error_kind: ToolErrorKind },
    RequiresApproval,
}

/// Decision made by the policy layer before handler execution.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolPolicyDecision {
    pub outcome: ToolPolicyOutcome,
    pub message: String,
    pub details: JsonValue,
}

impl ToolPolicyDecision {
    pub fn allow() -> Self {
        Self {
            outcome: ToolPolicyOutcome::Allowed,
            message: "allowed".to_string(),
            details: JsonValue::Null,
        }
    }

    pub fn deny(error_kind: ToolErrorKind, message: impl Into<String>) -> Self {
        Self {
            outcome: ToolPolicyOutcome::Denied { error_kind },
            message: message.into(),
            details: JsonValue::Null,
        }
    }

    pub fn require_approval(message: impl Into<String>) -> Self {
        Self {
            outcome: ToolPolicyOutcome::RequiresApproval,
            message: message.into(),
            details: JsonValue::Null,
        }
    }

    pub fn with_details(mut self, details: JsonValue) -> Self {
        self.details = details;
        self
    }

    pub fn is_allowed(&self) -> bool {
        matches!(self.outcome, ToolPolicyOutcome::Allowed)
    }
}

/// Object-safe policy hook used by tool runtimes.
pub trait ToolPolicyEngine {
    fn decide(
        &self,
        definition: &ToolDefinition,
        request: &ToolInvocationRequest,
    ) -> ToolPolicyDecision;
}

impl<F> ToolPolicyEngine for F
where
    F: Fn(&ToolDefinition, &ToolInvocationRequest) -> ToolPolicyDecision,
{
    fn decide(
        &self,
        definition: &ToolDefinition,
        request: &ToolInvocationRequest,
    ) -> ToolPolicyDecision {
        self(definition, request)
    }
}

/// Default policy for local tests and runtimes that enforce policy elsewhere.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AllowAllToolPolicy;

impl ToolPolicyEngine for AllowAllToolPolicy {
    fn decide(
        &self,
        _definition: &ToolDefinition,
        _request: &ToolInvocationRequest,
    ) -> ToolPolicyDecision {
        ToolPolicyDecision::allow()
    }
}

/// Small deterministic policy profile for repository runtimes and tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolPolicyProfile {
    pub max_tier: PrivilegeTier,
    pub allowed_capabilities: Vec<String>,
    pub allowed_side_effects: Vec<ToolSideEffects>,
    pub approval_required_side_effects: Vec<ToolSideEffects>,
}

impl ToolPolicyProfile {
    pub fn allow_all() -> Self {
        Self {
            max_tier: PrivilegeTier::Tier3,
            allowed_capabilities: vec!["*".to_string()],
            allowed_side_effects: vec![
                ToolSideEffects::None,
                ToolSideEffects::Read,
                ToolSideEffects::Write,
                ToolSideEffects::External,
            ],
            approval_required_side_effects: Vec::new(),
        }
    }

    pub fn read_only(max_tier: PrivilegeTier) -> Self {
        Self {
            max_tier,
            allowed_capabilities: vec!["*".to_string()],
            allowed_side_effects: vec![ToolSideEffects::None, ToolSideEffects::Read],
            approval_required_side_effects: Vec::new(),
        }
    }

    pub fn with_allowed_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.allowed_capabilities = capabilities;
        self
    }

    pub fn with_approval_required_for(mut self, side_effects: Vec<ToolSideEffects>) -> Self {
        self.approval_required_side_effects = side_effects;
        self
    }

    fn capability_allowed(&self, capability: &str) -> bool {
        self.allowed_capabilities
            .iter()
            .any(|allowed| allowed == "*" || allowed == capability)
    }
}

impl ToolPolicyEngine for ToolPolicyProfile {
    fn decide(
        &self,
        definition: &ToolDefinition,
        _request: &ToolInvocationRequest,
    ) -> ToolPolicyDecision {
        if definition.required_tier > self.max_tier {
            return ToolPolicyDecision::deny(
                ToolErrorKind::ToolTierDenied,
                format!(
                    "tool requires {}, but policy allows up to {}",
                    definition.required_tier, self.max_tier
                ),
            );
        }

        if let Some(capability) = definition
            .required_capabilities
            .iter()
            .find(|capability| !self.capability_allowed(capability))
        {
            return ToolPolicyDecision::deny(
                ToolErrorKind::ToolPermissionDenied,
                format!("required capability '{capability}' is not allowed by policy"),
            );
        }

        if !self.allowed_side_effects.contains(&definition.side_effects) {
            return ToolPolicyDecision::deny(
                ToolErrorKind::ToolPermissionDenied,
                format!(
                    "tool side effect '{}' is not allowed by policy",
                    definition.side_effects
                ),
            );
        }

        if self
            .approval_required_side_effects
            .contains(&definition.side_effects)
        {
            return ToolPolicyDecision::require_approval(format!(
                "tool side effect '{}' requires approval",
                definition.side_effects
            ));
        }

        ToolPolicyDecision::allow()
    }
}

/// One non-terminal event requested by a handler.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolHandlerEvent {
    pub kind: ToolEventKind,
    pub payload: JsonValue,
}

/// Domain output returned by a handler before runtime wrapping.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolHandlerOutput {
    pub output: JsonValue,
    pub artifact_refs: Vec<String>,
    pub memory_refs: Vec<String>,
    pub events: Vec<ToolHandlerEvent>,
}

impl ToolHandlerOutput {
    pub fn new(output: JsonValue) -> Self {
        Self {
            output,
            artifact_refs: Vec::new(),
            memory_refs: Vec::new(),
            events: Vec::new(),
        }
    }

    pub fn with_artifact_ref(mut self, artifact_ref: impl Into<String>) -> Self {
        self.artifact_refs.push(artifact_ref.into());
        self
    }

    pub fn with_memory_ref(mut self, memory_ref: impl Into<String>) -> Self {
        self.memory_refs.push(memory_ref.into());
        self
    }

    pub fn with_event(mut self, kind: ToolEventKind, payload: JsonValue) -> Self {
        self.events.push(ToolHandlerEvent { kind, payload });
        self
    }
}

/// Handler contract for in-process tool runtimes.
pub trait ToolHandler {
    fn invoke(
        &self,
        arguments: JsonValue,
        context: ToolExecutionContext,
    ) -> Result<ToolHandlerOutput, ToolCallError>;
}

impl<F> ToolHandler for F
where
    F: Fn(JsonValue, ToolExecutionContext) -> Result<ToolHandlerOutput, ToolCallError>,
{
    fn invoke(
        &self,
        arguments: JsonValue,
        context: ToolExecutionContext,
    ) -> Result<ToolHandlerOutput, ToolCallError> {
        self(arguments, context)
    }
}

/// Result plus event stream and call record for one invocation.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolExecutionTrace {
    pub record: ToolCallRecord,
    pub events: Vec<ToolEvent>,
    pub result: ToolResult,
}

/// Deterministic in-memory runtime for tests, built-ins, and small hosts.
pub struct InMemoryToolRuntime {
    registry: InMemoryToolRegistry,
    handlers: BTreeMap<ToolId, Box<dyn ToolHandler>>,
    policy: Box<dyn ToolPolicyEngine>,
}

impl Default for InMemoryToolRuntime {
    fn default() -> Self {
        Self {
            registry: InMemoryToolRegistry::new(),
            handlers: BTreeMap::new(),
            policy: Box::new(AllowAllToolPolicy),
        }
    }
}

impl InMemoryToolRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy<P>(policy: P) -> Self
    where
        P: ToolPolicyEngine + 'static,
    {
        Self {
            registry: InMemoryToolRegistry::new(),
            handlers: BTreeMap::new(),
            policy: Box::new(policy),
        }
    }

    pub fn set_policy<P>(&mut self, policy: P)
    where
        P: ToolPolicyEngine + 'static,
    {
        self.policy = Box::new(policy);
    }

    /// Register one definition and its in-process handler.
    pub fn register_handler<H>(
        &mut self,
        definition: ToolDefinition,
        handler: H,
    ) -> Result<(), ToolApiError>
    where
        H: ToolHandler + 'static,
    {
        let tool_id = definition.tool_id.clone();
        self.registry.register(definition)?;
        self.handlers.insert(tool_id, Box::new(handler));
        Ok(())
    }

    /// Fetch a registered definition.
    pub fn get(&self, tool_id: &str) -> Option<&ToolDefinition> {
        self.registry.get(tool_id)
    }

    /// List registered definitions sorted by tool id.
    pub fn list(&self) -> Vec<&ToolDefinition> {
        self.registry.list()
    }

    /// Query registered definitions sorted by tool id.
    pub fn query(&self, query: &ToolCatalogQuery) -> Vec<&ToolDefinition> {
        self.registry.query(query)
    }

    /// Validate request metadata and arguments without invoking the handler.
    pub fn validate(&self, request: &ToolInvocationRequest) -> ToolValidationReport {
        self.registry.validate_call(request)
    }

    /// Invoke a tool and return only the terminal result.
    pub fn invoke(&self, request: &ToolInvocationRequest) -> ToolResult {
        self.invoke_with_events(request).result
    }

    /// Invoke a tool with an explicit approval grant and return only the result.
    pub fn invoke_with_approval(
        &self,
        request: &ToolInvocationRequest,
        approval_grant: &ToolApprovalGrant,
    ) -> ToolResult {
        self.invoke_with_events_with_approval(request, approval_grant)
            .result
    }

    /// Invoke a tool and return the canonical event stream plus call record.
    pub fn invoke_with_events(&self, request: &ToolInvocationRequest) -> ToolExecutionTrace {
        self.invoke_with_events_inner(request, None)
    }

    /// Invoke a tool with an explicit approval grant and return the full trace.
    pub fn invoke_with_events_with_approval(
        &self,
        request: &ToolInvocationRequest,
        approval_grant: &ToolApprovalGrant,
    ) -> ToolExecutionTrace {
        self.invoke_with_events_inner(request, Some(approval_grant))
    }

    fn invoke_with_events_inner(
        &self,
        request: &ToolInvocationRequest,
        approval_grant: Option<&ToolApprovalGrant>,
    ) -> ToolExecutionTrace {
        let definition = self.registry.get(&request.tool_id);
        let mut record = ToolCallRecord::new(request, definition);
        record.status = ToolCallStatus::Validating;

        let validation = self.registry.validate_call(request);
        if !validation.ok {
            let result = ToolResult::failed(
                request.call_id.clone(),
                ToolCallError {
                    kind: ToolErrorKind::ToolValidationError,
                    message: "tool invocation failed validation".to_string(),
                    details: validation_errors_to_json(&validation.errors),
                },
            );
            record.status = ToolCallStatus::Failed;
            record.completed_at = Some(request.requested_at);
            return trace_with_terminal(record, request, Vec::new(), result);
        }

        let Some(definition) = self.registry.get(&request.tool_id) else {
            let result = ToolResult::failed(
                request.call_id.clone(),
                ToolCallError::new(ToolErrorKind::ToolNotFound, "tool is not registered"),
            );
            record.status = ToolCallStatus::Failed;
            record.completed_at = Some(request.requested_at);
            return trace_with_terminal(record, request, Vec::new(), result);
        };

        let policy_decision = self.policy.decide(definition, request);
        match policy_decision.outcome {
            ToolPolicyOutcome::Allowed => {
                record.approval_state = ApprovalState::NotRequired;
            }
            ToolPolicyOutcome::Denied { error_kind } => {
                let result = ToolResult::failed(
                    request.call_id.clone(),
                    ToolCallError {
                        kind: error_kind,
                        message: policy_decision.message,
                        details: policy_decision.details,
                    },
                );
                record.status = ToolCallStatus::Failed;
                record.approval_state = ApprovalState::Denied;
                record.completed_at = Some(request.requested_at);
                return trace_with_terminal(record, request, Vec::new(), result);
            }
            ToolPolicyOutcome::RequiresApproval => {
                if let Some(approval_grant) = approval_grant {
                    match validate_approval_grant(approval_grant, request) {
                        Ok(()) => {
                            record.approval_state = ApprovalState::Granted;
                        }
                        Err((approval_state, error)) => {
                            let result = ToolResult::failed(request.call_id.clone(), error);
                            record.status = ToolCallStatus::Failed;
                            record.approval_state = approval_state;
                            record.completed_at = Some(request.requested_at);
                            return trace_with_terminal(record, request, Vec::new(), result);
                        }
                    }
                } else {
                    let result = ToolResult::failed(
                        request.call_id.clone(),
                        ToolCallError {
                            kind: ToolErrorKind::ToolApprovalRequired,
                            message: policy_decision.message,
                            details: policy_decision.details,
                        },
                    );
                    record.status = ToolCallStatus::AwaitingApproval;
                    record.approval_state = ApprovalState::Pending;
                    record.completed_at = Some(request.requested_at);
                    return trace_with_terminal(record, request, Vec::new(), result);
                }
            }
        }

        let Some(handler) = self.handlers.get(&request.tool_id) else {
            let result = ToolResult::failed(
                request.call_id.clone(),
                ToolCallError::new(
                    ToolErrorKind::ToolNotFound,
                    "tool handler is not registered",
                ),
            );
            record.status = ToolCallStatus::Failed;
            record.completed_at = Some(request.requested_at);
            return trace_with_terminal(record, request, Vec::new(), result);
        };

        record.status = ToolCallStatus::Running;
        record.started_at = Some(request.requested_at);
        let context = ToolExecutionContext::from_request(request);
        if context.cancellation_token.is_cancelled() {
            let result = ToolResult::failed(
                request.call_id.clone(),
                ToolCallError::new(ToolErrorKind::ToolCancelled, "tool call was cancelled"),
            );
            record.status = ToolCallStatus::Cancelled;
            record.completed_at = Some(request.requested_at);
            return trace_with_terminal(record, request, started_event(request), result);
        }

        match handler.invoke(request.arguments.clone(), context) {
            Ok(output) if output.events.iter().any(|event| event.kind.is_terminal()) => {
                let result = ToolResult::failed(
                    request.call_id.clone(),
                    ToolCallError::new(
                        ToolErrorKind::ToolExecutionError,
                        "handlers must not emit terminal events directly",
                    ),
                );
                record.status = ToolCallStatus::Failed;
                record.completed_at = Some(request.requested_at);
                trace_with_terminal(record, request, started_event(request), result)
            }
            Ok(output) => {
                let mut events = started_event(request);
                events.extend(handler_events(request, output.events, events.len() as u64));

                if let Some(output_schema) = &definition.output_schema {
                    let output_validation = output_schema.validate_value(&output.output);
                    if !output_validation.ok {
                        let result = ToolResult::failed(
                            request.call_id.clone(),
                            ToolCallError {
                                kind: ToolErrorKind::ToolValidationError,
                                message: "tool handler output failed validation".to_string(),
                                details: validation_errors_to_json(&output_validation.errors),
                            },
                        );
                        record.status = ToolCallStatus::Failed;
                        record.completed_at = Some(request.requested_at);
                        return trace_with_terminal(record, request, events, result);
                    }
                }

                let mut result = ToolResult::completed(request.call_id.clone(), output.output);
                result.artifact_refs = output.artifact_refs;
                result.memory_refs = output.memory_refs;
                record.status = ToolCallStatus::Completed;
                record.completed_at = Some(request.requested_at);
                trace_with_terminal(record, request, events, result)
            }
            Err(error) => {
                let status = if error.kind == ToolErrorKind::ToolCancelled {
                    ToolCallStatus::Cancelled
                } else {
                    ToolCallStatus::Failed
                };
                let result = ToolResult::failed(request.call_id.clone(), error);
                record.status = status;
                record.completed_at = Some(request.requested_at);
                trace_with_terminal(record, request, started_event(request), result)
            }
        }
    }
}

fn validate_approval_grant(
    grant: &ToolApprovalGrant,
    request: &ToolInvocationRequest,
) -> Result<(), (ApprovalState, ToolCallError)> {
    if grant.call_id != request.call_id {
        return Err((
            ApprovalState::Denied,
            approval_error(
                "approval grant call_id does not match invocation",
                grant_details(grant),
            ),
        ));
    }
    if grant.tool_id != request.tool_id {
        return Err((
            ApprovalState::Denied,
            approval_error(
                "approval grant tool_id does not match invocation",
                grant_details(grant),
            ),
        ));
    }
    if grant.granted_by.trim().is_empty() {
        return Err((
            ApprovalState::Denied,
            approval_error("approval grant is missing granted_by", grant_details(grant)),
        ));
    }
    if grant
        .expires_at
        .is_some_and(|expires_at| expires_at < request.requested_at)
    {
        return Err((
            ApprovalState::Expired,
            approval_error("approval grant has expired", grant_details(grant)),
        ));
    }

    Ok(())
}

fn approval_error(message: impl Into<String>, details: JsonValue) -> ToolCallError {
    ToolCallError {
        kind: ToolErrorKind::ToolApprovalDenied,
        message: message.into(),
        details,
    }
}

fn grant_details(grant: &ToolApprovalGrant) -> JsonValue {
    let mut fields = vec![
        (
            "call_id".to_string(),
            JsonValue::String(grant.call_id.clone()),
        ),
        (
            "tool_id".to_string(),
            JsonValue::String(grant.tool_id.clone()),
        ),
        (
            "granted_by".to_string(),
            JsonValue::String(grant.granted_by.clone()),
        ),
        (
            "granted_at".to_string(),
            JsonValue::Number(JsonNumber::Integer(grant.granted_at as i64)),
        ),
    ];
    if let Some(expires_at) = grant.expires_at {
        fields.push((
            "expires_at".to_string(),
            JsonValue::Number(JsonNumber::Integer(expires_at as i64)),
        ));
    }
    JsonValue::Object(fields)
}

fn started_event(request: &ToolInvocationRequest) -> Vec<ToolEvent> {
    vec![ToolEvent {
        call_id: request.call_id.clone(),
        sequence: 0,
        at: request.requested_at,
        kind: ToolEventKind::Started,
        payload: JsonValue::Null,
    }]
}

fn handler_events(
    request: &ToolInvocationRequest,
    events: Vec<ToolHandlerEvent>,
    sequence_start: u64,
) -> Vec<ToolEvent> {
    events
        .into_iter()
        .enumerate()
        .map(|(index, event)| ToolEvent {
            call_id: request.call_id.clone(),
            sequence: sequence_start + index as u64,
            at: request.requested_at,
            kind: event.kind,
            payload: event.payload,
        })
        .collect()
}

fn trace_with_terminal(
    mut record: ToolCallRecord,
    request: &ToolInvocationRequest,
    mut events: Vec<ToolEvent>,
    result: ToolResult,
) -> ToolExecutionTrace {
    record.metrics = result.metrics;
    events.push(ToolEvent {
        call_id: request.call_id.clone(),
        sequence: events.len() as u64,
        at: request.requested_at,
        kind: terminal_kind(&result),
        payload: terminal_payload(&result),
    });
    ToolExecutionTrace {
        record,
        events,
        result,
    }
}

fn terminal_kind(result: &ToolResult) -> ToolEventKind {
    if result.ok {
        ToolEventKind::Completed
    } else if result
        .error
        .as_ref()
        .is_some_and(|error| error.kind == ToolErrorKind::ToolCancelled)
    {
        ToolEventKind::Cancelled
    } else {
        ToolEventKind::Failed
    }
}

fn terminal_payload(result: &ToolResult) -> JsonValue {
    if let Some(output) = &result.output {
        output.clone()
    } else if let Some(error) = &result.error {
        JsonValue::Object(vec![
            (
                "kind".to_string(),
                JsonValue::String(error.kind.as_str().to_string()),
            ),
            (
                "message".to_string(),
                JsonValue::String(error.message.clone()),
            ),
            ("details".to_string(), error.details.clone()),
        ])
    } else {
        JsonValue::Null
    }
}

fn validation_errors_to_json(errors: &[ToolValidationIssue]) -> JsonValue {
    JsonValue::Array(
        errors
            .iter()
            .map(|error| {
                JsonValue::Object(vec![
                    ("path".to_string(), JsonValue::String(error.path.clone())),
                    (
                        "message".to_string(),
                        JsonValue::String(error.message.clone()),
                    ),
                ])
            })
            .collect(),
    )
}

// ============================================================================
// Validation reports and crate errors
// ============================================================================

/// One validation issue with a stable path and human-readable message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolValidationIssue {
    pub path: String,
    pub message: String,
}

/// Result of validating a definition or invocation.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolValidationReport {
    pub ok: bool,
    pub normalized_arguments: Option<JsonValue>,
    pub errors: Vec<ToolValidationIssue>,
    pub warnings: Vec<ToolValidationIssue>,
}

impl ToolValidationReport {
    pub fn empty() -> Self {
        Self {
            ok: true,
            normalized_arguments: None,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn ok(normalized_arguments: JsonValue) -> Self {
        Self {
            ok: true,
            normalized_arguments: Some(normalized_arguments),
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

/// Errors raised by registry-level operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolApiError {
    InvalidToolId(String),
    DuplicateToolId(String),
    UnknownTool(String),
    InvalidDefinition(Vec<ToolValidationIssue>),
}

impl Display for ToolApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidToolId(tool_id) => write!(f, "invalid tool id '{tool_id}'"),
            Self::DuplicateToolId(tool_id) => write!(f, "duplicate tool id '{tool_id}'"),
            Self::UnknownTool(tool_id) => write!(f, "unknown tool '{tool_id}'"),
            Self::InvalidDefinition(issues) => {
                write!(f, "invalid tool definition with {} issue(s)", issues.len())
            }
        }
    }
}

impl Error for ToolApiError {}

/// Validate a D18D dotted tool identifier.
pub fn validate_tool_id(tool_id: &str) -> Result<(), ToolApiError> {
    if tool_id.is_empty() {
        return Err(ToolApiError::InvalidToolId(tool_id.to_string()));
    }

    for segment in tool_id.split('.') {
        if segment.is_empty() {
            return Err(ToolApiError::InvalidToolId(tool_id.to_string()));
        }
        let mut chars = segment.chars();
        let Some(first) = chars.next() else {
            return Err(ToolApiError::InvalidToolId(tool_id.to_string()));
        };
        if !first.is_ascii_lowercase() {
            return Err(ToolApiError::InvalidToolId(tool_id.to_string()));
        }
        if chars.any(|ch| !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')) {
            return Err(ToolApiError::InvalidToolId(tool_id.to_string()));
        }
    }

    Ok(())
}

fn validate_non_empty(path: impl Into<String>, value: &str, errors: &mut Vec<ToolValidationIssue>) {
    if value.trim().is_empty() {
        errors.push(issue(path, "value cannot be empty"));
    }
}

fn validate_optional_id(
    path: &'static str,
    value: &Option<String>,
    errors: &mut Vec<ToolValidationIssue>,
) {
    if let Some(value) = value {
        validate_non_empty(path, value, errors);
    }
}

fn validate_labels(path: &str, values: &[String], errors: &mut Vec<ToolValidationIssue>) {
    for (index, value) in values.iter().enumerate() {
        validate_non_empty(format!("{path}[{index}]"), value, errors);
        if value.contains(char::is_whitespace) {
            errors.push(issue(
                format!("{path}[{index}]"),
                "label cannot contain whitespace",
            ));
        }
    }
}

fn validate_schema_key(path: &str, value: &str, errors: &mut Vec<ToolValidationIssue>) {
    if value.is_empty() {
        errors.push(issue(path, "schema key cannot be empty"));
    }
    if value.contains('.') {
        errors.push(issue(path, "schema key cannot contain dots"));
    }
}

fn push_id_issue(
    errors: &mut Vec<ToolValidationIssue>,
    path: impl Into<String>,
    result: &Result<(), ToolApiError>,
) {
    if result.is_err() {
        errors.push(issue(path, "invalid dotted tool id"));
    }
}

fn issue(path: impl Into<String>, message: impl Into<String>) -> ToolValidationIssue {
    ToolValidationIssue {
        path: path.into(),
        message: message.into(),
    }
}

fn type_issue(path: &str, expected: &str, value: &JsonValue) -> ToolValidationIssue {
    issue(
        path,
        format!("expected {expected}, got {}", json_type_name(value)),
    )
}

fn json_type_name(value: &JsonValue) -> &'static str {
    match value {
        JsonValue::Object(_) => "object",
        JsonValue::Array(_) => "array",
        JsonValue::String(_) => "string",
        JsonValue::Number(JsonNumber::Integer(_)) => "integer",
        JsonValue::Number(JsonNumber::Float(_)) => "number",
        JsonValue::Bool(_) => "boolean",
        JsonValue::Null => "null",
    }
}

fn json_schema_type(type_name: &str) -> JsonValue {
    JsonValue::Object(vec![(
        "type".to_string(),
        JsonValue::String(type_name.to_string()),
    )])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json_object_lookup<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
        match value {
            JsonValue::Object(fields) => fields
                .iter()
                .find(|(candidate, _)| candidate == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    fn json_string(value: &str) -> JsonValue {
        JsonValue::String(value.to_string())
    }

    #[test]
    fn dotted_tool_ids_are_lowercase_and_segmented() {
        assert!(validate_tool_id("context.open_session").is_ok());
        assert!(validate_tool_id("artifact.create_v2").is_ok());
        assert!(validate_tool_id("Context.open_session").is_err());
        assert!(validate_tool_id("context..open").is_err());
        assert!(validate_tool_id("context/open").is_err());
        assert!(validate_tool_id("9context.open").is_err());
    }

    #[test]
    fn schema_validates_required_fields_and_unknowns() {
        let schema = artifact_create_schema();
        let valid = JsonValue::Object(vec![
            (
                "collection".to_string(),
                JsonValue::String("session-artifacts".to_string()),
            ),
            (
                "name".to_string(),
                JsonValue::String("brief.md".to_string()),
            ),
            (
                "body_base64".to_string(),
                JsonValue::String("SGVsbG8=".to_string()),
            ),
        ]);

        assert!(schema.validate_value(&valid).ok);

        let invalid = JsonValue::Object(vec![
            ("collection".to_string(), JsonValue::String("x".to_string())),
            (
                "surprise".to_string(),
                JsonValue::String("not allowed".to_string()),
            ),
        ]);
        let report = schema.validate_value(&invalid);
        assert!(!report.ok);
        assert!(report
            .errors
            .iter()
            .any(|error| error.path == "$.name" && error.message == "required field is missing"));
        assert!(report
            .errors
            .iter()
            .any(|error| error.path == "$.surprise"
                && error.message == "unknown field is not allowed"));
    }

    #[test]
    fn json_schema_projection_renders_strict_object_schema() {
        let schema = object_schema(
            vec![
                SchemaProperty::new("session_id", JsonSchema::String),
                SchemaProperty::new(
                    "limit",
                    JsonSchema::Enum {
                        values: vec![
                            JsonValue::Number(JsonNumber::Integer(10)),
                            JsonValue::Number(JsonNumber::Integer(25)),
                        ],
                    },
                ),
                SchemaProperty::new(
                    "tags",
                    JsonSchema::Array {
                        items: Box::new(JsonSchema::String),
                    },
                ),
            ],
            vec!["session_id"],
            false,
        );

        let projected = schema.to_json_schema_value();

        assert_eq!(
            json_object_lookup(&projected, "type"),
            Some(&json_string("object"))
        );
        assert_eq!(
            json_object_lookup(&projected, "additionalProperties"),
            Some(&JsonValue::Bool(false))
        );
        assert_eq!(
            json_object_lookup(&projected, "required"),
            Some(&JsonValue::Array(vec![json_string("session_id")]))
        );

        let properties = json_object_lookup(&projected, "properties")
            .expect("properties object should be present");
        let session_id = json_object_lookup(properties, "session_id")
            .expect("session_id schema should be present");
        let limit =
            json_object_lookup(properties, "limit").expect("limit schema should be present");
        let tags = json_object_lookup(properties, "tags").expect("tags schema should be present");

        assert_eq!(
            json_object_lookup(session_id, "type"),
            Some(&json_string("string"))
        );
        assert_eq!(
            json_object_lookup(limit, "enum"),
            Some(&JsonValue::Array(vec![
                JsonValue::Number(JsonNumber::Integer(10)),
                JsonValue::Number(JsonNumber::Integer(25)),
            ]))
        );
        assert_eq!(
            json_object_lookup(tags, "type"),
            Some(&json_string("array"))
        );
        assert_eq!(
            json_object_lookup(json_object_lookup(tags, "items").unwrap(), "type"),
            Some(&json_string("string"))
        );
    }

    #[test]
    fn tool_schema_document_exports_builtin_input_and_output_schemas() {
        let definition =
            builtin_tool_definition("context.read_entries").expect("builtin should exist");

        let document = definition.schema_document();

        assert_eq!(document.tool_id, "context.read_entries");
        assert_eq!(document.display_name, "Read context entries");
        assert!(document.description.contains("durable context session"));
        assert_eq!(
            json_object_lookup(&document.input_schema, "type"),
            Some(&json_string("object"))
        );
        assert!(json_object_lookup(&document.input_schema, "properties").is_some());
        assert!(document.output_schema.is_some());
        assert_eq!(definition.input_json_schema(), document.input_schema);
        assert_eq!(definition.output_json_schema(), document.output_schema);
    }

    #[test]
    fn definition_validation_catches_metadata_and_schema_errors() {
        let definition = ToolDefinition {
            tool_id: "Artifact.Create".to_string(),
            display_name: String::new(),
            description: "Create an artifact".to_string(),
            input_schema: JsonSchema::Object {
                properties: vec![SchemaProperty::new("name", JsonSchema::String)],
                required: vec!["missing".to_string()],
                allow_unknown_fields: false,
            },
            output_schema: None,
            side_effects: ToolSideEffects::Write,
            idempotency: ToolIdempotency::Conditional,
            concurrency: ToolConcurrency::Serialized,
            streaming: ToolStreaming::Events,
            required_tier: PrivilegeTier::Tier1,
            required_capabilities: vec!["artifact.write".to_string()],
            preferred_lock_scope: Some("artifact".to_string()),
            timeout_seconds: Some(30),
            tags: vec!["artifact".to_string()],
            stability: ToolStability::Experimental,
        };

        let report = definition.validate();
        assert!(!report.ok);
        assert!(report.errors.iter().any(|error| error.path == "tool_id"));
        assert!(report
            .errors
            .iter()
            .any(|error| error.path == "display_name"));
        assert!(report
            .errors
            .iter()
            .any(|error| error.path == "input_schema.required.missing"));
    }

    #[test]
    fn registry_rejects_duplicates_and_validates_registered_calls() {
        let definition = artifact_create_definition();
        let mut registry = InMemoryToolRegistry::new();

        registry.register(definition.clone()).unwrap();
        assert_eq!(
            registry.register(definition).unwrap_err(),
            ToolApiError::DuplicateToolId("artifact.create".to_string())
        );

        let request = ToolInvocationRequest {
            call_id: "call_1".to_string(),
            tool_id: "artifact.create".to_string(),
            arguments: JsonValue::Object(vec![
                (
                    "collection".to_string(),
                    JsonValue::String("session-artifacts".to_string()),
                ),
                (
                    "name".to_string(),
                    JsonValue::String("brief.md".to_string()),
                ),
                (
                    "body_base64".to_string(),
                    JsonValue::String("SGVsbG8=".to_string()),
                ),
            ]),
            requested_by: RequestedBy::Agent,
            session_id: Some("session_1".to_string()),
            job_id: None,
            agent_id: Some("agent_1".to_string()),
            user_id: None,
            requested_at: 100,
            deadline_at: Some(200),
            idempotency_key: Some("artifact-create-1".to_string()),
        };

        assert!(registry.validate_call(&request).ok);

        let unknown = ToolInvocationRequest {
            tool_id: "artifact.delete".to_string(),
            ..request
        };
        let report = registry.validate_call(&unknown);
        assert!(!report.ok);
        assert!(report
            .errors
            .iter()
            .any(|error| error.path == "tool_id" && error.message == "tool is not registered"));
    }

    #[test]
    fn registry_lists_definitions_by_tool_id() {
        let mut registry = InMemoryToolRegistry::new();
        registry
            .register(ToolDefinition {
                tool_id: "memory.remember".to_string(),
                display_name: "Remember".to_string(),
                ..artifact_create_definition()
            })
            .unwrap();
        registry.register(artifact_create_definition()).unwrap();

        let ids: Vec<_> = registry
            .list()
            .into_iter()
            .map(|definition| definition.tool_id.as_str())
            .collect();
        assert_eq!(ids, vec!["artifact.create", "memory.remember"]);
    }

    #[test]
    fn registry_queries_definitions_by_catalog_metadata() {
        let mut registry = InMemoryToolRegistry::new();
        for definition in [
            memory_search_definition(),
            memory_remember_definition(),
            artifact_create_definition(),
        ] {
            registry.register(definition).unwrap();
        }

        let read_memory = registry.query(
            &ToolCatalogQuery::new()
                .for_family(BuiltinToolFamily::Memory)
                .with_side_effects(ToolSideEffects::Read)
                .with_max_tier(PrivilegeTier::Tier0)
                .requiring_capability("memory:read")
                .with_tag("store"),
        );
        assert_eq!(
            read_memory
                .iter()
                .map(|definition| definition.tool_id.as_str())
                .collect::<Vec<_>>(),
            vec!["memory.search"]
        );

        assert!(registry
            .query(&ToolCatalogQuery::new().with_limit(0))
            .is_empty());
    }

    #[test]
    fn builtin_catalog_definitions_are_valid_and_registerable() {
        let catalog = builtin_tool_catalog();
        let mut registry = InMemoryToolRegistry::new();

        assert_eq!(catalog.len(), 33);
        for definition in catalog {
            assert!(
                definition.validate().ok,
                "builtin definition {} should validate",
                definition.tool_id
            );
            registry.register(definition).unwrap();
        }

        let ids: Vec<_> = registry
            .list()
            .into_iter()
            .map(|definition| definition.tool_id.as_str())
            .collect();
        assert_eq!(
            ids,
            vec![
                "artifact.create",
                "artifact.list",
                "artifact.mark_retention",
                "artifact.read",
                "artifact.read_revision",
                "artifact.tag",
                "artifact.write_revision",
                "context.append_entry",
                "context.archive_session",
                "context.compact",
                "context.create_snapshot",
                "context.open_session",
                "context.read_entries",
                "job.install",
                "job.list",
                "job.run_now",
                "job.status",
                "job.uninstall",
                "job.validate",
                "memory.expire",
                "memory.list_by_class",
                "memory.list_by_tag",
                "memory.remember",
                "memory.search",
                "memory.supersede",
                "memory.tombstone",
                "skill.activate",
                "skill.deactivate",
                "skill.install",
                "skill.list",
                "skill.read_asset",
                "skill.read_manifest",
                "skill.uninstall",
            ]
        );
    }

    #[test]
    fn builtin_catalog_can_filter_by_family_and_lookup_by_id() {
        let memory_tools = builtin_tools_for_family(BuiltinToolFamily::Memory);
        let memory_ids: Vec<_> = memory_tools
            .iter()
            .map(|definition| definition.tool_id.as_str())
            .collect();
        let skill_tools = builtin_tools_for_family(BuiltinToolFamily::Skill);
        let skill_ids: Vec<_> = skill_tools
            .iter()
            .map(|definition| definition.tool_id.as_str())
            .collect();

        assert_eq!(
            memory_ids,
            vec![
                "memory.remember",
                "memory.search",
                "memory.list_by_class",
                "memory.list_by_tag",
                "memory.supersede",
                "memory.expire",
                "memory.tombstone",
            ]
        );
        assert_eq!(
            skill_ids,
            vec![
                "skill.list",
                "skill.read_manifest",
                "skill.read_asset",
                "skill.install",
                "skill.activate",
                "skill.deactivate",
                "skill.uninstall",
            ]
        );
        assert_eq!(
            builtin_tool_definition("job.install")
                .unwrap()
                .required_capabilities,
            vec!["jobs:install"]
        );
        assert_eq!(
            builtin_tool_definition("job.run_now")
                .unwrap()
                .required_capabilities,
            vec!["jobs:run"]
        );
        assert_eq!(
            builtin_tool_definition("skill.install")
                .unwrap()
                .required_capabilities,
            vec!["skills:install"]
        );
        assert!(builtin_tool_definition("vault.request_lease").is_none());
    }

    #[test]
    fn builtin_catalog_can_query_by_capability_tag_and_limit() {
        let read_memory = builtin_tools_matching(
            ToolCatalogQuery::new()
                .for_family(BuiltinToolFamily::Memory)
                .with_side_effects(ToolSideEffects::Read)
                .requiring_capability("memory:read")
                .with_tag("store")
                .with_limit(2),
        );
        let read_memory_ids: Vec<_> = read_memory
            .iter()
            .map(|definition| definition.tool_id.as_str())
            .collect();

        assert_eq!(
            read_memory_ids,
            vec!["memory.search", "memory.list_by_class"]
        );
        assert!(read_memory
            .iter()
            .all(|definition| definition.required_tier <= PrivilegeTier::Tier0));
        assert!(builtin_tools_matching(
            ToolCatalogQuery::new().with_stability(ToolStability::Stable)
        )
        .is_empty());
    }

    #[test]
    fn builtin_catalog_export_summarizes_schema_documents_and_validation() {
        let export = builtin_tool_catalog_export(ToolCatalogQuery::new());

        assert!(export.ok());
        assert_eq!(export.summary.total_tools, 33);
        assert_eq!(export.schema_documents.len(), 33);
        assert_eq!(export.summary.by_family.get("context"), Some(&6));
        assert_eq!(export.summary.by_family.get("artifact"), Some(&7));
        assert_eq!(export.summary.by_family.get("skill"), Some(&7));
        assert_eq!(export.summary.by_family.get("memory"), Some(&7));
        assert_eq!(export.summary.by_family.get("job"), Some(&6));
        assert_eq!(export.summary.streaming_tools, 17);
        assert!(export.summary.write_or_external_tools > 0);
        assert_eq!(
            export.tool_ids().first().copied(),
            Some("context.open_session")
        );

        let read_export = builtin_tool_catalog_export(
            ToolCatalogQuery::new()
                .with_side_effects(ToolSideEffects::Read)
                .with_max_tier(PrivilegeTier::Tier0),
        );

        assert!(read_export.ok());
        assert!(read_export
            .schema_documents
            .iter()
            .any(|document| document.tool_id == "memory.search"));
        assert!(read_export
            .summary
            .by_side_effects
            .get("read")
            .is_some_and(|count| *count > 0));
    }

    #[test]
    fn builtin_catalog_exports_schema_light_definition_summaries() {
        let summaries = builtin_tool_definition_summaries(
            ToolCatalogQuery::new()
                .for_family(BuiltinToolFamily::Skill)
                .with_side_effects(ToolSideEffects::Read)
                .with_limit(2),
        );

        assert_eq!(
            summaries
                .iter()
                .map(|summary| summary.tool_id.as_str())
                .collect::<Vec<_>>(),
            vec!["skill.list", "skill.read_manifest"]
        );
        assert_eq!(summaries[0].display_name, "List skills");
        assert_eq!(summaries[0].side_effects, ToolSideEffects::Read);
        assert_eq!(summaries[0].idempotency, ToolIdempotency::Always);
        assert_eq!(summaries[0].concurrency, ToolConcurrency::Safe);
        assert_eq!(summaries[0].streaming, ToolStreaming::None);
        assert_eq!(summaries[0].required_tier, PrivilegeTier::Tier0);
        assert_eq!(summaries[0].required_capabilities, vec!["skills:read"]);
        assert_eq!(summaries[0].tags, vec!["skill", "store"]);
        assert!(summaries[0].has_output_schema);
    }

    #[test]
    fn catalog_validation_rejects_duplicate_tool_ids() {
        let definitions = vec![artifact_create_definition(), artifact_create_definition()];

        let report = validate_tool_catalog(definitions.iter());

        assert!(!report.ok);
        assert!(report.errors.iter().any(|error| {
            error.path == "tools[1].tool_id" && error.message.contains("duplicate tool id")
        }));
    }

    #[test]
    fn registry_exports_filtered_schema_documents_and_summary() {
        let mut registry = InMemoryToolRegistry::new();
        for definition in [
            memory_search_definition(),
            memory_remember_definition(),
            artifact_create_definition(),
        ] {
            registry.register(definition).unwrap();
        }

        let export = registry.export(
            &ToolCatalogQuery::new()
                .for_family(BuiltinToolFamily::Memory)
                .with_side_effects(ToolSideEffects::Read),
        );

        assert!(export.ok());
        assert_eq!(export.tool_ids(), vec!["memory.search"]);
        assert_eq!(export.summary.total_tools, 1);
        assert_eq!(export.summary.by_family.get("memory"), Some(&1));
        assert_eq!(registry.summary().total_tools, 3);
        assert_eq!(
            registry
                .schema_documents(&ToolCatalogQuery::new().for_family(BuiltinToolFamily::Artifact))
                .into_iter()
                .map(|document| document.tool_id)
                .collect::<Vec<_>>(),
            vec!["artifact.create"]
        );
        assert_eq!(
            registry
                .definition_summaries(
                    &ToolCatalogQuery::new().for_family(BuiltinToolFamily::Memory)
                )
                .into_iter()
                .map(|summary| (summary.tool_id, summary.has_output_schema))
                .collect::<Vec<_>>(),
            vec![
                ("memory.remember".to_string(), true),
                ("memory.search".to_string(), true)
            ]
        );
    }

    #[test]
    fn builtin_catalog_schemas_reject_malformed_calls_before_handlers() {
        let mut registry = InMemoryToolRegistry::new();
        registry
            .register(builtin_tool_definition("memory.search").unwrap())
            .unwrap();

        let missing_query = ToolInvocationRequest {
            call_id: "call_memory_search".to_string(),
            tool_id: "memory.search".to_string(),
            arguments: JsonValue::Object(vec![(
                "limit".to_string(),
                JsonValue::Number(JsonNumber::Integer(10)),
            )]),
            requested_by: RequestedBy::Session,
            session_id: Some("session_1".to_string()),
            job_id: None,
            agent_id: None,
            user_id: None,
            requested_at: 1_000,
            deadline_at: None,
            idempotency_key: None,
        };

        let report = registry.validate_call(&missing_query);

        assert!(!report.ok);
        assert!(report
            .errors
            .iter()
            .any(|error| error.path == "$.query" && error.message == "required field is missing"));

        let mut valid = missing_query;
        valid.arguments = JsonValue::Object(vec![
            (
                "query".to_string(),
                JsonValue::String("weekly briefing preferences".to_string()),
            ),
            (
                "limit".to_string(),
                JsonValue::Number(JsonNumber::Integer(10)),
            ),
        ]);

        assert!(registry.validate_call(&valid).ok);
    }

    #[test]
    fn builtin_skill_schemas_reject_malformed_asset_reads() {
        let mut registry = InMemoryToolRegistry::new();
        registry
            .register(builtin_tool_definition("skill.read_asset").unwrap())
            .unwrap();

        let missing_asset_path = ToolInvocationRequest {
            call_id: "call_skill_read_asset".to_string(),
            tool_id: "skill.read_asset".to_string(),
            arguments: JsonValue::Object(vec![
                (
                    "skill_id".to_string(),
                    JsonValue::String("daily_brief".to_string()),
                ),
                (
                    "version".to_string(),
                    JsonValue::String("1.0.0".to_string()),
                ),
            ]),
            requested_by: RequestedBy::Session,
            session_id: Some("session_1".to_string()),
            job_id: None,
            agent_id: None,
            user_id: None,
            requested_at: 1_000,
            deadline_at: None,
            idempotency_key: None,
        };

        let report = registry.validate_call(&missing_asset_path);

        assert!(!report.ok);
        assert!(report
            .errors
            .iter()
            .any(|error| error.path == "$.asset_path"
                && error.message == "required field is missing"));

        let mut valid = missing_asset_path;
        valid.arguments = JsonValue::Object(vec![
            (
                "skill_id".to_string(),
                JsonValue::String("daily_brief".to_string()),
            ),
            (
                "version".to_string(),
                JsonValue::String("1.0.0".to_string()),
            ),
            (
                "asset_path".to_string(),
                JsonValue::String("prompts/brief.md".to_string()),
            ),
        ]);

        assert!(registry.validate_call(&valid).ok);
    }

    #[test]
    fn expanded_builtin_schemas_enforce_store_specific_enums() {
        let mut registry = InMemoryToolRegistry::new();
        registry
            .register(builtin_tool_definition("artifact.mark_retention").unwrap())
            .unwrap();
        registry
            .register(builtin_tool_definition("memory.list_by_class").unwrap())
            .unwrap();

        let invalid_retention = ToolInvocationRequest {
            call_id: "call_artifact_retention".to_string(),
            tool_id: "artifact.mark_retention".to_string(),
            arguments: JsonValue::Object(vec![
                (
                    "artifact_id".to_string(),
                    JsonValue::String("artifact_1".to_string()),
                ),
                (
                    "retention".to_string(),
                    JsonValue::String("forever-ish".to_string()),
                ),
            ]),
            requested_by: RequestedBy::Session,
            session_id: Some("session_1".to_string()),
            job_id: None,
            agent_id: None,
            user_id: None,
            requested_at: 1_000,
            deadline_at: None,
            idempotency_key: None,
        };
        let invalid_class = ToolInvocationRequest {
            call_id: "call_memory_class".to_string(),
            tool_id: "memory.list_by_class".to_string(),
            arguments: JsonValue::Object(vec![(
                "class".to_string(),
                JsonValue::String("preference".to_string()),
            )]),
            requested_by: RequestedBy::Session,
            session_id: Some("session_1".to_string()),
            job_id: None,
            agent_id: None,
            user_id: None,
            requested_at: 1_000,
            deadline_at: None,
            idempotency_key: None,
        };

        assert!(registry
            .validate_call(&invalid_retention)
            .errors
            .iter()
            .any(|error| error.path == "$.retention" && error.message == "value is not in enum"));
        assert!(registry
            .validate_call(&invalid_class)
            .errors
            .iter()
            .any(|error| error.path == "$.class" && error.message == "value is not in enum"));

        let mut valid = invalid_retention;
        valid.arguments = JsonValue::Object(vec![
            (
                "artifact_id".to_string(),
                JsonValue::String("artifact_1".to_string()),
            ),
            (
                "retention".to_string(),
                JsonValue::String("retained".to_string()),
            ),
        ]);
        assert!(registry.validate_call(&valid).ok);
    }

    #[test]
    fn invocation_queries_filter_scope_time_and_sort_results() {
        let mut first = artifact_create_request();
        first.call_id = "call_1".to_string();
        first.session_id = Some("session_a".to_string());
        first.requested_at = 100;
        first.deadline_at = Some(250);

        let mut second = first.clone();
        second.call_id = "call_2".to_string();
        second.tool_id = "memory.search".to_string();
        second.requested_by = RequestedBy::Job;
        second.job_id = Some("job_1".to_string());
        second.requested_at = 200;
        second.deadline_at = Some(225);

        let mut third = first.clone();
        third.call_id = "call_3".to_string();
        third.session_id = Some("session_b".to_string());
        third.requested_at = 300;

        let query = ToolInvocationQuery::new()
            .in_session("session_a")
            .requested_since(150)
            .deadline_before(240)
            .sorted_by(ToolInvocationSort::RequestedAtDesc)
            .with_limit(1);
        let matches = query_tool_invocation_requests([&first, &second, &third], &query);

        assert_eq!(
            matches
                .iter()
                .map(|request| request.call_id.as_str())
                .collect::<Vec<_>>(),
            vec!["call_2"]
        );

        let by_tool = query_tool_invocation_requests(
            [&first, &second, &third],
            &ToolInvocationQuery::new().sorted_by(ToolInvocationSort::ToolIdThenRequestedAt),
        );
        assert_eq!(
            by_tool
                .iter()
                .map(|request| request.tool_id.as_str())
                .collect::<Vec<_>>(),
            vec!["artifact.create", "artifact.create", "memory.search"]
        );
    }

    #[test]
    fn call_record_queries_find_active_approval_work() {
        let queued = ToolCallRecord {
            call_id: "call_1".to_string(),
            tool_id: "artifact.create".to_string(),
            status: ToolCallStatus::Queued,
            started_at: None,
            completed_at: None,
            lock_scope: Some("artifact".to_string()),
            approval_state: ApprovalState::NotRequired,
            metrics: ToolMetrics::default(),
        };
        let awaiting_approval = ToolCallRecord {
            call_id: "call_2".to_string(),
            status: ToolCallStatus::AwaitingApproval,
            started_at: Some(220),
            approval_state: ApprovalState::Pending,
            ..queued.clone()
        };
        let completed = ToolCallRecord {
            call_id: "call_3".to_string(),
            status: ToolCallStatus::Completed,
            started_at: Some(120),
            completed_at: Some(180),
            approval_state: ApprovalState::Granted,
            ..queued.clone()
        };

        assert!(ToolCallStatus::Running.is_active());
        assert!(!ToolCallStatus::Completed.is_active());

        let records = vec![queued, awaiting_approval, completed];
        let matches = query_tool_call_records(
            records.iter(),
            &ToolCallRecordQuery::new()
                .active_only()
                .with_approval_state(ApprovalState::Pending)
                .with_lock_scope("artifact")
                .started_since(200)
                .sorted_by(ToolCallRecordSort::StartedAtDesc),
        );

        assert_eq!(
            matches
                .iter()
                .map(|record| record.call_id.as_str())
                .collect::<Vec<_>>(),
            vec!["call_2"]
        );
    }

    #[test]
    fn event_queries_filter_terminal_events_and_sequences() {
        let events = vec![
            ToolEvent {
                call_id: "call_1".to_string(),
                sequence: 0,
                at: 100,
                kind: ToolEventKind::Started,
                payload: JsonValue::Null,
            },
            ToolEvent {
                call_id: "call_1".to_string(),
                sequence: 1,
                at: 110,
                kind: ToolEventKind::Progress,
                payload: JsonValue::String("half way".to_string()),
            },
            ToolEvent {
                call_id: "call_1".to_string(),
                sequence: 2,
                at: 120,
                kind: ToolEventKind::Failed,
                payload: JsonValue::Null,
            },
            ToolEvent {
                call_id: "call_2".to_string(),
                sequence: 0,
                at: 115,
                kind: ToolEventKind::Completed,
                payload: JsonValue::Null,
            },
        ];

        let terminal = query_tool_events(
            events.iter(),
            &ToolEventQuery::new()
                .for_call("call_1")
                .terminal_only()
                .sequence_min(1)
                .sorted_by(ToolEventSort::SequenceDesc),
        );
        assert_eq!(
            terminal.iter().map(|event| event.kind).collect::<Vec<_>>(),
            vec![ToolEventKind::Failed]
        );

        let progress = query_tool_events(
            events.iter(),
            &ToolEventQuery::new()
                .with_kind(ToolEventKind::Progress)
                .since(100)
                .until(115),
        );
        assert_eq!(progress[0].sequence, 1);
    }

    #[test]
    fn result_queries_filter_failures_refs_and_metrics() {
        let mut completed =
            ToolResult::completed("call_1", JsonValue::String("artifact written".to_string()));
        completed.artifact_refs.push("artifact_1/rev_1".to_string());
        completed.metrics.run_ms = 10;

        let mut denied = ToolResult::failed(
            "call_2",
            ToolCallError::new(ToolErrorKind::ToolPermissionDenied, "policy denied"),
        );
        denied.metrics.run_ms = 40;

        let mut failed = ToolResult::failed(
            "call_3",
            ToolCallError::new(ToolErrorKind::ToolExecutionError, "handler failed"),
        );
        failed.metrics.run_ms = 20;

        let results = vec![completed, denied, failed];
        let permission_failures = query_tool_results(
            results.iter(),
            &ToolResultQuery::new()
                .with_success(false)
                .with_error_kind(ToolErrorKind::ToolPermissionDenied)
                .min_run_ms(30)
                .sorted_by(ToolResultSort::RunMsDesc),
        );
        assert_eq!(permission_failures[0].call_id, "call_2");

        let artifact_results = query_tool_results(
            results.iter(),
            &ToolResultQuery::new().with_artifact_refs(true),
        );
        assert_eq!(
            artifact_results
                .iter()
                .map(|result| result.call_id.as_str())
                .collect::<Vec<_>>(),
            vec!["call_1"]
        );
    }

    #[test]
    fn terminal_events_must_agree_with_final_results() {
        let completed = ToolEvent {
            call_id: "call_1".to_string(),
            sequence: 2,
            at: 150,
            kind: ToolEventKind::Completed,
            payload: JsonValue::Null,
        };
        let result = ToolResult::completed("call_1", JsonValue::String("ok".to_string()));
        assert!(completed.terminal_matches_result(&result));

        let failed = ToolEvent {
            kind: ToolEventKind::Failed,
            ..completed
        };
        assert!(!failed.terminal_matches_result(&result));

        let cancelled_result = ToolResult::failed(
            "call_1",
            ToolCallError::new(ToolErrorKind::ToolCancelled, "cancelled by caller"),
        );
        let cancelled = ToolEvent {
            kind: ToolEventKind::Cancelled,
            sequence: 3,
            ..failed
        };
        assert!(cancelled.terminal_matches_result(&cancelled_result));
    }

    #[test]
    fn runtime_invokes_registered_handler_and_emits_ordered_events() {
        let mut runtime = InMemoryToolRuntime::new();
        runtime
            .register_handler(
                artifact_create_definition(),
                |arguments: JsonValue, context: ToolExecutionContext| {
                    assert_eq!(context.call_id, "call_1");
                    assert!(matches!(arguments, JsonValue::Object(_)));
                    Ok(ToolHandlerOutput::new(JsonValue::Object(vec![(
                        "artifact_ref".to_string(),
                        JsonValue::String("artifact_1/rev_1".to_string()),
                    )]))
                    .with_artifact_ref("artifact_1/rev_1")
                    .with_event(
                        ToolEventKind::Progress,
                        JsonValue::String("writing revision".to_string()),
                    ))
                },
            )
            .unwrap();

        let trace = runtime.invoke_with_events(&artifact_create_request());

        assert!(trace.result.ok);
        assert_eq!(trace.result.artifact_refs, vec!["artifact_1/rev_1"]);
        assert_eq!(trace.record.status, ToolCallStatus::Completed);
        assert_eq!(
            trace
                .events
                .iter()
                .map(|event| event.kind)
                .collect::<Vec<_>>(),
            vec![
                ToolEventKind::Started,
                ToolEventKind::Progress,
                ToolEventKind::Completed,
            ]
        );
        assert!(trace
            .events
            .last()
            .unwrap()
            .terminal_matches_result(&trace.result));
    }

    #[test]
    fn runtime_rejects_invalid_handler_output_before_completed_result() {
        let mut runtime = InMemoryToolRuntime::new();
        runtime
            .register_handler(artifact_create_definition(), |_, _| {
                Ok(ToolHandlerOutput::new(JsonValue::Object(vec![(
                    "wrong_field".to_string(),
                    JsonValue::String("artifact_1/rev_1".to_string()),
                )]))
                .with_artifact_ref("artifact_1/rev_1")
                .with_event(
                    ToolEventKind::Progress,
                    JsonValue::String("writing revision".to_string()),
                ))
            })
            .unwrap();

        let trace = runtime.invoke_with_events(&artifact_create_request());

        assert!(!trace.result.ok);
        assert!(trace.result.output.is_none());
        assert!(trace.result.artifact_refs.is_empty());
        assert_eq!(trace.record.status, ToolCallStatus::Failed);
        let error = trace.result.error.as_ref().unwrap();
        assert_eq!(error.kind, ToolErrorKind::ToolValidationError);
        assert_eq!(error.message, "tool handler output failed validation");
        assert!(format!("{:?}", error.details).contains("$.artifact_ref"));
        assert_eq!(
            trace
                .events
                .iter()
                .map(|event| event.kind)
                .collect::<Vec<_>>(),
            vec![
                ToolEventKind::Started,
                ToolEventKind::Progress,
                ToolEventKind::Failed,
            ]
        );
        assert!(trace
            .events
            .last()
            .unwrap()
            .terminal_matches_result(&trace.result));
    }

    #[test]
    fn runtime_rejects_invalid_arguments_before_handler_execution() {
        use std::sync::{Arc, Mutex};

        let called = Arc::new(Mutex::new(false));
        let called_by_handler = Arc::clone(&called);
        let mut runtime = InMemoryToolRuntime::new();
        runtime
            .register_handler(artifact_create_definition(), move |_, _| {
                *called_by_handler.lock().unwrap() = true;
                Ok(ToolHandlerOutput::new(JsonValue::Null))
            })
            .unwrap();

        let mut request = artifact_create_request();
        request.arguments = JsonValue::Object(vec![(
            "collection".to_string(),
            JsonValue::String("session-artifacts".to_string()),
        )]);

        let trace = runtime.invoke_with_events(&request);

        assert!(!trace.result.ok);
        assert_eq!(
            trace.result.error.as_ref().unwrap().kind,
            ToolErrorKind::ToolValidationError
        );
        assert_eq!(trace.record.status, ToolCallStatus::Failed);
        assert_eq!(trace.events.len(), 1);
        assert_eq!(trace.events[0].kind, ToolEventKind::Failed);
        assert!(!*called.lock().unwrap());
    }

    #[test]
    fn runtime_denies_calls_before_handler_execution_when_policy_rejects() {
        use std::sync::{Arc, Mutex};

        let called = Arc::new(Mutex::new(false));
        let called_by_handler = Arc::clone(&called);
        let mut runtime =
            InMemoryToolRuntime::with_policy(ToolPolicyProfile::read_only(PrivilegeTier::Tier3));
        runtime
            .register_handler(artifact_create_definition(), move |_, _| {
                *called_by_handler.lock().unwrap() = true;
                Ok(ToolHandlerOutput::new(JsonValue::Null))
            })
            .unwrap();

        let trace = runtime.invoke_with_events(&artifact_create_request());

        assert!(!trace.result.ok);
        assert_eq!(trace.record.status, ToolCallStatus::Failed);
        assert_eq!(trace.record.approval_state, ApprovalState::Denied);
        assert_eq!(
            trace.result.error.as_ref().unwrap().kind,
            ToolErrorKind::ToolPermissionDenied
        );
        assert_eq!(trace.events.len(), 1);
        assert_eq!(trace.events[0].kind, ToolEventKind::Failed);
        assert!(!*called.lock().unwrap());
    }

    #[test]
    fn runtime_marks_calls_awaiting_approval_before_handler_execution() {
        use std::sync::{Arc, Mutex};

        let called = Arc::new(Mutex::new(false));
        let called_by_handler = Arc::clone(&called);
        let policy =
            ToolPolicyProfile::allow_all().with_approval_required_for(vec![ToolSideEffects::Write]);
        let mut runtime = InMemoryToolRuntime::with_policy(policy);
        runtime
            .register_handler(artifact_create_definition(), move |_, _| {
                *called_by_handler.lock().unwrap() = true;
                Ok(ToolHandlerOutput::new(JsonValue::Null))
            })
            .unwrap();

        let trace = runtime.invoke_with_events(&artifact_create_request());

        assert!(!trace.result.ok);
        assert_eq!(trace.record.status, ToolCallStatus::AwaitingApproval);
        assert_eq!(trace.record.approval_state, ApprovalState::Pending);
        assert_eq!(
            trace.result.error.as_ref().unwrap().kind,
            ToolErrorKind::ToolApprovalRequired
        );
        assert!(trace
            .result
            .error
            .as_ref()
            .unwrap()
            .message
            .contains("requires approval"));
        assert_eq!(trace.events.len(), 1);
        assert_eq!(trace.events[0].kind, ToolEventKind::Failed);
        assert!(!*called.lock().unwrap());
    }

    #[test]
    fn runtime_invokes_approval_required_calls_with_matching_grants() {
        use std::sync::{Arc, Mutex};

        let called = Arc::new(Mutex::new(false));
        let called_by_handler = Arc::clone(&called);
        let policy =
            ToolPolicyProfile::allow_all().with_approval_required_for(vec![ToolSideEffects::Write]);
        let mut runtime = InMemoryToolRuntime::with_policy(policy);
        runtime
            .register_handler(artifact_create_definition(), move |_, _| {
                *called_by_handler.lock().unwrap() = true;
                Ok(ToolHandlerOutput::new(JsonValue::Object(vec![(
                    "artifact_ref".to_string(),
                    JsonValue::String("artifact_1".to_string()),
                )])))
            })
            .unwrap();
        let request = artifact_create_request();
        let grant = ToolApprovalGrant::new(
            request.call_id.clone(),
            request.tool_id.clone(),
            "user_1",
            request.requested_at + 10,
        )
        .with_expires_at(request.requested_at + 1_000);

        let trace = runtime.invoke_with_events_with_approval(&request, &grant);

        assert!(trace.result.ok);
        assert_eq!(trace.record.status, ToolCallStatus::Completed);
        assert_eq!(trace.record.approval_state, ApprovalState::Granted);
        assert_eq!(
            trace
                .events
                .iter()
                .map(|event| event.kind)
                .collect::<Vec<_>>(),
            vec![ToolEventKind::Started, ToolEventKind::Completed]
        );
        assert!(trace
            .events
            .last()
            .unwrap()
            .terminal_matches_result(&trace.result));
        assert!(*called.lock().unwrap());
    }

    #[test]
    fn runtime_rejects_stale_or_mismatched_approval_grants() {
        use std::sync::{Arc, Mutex};

        let called = Arc::new(Mutex::new(false));
        let called_by_handler = Arc::clone(&called);
        let policy =
            ToolPolicyProfile::allow_all().with_approval_required_for(vec![ToolSideEffects::Write]);
        let mut runtime = InMemoryToolRuntime::with_policy(policy);
        runtime
            .register_handler(artifact_create_definition(), move |_, _| {
                *called_by_handler.lock().unwrap() = true;
                Ok(ToolHandlerOutput::new(JsonValue::Null))
            })
            .unwrap();
        let request = artifact_create_request();
        let stale = ToolApprovalGrant::new(
            request.call_id.clone(),
            request.tool_id.clone(),
            "user_1",
            request.requested_at,
        )
        .with_expires_at(request.requested_at - 1);

        let stale_trace = runtime.invoke_with_events_with_approval(&request, &stale);

        assert!(!stale_trace.result.ok);
        assert_eq!(stale_trace.record.status, ToolCallStatus::Failed);
        assert_eq!(stale_trace.record.approval_state, ApprovalState::Expired);
        assert_eq!(
            stale_trace.result.error.as_ref().unwrap().kind,
            ToolErrorKind::ToolApprovalDenied
        );
        assert!(stale_trace
            .result
            .error
            .as_ref()
            .unwrap()
            .message
            .contains("expired"));
        assert_eq!(stale_trace.events.len(), 1);
        assert_eq!(stale_trace.events[0].kind, ToolEventKind::Failed);

        let mismatched = ToolApprovalGrant::new(
            request.call_id.clone(),
            "artifact.read",
            "user_1",
            request.requested_at,
        );
        let mismatch_trace = runtime.invoke_with_events_with_approval(&request, &mismatched);

        assert!(!mismatch_trace.result.ok);
        assert_eq!(mismatch_trace.record.approval_state, ApprovalState::Denied);
        assert_eq!(
            mismatch_trace.result.error.as_ref().unwrap().kind,
            ToolErrorKind::ToolApprovalDenied
        );
        assert!(!*called.lock().unwrap());
    }

    #[test]
    fn policy_profile_denies_missing_capabilities_and_tiers() {
        let definition = ToolDefinition {
            required_tier: PrivilegeTier::Tier2,
            required_capabilities: vec!["vault.lease".to_string()],
            ..artifact_create_definition()
        };
        let request = artifact_create_request();
        let tier_limited = ToolPolicyProfile::allow_all();
        let decision = ToolPolicyProfile {
            max_tier: PrivilegeTier::Tier1,
            ..tier_limited
        }
        .decide(&definition, &request);

        assert_eq!(
            decision.outcome,
            ToolPolicyOutcome::Denied {
                error_kind: ToolErrorKind::ToolTierDenied
            }
        );

        let capability_limited = ToolPolicyProfile::allow_all()
            .with_allowed_capabilities(vec!["artifact.write".to_string()]);
        let decision = capability_limited.decide(&definition, &request);

        assert_eq!(
            decision.outcome,
            ToolPolicyOutcome::Denied {
                error_kind: ToolErrorKind::ToolPermissionDenied
            }
        );
        assert!(decision.message.contains("vault.lease"));
    }

    #[test]
    fn runtime_wraps_handler_errors_as_failed_terminal_events() {
        let mut runtime = InMemoryToolRuntime::new();
        runtime
            .register_handler(artifact_create_definition(), |_, _| {
                Err(ToolCallError::new(
                    ToolErrorKind::ToolExecutionError,
                    "storage backend failed",
                ))
            })
            .unwrap();

        let trace = runtime.invoke_with_events(&artifact_create_request());

        assert!(!trace.result.ok);
        assert_eq!(trace.record.status, ToolCallStatus::Failed);
        assert_eq!(
            trace
                .events
                .iter()
                .map(|event| event.kind)
                .collect::<Vec<_>>(),
            vec![ToolEventKind::Started, ToolEventKind::Failed]
        );
        assert!(trace
            .events
            .last()
            .unwrap()
            .terminal_matches_result(&trace.result));
    }

    #[test]
    fn runtime_rejects_handler_emitted_terminal_events() {
        let mut runtime = InMemoryToolRuntime::new();
        runtime
            .register_handler(artifact_create_definition(), |_, _| {
                Ok(ToolHandlerOutput::new(JsonValue::Null)
                    .with_event(ToolEventKind::Completed, JsonValue::Null))
            })
            .unwrap();

        let trace = runtime.invoke_with_events(&artifact_create_request());

        assert!(!trace.result.ok);
        assert_eq!(
            trace.result.error.as_ref().unwrap().kind,
            ToolErrorKind::ToolExecutionError
        );
        assert_eq!(
            trace.result.error.as_ref().unwrap().message,
            "handlers must not emit terminal events directly"
        );
        assert_eq!(trace.events.len(), 2);
        assert_eq!(trace.events[1].kind, ToolEventKind::Failed);
    }

    fn artifact_create_definition() -> ToolDefinition {
        ToolDefinition {
            tool_id: "artifact.create".to_string(),
            display_name: "Create artifact".to_string(),
            description: "Create a durable artifact in a named collection.".to_string(),
            input_schema: artifact_create_schema(),
            output_schema: Some(JsonSchema::Object {
                properties: vec![SchemaProperty::new("artifact_ref", JsonSchema::String)],
                required: vec!["artifact_ref".to_string()],
                allow_unknown_fields: false,
            }),
            side_effects: ToolSideEffects::Write,
            idempotency: ToolIdempotency::Conditional,
            concurrency: ToolConcurrency::Serialized,
            streaming: ToolStreaming::Events,
            required_tier: PrivilegeTier::Tier1,
            required_capabilities: vec!["artifact.write".to_string()],
            preferred_lock_scope: Some("artifact".to_string()),
            timeout_seconds: Some(30),
            tags: vec!["artifact".to_string(), "storage".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn artifact_create_schema() -> JsonSchema {
        JsonSchema::Object {
            properties: vec![
                SchemaProperty::new("collection", JsonSchema::String),
                SchemaProperty::new("name", JsonSchema::String),
                SchemaProperty::new("body_base64", JsonSchema::String),
            ],
            required: vec![
                "collection".to_string(),
                "name".to_string(),
                "body_base64".to_string(),
            ],
            allow_unknown_fields: false,
        }
    }

    fn artifact_create_request() -> ToolInvocationRequest {
        ToolInvocationRequest {
            call_id: "call_1".to_string(),
            tool_id: "artifact.create".to_string(),
            arguments: JsonValue::Object(vec![
                (
                    "collection".to_string(),
                    JsonValue::String("session-artifacts".to_string()),
                ),
                (
                    "name".to_string(),
                    JsonValue::String("brief.md".to_string()),
                ),
                (
                    "body_base64".to_string(),
                    JsonValue::String("SGVsbG8=".to_string()),
                ),
            ]),
            requested_by: RequestedBy::Agent,
            session_id: Some("session_1".to_string()),
            job_id: None,
            agent_id: Some("agent_1".to_string()),
            user_id: None,
            requested_at: 100,
            deadline_at: Some(200),
            idempotency_key: Some("artifact-create-1".to_string()),
        }
    }
}
