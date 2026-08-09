//! Provider-neutral runtime for D18 Level 1 `SKILL.md` agents.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::{BTreeMap, HashMap};
use std::fmt::{self, Display, Formatter};

use chief_of_staff_channel_crypto::Sequence;
use chief_of_staff_channel_endpoints::{
    ChannelEndpointError, MessageId, Originator, PublishedMessage, Receiver,
};
use chief_of_staff_host_control_protocol::{
    ChannelBindingAccess, LaunchBindings, LevelOneModelBinding,
};
use chief_of_staff_host_runtime::VerifiedAgentPackage;
use chief_of_staff_skill_package::{load_verified_skill, SkillPackageError};
use chief_of_staff_skill_parser::ParsedSkill;
use llm_gateway::{
    CompletionRequest, CompletionResponse, FinishReason, LlmClient, LlmError, Message,
    ProviderIdentity, TokenUsage,
};

/// MIME type used for Level 1 text responses.
pub const LEVEL_ONE_RESPONSE_CONTENT_TYPE: &str = "text/plain; charset=utf-8";

/// Bounded model settings for a Level 1 runtime.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelOneRuntimeConfig {
    /// Provider-specific model name passed through the LLM gateway.
    pub model: String,
    /// Sampling temperature in the inclusive range zero through two.
    pub temperature: f32,
    /// Maximum number of output tokens requested from the provider.
    pub max_tokens: usize,
}

impl LevelOneRuntimeConfig {
    /// Construct deterministic, conservative settings for one model.
    pub fn deterministic(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            temperature: 0.0,
            max_tokens: 1_024,
        }
    }

    /// Return the provider-specific model selector.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Return the finite sampling temperature.
    pub fn temperature(&self) -> f32 {
        self.temperature
    }

    /// Return the non-zero output-token cap.
    pub fn max_tokens(&self) -> usize {
        self.max_tokens
    }

    fn validate(&self) -> Result<(), LevelOneRuntimeError> {
        if self.model.trim().is_empty() || self.model.chars().count() > 200 {
            return Err(LevelOneRuntimeError::InvalidConfig(
                "model must contain between 1 and 200 characters",
            ));
        }
        if !self.temperature.is_finite() || !(0.0..=2.0).contains(&self.temperature) {
            return Err(LevelOneRuntimeError::InvalidConfig(
                "temperature must be finite and between 0 and 2",
            ));
        }
        if self.max_tokens == 0 {
            return Err(LevelOneRuntimeError::InvalidConfig(
                "max_tokens must be greater than zero",
            ));
        }
        Ok(())
    }
}

/// One completed model turn before or after channel publication.
#[derive(Clone, Debug)]
pub struct LevelOneResponse {
    /// Text emitted by the configured model.
    pub text: String,
    /// Provider-reported model name.
    pub model: String,
    /// Provider identity used for audit correlation.
    pub provider: ProviderIdentity,
    /// Provider-reported token accounting.
    pub usage: TokenUsage,
    /// Provider-reported reason generation stopped.
    pub finish_reason: FinishReason,
    /// Provider-reported request latency in milliseconds.
    pub latency_ms: u64,
}

/// Result of polling one input message.
#[derive(Clone, Debug)]
pub enum LevelOneRunOutcome {
    /// No channel message was waiting.
    Idle,
    /// One message was completed, published, and acknowledged.
    Processed {
        /// Input message that was acknowledged.
        input_message_id: MessageId,
        /// Sequence returned by the monotonic acknowledgement.
        acknowledged_through: Sequence,
        /// Receipt for the published model output.
        output: PublishedMessage,
        /// Model result published to the output channel.
        response: Box<LevelOneResponse>,
    },
}

/// Failures from configuration, message decoding, LLM completion, or channels.
#[derive(Debug)]
pub enum LevelOneRuntimeError {
    /// Static runtime settings are outside the bounded Level 1 contract.
    InvalidConfig(&'static str),
    /// Authorized launch bindings do not exactly match the signed Level 1 manifest.
    InvalidLaunchBindings,
    /// A verified channel payload was not UTF-8 text.
    NonUtf8Input,
    /// The provider returned an empty or whitespace-only response.
    EmptyResponse,
    /// The provider-neutral LLM gateway rejected the completion.
    Llm(Box<LlmError>),
    /// An injected channel endpoint rejected receive, publish, or acknowledge.
    Channel(ChannelEndpointError),
    /// A verified package did not contain matching Level 1 instructions and policy.
    Package(Box<SkillPackageError>),
}

impl Display for LevelOneRuntimeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(message) => write!(formatter, "invalid Level 1 runtime: {message}"),
            Self::InvalidLaunchBindings => {
                formatter.write_str("Level 1 launch bindings do not match signed policy")
            }
            Self::NonUtf8Input => formatter.write_str("Level 1 input payload is not UTF-8 text"),
            Self::EmptyResponse => formatter.write_str("Level 1 model response is empty"),
            Self::Llm(error) => write!(formatter, "Level 1 model call failed: {error}"),
            Self::Channel(error) => write!(formatter, "Level 1 channel operation failed: {error}"),
            Self::Package(error) => write!(formatter, "Level 1 package rejected: {error}"),
        }
    }
}

impl std::error::Error for LevelOneRuntimeError {}

impl From<LlmError> for LevelOneRuntimeError {
    fn from(error: LlmError) -> Self {
        Self::Llm(Box::new(error))
    }
}

impl From<ChannelEndpointError> for LevelOneRuntimeError {
    fn from(error: ChannelEndpointError) -> Self {
        Self::Channel(error)
    }
}

impl From<SkillPackageError> for LevelOneRuntimeError {
    fn from(error: SkillPackageError) -> Self {
        Self::Package(Box::new(error))
    }
}

/// Independently verified Level 1 policy bound to pipeline-authorized runtime inputs.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelOneLaunchPlan {
    skill: ParsedSkill,
    config: LevelOneRuntimeConfig,
    read_channels: BTreeMap<String, [u8; 16]>,
    write_channels: BTreeMap<String, [u8; 16]>,
}

impl LevelOneLaunchPlan {
    /// Require exact channel names, directions, and model settings for a signed package.
    pub fn from_verified_package(
        package: &VerifiedAgentPackage,
        bindings: &LaunchBindings,
    ) -> Result<Self, LevelOneRuntimeError> {
        let skill = load_verified_skill(package)?;
        let model = bindings
            .level_one_model()
            .ok_or(LevelOneRuntimeError::InvalidLaunchBindings)?;
        let config = runtime_config(model)?;
        let mut read_channels = BTreeMap::new();
        let mut write_channels = BTreeMap::new();
        for binding in bindings.channels() {
            let target = match binding.access() {
                ChannelBindingAccess::Read => &mut read_channels,
                ChannelBindingAccess::Write => &mut write_channels,
            };
            if target
                .insert(binding.name().to_string(), binding.channel_id())
                .is_some()
            {
                return Err(LevelOneRuntimeError::InvalidLaunchBindings);
            }
        }
        if read_channels.keys().map(String::as_str).collect::<Vec<_>>()
            != skill
                .manifest
                .channels
                .reads
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
            || write_channels
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>()
                != skill
                    .manifest
                    .channels
                    .writes
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
        {
            return Err(LevelOneRuntimeError::InvalidLaunchBindings);
        }
        Ok(Self {
            skill,
            config,
            read_channels,
            write_channels,
        })
    }

    /// Borrow the authenticated parsed skill.
    pub fn skill(&self) -> &ParsedSkill {
        &self.skill
    }

    /// Borrow the bounded model settings delivered for this launch.
    pub fn config(&self) -> &LevelOneRuntimeConfig {
        &self.config
    }

    /// Resolve one signed read-channel name to its authorized UUID.
    pub fn read_channel_id(&self, name: &str) -> Option<[u8; 16]> {
        self.read_channels.get(name).copied()
    }

    /// Resolve one signed write-channel name to its authorized UUID.
    pub fn write_channel_id(&self, name: &str) -> Option<[u8; 16]> {
        self.write_channels.get(name).copied()
    }

    /// Bind this verified plan to one provider-neutral model client.
    pub fn runtime<'a>(
        &self,
        client: &'a dyn LlmClient,
    ) -> Result<LevelOneSkillRuntime<'a>, LevelOneRuntimeError> {
        LevelOneSkillRuntime::new(self.skill.clone(), client, self.config.clone())
    }
}

fn runtime_config(
    model: &LevelOneModelBinding,
) -> Result<LevelOneRuntimeConfig, LevelOneRuntimeError> {
    let max_tokens = usize::try_from(model.max_tokens())
        .map_err(|_| LevelOneRuntimeError::InvalidLaunchBindings)?;
    let config = LevelOneRuntimeConfig {
        model: model.model().to_string(),
        temperature: model.temperature(),
        max_tokens,
    };
    config.validate()?;
    Ok(config)
}

/// Parsed Level 1 skill bound to one injected LLM provider.
pub struct LevelOneSkillRuntime<'a> {
    skill: ParsedSkill,
    client: &'a dyn LlmClient,
    config: LevelOneRuntimeConfig,
}

impl<'a> LevelOneSkillRuntime<'a> {
    /// Validate and bind a parsed skill, model client, and runtime settings.
    pub fn new(
        skill: ParsedSkill,
        client: &'a dyn LlmClient,
        config: LevelOneRuntimeConfig,
    ) -> Result<Self, LevelOneRuntimeError> {
        config.validate()?;
        Ok(Self {
            skill,
            client,
            config,
        })
    }

    /// Bind the exact instructions retained by sealed-package verification.
    pub fn from_verified_package(
        package: &VerifiedAgentPackage,
        client: &'a dyn LlmClient,
        config: LevelOneRuntimeConfig,
    ) -> Result<Self, LevelOneRuntimeError> {
        Self::new(load_verified_skill(package)?, client, config)
    }

    /// Borrow the parsed skill that supplies instructions and manifest identity.
    pub fn skill(&self) -> &ParsedSkill {
        &self.skill
    }

    /// Complete one text turn without performing channel operations.
    pub fn respond(
        &self,
        message: &str,
        input_content_type: &str,
    ) -> Result<LevelOneResponse, LevelOneRuntimeError> {
        let mut metadata = HashMap::new();
        metadata.insert("agent".to_string(), self.skill.manifest.agent.clone());
        metadata.insert("skill_title".to_string(), self.skill.title.clone());
        metadata.insert(
            "input_content_type".to_string(),
            input_content_type.to_string(),
        );
        let completion = self.client.complete(CompletionRequest {
            model: self.config.model.clone(),
            system: Some(self.skill.instructions.clone()),
            messages: vec![Message::user(message)],
            temperature: self.config.temperature,
            max_tokens: Some(self.config.max_tokens),
            stop_sequences: Vec::new(),
            seed: Some(0),
            metadata,
        })?;
        response_from_completion(completion)
    }

    /// Poll and process at most one channel message.
    ///
    /// Ordering is deliberate: receive, complete, publish, acknowledge. An LLM
    /// or publication failure therefore leaves the input cursor unchanged for
    /// crash recovery and retry.
    pub fn run_once(
        &self,
        receiver: &mut dyn Receiver,
        originator: &dyn Originator,
    ) -> Result<LevelOneRunOutcome, LevelOneRuntimeError> {
        let Some(input) = receiver.receive(1)?.into_iter().next() else {
            return Ok(LevelOneRunOutcome::Idle);
        };
        let message =
            std::str::from_utf8(&input.payload).map_err(|_| LevelOneRuntimeError::NonUtf8Input)?;
        let response = self.respond(message, &input.content_type)?;
        let output =
            originator.publish(response.text.as_bytes(), LEVEL_ONE_RESPONSE_CONTENT_TYPE)?;
        let acknowledged_through = receiver.acknowledge(input.message_id)?;
        Ok(LevelOneRunOutcome::Processed {
            input_message_id: input.message_id,
            acknowledged_through,
            output,
            response: Box::new(response),
        })
    }
}

fn response_from_completion(
    completion: CompletionResponse,
) -> Result<LevelOneResponse, LevelOneRuntimeError> {
    if completion.text.trim().is_empty() {
        return Err(LevelOneRuntimeError::EmptyResponse);
    }
    Ok(LevelOneResponse {
        text: completion.text,
        model: completion.model,
        provider: completion.provider_id,
        usage: completion.usage,
        finish_reason: completion.finish_reason,
        latency_ms: completion.latency_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chief_of_staff_channel_crypto::{ChannelId, Sequence};
    use chief_of_staff_channel_endpoints::{AgentId, ReceivedMessage};
    use chief_of_staff_host_control_protocol::ChannelBinding;
    use chief_of_staff_host_runtime::{
        verify_agent_package, PackageKeyType, PackageKeyring, TrustedPackageKey,
    };
    use chief_of_staff_skill_package::build_signed_skill_package;
    use chief_of_staff_skill_parser::parse_skill;
    use chief_of_staff_tool_api::PrivilegeTier;
    use coding_adventures_ed25519::generate_keypair;
    use llm_gateway::{MockLlmClient, MockResponse, RequestFingerprint};
    use std::cell::{Cell, RefCell};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    const SKILL: &str = "# Weather Reporter\n\nYou are a friendly weather reporting agent.\n\n## Capabilities needed\n- none\n\n## Output format\nKeep the forecast brief.\n";
    const WIRED_SKILL: &str = "---\nagent: weather-reporter\ndescription: Reports friendly forecasts for requested cities.\nprivilege_tier: 0\nreads: [weather-requests]\nwrites: [weather-reports]\nmessage_schema_versions: [weather-requests=1, weather-reports=1]\n---\n# Weather Reporter\n\nYou are a friendly weather reporting agent.\n\n## Capabilities needed\n- none\n";
    const MODEL: &str = "test-model";

    fn message_id(byte: u8) -> MessageId {
        let mut bytes = [byte; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        MessageId::from_uuid_v7(bytes).unwrap()
    }

    fn uuid_v7(byte: u8) -> [u8; 16] {
        let mut bytes = [0; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = byte;
        bytes
    }

    fn wired_bindings() -> LaunchBindings {
        LaunchBindings::new(
            vec![
                ChannelBinding::new("weather-requests", ChannelBindingAccess::Read, uuid_v7(1))
                    .unwrap(),
                ChannelBinding::new("weather-reports", ChannelBindingAccess::Write, uuid_v7(2))
                    .unwrap(),
            ],
            Some(LevelOneModelBinding::new(MODEL, 0.25, 256).unwrap()),
        )
        .unwrap()
    }

    fn response_provider() -> ProviderIdentity {
        ProviderIdentity {
            vendor: "test".to_string(),
            model_family: "fixture".to_string(),
            model_version: "v1".to_string(),
            endpoint: None,
        }
    }

    fn runtime_with_response(response: MockResponse) -> LevelOneSkillRuntime<'static> {
        let skill = parse_skill(SKILL).unwrap();
        let fingerprint = RequestFingerprint::new(
            MODEL,
            Some(&skill.instructions),
            &[Message::user("Seattle")],
        );
        let client = Box::leak(Box::new(
            MockLlmClient::new()
                .with_response(fingerprint, response)
                .with_strict_default(),
        ));
        LevelOneSkillRuntime::new(skill, client, LevelOneRuntimeConfig::deterministic(MODEL))
            .unwrap()
    }

    fn package_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "chief-skill-runtime-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    struct FakeReceiver {
        id: AgentId,
        messages: Vec<ReceivedMessage>,
        acknowledgements: Vec<MessageId>,
        fail_receive: bool,
        fail_acknowledge: bool,
    }

    impl FakeReceiver {
        fn with_message(payload: Vec<u8>) -> Self {
            Self {
                id: AgentId::new(b"level-one-agent".to_vec()).unwrap(),
                messages: vec![ReceivedMessage {
                    message_id: message_id(7),
                    sequence: Sequence(4),
                    timestamp_ns: 10,
                    content_type: "text/plain".to_string(),
                    payload,
                }],
                acknowledgements: Vec::new(),
                fail_receive: false,
                fail_acknowledge: false,
            }
        }
    }

    impl Receiver for FakeReceiver {
        fn id(&self) -> &AgentId {
            &self.id
        }

        fn channel_id(&self) -> ChannelId {
            ChannelId([1; 16])
        }

        fn public_key(&self) -> [u8; 32] {
            [2; 32]
        }

        fn receive(&mut self, _limit: usize) -> Result<Vec<ReceivedMessage>, ChannelEndpointError> {
            if self.fail_receive {
                return Err(ChannelEndpointError::DefinitionNotFound);
            }
            Ok(std::mem::take(&mut self.messages))
        }

        fn acknowledge(&mut self, message_id: MessageId) -> Result<Sequence, ChannelEndpointError> {
            if self.fail_acknowledge {
                return Err(ChannelEndpointError::ChannelDestroyed);
            }
            self.acknowledgements.push(message_id);
            Ok(Sequence(5))
        }
    }

    struct FakeOriginator {
        id: AgentId,
        publications: RefCell<Vec<(Vec<u8>, String)>>,
        fail: Cell<bool>,
    }

    impl FakeOriginator {
        fn new() -> Self {
            Self {
                id: AgentId::new(b"level-one-output".to_vec()).unwrap(),
                publications: RefCell::new(Vec::new()),
                fail: Cell::new(false),
            }
        }
    }

    impl Originator for FakeOriginator {
        fn id(&self) -> &AgentId {
            &self.id
        }

        fn channel_id(&self) -> ChannelId {
            ChannelId([3; 16])
        }

        fn public_key(&self) -> [u8; 32] {
            [4; 32]
        }

        fn publish(
            &self,
            payload: &[u8],
            content_type: &str,
        ) -> Result<PublishedMessage, ChannelEndpointError> {
            if self.fail.get() {
                return Err(ChannelEndpointError::ChannelDestroyed);
            }
            self.publications
                .borrow_mut()
                .push((payload.to_vec(), content_type.to_string()));
            Ok(PublishedMessage {
                message_id: message_id(9),
                sequence: Sequence(8),
                timestamp_ns: 11,
            })
        }
    }

    #[test]
    fn respond_sends_exact_skill_and_message_to_gateway() {
        let runtime = runtime_with_response(MockResponse::Text(
            "Rain is likely; bring an umbrella.".to_string(),
        ));
        let response = runtime.respond("Seattle", "text/plain").unwrap();
        assert_eq!(response.text, "Rain is likely; bring an umbrella.");
        assert_eq!(response.model, MODEL);
        assert_eq!(runtime.skill().manifest.agent, "weather-reporter");
    }

    #[test]
    fn verified_skill_package_executes_through_the_level_one_runtime() {
        let path = package_dir("verified");
        let (public_key, secret_key) = generate_keypair(&[61; 32]);
        let built = build_signed_skill_package(&path, SKILL, "dev-level-one", &secret_key).unwrap();
        let mut keyring = PackageKeyring::new();
        keyring
            .trust(
                TrustedPackageKey::new(
                    "dev-level-one",
                    PackageKeyType::Developer,
                    public_key,
                    PrivilegeTier::Tier1,
                )
                .unwrap(),
            )
            .unwrap();
        let package = verify_agent_package(&path, &keyring).unwrap();
        let fingerprint = RequestFingerprint::new(
            MODEL,
            Some(&built.instructions),
            &[Message::user("Seattle")],
        );
        let client = MockLlmClient::new()
            .with_response(fingerprint, MockResponse::Text("Clear skies.".to_string()))
            .with_strict_default();
        let runtime = LevelOneSkillRuntime::from_verified_package(
            &package,
            &client,
            LevelOneRuntimeConfig::deterministic(MODEL),
        )
        .unwrap();
        assert_eq!(
            runtime.respond("Seattle", "text/plain").unwrap().text,
            "Clear skies."
        );
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn verified_launch_plan_requires_exact_signed_channels_and_model_settings() {
        let path = package_dir("wired-launch");
        let (public_key, secret_key) = generate_keypair(&[63; 32]);
        build_signed_skill_package(&path, WIRED_SKILL, "dev-wired", &secret_key).unwrap();
        let mut keyring = PackageKeyring::new();
        keyring
            .trust(
                TrustedPackageKey::new(
                    "dev-wired",
                    PackageKeyType::Developer,
                    public_key,
                    PrivilegeTier::Tier1,
                )
                .unwrap(),
            )
            .unwrap();
        let package = verify_agent_package(&path, &keyring).unwrap();
        let bindings = wired_bindings();
        let plan = LevelOneLaunchPlan::from_verified_package(&package, &bindings).unwrap();
        assert_eq!(plan.read_channel_id("weather-requests"), Some(uuid_v7(1)));
        assert_eq!(plan.write_channel_id("weather-reports"), Some(uuid_v7(2)));
        assert_eq!(plan.config().model(), MODEL);
        assert_eq!(plan.config().temperature(), 0.25);
        assert_eq!(plan.config().max_tokens(), 256);

        let missing = LaunchBindings::new(
            vec![
                ChannelBinding::new("weather-requests", ChannelBindingAccess::Read, uuid_v7(1))
                    .unwrap(),
            ],
            Some(LevelOneModelBinding::new(MODEL, 0.0, 128).unwrap()),
        )
        .unwrap();
        assert!(matches!(
            LevelOneLaunchPlan::from_verified_package(&package, &missing),
            Err(LevelOneRuntimeError::InvalidLaunchBindings)
        ));
        let no_model = LaunchBindings::new(bindings.channels().to_vec(), None).unwrap();
        assert!(matches!(
            LevelOneLaunchPlan::from_verified_package(&package, &no_model),
            Err(LevelOneRuntimeError::InvalidLaunchBindings)
        ));
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn successful_run_publishes_before_acknowledging() {
        let runtime = runtime_with_response(MockResponse::Text("Sunny.".to_string()));
        let mut receiver = FakeReceiver::with_message(b"Seattle".to_vec());
        let originator = FakeOriginator::new();
        let outcome = runtime.run_once(&mut receiver, &originator).unwrap();
        assert!(matches!(
            outcome,
            LevelOneRunOutcome::Processed {
                acknowledged_through: Sequence(5),
                ..
            }
        ));
        assert_eq!(receiver.acknowledgements, [message_id(7)]);
        assert_eq!(
            originator.publications.borrow().as_slice(),
            &[(
                b"Sunny.".to_vec(),
                LEVEL_ONE_RESPONSE_CONTENT_TYPE.to_string()
            )]
        );
    }

    #[test]
    fn idle_poll_never_calls_model_or_output() {
        let runtime = runtime_with_response(MockResponse::Text("unused".to_string()));
        let mut receiver = FakeReceiver::with_message(Vec::new());
        receiver.messages.clear();
        let originator = FakeOriginator::new();
        assert!(matches!(
            runtime.run_once(&mut receiver, &originator).unwrap(),
            LevelOneRunOutcome::Idle
        ));
        assert!(originator.publications.borrow().is_empty());
    }

    #[test]
    fn model_or_publish_failure_leaves_input_unacknowledged() {
        let runtime = runtime_with_response(MockResponse::Error(LlmError::Other {
            provider: response_provider(),
            message: "fixture failure".to_string(),
        }));
        let mut receiver = FakeReceiver::with_message(b"Seattle".to_vec());
        let originator = FakeOriginator::new();
        assert!(matches!(
            runtime.run_once(&mut receiver, &originator),
            Err(LevelOneRuntimeError::Llm(_))
        ));
        assert!(receiver.acknowledgements.is_empty());
        assert!(originator.publications.borrow().is_empty());

        let runtime = runtime_with_response(MockResponse::Text("Rain.".to_string()));
        let mut receiver = FakeReceiver::with_message(b"Seattle".to_vec());
        originator.fail.set(true);
        assert!(matches!(
            runtime.run_once(&mut receiver, &originator),
            Err(LevelOneRuntimeError::Channel(_))
        ));
        assert!(receiver.acknowledgements.is_empty());
    }

    #[test]
    fn invalid_input_response_and_config_fail_closed() {
        let runtime = runtime_with_response(MockResponse::Text("   ".to_string()));
        assert!(matches!(
            runtime.respond("Seattle", "text/plain"),
            Err(LevelOneRuntimeError::EmptyResponse)
        ));
        let mut receiver = FakeReceiver::with_message(vec![0xff]);
        let originator = FakeOriginator::new();
        assert!(matches!(
            runtime.run_once(&mut receiver, &originator),
            Err(LevelOneRuntimeError::NonUtf8Input)
        ));
        assert!(receiver.acknowledgements.is_empty());

        let skill = parse_skill(SKILL).unwrap();
        let client = MockLlmClient::new();
        let mut config = LevelOneRuntimeConfig::deterministic("");
        assert!(matches!(
            LevelOneSkillRuntime::new(skill.clone(), &client, config.clone()),
            Err(LevelOneRuntimeError::InvalidConfig(_))
        ));
        config.model = MODEL.to_string();
        config.temperature = f32::NAN;
        assert!(matches!(
            LevelOneSkillRuntime::new(skill.clone(), &client, config.clone()),
            Err(LevelOneRuntimeError::InvalidConfig(_))
        ));
        config.temperature = 0.0;
        config.max_tokens = 0;
        assert!(matches!(
            LevelOneSkillRuntime::new(skill, &client, config),
            Err(LevelOneRuntimeError::InvalidConfig(_))
        ));
    }

    #[test]
    fn channel_receive_and_acknowledgement_errors_are_preserved() {
        let runtime = runtime_with_response(MockResponse::Text("Rain.".to_string()));
        let originator = FakeOriginator::new();
        let mut receiver = FakeReceiver::with_message(b"Seattle".to_vec());
        receiver.fail_receive = true;
        assert!(matches!(
            runtime.run_once(&mut receiver, &originator),
            Err(LevelOneRuntimeError::Channel(_))
        ));

        let mut receiver = FakeReceiver::with_message(b"Seattle".to_vec());
        receiver.fail_acknowledge = true;
        assert!(matches!(
            runtime.run_once(&mut receiver, &originator),
            Err(LevelOneRuntimeError::Channel(_))
        ));
        assert_eq!(originator.publications.borrow().len(), 1);
    }

    #[test]
    fn completion_shape_conversion_preserves_audit_identity() {
        let completion = CompletionResponse {
            text: "Cloudy.".to_string(),
            model: MODEL.to_string(),
            usage: TokenUsage::default(),
            finish_reason: FinishReason::Stop,
            provider_id: response_provider(),
            latency_ms: 4,
        };
        let response = response_from_completion(completion).unwrap();
        assert_eq!(response.provider.vendor, "test");
        assert_eq!(response.model, MODEL);
        assert_eq!(response.usage, TokenUsage::default());
        assert_eq!(response.finish_reason, FinishReason::Stop);
        assert_eq!(response.latency_ms, 4);
    }

    #[test]
    fn config_and_public_errors_have_stable_text() {
        let config = LevelOneRuntimeConfig::deterministic(MODEL);
        assert_eq!(config.temperature, 0.0);
        assert_eq!(config.max_tokens, 1_024);
        assert!(LevelOneRuntimeError::NonUtf8Input
            .to_string()
            .contains("UTF-8"));
        assert!(LevelOneRuntimeError::InvalidConfig("bad")
            .to_string()
            .contains("bad"));
        assert!(LevelOneRuntimeError::EmptyResponse
            .to_string()
            .contains("empty"));
        let llm_error = LevelOneRuntimeError::from(LlmError::Other {
            provider: response_provider(),
            message: "provider failed".to_string(),
        });
        assert!(llm_error.to_string().contains("provider failed"));
        let channel_error = LevelOneRuntimeError::from(ChannelEndpointError::DefinitionNotFound);
        assert!(channel_error.to_string().contains("definition not found"));
    }
}
