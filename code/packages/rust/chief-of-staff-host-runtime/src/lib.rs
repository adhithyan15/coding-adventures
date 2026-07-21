//! Profile-gated D18D tool runtime for Chief of Staff hosts.

#![forbid(unsafe_code)]

mod package;

pub use package::{
    verify_agent_package, DenoLaunchPlan, PackageKeyType, PackageKeyring, PackageVerificationError,
    TrustedPackageKey, VerifiedAgentPackage,
};

use chief_of_staff_tool_api::{
    validate_tool_id, InMemoryToolRuntime, PrivilegeTier, RequestedBy, ToolApiError,
    ToolDefinition, ToolExecutionTrace, ToolHandler, ToolInvocationRequest,
};
use coding_adventures_json_serializer::serialize as serialize_json;
use coding_adventures_json_value::{parse as parse_json, JsonValue};
use generic_job_protocol::{JobRequest, JobResponse};
use generic_job_runtime::{
    ExecutorLimits, ExecutorSnapshot, JobExecutor, StdioProcessPool, StdioProcessPoolOptions,
    StdioWorkerCommand, StdioWorkerRestartPolicy,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostProfile {
    pub profile_id: String,
    pub host_id: String,
    pub max_tier: PrivilegeTier,
    pub allowed_tools: Vec<String>,
    pub capabilities: Vec<String>,
}

impl HostProfile {
    pub fn from_json(text: &str) -> Result<Self, HostRuntimeError> {
        let value = parse_json(text)
            .map_err(|error| HostRuntimeError::InvalidJson(error.message.to_string()))?;
        let object = expect_object(&value, "$profile")?;
        let profile = Self {
            profile_id: required_string(object, "profile_id")?,
            host_id: required_string(object, "host_id")?,
            max_tier: parse_tier(&required_string(object, "max_tier")?)?,
            allowed_tools: required_string_array(object, "allowed_tools")?,
            capabilities: required_string_array(object, "capabilities")?,
        };
        profile.validate()?;
        Ok(profile)
    }

    pub fn validate(&self) -> Result<(), HostRuntimeError> {
        validate_label("profile_id", &self.profile_id)?;
        validate_label("host_id", &self.host_id)?;
        if self.allowed_tools.is_empty() {
            return Err(HostRuntimeError::EmptyToolCatalog);
        }

        let mut tools = BTreeSet::new();
        for tool_id in &self.allowed_tools {
            validate_tool_id(tool_id).map_err(HostRuntimeError::ToolApi)?;
            if !tools.insert(tool_id.clone()) {
                return Err(HostRuntimeError::DuplicateTool(tool_id.clone()));
            }
        }

        let mut capabilities = BTreeSet::new();
        for capability in &self.capabilities {
            validate_label("capability", capability)?;
            if !capabilities.insert(capability.clone()) {
                return Err(HostRuntimeError::DuplicateCapability(capability.clone()));
            }
        }
        Ok(())
    }

    pub fn allows_tool(&self, tool_id: &str) -> bool {
        self.allowed_tools.iter().any(|allowed| allowed == tool_id)
    }

    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities
            .iter()
            .any(|declared| declared == capability)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestratorProfile {
    pub profile_id: String,
    pub hosts: Vec<HostProfile>,
}

impl OrchestratorProfile {
    pub fn from_json(text: &str) -> Result<Self, HostRuntimeError> {
        let value = parse_json(text)
            .map_err(|error| HostRuntimeError::InvalidJson(error.message.to_string()))?;
        let object = expect_object(&value, "$profile")?;
        let profile_id = required_string(object, "profile_id")?;
        validate_label("profile_id", &profile_id)?;
        let JsonValue::Array(host_values) = required_value(object, "hosts")? else {
            return Err(HostRuntimeError::InvalidField {
                field: "hosts".to_string(),
                message: "expected array".to_string(),
            });
        };
        if host_values.is_empty() {
            return Err(HostRuntimeError::EmptyHostCatalog);
        }

        let mut hosts = Vec::with_capacity(host_values.len());
        let mut host_ids = BTreeSet::new();
        let mut tool_owners = BTreeMap::new();
        for (index, host_value) in host_values.iter().enumerate() {
            let host_object = expect_object(host_value, &format!("hosts[{index}]"))?;
            let host = HostProfile {
                profile_id: profile_id.clone(),
                host_id: required_string(host_object, "host_id")?,
                max_tier: parse_tier(&required_string(host_object, "max_tier")?)?,
                allowed_tools: required_string_array(host_object, "allowed_tools")?,
                capabilities: required_string_array(host_object, "capabilities")?,
            };
            host.validate()?;
            if !host_ids.insert(host.host_id.clone()) {
                return Err(HostRuntimeError::DuplicateHost(host.host_id));
            }
            for tool_id in &host.allowed_tools {
                if let Some(first_host) = tool_owners.insert(tool_id.clone(), host.host_id.clone())
                {
                    return Err(HostRuntimeError::DuplicateToolOwner {
                        tool_id: tool_id.clone(),
                        first_host,
                        second_host: host.host_id.clone(),
                    });
                }
            }
            hosts.push(host);
        }
        Ok(Self { profile_id, hosts })
    }

    pub fn validate(&self) -> Result<(), HostRuntimeError> {
        validate_label("profile_id", &self.profile_id)?;
        if self.hosts.is_empty() {
            return Err(HostRuntimeError::EmptyHostCatalog);
        }
        let mut host_ids = BTreeSet::new();
        let mut tool_owners = BTreeMap::new();
        for host in &self.hosts {
            host.validate()?;
            if host.profile_id != self.profile_id {
                return Err(HostRuntimeError::InvalidField {
                    field: "hosts.profile_id".to_string(),
                    message: format!(
                        "host '{}' belongs to profile '{}' instead of '{}'",
                        host.host_id, host.profile_id, self.profile_id
                    ),
                });
            }
            if !host_ids.insert(host.host_id.clone()) {
                return Err(HostRuntimeError::DuplicateHost(host.host_id.clone()));
            }
            for tool_id in &host.allowed_tools {
                if let Some(first_host) = tool_owners.insert(tool_id.clone(), host.host_id.clone())
                {
                    return Err(HostRuntimeError::DuplicateToolOwner {
                        tool_id: tool_id.clone(),
                        first_host,
                        second_host: host.host_id.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostProfileSummary {
    pub profile_id: String,
    pub host_id: String,
    pub max_tier: PrivilegeTier,
    pub allowed_tool_count: usize,
    pub registered_tool_count: usize,
    pub capability_count: usize,
    pub active: bool,
}

pub struct HostProfileRuntime {
    profile: HostProfile,
    runtime: InMemoryToolRuntime,
    registered_tools: BTreeSet<String>,
}

pub struct OrchestratorProfileRuntime {
    profile_id: String,
    hosts: BTreeMap<String, HostProfileRuntime>,
    tool_owners: BTreeMap<String, String>,
}

impl OrchestratorProfileRuntime {
    pub fn from_json(text: &str) -> Result<Self, HostRuntimeError> {
        Self::new(OrchestratorProfile::from_json(text)?)
    }

    pub fn new(profile: OrchestratorProfile) -> Result<Self, HostRuntimeError> {
        profile.validate()?;
        let mut hosts = BTreeMap::new();
        let mut tool_owners = BTreeMap::new();
        for host in profile.hosts {
            for tool_id in &host.allowed_tools {
                tool_owners.insert(tool_id.clone(), host.host_id.clone());
            }
            hosts.insert(host.host_id.clone(), HostProfileRuntime::new(host)?);
        }
        Ok(Self {
            profile_id: profile.profile_id,
            hosts,
            tool_owners,
        })
    }

    pub fn register_handler<H>(
        &mut self,
        definition: ToolDefinition,
        handler: H,
    ) -> Result<(), HostRuntimeError>
    where
        H: ToolHandler + 'static,
    {
        let tool_id = definition.tool_id.clone();
        let host_id = self
            .tool_owners
            .get(&tool_id)
            .ok_or_else(|| HostRuntimeError::ToolNotAllowed(tool_id.clone()))?;
        self.hosts
            .get_mut(host_id)
            .expect("validated orchestrator profile must retain each tool owner")
            .register_handler(definition, handler)
    }

    pub fn summary(&self) -> OrchestratorProfileSummary {
        let registered_tool_count = self
            .hosts
            .values()
            .map(|host| host.registered_tools.len())
            .sum();
        OrchestratorProfileSummary {
            profile_id: self.profile_id.clone(),
            host_count: self.hosts.len(),
            allowed_tool_count: self.tool_owners.len(),
            registered_tool_count,
            active: false,
        }
    }

    pub fn activate(self) -> Result<ActiveOrchestratorRuntime, HostRuntimeError> {
        let mut hosts = BTreeMap::new();
        for (host_id, runtime) in self.hosts {
            hosts.insert(host_id, runtime.activate()?);
        }
        Ok(ActiveOrchestratorRuntime {
            profile_id: self.profile_id,
            hosts,
            tool_owners: self.tool_owners,
        })
    }
}

impl HostProfileRuntime {
    pub fn from_json(text: &str) -> Result<Self, HostRuntimeError> {
        Self::new(HostProfile::from_json(text)?)
    }

    pub fn new(profile: HostProfile) -> Result<Self, HostRuntimeError> {
        profile.validate()?;
        Ok(Self {
            profile,
            runtime: InMemoryToolRuntime::new(),
            registered_tools: BTreeSet::new(),
        })
    }

    pub fn profile(&self) -> &HostProfile {
        &self.profile
    }

    pub fn register_handler<H>(
        &mut self,
        definition: ToolDefinition,
        handler: H,
    ) -> Result<(), HostRuntimeError>
    where
        H: ToolHandler + 'static,
    {
        let tool_id = definition.tool_id.clone();
        if !self.profile.allows_tool(&tool_id) {
            return Err(HostRuntimeError::ToolNotAllowed(tool_id));
        }
        if definition.required_tier > self.profile.max_tier {
            return Err(HostRuntimeError::PrivilegeCeilingExceeded {
                tool_id,
                required: definition.required_tier,
                maximum: self.profile.max_tier,
            });
        }
        for capability in &definition.required_capabilities {
            if !self.profile.has_capability(capability) {
                return Err(HostRuntimeError::MissingCapability {
                    tool_id: definition.tool_id.clone(),
                    capability: capability.clone(),
                });
            }
        }
        self.runtime
            .register_handler(definition, handler)
            .map_err(HostRuntimeError::ToolApi)?;
        self.registered_tools.insert(tool_id);
        Ok(())
    }

    pub fn summary(&self) -> HostProfileSummary {
        HostProfileSummary {
            profile_id: self.profile.profile_id.clone(),
            host_id: self.profile.host_id.clone(),
            max_tier: self.profile.max_tier,
            allowed_tool_count: self.profile.allowed_tools.len(),
            registered_tool_count: self.registered_tools.len(),
            capability_count: self.profile.capabilities.len(),
            active: false,
        }
    }

    pub fn activate(self) -> Result<ActiveHostToolRuntime, HostRuntimeError> {
        let missing_tools = self
            .profile
            .allowed_tools
            .iter()
            .filter(|tool_id| !self.registered_tools.contains(*tool_id))
            .cloned()
            .collect::<Vec<_>>();
        if !missing_tools.is_empty() {
            return Err(HostRuntimeError::CatalogIncomplete(missing_tools));
        }
        Ok(ActiveHostToolRuntime {
            profile: self.profile,
            runtime: self.runtime,
            registered_tools: self.registered_tools,
        })
    }
}

pub struct ActiveHostToolRuntime {
    profile: HostProfile,
    runtime: InMemoryToolRuntime,
    registered_tools: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestratorProfileSummary {
    pub profile_id: String,
    pub host_count: usize,
    pub allowed_tool_count: usize,
    pub registered_tool_count: usize,
    pub active: bool,
}

pub struct ActiveOrchestratorRuntime {
    profile_id: String,
    hosts: BTreeMap<String, ActiveHostToolRuntime>,
    tool_owners: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostRpcRequest {
    pub call_id: String,
    pub tool_id: String,
    pub arguments_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostRpcResponse {
    pub output_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostProcessSpec {
    pub host_id: String,
    pub command: StdioWorkerCommand,
    pub restart_policy: StdioWorkerRestartPolicy,
}

impl HostProcessSpec {
    pub fn new(
        host_id: impl Into<String>,
        command: StdioWorkerCommand,
        restart_policy: StdioWorkerRestartPolicy,
    ) -> Self {
        Self {
            host_id: host_id.into(),
            command,
            restart_policy,
        }
    }

    pub fn deny_all_deno(
        host_id: impl Into<String>,
        deno_program: impl Into<String>,
        package: &VerifiedAgentPackage,
        restart_policy: StdioWorkerRestartPolicy,
    ) -> Result<Self, HostRuntimeError> {
        let entrypoint = package.path().join(DenoLaunchPlan::entrypoint_relative());
        if !entrypoint.is_file() {
            return Err(HostRuntimeError::MissingDenoEntrypoint(entrypoint));
        }
        Ok(Self::new(
            host_id,
            StdioWorkerCommand::new(
                deno_program,
                DenoLaunchPlan::arguments(&entrypoint)
                    .map_err(|_| HostRuntimeError::InvalidDenoEntrypoint(entrypoint.clone()))?,
            ),
            restart_policy,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupervisedHostSnapshot {
    pub host_id: String,
    pub allowed_tool_count: usize,
    pub process: ExecutorSnapshot,
}

pub struct SupervisedOrchestratorRuntime {
    profile_id: String,
    tool_owners: BTreeMap<String, String>,
    allowed_tool_counts: BTreeMap<String, usize>,
    hosts: BTreeMap<String, StdioProcessPool<HostRpcRequest, HostRpcResponse>>,
}

impl SupervisedOrchestratorRuntime {
    pub fn spawn_deno_verified(
        profile: OrchestratorProfile,
        package_path: &std::path::Path,
        keyring: &PackageKeyring,
        deno_program: impl Into<String>,
        restart_policy: StdioWorkerRestartPolicy,
    ) -> Result<Self, HostRuntimeError> {
        let package = verify_agent_package(package_path, keyring)
            .map_err(|error| HostRuntimeError::PackageVerification(error.to_string()))?;
        let deno_program = deno_program.into();
        let specs = profile
            .hosts
            .iter()
            .map(|host| {
                HostProcessSpec::deny_all_deno(
                    host.host_id.clone(),
                    deno_program.clone(),
                    &package,
                    restart_policy.clone(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        Self::spawn_verified(profile, specs, &package)
    }

    pub fn spawn_verified(
        profile: OrchestratorProfile,
        specs: Vec<HostProcessSpec>,
        package: &VerifiedAgentPackage,
    ) -> Result<Self, HostRuntimeError> {
        let required_tier = profile
            .hosts
            .iter()
            .map(|host| host.max_tier)
            .max()
            .ok_or(HostRuntimeError::EmptyHostCatalog)?;
        if required_tier > package.maximum_tier() {
            return Err(HostRuntimeError::PackageTierExceeded {
                key_id: package.key_id().to_string(),
                required: required_tier,
                maximum: package.maximum_tier(),
            });
        }
        Self::spawn_unverified(profile, specs)
    }

    fn spawn_unverified(
        profile: OrchestratorProfile,
        specs: Vec<HostProcessSpec>,
    ) -> Result<Self, HostRuntimeError> {
        profile.validate()?;
        let profile_host_ids = profile
            .hosts
            .iter()
            .map(|host| host.host_id.clone())
            .collect::<BTreeSet<_>>();
        let mut specs_by_host = BTreeMap::new();
        for spec in specs {
            validate_label("host_id", &spec.host_id)?;
            if !profile_host_ids.contains(&spec.host_id) {
                return Err(HostRuntimeError::UnknownProcessHost(spec.host_id));
            }
            let host_id = spec.host_id.clone();
            if specs_by_host.insert(host_id.clone(), spec).is_some() {
                return Err(HostRuntimeError::DuplicateProcessHost(host_id));
            }
        }

        let missing = profile_host_ids
            .iter()
            .filter(|host_id| !specs_by_host.contains_key(*host_id))
            .cloned()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(HostRuntimeError::MissingProcessHosts(missing));
        }

        let mut hosts = BTreeMap::new();
        let mut tool_owners = BTreeMap::new();
        let mut allowed_tool_counts = BTreeMap::new();
        for host in profile.hosts {
            for tool_id in &host.allowed_tools {
                tool_owners.insert(tool_id.clone(), host.host_id.clone());
            }
            allowed_tool_counts.insert(host.host_id.clone(), host.allowed_tools.len());
            let spec = specs_by_host
                .remove(&host.host_id)
                .expect("profile coverage was validated before process launch");
            let pool = StdioProcessPool::spawn(
                spec.command,
                StdioProcessPoolOptions {
                    worker_count: 1,
                    limits: ExecutorLimits::default(),
                    default_job_timeout: Some(Duration::from_secs(30)),
                    restart_policy: spec.restart_policy,
                },
            )
            .map_err(|error| HostRuntimeError::ProcessIo {
                host_id: host.host_id.clone(),
                message: error.to_string(),
            })?;
            hosts.insert(host.host_id, pool);
        }

        Ok(Self {
            profile_id: profile.profile_id,
            tool_owners,
            allowed_tool_counts,
            hosts,
        })
    }

    pub fn profile_id(&self) -> &str {
        &self.profile_id
    }

    pub fn submit(&self, request: HostRpcRequest) -> Result<(), HostRuntimeError> {
        validate_label("call_id", &request.call_id)?;
        validate_tool_id(&request.tool_id).map_err(HostRuntimeError::ToolApi)?;
        parse_json(&request.arguments_json)
            .map_err(|error| HostRuntimeError::InvalidRpcArguments(error.message.to_string()))?;
        let host_id = self
            .tool_owners
            .get(&request.tool_id)
            .ok_or_else(|| HostRuntimeError::ToolNotAllowed(request.tool_id.clone()))?;
        self.hosts
            .get(host_id)
            .expect("supervised runtime must retain every profile host")
            .submit(JobRequest::new(request.call_id.clone(), request))
            .map_err(|error| HostRuntimeError::ProcessSubmit {
                host_id: host_id.clone(),
                message: error.to_string(),
            })
    }

    pub fn recv_for_host(
        &self,
        host_id: &str,
        timeout: Duration,
    ) -> Result<Option<JobResponse<HostRpcResponse>>, HostRuntimeError> {
        let host = self
            .hosts
            .get(host_id)
            .ok_or_else(|| HostRuntimeError::UnknownProcessHost(host_id.to_string()))?;
        host.recv_response_timeout(timeout)
            .map_err(|error| HostRuntimeError::ProcessIo {
                host_id: host_id.to_string(),
                message: error.to_string(),
            })
    }

    pub fn snapshots(&self) -> Vec<SupervisedHostSnapshot> {
        self.hosts
            .iter()
            .map(|(host_id, process)| SupervisedHostSnapshot {
                host_id: host_id.clone(),
                allowed_tool_count: self.allowed_tool_counts[host_id],
                process: process.snapshot(),
            })
            .collect()
    }

    pub fn shutdown(&self) {
        for host in self.hosts.values() {
            host.shutdown();
        }
    }
}

impl ActiveOrchestratorRuntime {
    pub fn summary(&self) -> OrchestratorProfileSummary {
        OrchestratorProfileSummary {
            profile_id: self.profile_id.clone(),
            host_count: self.hosts.len(),
            allowed_tool_count: self.tool_owners.len(),
            registered_tool_count: self
                .hosts
                .values()
                .map(|host| host.registered_tools.len())
                .sum(),
            active: true,
        }
    }

    pub fn host_for_tool(&self, tool_id: &str) -> Option<&str> {
        self.tool_owners.get(tool_id).map(String::as_str)
    }

    pub fn invoke_with_events(
        &self,
        request: &ToolInvocationRequest,
    ) -> Result<ToolExecutionTrace, HostRuntimeError> {
        let host_id = self
            .tool_owners
            .get(&request.tool_id)
            .ok_or_else(|| HostRuntimeError::ToolNotAllowed(request.tool_id.clone()))?;
        Ok(self
            .hosts
            .get(host_id)
            .expect("active orchestrator must retain each tool owner")
            .invoke_with_events(request))
    }

    /// Dispatch one subprocess-originated call through the canonical D18D
    /// handler runtime owned by the tool's profile host.
    pub fn handle_rpc(&self, request: HostRpcRequest) -> Result<HostRpcResponse, HostRuntimeError> {
        let host_id = self
            .tool_owners
            .get(&request.tool_id)
            .ok_or_else(|| HostRuntimeError::ToolNotAllowed(request.tool_id.clone()))?;
        self.hosts
            .get(host_id)
            .expect("active orchestrator must retain each tool owner")
            .handle_rpc(request)
    }
}

impl ActiveHostToolRuntime {
    pub fn profile(&self) -> &HostProfile {
        &self.profile
    }

    pub fn summary(&self) -> HostProfileSummary {
        HostProfileSummary {
            profile_id: self.profile.profile_id.clone(),
            host_id: self.profile.host_id.clone(),
            max_tier: self.profile.max_tier,
            allowed_tool_count: self.profile.allowed_tools.len(),
            registered_tool_count: self.registered_tools.len(),
            capability_count: self.profile.capabilities.len(),
            active: true,
        }
    }

    pub fn invoke_with_events(&self, request: &ToolInvocationRequest) -> ToolExecutionTrace {
        self.runtime.invoke_with_events(request)
    }

    pub fn definitions(&self) -> Vec<&ToolDefinition> {
        self.runtime.list()
    }

    /// Convert an untrusted subprocess RPC frame into the repository-owned
    /// invocation contract, then execute only through registered handlers.
    pub fn handle_rpc(&self, request: HostRpcRequest) -> Result<HostRpcResponse, HostRuntimeError> {
        validate_label("call_id", &request.call_id)?;
        validate_tool_id(&request.tool_id).map_err(HostRuntimeError::ToolApi)?;
        if !self.profile.allows_tool(&request.tool_id) {
            return Err(HostRuntimeError::ToolNotAllowed(request.tool_id));
        }
        let arguments = parse_json(&request.arguments_json)
            .map_err(|error| HostRuntimeError::InvalidRpcArguments(error.message.to_string()))?;
        let invocation = ToolInvocationRequest {
            call_id: request.call_id,
            tool_id: request.tool_id,
            arguments,
            requested_by: RequestedBy::Agent,
            session_id: None,
            job_id: None,
            agent_id: Some(self.profile.host_id.clone()),
            user_id: None,
            requested_at: 0,
            deadline_at: None,
            idempotency_key: None,
        };
        let trace = self.runtime.invoke_with_events(&invocation);
        if let Some(error) = trace.result.error {
            return Err(HostRuntimeError::ToolExecution {
                tool_id: invocation.tool_id,
                kind: error.kind.to_string(),
                message: error.message,
            });
        }
        let output = trace.result.output.unwrap_or(JsonValue::Null);
        let output_json = serialize_json(&output)
            .map_err(|error| HostRuntimeError::InvalidRpcOutput(error.message))?;
        Ok(HostRpcResponse { output_json })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostRuntimeError {
    InvalidJson(String),
    MissingField(String),
    InvalidField {
        field: String,
        message: String,
    },
    EmptyToolCatalog,
    EmptyHostCatalog,
    DuplicateHost(String),
    DuplicateTool(String),
    DuplicateToolOwner {
        tool_id: String,
        first_host: String,
        second_host: String,
    },
    DuplicateCapability(String),
    ToolNotAllowed(String),
    PrivilegeCeilingExceeded {
        tool_id: String,
        required: PrivilegeTier,
        maximum: PrivilegeTier,
    },
    MissingCapability {
        tool_id: String,
        capability: String,
    },
    CatalogIncomplete(Vec<String>),
    DuplicateProcessHost(String),
    MissingProcessHosts(Vec<String>),
    UnknownProcessHost(String),
    InvalidRpcArguments(String),
    InvalidRpcOutput(String),
    ToolExecution {
        tool_id: String,
        kind: String,
        message: String,
    },
    ProcessIo {
        host_id: String,
        message: String,
    },
    ProcessSubmit {
        host_id: String,
        message: String,
    },
    PackageTierExceeded {
        key_id: String,
        required: PrivilegeTier,
        maximum: PrivilegeTier,
    },
    PackageVerification(String),
    MissingDenoEntrypoint(std::path::PathBuf),
    InvalidDenoEntrypoint(std::path::PathBuf),
    ToolApi(ToolApiError),
}

impl Display for HostRuntimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(message) => write!(f, "invalid host profile JSON: {message}"),
            Self::MissingField(field) => write!(f, "host profile is missing '{field}'"),
            Self::InvalidField { field, message } => {
                write!(f, "invalid host profile field '{field}': {message}")
            }
            Self::EmptyToolCatalog => f.write_str("host profile tool catalog cannot be empty"),
            Self::EmptyHostCatalog => {
                f.write_str("orchestrator profile host catalog cannot be empty")
            }
            Self::DuplicateHost(host_id) => write!(f, "duplicate profile host '{host_id}'"),
            Self::DuplicateTool(tool_id) => write!(f, "duplicate profile tool '{tool_id}'"),
            Self::DuplicateToolOwner {
                tool_id,
                first_host,
                second_host,
            } => write!(
                f,
                "tool '{tool_id}' is owned by both '{first_host}' and '{second_host}'"
            ),
            Self::DuplicateCapability(capability) => {
                write!(f, "duplicate profile capability '{capability}'")
            }
            Self::ToolNotAllowed(tool_id) => {
                write!(f, "tool '{tool_id}' is not allowlisted by the host profile")
            }
            Self::PrivilegeCeilingExceeded {
                tool_id,
                required,
                maximum,
            } => write!(
                f,
                "tool '{tool_id}' requires {required}, above host ceiling {maximum}"
            ),
            Self::MissingCapability {
                tool_id,
                capability,
            } => write!(
                f,
                "tool '{tool_id}' requires undeclared host capability '{capability}'"
            ),
            Self::CatalogIncomplete(tool_ids) => write!(
                f,
                "host profile catalog is incomplete; missing {}",
                tool_ids.join(", ")
            ),
            Self::DuplicateProcessHost(host_id) => {
                write!(f, "duplicate process specification for host '{host_id}'")
            }
            Self::MissingProcessHosts(host_ids) => write!(
                f,
                "orchestrator process catalog is incomplete; missing {}",
                host_ids.join(", ")
            ),
            Self::UnknownProcessHost(host_id) => {
                write!(f, "process host '{host_id}' is not declared by the profile")
            }
            Self::InvalidRpcArguments(message) => {
                write!(f, "invalid host RPC arguments JSON: {message}")
            }
            Self::InvalidRpcOutput(message) => {
                write!(f, "host RPC output could not be serialized: {message}")
            }
            Self::ToolExecution {
                tool_id,
                kind,
                message,
            } => write!(f, "host handler '{tool_id}' failed with {kind}: {message}"),
            Self::ProcessIo { host_id, message } => {
                write!(f, "host '{host_id}' process I/O failed: {message}")
            }
            Self::ProcessSubmit { host_id, message } => {
                write!(f, "host '{host_id}' rejected RPC submission: {message}")
            }
            Self::PackageTierExceeded {
                key_id,
                required,
                maximum,
            } => write!(
                f,
                "package key '{key_id}' permits at most {maximum}, below profile requirement {required}"
            ),
            Self::PackageVerification(message) => {
                write!(f, "agent package failed launch-time verification: {message}")
            }
            Self::MissingDenoEntrypoint(path) => {
                write!(f, "signed package is missing Deno entrypoint '{}'", path.display())
            }
            Self::InvalidDenoEntrypoint(path) => {
                write!(f, "Deno entrypoint path is not UTF-8: '{}'", path.display())
            }
            Self::ToolApi(error) => Display::fmt(error, f),
        }
    }
}

impl Error for HostRuntimeError {}

fn expect_object<'a>(
    value: &'a JsonValue,
    field: &str,
) -> Result<&'a [(String, JsonValue)], HostRuntimeError> {
    match value {
        JsonValue::Object(fields) => Ok(fields),
        _ => Err(HostRuntimeError::InvalidField {
            field: field.to_string(),
            message: "expected object".to_string(),
        }),
    }
}

fn required_value<'a>(
    object: &'a [(String, JsonValue)],
    field: &str,
) -> Result<&'a JsonValue, HostRuntimeError> {
    object
        .iter()
        .find_map(|(name, value)| (name == field).then_some(value))
        .ok_or_else(|| HostRuntimeError::MissingField(field.to_string()))
}

fn required_string(
    object: &[(String, JsonValue)],
    field: &str,
) -> Result<String, HostRuntimeError> {
    match required_value(object, field)? {
        JsonValue::String(value) => Ok(value.clone()),
        _ => Err(HostRuntimeError::InvalidField {
            field: field.to_string(),
            message: "expected string".to_string(),
        }),
    }
}

fn required_string_array(
    object: &[(String, JsonValue)],
    field: &str,
) -> Result<Vec<String>, HostRuntimeError> {
    let JsonValue::Array(values) = required_value(object, field)? else {
        return Err(HostRuntimeError::InvalidField {
            field: field.to_string(),
            message: "expected array".to_string(),
        });
    };
    values
        .iter()
        .map(|value| match value {
            JsonValue::String(value) => Ok(value.clone()),
            _ => Err(HostRuntimeError::InvalidField {
                field: field.to_string(),
                message: "expected an array of strings".to_string(),
            }),
        })
        .collect()
}

fn parse_tier(value: &str) -> Result<PrivilegeTier, HostRuntimeError> {
    match value {
        "tier0" => Ok(PrivilegeTier::Tier0),
        "tier1" => Ok(PrivilegeTier::Tier1),
        "tier2" => Ok(PrivilegeTier::Tier2),
        "tier3" => Ok(PrivilegeTier::Tier3),
        _ => Err(HostRuntimeError::InvalidField {
            field: "max_tier".to_string(),
            message: "expected tier0, tier1, tier2, or tier3".to_string(),
        }),
    }
}

fn validate_label(field: &str, value: &str) -> Result<(), HostRuntimeError> {
    if value.is_empty()
        || value.chars().any(|character| {
            !(character.is_ascii_alphanumeric() || character == '_' || character == '-')
        })
    {
        return Err(HostRuntimeError::InvalidField {
            field: field.to_string(),
            message: "expected a non-empty ASCII label".to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chief_of_staff_tool_api::{
        JsonSchema, RequestedBy, ToolConcurrency, ToolHandlerOutput, ToolIdempotency,
        ToolInvocationRequest, ToolSideEffects, ToolStability, ToolStreaming,
    };
    use coding_adventures_ed25519::{generate_keypair, sign};
    use generic_job_protocol::JobResult;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_PACKAGE_ID: AtomicU64 = AtomicU64::new(0);

    const PROFILE: &str = r#"{
      "profile_id": "test_profile",
      "host_id": "test_host",
      "max_tier": "tier1",
      "allowed_tools": ["demo.read"],
      "capabilities": ["demo_read"]
    }"#;

    const ORCHESTRATOR_PROFILE: &str = r#"{
      "profile_id": "demo_orchestrator",
      "hosts": [
        {
          "host_id": "reader_host",
          "max_tier": "tier1",
          "allowed_tools": ["demo.read"],
          "capabilities": ["demo_read"]
        },
        {
          "host_id": "writer_host",
          "max_tier": "tier1",
          "allowed_tools": ["demo.write"],
          "capabilities": ["demo_write"]
        }
      ]
    }"#;

    fn definition(tool_id: &str, tier: PrivilegeTier, capabilities: &[&str]) -> ToolDefinition {
        ToolDefinition {
            tool_id: tool_id.to_string(),
            display_name: "Demo".to_string(),
            description: "Demo tool".to_string(),
            input_schema: JsonSchema::Object {
                properties: vec![],
                required: vec![],
                allow_unknown_fields: false,
            },
            output_schema: Some(JsonSchema::String),
            side_effects: ToolSideEffects::Read,
            idempotency: ToolIdempotency::Always,
            concurrency: ToolConcurrency::Safe,
            streaming: ToolStreaming::None,
            required_tier: tier,
            required_capabilities: capabilities.iter().map(|value| value.to_string()).collect(),
            preferred_lock_scope: None,
            timeout_seconds: Some(5),
            tags: vec!["demo".to_string()],
            stability: ToolStability::Stable,
        }
    }

    fn request(tool_id: &str) -> ToolInvocationRequest {
        ToolInvocationRequest {
            call_id: "call_1".to_string(),
            tool_id: tool_id.to_string(),
            arguments: JsonValue::Object(vec![]),
            requested_by: RequestedBy::Agent,
            session_id: Some("session_1".to_string()),
            job_id: Some("job_1".to_string()),
            agent_id: Some("agent_1".to_string()),
            user_id: Some("user_1".to_string()),
            requested_at: 100,
            deadline_at: Some(200),
            idempotency_key: Some("idem_1".to_string()),
        }
    }

    fn scripted_worker(script: &str) -> Option<StdioWorkerCommand> {
        let candidates = if cfg!(windows) {
            vec!["python"]
        } else {
            vec!["python3", "python"]
        };
        candidates.into_iter().find_map(|program| {
            Command::new(program)
                .arg("-c")
                .arg("import json, sys")
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|_| {
                    StdioWorkerCommand::new(
                        program.to_string(),
                        vec!["-c".to_string(), script.to_string()],
                    )
                })
        })
    }

    fn signed_deno_package() -> (std::path::PathBuf, PackageKeyring) {
        let path = std::env::temp_dir().join(format!(
            "chief-deno-package-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_PACKAGE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(path.join("code")).unwrap();
        std::fs::write(path.join("manifest.json"), b"{\"runtime\":\"typescript\"}").unwrap();
        DenoLaunchPlan::write_launch_script(&path).unwrap();
        std::fs::write(path.join("PUBKEY_ID"), b"dev-deno").unwrap();
        std::fs::write(
            path.join("code/agent_runtime.ts"),
            r#"
const encoder = new TextEncoder();
const decoder = new TextDecoder();
const reader = Deno.stdin.readable.getReader();
const writer = Deno.stdout.writable.getWriter();
let pending = "";

async function deniedChecks() {
  const denied = { env: false, read: false, write: false, run: false, net: false };
  try { Deno.env.get("HOME"); } catch { denied.env = true; }
  try { await Deno.readTextFile("/etc/hosts"); } catch { denied.read = true; }
  try { await Deno.writeTextFile("chief-denied.txt", "no"); } catch { denied.write = true; }
  try { new Deno.Command("echo").outputSync(); } catch { denied.run = true; }
  try { await fetch("http://127.0.0.1:9"); } catch { denied.net = true; }
  return denied;
}

async function respond(line) {
  const frame = JSON.parse(line);
  const body = frame.body;
  const payload = { output_json: JSON.stringify(await deniedChecks()) };
  const response = { version: 1, kind: "response", body: { id: body.id, result: { status: "ok", payload }, metadata: body.metadata } };
  await writer.write(encoder.encode(JSON.stringify(response) + "\n"));
}

while (true) {
  const { value, done } = await reader.read();
  if (done) break;
  pending += decoder.decode(value, { stream: true });
  while (pending.includes("\n")) {
    const index = pending.indexOf("\n");
    const line = pending.slice(0, index);
    pending = pending.slice(index + 1);
    if (line) await respond(line);
  }
}
"#,
        )
        .unwrap();
        let (public_key, secret_key) = generate_keypair(&[23; 32]);
        let (digest, _) = crate::package::hash_package_contents(&path).unwrap();
        std::fs::write(path.join("SIGNATURE"), sign(&digest, &secret_key)).unwrap();
        let mut keyring = PackageKeyring::new();
        keyring
            .trust(
                TrustedPackageKey::new(
                    "dev-deno",
                    PackageKeyType::Developer,
                    public_key,
                    PrivilegeTier::Tier1,
                )
                .unwrap(),
            )
            .unwrap();
        (path, keyring)
    }

    #[test]
    fn profile_activates_complete_allowlisted_catalog_and_invokes() {
        let mut runtime = HostProfileRuntime::from_json(PROFILE).unwrap();
        runtime
            .register_handler(
                definition("demo.read", PrivilegeTier::Tier1, &["demo_read"]),
                |_arguments, _context| {
                    Ok(ToolHandlerOutput::new(JsonValue::String("ok".to_string())))
                },
            )
            .unwrap();

        let active = runtime.activate().unwrap();
        assert!(active.summary().active);
        assert_eq!(active.summary().registered_tool_count, 1);
        assert_eq!(
            active
                .invoke_with_events(&request("demo.read"))
                .result
                .output,
            Some(JsonValue::String("ok".to_string()))
        );
    }

    #[test]
    fn active_host_dispatches_rpc_through_profile_gated_handler() {
        let mut runtime = HostProfileRuntime::from_json(PROFILE).unwrap();
        runtime
            .register_handler(
                definition("demo.read", PrivilegeTier::Tier1, &["demo_read"]),
                |arguments, context: chief_of_staff_tool_api::ToolExecutionContext| {
                    assert_eq!(context.requested_by, RequestedBy::Agent);
                    assert_eq!(context.agent_id.as_deref(), Some("test_host"));
                    assert_eq!(arguments, JsonValue::Object(vec![]));
                    Ok(ToolHandlerOutput::new(JsonValue::String(
                        "rust".to_string(),
                    )))
                },
            )
            .unwrap();
        let active = runtime.activate().unwrap();

        let response = active
            .handle_rpc(HostRpcRequest {
                call_id: "rpc_1".to_string(),
                tool_id: "demo.read".to_string(),
                arguments_json: "{}".to_string(),
            })
            .unwrap();

        assert_eq!(
            parse_json(&response.output_json).unwrap(),
            JsonValue::String("rust".to_string())
        );
    }

    #[test]
    fn active_host_rejects_rpc_before_unregistered_handler_execution() {
        let mut runtime = HostProfileRuntime::from_json(PROFILE).unwrap();
        runtime
            .register_handler(
                definition("demo.read", PrivilegeTier::Tier1, &["demo_read"]),
                |_arguments, _context| {
                    Ok(ToolHandlerOutput::new(JsonValue::String("ok".to_string())))
                },
            )
            .unwrap();
        let active = runtime.activate().unwrap();

        assert!(matches!(
            active.handle_rpc(HostRpcRequest {
                call_id: "rpc_2".to_string(),
                tool_id: "demo.write".to_string(),
                arguments_json: "{}".to_string(),
            }),
            Err(HostRuntimeError::ToolNotAllowed(tool_id)) if tool_id == "demo.write"
        ));
    }

    #[test]
    fn deno_process_arguments_match_signed_launch_plan() {
        let (package_path, keyring) = signed_deno_package();
        let package = verify_agent_package(&package_path, &keyring).unwrap();
        let spec = HostProcessSpec::deny_all_deno(
            "deno_host",
            "deno",
            &package,
            StdioWorkerRestartPolicy::Never,
        )
        .unwrap();
        let expected = DenoLaunchPlan::launch_script()
            .lines()
            .nth(1)
            .unwrap()
            .strip_prefix("exec deno ")
            .unwrap()
            .split_whitespace()
            .map(str::to_string)
            .collect::<Vec<_>>();
        assert_eq!(
            spec.command.args[..spec.command.args.len() - 1],
            expected[..expected.len() - 1]
        );
        assert!(spec
            .command
            .args
            .last()
            .unwrap()
            .ends_with(expected.last().unwrap()));
        std::fs::remove_dir_all(package_path).unwrap();
    }

    #[test]
    fn profile_rejects_unlisted_tools_privilege_escalation_and_capability_drift() {
        let mut runtime = HostProfileRuntime::from_json(PROFILE).unwrap();
        assert!(matches!(
            runtime.register_handler(
                definition("demo.write", PrivilegeTier::Tier1, &["demo_read"]),
                |_arguments, _context| Ok(ToolHandlerOutput::new(JsonValue::Null)),
            ),
            Err(HostRuntimeError::ToolNotAllowed(_))
        ));
        assert!(matches!(
            runtime.register_handler(
                definition("demo.read", PrivilegeTier::Tier2, &["demo_read"]),
                |_arguments, _context| Ok(ToolHandlerOutput::new(JsonValue::Null)),
            ),
            Err(HostRuntimeError::PrivilegeCeilingExceeded { .. })
        ));
        assert!(matches!(
            runtime.register_handler(
                definition("demo.read", PrivilegeTier::Tier1, &["undeclared"]),
                |_arguments, _context| Ok(ToolHandlerOutput::new(JsonValue::Null)),
            ),
            Err(HostRuntimeError::MissingCapability { .. })
        ));
    }

    #[test]
    fn profile_cannot_activate_with_missing_catalog_entries() {
        let runtime = HostProfileRuntime::from_json(PROFILE).unwrap();
        assert!(matches!(
            runtime.activate(),
            Err(HostRuntimeError::CatalogIncomplete(tool_ids))
                if tool_ids == vec!["demo.read".to_string()]
        ));
    }

    #[test]
    fn orchestrator_profile_routes_tools_to_isolated_owners() {
        let mut runtime = OrchestratorProfileRuntime::from_json(ORCHESTRATOR_PROFILE).unwrap();
        runtime
            .register_handler(
                definition("demo.read", PrivilegeTier::Tier1, &["demo_read"]),
                |_arguments, _context| {
                    Ok(ToolHandlerOutput::new(JsonValue::String(
                        "read".to_string(),
                    )))
                },
            )
            .unwrap();
        runtime
            .register_handler(
                definition("demo.write", PrivilegeTier::Tier1, &["demo_write"]),
                |_arguments, _context| {
                    Ok(ToolHandlerOutput::new(JsonValue::String(
                        "write".to_string(),
                    )))
                },
            )
            .unwrap();

        let active = runtime.activate().unwrap();
        assert_eq!(active.summary().host_count, 2);
        assert_eq!(active.host_for_tool("demo.read"), Some("reader_host"));
        assert_eq!(active.host_for_tool("demo.write"), Some("writer_host"));
        assert_eq!(
            active
                .invoke_with_events(&request("demo.read"))
                .unwrap()
                .result
                .output,
            Some(JsonValue::String("read".to_string()))
        );
    }

    #[test]
    fn orchestrator_profile_rejects_cross_host_tool_ownership() {
        let profile = ORCHESTRATOR_PROFILE.replace("demo.write", "demo.read");
        assert!(matches!(
            OrchestratorProfile::from_json(&profile),
            Err(HostRuntimeError::DuplicateToolOwner { .. })
        ));
    }

    #[test]
    fn supervised_profile_requires_exact_process_coverage() {
        let profile = OrchestratorProfile {
            profile_id: "test_profile".to_string(),
            hosts: vec![HostProfile::from_json(PROFILE).unwrap()],
        };
        assert!(matches!(
            SupervisedOrchestratorRuntime::spawn_unverified(profile, vec![]),
            Err(HostRuntimeError::MissingProcessHosts(host_ids))
                if host_ids == vec!["test_host".to_string()]
        ));
    }

    #[test]
    fn supervised_profile_routes_rpc_and_recovers_crashed_host() {
        let marker = std::env::temp_dir().join(format!(
            "chief-host-runtime-restart-once-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
        ));
        let marker_literal = format!("{:?}", marker.to_string_lossy());
        let script = format!(
            r#"
import json, os, sys
marker_path = {marker_literal}
for line in sys.stdin:
    frame = json.loads(line)
    body = frame["body"]
    payload = body["payload"]
    if not os.path.exists(marker_path):
        open(marker_path, "w").close()
        sys.exit(1)
    out = {{"output_json": json.dumps({{"host": "test_host", "tool": payload["tool_id"]}})}}
    response = {{"version": 1, "kind": "response", "body": {{"id": body["id"], "result": {{"status": "ok", "payload": out}}, "metadata": body["metadata"]}}}}
    print(json.dumps(response), flush=True)
"#
        );
        let Some(command) = scripted_worker(&script) else {
            eprintln!("skipping test because no Python interpreter was found");
            return;
        };
        let profile = OrchestratorProfile {
            profile_id: "test_profile".to_string(),
            hosts: vec![HostProfile::from_json(PROFILE).unwrap()],
        };
        let package = VerifiedAgentPackage {
            path: std::env::temp_dir(),
            digest: [0; 32],
            key_id: "dev-1".to_string(),
            key_type: PackageKeyType::Developer,
            maximum_tier: PrivilegeTier::Tier1,
        };
        let runtime = SupervisedOrchestratorRuntime::spawn_verified(
            profile,
            vec![HostProcessSpec::new(
                "test_host",
                command,
                StdioWorkerRestartPolicy::Bounded {
                    max_restarts: 1,
                    window: Duration::from_secs(60),
                },
            )],
            &package,
        )
        .unwrap();

        runtime
            .submit(HostRpcRequest {
                call_id: "call_1".to_string(),
                tool_id: "demo.read".to_string(),
                arguments_json: "{}".to_string(),
            })
            .unwrap();
        let first = runtime
            .recv_for_host("test_host", Duration::from_secs(5))
            .unwrap()
            .expect("crashed host should fail its in-flight RPC");
        assert!(matches!(first.result, JobResult::Error { .. }));

        runtime
            .submit(HostRpcRequest {
                call_id: "call_2".to_string(),
                tool_id: "demo.read".to_string(),
                arguments_json: "{}".to_string(),
            })
            .unwrap();
        let second = runtime
            .recv_for_host("test_host", Duration::from_secs(5))
            .unwrap()
            .expect("restarted host should answer the next RPC");
        match second.result {
            JobResult::Ok { payload } => {
                assert!(payload.output_json.contains("test_host"));
                assert!(payload.output_json.contains("demo.read"));
            }
            other => panic!("expected restarted host success, got {other:?}"),
        }
        assert_eq!(runtime.snapshots()[0].process.live_workers, 1);
        runtime.shutdown();
        assert!(runtime.snapshots()[0].process.shutting_down);
        let _ = std::fs::remove_file(marker);
    }

    #[test]
    fn verified_package_key_tier_gates_process_launch() {
        let profile = OrchestratorProfile {
            profile_id: "test_profile".to_string(),
            hosts: vec![HostProfile {
                profile_id: "test_profile".to_string(),
                host_id: "test_host".to_string(),
                max_tier: PrivilegeTier::Tier2,
                allowed_tools: vec!["demo.read".to_string()],
                capabilities: vec!["demo_read".to_string()],
            }],
        };
        let package = VerifiedAgentPackage {
            path: std::env::temp_dir(),
            digest: [0; 32],
            key_id: "dev-1".to_string(),
            key_type: PackageKeyType::Developer,
            maximum_tier: PrivilegeTier::Tier1,
        };
        assert!(matches!(
            SupervisedOrchestratorRuntime::spawn_verified(profile, vec![], &package),
            Err(HostRuntimeError::PackageTierExceeded {
                required: PrivilegeTier::Tier2,
                maximum: PrivilegeTier::Tier1,
                ..
            })
        ));
    }

    #[test]
    fn signed_package_launches_deny_all_deno_rpc_worker() {
        if !Command::new("deno")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
        {
            eprintln!("skipping test because Deno is unavailable");
            return;
        }
        let (package_path, keyring) = signed_deno_package();
        let profile = OrchestratorProfile {
            profile_id: "deno_profile".to_string(),
            hosts: vec![HostProfile {
                profile_id: "deno_profile".to_string(),
                host_id: "deno_host".to_string(),
                max_tier: PrivilegeTier::Tier1,
                allowed_tools: vec!["demo.read".to_string()],
                capabilities: vec!["demo_read".to_string()],
            }],
        };
        let runtime = SupervisedOrchestratorRuntime::spawn_deno_verified(
            profile,
            &package_path,
            &keyring,
            "deno",
            StdioWorkerRestartPolicy::Never,
        )
        .unwrap();
        runtime
            .submit(HostRpcRequest {
                call_id: "deno_call".to_string(),
                tool_id: "demo.read".to_string(),
                arguments_json: "{}".to_string(),
            })
            .unwrap();
        let response = runtime
            .recv_for_host("deno_host", Duration::from_secs(10))
            .unwrap()
            .expect("Deno worker response");
        match response.result {
            JobResult::Ok { payload } => {
                for denied in ["env", "read", "write", "run", "net"] {
                    assert!(payload.output_json.contains(&format!("\"{denied}\":true")));
                }
            }
            other => panic!("expected deny-all Deno response, got {other:?}"),
        }
        runtime.shutdown();
        std::fs::remove_dir_all(package_path).unwrap();
    }

    #[test]
    fn deno_launch_reverifies_package_after_tampering() {
        let (package_path, keyring) = signed_deno_package();
        std::fs::write(
            package_path.join("code/agent_runtime.ts"),
            b"console.log('tampered')",
        )
        .unwrap();
        let profile = OrchestratorProfile {
            profile_id: "deno_profile".to_string(),
            hosts: vec![HostProfile::from_json(PROFILE).unwrap()],
        };
        assert!(matches!(
            SupervisedOrchestratorRuntime::spawn_deno_verified(
                profile,
                &package_path,
                &keyring,
                "deno",
                StdioWorkerRestartPolicy::Never,
            ),
            Err(HostRuntimeError::PackageVerification(_))
        ));
        std::fs::remove_dir_all(package_path).unwrap();
    }
}
