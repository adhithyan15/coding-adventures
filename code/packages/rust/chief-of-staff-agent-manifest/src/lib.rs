//! Strict, versioned manifest contract for D18 Chief of Staff agents.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_json_parser::try_parse_json;
use coding_adventures_json_serializer::{serialize_pretty, SerializerConfig};
use coding_adventures_json_value::{from_ast, JsonNumber, JsonValue};
use core::fmt::{self, Display, Formatter};
use std::collections::{BTreeMap, BTreeSet};

/// Current agent-manifest schema version understood by this package.
///
/// Schema v3 adds `allowed_tools`: the D18D tool identifiers this agent may
/// call. Before v3 the signed manifest declared operating-system
/// `capabilities` and `vault_access` but named no tools at all, so a
/// profile-backed supervisor had no signed source for its tool surface.
pub const MANIFEST_VERSION: i64 = 3;
/// Schema version that introduced per-channel payload-schema versions.
///
/// Still accepted for installed packages; it differs from v3 only by the
/// absence of `allowed_tools`.
pub const MANIFEST_V2_VERSION: i64 = 2;
/// Oldest agent-manifest schema version accepted for installed packages.
pub const LEGACY_MANIFEST_VERSION: i64 = 1;
/// Maximum number of tool identifiers one manifest may declare.
pub const MAX_ALLOWED_TOOLS: usize = 256;
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
    "allowed_tools",
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

/// Typed agent manifest supporting legacy schema v1 and current schema v2.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentManifest {
    /// Manifest contract version used by this document.
    pub version: i64,
    /// Stable lowercase agent identifier.
    pub agent: String,
    /// Reviewer-facing purpose statement.
    pub description: String,
    /// D18 privilege tier in the inclusive range zero through three.
    pub privilege_tier: u8,
    /// Declared input and output channels.
    pub channels: ChannelAccess,
    /// Positive payload-schema version for every declared channel, scoped by channel name.
    ///
    /// Legacy schema-v1 manifests leave this map empty. Schema-v2 channel
    /// bindings each carry one version, represented here as a lookup map.
    pub message_schema_versions: BTreeMap<String, u32>,
    /// Optional vault-secret declarations.
    pub vault_access: Option<VaultAccess>,
    /// Validated OS capability profile.
    pub capabilities: Vec<Capability>,
    /// D18D tool identifiers this agent may call, sorted and deduplicated.
    ///
    /// Schema v3 and later. Always empty for v1 and v2 manifests, which had no
    /// way to declare a tool surface. An empty list on a v3 manifest is a
    /// deliberate declaration that the agent calls no tools, not an omission:
    /// the field is required at v3.
    pub allowed_tools: Vec<String>,
    /// Supervisor behavior: `always`, `on-failure`, or `never`.
    pub restart_policy: String,
    /// Overall capability-profile justification.
    pub justification: String,
}

impl AgentManifest {
    /// Validate an in-memory manifest against its declared semantic contract.
    pub fn validate(&self) -> Result<(), ManifestError> {
        validate_manifest(self)
    }

    /// Render deterministic, schema-shaped pretty JSON with a trailing newline.
    /// Render deterministic, schema-shaped pretty JSON with a trailing newline.
    ///
    /// Validates first. `manifest_json` emits `allowed_tools` only at v3, so
    /// rendering an unvalidated manifest whose `version` disagrees with its
    /// tool list would silently drop the tool surface -- a signed artifact
    /// authorizing something other than what its author declared, with no error
    /// raised anywhere in the pipeline. Fail closed instead.
    pub fn to_json(&self) -> Result<String, ManifestError> {
        self.validate()?;
        serialize_pretty(
            &manifest_json(self),
            &SerializerConfig {
                sort_keys: false,
                trailing_newline: true,
                ..SerializerConfig::default()
            },
        )
        .map_err(|error| ManifestError::Serialization(error.to_string()))
    }

    /// Return the declared payload-schema version for one channel, if present.
    pub fn message_schema_version(&self, channel: &str) -> Option<u32> {
        self.message_schema_versions.get(channel).copied()
    }
}

/// Stable reasons two manifests cannot be wired across one channel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChannelCompatibilityError {
    /// The originator manifest is not internally valid.
    InvalidOriginator(ManifestError),
    /// The receiver manifest is not internally valid.
    InvalidReceiver(ManifestError),
    /// The originator does not declare write authority for the channel.
    OriginatorDoesNotWrite(String),
    /// The receiver does not declare read authority for the channel.
    ReceiverDoesNotRead(String),
    /// The originator is a legacy manifest without a schema declaration.
    OriginatorSchemaUndeclared(String),
    /// The receiver is a legacy manifest without a schema declaration.
    ReceiverSchemaUndeclared(String),
    /// Both sides declare the channel, but their payload-schema versions differ.
    SchemaVersionMismatch {
        /// Channel whose declarations disagree.
        channel: String,
        /// Version emitted by the originator.
        originator: u32,
        /// Version accepted by the receiver.
        receiver: u32,
    },
}

impl Display for ChannelCompatibilityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOriginator(error) => write!(formatter, "invalid originator manifest: {error}"),
            Self::InvalidReceiver(error) => write!(formatter, "invalid receiver manifest: {error}"),
            Self::OriginatorDoesNotWrite(channel) => {
                write!(formatter, "originator does not write channel: {channel}")
            }
            Self::ReceiverDoesNotRead(channel) => {
                write!(formatter, "receiver does not read channel: {channel}")
            }
            Self::OriginatorSchemaUndeclared(channel) => {
                write!(formatter, "originator has no message schema version for channel: {channel}")
            }
            Self::ReceiverSchemaUndeclared(channel) => {
                write!(formatter, "receiver has no message schema version for channel: {channel}")
            }
            Self::SchemaVersionMismatch {
                channel,
                originator,
                receiver,
            } => write!(
                formatter,
                "message schema version mismatch for channel {channel}: originator {originator}, receiver {receiver}"
            ),
        }
    }
}

impl std::error::Error for ChannelCompatibilityError {}

/// Require exact payload-schema compatibility between one writer and reader.
///
/// Schema versions are scoped by channel name. Legacy schema-v1 manifests can
/// still be discovered and inspected, but they fail closed at this wiring
/// boundary because they do not declare what payload version they understand.
pub fn require_channel_compatibility(
    originator: &AgentManifest,
    receiver: &AgentManifest,
    channel: &str,
) -> Result<(), ChannelCompatibilityError> {
    originator
        .validate()
        .map_err(ChannelCompatibilityError::InvalidOriginator)?;
    receiver
        .validate()
        .map_err(ChannelCompatibilityError::InvalidReceiver)?;
    if !originator
        .channels
        .writes
        .iter()
        .any(|value| value == channel)
    {
        return Err(ChannelCompatibilityError::OriginatorDoesNotWrite(
            channel.to_string(),
        ));
    }
    if !receiver.channels.reads.iter().any(|value| value == channel) {
        return Err(ChannelCompatibilityError::ReceiverDoesNotRead(
            channel.to_string(),
        ));
    }
    let originator_version = originator.message_schema_version(channel).ok_or_else(|| {
        ChannelCompatibilityError::OriginatorSchemaUndeclared(channel.to_string())
    })?;
    let receiver_version = receiver
        .message_schema_version(channel)
        .ok_or_else(|| ChannelCompatibilityError::ReceiverSchemaUndeclared(channel.to_string()))?;
    if originator_version != receiver_version {
        return Err(ChannelCompatibilityError::SchemaVersionMismatch {
            channel: channel.to_string(),
            originator: originator_version,
            receiver: receiver_version,
        });
    }
    Ok(())
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
    /// Rendering a validated manifest to JSON failed.
    Serialization(String),
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
            Self::Serialization(message) => {
                write!(formatter, "agent manifest serialization failed: {message}")
            }
        }
    }
}

impl std::error::Error for ManifestError {}

/// Parse and validate one schema-v1, schema-v2, or current schema-v3 agent manifest.
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
    if !matches!(
        version,
        LEGACY_MANIFEST_VERSION | MANIFEST_V2_VERSION | MANIFEST_VERSION
    ) {
        return Err(ManifestError::UnsupportedVersion(version));
    }
    let agent = expect_string(object.required("agent")?, "agent")?;
    let description = expect_string(object.required("description")?, "description")?;
    let privilege_tier = expect_integer(object.required("privilege_tier")?, "privilege_tier")?;
    let privilege_tier =
        u8::try_from(privilege_tier).map_err(|_| ManifestError::InvalidField("privilege_tier"))?;
    let (channels, message_schema_versions) =
        parse_channels(object.required("channels")?, version)?;
    let vault_access = object
        .take("vault_access")
        .map(parse_vault_access)
        .transpose()?;
    let capabilities = parse_capabilities(object.required("capabilities")?)?;
    // `allowed_tools` is in ROOT_FIELDS so the strict-object check accepts the
    // key at every version; the version gate has to be explicit, or a v1 or v2
    // manifest could smuggle a tool surface past a consumer that trusts the
    // declared version.
    let allowed_tools = match version {
        MANIFEST_VERSION => parse_allowed_tools(object.required("allowed_tools")?)?,
        _ => {
            if object.take("allowed_tools").is_some() {
                return Err(ManifestError::InvalidField("allowed_tools"));
            }
            Vec::new()
        }
    };
    let restart_policy = object
        .take("restart_policy")
        .map(|value| expect_string(value, "restart_policy"))
        .transpose()?
        .unwrap_or_else(|| "on-failure".to_string());
    let justification = expect_string(object.required("justification")?, "justification")?;

    let manifest = AgentManifest {
        version,
        agent,
        description,
        privilege_tier,
        channels,
        message_schema_versions,
        vault_access,
        capabilities,
        allowed_tools,
        restart_policy,
        justification,
    };
    manifest.validate()?;
    Ok(manifest)
}

fn parse_channels(
    value: JsonValue,
    manifest_version: i64,
) -> Result<(ChannelAccess, BTreeMap<String, u32>), ManifestError> {
    let mut object = expect_object(value, "channels", CHANNEL_FIELDS)?;
    match manifest_version {
        LEGACY_MANIFEST_VERSION => Ok((
            ChannelAccess {
                reads: expect_string_array(object.required("reads")?, "channels.reads")?,
                writes: expect_string_array(object.required("writes")?, "channels.writes")?,
            },
            BTreeMap::new(),
        )),
        MANIFEST_V2_VERSION | MANIFEST_VERSION => {
            let reads = parse_channel_bindings(object.required("reads")?, "channels.reads")?;
            let writes = parse_channel_bindings(object.required("writes")?, "channels.writes")?;
            let mut versions = BTreeMap::new();
            for (channel, version) in reads.iter().chain(&writes) {
                versions.insert(channel.clone(), *version);
            }
            Ok((
                ChannelAccess {
                    reads: reads.into_iter().map(|(channel, _)| channel).collect(),
                    writes: writes.into_iter().map(|(channel, _)| channel).collect(),
                },
                versions,
            ))
        }
        _ => unreachable!("unsupported versions returned above"),
    }
}

fn parse_channel_bindings(
    value: JsonValue,
    field: &'static str,
) -> Result<Vec<(String, u32)>, ManifestError> {
    let bindings = match value {
        JsonValue::Object(bindings) => bindings,
        _ => return Err(ManifestError::InvalidField(field)),
    };
    let mut seen = BTreeSet::new();
    bindings
        .into_iter()
        .map(|(channel, value)| {
            if !valid_identifier(&channel) || !seen.insert(channel.clone()) {
                return Err(ManifestError::InvalidField(field));
            }
            let version = expect_integer(value, field)?;
            let version = u32::try_from(version).map_err(|_| ManifestError::InvalidField(field))?;
            if version == 0 {
                return Err(ManifestError::InvalidField(field));
            }
            Ok((channel, version))
        })
        .collect()
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
    if !matches!(
        manifest.version,
        LEGACY_MANIFEST_VERSION | MANIFEST_V2_VERSION | MANIFEST_VERSION
    ) {
        return Err(ManifestError::UnsupportedVersion(manifest.version));
    }
    validate_allowed_tools(manifest)?;
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
    match manifest.version {
        LEGACY_MANIFEST_VERSION if !manifest.message_schema_versions.is_empty() => {
            return Err(ManifestError::InvalidField("message_schema_versions"));
        }
        MANIFEST_V2_VERSION | MANIFEST_VERSION => {
            let channel_count = manifest.channels.reads.len() + manifest.channels.writes.len();
            let channels = manifest
                .channels
                .reads
                .iter()
                .chain(&manifest.channels.writes)
                .collect::<BTreeSet<_>>();
            let declarations = manifest
                .message_schema_versions
                .keys()
                .collect::<BTreeSet<_>>();
            if channels.len() != channel_count
                || channels != declarations
                || manifest
                    .message_schema_versions
                    .values()
                    .any(|version| *version == 0)
            {
                return Err(ManifestError::InvalidField("message_schema_versions"));
            }
        }
        LEGACY_MANIFEST_VERSION => {}
        _ => unreachable!("unsupported versions returned above"),
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

fn parse_allowed_tools(value: JsonValue) -> Result<Vec<String>, ManifestError> {
    let mut tools = expect_string_array(value, "allowed_tools")?;
    tools.sort();
    tools.dedup();
    Ok(tools)
}

/// A D18D tool identifier: two or more dot-separated segments, each starting
/// with a lowercase letter.
///
/// ```text
///   artifact.create        ok    two segments
///   context.append_entry   ok    underscores allowed inside a segment
///   artifact.create_v2     ok    digits allowed after the first byte
///   hue.bridge.status      ok    more than two segments
///   artifact               NO    a bare namespace names no tool
///   Artifact.create        NO    uppercase
///   artifact..create       NO    empty segment
///   .create / artifact.    NO    leading or trailing dot
/// ```
///
/// The shape matters beyond tidiness: `allowed_tools` is matched against tool
/// identifiers by the broker, and a manifest that could declare a bare
/// namespace would invite prefix matching, which is how one declared tool
/// becomes a whole namespace.
fn valid_tool_id(value: &str) -> bool {
    let mut segments = value.split('.');
    let mut count = 0usize;
    for segment in &mut segments {
        count += 1;
        if !valid_identifier_underscored(segment) {
            return false;
        }
    }
    count >= 2
}

fn valid_identifier_underscored(value: &str) -> bool {
    value
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_lowercase())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn validate_allowed_tools(manifest: &AgentManifest) -> Result<(), ManifestError> {
    // v1 and v2 have no tool surface at all. A non-empty list on one of those
    // would mean a consumer reading `version` learns something false about
    // what the signed bytes authorize.
    if manifest.version != MANIFEST_VERSION && !manifest.allowed_tools.is_empty() {
        return Err(ManifestError::InvalidField("allowed_tools"));
    }
    if manifest.allowed_tools.len() > MAX_ALLOWED_TOOLS {
        return Err(ManifestError::InvalidField("allowed_tools"));
    }
    let mut seen = BTreeSet::new();
    for tool in &manifest.allowed_tools {
        if !(3..=128).contains(&tool.len()) || !valid_tool_id(tool) {
            return Err(ManifestError::InvalidField("allowed_tools"));
        }
        if !seen.insert(tool.as_str()) {
            return Err(ManifestError::InvalidField("allowed_tools"));
        }
    }
    // Sorted storage is part of the contract: `to_json` must be deterministic,
    // and a consumer comparing two manifests compares two canonical orders.
    if manifest
        .allowed_tools
        .windows(2)
        .any(|pair| pair[0] > pair[1])
    {
        return Err(ManifestError::InvalidField("allowed_tools"));
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
            JsonValue::Number(JsonNumber::Integer(manifest.version)),
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
        ("channels".to_string(), channel_json(manifest, &strings)),
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
    if manifest.version == MANIFEST_VERSION {
        fields.push((
            "allowed_tools".to_string(),
            strings(&manifest.allowed_tools),
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

fn channel_json(manifest: &AgentManifest, strings: &impl Fn(&[String]) -> JsonValue) -> JsonValue {
    if manifest.version == LEGACY_MANIFEST_VERSION {
        return JsonValue::Object(vec![
            ("reads".to_string(), strings(&manifest.channels.reads)),
            ("writes".to_string(), strings(&manifest.channels.writes)),
        ]);
    }
    let bindings = |channels: &[String]| {
        JsonValue::Object(
            channels
                .iter()
                .map(|channel| {
                    (
                        channel.clone(),
                        JsonValue::Number(JsonNumber::Integer(i64::from(
                            manifest.message_schema_version(channel).unwrap_or(0),
                        ))),
                    )
                })
                .collect(),
        )
    };
    JsonValue::Object(vec![
        ("reads".to_string(), bindings(&manifest.channels.reads)),
        ("writes".to_string(), bindings(&manifest.channels.writes)),
    ])
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

    const CURRENT: &str = r#"{
      "version": 2,
      "agent": "weather-agent",
      "description": "Reports a concise local weather forecast.",
      "privilege_tier": 0,
      "channels": {
        "reads": {"weather-requests": 1},
        "writes": {"weather-reports": 2}
      },
      "capabilities": [],
      "justification": "Uses only encrypted channels and no operating-system access."
    }"#;

    const V3: &str = r#"{
      "version": 3,
      "agent": "weather-agent",
      "description": "Reports a concise local weather forecast.",
      "privilege_tier": 0,
      "channels": {
        "reads": {"weather-requests": 1},
        "writes": {"weather-reports": 2}
      },
      "capabilities": [],
      "allowed_tools": ["artifact.write", "artifact.create_v2", "context.append_entry"],
      "justification": "Uses only encrypted channels and no operating-system access."
    }"#;

    fn v3_with_tools(tools: &str) -> String {
        V3.replace(
            r#""allowed_tools": ["artifact.write", "artifact.create_v2", "context.append_entry"]"#,
            &format!(r#""allowed_tools": {tools}"#),
        )
    }

    #[test]
    fn parses_defaults_and_round_trips_schema_v1() {
        let manifest = parse_manifest(MINIMAL).unwrap();
        assert_eq!(manifest.version, LEGACY_MANIFEST_VERSION);
        assert_eq!(manifest.agent, "weather-agent");
        assert_eq!(manifest.restart_policy, "on-failure");
        assert!(manifest.vault_access.is_none());
        assert!(manifest.message_schema_versions.is_empty());

        let json = manifest.to_json().unwrap();
        assert!(json.starts_with(&format!("{{\n  \"$schema\": \"{MANIFEST_SCHEMA}\",")));
        assert_eq!(parse_manifest(&json).unwrap(), manifest);
    }

    #[test]
    fn parses_and_round_trips_schema_v2() {
        let manifest = parse_manifest(CURRENT).unwrap();
        assert_eq!(manifest.version, MANIFEST_V2_VERSION);
        // v2 has no tool surface at all, and must not acquire one by default.
        assert!(manifest.allowed_tools.is_empty());
        assert_eq!(manifest.message_schema_version("weather-requests"), Some(1));
        assert_eq!(manifest.message_schema_version("weather-reports"), Some(2));

        let json = manifest.to_json().unwrap();
        assert!(json.contains("\"weather-reports\": 2"));
        assert_eq!(parse_manifest(&json).unwrap(), manifest);
    }

    #[test]
    fn parses_and_round_trips_schema_v3_with_tools() {
        let manifest = parse_manifest(V3).unwrap();
        assert_eq!(manifest.version, MANIFEST_VERSION);
        // Stored sorted, not in source order: `to_json` must be deterministic
        // and two manifests must compare in one canonical order.
        assert_eq!(
            manifest.allowed_tools,
            vec![
                "artifact.create_v2".to_string(),
                "artifact.write".to_string(),
                "context.append_entry".to_string(),
            ]
        );

        let json = manifest.to_json().unwrap();
        assert!(json.contains("\"allowed_tools\""));
        assert_eq!(parse_manifest(&json).unwrap(), manifest);
    }

    #[test]
    fn schema_v3_requires_allowed_tools_and_accepts_an_empty_declaration() {
        // Required, so "calls no tools" is stated rather than defaulted into.
        let missing = V3.replace(
            r#""allowed_tools": ["artifact.write", "artifact.create_v2", "context.append_entry"],"#,
            "",
        );
        assert_eq!(
            parse_manifest(&missing),
            Err(ManifestError::MissingField("allowed_tools"))
        );

        let empty = parse_manifest(&v3_with_tools("[]")).unwrap();
        assert!(empty.allowed_tools.is_empty());
        assert_eq!(parse_manifest(&empty.to_json().unwrap()).unwrap(), empty);
    }

    #[test]
    fn earlier_schemas_may_not_declare_a_tool_surface() {
        // `allowed_tools` is in ROOT_FIELDS, so the strict-object check accepts
        // the key at any version. Without an explicit gate a v1 or v2 manifest
        // could carry a tool surface that a consumer trusting `version` would
        // never look for.
        for source in [MINIMAL, CURRENT] {
            let smuggled = source.replace(
                "\"capabilities\": []",
                "\"capabilities\": [], \"allowed_tools\": [\"artifact.write\"]",
            );
            assert_eq!(
                parse_manifest(&smuggled),
                Err(ManifestError::InvalidField("allowed_tools"))
            );
        }
    }

    #[test]
    fn rejects_tool_identifiers_that_are_not_namespaced() {
        // A bare namespace names no tool, and would invite prefix matching in
        // the broker -- which is how one declared tool becomes a namespace.
        for bad in [
            r#"["artifact"]"#,
            r#"["Artifact.create"]"#,
            r#"["artifact..create"]"#,
            r#"[".create"]"#,
            r#"["artifact."]"#,
            r#"["artifact.create!"]"#,
            r#"["artifact.Create"]"#,
            r#"["2artifact.create"]"#,
            r#"["artifact.2create"]"#,
            r#"["ab"]"#,
        ] {
            assert_eq!(
                parse_manifest(&v3_with_tools(bad)),
                Err(ManifestError::InvalidField("allowed_tools")),
                "should have rejected {bad}"
            );
        }
    }

    #[test]
    fn accepts_deeper_namespaces_and_deduplicates() {
        let manifest = parse_manifest(&v3_with_tools(
            r#"["hue.bridge.status", "artifact.write", "artifact.write"]"#,
        ))
        .unwrap();
        assert_eq!(
            manifest.allowed_tools,
            vec![
                "artifact.write".to_string(),
                "hue.bridge.status".to_string()
            ]
        );
    }

    #[test]
    fn rejects_more_tools_than_the_bound_allows() {
        let too_many = (0..=MAX_ALLOWED_TOOLS)
            .map(|index| format!("\"ns.tool_{index}\""))
            .collect::<Vec<_>>()
            .join(", ");
        assert_eq!(
            parse_manifest(&v3_with_tools(&format!("[{too_many}]"))),
            Err(ManifestError::InvalidField("allowed_tools"))
        );
    }

    #[test]
    fn validate_rejects_a_hand_built_manifest_that_bypasses_the_parser() {
        // parse_manifest sorts and dedups, so these states are only reachable
        // by constructing the struct directly -- which consumers can do.
        let mut manifest = parse_manifest(V3).unwrap();
        manifest.allowed_tools = vec!["context.append_entry".into(), "artifact.write".into()];
        assert_eq!(
            manifest.validate(),
            Err(ManifestError::InvalidField("allowed_tools"))
        );

        let mut duplicated = parse_manifest(V3).unwrap();
        duplicated.allowed_tools = vec!["artifact.write".into(), "artifact.write".into()];
        assert_eq!(
            duplicated.validate(),
            Err(ManifestError::InvalidField("allowed_tools"))
        );

        let mut downgraded = parse_manifest(V3).unwrap();
        downgraded.version = MANIFEST_V2_VERSION;
        assert_eq!(
            downgraded.validate(),
            Err(ManifestError::InvalidField("allowed_tools"))
        );
    }

    #[test]
    fn validate_rejects_each_malformed_field() {
        // These branches guard the fields `validate_manifest` walks after the
        // version gate. They were previously unexercised, which meant a typo in
        // any one of them would have failed open.
        let cases: [(&str, &str, &str); 6] = [
            (
                r#""capabilities": []"#,
                r#""vault_access": {"secrets": [""], "mode": "leased", "max_lease_ttl": 300}, "capabilities": []"#,
                "vault_access.secrets",
            ),
            (
                r#""capabilities": []"#,
                r#""vault_access": {"secrets": ["k"], "mode": "borrowed", "max_lease_ttl": 300}, "capabilities": []"#,
                "vault_access.mode",
            ),
            (
                r#""capabilities": []"#,
                r#""capabilities": [{"category": "net", "action": "connect", "target": "", "justification": "Reaches the forecast service."}]"#,
                "capabilities[].target",
            ),
            (
                r#""capabilities": []"#,
                r#""capabilities": [{"category": "net", "action": "connect", "target": "a:1", "justification": "short"}]"#,
                "capabilities[].justification",
            ),
            (
                r#""capabilities": []"#,
                r#""capabilities": [], "restart_policy": "sometimes""#,
                "restart_policy",
            ),
            (
                r#""justification": "Uses only encrypted channels and no operating-system access.""#,
                r#""justification": "too short""#,
                "justification",
            ),
        ];
        for (from, to, field) in cases {
            let source = MINIMAL.replace(from, to);
            assert_ne!(source, MINIMAL, "fixture substitution missed for {field}");
            assert_eq!(
                parse_manifest(&source),
                Err(ManifestError::InvalidField(field)),
                "should have rejected {field}"
            );
        }
    }

    #[test]
    fn validate_rejects_an_incomplete_schema_version_map() {
        // A v2 manifest whose declared channels and version map disagree.
        let mut manifest = parse_manifest(CURRENT).unwrap();
        manifest.message_schema_versions.remove("weather-reports");
        assert_eq!(
            manifest.validate(),
            Err(ManifestError::InvalidField("message_schema_versions"))
        );
    }

    #[test]
    fn to_json_refuses_to_silently_drop_a_tool_surface() {
        // manifest_json emits allowed_tools only at v3. Without validating
        // first, rendering a v3 manifest whose version had been set back to v2
        // produced JSON with NO tool surface and no error -- a signed artifact
        // authorizing something other than what its author declared. The
        // realistic path is a generator that builds the struct, populates
        // tools, and leaves version at the v2 constant.
        let mut manifest = parse_manifest(V3).unwrap();
        assert!(!manifest.allowed_tools.is_empty());
        manifest.version = MANIFEST_V2_VERSION;
        assert_eq!(
            manifest.to_json(),
            Err(ManifestError::InvalidField("allowed_tools"))
        );
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
            parse_manifest(&MINIMAL.replace("\"version\": 1", "\"version\": 4")),
            Err(ManifestError::UnsupportedVersion(4))
        );
        assert_eq!(
            parse_manifest(&MINIMAL.replace("\"version\": 1", "\"version\": 1.0")),
            Err(ManifestError::InvalidField("version"))
        );
        assert_eq!(parse_manifest("{"), Err(ManifestError::InvalidJson));
    }

    #[test]
    fn version_contracts_fail_closed() {
        let v1_with_v2_binding =
            MINIMAL.replace("[\"weather-requests\"]", "{\"weather-requests\": 1}");
        assert_eq!(
            parse_manifest(&v1_with_v2_binding),
            Err(ManifestError::InvalidField("channels.reads"))
        );
        assert_eq!(
            parse_manifest(&CURRENT.replace("\"weather-reports\": 2", "\"Bad_Channel\": 2")),
            Err(ManifestError::InvalidField("channels.writes"))
        );
        assert_eq!(
            parse_manifest(&CURRENT.replace("\"weather-reports\": 2", "\"weather-reports\": 0")),
            Err(ManifestError::InvalidField("channels.writes"))
        );
        assert_eq!(
            parse_manifest(&CURRENT.replace(
                "\"weather-requests\": 1",
                "\"weather-requests\": 1, \"weather-requests\": 2"
            )),
            Err(ManifestError::InvalidField("channels.reads"))
        );
        assert_eq!(
            parse_manifest(
                &CURRENT.replace("\"weather-reports\": 2", "\"weather-reports\": \"2\"")
            ),
            Err(ManifestError::InvalidField("channels.writes"))
        );
    }

    #[test]
    fn channel_compatibility_requires_matching_declared_versions() {
        let originator = parse_manifest(CURRENT).unwrap();
        let receiver = parse_manifest(
            r#"{
              "version": 2,
              "agent": "display-agent",
              "description": "Displays one concise local weather forecast.",
              "privilege_tier": 0,
              "channels": {
                "reads": {"weather-reports": 2},
                "writes": {}
              },
              "capabilities": [],
              "justification": "Uses only the declared encrypted weather channel."
            }"#,
        )
        .unwrap();
        assert_eq!(
            require_channel_compatibility(&originator, &receiver, "weather-reports"),
            Ok(())
        );
        assert_eq!(
            require_channel_compatibility(&originator, &receiver, "weather-requests"),
            Err(ChannelCompatibilityError::OriginatorDoesNotWrite(
                "weather-requests".to_string()
            ))
        );
        let unrelated_receiver = parse_manifest(
            r#"{
              "version": 2,
              "agent": "unrelated-agent",
              "description": "Consumes no channels in this compatibility test.",
              "privilege_tier": 0,
              "channels": {"reads": {}, "writes": {}},
              "capabilities": [],
              "justification": "Uses no channels or operating-system capabilities."
            }"#,
        )
        .unwrap();
        assert_eq!(
            require_channel_compatibility(&originator, &unrelated_receiver, "weather-reports"),
            Err(ChannelCompatibilityError::ReceiverDoesNotRead(
                "weather-reports".to_string()
            ))
        );

        let incompatible = parse_manifest(
            &receiver
                .to_json()
                .unwrap()
                .replace("\"weather-reports\": 2", "\"weather-reports\": 1"),
        )
        .unwrap();
        assert_eq!(
            require_channel_compatibility(&originator, &incompatible, "weather-reports"),
            Err(ChannelCompatibilityError::SchemaVersionMismatch {
                channel: "weather-reports".to_string(),
                originator: 2,
                receiver: 1,
            })
        );

        let legacy = parse_manifest(&MINIMAL.replace(
            "\"reads\": [\"weather-requests\"], \"writes\": [\"weather-reports\"]",
            "\"reads\": [\"weather-reports\"], \"writes\": []",
        ))
        .unwrap();
        assert_eq!(
            require_channel_compatibility(&originator, &legacy, "weather-reports"),
            Err(ChannelCompatibilityError::ReceiverSchemaUndeclared(
                "weather-reports".to_string()
            ))
        );
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
