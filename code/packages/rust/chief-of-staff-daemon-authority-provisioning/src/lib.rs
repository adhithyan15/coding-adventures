//! Explicit production authority provisioning for the D18 Chief daemon.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_channel_crypto::ChannelId;
use chief_of_staff_channel_endpoints::AgentId;
use chief_of_staff_daemon_config::{ChannelKeyAccess, ConfigPath, DataPlaneConfig};
use chief_of_staff_daemon_secret_file::{read_owner_only_secret, SecretFileError};
use chief_of_staff_host_data_plane::{
    ChannelKeyRegistrationError, ExactChannelKeyAuthority, ExactModelProviderRegistry,
    ModelProviderError,
};
use chief_of_staff_pipeline_bindings::{PipelineBindingError, PipelineId};
use coding_adventures_zeroize::Zeroizing;
use core::fmt::{self, Display, Formatter};
use llm_provider_ollama::{OllamaClient, OllamaConfigurationError};
use std::path::{Component, Path};
use std::sync::Arc;

const KEY_BYTES: usize = 32;

/// Stable payload-blind failure while provisioning production data-plane authorities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorityProvisioningError {
    /// The explicit home directory was not a safe absolute path.
    InvalidHome,
    /// A validated configuration identity could not enter its domain type.
    InvalidIdentity,
    /// One configured private-key file failed the owner-only read policy.
    SecretFile(SecretFileError),
    /// One exact channel-key registration conflicted or contained a placeholder.
    ChannelKey(ChannelKeyRegistrationError),
    /// One explicit Ollama client declaration was invalid.
    Ollama(OllamaConfigurationError),
    /// One exact model selector was empty or registered more than once.
    Model(ModelProviderError),
}

impl Display for AuthorityProvisioningError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidHome => "chief authority provisioning: invalid home directory",
            Self::InvalidIdentity => "chief authority provisioning: invalid identity",
            Self::SecretFile(_) => "chief authority provisioning: secret file failed",
            Self::ChannelKey(_) => "chief authority provisioning: channel key failed",
            Self::Ollama(_) => "chief authority provisioning: Ollama configuration failed",
            Self::Model(_) => "chief authority provisioning: model registration failed",
        })
    }
}

impl std::error::Error for AuthorityProvisioningError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SecretFile(error) => Some(error),
            Self::ChannelKey(error) => Some(error),
            Self::Ollama(error) => Some(error),
            Self::Model(error) => Some(error),
            Self::InvalidHome | Self::InvalidIdentity => None,
        }
    }
}

/// Immutable channel-key and exact-model authorities ready for daemon composition.
pub struct ProvisionedAuthorities {
    channel_keys: ExactChannelKeyAuthority,
    models: ExactModelProviderRegistry,
}

impl ProvisionedAuthorities {
    /// Borrow the exact zeroizing channel-key authority.
    pub fn channel_keys(&self) -> &ExactChannelKeyAuthority {
        &self.channel_keys
    }

    /// Borrow the exact model-provider authority.
    pub fn models(&self) -> &ExactModelProviderRegistry {
        &self.models
    }

    /// Transfer both immutable authorities to a production composition root.
    pub fn into_parts(self) -> (ExactChannelKeyAuthority, ExactModelProviderRegistry) {
        (self.channel_keys, self.models)
    }
}

/// Load every explicitly configured authority without environment or network access.
pub fn provision_authorities(
    config: &DataPlaneConfig,
    home: &Path,
) -> Result<ProvisionedAuthorities, AuthorityProvisioningError> {
    if !safe_absolute(home) {
        return Err(AuthorityProvisioningError::InvalidHome);
    }

    let mut channel_keys = ExactChannelKeyAuthority::new();
    for declaration in config.channel_keys() {
        let pipeline_id = PipelineId::new(declaration.pipeline_id()).map_err(identity_error)?;
        let agent_id = AgentId::new(declaration.agent_id().as_bytes().to_vec())
            .map_err(|_| AuthorityProvisioningError::InvalidIdentity)?;
        let channel_id = ChannelId(declaration.channel_id());
        match declaration.access() {
            ChannelKeyAccess::Read => {
                let path = declaration
                    .receiver_private_key_path()
                    .ok_or(AuthorityProvisioningError::InvalidIdentity)?;
                let private_key = load_key(path, home)?;
                channel_keys
                    .register_receiver(pipeline_id, &agent_id, channel_id, private_key)
                    .map_err(AuthorityProvisioningError::ChannelKey)?;
            }
            ChannelKeyAccess::Write => {
                let signing_path = declaration
                    .originator_signing_seed_path()
                    .ok_or(AuthorityProvisioningError::InvalidIdentity)?;
                let channel_path = declaration
                    .channel_master_key_path()
                    .ok_or(AuthorityProvisioningError::InvalidIdentity)?;
                let signing_seed = load_key(signing_path, home)?;
                let channel_key = load_key(channel_path, home)?;
                channel_keys
                    .register_originator(
                        pipeline_id,
                        &agent_id,
                        channel_id,
                        signing_seed,
                        channel_key,
                    )
                    .map_err(AuthorityProvisioningError::ChannelKey)?;
            }
        }
    }

    let mut models = ExactModelProviderRegistry::new();
    for declaration in config.ollama_models() {
        let client = OllamaClient::try_new(
            declaration.model(),
            declaration.endpoint(),
            declaration.timeout(),
        )
        .map_err(AuthorityProvisioningError::Ollama)?;
        models
            .register(declaration.model(), Arc::new(client))
            .map_err(AuthorityProvisioningError::Model)?;
    }

    Ok(ProvisionedAuthorities {
        channel_keys,
        models,
    })
}

fn identity_error(_error: PipelineBindingError) -> AuthorityProvisioningError {
    AuthorityProvisioningError::InvalidIdentity
}

fn load_key(
    path: &ConfigPath,
    home: &Path,
) -> Result<Zeroizing<[u8; KEY_BYTES]>, AuthorityProvisioningError> {
    let path = path
        .resolve(home)
        .map_err(|_| AuthorityProvisioningError::InvalidHome)?;
    let bytes =
        read_owner_only_secret(&path, KEY_BYTES).map_err(AuthorityProvisioningError::SecretFile)?;
    let mut key = Zeroizing::new([0; KEY_BYTES]);
    key.copy_from_slice(bytes.as_slice());
    Ok(key)
}

fn safe_absolute(path: &Path) -> bool {
    path.is_absolute()
        && !path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chief_of_staff_daemon_config::{parse_config, ChiefConfig};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "chief-authority-provisioning-{label}-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(fs::canonicalize(path).unwrap())
        }

        fn path(&self) -> &Path {
            &self.0
        }

        #[cfg(unix)]
        fn write_secret(&self, name: &str, bytes: &[u8]) {
            use std::os::unix::fs::PermissionsExt;
            let path = self.0.join(name);
            fs::write(&path, bytes).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn config(data_plane: &str) -> ChiefConfig {
        parse_config(&format!(
            r#"
[orchestrator]
bind = "127.0.0.1"
port = 7463
packages_dir = "~/agents"
state_dir = "~/state"
credential_path = "~/operator.credential"

[keyring]
trusted_keys = [{{ id = "dev", path = "~/dev.pub", type = "developer" }}]

[hosts.defaults]
restart_policy = "on-failure"
health_check_interval = 5000
executable = "~/chief-of-staff-host"
bootstrap_timeout = 10000
graceful_stop_timeout = 5000

[vault]
storage_path = "~/vault"
default_lease_ttl = 30
container = true

[privilege]
tier_1_auto_approve_timeout = 5
biometric_timeout = 30
hardware_key_timeout = 60

[data_plane]
{data_plane}
"#
        ))
        .unwrap()
    }

    #[test]
    fn provisions_exact_ollama_models_without_network_access() {
        let home = TestDirectory::new("models");
        let valid = config(
            r#"
channel_keys = []
ollama_models = [
  { model = "qwen2.5:0.5b", endpoint = "http://127.0.0.1:11434", timeout = 120000 },
]
"#,
        );
        let authorities = provision_authorities(valid.data_plane(), home.path()).unwrap();
        assert!(authorities.channel_keys().is_empty());
        assert_eq!(authorities.models().len(), 1);

        let invalid = config(
            r#"
channel_keys = []
ollama_models = [
  { model = "qwen2.5:0.5b", endpoint = "http://127.0.0.1:11434/path", timeout = 120000 },
]
"#,
        );
        assert_eq!(
            provision_authorities(invalid.data_plane(), home.path())
                .err()
                .unwrap(),
            AuthorityProvisioningError::Ollama(OllamaConfigurationError::InvalidEndpoint)
        );
    }

    #[test]
    fn rejects_invalid_home_and_missing_secret_without_disclosing_paths() {
        let home = TestDirectory::new("missing");
        let config = config(
            r#"
channel_keys = [
  { pipeline_id = "018f0c10-7b4a-7cc0-8000-000000000001", agent_id = "weather", channel_id = "018f0c10-7b4a-7cc0-8000-000000000002", access = "read", private_key_path = "~/missing.bin" },
]
ollama_models = []
"#,
        );
        assert_eq!(
            provision_authorities(config.data_plane(), Path::new("relative"))
                .err()
                .unwrap(),
            AuthorityProvisioningError::InvalidHome
        );
        let error = provision_authorities(config.data_plane(), home.path())
            .err()
            .unwrap();
        assert!(matches!(error, AuthorityProvisioningError::SecretFile(_)));
        assert_eq!(
            error.to_string(),
            "chief authority provisioning: secret file failed"
        );
        assert!(!error.to_string().contains("missing.bin"));
    }

    #[cfg(unix)]
    #[test]
    fn provisions_exact_read_and_write_keys_from_private_raw_files() {
        let home = TestDirectory::new("keys");
        home.write_secret("receiver.bin", &[0x11; KEY_BYTES]);
        home.write_secret("signing.bin", &[0x22; KEY_BYTES]);
        home.write_secret("channel.bin", &[0x33; KEY_BYTES]);
        let config = config(
            r#"
channel_keys = [
  { pipeline_id = "018f0c10-7b4a-7cc0-8000-000000000001", agent_id = "weather", channel_id = "018f0c10-7b4a-7cc0-8000-000000000002", access = "read", private_key_path = "~/receiver.bin" },
  { pipeline_id = "018f0c10-7b4a-7cc0-8000-000000000001", agent_id = "weather", channel_id = "018f0c10-7b4a-7cc0-8000-000000000003", access = "write", signing_seed_path = "~/signing.bin", channel_key_path = "~/channel.bin" },
]
ollama_models = []
"#,
        );
        let authorities = provision_authorities(config.data_plane(), home.path()).unwrap();
        assert_eq!(authorities.channel_keys().len(), 2);
        assert!(authorities.models().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_placeholder_and_broadly_readable_key_files() {
        use std::os::unix::fs::PermissionsExt;

        let home = TestDirectory::new("invalid-keys");
        home.write_secret("key.bin", &[0; KEY_BYTES]);
        let config = config(
            r#"
channel_keys = [
  { pipeline_id = "018f0c10-7b4a-7cc0-8000-000000000001", agent_id = "weather", channel_id = "018f0c10-7b4a-7cc0-8000-000000000002", access = "read", private_key_path = "~/key.bin" },
]
ollama_models = []
"#,
        );
        assert!(matches!(
            provision_authorities(config.data_plane(), home.path()),
            Err(AuthorityProvisioningError::ChannelKey(
                ChannelKeyRegistrationError::InvalidSecret
            ))
        ));

        fs::write(home.path().join("key.bin"), [0x44; KEY_BYTES]).unwrap();
        fs::set_permissions(
            home.path().join("key.bin"),
            fs::Permissions::from_mode(0o640),
        )
        .unwrap();
        assert_eq!(
            provision_authorities(config.data_plane(), home.path())
                .err()
                .unwrap(),
            AuthorityProvisioningError::SecretFile(SecretFileError::InsecurePermissions)
        );
    }
}
