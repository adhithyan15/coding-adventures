//! Explicit production authority provisioning for the D18 Chief daemon.
//!
//! What the daemon provisions for its data plane is the model providers:
//! one exact Ollama client per configured model, validated at startup, with
//! no implicit endpoint and no network probe.
//!
//! It provisions no channel key. Since D18S P2.6d-2c, `[data_plane]
//! channel_keys` belongs to the per-agent brokers: the broker launcher opens
//! one agent's key files when it launches that agent's broker, and the
//! daemon process never reads them.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_daemon_config::DataPlaneConfig;
use chief_of_staff_host_data_plane::{ExactModelProviderRegistry, ModelProviderError};
use core::fmt::{self, Display, Formatter};
use llm_provider_ollama::{OllamaClient, OllamaConfigurationError};
use std::sync::Arc;

/// Stable payload-blind failure while provisioning production data-plane authorities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorityProvisioningError {
    /// One explicit Ollama client declaration was invalid.
    Ollama(OllamaConfigurationError),
    /// One exact model selector was empty or registered more than once.
    Model(ModelProviderError),
}

impl Display for AuthorityProvisioningError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Ollama(_) => "chief authority provisioning: Ollama configuration failed",
            Self::Model(_) => "chief authority provisioning: model registration failed",
        })
    }
}

impl std::error::Error for AuthorityProvisioningError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Ollama(error) => Some(error),
            Self::Model(error) => Some(error),
        }
    }
}

/// The exact-model authority, ready for daemon composition.
pub struct ProvisionedAuthorities {
    models: ExactModelProviderRegistry,
}

impl ProvisionedAuthorities {
    /// Borrow the exact model-provider authority.
    pub fn models(&self) -> &ExactModelProviderRegistry {
        &self.models
    }

    /// Transfer the immutable model authority to a production composition root.
    pub fn into_models(self) -> ExactModelProviderRegistry {
        self.models
    }
}

/// Load every explicitly configured model provider without environment or
/// network access.
pub fn provision_authorities(
    config: &DataPlaneConfig,
) -> Result<ProvisionedAuthorities, AuthorityProvisioningError> {
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
    Ok(ProvisionedAuthorities { models })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chief_of_staff_daemon_config::{parse_config, ChiefConfig};

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
        let valid = config(
            r#"
channel_keys = []
ollama_models = [
  { model = "qwen2.5:0.5b", endpoint = "http://127.0.0.1:11434", timeout = 120000 },
]
"#,
        );
        let authorities = provision_authorities(valid.data_plane()).unwrap();
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
            provision_authorities(invalid.data_plane()).err().unwrap(),
            AuthorityProvisioningError::Ollama(OllamaConfigurationError::InvalidEndpoint)
        );
    }

    #[test]
    fn channel_keys_are_never_read_by_the_daemon() {
        // A key file that does not exist: the daemon's provisioning does not
        // look for it, because channel keys are the brokers' (P2.6d-2c).
        let config = config(
            r#"
channel_keys = [
  { pipeline_id = "018f0c10-7b4a-7cc0-8000-000000000001", agent_id = "weather", channel_id = "018f0c10-7b4a-7cc0-8000-000000000002", access = "read", private_key_path = "~/missing.bin" },
]
ollama_models = []
"#,
        );
        let authorities = provision_authorities(config.data_plane()).unwrap();
        assert!(authorities.models().is_empty());
    }
}
