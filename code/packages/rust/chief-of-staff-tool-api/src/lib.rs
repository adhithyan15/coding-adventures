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

#[cfg(test)]
mod tests {
    use super::*;

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
}
