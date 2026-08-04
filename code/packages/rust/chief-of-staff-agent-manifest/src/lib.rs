//! Strict, versioned manifest contract for D18 Chief of Staff agents.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_json_parser::try_parse_json;
use coding_adventures_json_serializer::{serialize_pretty, JsonSerializerError, SerializerConfig};
use coding_adventures_json_value::{from_ast, JsonNumber, JsonValue};
use core::fmt::{self, Display, Formatter};
use std::collections::BTreeSet;

/// Current agent-manifest schema version understood by this package.
pub const MANIFEST_VERSION: i64 = 1;
/// Canonical schema URL emitted by [`AgentManifest::to_json`].
pub const MANIFEST_SCHEMA: &str = "https://raw.githubusercontent.com/adhithyan15/coding-adventures/main/code/specs/schemas/agent_manifest.schema.json";
/// Maximum accepted UTF-8 manifest size.
pub const MAX_MANIFEST_BYTES: usize = 1024 * 1024;

const ROOT_FIELDS: &[&str] = &[
    "$schema",
    "version",
    "agent",
    "description",
    "privilege_tier",
    "channels",
    "vault_access",
    "capabilities",
    "restart_policy",
    "justification",
];
const CHANNEL_FIELDS: &[&str] = &["reads", "writes"];
const VAULT_FIELDS: &[&str] = &["secrets", "mode", "max_lease_ttl"];
const CAPABILITY_FIELDS: &[&str] = &["category", "action", "target", "justification"];

/// One validated operating-system capability from an agent manifest.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Capability {
    /// Capability taxonomy category.
    pub category: String,
    /// Operation within the category.
    pub action: String,
    /// Narrow resource selected by the operation.
    pub target: String,
    /// Human-readable reason for the access.
    pub justification: String,
}

/// Declared channel access for one agent.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChannelAccess {
    /// Channels consumed by the agent.
    pub reads: Vec<String>,
    /// Channels produced by the agent.
    pub writes: Vec<String>,
}

/// Optional access to specifically named vault secrets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VaultAccess {
    /// Secret names this agent is allowed to request.
    pub secrets: Vec<String>,
    /// Access mode: `direct`, `leased`, or `both`.
    pub mode: String,
    /// Maximum lease lifetime in seconds, from zero through 3600.
    pub max_lease_ttl: u16,
}

/// Typed schema-v1 agent manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentManifest {
    /// Stable lowercase agent identifier.
    pub agent: String,
    /// Reviewer-facing purpose statement.
    pub description: String,
    /// D18 privilege tier in the inclusive range zero through three.
    pub privilege_tier: u8,
    /// Declared input and output channels.
    pub channels: ChannelAccess,
    /// Optional vault-secret declarations.
    pub vault_access: Option<VaultAccess>,
    /// Validated OS capability profile.
    pub capabilities: Vec<Capability>,
    /// Supervisor behavior: `always`, `on-failure`, or `never`.
    pub restart_policy: String,
    /// Overall capability-profile justification.
    pub justification: String,
}

impl AgentManifest {
    /// Validate an in-memory manifest against the schema-v1 semantic contract.
    pub fn validate(&self) -> Result<(), ManifestError> {
        validate_manifest(self)
    }

    /// Render deterministic, schema-shaped pretty JSON with a trailing newline.
    pub fn to_json(&self) -> Result<String, JsonSerializerError> {
        serialize_pretty(
            &manifest_json(self),
            &SerializerConfig {
                sort_keys: false,
                trailing_newline: true,
                ..SerializerConfig::default()
            },
        )
    }
}

/// Stable classes of malformed or incompatible manifest input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ManifestError {
    /// The UTF-8 document exceeds [`MAX_MANIFEST_BYTES`].
    ManifestTooLarge,
    /// The input is not syntactically valid JSON.
    InvalidJson,
    /// A field or nested value has the wrong type or violates schema bounds.
    InvalidField(&'static str),
    /// A required object field is absent.
    MissingField(&'static str),
    /// A JSON object contains the same field more than once.
    DuplicateField(String),
    /// A JSON object contains a field outside the schema contract.
    UnknownField(String),
    /// The manifest uses a well-formed version this package does not understand.
    UnsupportedVersion(i64),
}

impl Display for ManifestError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ManifestTooLarge => formatter.write_str("agent manifest exceeds size limit"),
            Self::InvalidJson => formatter.write_str("invalid agent manifest JSON"),
            Self::InvalidField(field) => write!(formatter, "invalid agent manifest field: {field}"),
            Self::MissingField(field) => write!(formatter, "missing agent manifest field: {field}"),
            Self::DuplicateField(field) => {
                write!(formatter, "duplicate agent manifest field: {field}")
            }
            Self::UnknownField(field) => write!(formatter, "unknown agent manifest field: {field}"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported agent manifest version: {version}")
            }
        }
    }
}

impl std::error::Error for ManifestError {}

/// Parse and validate one schema-v1 agent manifest.
pub fn parse_manifest(source: &str) -> Result<AgentManifest, ManifestError> {
    if source.len() > MAX_MANIFEST_BYTES {
        return Err(ManifestError::ManifestTooLarge);
    }
    let ast = try_parse_json(source).map_err(|_| ManifestError::InvalidJson)?;
    let value = from_ast(&ast).map_err(|_| ManifestError::InvalidJson)?;
    let fields = match value {
        JsonValue::Object(fields) => fields,
        _ => return Err(ManifestError::InvalidField("manifest")),
    };
    let mut object = StrictObject::new(fields, ROOT_FIELDS)?;

    if let Some(schema) = object.take("$schema") {
        expect_string(schema, "$schema")?;
    }
    let version = expect_integer(object.required("version")?, "version")?;
    if version != MANIFEST_VERSION {
        return Err(ManifestError::UnsupportedVersion(version));
    }
    let agent = expect_string(object.required("agent")?, "agent")?;
    let description = expect_string(object.required("description")?, "description")?;
    let privilege_tier = expect_integer(object.required("privilege_tier")?, "privilege_tier")?;
    let privilege_tier =
        u8::try_from(privilege_tier).map_err(|_| ManifestError::InvalidField("privilege_tier"))?;
    let channels = parse_channels(object.required("channels")?)?;
    let vault_access = object
        .take("vault_access")
        .map(parse_vault_access)
        .transpose()?;
    let capabilities = parse_capabilities(object.required("capabilities")?)?;
    let restart_policy = object
        .take("restart_policy")
        .map(|value| expect_string(value, "restart_policy"))
        .transpose()?
        .unwrap_or_else(|| "on-failure".to_string());
    let justification = expect_string(object.required("justification")?, "justification")?;

    let manifest = AgentManifest {
        agent,
        description,
        privilege_tier,
        channels,
        vault_access,
        capabilities,
        restart_policy,
        justification,
    };
    manifest.validate()?;
    Ok(manifest)
}

fn parse_channels(value: JsonValue) -> Result<ChannelAccess, ManifestError> {
    let mut object = expect_object(value, "channels", CHANNEL_FIELDS)?;
    Ok(ChannelAccess {
        reads: expect_string_array(object.required("reads")?, "channels.reads")?,
        writes: expect_string_array(object.required("writes")?, "channels.writes")?,
    })
}

fn parse_vault_access(value: JsonValue) -> Result<VaultAccess, ManifestError> {
    let mut object = expect_object(value, "vault_access", VAULT_FIELDS)?;
    let max_lease_ttl = expect_integer(
        object.required("max_lease_ttl")?,
        "vault_access.max_lease_ttl",
    )?;
    Ok(VaultAccess {
        secrets: expect_string_array(object.required("secrets")?, "vault_access.secrets")?,
        mode: expect_string(object.required("mode")?, "vault_access.mode")?,
        max_lease_ttl: u16::try_from(max_lease_ttl)
            .map_err(|_| ManifestError::InvalidField("vault_access.max_lease_ttl"))?,
    })
}

fn parse_capabilities(value: JsonValue) -> Result<Vec<Capability>, ManifestError> {
    let values = match value {
        JsonValue::Array(values) => values,
        _ => return Err(ManifestError::InvalidField("capabilities")),
    };
    values
        .into_iter()
        .map(|value| {
            let mut object = expect_object(value, "capabilities[]", CAPABILITY_FIELDS)?;
            Ok(Capability {
                category: expect_string(object.required("category")?, "capabilities[].category")?,
                action: expect_string(object.required("action")?, "capabilities[].action")?,
                target: expect_string(object.required("target")?, "capabilities[].target")?,
                justification: expect_string(
                    object.required("justification")?,
                    "capabilities[].justification",
                )?,
            })
        })
        .collect()
}

fn validate_manifest(manifest: &AgentManifest) -> Result<(), ManifestError> {
    if !(2..=64).contains(&manifest.agent.len()) || !valid_identifier(&manifest.agent) {
        return Err(ManifestError::InvalidField("agent"));
    }
    if !(10..=200).contains(&manifest.description.chars().count()) {
        return Err(ManifestError::InvalidField("description"));
    }
    if manifest.privilege_tier > 3 {
        return Err(ManifestError::InvalidField("privilege_tier"));
    }
    for channel in manifest
        .channels
        .reads
        .iter()
        .chain(&manifest.channels.writes)
    {
        if !valid_identifier(channel) {
            return Err(ManifestError::InvalidField("channels"));
        }
    }
    let reads = manifest.channels.reads.iter().collect::<BTreeSet<_>>();
    if manifest
        .channels
        .writes
        .iter()
        .any(|channel| reads.contains(channel))
    {
        return Err(ManifestError::InvalidField("channels"));
    }
    if let Some(vault) = &manifest.vault_access {
        if vault.secrets.iter().any(String::is_empty) {
            return Err(ManifestError::InvalidField("vault_access.secrets"));
        }
        if !matches!(vault.mode.as_str(), "direct" | "leased" | "both") {
            return Err(ManifestError::InvalidField("vault_access.mode"));
        }
        if vault.max_lease_ttl > 3600 {
            return Err(ManifestError::InvalidField("vault_access.max_lease_ttl"));
        }
    }
    for capability in &manifest.capabilities {
        if !valid_capability(&capability.category, &capability.action) {
            return Err(ManifestError::InvalidField("capabilities"));
        }
        if capability.target.is_empty() {
            return Err(ManifestError::InvalidField("capabilities[].target"));
        }
        if capability.justification.chars().count() < 10 {
            return Err(ManifestError::InvalidField("capabilities[].justification"));
        }
    }
    if !matches!(
        manifest.restart_policy.as_str(),
        "always" | "on-failure" | "never"
    ) {
        return Err(ManifestError::InvalidField("restart_policy"));
    }
    if manifest.justification.chars().count() < 10 {
        return Err(ManifestError::InvalidField("justification"));
    }
    Ok(())
}

fn valid_identifier(value: &str) -> bool {
    value
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_lowercase())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn valid_capability(category: &str, action: &str) -> bool {
    matches!(
        (category, action),
        ("fs", "read" | "write" | "create" | "delete" | "list")
            | ("net", "connect" | "listen" | "dns")
            | ("proc", "exec" | "fork" | "signal")
            | ("env", "read" | "write")
            | ("ffi", "call" | "load")
            | ("time", "read" | "sleep")
            | ("stdin", "read")
            | ("stdout", "write")
    )
}

fn expect_object(
    value: JsonValue,
    field: &'static str,
    allowed: &[&str],
) -> Result<StrictObject, ManifestError> {
    match value {
        JsonValue::Object(fields) => StrictObject::new(fields, allowed),
        _ => Err(ManifestError::InvalidField(field)),
    }
}

fn expect_string(value: JsonValue, field: &'static str) -> Result<String, ManifestError> {
    match value {
        JsonValue::String(value) => Ok(value),
        _ => Err(ManifestError::InvalidField(field)),
    }
}

fn expect_integer(value: JsonValue, field: &'static str) -> Result<i64, ManifestError> {
    match value {
        JsonValue::Number(JsonNumber::Integer(value)) => Ok(value),
        _ => Err(ManifestError::InvalidField(field)),
    }
}

fn expect_string_array(
    value: JsonValue,
    field: &'static str,
) -> Result<Vec<String>, ManifestError> {
    match value {
        JsonValue::Array(values) => values
            .into_iter()
            .map(|value| expect_string(value, field))
            .collect(),
        _ => Err(ManifestError::InvalidField(field)),
    }
}

struct StrictObject {
    fields: Vec<(String, JsonValue)>,
}

impl StrictObject {
    fn new(fields: Vec<(String, JsonValue)>, allowed: &[&str]) -> Result<Self, ManifestError> {
        let mut seen = BTreeSet::new();
        for (name, _) in &fields {
            if !seen.insert(name.as_str()) {
                return Err(ManifestError::DuplicateField(name.clone()));
            }
            if !allowed.contains(&name.as_str()) {
                return Err(ManifestError::UnknownField(name.clone()));
            }
        }
        Ok(Self { fields })
    }

    fn take(&mut self, name: &str) -> Option<JsonValue> {
        self.fields
            .iter()
            .position(|(key, _)| key == name)
            .map(|index| self.fields.remove(index).1)
    }

    fn required(&mut self, name: &'static str) -> Result<JsonValue, ManifestError> {
        self.take(name).ok_or(ManifestError::MissingField(name))
    }
}

fn manifest_json(manifest: &AgentManifest) -> JsonValue {
    let strings = |values: &[String]| {
        JsonValue::Array(values.iter().cloned().map(JsonValue::String).collect())
    };
    let mut fields = vec![
        (
            "$schema".to_string(),
            JsonValue::String(MANIFEST_SCHEMA.to_string()),
        ),
        (
            "version".to_string(),
            JsonValue::Number(JsonNumber::Integer(MANIFEST_VERSION)),
        ),
        (
            "agent".to_string(),
            JsonValue::String(manifest.agent.clone()),
        ),
        (
            "description".to_string(),
            JsonValue::String(manifest.description.clone()),
        ),
        (
            "privilege_tier".to_string(),
            JsonValue::Number(JsonNumber::Integer(i64::from(manifest.privilege_tier))),
        ),
        (
            "channels".to_string(),
            JsonValue::Object(vec![
                ("reads".to_string(), strings(&manifest.channels.reads)),
                ("writes".to_string(), strings(&manifest.channels.writes)),
            ]),
        ),
    ];
    if let Some(vault) = &manifest.vault_access {
        fields.push((
            "vault_access".to_string(),
            JsonValue::Object(vec![
                ("secrets".to_string(), strings(&vault.secrets)),
                ("mode".to_string(), JsonValue::String(vault.mode.clone())),
                (
                    "max_lease_ttl".to_string(),
                    JsonValue::Number(JsonNumber::Integer(i64::from(vault.max_lease_ttl))),
                ),
            ]),
        ));
    }
    fields.extend([
        (
            "capabilities".to_string(),
            JsonValue::Array(
                manifest
                    .capabilities
                    .iter()
                    .map(|capability| {
                        JsonValue::Object(vec![
                            (
                                "category".to_string(),
                                JsonValue::String(capability.category.clone()),
                            ),
                            (
                                "action".to_string(),
                                JsonValue::String(capability.action.clone()),
                            ),
                            (
                                "target".to_string(),
                                JsonValue::String(capability.target.clone()),
                            ),
                            (
                                "justification".to_string(),
                                JsonValue::String(capability.justification.clone()),
                            ),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "restart_policy".to_string(),
            JsonValue::String(manifest.restart_policy.clone()),
        ),
        (
            "justification".to_string(),
            JsonValue::String(manifest.justification.clone()),
        ),
    ]);
    JsonValue::Object(fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"{
      "version": 1,
      "agent": "weather-agent",
      "description": "Reports a concise local weather forecast.",
      "privilege_tier": 0,
      "channels": {"reads": ["weather-requests"], "writes": ["weather-reports"]},
      "capabilities": [],
      "justification": "Uses only encrypted channels and no operating-system access."
    }"#;

    #[test]
    fn parses_defaults_and_round_trips_schema_v1() {
        let manifest = parse_manifest(MINIMAL).unwrap();
        assert_eq!(manifest.agent, "weather-agent");
        assert_eq!(manifest.restart_policy, "on-failure");
        assert!(manifest.vault_access.is_none());

        let json = manifest.to_json().unwrap();
        assert!(json.starts_with(&format!("{{\n  \"$schema\": \"{MANIFEST_SCHEMA}\",")));
        assert_eq!(parse_manifest(&json).unwrap(), manifest);
    }

    #[test]
    fn parses_complete_vault_and_capability_profile() {
        let source = MINIMAL.replace(
            "\"capabilities\": []",
            r#""vault_access": {"secrets": ["weather-api"], "mode": "leased", "max_lease_ttl": 300},
      "capabilities": [{"category": "net", "action": "connect", "target": "api.weather.gov:443", "justification": "Fetches the requested forecast."}]"#,
        );
        let manifest = parse_manifest(&source).unwrap();
        assert_eq!(manifest.vault_access.unwrap().max_lease_ttl, 300);
        assert_eq!(manifest.capabilities[0].category, "net");
    }

    #[test]
    fn rejects_unsupported_or_malformed_versions() {
        assert_eq!(
            parse_manifest(&MINIMAL.replace("\"version\": 1", "\"version\": 2")),
            Err(ManifestError::UnsupportedVersion(2))
        );
        assert_eq!(
            parse_manifest(&MINIMAL.replace("\"version\": 1", "\"version\": 1.0")),
            Err(ManifestError::InvalidField("version"))
        );
        assert_eq!(parse_manifest("{"), Err(ManifestError::InvalidJson));
    }

    #[test]
    fn rejects_duplicate_unknown_and_missing_fields() {
        assert_eq!(
            parse_manifest(&MINIMAL.replace("\"version\": 1,", "\"version\": 1, \"version\": 1,")),
            Err(ManifestError::DuplicateField("version".to_string()))
        );
        assert_eq!(
            parse_manifest(
                &MINIMAL.replace("\"version\": 1,", "\"version\": 1, \"runtime\": \"deno\",")
            ),
            Err(ManifestError::UnknownField("runtime".to_string()))
        );
        assert_eq!(
            parse_manifest(&MINIMAL.replace("\"agent\": \"weather-agent\",", "")),
            Err(ManifestError::MissingField("agent"))
        );
    }

    #[test]
    fn rejects_nested_duplicate_and_unknown_fields() {
        assert_eq!(
            parse_manifest(&MINIMAL.replace(
                "\"reads\": [\"weather-requests\"],",
                "\"reads\": [], \"reads\": [\"weather-requests\"],"
            )),
            Err(ManifestError::DuplicateField("reads".to_string()))
        );
        assert_eq!(
            parse_manifest(&MINIMAL.replace(
                "\"writes\": [\"weather-reports\"]",
                "\"writes\": [\"weather-reports\"], \"admin\": true"
            )),
            Err(ManifestError::UnknownField("admin".to_string()))
        );
    }

    #[test]
    fn rejects_invalid_schema_values_and_semantics() {
        let cases = [
            ("\"weather-agent\"", "\"Weather_Agent\"", "agent"),
            (
                "Reports a concise local weather forecast.",
                "short",
                "description",
            ),
            (
                "\"privilege_tier\": 0",
                "\"privilege_tier\": 4",
                "privilege_tier",
            ),
            (
                "\"writes\": [\"weather-reports\"]",
                "\"writes\": [\"weather-requests\"]",
                "channels",
            ),
        ];
        for (from, to, field) in cases {
            assert_eq!(
                parse_manifest(&MINIMAL.replace(from, to)),
                Err(ManifestError::InvalidField(field))
            );
        }
    }

    #[test]
    fn rejects_invalid_capability_pair_and_vault_bounds() {
        let bad_capability = MINIMAL.replace(
            "\"capabilities\": []",
            r#""capabilities": [{"category": "net", "action": "exec", "target": "curl", "justification": "Runs the network client safely."}]"#,
        );
        assert_eq!(
            parse_manifest(&bad_capability),
            Err(ManifestError::InvalidField("capabilities"))
        );

        let bad_vault = MINIMAL.replace(
            "\"capabilities\": []",
            r#""vault_access": {"secrets": ["weather-api"], "mode": "leased", "max_lease_ttl": 3601},
      "capabilities": []"#,
        );
        assert_eq!(
            parse_manifest(&bad_vault),
            Err(ManifestError::InvalidField("vault_access.max_lease_ttl"))
        );
    }

    #[test]
    fn enforces_document_size_bound() {
        let oversized = " ".repeat(MAX_MANIFEST_BYTES + 1);
        assert_eq!(
            parse_manifest(&oversized),
            Err(ManifestError::ManifestTooLarge)
        );
    }
}
