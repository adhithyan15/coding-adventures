//! Concrete authenticated child host for D18 Chief agent packages.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_host_control_protocol::{
    ChannelBindingAccess, CompletionCall, CompletionFinishReason, CompletionProvider,
    CompletionResult, DataPlaneFailure, DataPlaneResponse, LaunchBindings,
    ModelToolCall as WireModelToolCall, ModelToolChoice as WireModelToolChoice,
    ModelToolDefinition as WireModelToolDefinition, ModelToolResult as WireModelToolResult,
    PromptMessage, PromptRole, ToolCompletionCall as WireToolCompletionCall,
    ToolCompletionOutput as WireToolCompletionOutput,
    ToolCompletionResult as WireToolCompletionResult,
};
use chief_of_staff_host_runtime::{
    verify_agent_package, AgentPackageRuntime, PackageKeyring, PackageVerificationError,
};
use chief_of_staff_process_supervisor::{ChildProcessControl, ProcessSupervisorError};
use chief_of_staff_skill_runtime::{
    LevelOneLaunchPlan, LevelOneRuntimeError, LevelOneSkillRuntime, LEVEL_ONE_RESPONSE_CONTENT_TYPE,
};
use llm_gateway::{
    Capabilities, CompletionJsonResponse, CompletionRequest, CompletionResponse, FinishReason,
    JsonSchema, LlmClient, LlmError, MessageContent, ModelToolCall, ModelToolChoice,
    ProviderIdentity, Role, TokenUsage, ToolCompletionOutput, ToolCompletionRequest,
    ToolCompletionResponse,
};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt::{self, Display, Formatter};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

const PACKAGE_RUNTIME_ARGUMENT: &str = "--package-runtime";
const SKILL_RUNTIME_LABEL: &str = "skill";
const IDLE_POLL_INTERVAL: Duration = Duration::from_millis(250);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(1);

/// Normal terminal condition for one concrete host process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostExit {
    /// The orchestrator sent an authenticated graceful termination record.
    Terminated,
}

/// Stable failure from host arguments, verification, policy, control, or execution.
#[derive(Debug)]
pub enum HostError {
    /// Reserved process arguments were absent, duplicated, or malformed.
    InvalidArguments,
    /// The configured package runtime has no concrete adapter in this executable.
    UnsupportedRuntime,
    /// The first Level 1 host requires exactly one read and one write channel.
    UnsupportedTopology,
    /// Package verification failed against authenticated public trust.
    Package(Box<PackageVerificationError>),
    /// Signed Level 1 policy or execution failed.
    Runtime(Box<LevelOneRuntimeError>),
    /// Secure bootstrap, framing, or authenticated control failed.
    Control(ProcessSupervisorError),
    /// An authenticated data-plane operation returned a redacted failure.
    DataPlane(DataPlaneFailure),
    /// A successful response did not have the exact locally required shape.
    ResponseShape,
    /// An internal control lock was poisoned by an earlier failure.
    ControlPoisoned,
}

impl Display for HostError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidArguments => "chief-host: invalid process arguments",
            Self::UnsupportedRuntime => "chief-host: package runtime is not supported",
            Self::UnsupportedTopology => "chief-host: Level 1 topology is not supported",
            Self::Package(_) => "chief-host: package verification failed",
            Self::Runtime(_) => "chief-host: Level 1 execution failed",
            Self::Control(_) => "chief-host: authenticated control failed",
            Self::DataPlane(_) => "chief-host: data-plane operation failed",
            Self::ResponseShape => "chief-host: invalid data-plane response",
            Self::ControlPoisoned => "chief-host: control state is unavailable",
        })
    }
}

impl std::error::Error for HostError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Package(error) => Some(error),
            Self::Runtime(error) => Some(error),
            Self::Control(error) => Some(error),
            _ => None,
        }
    }
}

impl From<PackageVerificationError> for HostError {
    fn from(error: PackageVerificationError) -> Self {
        Self::Package(Box::new(error))
    }
}

impl From<LevelOneRuntimeError> for HostError {
    fn from(error: LevelOneRuntimeError) -> Self {
        Self::Runtime(Box::new(error))
    }
}

impl From<ProcessSupervisorError> for HostError {
    fn from(error: ProcessSupervisorError) -> Self {
        Self::Control(error)
    }
}

/// Parse the reserved runtime argument and run the concrete host over stdio.
pub fn run_from_env() -> Result<HostExit, HostError> {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    require_skill_runtime_argument(&arguments)?;
    let reader = BufReader::new(io::stdin());
    let writer = BufWriter::new(io::stdout());
    run_level_one_host(reader, writer, Path::new("."))
}

/// Run one independently verifying Level 1 host over caller-owned streams.
pub fn run_level_one_host<R, W>(
    reader: R,
    writer: W,
    package_path: &Path,
) -> Result<HostExit, HostError>
where
    R: Read + Send,
    W: Write + Send,
{
    let mut control = ChildProcessControl::bootstrap(reader, writer)?;
    let mut keyring = PackageKeyring::new();
    keyring.trust(control.receive_package_trust()?)?;
    let package = verify_agent_package(package_path, &keyring)?;
    if package.runtime() != AgentPackageRuntime::Skill {
        return Err(HostError::UnsupportedRuntime);
    }
    let bindings = control.receive_launch_bindings()?;
    let plan = LevelOneLaunchPlan::from_verified_package(&package, &bindings)?;
    let (read_channel, write_channel) = sole_level_one_channels(&bindings)?;
    control.ready(package.digest())?;

    let terminated = AtomicBool::new(false);
    let control = Mutex::new(control);
    let client = ControlLlmClient::new(&control, &terminated, plan.config().model());
    let runtime = plan.runtime(&client)?;
    let mut last_heartbeat = Instant::now();

    loop {
        match run_level_one_once(&control, &terminated, &runtime, read_channel, write_channel) {
            Ok(HostStep::Idle | HostStep::Unavailable) => thread::sleep(IDLE_POLL_INTERVAL),
            Ok(HostStep::Processed) => {}
            Err(HostError::Control(ProcessSupervisorError::Terminated)) => {
                return Ok(HostExit::Terminated)
            }
            Err(HostError::Runtime(_)) if terminated.load(Ordering::SeqCst) => {
                return Ok(HostExit::Terminated)
            }
            Err(error) => return Err(error),
        }
        if last_heartbeat.elapsed() >= HEARTBEAT_INTERVAL {
            lock_control(&control)?.heartbeat()?;
            last_heartbeat = Instant::now();
        }
    }
}

fn require_skill_runtime_argument(arguments: &[OsString]) -> Result<(), HostError> {
    if arguments.len() != 2 || arguments[0] != PACKAGE_RUNTIME_ARGUMENT {
        return Err(HostError::InvalidArguments);
    }
    match arguments[1].to_str() {
        Some(SKILL_RUNTIME_LABEL) => Ok(()),
        Some(_) => Err(HostError::UnsupportedRuntime),
        None => Err(HostError::InvalidArguments),
    }
}

fn sole_level_one_channels(bindings: &LaunchBindings) -> Result<([u8; 16], [u8; 16]), HostError> {
    let reads = bindings
        .channels()
        .iter()
        .filter(|binding| binding.access() == ChannelBindingAccess::Read)
        .collect::<Vec<_>>();
    let writes = bindings
        .channels()
        .iter()
        .filter(|binding| binding.access() == ChannelBindingAccess::Write)
        .collect::<Vec<_>>();
    match (reads.as_slice(), writes.as_slice()) {
        ([read], [write]) => Ok((read.channel_id(), write.channel_id())),
        _ => Err(HostError::UnsupportedTopology),
    }
}

enum HostStep {
    Idle,
    Unavailable,
    Processed,
}

fn run_level_one_once<R, W>(
    control: &Mutex<ChildProcessControl<R, W>>,
    terminated: &AtomicBool,
    runtime: &LevelOneSkillRuntime<'_>,
    read_channel: [u8; 16],
    write_channel: [u8; 16],
) -> Result<HostStep, HostError>
where
    R: Read + Send,
    W: Write + Send,
{
    let received = lock_control(control)?.request_receive(read_channel, 1)?;
    let message = match received {
        DataPlaneResponse::Received { messages, .. } => match messages.as_slice() {
            [] => return Ok(HostStep::Idle),
            [message] => message.clone(),
            _ => return Err(HostError::ResponseShape),
        },
        DataPlaneResponse::Failed {
            failure: DataPlaneFailure::Unavailable,
            ..
        } => return Ok(HostStep::Unavailable),
        DataPlaneResponse::Failed { failure, .. } => return Err(HostError::DataPlane(failure)),
        _ => return Err(HostError::ResponseShape),
    };
    let input =
        std::str::from_utf8(&message.payload).map_err(|_| LevelOneRuntimeError::NonUtf8Input)?;
    let response = match runtime.respond(input, &message.content_type) {
        Ok(response) => response,
        Err(_error) if terminated.load(Ordering::SeqCst) => {
            return Err(HostError::Control(ProcessSupervisorError::Terminated))
        }
        Err(error) => return Err(error.into()),
    };
    let published = lock_control(control)?.request_publish(
        write_channel,
        LEVEL_ONE_RESPONSE_CONTENT_TYPE.to_string(),
        response.text.into_bytes(),
    )?;
    match published {
        DataPlaneResponse::Published { .. } => {}
        DataPlaneResponse::Failed { failure, .. } => return Err(HostError::DataPlane(failure)),
        _ => return Err(HostError::ResponseShape),
    }
    let acknowledged =
        lock_control(control)?.request_acknowledge(read_channel, message.message_id)?;
    match acknowledged {
        DataPlaneResponse::Acknowledged { .. } => Ok(HostStep::Processed),
        DataPlaneResponse::Failed { failure, .. } => Err(HostError::DataPlane(failure)),
        _ => Err(HostError::ResponseShape),
    }
}

fn lock_control<R, W>(
    control: &Mutex<ChildProcessControl<R, W>>,
) -> Result<MutexGuard<'_, ChildProcessControl<R, W>>, HostError>
where
    R: Read + Send,
    W: Write + Send,
{
    control.lock().map_err(|_| HostError::ControlPoisoned)
}

struct ControlLlmClient<'a, R: Read + Send, W: Write + Send> {
    control: &'a Mutex<ChildProcessControl<R, W>>,
    terminated: &'a AtomicBool,
    identity: ProviderIdentity,
}

#[derive(Clone, Copy)]
enum CompletionAdapterError {
    Multimodal,
    TokenCap,
    Usage,
}

impl CompletionAdapterError {
    fn message(self) -> &'static str {
        match self {
            Self::Multimodal => "multimodal completion is unavailable",
            Self::TokenCap => "completion token cap is out of range",
            Self::Usage => "completion usage is out of range",
        }
    }
}

impl<'a, R: Read + Send, W: Write + Send> ControlLlmClient<'a, R, W> {
    fn new(
        control: &'a Mutex<ChildProcessControl<R, W>>,
        terminated: &'a AtomicBool,
        model: &str,
    ) -> Self {
        Self {
            control,
            terminated,
            identity: ProviderIdentity {
                vendor: "chief-host-data-plane".to_string(),
                model_family: model.to_string(),
                model_version: "authenticated".to_string(),
                endpoint: None,
            },
        }
    }

    fn error(&self, message: &str) -> LlmError {
        LlmError::Other {
            provider: self.identity.clone(),
            message: message.to_string(),
        }
    }

    fn completion_call(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionCall, CompletionAdapterError> {
        let messages = request
            .messages
            .into_iter()
            .map(|message| {
                let role = match message.role {
                    Role::System => PromptRole::System,
                    Role::User => PromptRole::User,
                    Role::Assistant => PromptRole::Assistant,
                };
                let MessageContent::Text(text) = message.content else {
                    return Err(CompletionAdapterError::Multimodal);
                };
                Ok(PromptMessage { role, text })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let max_tokens = request
            .max_tokens
            .map(u32::try_from)
            .transpose()
            .map_err(|_| CompletionAdapterError::TokenCap)?;
        Ok(CompletionCall {
            model: request.model,
            system: request.system,
            messages,
            temperature: request.temperature,
            max_tokens,
            stop_sequences: request.stop_sequences,
            seed: request.seed,
            metadata: request.metadata.into_iter().collect::<BTreeMap<_, _>>(),
        })
    }

    fn completion_response(
        &self,
        result: CompletionResult,
    ) -> Result<CompletionResponse, CompletionAdapterError> {
        let input_tokens = usize::try_from(result.usage.input_tokens)
            .map_err(|_| CompletionAdapterError::Usage)?;
        let output_tokens = usize::try_from(result.usage.output_tokens)
            .map_err(|_| CompletionAdapterError::Usage)?;
        let cached_tokens = usize::try_from(result.usage.cached_tokens)
            .map_err(|_| CompletionAdapterError::Usage)?;
        Ok(CompletionResponse {
            text: result.text,
            model: result.model,
            usage: TokenUsage {
                input_tokens,
                output_tokens,
                cached_tokens,
            },
            finish_reason: match result.finish_reason {
                CompletionFinishReason::Stop => FinishReason::Stop,
                CompletionFinishReason::MaxTokens => FinishReason::MaxTokens,
                CompletionFinishReason::Refusal => FinishReason::Refusal,
                CompletionFinishReason::Other => FinishReason::Other,
            },
            provider_id: provider_identity(result.provider),
            latency_ms: result.latency_ms,
        })
    }

    fn tool_completion_call(
        &self,
        request: ToolCompletionRequest,
    ) -> Result<WireToolCompletionCall, CompletionAdapterError> {
        Ok(WireToolCompletionCall {
            completion: self.completion_call(request.completion)?,
            tools: request
                .tools
                .into_iter()
                .map(|tool| WireModelToolDefinition {
                    name: tool.name,
                    description: tool.description,
                    input_schema: tool.input_schema,
                })
                .collect(),
            choice: match request.choice {
                ModelToolChoice::Auto => WireModelToolChoice::Auto,
                ModelToolChoice::Required => WireModelToolChoice::Required,
                ModelToolChoice::Named(name) => WireModelToolChoice::Named(name),
            },
            results: request
                .results
                .into_iter()
                .map(|result| WireModelToolResult {
                    call: WireModelToolCall {
                        call_id: result.call.call_id,
                        name: result.call.name,
                        arguments: result.call.arguments,
                    },
                    output: result.output,
                    is_error: result.is_error,
                })
                .collect(),
        })
    }

    fn tool_completion_response(
        &self,
        result: WireToolCompletionResult,
    ) -> Result<ToolCompletionResponse, CompletionAdapterError> {
        let input_tokens = usize::try_from(result.usage.input_tokens)
            .map_err(|_| CompletionAdapterError::Usage)?;
        let output_tokens = usize::try_from(result.usage.output_tokens)
            .map_err(|_| CompletionAdapterError::Usage)?;
        let cached_tokens = usize::try_from(result.usage.cached_tokens)
            .map_err(|_| CompletionAdapterError::Usage)?;
        Ok(ToolCompletionResponse {
            output: match result.output {
                WireToolCompletionOutput::FinalText(text) => ToolCompletionOutput::FinalText(text),
                WireToolCompletionOutput::ToolCall(call) => {
                    ToolCompletionOutput::ToolCall(ModelToolCall {
                        call_id: call.call_id,
                        name: call.name,
                        arguments: call.arguments,
                    })
                }
            },
            model: result.model,
            usage: TokenUsage {
                input_tokens,
                output_tokens,
                cached_tokens,
            },
            finish_reason: protocol_finish_reason(result.finish_reason),
            provider_id: provider_identity(result.provider),
            latency_ms: result.latency_ms,
            polyfill_used: result.polyfill_used,
        })
    }
}

impl<R: Read + Send, W: Write + Send> LlmClient for ControlLlmClient<'_, R, W> {
    fn identity(&self) -> ProviderIdentity {
        self.identity.clone()
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            json_mode_native: false,
            tool_use_native: false,
            streaming_native: false,
            prompt_caching_native: false,
            multimodal_image_input: false,
            max_context_window: 0,
        }
    }

    fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let call = self
            .completion_call(request)
            .map_err(|error| self.error(error.message()))?;
        let response = self
            .control
            .lock()
            .map_err(|_| self.error("host control is unavailable"))?
            .request_completion(call);
        let response = match response {
            Ok(response) => response,
            Err(ProcessSupervisorError::Terminated) => {
                self.terminated.store(true, Ordering::SeqCst);
                return Err(self.error("host termination requested"));
            }
            Err(_) => return Err(self.error("host control is unavailable")),
        };
        match response {
            DataPlaneResponse::Completed { result, .. } => self
                .completion_response(*result)
                .map_err(|error| self.error(error.message())),
            DataPlaneResponse::Failed { .. } => Err(self.error("completion failed")),
            _ => Err(self.error("invalid completion response")),
        }
    }

    fn complete_json(
        &self,
        _request: CompletionRequest,
        _schema: &JsonSchema,
    ) -> Result<CompletionJsonResponse, LlmError> {
        Err(self.error("structured completion is unavailable"))
    }

    fn complete_with_tools(
        &self,
        request: ToolCompletionRequest,
    ) -> Result<ToolCompletionResponse, LlmError> {
        let call = self
            .tool_completion_call(request)
            .map_err(|error| self.error(error.message()))?;
        let response = self
            .control
            .lock()
            .map_err(|_| self.error("host control is unavailable"))?
            .request_tool_completion(call.clone());
        let response = match response {
            Ok(response) => response,
            Err(ProcessSupervisorError::Terminated) => {
                self.terminated.store(true, Ordering::SeqCst);
                return Err(self.error("host termination requested"));
            }
            Err(_) => return Err(self.error("host control is unavailable")),
        };
        match response {
            DataPlaneResponse::ToolCompleted { result, .. } => {
                if !wire_tool_output_allowed(&result.output, &call.tools, &call.choice) {
                    return Err(self.error("tool completion selected an unoffered output"));
                }
                self.tool_completion_response(*result)
                    .map_err(|error| self.error(error.message()))
            }
            DataPlaneResponse::Failed { .. } => Err(self.error("tool completion failed")),
            _ => Err(self.error("invalid tool completion response")),
        }
    }
}

fn protocol_finish_reason(reason: CompletionFinishReason) -> FinishReason {
    match reason {
        CompletionFinishReason::Stop => FinishReason::Stop,
        CompletionFinishReason::MaxTokens => FinishReason::MaxTokens,
        CompletionFinishReason::Refusal => FinishReason::Refusal,
        CompletionFinishReason::Other => FinishReason::Other,
    }
}

fn wire_tool_output_allowed(
    output: &WireToolCompletionOutput,
    tools: &[WireModelToolDefinition],
    choice: &WireModelToolChoice,
) -> bool {
    match output {
        WireToolCompletionOutput::FinalText(text) => {
            !text.is_empty() && matches!(choice, WireModelToolChoice::Auto)
        }
        WireToolCompletionOutput::ToolCall(call) => {
            tools.iter().any(|tool| tool.name == call.name)
                && match choice {
                    WireModelToolChoice::Named(name) => name == &call.name,
                    WireModelToolChoice::Auto | WireModelToolChoice::Required => true,
                }
        }
    }
}

fn provider_identity(provider: CompletionProvider) -> ProviderIdentity {
    ProviderIdentity {
        vendor: provider.vendor,
        model_family: provider.model_family,
        model_version: provider.model_version,
        endpoint: provider.endpoint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chief_of_staff_host_control_protocol::{ChannelBinding, LevelOneModelBinding};

    fn uuid_v7(last: u8) -> [u8; 16] {
        let mut bytes = [0; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = last;
        bytes
    }

    fn binding(name: &str, access: ChannelBindingAccess, id: u8) -> ChannelBinding {
        ChannelBinding::new(name, access, uuid_v7(id)).unwrap()
    }

    #[test]
    fn process_arguments_select_only_the_skill_runtime() {
        assert!(require_skill_runtime_argument(&[
            OsString::from("--package-runtime"),
            OsString::from("skill")
        ])
        .is_ok());
        assert!(matches!(
            require_skill_runtime_argument(&[
                OsString::from("--package-runtime"),
                OsString::from("deno")
            ]),
            Err(HostError::UnsupportedRuntime)
        ));
        assert!(matches!(
            require_skill_runtime_argument(&[]),
            Err(HostError::InvalidArguments)
        ));
    }

    #[test]
    fn first_level_one_host_requires_one_read_and_one_write() {
        let model = Some(LevelOneModelBinding::new("test-model", 0.0, 128).unwrap());
        let supported = LaunchBindings::new(
            vec![
                binding("requests", ChannelBindingAccess::Read, 1),
                binding("reports", ChannelBindingAccess::Write, 2),
            ],
            model.clone(),
        )
        .unwrap();
        assert_eq!(
            sole_level_one_channels(&supported).unwrap(),
            (uuid_v7(1), uuid_v7(2))
        );

        let multiple_reads = LaunchBindings::new(
            vec![
                binding("alerts", ChannelBindingAccess::Read, 3),
                binding("requests", ChannelBindingAccess::Read, 1),
                binding("reports", ChannelBindingAccess::Write, 2),
            ],
            model,
        )
        .unwrap();
        assert!(matches!(
            sole_level_one_channels(&multiple_reads),
            Err(HostError::UnsupportedTopology)
        ));
    }

    #[test]
    fn tool_response_must_match_the_offered_catalog_and_choice() {
        let tools = vec![WireModelToolDefinition {
            name: "smart_home.list_entities".to_string(),
            description: "List normalized entities".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
        }];
        let offered = WireToolCompletionOutput::ToolCall(WireModelToolCall {
            call_id: "call-1".to_string(),
            name: "smart_home.list_entities".to_string(),
            arguments: serde_json::json!({}),
        });
        let unoffered = WireToolCompletionOutput::ToolCall(WireModelToolCall {
            call_id: "call-2".to_string(),
            name: "smart_home.command".to_string(),
            arguments: serde_json::json!({}),
        });
        assert!(wire_tool_output_allowed(
            &offered,
            &tools,
            &WireModelToolChoice::Required
        ));
        assert!(!wire_tool_output_allowed(
            &unoffered,
            &tools,
            &WireModelToolChoice::Auto
        ));
        assert!(!wire_tool_output_allowed(
            &WireToolCompletionOutput::FinalText("done".to_string()),
            &tools,
            &WireModelToolChoice::Named("smart_home.list_entities".to_string())
        ));
    }
}
