//! Fail-closed local authentication and wiring policy for the D18 Chief daemon.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_biometric_approval::{BiometricApprovalError, BiometricCommandProvider};
use chief_of_staff_daemon_api::{Operation, SessionAuthorizer};
use chief_of_staff_daemon_config::{ConfiguredPrivilegeTier, PrivilegeConfig};
use chief_of_staff_hardware_key_approval::{HardwareKeyApprovalError, HardwareKeyCommandProvider};
use chief_of_staff_notification_approval::{
    NotificationApprovalError, NotificationCommandProvider,
};
use chief_of_staff_orchestrator_core::{
    ChannelPrivilegeResolver, ChannelWiringAuthorizer, ChannelWiringRequest,
    PipelinePrivilegeResolver, PipelineWiringAuthorizer, PipelineWiringRequest,
    TrustCheckingChannelWiring,
};
use chief_of_staff_tool_api::PrivilegeTier;
use chief_of_staff_trust_checker::{
    ApprovalOutcome, ApprovalPrompt, ApprovalProvider, ApprovalRequirement, TrustRequestContext,
};
use coding_adventures_csprng::random_array;
use coding_adventures_ct_compare::ct_eq_fixed;
use coding_adventures_zeroize::Zeroizing;
use core::fmt::{self, Display, Formatter};
use std::collections::BTreeMap;
use std::path::Path;

const SECRET_BYTES: usize = 32;
const ENCODED_BYTES: usize = SECRET_BYTES * 2;
const HEX: &[u8; 16] = b"0123456789abcdef";
const LOCAL_OPERATOR_ID: &str = "operator:local";

/// Stable payload-blind local authentication failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalAuthError {
    /// The OS CSPRNG could not generate a fresh credential.
    RandomnessUnavailable,
    /// A retained credential was not exactly 64 lowercase hexadecimal bytes.
    InvalidCredentialEncoding,
    /// A presented credential did not authenticate.
    AuthenticationFailed,
}

impl Display for LocalAuthError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::RandomnessUnavailable => "chief daemon policy: randomness unavailable",
            Self::InvalidCredentialEncoding => "chief daemon policy: invalid credential encoding",
            Self::AuthenticationFailed => "chief daemon policy: authentication failed",
        })
    }
}

impl std::error::Error for LocalAuthError {}

/// Generate one fresh lowercase-hex 256-bit bearer credential.
///
/// The returned string is wiped on drop. Outer composition is responsible for
/// persisting it with OS-appropriate owner-only permissions and delivering it
/// to an already protected CLI boundary.
pub fn generate_local_credential() -> Result<Zeroizing<String>, LocalAuthError> {
    let secret = Zeroizing::new(
        random_array::<SECRET_BYTES>().map_err(|_| LocalAuthError::RandomnessUnavailable)?,
    );
    Ok(Zeroizing::new(encode_secret(&secret)))
}

fn encode_secret(secret: &[u8; SECRET_BYTES]) -> String {
    let mut encoded = String::with_capacity(ENCODED_BYTES);
    for byte in secret {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

/// Opaque authority attached to one successfully authenticated connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalOperatorSession(());

/// Constant-time bearer policy for one loopback Chief daemon instance.
///
/// The retained credential is wiped on drop and intentionally has no `Debug`,
/// `Display`, or cloning implementation.
pub struct LocalBearerAuthorizer {
    expected: Zeroizing<[u8; ENCODED_BYTES]>,
}

impl LocalBearerAuthorizer {
    /// Retain one canonical lowercase-hex credential for authentication.
    pub fn new(encoded: &str) -> Result<Self, LocalAuthError> {
        if encoded.len() != ENCODED_BYTES
            || !encoded
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(LocalAuthError::InvalidCredentialEncoding);
        }
        let mut expected = [0u8; ENCODED_BYTES];
        expected.copy_from_slice(encoded.as_bytes());
        Ok(Self {
            expected: Zeroizing::new(expected),
        })
    }
}

impl SessionAuthorizer for LocalBearerAuthorizer {
    type Session = LocalOperatorSession;
    type Error = LocalAuthError;

    fn authenticate(&self, credential: &str) -> Result<Self::Session, Self::Error> {
        let candidate: &[u8; ENCODED_BYTES] = credential
            .as_bytes()
            .try_into()
            .map_err(|_| LocalAuthError::AuthenticationFailed)?;
        if ct_eq_fixed(&self.expected, candidate) {
            Ok(LocalOperatorSession(()))
        } else {
            Err(LocalAuthError::AuthenticationFailed)
        }
    }

    fn authorize(
        &self,
        _session: &Self::Session,
        _operation: Operation,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }

    fn requester_id<'a>(&self, _session: &'a Self::Session) -> &'a str {
        LOCAL_OPERATOR_ID
    }
}

/// Stable refusal from the placeholder channel-wiring trust boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChannelWiringDenied;

impl Display for ChannelWiringDenied {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("chief daemon policy: channel wiring denied")
    }
}

impl std::error::Error for ChannelWiringDenied {}

/// Deny every channel topology mutation until Trust Checker approval exists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DenyChannelWiring;

impl ChannelWiringAuthorizer for DenyChannelWiring {
    type Error = ChannelWiringDenied;

    fn authorize(
        &mut self,
        _context: &TrustRequestContext,
        _request: ChannelWiringRequest<'_>,
    ) -> Result<(), Self::Error> {
        Err(ChannelWiringDenied)
    }
}

impl PipelineWiringAuthorizer for DenyChannelWiring {
    type Error = ChannelWiringDenied;

    fn authorize_pipeline(
        &mut self,
        _context: &TrustRequestContext,
        _request: PipelineWiringRequest<'_>,
    ) -> Result<(), Self::Error> {
        Err(ChannelWiringDenied)
    }
}

/// Stable payload-blind failure from exact privilege resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrivilegeResolutionError {
    /// The exact agent identity has no authoritative tier assignment.
    AgentUnassigned,
    /// The exact channel identity has no authoritative tier assignment.
    ChannelUnassigned,
    /// The immutable package hash has no authoritative tier assignment.
    PackageUnassigned,
    /// The selected model has no authoritative tier assignment.
    ModelUnassigned,
}

impl Display for PrivilegeResolutionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::AgentUnassigned => "chief daemon policy: agent tier unassigned",
            Self::ChannelUnassigned => "chief daemon policy: channel tier unassigned",
            Self::PackageUnassigned => "chief daemon policy: package tier unassigned",
            Self::ModelUnassigned => "chief daemon policy: model tier unassigned",
        })
    }
}

impl std::error::Error for PrivilegeResolutionError {}

/// Immutable exact tier authority derived from the validated Chief config.
///
/// Every resource referenced by a mutation must have an explicit assignment;
/// omission is a denial rather than an implicit Tier 0 fallback.
pub struct ExplicitPrivilegeResolver {
    agents: BTreeMap<Vec<u8>, PrivilegeTier>,
    channels: BTreeMap<[u8; 16], PrivilegeTier>,
    packages: BTreeMap<[u8; 32], PrivilegeTier>,
    models: BTreeMap<String, PrivilegeTier>,
}

impl ExplicitPrivilegeResolver {
    /// Build exact immutable authority from a fully validated config section.
    pub fn from_config(config: &PrivilegeConfig) -> Self {
        Self {
            agents: config
                .agent_tiers()
                .iter()
                .map(|assignment| {
                    (
                        assignment.agent_id().to_vec(),
                        configured_tier(assignment.tier()),
                    )
                })
                .collect(),
            channels: config
                .channel_tiers()
                .iter()
                .map(|assignment| (assignment.channel_id(), configured_tier(assignment.tier())))
                .collect(),
            packages: config
                .package_tiers()
                .iter()
                .map(|assignment| {
                    (
                        assignment.package_hash(),
                        configured_tier(assignment.tier()),
                    )
                })
                .collect(),
            models: config
                .model_tiers()
                .iter()
                .map(|assignment| {
                    (
                        assignment.model().to_string(),
                        configured_tier(assignment.tier()),
                    )
                })
                .collect(),
        }
    }

    fn agent_tier_exact(&self, agent_id: &[u8]) -> Result<PrivilegeTier, PrivilegeResolutionError> {
        self.agents
            .get(agent_id)
            .copied()
            .ok_or(PrivilegeResolutionError::AgentUnassigned)
    }

    fn channel_tier_exact(
        &self,
        channel_id: [u8; 16],
    ) -> Result<PrivilegeTier, PrivilegeResolutionError> {
        self.channels
            .get(&channel_id)
            .copied()
            .ok_or(PrivilegeResolutionError::ChannelUnassigned)
    }
}

impl ChannelPrivilegeResolver for ExplicitPrivilegeResolver {
    type Error = PrivilegeResolutionError;

    fn channel_tier(
        &mut self,
        request: ChannelWiringRequest<'_>,
    ) -> Result<PrivilegeTier, Self::Error> {
        self.channel_tier_exact(request.channel_id().0)
    }

    fn agent_tier(
        &mut self,
        agent_id: &chief_of_staff_channel_endpoints::AgentId,
    ) -> Result<PrivilegeTier, Self::Error> {
        self.agent_tier_exact(agent_id.as_bytes())
    }
}

impl PipelinePrivilegeResolver for ExplicitPrivilegeResolver {
    type Error = PrivilegeResolutionError;

    fn pipeline_tier(
        &mut self,
        request: PipelineWiringRequest<'_>,
    ) -> Result<PrivilegeTier, Self::Error> {
        let binding = request.binding();
        let mut tier = self
            .packages
            .get(binding.registration().package_hash())
            .copied()
            .ok_or(PrivilegeResolutionError::PackageUnassigned)?;
        for channel in binding.launch_bindings().channels() {
            tier = tier.max(self.channel_tier_exact(channel.channel_id())?);
        }
        if let Some(model) = binding.launch_bindings().level_one_model() {
            tier = tier.max(
                self.models
                    .get(model.model())
                    .copied()
                    .ok_or(PrivilegeResolutionError::ModelUnassigned)?,
            );
        }
        Ok(tier)
    }

    fn pipeline_agent_tier(
        &mut self,
        agent_id: &chief_of_staff_channel_endpoints::AgentId,
    ) -> Result<PrivilegeTier, Self::Error> {
        self.agent_tier_exact(agent_id.as_bytes())
    }
}

fn configured_tier(tier: ConfiguredPrivilegeTier) -> PrivilegeTier {
    match tier {
        ConfiguredPrivilegeTier::Tier0 => PrivilegeTier::Tier0,
        ConfiguredPrivilegeTier::Tier1 => PrivilegeTier::Tier1,
        ConfiguredPrivilegeTier::Tier2 => PrivilegeTier::Tier2,
        ConfiguredPrivilegeTier::Tier3 => PrivilegeTier::Tier3,
    }
}

/// Stable refusal from production when an interactive provider is required.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApprovalProviderUnavailable;

impl Display for ApprovalProviderUnavailable {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("chief daemon policy: approval provider unavailable")
    }
}

impl std::error::Error for ApprovalProviderUnavailable {}

/// Production provider that deliberately fails every interactive request.
///
/// Trust Checker never calls this provider for Tier 0. Tier 1 through Tier 3
/// therefore remain unavailable rather than being silently downgraded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UnavailableApprovalProvider;

impl ApprovalProvider for UnavailableApprovalProvider {
    type Error = ApprovalProviderUnavailable;

    fn request_approval(
        &mut self,
        _prompt: ApprovalPrompt<'_>,
    ) -> Result<ApprovalOutcome, Self::Error> {
        Err(ApprovalProviderUnavailable)
    }
}

/// Payload-blind production approval-provider failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProductionApprovalError {
    /// No interactive provider was configured.
    Unavailable,
    /// The configured Tier 1 notification helper failed closed.
    Notification(NotificationApprovalError),
    /// The configured Tier 2 biometric helper failed closed.
    Biometric(BiometricApprovalError),
    /// The configured Tier 3 hardware-key helper failed closed.
    HardwareKey(HardwareKeyApprovalError),
}

impl Display for ProductionApprovalError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unavailable => "chief daemon policy: approval provider unavailable",
            Self::Notification(_) => "chief daemon policy: notification approval failed",
            Self::Biometric(_) => "chief daemon policy: biometric approval failed",
            Self::HardwareKey(_) => "chief daemon policy: hardware-key approval failed",
        })
    }
}

impl std::error::Error for ProductionApprovalError {}

/// Production provider selected from the closed daemon configuration.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProductionApprovalProvider {
    notification: Option<NotificationCommandProvider>,
    biometric: Option<BiometricCommandProvider>,
    hardware_key: Option<HardwareKeyCommandProvider>,
}

impl ProductionApprovalProvider {
    /// Compose the independently optional reviewed Tier 1, Tier 2, and Tier 3 helpers.
    pub fn new(
        notification: Option<NotificationCommandProvider>,
        biometric: Option<BiometricCommandProvider>,
        hardware_key: Option<HardwareKeyCommandProvider>,
    ) -> Self {
        Self {
            notification,
            biometric,
            hardware_key,
        }
    }
}

impl ApprovalProvider for ProductionApprovalProvider {
    type Error = ProductionApprovalError;

    fn request_approval(
        &mut self,
        prompt: ApprovalPrompt<'_>,
    ) -> Result<ApprovalOutcome, Self::Error> {
        match prompt.requirement() {
            ApprovalRequirement::Notification { .. } => self
                .notification
                .as_mut()
                .ok_or(ProductionApprovalError::Unavailable)?
                .request_approval(prompt)
                .map_err(ProductionApprovalError::Notification),
            ApprovalRequirement::Biometric { .. } => self
                .biometric
                .as_mut()
                .ok_or(ProductionApprovalError::Unavailable)?
                .request_approval(prompt)
                .map_err(ProductionApprovalError::Biometric),
            ApprovalRequirement::HardwareKey { .. } => self
                .hardware_key
                .as_mut()
                .ok_or(ProductionApprovalError::Unavailable)?
                .request_approval(prompt)
                .map_err(ProductionApprovalError::HardwareKey),
            ApprovalRequirement::None => Err(ProductionApprovalError::Unavailable),
        }
    }
}

/// Stable production-composition failure before the daemon starts serving.
#[derive(Debug)]
pub enum ProductionPolicyError {
    /// The configured helper path could not be resolved against the explicit home.
    Config(chief_of_staff_daemon_config::ConfigError),
    /// The resolved helper path was not acceptable to the notification provider.
    Notification(NotificationApprovalError),
    /// The resolved helper path was not acceptable to the biometric provider.
    Biometric(BiometricApprovalError),
    /// The resolved helper path was not acceptable to the hardware-key provider.
    HardwareKey(HardwareKeyApprovalError),
}

impl Display for ProductionPolicyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Config(_) => "chief daemon policy: approval path resolution failed",
            Self::Notification(_) => "chief daemon policy: approval provider configuration failed",
            Self::Biometric(_) => "chief daemon policy: approval provider configuration failed",
            Self::HardwareKey(_) => "chief daemon policy: approval provider configuration failed",
        })
    }
}

impl std::error::Error for ProductionPolicyError {}

/// Current production Trust Checker composition.
pub type ProductionWiringAuthorizer =
    TrustCheckingChannelWiring<ProductionApprovalProvider, ExplicitPrivilegeResolver>;

/// Compose exact configured tiers with optional reviewed Tier 1, Tier 2, and Tier 3 helpers.
pub fn production_wiring_authorizer(
    config: &PrivilegeConfig,
    home: &Path,
) -> Result<ProductionWiringAuthorizer, ProductionPolicyError> {
    let notification = match config.tier_1_notification_command() {
        None => None,
        Some(path) => {
            let executable = path.resolve(home).map_err(ProductionPolicyError::Config)?;
            Some(
                NotificationCommandProvider::new(executable)
                    .map_err(ProductionPolicyError::Notification)?,
            )
        }
    };
    let biometric = match config.tier_2_biometric_command() {
        None => None,
        Some(path) => {
            let executable = path.resolve(home).map_err(ProductionPolicyError::Config)?;
            Some(
                BiometricCommandProvider::new(executable)
                    .map_err(ProductionPolicyError::Biometric)?,
            )
        }
    };
    let hardware_key = match config.tier_3_hardware_key_command() {
        None => None,
        Some(path) => {
            let executable = path.resolve(home).map_err(ProductionPolicyError::Config)?;
            Some(
                HardwareKeyCommandProvider::new(executable)
                    .map_err(ProductionPolicyError::HardwareKey)?,
            )
        }
    };
    let provider = ProductionApprovalProvider::new(notification, biometric, hardware_key);
    Ok(TrustCheckingChannelWiring::new(
        provider,
        ExplicitPrivilegeResolver::from_config(config),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chief_of_staff_channel_crypto::{ChannelId, KeyEpoch};
    use chief_of_staff_channel_endpoints::{
        AgentId, ChannelDefinition, OriginatorIdentity, ReceiverIdentity,
    };
    use chief_of_staff_daemon_config::parse_config;
    use chief_of_staff_host_control_protocol::{
        ChannelBinding, ChannelBindingAccess, LaunchBindings, LevelOneModelBinding,
    };
    use chief_of_staff_pipeline_bindings::{HostPipelineBinding, PipelineId};
    use chief_of_staff_service_registry::{HostName, HostRegistration, PackagePath, RestartPolicy};
    use std::path::Path;

    const CREDENTIAL: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

    #[test]
    fn deterministic_encoder_produces_canonical_lowercase_hex() {
        let mut secret = [0u8; SECRET_BYTES];
        for (index, byte) in secret.iter_mut().enumerate() {
            *byte = u8::try_from(index).unwrap();
        }
        assert_eq!(encode_secret(&secret), CREDENTIAL);
    }

    #[test]
    fn exact_credential_authenticates_and_authorizes_every_current_operation() {
        let policy = LocalBearerAuthorizer::new(CREDENTIAL).unwrap();
        let session = policy.authenticate(CREDENTIAL).unwrap();
        for operation in [
            Operation::RegisterHost,
            Operation::ReloadHost,
            Operation::ListHosts,
            Operation::SetDesiredState,
            Operation::ReconcileOnce,
            Operation::HealthCheck,
            Operation::DeregisterHost,
            Operation::WireHostPipeline,
            Operation::UnwireHostPipeline,
        ] {
            assert!(policy.authorize(&session, operation).unwrap());
        }
        assert_eq!(policy.requester_id(&session), LOCAL_OPERATOR_ID);
    }

    #[test]
    fn equal_length_mismatches_and_invalid_lengths_fail_without_authority() {
        let policy = LocalBearerAuthorizer::new(CREDENTIAL).unwrap();
        for candidate in [
            format!("f{}", &CREDENTIAL[1..]),
            format!("{}z{}", &CREDENTIAL[..31], &CREDENTIAL[32..]),
            format!("{}0", &CREDENTIAL[..63]),
            "short".to_string(),
            format!("{CREDENTIAL}0"),
        ] {
            assert_eq!(
                policy.authenticate(&candidate),
                Err(LocalAuthError::AuthenticationFailed)
            );
        }
    }

    #[test]
    fn retained_credentials_require_one_canonical_encoding() {
        for invalid in [
            "",
            "abc",
            "000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F",
            "g00102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
        ] {
            assert!(matches!(
                LocalBearerAuthorizer::new(invalid),
                Err(LocalAuthError::InvalidCredentialEncoding)
            ));
        }
    }

    #[test]
    fn generated_credentials_are_canonical_fresh_and_authenticatable() {
        let first = generate_local_credential().unwrap();
        let second = generate_local_credential().unwrap();
        assert_eq!(first.len(), ENCODED_BYTES);
        assert!(first
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
        assert_ne!(&*first, &*second);
        assert!(LocalBearerAuthorizer::new(&first)
            .unwrap()
            .authenticate(&first)
            .is_ok());
    }

    #[test]
    fn channel_topology_is_denied_until_a_trust_checker_approves_it() {
        let definition = channel_definition();
        let context = TrustRequestContext::new("request", "operator:local").unwrap();
        let mut policy = DenyChannelWiring;
        assert_eq!(
            policy.authorize(&context, ChannelWiringRequest::Create(&definition)),
            Err(ChannelWiringDenied)
        );
        assert_eq!(
            policy.authorize(&context, ChannelWiringRequest::Destroy(&definition)),
            Err(ChannelWiringDenied)
        );
        fn assert_pipeline_policy<T: PipelineWiringAuthorizer<Error = ChannelWiringDenied>>() {}
        assert_pipeline_policy::<DenyChannelWiring>();
    }

    #[test]
    fn production_authorizer_allows_only_fully_assigned_tier_zero_pipeline() {
        let config = policy_config(0, true);
        let mut authorizer = production_wiring_authorizer(config.privilege(), test_home()).unwrap();
        let binding = pipeline_binding();
        let context = TrustRequestContext::new("request", "operator:local").unwrap();
        assert!(authorizer
            .authorize_pipeline(&context, PipelineWiringRequest::Wire(&binding))
            .is_ok());

        let config = policy_config(1, true);
        let mut authorizer = production_wiring_authorizer(config.privilege(), test_home()).unwrap();
        assert!(matches!(
            authorizer.authorize_pipeline(&context, PipelineWiringRequest::Wire(&binding)),
            Err(chief_of_staff_orchestrator_core::TrustPipelineWiringError::Approval(_))
        ));

        let config = policy_config(0, false);
        let mut authorizer = production_wiring_authorizer(config.privilege(), test_home()).unwrap();
        assert!(matches!(
            authorizer.authorize_pipeline(&context, PipelineWiringRequest::Wire(&binding)),
            Err(
                chief_of_staff_orchestrator_core::TrustPipelineWiringError::Resolver(
                    PrivilegeResolutionError::ModelUnassigned
                )
            )
        ));
    }

    #[test]
    fn production_channel_authority_denies_every_implicit_tier() {
        let config = policy_config(0, true);
        let context = TrustRequestContext::new("request", "operator:local").unwrap();
        let definition = channel_definition();
        let mut authorizer = production_wiring_authorizer(config.privilege(), test_home()).unwrap();
        assert!(authorizer
            .authorize(&context, ChannelWiringRequest::Create(&definition))
            .is_ok());

        let empty = parse_config(&base_config("")).unwrap();
        let mut authorizer = production_wiring_authorizer(empty.privilege(), test_home()).unwrap();
        assert!(matches!(
            authorizer.authorize(&context, ChannelWiringRequest::Create(&definition)),
            Err(
                chief_of_staff_orchestrator_core::TrustChannelWiringError::Resolver(
                    PrivilegeResolutionError::ChannelUnassigned
                )
            )
        ));
    }

    #[test]
    fn errors_are_stable_and_payload_blind() {
        assert_eq!(
            LocalAuthError::RandomnessUnavailable.to_string(),
            "chief daemon policy: randomness unavailable"
        );
        assert_eq!(
            LocalAuthError::InvalidCredentialEncoding.to_string(),
            "chief daemon policy: invalid credential encoding"
        );
        assert_eq!(
            LocalAuthError::AuthenticationFailed.to_string(),
            "chief daemon policy: authentication failed"
        );
        assert_eq!(
            ChannelWiringDenied.to_string(),
            "chief daemon policy: channel wiring denied"
        );
        assert_eq!(
            ProductionApprovalError::Notification(NotificationApprovalError::SpawnFailed)
                .to_string(),
            "chief daemon policy: notification approval failed"
        );
        assert_eq!(
            ProductionApprovalError::Biometric(BiometricApprovalError::SpawnFailed).to_string(),
            "chief daemon policy: biometric approval failed"
        );
        assert_eq!(
            ProductionApprovalError::HardwareKey(HardwareKeyApprovalError::SpawnFailed).to_string(),
            "chief daemon policy: hardware-key approval failed"
        );
    }

    #[test]
    fn configured_tier_one_helper_is_selected_without_opening_higher_tiers() {
        let config = policy_config_with_notification(1);
        let context = TrustRequestContext::new("request", "operator:local").unwrap();
        let binding = pipeline_binding();
        let mut authorizer = production_wiring_authorizer(config.privilege(), test_home()).unwrap();
        assert!(matches!(
            authorizer.authorize_pipeline(&context, PipelineWiringRequest::Wire(&binding)),
            Err(
                chief_of_staff_orchestrator_core::TrustPipelineWiringError::Approval(
                    chief_of_staff_trust_checker::TrustCheckerError::Provider(
                        ProductionApprovalError::Notification(
                            NotificationApprovalError::SpawnFailed
                        )
                    )
                )
            )
        ));

        let tier_two = policy_config_with_notification(2);
        let mut authorizer =
            production_wiring_authorizer(tier_two.privilege(), test_home()).unwrap();
        assert!(matches!(
            authorizer.authorize_pipeline(&context, PipelineWiringRequest::Wire(&binding)),
            Err(
                chief_of_staff_orchestrator_core::TrustPipelineWiringError::Approval(
                    chief_of_staff_trust_checker::TrustCheckerError::Provider(
                        ProductionApprovalError::Unavailable
                    )
                )
            )
        ));
    }

    #[test]
    fn configured_tier_two_helper_is_selected_without_opening_other_tiers() {
        let config = policy_config_with_biometric(2);
        let context = TrustRequestContext::new("request", "operator:local").unwrap();
        let binding = pipeline_binding();
        let mut authorizer = production_wiring_authorizer(config.privilege(), test_home()).unwrap();
        assert!(matches!(
            authorizer.authorize_pipeline(&context, PipelineWiringRequest::Wire(&binding)),
            Err(
                chief_of_staff_orchestrator_core::TrustPipelineWiringError::Approval(
                    chief_of_staff_trust_checker::TrustCheckerError::Provider(
                        ProductionApprovalError::Biometric(BiometricApprovalError::SpawnFailed)
                    )
                )
            )
        ));

        let tier_one = policy_config_with_biometric(1);
        let mut authorizer =
            production_wiring_authorizer(tier_one.privilege(), test_home()).unwrap();
        assert!(matches!(
            authorizer.authorize_pipeline(&context, PipelineWiringRequest::Wire(&binding)),
            Err(
                chief_of_staff_orchestrator_core::TrustPipelineWiringError::Approval(
                    chief_of_staff_trust_checker::TrustCheckerError::Provider(
                        ProductionApprovalError::Unavailable
                    )
                )
            )
        ));

        let tier_three = policy_config_with_biometric(3);
        let mut authorizer =
            production_wiring_authorizer(tier_three.privilege(), test_home()).unwrap();
        assert!(matches!(
            authorizer.authorize_pipeline(&context, PipelineWiringRequest::Wire(&binding)),
            Err(
                chief_of_staff_orchestrator_core::TrustPipelineWiringError::Approval(
                    chief_of_staff_trust_checker::TrustCheckerError::Provider(
                        ProductionApprovalError::Unavailable
                    )
                )
            )
        ));
    }

    #[test]
    fn configured_tier_three_helper_is_selected_without_opening_lower_tiers() {
        let config = policy_config_with_hardware_key(3);
        let context = TrustRequestContext::new("request", "operator:local").unwrap();
        let binding = pipeline_binding();
        let mut authorizer = production_wiring_authorizer(config.privilege(), test_home()).unwrap();
        assert!(matches!(
            authorizer.authorize_pipeline(&context, PipelineWiringRequest::Wire(&binding)),
            Err(
                chief_of_staff_orchestrator_core::TrustPipelineWiringError::Approval(
                    chief_of_staff_trust_checker::TrustCheckerError::Provider(
                        ProductionApprovalError::HardwareKey(HardwareKeyApprovalError::SpawnFailed)
                    )
                )
            )
        ));

        for tier in [1, 2] {
            let lower = policy_config_with_hardware_key(tier);
            let mut authorizer =
                production_wiring_authorizer(lower.privilege(), test_home()).unwrap();
            assert!(matches!(
                authorizer.authorize_pipeline(&context, PipelineWiringRequest::Wire(&binding)),
                Err(
                    chief_of_staff_orchestrator_core::TrustPipelineWiringError::Approval(
                        chief_of_staff_trust_checker::TrustCheckerError::Provider(
                            ProductionApprovalError::Unavailable
                        )
                    )
                )
            ));
        }
    }

    fn channel_definition() -> ChannelDefinition {
        let mut channel_id = [0u8; 16];
        channel_id[6] = 0x70;
        channel_id[8] = 0x80;
        ChannelDefinition::new(
            ChannelId(channel_id),
            OriginatorIdentity {
                agent_id: AgentId::new(b"originator".to_vec()).unwrap(),
                public_key: [1; 32],
            },
            vec![ReceiverIdentity {
                agent_id: AgentId::new(b"receiver".to_vec()).unwrap(),
                public_key: [2; 32],
            }],
            1,
            KeyEpoch(1),
        )
        .unwrap()
    }

    fn pipeline_binding() -> HostPipelineBinding {
        let channel_id = channel_definition().channel_id().0;
        HostPipelineBinding::new(
            PipelineId::new(uuid_v7(9)).unwrap(),
            HostRegistration::new(
                HostName::new("weather-host").unwrap(),
                PackagePath::new("agents/weather").unwrap(),
                [0xab; 32],
                RestartPolicy::Never,
            ),
            AgentId::new(b"weather".to_vec()).unwrap(),
            LaunchBindings::new(
                vec![
                    ChannelBinding::new("weather-input", ChannelBindingAccess::Read, channel_id)
                        .unwrap(),
                ],
                Some(LevelOneModelBinding::new("qwen2.5:0.5b", 0.0, 128).unwrap()),
            )
            .unwrap(),
        )
    }

    fn uuid_v7(tag: u8) -> [u8; 16] {
        let mut bytes = [tag; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes
    }

    fn policy_config(
        package_tier: u8,
        include_model: bool,
    ) -> chief_of_staff_daemon_config::ChiefConfig {
        let model = if include_model {
            "model_tiers = [{ model = \"qwen2.5:0.5b\", tier = 0 }]"
        } else {
            ""
        };
        parse_config(&base_config(&format!(
            r#"agent_tiers = [
  {{ agent_id = "77656174686572", tier = 0 }},
  {{ agent_id = "6f726967696e61746f72", tier = 0 }},
  {{ agent_id = "7265636569766572", tier = 0 }},
]
channel_tiers = [{{ channel_id = "00000000-0000-7000-8000-000000000000", tier = 0 }}]
package_tiers = [{{ package_hash = "{}", tier = {package_tier} }}]
{model}"#,
            "ab".repeat(32)
        )))
        .unwrap()
    }

    fn policy_config_with_notification(
        package_tier: u8,
    ) -> chief_of_staff_daemon_config::ChiefConfig {
        let source = base_config(&format!(
            r#"agent_tiers = [{{ agent_id = "77656174686572", tier = 0 }}]
channel_tiers = [{{ channel_id = "00000000-0000-7000-8000-000000000000", tier = 0 }}]
package_tiers = [{{ package_hash = "{}", tier = {package_tier} }}]
model_tiers = [{{ model = "qwen2.5:0.5b", tier = 0 }}]"#,
            "ab".repeat(32)
        ))
        .replace(
            "tier_1_auto_approve_timeout = 5",
            "tier_1_auto_approve_timeout = 5\ntier_1_notification_command = \"~/missing-notification-helper\"",
        );
        parse_config(&source).unwrap()
    }

    fn policy_config_with_biometric(package_tier: u8) -> chief_of_staff_daemon_config::ChiefConfig {
        let source = base_config(&format!(
            r#"agent_tiers = [{{ agent_id = "77656174686572", tier = 0 }}]
channel_tiers = [{{ channel_id = "00000000-0000-7000-8000-000000000000", tier = 0 }}]
package_tiers = [{{ package_hash = "{}", tier = {package_tier} }}]
model_tiers = [{{ model = "qwen2.5:0.5b", tier = 0 }}]"#,
            "ab".repeat(32)
        ))
        .replace(
            "biometric_timeout = 30",
            "biometric_timeout = 30\ntier_2_biometric_command = \"~/missing-biometric-helper\"",
        );
        parse_config(&source).unwrap()
    }

    fn policy_config_with_hardware_key(
        package_tier: u8,
    ) -> chief_of_staff_daemon_config::ChiefConfig {
        let source = base_config(&format!(
            r#"agent_tiers = [{{ agent_id = "77656174686572", tier = 0 }}]
channel_tiers = [{{ channel_id = "00000000-0000-7000-8000-000000000000", tier = 0 }}]
package_tiers = [{{ package_hash = "{}", tier = {package_tier} }}]
model_tiers = [{{ model = "qwen2.5:0.5b", tier = 0 }}]"#,
            "ab".repeat(32)
        ))
        .replace(
            "hardware_key_timeout = 60",
            "hardware_key_timeout = 60\ntier_3_hardware_key_command = \"~/missing-hardware-key-helper\"",
        );
        parse_config(&source).unwrap()
    }

    fn test_home() -> &'static Path {
        #[cfg(unix)]
        {
            Path::new("/home/operator")
        }
        #[cfg(windows)]
        {
            Path::new(r"C:\Users\operator")
        }
    }

    fn base_config(privilege_assignments: &str) -> String {
        format!(
            r#"[orchestrator]
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
{privilege_assignments}
"#
        )
    }
}
