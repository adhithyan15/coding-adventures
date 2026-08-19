//! Strict storage-neutral V1 configuration for vault-pm clients.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_toml_parser::{try_parse_toml, TomlParseError};
use core::fmt::{self, Debug, Display, Formatter};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use std::collections::{BTreeMap, BTreeSet};

/// The only configuration version accepted by this crate.
pub const FORMAT_VERSION_V1: u16 = 1;
/// Default idle-lock policy for a newly created vault.
pub const DEFAULT_AUTO_LOCK_SECONDS: u32 = 300;
/// Default clipboard-clear policy for a newly created vault.
pub const DEFAULT_CLIPBOARD_CLEAR_SECONDS: u32 = 30;

const MAX_CONFIG_BYTES: usize = 64 * 1024;
const MAX_NAME_BYTES: usize = 64;
const MAX_LOCATION_BYTES: usize = 4096;
const MAX_CREDENTIAL_REF_BYTES: usize = 256;
const MAX_VAULTS: usize = 64;
const MAX_STORES: usize = 64;
const MAX_REMOTE_STORES: usize = 16;
const MAX_AUTO_LOCK_SECONDS: u32 = 24 * 60 * 60;
const MAX_CLIPBOARD_CLEAR_SECONDS: u32 = 60 * 60;

/// Stable payload-blind configuration failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigError {
    /// TOML tokenization or parsing failed.
    Toml(TomlParseError),
    /// The source exceeded the V1 byte bound.
    InputTooLarge,
    /// Array-of-table syntax is outside the V1 schema.
    UnsupportedArrayTable,
    /// A table, key, list entry, or locator was duplicated.
    Duplicate,
    /// A required table or field was absent.
    Missing,
    /// A declaration was outside the closed V1 schema.
    Unknown,
    /// A value used the wrong TOML kind.
    InvalidType,
    /// The declared configuration version is unsupported.
    UnsupportedVersion,
    /// A vault or storage name violated the bounded identifier grammar.
    InvalidName,
    /// A bounded policy, location, or credential reference was invalid.
    InvalidValue,
    /// A vault locator was not canonical lowercase 32-byte hexadecimal.
    InvalidLocator,
    /// A vault referred to a storage declaration that does not exist.
    UnknownStorage,
}

impl Display for ConfigError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Toml(_) => "vault-pm config: malformed TOML",
            Self::InputTooLarge => "vault-pm config: input too large",
            Self::UnsupportedArrayTable => "vault-pm config: array tables unsupported",
            Self::Duplicate => "vault-pm config: duplicate declaration",
            Self::Missing => "vault-pm config: required declaration missing",
            Self::Unknown => "vault-pm config: unknown declaration",
            Self::InvalidType => "vault-pm config: invalid value type",
            Self::UnsupportedVersion => "vault-pm config: unsupported version",
            Self::InvalidName => "vault-pm config: invalid name",
            Self::InvalidValue => "vault-pm config: invalid value",
            Self::InvalidLocator => "vault-pm config: invalid vault locator",
            Self::UnknownStorage => "vault-pm config: unknown storage reference",
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
    fn from(value: TomlParseError) -> Self {
        Self::Toml(value)
    }
}

/// A bounded bare TOML identifier used for vault and storage aliases.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConfigName(String);

impl ConfigName {
    /// Validate a vault or storage alias.
    pub fn new(value: impl Into<String>) -> Result<Self, ConfigError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= MAX_NAME_BYTES
            && value.bytes().enumerate().all(|(index, byte)| {
                byte.is_ascii_alphanumeric() || (index > 0 && matches!(byte, b'_' | b'-'))
            });
        if !valid {
            return Err(ConfigError::InvalidName);
        }
        Ok(Self(value))
    }

    /// Borrow the validated spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Debug for ConfigName {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("ConfigName").field(&self.0).finish()
    }
}

/// Opaque random locator for one vault's signed bootstrap chain.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VaultLocator([u8; 32]);

impl VaultLocator {
    /// Construct a locator from exact caller-generated random bytes.
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrow the exact locator bytes for application-layer conversion.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    fn parse(value: &str) -> Result<Self, ConfigError> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(ConfigError::InvalidLocator);
        }
        let mut bytes = [0_u8; 32];
        for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
            bytes[index] = (hex_nibble(pair[0])? << 4) | hex_nibble(pair[1])?;
        }
        Ok(Self(bytes))
    }

    fn encode(self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut encoded = String::with_capacity(64);
        for byte in self.0 {
            encoded.push(char::from(HEX[usize::from(byte >> 4)]));
            encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        encoded
    }
}

impl Debug for VaultLocator {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("VaultLocator(<redacted>)")
    }
}

fn hex_nibble(byte: u8) -> Result<u8, ConfigError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err(ConfigError::InvalidLocator),
    }
}

/// Storage adapter kind selected by configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageKind {
    /// Local or mounted filesystem storage.
    Filesystem,
    /// A user-managed removable or third-party-synced folder (VLT-PM00 §12,
    /// §23 item 14).
    ///
    /// Same on-disk immutable object format as [`Self::Filesystem`] — this is
    /// a variant, not a new transport. The only difference is what the
    /// storage layer *assumes*: a plain `filesystem` location is exclusively
    /// written by this product, so a foreign file appearing next to its
    /// objects is always worth investigating, whereas a `removable` location
    /// may also be written by a third-party sync tool (Dropbox, OneDrive,
    /// Syncthing, a NAS client) or physically moved between machines, so
    /// `vault-pm-storage-removable`'s conflict-copy detection is expected to
    /// fire there in ordinary use and is reported as a warning rather than
    /// silently proceeding or refusing to open.
    Removable,
    /// Google Drive storage.
    GoogleDrive,
    /// WebDAV storage.
    WebDav,
    /// S3-compatible object storage.
    S3,
}

impl StorageKind {
    /// Parse the closed set of TOML/CLI spellings for a storage kind.
    pub fn parse(value: &str) -> Result<Self, ConfigError> {
        match value {
            "filesystem" => Ok(Self::Filesystem),
            "removable" => Ok(Self::Removable),
            "gdrive" => Ok(Self::GoogleDrive),
            "webdav" => Ok(Self::WebDav),
            "s3" => Ok(Self::S3),
            _ => Err(ConfigError::InvalidValue),
        }
    }

    /// Return the canonical TOML/CLI spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Filesystem => "filesystem",
            Self::Removable => "removable",
            Self::GoogleDrive => "gdrive",
            Self::WebDav => "webdav",
            Self::S3 => "s3",
        }
    }

    /// Whether this kind is backed by an ordinary local directory tree in the
    /// `storage-fs` on-disk shape (VLT-PM00 §12's `filesystem`/`removable`
    /// rows). Cloud kinds are not, and stay `Unsupported` in Phase 1B.
    pub const fn is_local_directory(self) -> bool {
        matches!(self, Self::Filesystem | Self::Removable)
    }
}

/// Adapter-owned bounded location string.
#[derive(Clone, PartialEq, Eq)]
pub struct StorageLocation(String);

impl StorageLocation {
    /// Validate an opaque provider location without interpreting its syntax.
    pub fn new(value: impl Into<String>) -> Result<Self, ConfigError> {
        let value = value.into();
        validate_opaque(&value, MAX_LOCATION_BYTES)?;
        Ok(Self(value))
    }

    /// Borrow the value for the selected storage adapter.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Debug for StorageLocation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("StorageLocation(<redacted>)")
    }
}

/// Opaque reference to provider credentials held outside configuration.
#[derive(Clone, PartialEq, Eq)]
pub struct CredentialRef(String);

impl CredentialRef {
    /// Validate a credential reference. The literal `none` means no credential.
    pub fn new(value: impl Into<String>) -> Result<Self, ConfigError> {
        let value = value.into();
        validate_opaque(&value, MAX_CREDENTIAL_REF_BYTES)?;
        Ok(Self(value))
    }

    /// Construct the no-credential marker used by local filesystems.
    pub fn none() -> Self {
        Self("none".to_owned())
    }

    /// Borrow the adapter-facing reference.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Debug for CredentialRef {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("CredentialRef(<redacted>)")
    }
}

fn validate_opaque(value: &str, max: usize) -> Result<(), ConfigError> {
    if value.is_empty()
        || value.len() > max
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(ConfigError::InvalidValue);
    }
    Ok(())
}

/// One named storage adapter declaration.
#[derive(Clone, PartialEq, Eq)]
pub struct StorageConfigV1 {
    kind: StorageKind,
    location: StorageLocation,
    credential_ref: CredentialRef,
}

impl StorageConfigV1 {
    /// Construct a validated storage declaration.
    pub fn new(
        kind: StorageKind,
        location: StorageLocation,
        credential_ref: CredentialRef,
    ) -> Self {
        Self {
            kind,
            location,
            credential_ref,
        }
    }

    /// Return the adapter kind.
    pub const fn kind(&self) -> StorageKind {
        self.kind
    }

    /// Return the adapter-owned location.
    pub fn location(&self) -> &StorageLocation {
        &self.location
    }

    /// Return the external credential reference.
    pub fn credential_ref(&self) -> &CredentialRef {
        &self.credential_ref
    }
}

impl Debug for StorageConfigV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StorageConfigV1")
            .field("kind", &self.kind)
            .field("location", &"<redacted>")
            .field("credential_ref", &"<redacted>")
            .finish()
    }
}

/// One named vault's locator, replica set, and local secret-lifetime policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VaultConfigV1 {
    locator: VaultLocator,
    local_store: ConfigName,
    remote_stores: Vec<ConfigName>,
    auto_lock_seconds: u32,
    clipboard_clear_seconds: u32,
}

impl VaultConfigV1 {
    /// Construct and validate one vault declaration.
    pub fn new(
        locator: VaultLocator,
        local_store: ConfigName,
        remote_stores: Vec<ConfigName>,
        auto_lock_seconds: u32,
        clipboard_clear_seconds: u32,
    ) -> Result<Self, ConfigError> {
        if remote_stores.len() > MAX_REMOTE_STORES
            || auto_lock_seconds == 0
            || auto_lock_seconds > MAX_AUTO_LOCK_SECONDS
            || clipboard_clear_seconds == 0
            || clipboard_clear_seconds > MAX_CLIPBOARD_CLEAR_SECONDS
        {
            return Err(ConfigError::InvalidValue);
        }
        let mut unique = BTreeSet::new();
        for name in &remote_stores {
            if name == &local_store || !unique.insert(name.clone()) {
                return Err(ConfigError::Duplicate);
            }
        }
        Ok(Self {
            locator,
            local_store,
            remote_stores,
            auto_lock_seconds,
            clipboard_clear_seconds,
        })
    }

    /// Return the opaque bootstrap locator.
    pub const fn locator(&self) -> VaultLocator {
        self.locator
    }

    /// Return the primary local storage alias.
    pub fn local_store(&self) -> &ConfigName {
        &self.local_store
    }

    /// Return ordered remote replica aliases.
    pub fn remote_stores(&self) -> &[ConfigName] {
        &self.remote_stores
    }

    /// Return the idle-lock timeout in seconds.
    pub const fn auto_lock_seconds(&self) -> u32 {
        self.auto_lock_seconds
    }

    /// Return the clipboard-clear timeout in seconds.
    pub const fn clipboard_clear_seconds(&self) -> u32 {
        self.clipboard_clear_seconds
    }
}

/// Complete validated V1 client configuration.
#[derive(Clone, PartialEq, Eq)]
pub struct VaultPmConfigV1 {
    default_vault: ConfigName,
    vaults: BTreeMap<ConfigName, VaultConfigV1>,
    storage: BTreeMap<ConfigName, StorageConfigV1>,
}

impl VaultPmConfigV1 {
    /// Construct a configuration and validate all cross-references.
    pub fn new(
        default_vault: ConfigName,
        vaults: BTreeMap<ConfigName, VaultConfigV1>,
        storage: BTreeMap<ConfigName, StorageConfigV1>,
    ) -> Result<Self, ConfigError> {
        if vaults.is_empty()
            || vaults.len() > MAX_VAULTS
            || storage.is_empty()
            || storage.len() > MAX_STORES
        {
            return Err(ConfigError::InvalidValue);
        }
        if !vaults.contains_key(&default_vault) {
            return Err(ConfigError::Missing);
        }
        let mut locators = BTreeSet::new();
        for vault in vaults.values() {
            if !locators.insert(vault.locator) {
                return Err(ConfigError::Duplicate);
            }
            if !storage.contains_key(&vault.local_store)
                || vault
                    .remote_stores
                    .iter()
                    .any(|name| !storage.contains_key(name))
            {
                return Err(ConfigError::UnknownStorage);
            }
        }
        let config = Self {
            default_vault,
            vaults,
            storage,
        };
        if render_config(&config).len() > MAX_CONFIG_BYTES {
            return Err(ConfigError::InvalidValue);
        }
        Ok(config)
    }

    /// Return the default vault alias.
    pub fn default_vault(&self) -> &ConfigName {
        &self.default_vault
    }

    /// Return all vault declarations in deterministic name order.
    pub fn vaults(&self) -> &BTreeMap<ConfigName, VaultConfigV1> {
        &self.vaults
    }

    /// Return all storage declarations in deterministic name order.
    pub fn storage(&self) -> &BTreeMap<ConfigName, StorageConfigV1> {
        &self.storage
    }

    /// Select a vault by alias, or the configured default when absent.
    pub fn select_vault(&self, name: Option<&ConfigName>) -> Option<&VaultConfigV1> {
        self.vaults.get(name.unwrap_or(&self.default_vault))
    }
}

impl Debug for VaultPmConfigV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VaultPmConfigV1")
            .field("default_vault", &self.default_vault)
            .field("vault_count", &self.vaults.len())
            .field("storage_count", &self.storage.len())
            .finish()
    }
}

/// Parse and fully validate one closed-schema V1 TOML document.
pub fn parse_config(source: &str) -> Result<VaultPmConfigV1, ConfigError> {
    if source.len() > MAX_CONFIG_BYTES {
        return Err(ConfigError::InputTooLarge);
    }
    let ast = try_parse_toml(source)?;
    let mut document = RawDocument::from_ast(&ast)?;
    let version = expect_integer(document.take_root("format_version")?)?;
    if version != i64::from(FORMAT_VERSION_V1) {
        return Err(ConfigError::UnsupportedVersion);
    }
    let default_vault = ConfigName::new(expect_string(document.take_root("default_vault")?)?)?;

    let vault_names = document.table_names("vaults")?;
    let storage_names = document.table_names("storage")?;
    if vault_names.is_empty() || storage_names.is_empty() {
        return Err(ConfigError::Missing);
    }

    let mut vaults = BTreeMap::new();
    for name in vault_names {
        let config_name = ConfigName::new(name.clone())?;
        let table = ["vaults", name.as_str()];
        let locator =
            VaultLocator::parse(&expect_string(document.take(&table, "vault_locator")?)?)?;
        let local_store = ConfigName::new(expect_string(document.take(&table, "local_store")?)?)?;
        let remote_stores = expect_string_array(document.take(&table, "remote_stores")?)?
            .into_iter()
            .map(ConfigName::new)
            .collect::<Result<Vec<_>, _>>()?;
        let auto_lock_seconds = bounded_u32(document.take(&table, "auto_lock_seconds")?)?;
        let clipboard_clear_seconds =
            bounded_u32(document.take(&table, "clipboard_clear_seconds")?)?;
        let vault = VaultConfigV1::new(
            locator,
            local_store,
            remote_stores,
            auto_lock_seconds,
            clipboard_clear_seconds,
        )?;
        vaults.insert(config_name, vault);
    }

    let mut storage = BTreeMap::new();
    for name in storage_names {
        let config_name = ConfigName::new(name.clone())?;
        let table = ["storage", name.as_str()];
        let kind = StorageKind::parse(&expect_string(document.take(&table, "kind")?)?)?;
        let location = StorageLocation::new(expect_string(document.take(&table, "path")?)?)?;
        let credential_ref =
            CredentialRef::new(expect_string(document.take(&table, "credential_ref")?)?)?;
        storage.insert(
            config_name,
            StorageConfigV1::new(kind, location, credential_ref),
        );
    }

    if !document.fields.is_empty() {
        return Err(ConfigError::Unknown);
    }
    VaultPmConfigV1::new(default_vault, vaults, storage)
}

/// Render validated V1 configuration as deterministic canonical TOML.
pub fn render_config(config: &VaultPmConfigV1) -> String {
    let mut output = format!(
        "format_version = {}\ndefault_vault = \"{}\"\n",
        FORMAT_VERSION_V1,
        escape_string(config.default_vault.as_str())
    );
    for (name, vault) in &config.vaults {
        output.push_str(&format!(
            "\n[vaults.{}]\nvault_locator = \"{}\"\nlocal_store = \"{}\"\nremote_stores = [{}]\nauto_lock_seconds = {}\nclipboard_clear_seconds = {}\n",
            name.as_str(),
            vault.locator.encode(),
            escape_string(vault.local_store.as_str()),
            vault
                .remote_stores
                .iter()
                .map(|remote| format!("\"{}\"", escape_string(remote.as_str())))
                .collect::<Vec<_>>()
                .join(", "),
            vault.auto_lock_seconds,
            vault.clipboard_clear_seconds
        ));
    }
    for (name, store) in &config.storage {
        output.push_str(&format!(
            "\n[storage.{}]\nkind = \"{}\"\npath = \"{}\"\ncredential_ref = \"{}\"\n",
            name.as_str(),
            store.kind.as_str(),
            escape_string(store.location.as_str()),
            escape_string(store.credential_ref.as_str())
        ));
    }
    output
}

fn escape_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn bounded_u32(value: RawValue) -> Result<u32, ConfigError> {
    let value = expect_integer(value)?;
    u32::try_from(value).map_err(|_| ConfigError::InvalidValue)
}

fn expect_integer(value: RawValue) -> Result<i64, ConfigError> {
    match value {
        RawValue::Integer(value) => Ok(value),
        _ => Err(ConfigError::InvalidType),
    }
}

fn expect_string(value: RawValue) -> Result<String, ConfigError> {
    match value {
        RawValue::String(value) => Ok(value),
        _ => Err(ConfigError::InvalidType),
    }
}

fn expect_string_array(value: RawValue) -> Result<Vec<String>, ConfigError> {
    let RawValue::Array(values) = value else {
        return Err(ConfigError::InvalidType);
    };
    values.into_iter().map(expect_string).collect()
}

#[derive(Clone, Debug, PartialEq)]
enum RawValue {
    String(String),
    Integer(i64),
    Array(Vec<Self>),
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

    fn take_root(&mut self, field: &str) -> Result<RawValue, ConfigError> {
        self.fields
            .remove(&vec![field.to_owned()])
            .ok_or(ConfigError::Missing)
    }

    fn take(&mut self, table: &[&str], field: &str) -> Result<RawValue, ConfigError> {
        let mut key = table
            .iter()
            .map(|part| (*part).to_owned())
            .collect::<Vec<_>>();
        key.push(field.to_owned());
        self.fields.remove(&key).ok_or(ConfigError::Missing)
    }

    fn table_names(&self, prefix: &str) -> Result<Vec<String>, ConfigError> {
        let mut names = Vec::new();
        for table in &self.tables {
            if table.len() != 2 || !matches!(table.first(), Some(value) if value == prefix) {
                if !matches!(table.first(), Some(value) if value == "vaults" || value == "storage")
                {
                    return Err(ConfigError::Unknown);
                }
                if table.first().map(String::as_str) == Some(prefix) {
                    return Err(ConfigError::Unknown);
                }
                continue;
            }
            names.push(table[1].clone());
        }
        Ok(names)
    }
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
                simple.token().and_then(|token| {
                    (token.effective_type_name() == "BARE_KEY").then(|| token.value.clone())
                })
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
                    "BASIC_STRING" => RawValue::String(decode_basic_string(&token.value)?),
                    "LITERAL_STRING" => RawValue::String(token.value.clone()),
                    "INTEGER" => RawValue::Integer(parse_toml_integer(&token.value)?),
                    _ => RawValue::Other,
                });
            }
            ASTNodeOrToken::Node(child) if child.rule_name == "array" => {
                return parse_array(child).map(RawValue::Array);
            }
            ASTNodeOrToken::Node(_) => {}
        }
    }
    Err(ConfigError::InvalidValue)
}

fn decode_basic_string(value: &str) -> Result<String, ConfigError> {
    let mut decoded = String::with_capacity(value.len());
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            decoded.push(character);
            continue;
        }
        let escaped = characters.next().ok_or(ConfigError::InvalidValue)?;
        match escaped {
            'b' => decoded.push('\u{0008}'),
            't' => decoded.push('\t'),
            'n' => decoded.push('\n'),
            'f' => decoded.push('\u{000c}'),
            'r' => decoded.push('\r'),
            '"' => decoded.push('"'),
            '\\' => decoded.push('\\'),
            'u' => decoded.push(decode_unicode_escape(&mut characters, 4)?),
            'U' => decoded.push(decode_unicode_escape(&mut characters, 8)?),
            _ => return Err(ConfigError::InvalidValue),
        }
    }
    Ok(decoded)
}

fn decode_unicode_escape(
    characters: &mut impl Iterator<Item = char>,
    digits: usize,
) -> Result<char, ConfigError> {
    let mut scalar = 0_u32;
    for _ in 0..digits {
        let digit = characters.next().ok_or(ConfigError::InvalidValue)?;
        scalar = scalar
            .checked_mul(16)
            .and_then(|value| {
                digit
                    .to_digit(16)
                    .and_then(|digit| value.checked_add(digit))
            })
            .ok_or(ConfigError::InvalidValue)?;
    }
    char::from_u32(scalar).ok_or(ConfigError::InvalidValue)
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
        return Ok(Vec::new());
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

#[cfg(test)]
mod tests {
    use super::*;

    const LOCATOR: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

    fn source() -> String {
        format!(
            r#"format_version = 1
default_vault = "personal"

[vaults.personal]
vault_locator = "{LOCATOR}"
local_store = "local"
remote_stores = ["backup"]
auto_lock_seconds = 300
clipboard_clear_seconds = 30

[storage.local]
kind = "filesystem"
path = "/private/vault-pm/objects"
credential_ref = "none"

[storage.backup]
kind = "gdrive"
path = "appDataFolder"
credential_ref = "google-primary"
"#
        )
    }

    #[test]
    fn parses_closed_storage_neutral_configuration() {
        let config = parse_config(&source()).unwrap();
        assert_eq!(config.default_vault().as_str(), "personal");
        let vault = config.select_vault(None).unwrap();
        assert_eq!(vault.locator().as_bytes()[0..4], [0, 17, 34, 51]);
        assert_eq!(vault.local_store().as_str(), "local");
        assert_eq!(vault.remote_stores()[0].as_str(), "backup");
        assert_eq!(vault.auto_lock_seconds(), 300);
        assert_eq!(vault.clipboard_clear_seconds(), 30);
        assert_eq!(
            config.storage()[&ConfigName::new("local").unwrap()].kind(),
            StorageKind::Filesystem
        );
        assert_eq!(
            config.storage()[&ConfigName::new("backup").unwrap()].kind(),
            StorageKind::GoogleDrive
        );
    }

    #[test]
    fn removable_kind_parses_renders_and_is_a_local_directory() {
        assert_eq!(StorageKind::parse("removable"), Ok(StorageKind::Removable));
        assert_eq!(StorageKind::Removable.as_str(), "removable");
        assert!(StorageKind::Removable.is_local_directory());
        assert!(StorageKind::Filesystem.is_local_directory());
        assert!(!StorageKind::GoogleDrive.is_local_directory());
        assert!(!StorageKind::WebDav.is_local_directory());
        assert!(!StorageKind::S3.is_local_directory());

        let mut config = parse_config(&source()).unwrap();
        config.storage.insert(
            ConfigName::new("thumbdrive").unwrap(),
            StorageConfigV1::new(
                StorageKind::Removable,
                StorageLocation::new("/media/usb/vault").unwrap(),
                CredentialRef::none(),
            ),
        );
        let rendered = render_config(&config);
        assert!(rendered.contains("kind = \"removable\""));
        assert_eq!(parse_config(&rendered).unwrap(), config);
    }

    #[test]
    fn canonical_render_round_trips_and_sorts_tables() {
        let config = parse_config(&source()).unwrap();
        let rendered = render_config(&config);
        assert!(
            rendered.find("[vaults.personal]").unwrap()
                < rendered.find("[storage.backup]").unwrap()
        );
        assert!(
            rendered.find("[storage.backup]").unwrap() < rendered.find("[storage.local]").unwrap()
        );
        assert_eq!(parse_config(&rendered).unwrap(), config);
        assert_eq!(render_config(&parse_config(&rendered).unwrap()), rendered);
    }

    #[test]
    fn empty_remote_list_round_trips() {
        let source = source().replace("[\"backup\"]", "[]");
        let config = parse_config(&source).unwrap();
        assert!(config
            .select_vault(None)
            .unwrap()
            .remote_stores()
            .is_empty());
        assert_eq!(parse_config(&render_config(&config)).unwrap(), config);
    }

    #[test]
    fn rejects_unknown_missing_duplicate_and_wrong_typed_fields() {
        let unknown = source().replace(
            "auto_lock_seconds = 300",
            "auto_lock_seconds = 300\nsecret = \"no\"",
        );
        assert_eq!(parse_config(&unknown), Err(ConfigError::Unknown));
        let missing = source().replace("credential_ref = \"none\"\n", "");
        assert_eq!(parse_config(&missing), Err(ConfigError::Missing));
        let duplicate = source().replace(
            "format_version = 1",
            "format_version = 1\nformat_version = 1",
        );
        assert_eq!(parse_config(&duplicate), Err(ConfigError::Duplicate));
        let wrong_type = source().replace("auto_lock_seconds = 300", "auto_lock_seconds = \"300\"");
        assert_eq!(parse_config(&wrong_type), Err(ConfigError::InvalidType));
    }

    #[test]
    fn rejects_unsupported_version_kind_and_table_shapes() {
        assert_eq!(
            parse_config(&source().replacen("format_version = 1", "format_version = 2", 1)),
            Err(ConfigError::UnsupportedVersion)
        );
        assert_eq!(
            parse_config(&source().replacen("kind = \"filesystem\"", "kind = \"ftp\"", 1)),
            Err(ConfigError::InvalidValue)
        );
        let array_table = source().replace("[storage.local]", "[[storage.local]]");
        assert_eq!(
            parse_config(&array_table),
            Err(ConfigError::UnsupportedArrayTable)
        );
        let nested = source().replace("[storage.local]", "[storage.local.extra]");
        assert_eq!(parse_config(&nested), Err(ConfigError::Unknown));
    }

    #[test]
    fn rejects_noncanonical_locators_and_duplicate_locator_identity() {
        assert_eq!(
            parse_config(&source().replace(LOCATOR, &LOCATOR.to_uppercase())),
            Err(ConfigError::InvalidLocator)
        );
        let second = format!(
            "\n[vaults.work]\nvault_locator = \"{LOCATOR}\"\nlocal_store = \"local\"\nremote_stores = []\nauto_lock_seconds = 300\nclipboard_clear_seconds = 30\n"
        );
        assert_eq!(
            parse_config(&(source() + &second)),
            Err(ConfigError::Duplicate)
        );
    }

    #[test]
    fn rejects_invalid_names_references_lists_and_policy_bounds() {
        assert_eq!(
            parse_config(&source().replace(
                "default_vault = \"personal\"",
                "default_vault = \"missing\""
            )),
            Err(ConfigError::Missing)
        );
        assert_eq!(
            parse_config(&source().replace("local_store = \"local\"", "local_store = \"missing\"")),
            Err(ConfigError::UnknownStorage)
        );
        assert_eq!(
            parse_config(&source().replace(
                "remote_stores = [\"backup\"]",
                "remote_stores = [\"local\"]"
            )),
            Err(ConfigError::Duplicate)
        );
        assert_eq!(
            parse_config(&source().replace("[vaults.personal]", "[vaults.bad.name]")),
            Err(ConfigError::Unknown)
        );
        assert_eq!(
            parse_config(&source().replace("auto_lock_seconds = 300", "auto_lock_seconds = 0")),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            parse_config(&source().replace(
                "clipboard_clear_seconds = 30",
                "clipboard_clear_seconds = 3601"
            )),
            Err(ConfigError::InvalidValue)
        );
    }

    #[test]
    fn rejects_oversize_and_malformed_inputs_without_payload_diagnostics() {
        assert_eq!(
            parse_config(&"x".repeat(MAX_CONFIG_BYTES + 1)),
            Err(ConfigError::InputTooLarge)
        );
        assert!(matches!(parse_config("["), Err(ConfigError::Toml(_))));
        for error in [
            ConfigError::InputTooLarge,
            ConfigError::Duplicate,
            ConfigError::Missing,
            ConfigError::Unknown,
            ConfigError::InvalidType,
            ConfigError::UnsupportedVersion,
            ConfigError::InvalidName,
            ConfigError::InvalidValue,
            ConfigError::InvalidLocator,
            ConfigError::UnknownStorage,
        ] {
            let rendered = format!("{error:?} {error}");
            assert!(!rendered.contains(LOCATOR));
            assert!(!rendered.contains("/private"));
        }
    }

    #[test]
    fn debug_redacts_locator_storage_location_and_credential_reference() {
        let config = parse_config(&source()).unwrap();
        let vault = config.select_vault(None).unwrap();
        let store = &config.storage()[&ConfigName::new("backup").unwrap()];
        assert_eq!(format!("{:?}", vault.locator()), "VaultLocator(<redacted>)");
        assert_eq!(
            format!("{:?}", store.location()),
            "StorageLocation(<redacted>)"
        );
        assert_eq!(
            format!("{:?}", store.credential_ref()),
            "CredentialRef(<redacted>)"
        );
        let debug = format!("{config:?}");
        assert!(!debug.contains(LOCATOR));
        assert!(!debug.contains("appDataFolder"));
    }

    #[test]
    fn constructors_enforce_cross_reference_and_identifier_invariants() {
        assert_eq!(ConfigName::new("bad.name"), Err(ConfigError::InvalidName));
        assert_eq!(
            StorageLocation::new(" line\n"),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(CredentialRef::new(""), Err(ConfigError::InvalidValue));
        let vault = VaultConfigV1::new(
            VaultLocator::new([7; 32]),
            ConfigName::new("local").unwrap(),
            vec![
                ConfigName::new("remote").unwrap(),
                ConfigName::new("remote").unwrap(),
            ],
            DEFAULT_AUTO_LOCK_SECONDS,
            DEFAULT_CLIPBOARD_CLEAR_SECONDS,
        );
        assert_eq!(vault, Err(ConfigError::Duplicate));
    }

    #[test]
    fn canonical_render_escapes_adapter_owned_strings() {
        let mut config = parse_config(&source()).unwrap();
        config.storage.insert(
            ConfigName::new("escaped").unwrap(),
            StorageConfigV1::new(
                StorageKind::WebDav,
                StorageLocation::new(r#"https://host/\"vault\folder"#).unwrap(),
                CredentialRef::new(r#"key\"alias"#).unwrap(),
            ),
        );
        config.storage.insert(
            ConfigName::new("archive").unwrap(),
            StorageConfigV1::new(
                StorageKind::S3,
                StorageLocation::new("bucket/prefix").unwrap(),
                CredentialRef::none(),
            ),
        );
        let rendered = render_config(&config);
        assert!(rendered.contains("kind = \"webdav\""));
        assert!(rendered.contains("kind = \"s3\""));
        assert!(rendered.contains("path = \"https://host/\\\\\\\"vault\\\\folder\""));
        assert_eq!(parse_config(&rendered).unwrap(), config);
        assert_eq!(CredentialRef::none().as_str(), "none");
    }

    #[test]
    fn public_views_and_debug_are_complete_but_redacted() {
        let config = parse_config(&source()).unwrap();
        assert_eq!(config.vaults().len(), 1);
        assert_eq!(
            config.select_vault(Some(&ConfigName::new("missing").unwrap())),
            None
        );
        let debug = format!("{:?}", config.storage()[&ConfigName::new("local").unwrap()]);
        assert!(debug.contains("Filesystem"));
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("/private"));
    }

    #[test]
    fn constructors_reject_empty_configuration_and_missing_storage() {
        assert_eq!(
            VaultPmConfigV1::new(
                ConfigName::new("personal").unwrap(),
                BTreeMap::new(),
                BTreeMap::new(),
            ),
            Err(ConfigError::InvalidValue)
        );
        let vault = VaultConfigV1::new(
            VaultLocator::new([9; 32]),
            ConfigName::new("local").unwrap(),
            Vec::new(),
            DEFAULT_AUTO_LOCK_SECONDS,
            DEFAULT_CLIPBOARD_CLEAR_SECONDS,
        )
        .unwrap();
        assert_eq!(
            VaultPmConfigV1::new(
                ConfigName::new("personal").unwrap(),
                BTreeMap::from([(ConfigName::new("personal").unwrap(), vault)]),
                BTreeMap::from([(
                    ConfigName::new("unused").unwrap(),
                    StorageConfigV1::new(
                        StorageKind::Filesystem,
                        StorageLocation::new("/objects").unwrap(),
                        CredentialRef::none(),
                    ),
                )]),
            ),
            Err(ConfigError::UnknownStorage)
        );
    }

    #[test]
    fn constructor_preserves_the_global_canonical_byte_bound() {
        let vault = VaultConfigV1::new(
            VaultLocator::new([3; 32]),
            ConfigName::new("store0").unwrap(),
            Vec::new(),
            DEFAULT_AUTO_LOCK_SECONDS,
            DEFAULT_CLIPBOARD_CLEAR_SECONDS,
        )
        .unwrap();
        let storage = (0..MAX_STORES)
            .map(|index| {
                (
                    ConfigName::new(format!("store{index}")).unwrap(),
                    StorageConfigV1::new(
                        StorageKind::Filesystem,
                        StorageLocation::new("x".repeat(MAX_LOCATION_BYTES)).unwrap(),
                        CredentialRef::none(),
                    ),
                )
            })
            .collect();
        assert_eq!(
            VaultPmConfigV1::new(
                ConfigName::new("personal").unwrap(),
                BTreeMap::from([(ConfigName::new("personal").unwrap(), vault)]),
                storage,
            ),
            Err(ConfigError::InvalidValue)
        );
    }

    #[test]
    fn parser_rejects_missing_table_families_and_non_string_values() {
        let without_storage = source().split("[storage.local]").next().unwrap().to_owned();
        assert_eq!(parse_config(&without_storage), Err(ConfigError::Missing));
        assert_eq!(
            parse_config(&source().replace("local_store = \"local\"", "local_store = true")),
            Err(ConfigError::InvalidType)
        );
        assert_eq!(
            parse_config(&source().replace("remote_stores = [\"backup\"]", "remote_stores = 1")),
            Err(ConfigError::InvalidType)
        );
        assert_eq!(
            parse_config(&source().replace("remote_stores = [\"backup\"]", "remote_stores = [1]")),
            Err(ConfigError::InvalidType)
        );
        assert_eq!(
            parse_config(&source().replace("[storage.local]", "[unexpected.local]")),
            Err(ConfigError::Unknown)
        );
        assert_eq!(
            parse_config(&(source() + "\n[storage.local]\n")),
            Err(ConfigError::Duplicate)
        );
    }

    #[test]
    fn integer_spellings_are_parsed_then_policy_checked() {
        let hexadecimal = source()
            .replace("auto_lock_seconds = 300", "auto_lock_seconds = 0x12c")
            .replace(
                "clipboard_clear_seconds = 30",
                "clipboard_clear_seconds = 0o36",
            );
        assert_eq!(
            parse_config(&hexadecimal)
                .unwrap()
                .select_vault(None)
                .unwrap()
                .auto_lock_seconds(),
            300
        );
        let binary = source().replace(
            "clipboard_clear_seconds = 30",
            "clipboard_clear_seconds = 0b11110",
        );
        assert_eq!(
            parse_config(&binary)
                .unwrap()
                .select_vault(None)
                .unwrap()
                .clipboard_clear_seconds(),
            30
        );
        assert_eq!(
            parse_config(&source().replace("auto_lock_seconds = 300", "auto_lock_seconds = -1")),
            Err(ConfigError::InvalidValue)
        );
    }

    #[test]
    fn every_stable_error_has_a_closed_display_and_toml_source() {
        let malformed = parse_config("[").unwrap_err();
        assert_eq!(malformed.to_string(), "vault-pm config: malformed TOML");
        assert!(std::error::Error::source(&malformed).is_some());
        assert!(std::error::Error::source(&ConfigError::Missing).is_none());
        assert_eq!(
            ConfigError::UnsupportedArrayTable.to_string(),
            "vault-pm config: array tables unsupported"
        );
    }

    #[test]
    fn basic_string_decoder_handles_every_v1_escape_and_fails_closed() {
        assert_eq!(
            decode_basic_string(r#"\b\t\n\f\r\"\\\u0041\U0001f642"#).unwrap(),
            "\u{0008}\t\n\u{000c}\r\"\\A🙂"
        );
        assert_eq!(decode_basic_string(r#"\q"#), Err(ConfigError::InvalidValue));
        assert_eq!(decode_basic_string("\\"), Err(ConfigError::InvalidValue));
        assert_eq!(
            decode_basic_string(r#"\u12"#),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            decode_basic_string(r#"\uZZZZ"#),
            Err(ConfigError::InvalidValue)
        );
        assert_eq!(
            decode_basic_string(r#"\uD800"#),
            Err(ConfigError::InvalidValue)
        );
    }
}
