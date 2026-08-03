//! Strict trusted package-key loading for the D18 Chief daemon.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_daemon_config::{KeyringConfig, TrustedKeyType};
use chief_of_staff_host_runtime::{PackageKeyType, PackageKeyring, TrustedPackageKey};
use chief_of_staff_tool_api::PrivilegeTier;
use core::fmt::{self, Display, Formatter};
use std::fs::{self, File, Metadata};
use std::io::Read;
use std::path::Path;

const ED25519_PUBLIC_KEY_BYTES: usize = 32;

/// Stable payload-blind failure while constructing the daemon package keyring.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyringLoadError {
    /// The explicit home directory could not resolve a validated config path.
    InvalidHome,
    /// A configured key path could not be inspected, opened, or read.
    KeyFileUnavailable,
    /// A configured key path was a symlink or was not a regular file.
    KeyFileNotRegular,
    /// The path stopped naming the opened file during validation.
    KeyFileChanged,
    /// A key file did not contain exactly one raw 32-byte Ed25519 public key.
    InvalidKeyLength,
    /// A 32-byte key was not a canonical non-identity prime-subgroup point.
    InvalidPublicKey,
    /// A trusted-key declaration violated a host-runtime keyring invariant.
    InvalidTrustDeclaration,
}

impl Display for KeyringLoadError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidHome => "chief daemon keyring: invalid home directory",
            Self::KeyFileUnavailable => "chief daemon keyring: key file unavailable",
            Self::KeyFileNotRegular => "chief daemon keyring: key path is not a regular file",
            Self::KeyFileChanged => "chief daemon keyring: key path changed during validation",
            Self::InvalidKeyLength => "chief daemon keyring: invalid public-key length",
            Self::InvalidPublicKey => "chief daemon keyring: invalid Ed25519 public key",
            Self::InvalidTrustDeclaration => {
                "chief daemon keyring: invalid trusted-key declaration"
            }
        })
    }
}

impl std::error::Error for KeyringLoadError {}

/// Load every configured public key into a package-verification keyring.
///
/// Key files use the same raw representation as the repository Ed25519 API:
/// exactly 32 bytes with no text encoding, header, or trailing newline. Paths
/// are resolved only against `home`; this function never consults the process
/// environment.
pub fn load_package_keyring(
    config: &KeyringConfig,
    home: &Path,
) -> Result<PackageKeyring, KeyringLoadError> {
    let mut keyring = PackageKeyring::new();
    for declaration in config.trusted_keys() {
        let path = declaration
            .path()
            .resolve(home)
            .map_err(|_| KeyringLoadError::InvalidHome)?;
        let public_key = read_public_key(&path)?;
        let (key_type, maximum_tier) = match declaration.key_type() {
            TrustedKeyType::Production => (PackageKeyType::Production, PrivilegeTier::Tier3),
            TrustedKeyType::Developer => (PackageKeyType::Developer, PrivilegeTier::Tier1),
        };
        let trusted = TrustedPackageKey::new(declaration.id(), key_type, public_key, maximum_tier)
            .map_err(|_| KeyringLoadError::InvalidTrustDeclaration)?;
        keyring
            .trust(trusted)
            .map_err(|_| KeyringLoadError::InvalidTrustDeclaration)?;
    }
    Ok(keyring)
}

fn read_public_key(path: &Path) -> Result<[u8; ED25519_PUBLIC_KEY_BYTES], KeyringLoadError> {
    let before = fs::symlink_metadata(path).map_err(|_| KeyringLoadError::KeyFileUnavailable)?;
    if !before.file_type().is_file() {
        return Err(KeyringLoadError::KeyFileNotRegular);
    }

    let file = File::open(path).map_err(|_| KeyringLoadError::KeyFileUnavailable)?;
    let opened = file
        .metadata()
        .map_err(|_| KeyringLoadError::KeyFileUnavailable)?;
    let after = fs::symlink_metadata(path).map_err(|_| KeyringLoadError::KeyFileUnavailable)?;
    if !opened.file_type().is_file() || !after.file_type().is_file() {
        return Err(KeyringLoadError::KeyFileNotRegular);
    }
    if !same_file(&before, &opened) || !same_file(&after, &opened) {
        return Err(KeyringLoadError::KeyFileChanged);
    }

    let mut bytes = Vec::with_capacity(ED25519_PUBLIC_KEY_BYTES + 1);
    file.take((ED25519_PUBLIC_KEY_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| KeyringLoadError::KeyFileUnavailable)?;
    let public_key = bytes
        .try_into()
        .map_err(|_| KeyringLoadError::InvalidKeyLength)?;
    if !coding_adventures_ed25519::is_valid_public_key(&public_key) {
        return Err(KeyringLoadError::InvalidPublicKey);
    }
    Ok(public_key)
}

#[cfg(unix)]
fn same_file(left: &Metadata, right: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(windows)]
fn same_file(left: &Metadata, right: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    left.volume_serial_number() == right.volume_serial_number()
        && left.file_index() == right.file_index()
}

#[cfg(not(any(unix, windows)))]
fn same_file(left: &Metadata, right: &Metadata) -> bool {
    left.len() == right.len()
        && left.modified().ok() == right.modified().ok()
        && left.created().ok() == right.created().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chief_of_staff_daemon_config::{parse_config, ChiefConfig};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(label: &str) -> Self {
            let sequence = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "chief-daemon-keyring-{label}-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn config(keys: &str) -> ChiefConfig {
        parse_config(&format!(
            r#"
[orchestrator]
bind = "127.0.0.1"
packages_dir = "~/agents"

[keyring]
trusted_keys = [{keys}]

[hosts.defaults]
restart_policy = "on-failure"
health_check_interval = 5000

[vault]
storage_path = "~/vault"
default_lease_ttl = 30
container = true

[privilege]
tier_1_auto_approve_timeout = 5
biometric_timeout = 30
hardware_key_timeout = 60
"#
        ))
        .unwrap()
    }

    fn public_key(seed: u8) -> [u8; ED25519_PUBLIC_KEY_BYTES] {
        coding_adventures_ed25519::generate_keypair(&[seed; 32]).0
    }

    #[test]
    fn loads_raw_keys_and_preserves_production_and_developer_ceilings() {
        let home = TestDir::new("valid");
        let production_public = public_key(7);
        let developer_public = public_key(9);
        fs::write(home.path().join("production.pub"), production_public).unwrap();
        fs::write(home.path().join("developer.pub"), developer_public).unwrap();
        let config = config(
            r#"
  { id = "prod-001", path = "~/production.pub", type = "production" },
  { id = "dev-local", path = "~/developer.pub", type = "developer" }
"#,
        );

        let keyring = load_package_keyring(config.keyring(), home.path()).unwrap();
        let production = keyring.trusted_key("prod-001").unwrap();
        assert_eq!(production.public_key, production_public);
        assert_eq!(production.key_type, PackageKeyType::Production);
        assert_eq!(production.maximum_tier, PrivilegeTier::Tier3);
        let developer = keyring.trusted_key("dev-local").unwrap();
        assert_eq!(developer.public_key, developer_public);
        assert_eq!(developer.key_type, PackageKeyType::Developer);
        assert_eq!(developer.maximum_tier, PrivilegeTier::Tier1);
    }

    #[test]
    fn only_configured_identifiers_gain_trust() {
        let home = TestDir::new("configured-only");
        fs::write(home.path().join("trusted.pub"), public_key(11)).unwrap();
        let config = config(r#"{ id = "prod", path = "~/trusted.pub", type = "production" }"#);
        let keyring = load_package_keyring(config.keyring(), home.path()).unwrap();
        assert!(keyring.trusted_key("prod").is_some());
        assert!(keyring.trusted_key("unconfigured").is_none());
    }

    #[test]
    fn rejects_missing_non_regular_and_wrong_length_paths() {
        let home = TestDir::new("invalid-files");
        let cases = [
            ("missing.pub", KeyringLoadError::KeyFileUnavailable),
            ("directory.pub", KeyringLoadError::KeyFileNotRegular),
            ("short.pub", KeyringLoadError::InvalidKeyLength),
            ("long.pub", KeyringLoadError::InvalidKeyLength),
        ];
        fs::create_dir(home.path().join("directory.pub")).unwrap();
        fs::write(home.path().join("short.pub"), [0u8; 31]).unwrap();
        fs::write(home.path().join("long.pub"), [0u8; 33]).unwrap();

        for (path, expected) in cases {
            let config = config(&format!(
                r#"{{ id = "key", path = "~/{path}", type = "production" }}"#
            ));
            assert_eq!(
                load_package_keyring(config.keyring(), home.path()).unwrap_err(),
                expected
            );
        }
    }

    #[test]
    fn invalid_home_is_rejected_without_environment_fallback() {
        let config = config(r#"{ id = "prod", path = "~/trusted.pub", type = "production" }"#);
        assert_eq!(
            load_package_keyring(config.keyring(), Path::new("relative-home")).unwrap_err(),
            KeyringLoadError::InvalidHome
        );
    }

    #[test]
    fn rejects_invalid_ed25519_points() {
        let home = TestDir::new("invalid-points");
        let mut identity = [0u8; ED25519_PUBLIC_KEY_BYTES];
        identity[0] = 1;
        fs::write(home.path().join("identity.pub"), identity).unwrap();
        let config = config(r#"{ id = "prod", path = "~/identity.pub", type = "production" }"#);

        assert_eq!(
            load_package_keyring(config.keyring(), home.path()).unwrap_err(),
            KeyringLoadError::InvalidPublicKey
        );
    }

    #[cfg(unix)]
    #[test]
    fn final_component_symlink_is_rejected() {
        use std::os::unix::fs::symlink;

        let home = TestDir::new("symlink");
        fs::write(home.path().join("real.pub"), public_key(1)).unwrap();
        symlink("real.pub", home.path().join("linked.pub")).unwrap();
        let config = config(r#"{ id = "prod", path = "~/linked.pub", type = "production" }"#);
        assert_eq!(
            load_package_keyring(config.keyring(), home.path()).unwrap_err(),
            KeyringLoadError::KeyFileNotRegular
        );
    }

    #[test]
    fn errors_are_stable_and_payload_blind() {
        assert_eq!(
            KeyringLoadError::KeyFileUnavailable.to_string(),
            "chief daemon keyring: key file unavailable"
        );
        assert_eq!(
            KeyringLoadError::InvalidKeyLength.to_string(),
            "chief daemon keyring: invalid public-key length"
        );
        assert_eq!(
            KeyringLoadError::InvalidPublicKey.to_string(),
            "chief daemon keyring: invalid Ed25519 public key"
        );
    }
}
