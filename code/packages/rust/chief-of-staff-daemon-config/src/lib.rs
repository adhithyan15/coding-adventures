//! Strict typed TOML configuration for the D18 Chief daemon.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_toml_parser::{try_parse_toml, TomlParseError};
use core::fmt::{self, Display, Formatter};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use std::collections::{BTreeMap, BTreeSet};
use std::net::IpAddr;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

const ORCHESTRATOR: &[&str] = &["orchestrator"];
const KEYRING: &[&str] = &["keyring"];
const HOST_DEFAULTS: &[&str] = &["hosts", "defaults"];
const VAULT: &[&str] = &["vault"];
const PRIVILEGE: &[&str] = &["privilege"];
const MAX_CONFIG_BYTES: usize = 256 * 1024;
const MAX_PATH_BYTES: usize = 4096;
const MAX_TRUSTED_KEYS: usize = 256;

/// Stable payload-blind configuration failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigError {
    /// TOML tokenization or syntax failed.
    Toml(TomlParseError),
    /// Array-of-table syntax is not part of the D18 configuration schema.
    UnsupportedArrayTable,
    /// A table, key, or inline-table field was declared more than once.
    Duplicate,
    /// A required table or field was absent.
    Missing,
    /// The document contained a table or field outside the closed schema.
    Unknown,
    /// A field used the wrong TOML value kind.
    InvalidType,
    /// A field value violated a bounded domain invariant.
    InvalidValue,
    /// The orchestrator bind address was not a loopback IP address.
    NonLoopbackBind,
    /// A configured path was neither absolute nor explicitly home-relative.
    UnsafePath,
    /// The caller supplied an invalid home directory for path resolution.
    InvalidHome,
}

impl Display for ConfigError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Toml(_) => "chief config: malformed TOML",
            Self::UnsupportedArrayTable => "chief config: array tables are unsupported",
            Self::Duplicate => "chief config: duplicate declaration",
            Self::Missing => "chief config: required declaration missing",
            Self::Unknown => "chief config: unknown declaration",
            Self::InvalidType => "chief config: invalid value type",
            Self::InvalidValue => "chief config: invalid value",
            Self::NonLoopbackBind => "chief config: bind address is not loopback",
            Self::UnsafePath => "chief config: unsafe path",
            Self::InvalidHome => "chief config: invalid home directory",
        })
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Toml(error) => Some(error),
            _ => None,
        }
    }
}

impl From<TomlParseError> for ConfigError {
    fn from(error: TomlParseError) -> Self {
        Self::Toml(error)
    }
}

/// An absolute or `~/`-relative configuration path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConfigPath(String);

impl ConfigPath {
    fn parse(value: String) -> Result<Self, ConfigError> {
        let relative = value.strip_prefix("~/");
        let candidate = relative.unwrap_or(&value);
        if value.is_empty()
            || value.len() > MAX_PATH_BYTES
            || (!Path::new(&value).is_absolute() && relative.is_none())
            || candidate.is_empty()
            || has_unsafe_components(Path::new(candidate))
        {
            return Err(ConfigError::UnsafePath);
        }
        Ok(Self(value))
    }

    /// Return the exact validated configuration spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Resolve this path using an explicit absolute home directory.
    pub fn resolve(&self, home: &Path) -> Result<PathBuf, ConfigError> {
        if !home.is_absolute() || has_unsafe_components(home) {
            return Err(ConfigError::InvalidHome);
        }
        match self.0.strip_prefix("~/") {
            Some(relative) => Ok(home.join(relative)),
            None => Ok(PathBuf::from(&self.0)),
        }
    }
}

fn has_unsafe_components(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
}

/// Package-signing trust class from the D18 configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrustedKeyType {
    /// A production signing key.
    Production,
    /// A local developer signing key.
    Developer,
}

/// One unique trusted package-signing key declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustedKey {
    id: String,
    path: ConfigPath,
    key_type: TrustedKeyType,
}

impl TrustedKey {
    /// Return the stable operator-assigned key identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Return the public-key path.
    pub fn path(&self) -> &ConfigPath {
        &self.path
    }

    /// Return the configured trust class.
    pub fn key_type(&self) -> TrustedKeyType {
        self.key_type
    }
}

/// Validated orchestrator listener and package-root settings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrchestratorConfig {
    bind: IpAddr,
    packages_dir: ConfigPath,
}

impl OrchestratorConfig {
    /// Return the loopback-only listener IP.
    pub fn bind(&self) -> IpAddr {
        self.bind
    }

    /// Return the package installation root.
    pub fn packages_dir(&self) -> &ConfigPath {
        &self.packages_dir
    }
}

/// Validated package-signing trust configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyringConfig {
    trusted_keys: Vec<TrustedKey>,
}

impl KeyringConfig {
    /// Return non-empty unique trusted key declarations in source order.
    pub fn trusted_keys(&self) -> &[TrustedKey] {
        &self.trusted_keys
    }
}

/// Host restart behavior promised by D18.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostRestartPolicy {
    /// Restart after every exit.
    Always,
    /// Restart only after unsuccessful exit.
    OnFailure,
    /// Never restart automatically.
    Never,
}

/// Default lifecycle policy for registered hosts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostDefaultsConfig {
    restart_policy: HostRestartPolicy,
    health_check_interval: Duration,
}

impl HostDefaultsConfig {
    /// Return the default restart policy.
    pub fn restart_policy(self) -> HostRestartPolicy {
        self.restart_policy
    }

    /// Return the non-zero health-check interval.
    pub fn health_check_interval(self) -> Duration {
        self.health_check_interval
    }
}

/// Validated vault coordination settings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VaultConfig {
    storage_path: ConfigPath,
    default_lease_ttl: Duration,
    container: bool,
}

impl VaultConfig {
    /// Return the vault storage root.
    pub fn storage_path(&self) -> &ConfigPath {
        &self.storage_path
    }

    /// Return the non-zero default lease duration.
    pub fn default_lease_ttl(&self) -> Duration {
        self.default_lease_ttl
    }

    /// Return whether the vault must run in its OS containment boundary.
    pub fn container(&self) -> bool {
        self.container
    }
}

/// Validated privilege-interaction deadlines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrivilegeConfig {
    tier_1_auto_approve_timeout: Duration,
    biometric_timeout: Duration,
    hardware_key_timeout: Duration,
}

impl PrivilegeConfig {
    /// Return the non-zero Tier 1 auto-approval timeout.
    pub fn tier_1_auto_approve_timeout(self) -> Duration {
        self.tier_1_auto_approve_timeout
    }

    /// Return the non-zero biometric interaction timeout.
    pub fn biometric_timeout(self) -> Duration {
        self.biometric_timeout
    }

    /// Return the non-zero hardware-key interaction timeout.
    pub fn hardware_key_timeout(self) -> Duration {
        self.hardware_key_timeout
    }
}

/// Complete validated D18 Chief daemon configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChiefConfig {
    orchestrator: OrchestratorConfig,
    keyring: KeyringConfig,
    host_defaults: HostDefaultsConfig,
    vault: VaultConfig,
    privilege: PrivilegeConfig,
}

impl ChiefConfig {
    /// Return listener and package-root settings.
    pub fn orchestrator(&self) -> &OrchestratorConfig {
        &self.orchestrator
    }

    /// Return package-signing trust settings.
    pub fn keyring(&self) -> &KeyringConfig {
        &self.keyring
    }

    /// Return default host lifecycle settings.
    pub fn host_defaults(&self) -> HostDefaultsConfig {
        self.host_defaults
    }

    /// Return vault coordination settings.
    pub fn vault(&self) -> &VaultConfig {
        &self.vault
    }

    /// Return privilege-interaction deadlines.
    pub fn privilege(&self) -> PrivilegeConfig {
        self.privilege
    }
}

/// Parse and fully validate one D18 Chief TOML document.
pub fn parse_config(source: &str) -> Result<ChiefConfig, ConfigError> {
    if source.len() > MAX_CONFIG_BYTES {
        return Err(ConfigError::InvalidValue);
    }
    let ast = try_parse_toml(source)?;
    let mut document = RawDocument::from_ast(&ast)?;
    document.validate_tables()?;

    let bind = expect_string(document.take(ORCHESTRATOR, "bind")?)?
        .parse::<IpAddr>()
        .map_err(|_| ConfigError::InvalidValue)?;
    if !bind.is_loopback() {
        return Err(ConfigError::NonLoopbackBind);
    }
    let packages_dir =
        ConfigPath::parse(expect_string(document.take(ORCHESTRATOR, "packages_dir")?)?)?;
    let trusted_keys = parse_trusted_keys(document.take(KEYRING, "trusted_keys")?)?;
    let restart_policy = parse_restart_policy(document.take(HOST_DEFAULTS, "restart_policy")?)?;
    let health_check_interval =
        positive_millis(document.take(HOST_DEFAULTS, "health_check_interval")?)?;
    let storage_path = ConfigPath::parse(expect_string(document.take(VAULT, "storage_path")?)?)?;
    let default_lease_ttl = positive_secs(document.take(VAULT, "default_lease_ttl")?)?;
    let container = expect_bool(document.take(VAULT, "container")?)?;
    let tier_1_auto_approve_timeout =
        positive_secs(document.take(PRIVILEGE, "tier_1_auto_approve_timeout")?)?;
    let biometric_timeout = positive_secs(document.take(PRIVILEGE, "biometric_timeout")?)?;
    let hardware_key_timeout = positive_secs(document.take(PRIVILEGE, "hardware_key_timeout")?)?;
    if !document.fields.is_empty() {
        return Err(ConfigError::Unknown);
    }

    Ok(ChiefConfig {
        orchestrator: OrchestratorConfig { bind, packages_dir },
        keyring: KeyringConfig { trusted_keys },
        host_defaults: HostDefaultsConfig {
            restart_policy,
            health_check_interval,
        },
        vault: VaultConfig {
            storage_path,
            default_lease_ttl,
            container,
        },
        privilege: PrivilegeConfig {
            tier_1_auto_approve_timeout,
            biometric_timeout,
            hardware_key_timeout,
        },
    })
}

fn parse_restart_policy(value: RawValue) -> Result<HostRestartPolicy, ConfigError> {
    match expect_string(value)?.as_str() {
        "always" => Ok(HostRestartPolicy::Always),
        "on-failure" => Ok(HostRestartPolicy::OnFailure),
        "never" => Ok(HostRestartPolicy::Never),
        _ => Err(ConfigError::InvalidValue),
    }
}

fn parse_trusted_keys(value: RawValue) -> Result<Vec<TrustedKey>, ConfigError> {
    let RawValue::Array(values) = value else {
        return Err(ConfigError::InvalidType);
    };
    if values.is_empty() || values.len() > MAX_TRUSTED_KEYS {
        return Err(ConfigError::InvalidValue);
    }
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut keys = Vec::with_capacity(values.len());
    for value in values {
        let RawValue::InlineTable(mut fields) = value else {
            return Err(ConfigError::InvalidType);
        };
        let id = expect_string(take_inline(&mut fields, "id")?)?;
        if id.is_empty()
            || id.len() > 128
            || !id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        {
            return Err(ConfigError::InvalidValue);
        }
        let path = ConfigPath::parse(expect_string(take_inline(&mut fields, "path")?)?)?;
        let key_type = match expect_string(take_inline(&mut fields, "type")?)?.as_str() {
            "production" => TrustedKeyType::Production,
            "developer" => TrustedKeyType::Developer,
            _ => return Err(ConfigError::InvalidValue),
        };
        if !fields.is_empty() {
            return Err(ConfigError::Unknown);
        }
        if !ids.insert(id.clone()) || !paths.insert(path.clone()) {
            return Err(ConfigError::Duplicate);
        }
        keys.push(TrustedKey { id, path, key_type });
    }
    Ok(keys)
}

fn take_inline(
    fields: &mut BTreeMap<Vec<String>, RawValue>,
    key: &str,
) -> Result<RawValue, ConfigError> {
    fields
        .remove(&vec![key.to_string()])
        .ok_or(ConfigError::Missing)
}

fn positive_millis(value: RawValue) -> Result<Duration, ConfigError> {
    positive_integer(value).map(Duration::from_millis)
}

fn positive_secs(value: RawValue) -> Result<Duration, ConfigError> {
    positive_integer(value).map(Duration::from_secs)
}

fn positive_integer(value: RawValue) -> Result<u64, ConfigError> {
    let RawValue::Integer(value) = value else {
        return Err(ConfigError::InvalidType);
    };
    u64::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or(ConfigError::InvalidValue)
}

fn expect_string(value: RawValue) -> Result<String, ConfigError> {
    match value {
        RawValue::String(value) => Ok(value),
        _ => Err(ConfigError::InvalidType),
    }
}

fn expect_bool(value: RawValue) -> Result<bool, ConfigError> {
    match value {
        RawValue::Boolean(value) => Ok(value),
        _ => Err(ConfigError::InvalidType),
    }
}

#[derive(Clone, Debug, PartialEq)]
enum RawValue {
    String(String),
    Integer(i64),
    Boolean(bool),
    Array(Vec<Self>),
    InlineTable(BTreeMap<Vec<String>, Self>),
    Other,
}

struct RawDocument {
    fields: BTreeMap<Vec<String>, RawValue>,
    tables: BTreeSet<Vec<String>>,
}

impl RawDocument {
    fn from_ast(ast: &GrammarASTNode) -> Result<Self, ConfigError> {
        let mut document = Self {
            fields: BTreeMap::new(),
            tables: BTreeSet::new(),
        };
        let mut current_table = Vec::new();
        for child in &ast.children {
            let ASTNodeOrToken::Node(expression) = child else {
                continue;
            };
            let Some(node) = first_child_node(expression) else {
                continue;
            };
            match node.rule_name.as_str() {
                "table_header" => {
                    current_table = key_from_container(node)?;
                    if !document.tables.insert(current_table.clone()) {
                        return Err(ConfigError::Duplicate);
                    }
                }
                "array_table_header" => return Err(ConfigError::UnsupportedArrayTable),
                "keyval" => {
                    let (mut key, value) = parse_keyval(node)?;
                    let mut full_key = current_table.clone();
                    full_key.append(&mut key);
                    if document.fields.insert(full_key, value).is_some() {
                        return Err(ConfigError::Duplicate);
                    }
                }
                _ => return Err(ConfigError::InvalidValue),
            }
        }
        Ok(document)
    }

    fn validate_tables(&self) -> Result<(), ConfigError> {
        let allowed = [ORCHESTRATOR, KEYRING, HOST_DEFAULTS, VAULT, PRIVILEGE]
            .into_iter()
            .map(strings_to_vec)
            .collect::<BTreeSet<_>>();
        if self.tables == allowed {
            Ok(())
        } else if self.tables.iter().any(|table| !allowed.contains(table)) {
            Err(ConfigError::Unknown)
        } else {
            Err(ConfigError::Missing)
        }
    }

    fn take(&mut self, table: &[&str], field: &str) -> Result<RawValue, ConfigError> {
        let mut key = strings_to_vec(table);
        key.push(field.to_string());
        self.fields.remove(&key).ok_or(ConfigError::Missing)
    }
}

fn strings_to_vec(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| (*part).to_string()).collect()
}

fn first_child_node(node: &GrammarASTNode) -> Option<&GrammarASTNode> {
    node.children.iter().find_map(|child| match child {
        ASTNodeOrToken::Node(node) => Some(node),
        ASTNodeOrToken::Token(_) => None,
    })
}

fn key_from_container(node: &GrammarASTNode) -> Result<Vec<String>, ConfigError> {
    node.children
        .iter()
        .find_map(|child| match child {
            ASTNodeOrToken::Node(node) if node.rule_name == "key" => Some(parse_key(node)),
            _ => None,
        })
        .unwrap_or(Err(ConfigError::InvalidValue))
}

fn parse_key(node: &GrammarASTNode) -> Result<Vec<String>, ConfigError> {
    let keys = node
        .children
        .iter()
        .filter_map(|child| match child {
            ASTNodeOrToken::Node(simple) if simple.rule_name == "simple_key" => {
                simple.token().map(|token| token.value.clone())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if keys.is_empty() {
        Err(ConfigError::InvalidValue)
    } else {
        Ok(keys)
    }
}

fn parse_keyval(node: &GrammarASTNode) -> Result<(Vec<String>, RawValue), ConfigError> {
    let mut key = None;
    let mut value = None;
    for child in &node.children {
        if let ASTNodeOrToken::Node(child) = child {
            match child.rule_name.as_str() {
                "key" => key = Some(parse_key(child)?),
                "value" => value = Some(parse_value(child)?),
                _ => {}
            }
        }
    }
    Ok((
        key.ok_or(ConfigError::InvalidValue)?,
        value.ok_or(ConfigError::InvalidValue)?,
    ))
}

fn parse_value(node: &GrammarASTNode) -> Result<RawValue, ConfigError> {
    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(token) => {
                return Ok(match token.effective_type_name() {
                    "BASIC_STRING" | "ML_BASIC_STRING" | "LITERAL_STRING" | "ML_LITERAL_STRING" => {
                        RawValue::String(token.value.clone())
                    }
                    "INTEGER" => RawValue::Integer(parse_toml_integer(&token.value)?),
                    "TRUE" => RawValue::Boolean(true),
                    "FALSE" => RawValue::Boolean(false),
                    _ => RawValue::Other,
                });
            }
            ASTNodeOrToken::Node(child) if child.rule_name == "array" => {
                return parse_array(child).map(RawValue::Array);
            }
            ASTNodeOrToken::Node(child) if child.rule_name == "inline_table" => {
                return parse_inline_table(child).map(RawValue::InlineTable);
            }
            ASTNodeOrToken::Node(_) => {}
        }
    }
    Err(ConfigError::InvalidValue)
}

fn parse_toml_integer(value: &str) -> Result<i64, ConfigError> {
    let compact = value.replace('_', "");
    let (negative, unsigned) = match compact.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, compact.strip_prefix('+').unwrap_or(&compact)),
    };
    let (radix, digits) = if let Some(rest) = unsigned.strip_prefix("0x") {
        (16, rest)
    } else if let Some(rest) = unsigned.strip_prefix("0o") {
        (8, rest)
    } else if let Some(rest) = unsigned.strip_prefix("0b") {
        (2, rest)
    } else {
        (10, unsigned)
    };
    let magnitude = i128::from_str_radix(digits, radix).map_err(|_| ConfigError::InvalidValue)?;
    let signed = if negative { -magnitude } else { magnitude };
    i64::try_from(signed).map_err(|_| ConfigError::InvalidValue)
}

fn parse_array(node: &GrammarASTNode) -> Result<Vec<RawValue>, ConfigError> {
    let Some(values) = node.children.iter().find_map(|child| match child {
        ASTNodeOrToken::Node(node) if node.rule_name == "array_values" => Some(node),
        _ => None,
    }) else {
        return Err(ConfigError::InvalidValue);
    };
    values
        .children
        .iter()
        .filter_map(|child| match child {
            ASTNodeOrToken::Node(node) if node.rule_name == "value" => Some(parse_value(node)),
            _ => None,
        })
        .collect()
}

fn parse_inline_table(
    node: &GrammarASTNode,
) -> Result<BTreeMap<Vec<String>, RawValue>, ConfigError> {
    let mut fields = BTreeMap::new();
    for child in &node.children {
        if let ASTNodeOrToken::Node(child) = child {
            if child.rule_name == "keyval" {
                let (key, value) = parse_keyval(child)?;
                if fields.insert(key, value).is_some() {
                    return Err(ConfigError::Duplicate);
                }
            }
        }
    }
    Ok(fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"
[orchestrator]
bind = "127.0.0.1"
packages_dir = "~/.chief-of-staff/agents/"

[keyring]
trusted_keys = [
  { id = "prod-001", path = "~/.chief-of-staff/keys/prod-001.pub", type = "production" },
  { id = "dev-local", path = "~/.chief-of-staff/keys/dev.pub", type = "developer" },
]

[hosts.defaults]
restart_policy = "on-failure"
health_check_interval = 5_000

[vault]
storage_path = "~/.chief-of-staff/vault/"
default_lease_ttl = 30
container = true

[privilege]
tier_1_auto_approve_timeout = 5
biometric_timeout = 30
hardware_key_timeout = 60
"#;

    #[test]
    fn parses_the_complete_spec_schema_into_typed_values() {
        let config = parse_config(VALID).expect("valid config");
        assert_eq!(
            config.orchestrator().bind(),
            "127.0.0.1".parse::<IpAddr>().unwrap()
        );
        assert_eq!(
            config.orchestrator().packages_dir().as_str(),
            "~/.chief-of-staff/agents/"
        );
        assert_eq!(config.keyring().trusted_keys().len(), 2);
        assert_eq!(config.keyring().trusted_keys()[0].id(), "prod-001");
        assert_eq!(
            config.keyring().trusted_keys()[0].key_type(),
            TrustedKeyType::Production
        );
        assert_eq!(
            config.host_defaults().restart_policy(),
            HostRestartPolicy::OnFailure
        );
        assert_eq!(
            config.host_defaults().health_check_interval(),
            Duration::from_secs(5)
        );
        assert_eq!(config.vault().default_lease_ttl(), Duration::from_secs(30));
        assert!(config.vault().container());
        assert_eq!(
            config.privilege().hardware_key_timeout(),
            Duration::from_secs(60)
        );
    }

    #[test]
    fn resolves_home_paths_only_from_an_explicit_safe_home() {
        let config = parse_config(VALID).unwrap();
        let home = absolute_home();
        assert_eq!(
            config.orchestrator().packages_dir().resolve(&home).unwrap(),
            home.join(".chief-of-staff/agents/")
        );
        assert_eq!(
            config.keyring().trusted_keys()[1]
                .path()
                .resolve(&home)
                .unwrap(),
            home.join(".chief-of-staff/keys/dev.pub")
        );
        assert_eq!(
            config.vault().storage_path().resolve(Path::new("relative")),
            Err(ConfigError::InvalidHome)
        );
    }

    #[test]
    fn malformed_duplicate_missing_and_unknown_documents_fail_closed() {
        assert!(matches!(
            parse_config("[orchestrator\nbind = 1"),
            Err(ConfigError::Toml(_))
        ));
        assert_eq!(
            parse_config(&VALID.replace(
                "bind = \"127.0.0.1\"",
                "bind = \"127.0.0.1\"\nbind = \"127.0.0.1\""
            )),
            Err(ConfigError::Duplicate)
        );
        assert_eq!(
            parse_config(&VALID.replace("packages_dir = \"~/.chief-of-staff/agents/\"\n", "")),
            Err(ConfigError::Missing)
        );
        assert_eq!(
            parse_config(&VALID.replace("container = true", "container = true\nsurprise = true")),
            Err(ConfigError::Unknown)
        );
        assert_eq!(
            parse_config(&format!("{VALID}\n[extra]\nvalue = true\n")),
            Err(ConfigError::Unknown)
        );
    }

    #[test]
    fn listener_paths_and_positive_durations_enforce_security_invariants() {
        assert_eq!(
            parse_config(&VALID.replace("127.0.0.1", "0.0.0.0")),
            Err(ConfigError::NonLoopbackBind)
        );
        assert_eq!(
            parse_config(&VALID.replace("127.0.0.1", "localhost")),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            parse_config(&VALID.replace("~/.chief-of-staff/agents/", "relative/agents")),
            Err(ConfigError::UnsafePath)
        );
        assert_eq!(
            parse_config(&VALID.replace("~/.chief-of-staff/agents/", "~/../escape")),
            Err(ConfigError::UnsafePath)
        );
        assert_eq!(
            parse_config(
                &VALID.replace("health_check_interval = 5_000", "health_check_interval = 0")
            ),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            parse_config(&VALID.replace("default_lease_ttl = 30", "default_lease_ttl = true")),
            Err(ConfigError::InvalidType)
        );
    }

    #[test]
    fn trusted_keys_are_nonempty_unique_and_closed_typed_records() {
        assert_eq!(
            parse_config(&VALID.replace(
                "  { id = \"dev-local\", path = \"~/.chief-of-staff/keys/dev.pub\", type = \"developer\" },",
                "  { id = \"prod-001\", path = \"~/.chief-of-staff/keys/dev.pub\", type = \"developer\" },"
            )),
            Err(ConfigError::Duplicate)
        );
        assert_eq!(
            parse_config(&VALID.replace("id = \"dev-local\"", "id = \"bad id\"")),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            parse_config(&VALID.replace("type = \"developer\"", "type = \"unknown\"")),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            parse_config(
                &VALID.replace("type = \"developer\"", "type = \"developer\", extra = true")
            ),
            Err(ConfigError::Unknown)
        );
        let empty = VALID.replace(
            "trusted_keys = [\n  { id = \"prod-001\", path = \"~/.chief-of-staff/keys/prod-001.pub\", type = \"production\" },\n  { id = \"dev-local\", path = \"~/.chief-of-staff/keys/dev.pub\", type = \"developer\" },\n]",
            "trusted_keys = []",
        );
        assert_eq!(parse_config(&empty), Err(ConfigError::InvalidValue));
    }

    #[test]
    fn restart_policies_and_integer_spellings_are_bounded() {
        for (spelling, expected) in [
            ("always", HostRestartPolicy::Always),
            ("on-failure", HostRestartPolicy::OnFailure),
            ("never", HostRestartPolicy::Never),
        ] {
            let source = VALID.replace("on-failure", spelling);
            assert_eq!(
                parse_config(&source)
                    .unwrap()
                    .host_defaults()
                    .restart_policy(),
                expected
            );
        }
        assert_eq!(
            parse_config(&VALID.replace("on-failure", "sometimes")),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            parse_config(&VALID.replace("default_lease_ttl = 30", "default_lease_ttl = 0x1e"))
                .unwrap()
                .vault()
                .default_lease_ttl(),
            Duration::from_secs(30)
        );
    }

    #[test]
    fn unsupported_table_arrays_and_wrong_value_shapes_are_rejected() {
        assert_eq!(
            parse_config(&VALID.replace("[keyring]", "[[keyring]]")),
            Err(ConfigError::UnsupportedArrayTable)
        );
        assert_eq!(
            parse_config(&VALID.replace("trusted_keys = [", "trusted_keys = [\n  true,")),
            Err(ConfigError::InvalidType)
        );
        assert_eq!(
            parse_config(&VALID.replace("container = true", "container = 1.5")),
            Err(ConfigError::InvalidType)
        );
    }

    #[test]
    fn source_paths_and_keyring_width_are_bounded_before_composition() {
        assert_eq!(
            parse_config(&" ".repeat(MAX_CONFIG_BYTES + 1)),
            Err(ConfigError::InvalidValue)
        );
        let long_path = format!("~/{}", "a".repeat(MAX_PATH_BYTES));
        assert_eq!(
            parse_config(&VALID.replace("~/.chief-of-staff/agents/", &long_path)),
            Err(ConfigError::UnsafePath)
        );
        let entry = "{ id = \"key\", path = \"~/key.pub\", type = \"developer\" }";
        let oversized = format!(
            "trusted_keys = [{}]",
            vec![entry; MAX_TRUSTED_KEYS + 1].join(",")
        );
        let start = VALID.find("trusted_keys = [").unwrap();
        let end = VALID[start..].find("]\n").unwrap() + start + 1;
        let source = format!("{}{}{}", &VALID[..start], oversized, &VALID[end..]);
        assert_eq!(parse_config(&source), Err(ConfigError::InvalidValue));
    }

    #[cfg(windows)]
    fn absolute_home() -> PathBuf {
        PathBuf::from(r"C:\Users\example")
    }

    #[cfg(not(windows))]
    fn absolute_home() -> PathBuf {
        PathBuf::from("/Users/example")
    }
}
