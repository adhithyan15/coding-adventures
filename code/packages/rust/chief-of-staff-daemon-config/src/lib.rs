//! Strict typed TOML configuration for the D18 Chief daemon.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_trust_checker::{
    TIER_1_AUTO_APPROVE_TIMEOUT, TIER_2_BIOMETRIC_TIMEOUT, TIER_3_HARDWARE_KEY_TIMEOUT,
};
use coding_adventures_toml_parser::{try_parse_toml, TomlParseError};
use core::fmt::{self, Display, Formatter};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use std::collections::{BTreeMap, BTreeSet};
use std::net::{IpAddr, SocketAddr};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

const ORCHESTRATOR: &[&str] = &["orchestrator"];
const KEYRING: &[&str] = &["keyring"];
const HOST_DEFAULTS: &[&str] = &["hosts", "defaults"];
const VAULT: &[&str] = &["vault"];
const PRIVILEGE: &[&str] = &["privilege"];
const DATA_PLANE: &[&str] = &["data_plane"];
const SMART_HOME: &[&str] = &["smart_home"];
const MAX_CONFIG_BYTES: usize = 256 * 1024;
const MAX_PATH_BYTES: usize = 4096;
const MAX_TRUSTED_KEYS: usize = 256;
const MAX_PRIVILEGE_ASSIGNMENTS: usize = 4096;
const MAX_CHANNEL_KEYS: usize = 1024;
const MAX_OLLAMA_MODELS: usize = 256;
const MAX_SMART_HOME_TOOL_GRANTS: usize = 4096;
const MAX_AGENT_ID_BYTES: usize = 4 * 1024;
const MAX_GRANT_ID_BYTES: usize = 4 * 1024;
const MAX_GRANTED_BY_BYTES: usize = 4 * 1024;
const MAX_TOOL_ID_BYTES: usize = 512;
const MAX_SMART_HOME_INSTANCE_NAME_BYTES: usize = 200;
const MAX_NETWORK_INTERFACE_BYTES: usize = 255;
const MAX_PAIRING_SECRET_BYTES: u64 = 4 * 1024;
const MAX_MODEL_BYTES: usize = 200;
const MAX_ENDPOINT_BYTES: usize = 512;
const MAX_PROCESS_TIMEOUT_MILLIS: u64 = 5 * 60 * 1000;

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
    port: u16,
    packages_dir: ConfigPath,
    state_dir: ConfigPath,
    credential_path: ConfigPath,
}

/// Optional loopback Home Assistant-compatible listener owned by Chief.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SmartHomeListenerConfig {
    bind: IpAddr,
    port: u16,
    instance_name: String,
    hue_mdns_interface: Option<String>,
    hue_pairing_kek_path: Option<ConfigPath>,
    onvif_pairing: Option<OnvifPairingConfig>,
    axis_pairing: Option<AxisPairingConfig>,
    zoneminder_pairing: Option<ZoneMinderPairingConfig>,
    reolink_pairing: Option<ReolinkPairingConfig>,
    synology_pairing: Option<SynologyPairingConfig>,
}

/// Exact owner-provisioned inputs for one supervised Reolink pairing worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReolinkPairingConfig {
    bridge_id: String,
    canonical_host: String,
    pinned_address: SocketAddr,
    kek_path: ConfigPath,
    username_path: ConfigPath,
    username_length: usize,
    password_path: ConfigPath,
    password_length: usize,
}

/// Exact owner-provisioned inputs for one supervised Synology pairing worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SynologyPairingConfig {
    bridge_id: String,
    canonical_host: String,
    pinned_address: SocketAddr,
    kek_path: ConfigPath,
    username_path: ConfigPath,
    username_length: usize,
    password_path: ConfigPath,
    password_length: usize,
}

impl SynologyPairingConfig {
    /// Return the exact D23 bridge whose credentials may be consumed.
    pub fn bridge_id(&self) -> &str {
        &self.bridge_id
    }
    /// Return the exact host name expected in the installed bridge endpoint.
    pub fn canonical_host(&self) -> &str {
        &self.canonical_host
    }
    /// Return the socket address used without a second DNS lookup.
    pub fn pinned_address(&self) -> SocketAddr {
        self.pinned_address
    }
    /// Return the owner-only KEK file used to initialize or unseal the Vault.
    pub fn kek_path(&self) -> &ConfigPath {
        &self.kek_path
    }
    /// Return the owner-only username file.
    pub fn username_path(&self) -> &ConfigPath {
        &self.username_path
    }
    /// Return the exact username byte length expected from the file.
    pub fn username_length(&self) -> usize {
        self.username_length
    }
    /// Return the owner-only password file.
    pub fn password_path(&self) -> &ConfigPath {
        &self.password_path
    }
    /// Return the exact password byte length expected from the file.
    pub fn password_length(&self) -> usize {
        self.password_length
    }
}

impl ReolinkPairingConfig {
    /// Return the exact D23 bridge whose credentials may be consumed.
    pub fn bridge_id(&self) -> &str {
        &self.bridge_id
    }
    /// Return the exact host name expected in the installed bridge endpoint.
    pub fn canonical_host(&self) -> &str {
        &self.canonical_host
    }
    /// Return the socket address used without a second DNS lookup.
    pub fn pinned_address(&self) -> SocketAddr {
        self.pinned_address
    }
    /// Return the owner-only KEK file used to initialize or unseal the Vault.
    pub fn kek_path(&self) -> &ConfigPath {
        &self.kek_path
    }
    /// Return the owner-only username file.
    pub fn username_path(&self) -> &ConfigPath {
        &self.username_path
    }
    /// Return the exact username byte length expected from the file.
    pub fn username_length(&self) -> usize {
        self.username_length
    }
    /// Return the owner-only password file.
    pub fn password_path(&self) -> &ConfigPath {
        &self.password_path
    }
    /// Return the exact password byte length expected from the file.
    pub fn password_length(&self) -> usize {
        self.password_length
    }
}

/// Exact owner-provisioned inputs for one supervised ZoneMinder pairing worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZoneMinderPairingConfig {
    bridge_id: String,
    kek_path: ConfigPath,
    username_path: ConfigPath,
    username_length: usize,
    password_path: ConfigPath,
    password_length: usize,
}

impl ZoneMinderPairingConfig {
    /// Return the exact D23 bridge whose credentials may be consumed.
    pub fn bridge_id(&self) -> &str {
        &self.bridge_id
    }

    /// Return the owner-only KEK file used to initialize or unseal the Vault.
    pub fn kek_path(&self) -> &ConfigPath {
        &self.kek_path
    }

    /// Return the owner-only username file.
    pub fn username_path(&self) -> &ConfigPath {
        &self.username_path
    }

    /// Return the exact username byte length expected from the file.
    pub fn username_length(&self) -> usize {
        self.username_length
    }

    /// Return the owner-only password file.
    pub fn password_path(&self) -> &ConfigPath {
        &self.password_path
    }

    /// Return the exact password byte length expected from the file.
    pub fn password_length(&self) -> usize {
        self.password_length
    }
}

/// Exact owner-provisioned inputs for one supervised Axis pairing worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AxisPairingConfig {
    bridge_id: String,
    kek_path: ConfigPath,
    username_path: ConfigPath,
    username_length: usize,
    password_path: ConfigPath,
    password_length: usize,
}

impl AxisPairingConfig {
    /// Return the exact D23 bridge whose credentials may be consumed.
    pub fn bridge_id(&self) -> &str {
        &self.bridge_id
    }

    /// Return the owner-only KEK file used to initialize or unseal the Vault.
    pub fn kek_path(&self) -> &ConfigPath {
        &self.kek_path
    }

    /// Return the owner-only username file.
    pub fn username_path(&self) -> &ConfigPath {
        &self.username_path
    }

    /// Return the exact username byte length expected from the file.
    pub fn username_length(&self) -> usize {
        self.username_length
    }

    /// Return the owner-only password file.
    pub fn password_path(&self) -> &ConfigPath {
        &self.password_path
    }

    /// Return the exact password byte length expected from the file.
    pub fn password_length(&self) -> usize {
        self.password_length
    }
}

/// Exact owner-provisioned inputs for one supervised ONVIF pairing worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OnvifPairingConfig {
    bridge_id: String,
    kek_path: ConfigPath,
    username_path: ConfigPath,
    username_length: usize,
    password_path: ConfigPath,
    password_length: usize,
}

impl OnvifPairingConfig {
    /// Return the exact D23 bridge whose credentials may be consumed.
    pub fn bridge_id(&self) -> &str {
        &self.bridge_id
    }

    /// Return the owner-only KEK file used to initialize or unseal the Vault.
    pub fn kek_path(&self) -> &ConfigPath {
        &self.kek_path
    }

    /// Return the owner-only username file.
    pub fn username_path(&self) -> &ConfigPath {
        &self.username_path
    }

    /// Return the exact username byte length expected from the file.
    pub fn username_length(&self) -> usize {
        self.username_length
    }

    /// Return the owner-only password file.
    pub fn password_path(&self) -> &ConfigPath {
        &self.password_path
    }

    /// Return the exact password byte length expected from the file.
    pub fn password_length(&self) -> usize {
        self.password_length
    }
}

impl SmartHomeListenerConfig {
    /// Return the loopback-only listener IP.
    pub fn bind(&self) -> IpAddr {
        self.bind
    }

    /// Return the non-zero TCP listener port.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Return the bounded Home Assistant instance name.
    pub fn instance_name(&self) -> &str {
        &self.instance_name
    }

    /// Return the network interface on which Chief supervises Hue mDNS discovery.
    pub fn hue_mdns_interface(&self) -> Option<&str> {
        self.hue_mdns_interface.as_deref()
    }

    /// Return the owner-only injected KEK file used by the Chief Hue pairing worker.
    pub fn hue_pairing_kek_path(&self) -> Option<&ConfigPath> {
        self.hue_pairing_kek_path.as_ref()
    }

    /// Return the exact owner-provisioned ONVIF pairing worker configuration.
    pub fn onvif_pairing(&self) -> Option<&OnvifPairingConfig> {
        self.onvif_pairing.as_ref()
    }

    /// Return the exact owner-provisioned Axis pairing worker configuration.
    pub fn axis_pairing(&self) -> Option<&AxisPairingConfig> {
        self.axis_pairing.as_ref()
    }

    /// Return the exact owner-provisioned ZoneMinder pairing worker configuration.
    pub fn zoneminder_pairing(&self) -> Option<&ZoneMinderPairingConfig> {
        self.zoneminder_pairing.as_ref()
    }

    /// Return the exact owner-provisioned Reolink pairing worker configuration.
    pub fn reolink_pairing(&self) -> Option<&ReolinkPairingConfig> {
        self.reolink_pairing.as_ref()
    }

    /// Return the exact owner-provisioned Synology pairing worker configuration.
    pub fn synology_pairing(&self) -> Option<&SynologyPairingConfig> {
        self.synology_pairing.as_ref()
    }
}

impl OrchestratorConfig {
    /// Return the loopback-only listener IP.
    pub fn bind(&self) -> IpAddr {
        self.bind
    }

    /// Return the non-zero TCP listener port.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Return the package installation root.
    pub fn packages_dir(&self) -> &ConfigPath {
        &self.packages_dir
    }

    /// Return the durable orchestrator state root.
    pub fn state_dir(&self) -> &ConfigPath {
        &self.state_dir
    }

    /// Return the local operator credential file path.
    pub fn credential_path(&self) -> &ConfigPath {
        &self.credential_path
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostDefaultsConfig {
    restart_policy: HostRestartPolicy,
    health_check_interval: Duration,
    executable: ConfigPath,
    bootstrap_timeout: Duration,
    graceful_stop_timeout: Duration,
    restart_window_ns: u64,
    max_restarts_per_window: u32,
}

impl HostDefaultsConfig {
    /// Return the default restart policy.
    pub fn restart_policy(&self) -> HostRestartPolicy {
        self.restart_policy
    }

    /// Return the non-zero health-check interval.
    pub fn health_check_interval(&self) -> Duration {
        self.health_check_interval
    }

    /// Return the shell-free host runtime executable path.
    pub fn executable(&self) -> &ConfigPath {
        &self.executable
    }

    /// Return the bounded secure-bootstrap deadline.
    pub fn bootstrap_timeout(&self) -> Duration {
        self.bootstrap_timeout
    }

    /// Return the bounded graceful-stop deadline.
    pub fn graceful_stop_timeout(&self) -> Duration {
        self.graceful_stop_timeout
    }

    /// Return the restart-intensity window in nanoseconds (D18R R2).
    ///
    /// Together with [`Self::max_restarts_per_window`] this bounds how often a
    /// single host may be restarted before the reconciler quarantines it
    /// instead. Both keys are optional; omitting them keeps the reconciler's
    /// own defaults, which `restart_intensity_defaults_match_the_reconciler`
    /// in `chief-of-staff-daemon` pins against drift.
    ///
    /// Nanoseconds, not a `Duration`, because that is the unit the reconciler
    /// works in and the conversion is the part that can go wrong. The parser
    /// bounds the accepted value at [`MAX_RESTART_WINDOW_MILLIS`], so this
    /// always fits -- an unbounded window would convert to a saturated
    /// `u64::MAX`, which reads as "N restarts ever" and produces a quarantine
    /// that never lifts, from nothing worse than a typo.
    pub fn restart_window_ns(&self) -> u64 {
        self.restart_window_ns
    }

    /// Return the restarts permitted inside one window (D18R R2).
    pub fn max_restarts_per_window(&self) -> u32 {
        self.max_restarts_per_window
    }
}

/// Sixty seconds, mirroring the reconciler's own default window.
const DEFAULT_RESTART_WINDOW_NS: u64 = 60_000_000_000;
/// Five restarts, mirroring the reconciler's own default budget.
const DEFAULT_MAX_RESTARTS_PER_WINDOW: u32 = 5;

/// One day, the largest restart window an operator may configure.
///
/// The ceiling exists so the millisecond-to-nanosecond conversion cannot
/// overflow, not because a longer window is conceptually wrong. A day is
/// already far past the point where "restarts per window" describes a rate
/// anyone is watching.
pub const MAX_RESTART_WINDOW_MILLIS: u64 = 24 * 60 * 60 * 1000;

/// Return the restart window applied when `[hosts.defaults]` omits the key.
///
/// Exported so that `chief-of-staff-daemon` -- the one crate that depends on
/// both this one and the reconciler -- can pin the two defaults against drift.
pub fn default_restart_window_ns() -> u64 {
    DEFAULT_RESTART_WINDOW_NS
}

/// Return the restart budget applied when `[hosts.defaults]` omits the key.
pub fn default_max_restarts_per_window() -> u32 {
    DEFAULT_MAX_RESTARTS_PER_WINDOW
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrivilegeConfig {
    tier_1_auto_approve_timeout: Duration,
    tier_1_notification_command: Option<ConfigPath>,
    tier_2_biometric_command: Option<ConfigPath>,
    tier_3_hardware_key_command: Option<ConfigPath>,
    biometric_timeout: Duration,
    hardware_key_timeout: Duration,
    agent_tiers: Vec<AgentPrivilegeTierConfig>,
    channel_tiers: Vec<ChannelPrivilegeTierConfig>,
    package_tiers: Vec<PackagePrivilegeTierConfig>,
    model_tiers: Vec<ModelPrivilegeTierConfig>,
}

/// One configured D18 privilege tier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfiguredPrivilegeTier {
    /// No interactive approval is required.
    Tier0,
    /// Notification approval with the canonical timeout policy is required.
    Tier1,
    /// Biometric assurance is required.
    Tier2,
    /// Hardware-key assurance is required.
    Tier3,
}

/// Exact privilege assignment for one agent identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentPrivilegeTierConfig {
    agent_id: Vec<u8>,
    tier: ConfiguredPrivilegeTier,
}

impl AgentPrivilegeTierConfig {
    /// Return the exact decoded agent identity bytes.
    pub fn agent_id(&self) -> &[u8] {
        &self.agent_id
    }

    /// Return the assigned privilege tier.
    pub fn tier(&self) -> ConfiguredPrivilegeTier {
        self.tier
    }
}

/// Exact privilege assignment for one channel UUID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelPrivilegeTierConfig {
    channel_id: [u8; 16],
    tier: ConfiguredPrivilegeTier,
}

impl ChannelPrivilegeTierConfig {
    /// Return the canonical UUID-v7 channel identity.
    pub fn channel_id(&self) -> [u8; 16] {
        self.channel_id
    }

    /// Return the assigned privilege tier.
    pub fn tier(&self) -> ConfiguredPrivilegeTier {
        self.tier
    }
}

/// Exact privilege assignment for one immutable package hash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackagePrivilegeTierConfig {
    package_hash: [u8; 32],
    tier: ConfiguredPrivilegeTier,
}

impl PackagePrivilegeTierConfig {
    /// Return the exact SHA-256 package identity.
    pub fn package_hash(&self) -> [u8; 32] {
        self.package_hash
    }

    /// Return the assigned privilege tier.
    pub fn tier(&self) -> ConfiguredPrivilegeTier {
        self.tier
    }
}

/// Exact privilege assignment for one model selector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelPrivilegeTierConfig {
    model: String,
    tier: ConfiguredPrivilegeTier,
}

impl ModelPrivilegeTierConfig {
    /// Return the exact model selector.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Return the assigned privilege tier.
    pub fn tier(&self) -> ConfiguredPrivilegeTier {
        self.tier
    }
}

/// Direction of one exact channel-key file declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelKeyAccess {
    /// The agent receives from the channel using an X25519 private key.
    Read,
    /// The agent publishes to the channel using an Ed25519 seed and channel master key.
    Write,
}

/// Validated file-backed key material for one exact pipeline, agent, and channel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelKeyConfig {
    pipeline_id: [u8; 16],
    agent_id: String,
    channel_id: [u8; 16],
    access: ChannelKeyAccess,
    receiver_private_key_path: Option<ConfigPath>,
    originator_signing_seed_path: Option<ConfigPath>,
    channel_master_key_path: Option<ConfigPath>,
}

impl ChannelKeyConfig {
    /// Return the canonical UUID-v7 pipeline identity.
    pub fn pipeline_id(&self) -> [u8; 16] {
        self.pipeline_id
    }

    /// Return the exact UTF-8 channel-membership agent identity.
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    /// Return the canonical UUID-v7 channel identity.
    pub fn channel_id(&self) -> [u8; 16] {
        self.channel_id
    }

    /// Return whether this declaration provisions a read or write authority.
    pub fn access(&self) -> ChannelKeyAccess {
        self.access
    }

    /// Return the receiver private-key path for a read declaration.
    pub fn receiver_private_key_path(&self) -> Option<&ConfigPath> {
        self.receiver_private_key_path.as_ref()
    }

    /// Return the originator signing-seed path for a write declaration.
    pub fn originator_signing_seed_path(&self) -> Option<&ConfigPath> {
        self.originator_signing_seed_path.as_ref()
    }

    /// Return the channel-master-key path for a write declaration.
    pub fn channel_master_key_path(&self) -> Option<&ConfigPath> {
        self.channel_master_key_path.as_ref()
    }
}

/// Validated exact Ollama model registration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OllamaModelConfig {
    model: String,
    endpoint: String,
    timeout: Duration,
}

/// Configured lifecycle state for one durable smart-home capability grant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmartHomeToolGrantStatus {
    /// The grant is declared but cannot authorize requests yet.
    Pending,
    /// The grant may authorize requests while its time bounds remain valid.
    Active,
    /// The grant is durably disabled and retained for governance history.
    Revoked,
}

/// One operator-declared, exact Chief-host smart-home tool grant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SmartHomeToolGrantConfig {
    grant_id: String,
    principal_id: String,
    tool_id: String,
    granted_by: String,
    granted_at_ms: u64,
    expires_at_ms: Option<u64>,
    status: SmartHomeToolGrantStatus,
}

impl SmartHomeToolGrantConfig {
    /// Return the stable operator-assigned grant identifier.
    pub fn grant_id(&self) -> &str {
        &self.grant_id
    }

    /// Return the exact Chief host name authorized by this grant.
    pub fn principal_id(&self) -> &str {
        &self.principal_id
    }

    /// Return the exact production smart-home tool identifier.
    pub fn tool_id(&self) -> &str {
        &self.tool_id
    }

    /// Return the non-secret operator identity recorded on the grant.
    pub fn granted_by(&self) -> &str {
        &self.granted_by
    }

    /// Return the absolute Unix-millisecond issuance timestamp.
    pub fn granted_at_ms(&self) -> u64 {
        self.granted_at_ms
    }

    /// Return the optional exclusive Unix-millisecond expiry timestamp.
    pub fn expires_at_ms(&self) -> Option<u64> {
        self.expires_at_ms
    }

    /// Return the configured lifecycle state.
    pub fn status(&self) -> SmartHomeToolGrantStatus {
        self.status
    }
}

impl OllamaModelConfig {
    /// Return the exact launch selector and Ollama model tag.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Return the explicit plain-HTTP Ollama endpoint.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Return the bounded per-request timeout.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

/// Optional explicit production authorities for the authenticated host data plane.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DataPlaneConfig {
    channel_keys: Vec<ChannelKeyConfig>,
    ollama_models: Vec<OllamaModelConfig>,
    smart_home_tool_grants: Vec<SmartHomeToolGrantConfig>,
}

impl DataPlaneConfig {
    /// Return exact file-backed directional channel-key declarations.
    pub fn channel_keys(&self) -> &[ChannelKeyConfig] {
        &self.channel_keys
    }

    /// Return exact Ollama model registrations.
    pub fn ollama_models(&self) -> &[OllamaModelConfig] {
        &self.ollama_models
    }

    /// Return exact durable smart-home tool grants for Chief host principals.
    pub fn smart_home_tool_grants(&self) -> &[SmartHomeToolGrantConfig] {
        &self.smart_home_tool_grants
    }
}

impl PrivilegeConfig {
    /// Return the canonical Tier 1 auto-approval timeout.
    pub fn tier_1_auto_approve_timeout(&self) -> Duration {
        self.tier_1_auto_approve_timeout
    }

    /// Return the optional operator-reviewed Tier 1 notification helper.
    pub fn tier_1_notification_command(&self) -> Option<&ConfigPath> {
        self.tier_1_notification_command.as_ref()
    }

    /// Return the optional operator-reviewed Tier 2 biometric helper.
    pub fn tier_2_biometric_command(&self) -> Option<&ConfigPath> {
        self.tier_2_biometric_command.as_ref()
    }

    /// Return the optional operator-reviewed Tier 3 hardware-key helper.
    pub fn tier_3_hardware_key_command(&self) -> Option<&ConfigPath> {
        self.tier_3_hardware_key_command.as_ref()
    }

    /// Return the canonical Tier 2 biometric interaction timeout.
    pub fn biometric_timeout(&self) -> Duration {
        self.biometric_timeout
    }

    /// Return the canonical Tier 3 hardware-key interaction timeout.
    pub fn hardware_key_timeout(&self) -> Duration {
        self.hardware_key_timeout
    }

    /// Return exact configured agent-tier assignments.
    pub fn agent_tiers(&self) -> &[AgentPrivilegeTierConfig] {
        &self.agent_tiers
    }

    /// Return exact configured channel-tier assignments.
    pub fn channel_tiers(&self) -> &[ChannelPrivilegeTierConfig] {
        &self.channel_tiers
    }

    /// Return exact configured package-tier assignments.
    pub fn package_tiers(&self) -> &[PackagePrivilegeTierConfig] {
        &self.package_tiers
    }

    /// Return exact configured model-tier assignments.
    pub fn model_tiers(&self) -> &[ModelPrivilegeTierConfig] {
        &self.model_tiers
    }
}

/// Complete validated D18 Chief daemon configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChiefConfig {
    orchestrator: OrchestratorConfig,
    smart_home: Option<SmartHomeListenerConfig>,
    keyring: KeyringConfig,
    host_defaults: HostDefaultsConfig,
    vault: VaultConfig,
    privilege: PrivilegeConfig,
    data_plane: DataPlaneConfig,
}

impl ChiefConfig {
    /// Return listener and package-root settings.
    pub fn orchestrator(&self) -> &OrchestratorConfig {
        &self.orchestrator
    }

    /// Return the optional Chief-owned Home Assistant-compatible listener.
    pub fn smart_home(&self) -> Option<&SmartHomeListenerConfig> {
        self.smart_home.as_ref()
    }

    /// Return package-signing trust settings.
    pub fn keyring(&self) -> &KeyringConfig {
        &self.keyring
    }

    /// Return default host lifecycle settings.
    pub fn host_defaults(&self) -> &HostDefaultsConfig {
        &self.host_defaults
    }

    /// Return vault coordination settings.
    pub fn vault(&self) -> &VaultConfig {
        &self.vault
    }

    /// Return privilege-interaction deadlines.
    pub fn privilege(&self) -> &PrivilegeConfig {
        &self.privilege
    }

    /// Return explicit host data-plane authority declarations.
    pub fn data_plane(&self) -> &DataPlaneConfig {
        &self.data_plane
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

    let orchestrator_bind = expect_string(document.take(ORCHESTRATOR, "bind")?)?
        .parse::<IpAddr>()
        .map_err(|_| ConfigError::InvalidValue)?;
    if !orchestrator_bind.is_loopback() {
        return Err(ConfigError::NonLoopbackBind);
    }
    let orchestrator_port = parse_port(document.take(ORCHESTRATOR, "port")?)?;
    let packages_dir =
        ConfigPath::parse(expect_string(document.take(ORCHESTRATOR, "packages_dir")?)?)?;
    let state_dir = ConfigPath::parse(expect_string(document.take(ORCHESTRATOR, "state_dir")?)?)?;
    let credential_path = ConfigPath::parse(expect_string(
        document.take(ORCHESTRATOR, "credential_path")?,
    )?)?;
    let trusted_keys = parse_trusted_keys(document.take(KEYRING, "trusted_keys")?)?;
    let restart_policy = parse_restart_policy(document.take(HOST_DEFAULTS, "restart_policy")?)?;
    let health_check_interval =
        positive_millis(document.take(HOST_DEFAULTS, "health_check_interval")?)?;
    let executable =
        ConfigPath::parse(expect_string(document.take(HOST_DEFAULTS, "executable")?)?)?;
    let bootstrap_timeout =
        bounded_process_millis(document.take(HOST_DEFAULTS, "bootstrap_timeout")?)?;
    let graceful_stop_timeout =
        bounded_process_millis(document.take(HOST_DEFAULTS, "graceful_stop_timeout")?)?;
    // Both restart-intensity keys are optional so that a config written before
    // D18R R2 existed still loads. A zero for either is refused rather than
    // read as "never restart": `restart_policy = "never"` says that outright,
    // and a bound that silently overrode a declared policy would be a
    // surprising way to find out.
    let restart_window_ns = match document.take_optional(HOST_DEFAULTS, "restart_window") {
        Some(value) => bounded_restart_window_ns(value)?,
        None => DEFAULT_RESTART_WINDOW_NS,
    };
    let max_restarts_per_window =
        match document.take_optional(HOST_DEFAULTS, "max_restarts_per_window") {
            Some(value) => {
                u32::try_from(positive_integer(value)?).map_err(|_| ConfigError::InvalidValue)?
            }
            None => DEFAULT_MAX_RESTARTS_PER_WINDOW,
        };
    let storage_path = ConfigPath::parse(expect_string(document.take(VAULT, "storage_path")?)?)?;
    let default_lease_ttl = positive_secs(document.take(VAULT, "default_lease_ttl")?)?;
    let container = expect_bool(document.take(VAULT, "container")?)?;
    let tier_1_auto_approve_timeout = canonical_secs(
        document.take(PRIVILEGE, "tier_1_auto_approve_timeout")?,
        TIER_1_AUTO_APPROVE_TIMEOUT,
    )?;
    let tier_1_notification_command = document
        .take_optional(PRIVILEGE, "tier_1_notification_command")
        .map(expect_string)
        .transpose()?
        .map(ConfigPath::parse)
        .transpose()?;
    let tier_2_biometric_command = document
        .take_optional(PRIVILEGE, "tier_2_biometric_command")
        .map(expect_string)
        .transpose()?
        .map(ConfigPath::parse)
        .transpose()?;
    let tier_3_hardware_key_command = document
        .take_optional(PRIVILEGE, "tier_3_hardware_key_command")
        .map(expect_string)
        .transpose()?
        .map(ConfigPath::parse)
        .transpose()?;
    let biometric_timeout = canonical_secs(
        document.take(PRIVILEGE, "biometric_timeout")?,
        TIER_2_BIOMETRIC_TIMEOUT,
    )?;
    let hardware_key_timeout = canonical_secs(
        document.take(PRIVILEGE, "hardware_key_timeout")?,
        TIER_3_HARDWARE_KEY_TIMEOUT,
    )?;
    let agent_tiers = document
        .take_optional(PRIVILEGE, "agent_tiers")
        .map(parse_agent_privilege_tiers)
        .transpose()?
        .unwrap_or_default();
    let channel_tiers = document
        .take_optional(PRIVILEGE, "channel_tiers")
        .map(parse_channel_privilege_tiers)
        .transpose()?
        .unwrap_or_default();
    let package_tiers = document
        .take_optional(PRIVILEGE, "package_tiers")
        .map(parse_package_privilege_tiers)
        .transpose()?
        .unwrap_or_default();
    let model_tiers = document
        .take_optional(PRIVILEGE, "model_tiers")
        .map(parse_model_privilege_tiers)
        .transpose()?
        .unwrap_or_default();
    let smart_home = if document.has_table(SMART_HOME) {
        let bind = expect_string(document.take(SMART_HOME, "bind")?)?
            .parse::<IpAddr>()
            .map_err(|_| ConfigError::InvalidValue)?;
        if !bind.is_loopback() {
            return Err(ConfigError::NonLoopbackBind);
        }
        let port = parse_port(document.take(SMART_HOME, "port")?)?;
        let instance_name = bounded_identity(
            expect_string(document.take(SMART_HOME, "instance_name")?)?,
            MAX_SMART_HOME_INSTANCE_NAME_BYTES,
        )?;
        let hue_mdns_interface = document
            .take_optional(SMART_HOME, "hue_mdns_interface")
            .map(expect_string)
            .transpose()?
            .map(|value| bounded_identity(value, MAX_NETWORK_INTERFACE_BYTES))
            .transpose()?;
        let hue_pairing_kek_path = document
            .take_optional(SMART_HOME, "hue_pairing_kek_path")
            .map(expect_string)
            .transpose()?
            .map(ConfigPath::parse)
            .transpose()?;
        let onvif_bridge_id = document
            .take_optional(SMART_HOME, "onvif_pairing_bridge_id")
            .map(expect_string)
            .transpose()?
            .map(|value| bounded_identity(value, MAX_AGENT_ID_BYTES))
            .transpose()?;
        let onvif_kek_path = document
            .take_optional(SMART_HOME, "onvif_pairing_kek_path")
            .map(expect_string)
            .transpose()?
            .map(ConfigPath::parse)
            .transpose()?;
        let onvif_username_path = document
            .take_optional(SMART_HOME, "onvif_pairing_username_path")
            .map(expect_string)
            .transpose()?
            .map(ConfigPath::parse)
            .transpose()?;
        let onvif_username_length = document
            .take_optional(SMART_HOME, "onvif_pairing_username_length")
            .map(bounded_pairing_secret_length)
            .transpose()?;
        let onvif_password_path = document
            .take_optional(SMART_HOME, "onvif_pairing_password_path")
            .map(expect_string)
            .transpose()?
            .map(ConfigPath::parse)
            .transpose()?;
        let onvif_password_length = document
            .take_optional(SMART_HOME, "onvif_pairing_password_length")
            .map(bounded_pairing_secret_length)
            .transpose()?;
        let onvif_pairing = match (
            onvif_bridge_id,
            onvif_kek_path,
            onvif_username_path,
            onvif_username_length,
            onvif_password_path,
            onvif_password_length,
        ) {
            (None, None, None, None, None, None) => None,
            (
                Some(bridge_id),
                Some(kek_path),
                Some(username_path),
                Some(username_length),
                Some(password_path),
                Some(password_length),
            ) => Some(OnvifPairingConfig {
                bridge_id,
                kek_path,
                username_path,
                username_length,
                password_path,
                password_length,
            }),
            _ => return Err(ConfigError::InvalidValue),
        };
        let axis_bridge_id = document
            .take_optional(SMART_HOME, "axis_pairing_bridge_id")
            .map(expect_string)
            .transpose()?
            .map(|value| bounded_identity(value, MAX_AGENT_ID_BYTES))
            .transpose()?;
        let axis_kek_path = document
            .take_optional(SMART_HOME, "axis_pairing_kek_path")
            .map(expect_string)
            .transpose()?
            .map(ConfigPath::parse)
            .transpose()?;
        let axis_username_path = document
            .take_optional(SMART_HOME, "axis_pairing_username_path")
            .map(expect_string)
            .transpose()?
            .map(ConfigPath::parse)
            .transpose()?;
        let axis_username_length = document
            .take_optional(SMART_HOME, "axis_pairing_username_length")
            .map(bounded_pairing_secret_length)
            .transpose()?;
        let axis_password_path = document
            .take_optional(SMART_HOME, "axis_pairing_password_path")
            .map(expect_string)
            .transpose()?
            .map(ConfigPath::parse)
            .transpose()?;
        let axis_password_length = document
            .take_optional(SMART_HOME, "axis_pairing_password_length")
            .map(bounded_pairing_secret_length)
            .transpose()?;
        let axis_pairing = match (
            axis_bridge_id,
            axis_kek_path,
            axis_username_path,
            axis_username_length,
            axis_password_path,
            axis_password_length,
        ) {
            (None, None, None, None, None, None) => None,
            (
                Some(bridge_id),
                Some(kek_path),
                Some(username_path),
                Some(username_length),
                Some(password_path),
                Some(password_length),
            ) => Some(AxisPairingConfig {
                bridge_id,
                kek_path,
                username_path,
                username_length,
                password_path,
                password_length,
            }),
            _ => return Err(ConfigError::InvalidValue),
        };
        let zoneminder_bridge_id = document
            .take_optional(SMART_HOME, "zoneminder_pairing_bridge_id")
            .map(expect_string)
            .transpose()?
            .map(|value| bounded_identity(value, MAX_AGENT_ID_BYTES))
            .transpose()?;
        let zoneminder_kek_path = document
            .take_optional(SMART_HOME, "zoneminder_pairing_kek_path")
            .map(expect_string)
            .transpose()?
            .map(ConfigPath::parse)
            .transpose()?;
        let zoneminder_username_path = document
            .take_optional(SMART_HOME, "zoneminder_pairing_username_path")
            .map(expect_string)
            .transpose()?
            .map(ConfigPath::parse)
            .transpose()?;
        let zoneminder_username_length = document
            .take_optional(SMART_HOME, "zoneminder_pairing_username_length")
            .map(bounded_pairing_secret_length)
            .transpose()?;
        let zoneminder_password_path = document
            .take_optional(SMART_HOME, "zoneminder_pairing_password_path")
            .map(expect_string)
            .transpose()?
            .map(ConfigPath::parse)
            .transpose()?;
        let zoneminder_password_length = document
            .take_optional(SMART_HOME, "zoneminder_pairing_password_length")
            .map(bounded_pairing_secret_length)
            .transpose()?;
        let zoneminder_pairing = match (
            zoneminder_bridge_id,
            zoneminder_kek_path,
            zoneminder_username_path,
            zoneminder_username_length,
            zoneminder_password_path,
            zoneminder_password_length,
        ) {
            (None, None, None, None, None, None) => None,
            (
                Some(bridge_id),
                Some(kek_path),
                Some(username_path),
                Some(username_length),
                Some(password_path),
                Some(password_length),
            ) => Some(ZoneMinderPairingConfig {
                bridge_id,
                kek_path,
                username_path,
                username_length,
                password_path,
                password_length,
            }),
            _ => return Err(ConfigError::InvalidValue),
        };
        let reolink_bridge_id = document
            .take_optional(SMART_HOME, "reolink_pairing_bridge_id")
            .map(expect_string)
            .transpose()?
            .map(|value| bounded_identity(value, MAX_AGENT_ID_BYTES))
            .transpose()?;
        let reolink_canonical_host = document
            .take_optional(SMART_HOME, "reolink_pairing_canonical_host")
            .map(expect_string)
            .transpose()?
            .map(|value| bounded_identity(value, MAX_ENDPOINT_BYTES))
            .transpose()?;
        let reolink_pinned_address = document
            .take_optional(SMART_HOME, "reolink_pairing_pinned_address")
            .map(expect_string)
            .transpose()?
            .map(|value| value.parse().map_err(|_| ConfigError::InvalidValue))
            .transpose()?;
        let reolink_kek_path = document
            .take_optional(SMART_HOME, "reolink_pairing_kek_path")
            .map(expect_string)
            .transpose()?
            .map(ConfigPath::parse)
            .transpose()?;
        let reolink_username_path = document
            .take_optional(SMART_HOME, "reolink_pairing_username_path")
            .map(expect_string)
            .transpose()?
            .map(ConfigPath::parse)
            .transpose()?;
        let reolink_username_length = document
            .take_optional(SMART_HOME, "reolink_pairing_username_length")
            .map(bounded_pairing_secret_length)
            .transpose()?;
        let reolink_password_path = document
            .take_optional(SMART_HOME, "reolink_pairing_password_path")
            .map(expect_string)
            .transpose()?
            .map(ConfigPath::parse)
            .transpose()?;
        let reolink_password_length = document
            .take_optional(SMART_HOME, "reolink_pairing_password_length")
            .map(bounded_pairing_secret_length)
            .transpose()?;
        let reolink_pairing = match (
            reolink_bridge_id,
            reolink_canonical_host,
            reolink_pinned_address,
            reolink_kek_path,
            reolink_username_path,
            reolink_username_length,
            reolink_password_path,
            reolink_password_length,
        ) {
            (None, None, None, None, None, None, None, None) => None,
            (
                Some(bridge_id),
                Some(canonical_host),
                Some(pinned_address),
                Some(kek_path),
                Some(username_path),
                Some(username_length),
                Some(password_path),
                Some(password_length),
            ) => Some(ReolinkPairingConfig {
                bridge_id,
                canonical_host,
                pinned_address,
                kek_path,
                username_path,
                username_length,
                password_path,
                password_length,
            }),
            _ => return Err(ConfigError::InvalidValue),
        };
        let synology_bridge_id = document
            .take_optional(SMART_HOME, "synology_pairing_bridge_id")
            .map(expect_string)
            .transpose()?
            .map(|value| bounded_identity(value, MAX_AGENT_ID_BYTES))
            .transpose()?;
        let synology_canonical_host = document
            .take_optional(SMART_HOME, "synology_pairing_canonical_host")
            .map(expect_string)
            .transpose()?
            .map(|value| bounded_identity(value, MAX_ENDPOINT_BYTES))
            .transpose()?;
        let synology_pinned_address = document
            .take_optional(SMART_HOME, "synology_pairing_pinned_address")
            .map(expect_string)
            .transpose()?
            .map(|value| value.parse().map_err(|_| ConfigError::InvalidValue))
            .transpose()?;
        let synology_kek_path = document
            .take_optional(SMART_HOME, "synology_pairing_kek_path")
            .map(expect_string)
            .transpose()?
            .map(ConfigPath::parse)
            .transpose()?;
        let synology_username_path = document
            .take_optional(SMART_HOME, "synology_pairing_username_path")
            .map(expect_string)
            .transpose()?
            .map(ConfigPath::parse)
            .transpose()?;
        let synology_username_length = document
            .take_optional(SMART_HOME, "synology_pairing_username_length")
            .map(bounded_pairing_secret_length)
            .transpose()?;
        let synology_password_path = document
            .take_optional(SMART_HOME, "synology_pairing_password_path")
            .map(expect_string)
            .transpose()?
            .map(ConfigPath::parse)
            .transpose()?;
        let synology_password_length = document
            .take_optional(SMART_HOME, "synology_pairing_password_length")
            .map(bounded_pairing_secret_length)
            .transpose()?;
        let synology_pairing = match (
            synology_bridge_id,
            synology_canonical_host,
            synology_pinned_address,
            synology_kek_path,
            synology_username_path,
            synology_username_length,
            synology_password_path,
            synology_password_length,
        ) {
            (None, None, None, None, None, None, None, None) => None,
            (
                Some(bridge_id),
                Some(canonical_host),
                Some(pinned_address),
                Some(kek_path),
                Some(username_path),
                Some(username_length),
                Some(password_path),
                Some(password_length),
            ) => Some(SynologyPairingConfig {
                bridge_id,
                canonical_host,
                pinned_address,
                kek_path,
                username_path,
                username_length,
                password_path,
                password_length,
            }),
            _ => return Err(ConfigError::InvalidValue),
        };
        Some(SmartHomeListenerConfig {
            bind,
            port,
            instance_name,
            hue_mdns_interface,
            hue_pairing_kek_path,
            onvif_pairing,
            axis_pairing,
            zoneminder_pairing,
            reolink_pairing,
            synology_pairing,
        })
    } else {
        None
    };
    if smart_home.as_ref().is_some_and(|listener| {
        listener.bind == orchestrator_bind && listener.port == orchestrator_port
    }) {
        return Err(ConfigError::InvalidValue);
    }
    if container
        && smart_home.as_ref().is_some_and(|listener| {
            listener.hue_pairing_kek_path.is_some()
                || listener.onvif_pairing.is_some()
                || listener.axis_pairing.is_some()
                || listener.zoneminder_pairing.is_some()
                || listener.reolink_pairing.is_some()
                || listener.synology_pairing.is_some()
        })
    {
        return Err(ConfigError::InvalidValue);
    }
    let data_plane = if document.has_table(DATA_PLANE) {
        DataPlaneConfig {
            channel_keys: parse_channel_keys(document.take(DATA_PLANE, "channel_keys")?)?,
            ollama_models: parse_ollama_models(document.take(DATA_PLANE, "ollama_models")?)?,
            smart_home_tool_grants: document
                .take_optional(DATA_PLANE, "smart_home_tool_grants")
                .map(parse_smart_home_tool_grants)
                .transpose()?
                .unwrap_or_default(),
        }
    } else {
        DataPlaneConfig::default()
    };
    if !document.fields.is_empty() {
        return Err(ConfigError::Unknown);
    }

    Ok(ChiefConfig {
        orchestrator: OrchestratorConfig {
            bind: orchestrator_bind,
            port: orchestrator_port,
            packages_dir,
            state_dir,
            credential_path,
        },
        smart_home,
        keyring: KeyringConfig { trusted_keys },
        host_defaults: HostDefaultsConfig {
            restart_policy,
            health_check_interval,
            executable,
            bootstrap_timeout,
            graceful_stop_timeout,
            restart_window_ns,
            max_restarts_per_window,
        },
        vault: VaultConfig {
            storage_path,
            default_lease_ttl,
            container,
        },
        privilege: PrivilegeConfig {
            tier_1_auto_approve_timeout,
            tier_1_notification_command,
            tier_2_biometric_command,
            tier_3_hardware_key_command,
            biometric_timeout,
            hardware_key_timeout,
            agent_tiers,
            channel_tiers,
            package_tiers,
            model_tiers,
        },
        data_plane,
    })
}

fn parse_agent_privilege_tiers(
    value: RawValue,
) -> Result<Vec<AgentPrivilegeTierConfig>, ConfigError> {
    parse_privilege_assignments(value, |mut fields| {
        let agent_id = decode_lower_hex(
            expect_string(take_inline(&mut fields, "agent_id")?)?,
            MAX_AGENT_ID_BYTES,
        )?;
        let tier = parse_configured_privilege_tier(take_inline(&mut fields, "tier")?)?;
        require_closed_inline(fields)?;
        Ok((
            agent_id.clone(),
            AgentPrivilegeTierConfig { agent_id, tier },
        ))
    })
}

fn parse_channel_privilege_tiers(
    value: RawValue,
) -> Result<Vec<ChannelPrivilegeTierConfig>, ConfigError> {
    parse_privilege_assignments(value, |mut fields| {
        let channel_id = parse_uuid_v7(expect_string(take_inline(&mut fields, "channel_id")?)?)?;
        let tier = parse_configured_privilege_tier(take_inline(&mut fields, "tier")?)?;
        require_closed_inline(fields)?;
        Ok((channel_id, ChannelPrivilegeTierConfig { channel_id, tier }))
    })
}

fn parse_package_privilege_tiers(
    value: RawValue,
) -> Result<Vec<PackagePrivilegeTierConfig>, ConfigError> {
    parse_privilege_assignments(value, |mut fields| {
        let bytes = decode_lower_hex(
            expect_string(take_inline(&mut fields, "package_hash")?)?,
            32,
        )?;
        let package_hash: [u8; 32] = bytes.try_into().map_err(|_| ConfigError::InvalidValue)?;
        let tier = parse_configured_privilege_tier(take_inline(&mut fields, "tier")?)?;
        require_closed_inline(fields)?;
        Ok((
            package_hash,
            PackagePrivilegeTierConfig { package_hash, tier },
        ))
    })
}

fn parse_model_privilege_tiers(
    value: RawValue,
) -> Result<Vec<ModelPrivilegeTierConfig>, ConfigError> {
    parse_privilege_assignments(value, |mut fields| {
        let model = bounded_identity(
            expect_string(take_inline(&mut fields, "model")?)?,
            MAX_MODEL_BYTES,
        )?;
        let tier = parse_configured_privilege_tier(take_inline(&mut fields, "tier")?)?;
        require_closed_inline(fields)?;
        Ok((model.clone(), ModelPrivilegeTierConfig { model, tier }))
    })
}

fn parse_privilege_assignments<K, T>(
    value: RawValue,
    mut parse: impl FnMut(BTreeMap<Vec<String>, RawValue>) -> Result<(K, T), ConfigError>,
) -> Result<Vec<T>, ConfigError>
where
    K: Ord,
{
    let RawValue::Array(values) = value else {
        return Err(ConfigError::InvalidType);
    };
    if values.len() > MAX_PRIVILEGE_ASSIGNMENTS {
        return Err(ConfigError::InvalidValue);
    }
    let mut identities = BTreeSet::new();
    let mut assignments = Vec::with_capacity(values.len());
    for value in values {
        let RawValue::InlineTable(fields) = value else {
            return Err(ConfigError::InvalidType);
        };
        let (identity, assignment) = parse(fields)?;
        if !identities.insert(identity) {
            return Err(ConfigError::Duplicate);
        }
        assignments.push(assignment);
    }
    Ok(assignments)
}

fn parse_configured_privilege_tier(
    value: RawValue,
) -> Result<ConfiguredPrivilegeTier, ConfigError> {
    let RawValue::Integer(value) = value else {
        return Err(ConfigError::InvalidType);
    };
    match value {
        0 => Ok(ConfiguredPrivilegeTier::Tier0),
        1 => Ok(ConfiguredPrivilegeTier::Tier1),
        2 => Ok(ConfiguredPrivilegeTier::Tier2),
        3 => Ok(ConfiguredPrivilegeTier::Tier3),
        _ => Err(ConfigError::InvalidValue),
    }
}

fn require_closed_inline(fields: BTreeMap<Vec<String>, RawValue>) -> Result<(), ConfigError> {
    if fields.is_empty() {
        Ok(())
    } else {
        Err(ConfigError::Unknown)
    }
}

fn decode_lower_hex(value: String, maximum_bytes: usize) -> Result<Vec<u8>, ConfigError> {
    if value.is_empty()
        || value.len() > maximum_bytes.saturating_mul(2)
        || !value.len().is_multiple_of(2)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ConfigError::InvalidValue);
    }
    Ok(value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]))
        .collect())
}

fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => unreachable!("hex input was validated before decoding"),
    }
}

fn parse_channel_keys(value: RawValue) -> Result<Vec<ChannelKeyConfig>, ConfigError> {
    let RawValue::Array(values) = value else {
        return Err(ConfigError::InvalidType);
    };
    if values.len() > MAX_CHANNEL_KEYS {
        return Err(ConfigError::InvalidValue);
    }
    let mut identities = BTreeSet::new();
    let mut declarations = Vec::with_capacity(values.len());
    for value in values {
        let RawValue::InlineTable(mut fields) = value else {
            return Err(ConfigError::InvalidType);
        };
        let pipeline_id = parse_uuid_v7(expect_string(take_inline(&mut fields, "pipeline_id")?)?)?;
        let agent_id = expect_string(take_inline(&mut fields, "agent_id")?)?;
        if agent_id.is_empty()
            || agent_id.len() > MAX_AGENT_ID_BYTES
            || agent_id.trim() != agent_id
            || agent_id.chars().any(char::is_control)
        {
            return Err(ConfigError::InvalidValue);
        }
        let channel_id = parse_uuid_v7(expect_string(take_inline(&mut fields, "channel_id")?)?)?;
        let access = match expect_string(take_inline(&mut fields, "access")?)?.as_str() {
            "read" => ChannelKeyAccess::Read,
            "write" => ChannelKeyAccess::Write,
            _ => return Err(ConfigError::InvalidValue),
        };
        let (receiver_private_key_path, originator_signing_seed_path, channel_master_key_path) =
            match access {
                ChannelKeyAccess::Read => (
                    Some(ConfigPath::parse(expect_string(take_inline(
                        &mut fields,
                        "private_key_path",
                    )?)?)?),
                    None,
                    None,
                ),
                ChannelKeyAccess::Write => (
                    None,
                    Some(ConfigPath::parse(expect_string(take_inline(
                        &mut fields,
                        "signing_seed_path",
                    )?)?)?),
                    Some(ConfigPath::parse(expect_string(take_inline(
                        &mut fields,
                        "channel_key_path",
                    )?)?)?),
                ),
            };
        if !fields.is_empty() {
            return Err(ConfigError::Unknown);
        }
        if !identities.insert((pipeline_id, agent_id.clone(), channel_id)) {
            return Err(ConfigError::Duplicate);
        }
        declarations.push(ChannelKeyConfig {
            pipeline_id,
            agent_id,
            channel_id,
            access,
            receiver_private_key_path,
            originator_signing_seed_path,
            channel_master_key_path,
        });
    }
    Ok(declarations)
}

fn parse_ollama_models(value: RawValue) -> Result<Vec<OllamaModelConfig>, ConfigError> {
    let RawValue::Array(values) = value else {
        return Err(ConfigError::InvalidType);
    };
    if values.len() > MAX_OLLAMA_MODELS {
        return Err(ConfigError::InvalidValue);
    }
    let mut models = BTreeSet::new();
    let mut declarations = Vec::with_capacity(values.len());
    for value in values {
        let RawValue::InlineTable(mut fields) = value else {
            return Err(ConfigError::InvalidType);
        };
        let model = expect_string(take_inline(&mut fields, "model")?)?;
        let endpoint = expect_string(take_inline(&mut fields, "endpoint")?)?;
        let timeout = bounded_process_millis(take_inline(&mut fields, "timeout")?)?;
        if model.trim().is_empty()
            || model.trim() != model
            || model.len() > MAX_MODEL_BYTES
            || model.chars().any(char::is_control)
            || endpoint.is_empty()
            || endpoint.len() > MAX_ENDPOINT_BYTES
            || endpoint.chars().any(char::is_control)
        {
            return Err(ConfigError::InvalidValue);
        }
        if !fields.is_empty() {
            return Err(ConfigError::Unknown);
        }
        if !models.insert(model.clone()) {
            return Err(ConfigError::Duplicate);
        }
        declarations.push(OllamaModelConfig {
            model,
            endpoint,
            timeout,
        });
    }
    Ok(declarations)
}

fn parse_smart_home_tool_grants(
    value: RawValue,
) -> Result<Vec<SmartHomeToolGrantConfig>, ConfigError> {
    let RawValue::Array(values) = value else {
        return Err(ConfigError::InvalidType);
    };
    if values.len() > MAX_SMART_HOME_TOOL_GRANTS {
        return Err(ConfigError::InvalidValue);
    }
    let mut grant_ids = BTreeSet::new();
    let mut grants = Vec::with_capacity(values.len());
    for value in values {
        let RawValue::InlineTable(mut fields) = value else {
            return Err(ConfigError::InvalidType);
        };
        let grant_id = bounded_identity(
            expect_string(take_inline(&mut fields, "grant_id")?)?,
            MAX_GRANT_ID_BYTES,
        )?;
        let principal_id = bounded_identity(
            expect_string(take_inline(&mut fields, "principal_id")?)?,
            MAX_AGENT_ID_BYTES,
        )?;
        let tool_id = bounded_identity(
            expect_string(take_inline(&mut fields, "tool_id")?)?,
            MAX_TOOL_ID_BYTES,
        )?;
        if !tool_id.starts_with("smart_home.") {
            return Err(ConfigError::InvalidValue);
        }
        let granted_by = bounded_identity(
            expect_string(take_inline(&mut fields, "granted_by")?)?,
            MAX_GRANTED_BY_BYTES,
        )?;
        let granted_at_ms = positive_integer(take_inline(&mut fields, "granted_at_ms")?)?;
        let expires_at_ms = fields
            .remove(&vec!["expires_at_ms".to_string()])
            .map(positive_integer)
            .transpose()?;
        if expires_at_ms.is_some_and(|expires_at_ms| expires_at_ms <= granted_at_ms) {
            return Err(ConfigError::InvalidValue);
        }
        let status = fields
            .remove(&vec!["status".to_string()])
            .map(expect_string)
            .transpose()?
            .map(|status| match status.as_str() {
                "pending" => Ok(SmartHomeToolGrantStatus::Pending),
                "active" => Ok(SmartHomeToolGrantStatus::Active),
                "revoked" => Ok(SmartHomeToolGrantStatus::Revoked),
                _ => Err(ConfigError::InvalidValue),
            })
            .transpose()?
            .unwrap_or(SmartHomeToolGrantStatus::Active);
        if !fields.is_empty() {
            return Err(ConfigError::Unknown);
        }
        if !grant_ids.insert(grant_id.clone()) {
            return Err(ConfigError::Duplicate);
        }
        grants.push(SmartHomeToolGrantConfig {
            grant_id,
            principal_id,
            tool_id,
            granted_by,
            granted_at_ms,
            expires_at_ms,
            status,
        });
    }
    Ok(grants)
}

fn bounded_identity(value: String, max_bytes: usize) -> Result<String, ConfigError> {
    if value.is_empty()
        || value.len() > max_bytes
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        Err(ConfigError::InvalidValue)
    } else {
        Ok(value)
    }
}

fn parse_uuid_v7(value: String) -> Result<[u8; 16], ConfigError> {
    let parsed = coding_adventures_uuid::parse(&value).map_err(|_| ConfigError::InvalidValue)?;
    if parsed.version() != 7 || parsed.variant() != "rfc4122" || parsed.to_string() != value {
        return Err(ConfigError::InvalidValue);
    }
    Ok(parsed.bytes())
}

fn parse_port(value: RawValue) -> Result<u16, ConfigError> {
    let RawValue::Integer(value) = value else {
        return Err(ConfigError::InvalidType);
    };
    u16::try_from(value)
        .ok()
        .filter(|port| *port != 0)
        .ok_or(ConfigError::InvalidValue)
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

fn bounded_process_millis(value: RawValue) -> Result<Duration, ConfigError> {
    let millis = positive_integer(value)?;
    if millis > MAX_PROCESS_TIMEOUT_MILLIS {
        return Err(ConfigError::InvalidValue);
    }
    Ok(Duration::from_millis(millis))
}

/// Parse a restart window in milliseconds and return it in nanoseconds.
///
/// Bounded so the conversion is total. Without the ceiling a large value
/// saturates to `u64::MAX` nanoseconds, which the reconciler reads as a window
/// that never elapses -- a quarantine that never lifts, and no error anywhere
/// to say so.
fn bounded_restart_window_ns(value: RawValue) -> Result<u64, ConfigError> {
    let millis = positive_integer(value)?;
    if millis > MAX_RESTART_WINDOW_MILLIS {
        return Err(ConfigError::InvalidValue);
    }
    Ok(millis * 1_000_000)
}

fn positive_secs(value: RawValue) -> Result<Duration, ConfigError> {
    positive_integer(value).map(Duration::from_secs)
}

fn canonical_secs(value: RawValue, canonical: Duration) -> Result<Duration, ConfigError> {
    let parsed = positive_secs(value)?;
    if parsed != canonical {
        return Err(ConfigError::InvalidValue);
    }
    Ok(canonical)
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

fn bounded_pairing_secret_length(value: RawValue) -> Result<usize, ConfigError> {
    let value = positive_integer(value)?;
    if value > MAX_PAIRING_SECRET_BYTES {
        return Err(ConfigError::InvalidValue);
    }
    usize::try_from(value).map_err(|_| ConfigError::InvalidValue)
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
        let required = [ORCHESTRATOR, KEYRING, HOST_DEFAULTS, VAULT, PRIVILEGE]
            .into_iter()
            .map(strings_to_vec)
            .collect::<BTreeSet<_>>();
        let mut allowed = required.clone();
        allowed.insert(strings_to_vec(DATA_PLANE));
        allowed.insert(strings_to_vec(SMART_HOME));
        if self.tables.iter().any(|table| !allowed.contains(table)) {
            Err(ConfigError::Unknown)
        } else if required.iter().any(|table| !self.tables.contains(table)) {
            Err(ConfigError::Missing)
        } else {
            Ok(())
        }
    }

    fn has_table(&self, table: &[&str]) -> bool {
        self.tables.contains(&strings_to_vec(table))
    }

    fn take(&mut self, table: &[&str], field: &str) -> Result<RawValue, ConfigError> {
        let mut key = strings_to_vec(table);
        key.push(field.to_string());
        self.fields.remove(&key).ok_or(ConfigError::Missing)
    }

    fn take_optional(&mut self, table: &[&str], field: &str) -> Option<RawValue> {
        let mut key = strings_to_vec(table);
        key.push(field.to_string());
        self.fields.remove(&key)
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
port = 7463
packages_dir = "~/.chief-of-staff/agents/"
state_dir = "~/.chief-of-staff/state/"
credential_path = "~/.chief-of-staff/run/operator.credential"

[keyring]
trusted_keys = [
  { id = "prod-001", path = "~/.chief-of-staff/keys/prod-001.pub", type = "production" },
  { id = "dev-local", path = "~/.chief-of-staff/keys/dev.pub", type = "developer" },
]

[hosts.defaults]
restart_policy = "on-failure"
health_check_interval = 5_000
executable = "~/.chief-of-staff/bin/chief-of-staff-host"
bootstrap_timeout = 10_000
graceful_stop_timeout = 5_000

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
        assert_eq!(config.orchestrator().port(), 7463);
        assert_eq!(
            config.orchestrator().packages_dir().as_str(),
            "~/.chief-of-staff/agents/"
        );
        assert_eq!(
            config.orchestrator().state_dir().as_str(),
            "~/.chief-of-staff/state/"
        );
        assert_eq!(
            config.orchestrator().credential_path().as_str(),
            "~/.chief-of-staff/run/operator.credential"
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
        assert_eq!(
            config.host_defaults().executable().as_str(),
            "~/.chief-of-staff/bin/chief-of-staff-host"
        );
        assert_eq!(
            config.host_defaults().bootstrap_timeout(),
            Duration::from_secs(10)
        );
        assert_eq!(
            config.host_defaults().graceful_stop_timeout(),
            Duration::from_secs(5)
        );
        assert_eq!(config.vault().default_lease_ttl(), Duration::from_secs(30));
        assert!(config.vault().container());
        assert_eq!(
            config.privilege().tier_1_auto_approve_timeout(),
            TIER_1_AUTO_APPROVE_TIMEOUT
        );
        assert_eq!(
            config.privilege().biometric_timeout(),
            TIER_2_BIOMETRIC_TIMEOUT
        );
        assert_eq!(
            config.privilege().hardware_key_timeout(),
            TIER_3_HARDWARE_KEY_TIMEOUT
        );
        assert!(config.privilege().tier_1_notification_command().is_none());
        assert!(config.privilege().tier_2_biometric_command().is_none());
        assert!(config.privilege().tier_3_hardware_key_command().is_none());
        assert!(config.data_plane().channel_keys().is_empty());
        assert!(config.data_plane().ollama_models().is_empty());
        assert!(config.data_plane().smart_home_tool_grants().is_empty());
        assert!(config.smart_home().is_none());
    }

    #[test]
    fn privilege_deadlines_are_canonical_and_fail_closed() {
        for (declared, replacement) in [
            (
                "tier_1_auto_approve_timeout = 5",
                "tier_1_auto_approve_timeout = 4",
            ),
            (
                "tier_1_auto_approve_timeout = 5",
                "tier_1_auto_approve_timeout = 6",
            ),
            ("biometric_timeout = 30", "biometric_timeout = 29"),
            ("biometric_timeout = 30", "biometric_timeout = 31"),
            ("hardware_key_timeout = 60", "hardware_key_timeout = 59"),
            ("hardware_key_timeout = 60", "hardware_key_timeout = 61"),
        ] {
            assert_eq!(
                parse_config(&VALID.replace(declared, replacement)),
                Err(ConfigError::InvalidValue)
            );
        }
        assert_eq!(
            parse_config(&VALID.replace(
                "tier_1_auto_approve_timeout = 5",
                "tier_1_auto_approve_timeout = true"
            )),
            Err(ConfigError::InvalidType)
        );
    }

    #[test]
    fn tier_one_notification_command_is_optional_typed_and_normalized() {
        let source = VALID.replace(
            "tier_1_auto_approve_timeout = 5",
            "tier_1_auto_approve_timeout = 5\ntier_1_notification_command = \"~/.chief-of-staff/bin/notify\"",
        );
        let config = parse_config(&source).unwrap();
        let command = config.privilege().tier_1_notification_command().unwrap();
        assert_eq!(command.as_str(), "~/.chief-of-staff/bin/notify");
        assert_eq!(
            command.resolve(&absolute_home()).unwrap(),
            absolute_home().join(".chief-of-staff/bin/notify")
        );
        for invalid in ["notify", "~/../notify", "", "~/"] {
            assert_eq!(
                parse_config(&source.replace("~/.chief-of-staff/bin/notify", invalid)),
                Err(ConfigError::UnsafePath)
            );
        }
    }

    #[test]
    fn tier_two_biometric_command_is_optional_typed_and_normalized() {
        let source = VALID.replace(
            "biometric_timeout = 30",
            "biometric_timeout = 30\ntier_2_biometric_command = \"~/.chief-of-staff/bin/biometric\"",
        );
        let config = parse_config(&source).unwrap();
        let command = config.privilege().tier_2_biometric_command().unwrap();
        assert_eq!(command.as_str(), "~/.chief-of-staff/bin/biometric");
        assert_eq!(
            command.resolve(&absolute_home()).unwrap(),
            absolute_home().join(".chief-of-staff/bin/biometric")
        );
        for invalid in ["biometric", "~/../biometric", "", "~/"] {
            assert_eq!(
                parse_config(&source.replace("~/.chief-of-staff/bin/biometric", invalid)),
                Err(ConfigError::UnsafePath)
            );
        }
    }

    #[test]
    fn tier_three_hardware_key_command_is_optional_typed_and_normalized() {
        let source = VALID.replace(
            "hardware_key_timeout = 60",
            "hardware_key_timeout = 60\ntier_3_hardware_key_command = \"~/.chief-of-staff/bin/hardware-key\"",
        );
        let config = parse_config(&source).unwrap();
        let command = config.privilege().tier_3_hardware_key_command().unwrap();
        assert_eq!(command.as_str(), "~/.chief-of-staff/bin/hardware-key");
        assert_eq!(
            command.resolve(&absolute_home()).unwrap(),
            absolute_home().join(".chief-of-staff/bin/hardware-key")
        );
        for invalid in ["hardware-key", "~/../hardware-key", "", "~/"] {
            assert_eq!(
                parse_config(&source.replace("~/.chief-of-staff/bin/hardware-key", invalid)),
                Err(ConfigError::UnsafePath)
            );
        }
    }

    #[test]
    fn privilege_resource_assignments_are_exact_bounded_and_closed() {
        let source = format!(
            r#"{VALID}
agent_tiers = [
  {{ agent_id = "77656174686572", tier = 0 }},
]
channel_tiers = [
  {{ channel_id = "018f0c10-7b4a-7cc0-8000-000000000002", tier = 1 }},
]
package_tiers = [
  {{ package_hash = "{}", tier = 2 }},
]
model_tiers = [
  {{ model = "qwen2.5:0.5b", tier = 3 }},
]
"#,
            "ab".repeat(32)
        );
        let config = parse_config(&source).unwrap();
        let privilege = config.privilege();
        assert_eq!(privilege.agent_tiers()[0].agent_id(), b"weather");
        assert_eq!(
            privilege.agent_tiers()[0].tier(),
            ConfiguredPrivilegeTier::Tier0
        );
        assert_eq!(
            privilege.channel_tiers()[0].tier(),
            ConfiguredPrivilegeTier::Tier1
        );
        assert_eq!(privilege.package_tiers()[0].package_hash(), [0xab; 32]);
        assert_eq!(
            privilege.package_tiers()[0].tier(),
            ConfiguredPrivilegeTier::Tier2
        );
        assert_eq!(privilege.model_tiers()[0].model(), "qwen2.5:0.5b");
        assert_eq!(
            privilege.model_tiers()[0].tier(),
            ConfiguredPrivilegeTier::Tier3
        );

        assert_eq!(
            parse_config(&source.replace("77656174686572", "7765617468657A")),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            parse_config(&source.replace("tier = 3", "tier = 4")),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            parse_config(&source.replace(
                "{ agent_id = \"77656174686572\", tier = 0 },",
                "{ agent_id = \"77656174686572\", tier = 0 },\n  { agent_id = \"77656174686572\", tier = 1 },"
            )),
            Err(ConfigError::Duplicate)
        );
        assert_eq!(
            parse_config(&source.replace(
                "model = \"qwen2.5:0.5b\", tier = 3",
                "model = \"qwen2.5:0.5b\", tier = 3, extra = true"
            )),
            Err(ConfigError::Unknown)
        );
    }

    #[test]
    fn parses_an_optional_distinct_loopback_smart_home_listener() {
        let source = format!(
            "{VALID}\n[smart_home]\nbind = \"127.0.0.1\"\nport = 8123\ninstance_name = \"Codex Home\"\nhue_mdns_interface = \"en0\"\n"
        );
        let config = parse_config(&source).unwrap();
        let listener = config.smart_home().unwrap();
        assert_eq!(listener.bind(), "127.0.0.1".parse::<IpAddr>().unwrap());
        assert_eq!(listener.port(), 8123);
        assert_eq!(listener.instance_name(), "Codex Home");
        assert_eq!(listener.hue_mdns_interface(), Some("en0"));
        assert!(listener.hue_pairing_kek_path().is_none());
        assert!(listener.onvif_pairing().is_none());
        assert!(listener.axis_pairing().is_none());
        assert!(listener.zoneminder_pairing().is_none());
        assert!(listener.reolink_pairing().is_none());
        assert!(listener.synology_pairing().is_none());

        assert_eq!(
            parse_config(&source.replace("port = 8123", "port = 7463")),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            parse_config(&source.replace("127.0.0.1", "0.0.0.0")),
            Err(ConfigError::NonLoopbackBind)
        );
        assert_eq!(
            parse_config(&source.replace("Codex Home", " Codex Home")),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            parse_config(&source.replace(
                "instance_name = \"Codex Home\"",
                "instance_name = \"Codex Home\"\nsurprise = true"
            )),
            Err(ConfigError::Unknown)
        );
        assert_eq!(
            parse_config(
                &source.replace("hue_mdns_interface = \"en0\"", "hue_mdns_interface = \"\"")
            ),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            parse_config(&source.replace(
                "hue_mdns_interface = \"en0\"",
                "hue_mdns_interface = \" en0\""
            )),
            Err(ConfigError::InvalidValue)
        );
    }

    #[test]
    fn hue_pairing_requires_explicit_in_process_vault_custody() {
        let source = format!(
            "{VALID}\n[smart_home]\nbind = \"127.0.0.1\"\nport = 8123\ninstance_name = \"Codex Home\"\nhue_pairing_kek_path = \"~/.chief-of-staff/keys/smart-home-vault.kek\"\n"
        );
        assert_eq!(parse_config(&source), Err(ConfigError::InvalidValue));

        let source = source.replace("container = true", "container = false");
        let config = parse_config(&source).unwrap();
        assert_eq!(
            config
                .smart_home()
                .unwrap()
                .hue_pairing_kek_path()
                .unwrap()
                .as_str(),
            "~/.chief-of-staff/keys/smart-home-vault.kek"
        );
    }

    #[test]
    fn onvif_pairing_requires_complete_owner_only_inputs_and_vault_custody() {
        let source = format!(
            "{VALID}\n[smart_home]\nbind = \"127.0.0.1\"\nport = 8123\ninstance_name = \"Codex Home\"\nonvif_pairing_bridge_id = \"camera-front\"\nonvif_pairing_kek_path = \"~/.chief-of-staff/keys/smart-home-vault.kek\"\nonvif_pairing_username_path = \"~/.chief-of-staff/keys/onvif-user\"\nonvif_pairing_username_length = 8\nonvif_pairing_password_path = \"~/.chief-of-staff/keys/onvif-password\"\nonvif_pairing_password_length = 19\n"
        );
        assert_eq!(parse_config(&source), Err(ConfigError::InvalidValue));

        let source = source.replace("container = true", "container = false");
        let config = parse_config(&source).unwrap();
        let pairing = config.smart_home().unwrap().onvif_pairing().unwrap();
        assert_eq!(pairing.bridge_id(), "camera-front");
        assert_eq!(
            pairing.kek_path().as_str(),
            "~/.chief-of-staff/keys/smart-home-vault.kek"
        );
        assert_eq!(
            pairing.username_path().as_str(),
            "~/.chief-of-staff/keys/onvif-user"
        );
        assert_eq!(pairing.username_length(), 8);
        assert_eq!(
            pairing.password_path().as_str(),
            "~/.chief-of-staff/keys/onvif-password"
        );
        assert_eq!(pairing.password_length(), 19);

        assert_eq!(
            parse_config(&source.replace("onvif_pairing_password_length = 19\n", "")),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            parse_config(&source.replace(
                "onvif_pairing_password_length = 19",
                "onvif_pairing_password_length = 4097"
            )),
            Err(ConfigError::InvalidValue)
        );
    }

    #[test]
    fn axis_pairing_requires_complete_owner_only_inputs_and_vault_custody() {
        let source = format!(
            "{VALID}\n[smart_home]\nbind = \"127.0.0.1\"\nport = 8123\ninstance_name = \"Codex Home\"\naxis_pairing_bridge_id = \"axis-camera-front\"\naxis_pairing_kek_path = \"~/.chief-of-staff/keys/smart-home-vault.kek\"\naxis_pairing_username_path = \"~/.chief-of-staff/keys/axis-user\"\naxis_pairing_username_length = 11\naxis_pairing_password_path = \"~/.chief-of-staff/keys/axis-password\"\naxis_pairing_password_length = 20\n"
        );
        assert_eq!(parse_config(&source), Err(ConfigError::InvalidValue));

        let source = source.replace("container = true", "container = false");
        let config = parse_config(&source).unwrap();
        let pairing = config.smart_home().unwrap().axis_pairing().unwrap();
        assert_eq!(pairing.bridge_id(), "axis-camera-front");
        assert_eq!(
            pairing.kek_path().as_str(),
            "~/.chief-of-staff/keys/smart-home-vault.kek"
        );
        assert_eq!(
            pairing.username_path().as_str(),
            "~/.chief-of-staff/keys/axis-user"
        );
        assert_eq!(pairing.username_length(), 11);
        assert_eq!(
            pairing.password_path().as_str(),
            "~/.chief-of-staff/keys/axis-password"
        );
        assert_eq!(pairing.password_length(), 20);

        assert_eq!(
            parse_config(&source.replace("axis_pairing_password_length = 20\n", "")),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            parse_config(&source.replace(
                "axis_pairing_password_length = 20",
                "axis_pairing_password_length = 4097"
            )),
            Err(ConfigError::InvalidValue)
        );
    }

    #[test]
    fn zoneminder_pairing_requires_complete_owner_only_inputs_and_vault_custody() {
        let source = format!(
            "{VALID}\n[smart_home]\nbind = \"127.0.0.1\"\nport = 8123\ninstance_name = \"Codex Home\"\nzoneminder_pairing_bridge_id = \"zoneminder-nvr\"\nzoneminder_pairing_kek_path = \"~/.chief-of-staff/keys/smart-home-vault.kek\"\nzoneminder_pairing_username_path = \"~/.chief-of-staff/keys/zoneminder-user\"\nzoneminder_pairing_username_length = 15\nzoneminder_pairing_password_path = \"~/.chief-of-staff/keys/zoneminder-password\"\nzoneminder_pairing_password_length = 24\n"
        );
        assert_eq!(parse_config(&source), Err(ConfigError::InvalidValue));

        let source = source.replace("container = true", "container = false");
        let config = parse_config(&source).unwrap();
        let pairing = config.smart_home().unwrap().zoneminder_pairing().unwrap();
        assert_eq!(pairing.bridge_id(), "zoneminder-nvr");
        assert_eq!(
            pairing.kek_path().as_str(),
            "~/.chief-of-staff/keys/smart-home-vault.kek"
        );
        assert_eq!(
            pairing.username_path().as_str(),
            "~/.chief-of-staff/keys/zoneminder-user"
        );
        assert_eq!(pairing.username_length(), 15);
        assert_eq!(
            pairing.password_path().as_str(),
            "~/.chief-of-staff/keys/zoneminder-password"
        );
        assert_eq!(pairing.password_length(), 24);

        assert_eq!(
            parse_config(&source.replace("zoneminder_pairing_password_length = 24\n", "")),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            parse_config(&source.replace(
                "zoneminder_pairing_password_length = 24",
                "zoneminder_pairing_password_length = 4097"
            )),
            Err(ConfigError::InvalidValue)
        );
    }

    #[test]
    fn reolink_pairing_requires_complete_owner_only_inputs_and_vault_custody() {
        let source = format!(
            "{VALID}\n[smart_home]\nbind = \"127.0.0.1\"\nport = 8123\ninstance_name = \"Codex Home\"\nreolink_pairing_bridge_id = \"reolink-camera\"\nreolink_pairing_canonical_host = \"camera.home.arpa\"\nreolink_pairing_pinned_address = \"192.0.2.10:443\"\nreolink_pairing_kek_path = \"~/.chief-of-staff/keys/smart-home-vault.kek\"\nreolink_pairing_username_path = \"~/.chief-of-staff/keys/reolink-user\"\nreolink_pairing_username_length = 12\nreolink_pairing_password_path = \"~/.chief-of-staff/keys/reolink-password\"\nreolink_pairing_password_length = 21\n"
        );
        assert_eq!(parse_config(&source), Err(ConfigError::InvalidValue));

        let source = source.replace("container = true", "container = false");
        let config = parse_config(&source).unwrap();
        let pairing = config.smart_home().unwrap().reolink_pairing().unwrap();
        assert_eq!(pairing.bridge_id(), "reolink-camera");
        assert_eq!(pairing.canonical_host(), "camera.home.arpa");
        assert_eq!(pairing.pinned_address(), "192.0.2.10:443".parse().unwrap());
        assert_eq!(pairing.username_length(), 12);
        assert_eq!(pairing.password_length(), 21);
        assert_eq!(
            parse_config(&source.replace("reolink_pairing_password_length = 21\n", "")),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            parse_config(&source.replace(
                "reolink_pairing_password_length = 21",
                "reolink_pairing_password_length = 4097"
            )),
            Err(ConfigError::InvalidValue)
        );
    }

    #[test]
    fn synology_pairing_requires_complete_owner_only_inputs_and_vault_custody() {
        let source = format!(
            "{VALID}\n[smart_home]\nbind = \"127.0.0.1\"\nport = 8123\ninstance_name = \"Codex Home\"\nsynology_pairing_bridge_id = \"synology-nvr\"\nsynology_pairing_canonical_host = \"nvr.home.arpa\"\nsynology_pairing_pinned_address = \"192.0.2.20:443\"\nsynology_pairing_kek_path = \"~/.chief-of-staff/keys/smart-home-vault.kek\"\nsynology_pairing_username_path = \"~/.chief-of-staff/keys/synology-user\"\nsynology_pairing_username_length = 13\nsynology_pairing_password_path = \"~/.chief-of-staff/keys/synology-password\"\nsynology_pairing_password_length = 22\n"
        );
        assert_eq!(parse_config(&source), Err(ConfigError::InvalidValue));

        let source = source.replace("container = true", "container = false");
        let config = parse_config(&source).unwrap();
        let pairing = config.smart_home().unwrap().synology_pairing().unwrap();
        assert_eq!(pairing.bridge_id(), "synology-nvr");
        assert_eq!(pairing.canonical_host(), "nvr.home.arpa");
        assert_eq!(pairing.pinned_address(), "192.0.2.20:443".parse().unwrap());
        assert_eq!(pairing.username_length(), 13);
        assert_eq!(pairing.password_length(), 22);
        assert_eq!(
            parse_config(&source.replace("synology_pairing_password_length = 22\n", "")),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            parse_config(&source.replace(
                "synology_pairing_password_length = 22",
                "synology_pairing_password_length = 4097"
            )),
            Err(ConfigError::InvalidValue)
        );
    }

    #[test]
    fn parses_exact_directional_keys_and_ollama_models() {
        let source = format!(
            r#"{VALID}

[data_plane]
channel_keys = [
  {{ pipeline_id = "018f0c10-7b4a-7cc0-8000-000000000001", agent_id = "weather", channel_id = "018f0c10-7b4a-7cc0-8000-000000000002", access = "read", private_key_path = "~/.chief-of-staff/keys/weather-receiver.bin" }},
  {{ pipeline_id = "018f0c10-7b4a-7cc0-8000-000000000001", agent_id = "weather", channel_id = "018f0c10-7b4a-7cc0-8000-000000000003", access = "write", signing_seed_path = "~/.chief-of-staff/keys/weather-signing.bin", channel_key_path = "~/.chief-of-staff/keys/weather-channel.bin" }},
]
ollama_models = [
  {{ model = "qwen2.5:0.5b", endpoint = "http://127.0.0.1:11434", timeout = 120000 }},
]
"#
        );
        let config = parse_config(&source).unwrap();
        let keys = config.data_plane().channel_keys();
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0].access(), ChannelKeyAccess::Read);
        assert!(keys[0].receiver_private_key_path().is_some());
        assert!(keys[0].originator_signing_seed_path().is_none());
        assert_eq!(keys[1].access(), ChannelKeyAccess::Write);
        assert!(keys[1].receiver_private_key_path().is_none());
        assert!(keys[1].originator_signing_seed_path().is_some());
        assert!(keys[1].channel_master_key_path().is_some());
        let models = config.data_plane().ollama_models();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].model(), "qwen2.5:0.5b");
        assert_eq!(models[0].endpoint(), "http://127.0.0.1:11434");
        assert_eq!(models[0].timeout(), Duration::from_secs(120));
        assert!(config.data_plane().smart_home_tool_grants().is_empty());
    }

    #[test]
    fn parses_bounded_smart_home_tool_grants_without_weakening_existing_configs() {
        let source = format!(
            r#"{VALID}

[data_plane]
channel_keys = []
ollama_models = [
  {{ model = "qwen2.5:0.5b", endpoint = "http://127.0.0.1:11434", timeout = 120000 }},
]
smart_home_tool_grants = [
  {{ grant_id = "grant-weather-list", principal_id = "weather-level-one", tool_id = "smart_home.list_devices", granted_by = "operator:local", granted_at_ms = 1000, expires_at_ms = 2000 }},
  {{ grant_id = "grant-weather-state", principal_id = "weather-level-one", tool_id = "smart_home.get_state", granted_by = "operator:local", granted_at_ms = 1000, status = "revoked" }},
]
"#
        );
        let config = parse_config(&source).unwrap();
        let grants = config.data_plane().smart_home_tool_grants();
        assert_eq!(grants.len(), 2);
        assert_eq!(grants[0].grant_id(), "grant-weather-list");
        assert_eq!(grants[0].principal_id(), "weather-level-one");
        assert_eq!(grants[0].tool_id(), "smart_home.list_devices");
        assert_eq!(grants[0].granted_by(), "operator:local");
        assert_eq!(grants[0].granted_at_ms(), 1_000);
        assert_eq!(grants[0].expires_at_ms(), Some(2_000));
        assert_eq!(grants[0].status(), SmartHomeToolGrantStatus::Active);
        assert_eq!(grants[1].status(), SmartHomeToolGrantStatus::Revoked);

        assert_eq!(
            parse_config(&source.replace("expires_at_ms = 2000", "expires_at_ms = 1000")),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            parse_config(&source.replace("smart_home.list_devices", "network.fetch")),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            parse_config(&source.replace("grant-weather-state", "grant-weather-list")),
            Err(ConfigError::Duplicate)
        );
        assert_eq!(
            parse_config(&source.replace("status = \"revoked\"", "status = \"expired\"")),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            parse_config(&source.replace(
                "status = \"revoked\"",
                "status = \"revoked\", surprise = true"
            )),
            Err(ConfigError::Unknown)
        );
    }

    #[test]
    fn data_plane_declarations_are_canonical_unique_and_closed() {
        let section = r#"

[data_plane]
channel_keys = [
  { pipeline_id = "018f0c10-7b4a-7cc0-8000-000000000001", agent_id = "weather", channel_id = "018f0c10-7b4a-7cc0-8000-000000000002", access = "read", private_key_path = "~/receiver.bin" },
]
ollama_models = [
  { model = "qwen2.5:0.5b", endpoint = "http://127.0.0.1:11434", timeout = 120000 },
]
"#;
        let valid = format!("{VALID}{section}");
        assert!(parse_config(&valid).is_ok());
        assert_eq!(
            parse_config(&valid.replace("018f0c10", "018F0C10")),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            parse_config(&valid.replace("access = \"read\"", "access = \"both\"")),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            parse_config(&valid.replace(
                "private_key_path = \"~/receiver.bin\"",
                "private_key_path = \"~/receiver.bin\", extra = true"
            )),
            Err(ConfigError::Unknown)
        );
        assert_eq!(
            parse_config(&valid.replace(
                "  { model = \"qwen2.5:0.5b\", endpoint = \"http://127.0.0.1:11434\", timeout = 120000 },",
                "  { model = \"qwen2.5:0.5b\", endpoint = \"http://127.0.0.1:11434\", timeout = 120000 },\n  { model = \"qwen2.5:0.5b\", endpoint = \"http://localhost:11434\", timeout = 120000 },"
            )),
            Err(ConfigError::Duplicate)
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
            config.orchestrator().state_dir().resolve(&home).unwrap(),
            home.join(".chief-of-staff/state/")
        );
        assert_eq!(
            config
                .orchestrator()
                .credential_path()
                .resolve(&home)
                .unwrap(),
            home.join(".chief-of-staff/run/operator.credential")
        );
        assert_eq!(
            config.host_defaults().executable().resolve(&home).unwrap(),
            home.join(".chief-of-staff/bin/chief-of-staff-host")
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
            parse_config(&VALID.replace("port = 7463", "port = 0")),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            parse_config(&VALID.replace("port = 7463", "port = 65536")),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            parse_config(&VALID.replace("port = 7463", "port = \"7463\"")),
            Err(ConfigError::InvalidType)
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
            parse_config(&VALID.replace("bootstrap_timeout = 10_000", "bootstrap_timeout = 0")),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            parse_config(
                &VALID.replace("bootstrap_timeout = 10_000", "bootstrap_timeout = 300_001")
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

    /// Both restart-intensity keys are optional, and absent means the defaults.
    ///
    /// This is what lets a config file written before D18R R2 existed keep
    /// loading unchanged -- the bound arrives without an operator edit.
    #[test]
    fn restart_intensity_keys_are_optional_and_override_the_defaults() {
        let config = parse_config(VALID).expect("the fixture omits both keys");
        assert_eq!(
            config.host_defaults().restart_window_ns(),
            default_restart_window_ns()
        );
        assert_eq!(
            config.host_defaults().max_restarts_per_window(),
            default_max_restarts_per_window()
        );

        let overridden = VALID.replace(
            "graceful_stop_timeout = 5_000",
            "graceful_stop_timeout = 5_000\nrestart_window = 30_000\nmax_restarts_per_window = 2",
        );
        let config = parse_config(&overridden).expect("both keys are accepted");
        assert_eq!(config.host_defaults().restart_window_ns(), 30_000_000_000);
        assert_eq!(config.host_defaults().max_restarts_per_window(), 2);
    }

    /// A zero for either key is refused rather than read as "never restart",
    /// and an unbounded window is refused rather than silently saturating.
    #[test]
    fn a_zero_restart_intensity_is_refused_by_the_parser() {
        for line in ["restart_window = 0", "max_restarts_per_window = 0"] {
            let source = VALID.replace(
                "graceful_stop_timeout = 5_000",
                &format!("graceful_stop_timeout = 5_000\n{line}"),
            );
            assert!(
                parse_config(&source).is_err(),
                "expected `{line}` to be refused"
            );
        }
    }
}
