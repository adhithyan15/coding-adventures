//! Profile-gated D18D tool runtime for Chief of Staff hosts.

#![forbid(unsafe_code)]

use chief_of_staff_tool_api::{
    validate_tool_id, InMemoryToolRuntime, PrivilegeTier, ToolApiError, ToolDefinition,
    ToolExecutionTrace, ToolHandler, ToolInvocationRequest,
};
use coding_adventures_json_value::{parse as parse_json, JsonValue};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

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
}
