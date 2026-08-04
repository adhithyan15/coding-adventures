//! Provider-neutral runtime for D18 Level 1 `SKILL.md` agents.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};

use chief_of_staff_channel_crypto::Sequence;
use chief_of_staff_channel_endpoints::{
    ChannelEndpointError, MessageId, Originator, PublishedMessage, Receiver,
};
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
    /// A verified channel payload was not UTF-8 text.
    NonUtf8Input,
    /// The provider returned an empty or whitespace-only response.
    EmptyResponse,
    /// The provider-neutral LLM gateway rejected the completion.
    Llm(Box<LlmError>),
    /// An injected channel endpoint rejected receive, publish, or acknowledge.
    Channel(ChannelEndpointError),
}

impl Display for LevelOneRuntimeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(message) => write!(formatter, "invalid Level 1 runtime: {message}"),
            Self::NonUtf8Input => formatter.write_str("Level 1 input payload is not UTF-8 text"),
            Self::EmptyResponse => formatter.write_str("Level 1 model response is empty"),
            Self::Llm(error) => write!(formatter, "Level 1 model call failed: {error}"),
            Self::Channel(error) => write!(formatter, "Level 1 channel operation failed: {error}"),
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
    use chief_of_staff_skill_parser::parse_skill;
    use llm_gateway::{MockLlmClient, MockResponse, RequestFingerprint};
    use std::cell::{Cell, RefCell};

    const SKILL: &str = "# Weather Reporter\n\nYou are a friendly weather reporting agent.\n\n## Capabilities needed\n- none\n\n## Output format\nKeep the forecast brief.\n";
    const MODEL: &str = "test-model";

    fn message_id(byte: u8) -> MessageId {
        let mut bytes = [byte; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        MessageId::from_uuid_v7(bytes).unwrap()
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
