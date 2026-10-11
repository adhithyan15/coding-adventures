//! Declarative operator command core for the authenticated D18 Chief daemon.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_channel_endpoints::AgentId;
use chief_of_staff_daemon_api::{DaemonClient, DaemonClientError};
use chief_of_staff_host_control_protocol::{
    ChannelBinding, ChannelBindingAccess, LaunchBindings, LevelOneModelBinding,
    MAX_LAUNCH_CHANNEL_BINDINGS,
};
use chief_of_staff_pipeline_bindings::{HostPipelineBinding, PipelineId};
use chief_of_staff_service_registry::{
    DesiredState, HostName, HostRegistration, PackagePath, RegistryError, RestartPolicy,
};
use chief_of_staff_vault_runtime::{AllowedAgents, SecretPolicy, VaultDeliveryMode};
use chief_of_staff_vault_secret_store::{
    validate_destination, validate_policy, NameError, SecretName, MAX_PRIVILEGE_TIER,
};
use cli_builder::{load_spec_from_str, CliBuilderError, ParseResult, Parser, ParserOutput};
use coding_adventures_json_serializer::{serialize_pretty, JsonSerializerError, SerializerConfig};
use coding_adventures_json_value::JsonValue;
use core::fmt::{self, Display, Formatter};
use std::collections::BTreeSet;

const CLI_SPEC: &str = r#"{
  "cli_builder_spec_version": "1.0",
  "name": "chief-of-staff",
  "description": "Operate the local D18 Chief daemon.",
  "version": "0.1.0",
  "commands": [
    {
      "id": "agents",
      "name": "agents",
      "description": "List registered host agents."
    },
    {
      "id": "install_daemon",
      "name": "install-daemon",
      "description": "Install and start the current-user Chief daemon."
    },
    {
      "id": "doctor",
      "name": "doctor",
      "description": "Inspect durable and authoritative host health.",
      "arguments": [
        {"id": "host", "name": "HOST", "description": "Registered host name.", "type": "string", "required": true}
      ]
    },
    {
      "id": "register",
      "name": "register",
      "description": "Register immutable host package identity.",
      "arguments": [
        {"id": "host", "name": "HOST", "description": "Stable host name.", "type": "string", "required": true},
        {"id": "package_path", "name": "PACKAGE_PATH", "description": "Portable package path.", "type": "string", "required": true},
        {"id": "package_hash", "name": "PACKAGE_HASH", "description": "Lowercase SHA-256 package hash.", "type": "string", "required": true}
      ],
      "flags": [
        {"id": "restart", "long": "restart", "description": "Durable restart policy.", "type": "enum", "enum_values": ["always", "on_failure", "never"], "default": "never", "value_name": "POLICY"},
        {"id": "state", "long": "state", "description": "Initial desired state.", "type": "enum", "enum_values": ["running", "stopped"], "default": "stopped", "value_name": "STATE"}
      ]
    },
    {
      "id": "start",
      "name": "start",
      "description": "Set one host's desired state to running.",
      "arguments": [
        {"id": "host", "name": "HOST", "description": "Registered host name.", "type": "string", "required": true}
      ]
    },
    {
      "id": "stop",
      "name": "stop",
      "description": "Set one host's desired state to stopped.",
      "arguments": [
        {"id": "host", "name": "HOST", "description": "Registered host name.", "type": "string", "required": true}
      ]
    },
    {
      "id": "reconcile",
      "name": "reconcile",
      "description": "Run one bounded host reconciliation tick."
    },
    {
      "id": "deregister",
      "name": "deregister",
      "description": "Remove stopped, inactive host intent.",
      "arguments": [
        {"id": "host", "name": "HOST", "description": "Registered host name.", "type": "string", "required": true}
      ]
    },
    {
      "id": "wire",
      "name": "wire",
      "description": "Authorize and persist one exact host pipeline binding.",
      "arguments": [
        {"id": "host", "name": "HOST", "description": "Registered host name.", "type": "string", "required": true},
        {"id": "package_path", "name": "PACKAGE_PATH", "description": "Exact registered package path.", "type": "string", "required": true},
        {"id": "package_hash", "name": "PACKAGE_HASH", "description": "Exact lowercase SHA-256 package hash.", "type": "string", "required": true},
        {"id": "pipeline_id", "name": "PIPELINE_ID", "description": "Canonical lowercase dashed UUID-v7.", "type": "string", "required": true},
        {"id": "agent_id", "name": "AGENT_ID", "description": "Non-empty lowercase hexadecimal agent identity.", "type": "string", "required": true}
      ],
      "flags": [
        {"id": "restart", "long": "restart", "description": "Exact registered restart policy.", "type": "enum", "enum_values": ["always", "on_failure", "never"], "default": "never", "value_name": "POLICY"},
        {"id": "channel", "long": "channel", "description": "Repeatable NAME:read|write:UUID_V7 launch binding.", "type": "string", "repeatable": true, "value_name": "BINDING"},
        {"id": "model", "long": "model", "description": "Optional Level 1 model selector; requires both model-setting flags.", "type": "string", "value_name": "MODEL"},
        {"id": "temperature", "long": "temperature", "description": "Optional Level 1 temperature; requires both model-setting flags.", "type": "float", "value_name": "FLOAT"},
        {"id": "max_tokens", "long": "max-tokens", "description": "Optional Level 1 token cap; requires both model-setting flags.", "type": "integer", "value_name": "COUNT"}
      ]
    },
    {
      "id": "unwire",
      "name": "unwire",
      "description": "Authorize and remove one host pipeline binding.",
      "arguments": [
        {"id": "host", "name": "HOST", "description": "Registered host name.", "type": "string", "required": true}
      ]
    },
    {
      "id": "vault",
      "name": "vault",
      "description": "Provision Chief vault secrets locally; changes take effect at the next daemon restart.",
      "commands": [
        {
          "id": "vault_put",
          "name": "put",
          "description": "Seal one secret, read from stdin, with its admission policy.",
          "arguments": [
            {"id": "name", "name": "NAME", "description": "Secret name: a lowercase letter or digit, then a-z 0-9 . _ - (at most 120 bytes).", "type": "string", "required": true}
          ],
          "flags": [
            {"id": "mode", "long": "mode", "description": "How the secret may leave the vault.", "type": "enum", "enum_values": ["direct", "leased", "both"], "required": true, "value_name": "MODE"},
            {"id": "tier", "long": "tier", "description": "Minimum approval tier, 0-3 (recorded, not yet enforced).", "type": "integer", "required": true, "value_name": "TIER"},
            {"id": "allow_agent", "long": "allow-agent", "description": "Repeatable registered host name allowed to request the secret.", "type": "string", "repeatable": true, "value_name": "HOST"},
            {"id": "any_agent", "long": "any-agent", "description": "Allow every agent; must be stated explicitly.", "type": "boolean"},
            {"id": "destination", "long": "destination", "description": "Repeatable HOST:PORT net.fetch may send the secret to; required for leased and both.", "type": "string", "repeatable": true, "value_name": "HOST:PORT"},
            {"id": "raw", "long": "raw", "description": "Keep the stdin bytes exactly instead of stripping one trailing newline.", "type": "boolean"}
          ],
          "mutually_exclusive_groups": [
            {"id": "agents", "flag_ids": ["allow_agent", "any_agent"], "required": true}
          ]
        },
        {
          "id": "vault_delete",
          "name": "delete",
          "description": "Remove one sealed secret.",
          "arguments": [
            {"id": "name", "name": "NAME", "description": "Secret name.", "type": "string", "required": true}
          ]
        },
        {
          "id": "vault_list",
          "name": "list",
          "description": "List sealed secret names without decrypting anything."
        }
      ]
    }
  ]
}"#;

const MAX_AGENT_ID_HEX_CHARS: usize = 8 * 1024;

/// One complete successful parser outcome.
#[derive(Debug)]
pub enum CliAction {
    /// Generated help text; no daemon connection is needed.
    Help(String),
    /// Generated version text; no daemon connection is needed.
    Version(String),
    /// Install and start the current-user daemon through the native supervisor.
    InstallDaemon,
    /// One validated operation for an already-authenticated daemon client.
    Command(CliCommand),
    /// One local vault provisioning operation (D18U "Provisioning commands").
    ///
    /// Like [`CliAction::InstallDaemon`] this never reaches the daemon: it
    /// opens the sealed store directly with the owner-only KEK file.
    Vault(VaultCommand),
}

/// A local vault provisioning operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VaultCommand {
    /// Seal one secret read from stdin.
    Put(VaultPut),
    /// Remove one sealed secret.
    Delete(SecretName),
    /// List sealed secret names.
    List,
}

/// A validated `vault put`: everything except the secret itself.
///
/// The secret is deliberately absent. It is read from stdin by the
/// executable adapter (D18U U-C1), so it never passes through argv, the
/// declarative parser, or this type's `Debug` output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VaultPut {
    /// The secret's name.
    pub name: SecretName,
    /// Minimum approval tier, 0–3.
    pub privilege_tier: u8,
    /// Which attested hosts may request the secret.
    pub allowed_agents: AllowedAgents,
    /// Which delivery modes are admissible.
    pub allowed_mode: VaultDeliveryMode,
    /// Where `net.fetch` may send the secret (D18U U-C4a, VLT06 P9).
    pub allowed_destinations: BTreeSet<String>,
    /// Keep stdin bytes exactly (`--raw`) instead of stripping one newline.
    pub raw: bool,
}

impl VaultPut {
    /// The policy to store, stamped with the time of this write.
    ///
    /// The timestamp is supplied by the caller because this crate has no
    /// clock authority; the adapter reads the wall clock.
    pub fn policy_at(&self, rotated_at_ms: u64) -> SecretPolicy {
        SecretPolicy {
            privilege_tier: self.privilege_tier,
            allowed_agents: self.allowed_agents.clone(),
            allowed_mode: self.allowed_mode,
            rotated_at_ms,
            allowed_destinations: self.allowed_destinations.clone(),
        }
    }

    /// Apply D18U U-C3 to the bytes read from stdin.
    ///
    /// `echo` and `printf '%s\n'` both append a newline, and keeping it would
    /// provision a value that never matches what the owner meant. So exactly
    /// one trailing `\n`, or one `\r\n`, is dropped — never more, because a
    /// secret may legitimately end in whitespace. `--raw` keeps every byte.
    ///
    /// | input        | default  | `--raw`      |
    /// |--------------|----------|--------------|
    /// | `abc`        | `abc`    | `abc`        |
    /// | `abc\n`      | `abc`    | `abc\n`      |
    /// | `abc\r\n`    | `abc`    | `abc\r\n`    |
    /// | `abc\n\n`    | `abc\n`  | `abc\n\n`    |
    pub fn secret_bytes<'a>(&self, stdin: &'a [u8]) -> &'a [u8] {
        if self.raw {
            return stdin;
        }
        stdin
            .strip_suffix(b"\r\n")
            .or_else(|| stdin.strip_suffix(b"\n"))
            .unwrap_or(stdin)
    }
}

/// One typed operator command supported by the current daemon API.
#[derive(Clone, Debug, PartialEq)]
pub enum CliCommand {
    /// List all durable host registrations.
    Agents,
    /// Inspect durable intent and authoritative health for one host.
    Doctor(HostName),
    /// Register immutable package identity and initial operator intent.
    Register {
        /// Validated immutable host registration.
        registration: HostRegistration,
        /// Conservative initial lifecycle intent.
        desired_state: DesiredState,
    },
    /// Set one host's desired state to running.
    Start(HostName),
    /// Set one host's desired state to stopped.
    Stop(HostName),
    /// Run one bounded reconciliation tick.
    Reconcile,
    /// Remove stopped, inactive host intent.
    Deregister(HostName),
    /// Authorize and persist one exact host pipeline binding.
    Wire(HostPipelineBinding),
    /// Authorize and remove one host's current pipeline binding.
    Unwire(HostName),
}

/// Host-control calls required by the CLI dispatcher.
///
/// Implementations must already be authenticated before [`execute`] is called.
/// Keeping connection and authentication outside this trait prevents secrets
/// and endpoints from entering the argv model.
pub trait AuthenticatedDaemonClient {
    /// Register immutable package identity and initial intent.
    fn register_host(
        &mut self,
        registration: &HostRegistration,
        desired_state: DesiredState,
    ) -> Result<JsonValue, DaemonClientError>;

    /// List durable hosts.
    fn list_hosts(&mut self) -> Result<JsonValue, DaemonClientError>;

    /// Change durable desired lifecycle state.
    fn set_desired_state(
        &mut self,
        host_name: &HostName,
        desired_state: DesiredState,
    ) -> Result<JsonValue, DaemonClientError>;

    /// Run one bounded reconciliation tick.
    fn reconcile_once(&mut self) -> Result<JsonValue, DaemonClientError>;

    /// Inspect durable and authoritative host health.
    fn health_check(&mut self, host_name: &HostName) -> Result<JsonValue, DaemonClientError>;

    /// Remove stopped, inactive host intent.
    fn deregister_host(&mut self, host_name: &HostName) -> Result<JsonValue, DaemonClientError>;

    /// Authorize and persist one exact host pipeline binding.
    fn wire_host_pipeline(
        &mut self,
        binding: &HostPipelineBinding,
    ) -> Result<JsonValue, DaemonClientError>;

    /// Authorize and remove one host's current pipeline binding.
    fn unwire_host_pipeline(
        &mut self,
        host_name: &HostName,
    ) -> Result<JsonValue, DaemonClientError>;
}

impl AuthenticatedDaemonClient for DaemonClient {
    fn register_host(
        &mut self,
        registration: &HostRegistration,
        desired_state: DesiredState,
    ) -> Result<JsonValue, DaemonClientError> {
        DaemonClient::register_host(self, registration, desired_state)
    }

    fn list_hosts(&mut self) -> Result<JsonValue, DaemonClientError> {
        DaemonClient::list_hosts(self)
    }

    fn set_desired_state(
        &mut self,
        host_name: &HostName,
        desired_state: DesiredState,
    ) -> Result<JsonValue, DaemonClientError> {
        DaemonClient::set_desired_state(self, host_name, desired_state)
    }

    fn reconcile_once(&mut self) -> Result<JsonValue, DaemonClientError> {
        DaemonClient::reconcile_once(self)
    }

    fn health_check(&mut self, host_name: &HostName) -> Result<JsonValue, DaemonClientError> {
        DaemonClient::health_check(self, host_name)
    }

    fn deregister_host(&mut self, host_name: &HostName) -> Result<JsonValue, DaemonClientError> {
        DaemonClient::deregister_host(self, host_name)
    }

    fn wire_host_pipeline(
        &mut self,
        binding: &HostPipelineBinding,
    ) -> Result<JsonValue, DaemonClientError> {
        DaemonClient::wire_host_pipeline(self, binding)
    }

    fn unwire_host_pipeline(
        &mut self,
        host_name: &HostName,
    ) -> Result<JsonValue, DaemonClientError> {
        DaemonClient::unwire_host_pipeline(self, host_name)
    }
}

/// Parse one complete argv vector, including `argv[0]`.
///
/// Help and version are returned as local actions and never require a daemon
/// connection. Normal commands are fully validated before being returned.
pub fn parse_argv(argv: &[String]) -> Result<CliAction, CliError> {
    let spec = load_spec_from_str(CLI_SPEC).map_err(CliError::Parse)?;
    let parser = Parser::new(spec);
    match parser.parse(argv).map_err(CliError::Parse)? {
        ParserOutput::Help(help) => Ok(CliAction::Help(help.text)),
        ParserOutput::Version(version) => Ok(CliAction::Version(version.version)),
        ParserOutput::Parse(result)
            if result.command_path.last().map(String::as_str) == Some("install-daemon") =>
        {
            Ok(CliAction::InstallDaemon)
        }
        ParserOutput::Parse(result) if result.command_path.iter().any(|part| part == "vault") => {
            parse_vault(&result).map(CliAction::Vault)
        }
        ParserOutput::Parse(result) => parse_command(&result).map(CliAction::Command),
    }
}

/// Execute one validated command through an already-authenticated client.
pub fn execute(
    client: &mut impl AuthenticatedDaemonClient,
    command: CliCommand,
) -> Result<JsonValue, CliError> {
    match command {
        CliCommand::Agents => client.list_hosts(),
        CliCommand::Doctor(host_name) => client.health_check(&host_name),
        CliCommand::Register {
            registration,
            desired_state,
        } => client.register_host(&registration, desired_state),
        CliCommand::Start(host_name) => client.set_desired_state(&host_name, DesiredState::Running),
        CliCommand::Stop(host_name) => client.set_desired_state(&host_name, DesiredState::Stopped),
        CliCommand::Reconcile => client.reconcile_once(),
        CliCommand::Deregister(host_name) => client.deregister_host(&host_name),
        CliCommand::Wire(binding) => client.wire_host_pipeline(&binding),
        CliCommand::Unwire(host_name) => client.unwire_host_pipeline(&host_name),
    }
    .map_err(CliError::Daemon)
}

/// Render one successful daemon result as deterministic pretty JSON.
pub fn render_result(result: &JsonValue) -> Result<String, CliError> {
    let config = SerializerConfig {
        sort_keys: true,
        trailing_newline: true,
        ..SerializerConfig::default()
    };
    serialize_pretty(result, &config).map_err(CliError::Serialize)
}

fn parse_command(result: &ParseResult) -> Result<CliCommand, CliError> {
    match result.command_path.last().map(String::as_str) {
        Some("agents") => Ok(CliCommand::Agents),
        Some("doctor") => Ok(CliCommand::Doctor(host_name(result)?)),
        Some("register") => parse_register(result),
        Some("start") => Ok(CliCommand::Start(host_name(result)?)),
        Some("stop") => Ok(CliCommand::Stop(host_name(result)?)),
        Some("reconcile") => Ok(CliCommand::Reconcile),
        Some("deregister") => Ok(CliCommand::Deregister(host_name(result)?)),
        Some("wire") => parse_wire(result),
        Some("unwire") => Ok(CliCommand::Unwire(host_name(result)?)),
        _ => Err(CliError::InvalidInput {
            field: "command",
            message: "a supported subcommand is required",
        }),
    }
}

fn parse_wire(result: &ParseResult) -> Result<CliCommand, CliError> {
    let registration = HostRegistration::new(
        host_name(result)?,
        PackagePath::new(argument(result, "package_path")?).map_err(CliError::Registry)?,
        decode_fixed_hex(argument(result, "package_hash")?, "package_hash")?,
        parse_restart_policy(flag(result, "restart")?)?,
    );
    let pipeline_id = PipelineId::new(parse_uuid_v7(
        argument(result, "pipeline_id")?,
        "pipeline_id",
    )?)
    .map_err(|_| invalid_value("pipeline_id", "must encode a canonical UUID-v7"))?;
    let agent_bytes = decode_hex_vec(argument(result, "agent_id")?, "agent_id")?;
    let agent_id = AgentId::new(agent_bytes)
        .map_err(|_| invalid_value("agent_id", "exceeds the bounded agent identity size"))?;
    let channel_values = repeatable_flag(result, "channel")?;
    if channel_values.len() > MAX_LAUNCH_CHANNEL_BINDINGS {
        return Err(invalid_value(
            "channel",
            "too many channel bindings for one launch",
        ));
    }
    let channels = channel_values
        .iter()
        .map(|value| parse_channel_binding(value))
        .collect::<Result<Vec<_>, _>>()?;
    let level_one_model = parse_level_one_model(result)?;
    let launch_bindings = LaunchBindings::new(channels, level_one_model).map_err(|_| {
        invalid_value(
            "channel",
            "bindings must have unique names and UUIDs within the launch bound",
        )
    })?;
    Ok(CliCommand::Wire(HostPipelineBinding::new(
        pipeline_id,
        registration,
        agent_id,
        launch_bindings,
    )))
}

fn parse_vault(result: &ParseResult) -> Result<VaultCommand, CliError> {
    match result.command_path.last().map(String::as_str) {
        Some("put") => parse_vault_put(result).map(VaultCommand::Put),
        Some("delete") => secret_name(result).map(VaultCommand::Delete),
        Some("list") => Ok(VaultCommand::List),
        _ => Err(invalid_value(
            "command",
            "vault needs a subcommand: put, delete, or list",
        )),
    }
}

/// Validate `vault put`. The declarative parser already requires `--mode` and
/// `--tier` and enforces the agent group; every rule is checked again here so
/// the typed boundary does not depend on the parser's spec staying correct.
fn parse_vault_put(result: &ParseResult) -> Result<VaultPut, CliError> {
    let name = secret_name(result)?;
    let allowed_mode = match flag(result, "mode")? {
        "direct" => VaultDeliveryMode::Direct,
        "leased" => VaultDeliveryMode::Leased,
        "both" => VaultDeliveryMode::Both,
        _ => return Err(invalid_spec_value("mode")),
    };
    let privilege_tier = result
        .flags
        .get("tier")
        .and_then(|value| value.as_i64())
        .ok_or_else(|| invalid_value("tier", "--tier is required"))?;
    let privilege_tier = u8::try_from(privilege_tier)
        .ok()
        .filter(|tier| *tier <= MAX_PRIVILEGE_TIER)
        .ok_or_else(|| invalid_value("tier", "must be 0, 1, 2, or 3"))?;
    let allow = repeatable_flag(result, "allow_agent")?;
    let any = bool_flag(result, "any_agent")?;
    // D18U U-C4: exactly one of the two, and never a default.
    let allowed_agents = match (allow.is_empty(), any) {
        (true, true) => AllowedAgents::Any,
        (false, false) => {
            // U-C5: an allow-list entry must be a name a caller can actually
            // be attested as, which is a registered host name.
            let hosts = allow
                .iter()
                .map(|host| HostName::new(*host).map(|host| host.as_str().to_string()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(CliError::Registry)?;
            AllowedAgents::only(hosts)
        }
        (true, false) => {
            return Err(invalid_value(
                "allow_agent",
                "state who may request the secret: --allow-agent HOST or --any-agent",
            ))
        }
        (false, true) => {
            return Err(invalid_value(
                "any_agent",
                "--any-agent cannot be combined with --allow-agent",
            ))
        }
    };
    // D18U U-C4a: a leasable secret must say where it may be sent, and a
    // direct-only secret never reaches net.fetch, so naming a destination for
    // one is a mistake rather than a harmless extra.
    let allowed_destinations = repeatable_flag(result, "destination")?
        .into_iter()
        .map(|destination| {
            let destination = destination.to_ascii_lowercase();
            validate_destination(&destination)
                .map(|()| destination)
                .map_err(|why| invalid_value("destination", why))
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    match (allowed_mode, allowed_destinations.is_empty()) {
        (VaultDeliveryMode::Direct, false) => {
            return Err(invalid_value(
                "destination",
                "a direct-only secret is never sent by net.fetch; drop --destination",
            ))
        }
        (VaultDeliveryMode::Leased | VaultDeliveryMode::Both, true) => {
            return Err(invalid_value(
                "destination",
                "a leasable secret must name where it may be sent: --destination HOST:PORT",
            ))
        }
        _ => {}
    }
    let put = VaultPut {
        name,
        privilege_tier,
        allowed_agents,
        allowed_mode,
        allowed_destinations,
        raw: bool_flag(result, "raw")?,
    };
    // The record format's own bounds (agent count, id length) are checked
    // now, before the adapter reads a single byte of the secret.
    validate_policy(&put.policy_at(0)).map_err(|_| {
        invalid_value(
            "allow_agent",
            "the allow-list exceeds the sealed record's bounds",
        )
    })?;
    Ok(put)
}

fn secret_name(result: &ParseResult) -> Result<SecretName, CliError> {
    SecretName::parse(argument(result, "name")?).map_err(CliError::SecretName)
}

fn bool_flag(result: &ParseResult, id: &'static str) -> Result<bool, CliError> {
    match result.flags.get(id) {
        None => Ok(false),
        Some(value) if value.is_null() => Ok(false),
        Some(value) => value.as_bool().ok_or_else(|| invalid_spec_value(id)),
    }
}

fn parse_restart_policy(value: &str) -> Result<RestartPolicy, CliError> {
    match value {
        "always" => Ok(RestartPolicy::Always),
        "on_failure" => Ok(RestartPolicy::OnFailure),
        "never" => Ok(RestartPolicy::Never),
        _ => Err(invalid_spec_value("restart")),
    }
}

fn parse_channel_binding(value: &str) -> Result<ChannelBinding, CliError> {
    let mut fields = value.split(':');
    let name = fields.next().unwrap_or_default();
    let access = match fields.next() {
        Some("read") => ChannelBindingAccess::Read,
        Some("write") => ChannelBindingAccess::Write,
        _ => {
            return Err(invalid_value("channel", "must use NAME:read|write:UUID_V7"));
        }
    };
    let channel_id = fields
        .next()
        .ok_or_else(|| invalid_value("channel", "must use NAME:read|write:UUID_V7"))?;
    if fields.next().is_some() {
        return Err(invalid_value("channel", "must use NAME:read|write:UUID_V7"));
    }
    ChannelBinding::new(name, access, parse_uuid_v7(channel_id, "channel")?).map_err(|_| {
        invalid_value(
            "channel",
            "name and UUID-v7 must satisfy the launch binding contract",
        )
    })
}

fn parse_level_one_model(result: &ParseResult) -> Result<Option<LevelOneModelBinding>, CliError> {
    let model = optional_string_flag(result, "model")?;
    let temperature = match result.flags.get("temperature") {
        Some(value) if value.is_null() => None,
        Some(value) => value.as_f64(),
        None => return Err(invalid_spec_value("temperature")),
    };
    let max_tokens = match result.flags.get("max_tokens") {
        Some(value) if value.is_null() => None,
        Some(value) => value.as_i64(),
        None => return Err(invalid_spec_value("max_tokens")),
    };
    let (Some(model), Some(temperature), Some(max_tokens)) = (model, temperature, max_tokens)
    else {
        if model.is_none() && temperature.is_none() && max_tokens.is_none() {
            return Ok(None);
        }
        return Err(invalid_value(
            "model",
            "model, temperature, and max-tokens must be supplied together",
        ));
    };
    let max_tokens = u32::try_from(max_tokens)
        .map_err(|_| invalid_value("max_tokens", "must fit a positive 32-bit count"))?;
    LevelOneModelBinding::new(model, temperature as f32, max_tokens)
        .map(Some)
        .map_err(|_| invalid_value("model", "settings violate the Level 1 launch bound"))
}

fn parse_register(result: &ParseResult) -> Result<CliCommand, CliError> {
    let host_name = host_name(result)?;
    let package_path =
        PackagePath::new(argument(result, "package_path")?).map_err(CliError::Registry)?;
    let package_hash = decode_fixed_hex(argument(result, "package_hash")?, "package_hash")?;
    let restart_policy = parse_restart_policy(flag(result, "restart")?)?;
    let desired_state = match flag(result, "state")? {
        "running" => DesiredState::Running,
        "stopped" => DesiredState::Stopped,
        _ => return Err(invalid_spec_value("state")),
    };
    Ok(CliCommand::Register {
        registration: HostRegistration::new(host_name, package_path, package_hash, restart_policy),
        desired_state,
    })
}

fn host_name(result: &ParseResult) -> Result<HostName, CliError> {
    HostName::new(argument(result, "host")?).map_err(CliError::Registry)
}

fn argument<'a>(result: &'a ParseResult, id: &'static str) -> Result<&'a str, CliError> {
    result
        .arguments
        .get(id)
        .and_then(|value| value.as_str())
        .ok_or_else(|| invalid_spec_value(id))
}

fn flag<'a>(result: &'a ParseResult, id: &'static str) -> Result<&'a str, CliError> {
    result
        .flags
        .get(id)
        .and_then(|value| value.as_str())
        .ok_or_else(|| invalid_spec_value(id))
}

fn optional_string_flag<'a>(
    result: &'a ParseResult,
    id: &'static str,
) -> Result<Option<&'a str>, CliError> {
    match result.flags.get(id) {
        Some(value) if value.is_null() => Ok(None),
        Some(value) => value
            .as_str()
            .map(Some)
            .ok_or_else(|| invalid_spec_value(id)),
        None => Err(invalid_spec_value(id)),
    }
}

fn repeatable_flag<'a>(
    result: &'a ParseResult,
    id: &'static str,
) -> Result<Vec<&'a str>, CliError> {
    result
        .flags
        .get(id)
        .and_then(|value| value.as_array())
        .ok_or_else(|| invalid_spec_value(id))?
        .iter()
        .map(|value| value.as_str().ok_or_else(|| invalid_spec_value(id)))
        .collect()
}

fn invalid_spec_value(field: &'static str) -> CliError {
    CliError::InvalidInput {
        field,
        message: "declarative parser returned an unexpected value",
    }
}

fn invalid_value(field: &'static str, message: &'static str) -> CliError {
    CliError::InvalidInput { field, message }
}

fn decode_fixed_hex<const N: usize>(value: &str, field: &'static str) -> Result<[u8; N], CliError> {
    if value.len() != N * 2
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid_value(
            field,
            "must have the exact lowercase hexadecimal length",
        ));
    }
    let mut bytes = [0_u8; N];
    for (index, pair) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        bytes[index] = (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]);
    }
    Ok(bytes)
}

fn decode_hex_vec(value: &str, field: &'static str) -> Result<Vec<u8>, CliError> {
    if value.is_empty()
        || value.len() > MAX_AGENT_ID_HEX_CHARS
        || !value.len().is_multiple_of(2)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid_value(
            field,
            "must be non-empty lowercase hexadecimal bytes",
        ));
    }
    Ok(value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]))
        .collect())
}

fn parse_uuid_v7(value: &str, field: &'static str) -> Result<[u8; 16], CliError> {
    let parsed = coding_adventures_uuid::parse(value)
        .map_err(|_| invalid_value(field, "must be a canonical lowercase dashed UUID-v7"))?;
    if parsed.version() != 7 || parsed.variant() != "rfc4122" || parsed.to_string() != value {
        return Err(invalid_value(
            field,
            "must be a canonical lowercase dashed UUID-v7",
        ));
    }
    Ok(parsed.bytes())
}

fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => unreachable!("package hash was validated before decoding"),
    }
}

/// Stable failure categories from parsing, validation, dispatch, or rendering.
#[derive(Debug)]
pub enum CliError {
    /// Declarative specification or argv parsing failed.
    Parse(CliBuilderError),
    /// A parsed value violated a typed CLI boundary.
    InvalidInput {
        /// Stable input field name.
        field: &'static str,
        /// Payload-independent validation message.
        message: &'static str,
    },
    /// Shared registry identity validation failed.
    Registry(RegistryError),
    /// A vault secret name violated D18U U-N1.
    SecretName(NameError),
    /// The authenticated daemon client rejected or could not complete the call.
    Daemon(DaemonClientError),
    /// A successful JSON result could not be represented.
    Serialize(JsonSerializerError),
}

impl Display for CliError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(formatter, "{error}"),
            Self::InvalidInput { field, message } => {
                write!(formatter, "invalid {field}: {message}")
            }
            Self::Registry(error) => write!(formatter, "{error}"),
            Self::SecretName(error) => write!(formatter, "invalid secret name: {error}"),
            Self::Daemon(error) => write!(formatter, "{error}"),
            Self::Serialize(_) => formatter.write_str("chief CLI could not serialize the result"),
        }
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(error) => Some(error),
            Self::Registry(error) => Some(error),
            Self::SecretName(error) => Some(error),
            Self::Daemon(error) => Some(error),
            Self::Serialize(error) => Some(error),
            Self::InvalidInput { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_json_value::JsonNumber;

    fn argv(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    fn command(values: &[&str]) -> CliCommand {
        match parse_argv(&argv(values)).unwrap() {
            CliAction::Command(command) => command,
            other => panic!("expected command, got {other:?}"),
        }
    }

    #[test]
    fn help_and_version_are_local_actions_without_secret_flags() {
        let CliAction::Help(help) = parse_argv(&argv(&["chief", "--help"])).unwrap() else {
            panic!("expected help");
        };
        assert!(help.contains("agents"));
        assert!(help.contains("doctor"));
        assert!(help.contains("install-daemon"));
        for forbidden in ["credential", "password", "passphrase", "token", "endpoint"] {
            assert!(!help.to_ascii_lowercase().contains(forbidden));
        }
        assert!(matches!(
            parse_argv(&argv(&["chief", "agents", "--token", "secret"])),
            Err(CliError::Parse(_))
        ));

        let CliAction::Version(version) = parse_argv(&argv(&["chief", "--version"])).unwrap()
        else {
            panic!("expected version");
        };
        assert_eq!(version, "0.1.0");
    }

    #[test]
    fn parses_every_host_lifecycle_command() {
        assert!(matches!(
            parse_argv(&argv(&["chief", "install-daemon"])).unwrap(),
            CliAction::InstallDaemon
        ));
        assert_eq!(command(&["chief", "agents"]), CliCommand::Agents);
        assert!(matches!(
            command(&["chief", "doctor", "alpha-host"]),
            CliCommand::Doctor(host) if host.as_str() == "alpha-host"
        ));
        assert!(matches!(
            command(&["chief", "start", "alpha-host"]),
            CliCommand::Start(host) if host.as_str() == "alpha-host"
        ));
        assert!(matches!(
            command(&["chief", "stop", "alpha-host"]),
            CliCommand::Stop(host) if host.as_str() == "alpha-host"
        ));
        assert_eq!(command(&["chief", "reconcile"]), CliCommand::Reconcile);
        assert!(matches!(
            command(&["chief", "deregister", "alpha-host"]),
            CliCommand::Deregister(host) if host.as_str() == "alpha-host"
        ));
    }

    #[test]
    fn register_uses_conservative_defaults_and_decodes_hash() {
        let hash = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let CliCommand::Register {
            registration,
            desired_state,
        } = command(&["chief", "register", "alpha-host", "pkg/alpha", hash])
        else {
            panic!("expected register");
        };
        assert_eq!(registration.host_name().as_str(), "alpha-host");
        assert_eq!(registration.package_path().as_str(), "pkg/alpha");
        assert_eq!(registration.package_hash()[..4], [0x01, 0x23, 0x45, 0x67]);
        assert_eq!(registration.restart_policy(), RestartPolicy::Never);
        assert_eq!(desired_state, DesiredState::Stopped);

        let CliCommand::Register {
            registration,
            desired_state,
        } = command(&[
            "chief",
            "register",
            "alpha-host",
            "pkg/alpha",
            hash,
            "--restart",
            "on_failure",
            "--state",
            "running",
        ])
        else {
            panic!("expected register");
        };
        assert_eq!(registration.restart_policy(), RestartPolicy::OnFailure);
        assert_eq!(desired_state, DesiredState::Running);
    }

    #[test]
    fn typed_boundaries_reject_invalid_identity_before_dispatch() {
        assert!(matches!(
            parse_argv(&argv(&["chief", "doctor", "UPPER"])),
            Err(CliError::Registry(_))
        ));
        assert!(matches!(
            parse_argv(&argv(&[
                "chief",
                "register",
                "alpha-host",
                "pkg/alpha",
                "ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789"
            ])),
            Err(CliError::InvalidInput {
                field: "package_hash",
                ..
            })
        ));
        assert!(matches!(
            parse_argv(&argv(&[
                "chief",
                "register",
                "alpha-host",
                "bad\npath",
                &"0".repeat(64)
            ])),
            Err(CliError::Registry(_))
        ));
    }

    #[test]
    fn wire_parses_exact_repeatable_bindings_and_optional_model() {
        let package_hash = "01".repeat(32);
        let pipeline_id = "00000000-0000-7000-8000-000000000000";
        let read_channel = "00000000-0000-7000-8000-000000000001";
        let write_channel = "00000000-0000-7000-8000-000000000002";
        let CliCommand::Wire(binding) = command(&[
            "chief",
            "wire",
            "alpha-host",
            "pkg/alpha",
            &package_hash,
            pipeline_id,
            "6167656e742d31",
            "--restart",
            "always",
            "--channel",
            &format!("requests:read:{read_channel}"),
            "--channel",
            &format!("responses:write:{write_channel}"),
            "--model",
            "local-model",
            "--temperature",
            "0.25",
            "--max-tokens",
            "2048",
        ]) else {
            panic!("expected wire");
        };
        assert_eq!(binding.pipeline_id().as_bytes()[6] >> 4, 7);
        assert_eq!(binding.registration().host_name().as_str(), "alpha-host");
        assert_eq!(
            binding.registration().restart_policy(),
            RestartPolicy::Always
        );
        assert_eq!(binding.agent_id().as_bytes(), b"agent-1");
        let launch = binding.launch_bindings();
        assert_eq!(launch.channels().len(), 2);
        assert_eq!(launch.channels()[0].name(), "requests");
        assert_eq!(launch.channels()[0].access(), ChannelBindingAccess::Read);
        assert_eq!(launch.channels()[1].name(), "responses");
        assert_eq!(launch.channels()[1].access(), ChannelBindingAccess::Write);
        let model = launch.level_one_model().expect("model binding");
        assert_eq!(model.model(), "local-model");
        assert_eq!(model.temperature(), 0.25);
        assert_eq!(model.max_tokens(), 2048);
    }

    #[test]
    fn wire_and_unwire_reject_invalid_or_partial_argv() {
        let hash = "0".repeat(64);
        let pipeline_id = "00000000-0000-7000-8000-000000000000";
        assert!(matches!(
            parse_argv(&argv(&[
                "chief",
                "wire",
                "alpha-host",
                "pkg/alpha",
                &hash,
                pipeline_id,
                "00",
                "--channel",
                "bad:execute:00000000-0000-7000-8000-000000000001"
            ])),
            Err(CliError::InvalidInput {
                field: "channel",
                ..
            })
        ));
        assert!(parse_argv(&argv(&[
            "chief",
            "wire",
            "alpha-host",
            "pkg/alpha",
            &hash,
            pipeline_id,
            "00",
            "--model",
            "partial-model"
        ]))
        .is_err());
        assert!(matches!(
            parse_argv(&argv(&[
                "chief",
                "wire",
                "alpha-host",
                "pkg/alpha",
                &hash,
                "00000000000070008000000000000000",
                "00"
            ])),
            Err(CliError::InvalidInput {
                field: "pipeline_id",
                ..
            })
        ));
        assert!(matches!(
            command(&["chief", "unwire", "alpha-host"]),
            CliCommand::Unwire(host) if host.as_str() == "alpha-host"
        ));
    }

    struct RecordingClient {
        calls: Vec<String>,
        fail_next: bool,
    }

    impl RecordingClient {
        fn record(&mut self, call: String) -> Result<JsonValue, DaemonClientError> {
            self.calls.push(call);
            if self.fail_next {
                self.fail_next = false;
                Err(DaemonClientError::Remote {
                    code: "forbidden".to_string(),
                    message: "sensitive adapter detail".to_string(),
                })
            } else {
                Ok(JsonValue::String("ok".to_string()))
            }
        }
    }

    impl AuthenticatedDaemonClient for RecordingClient {
        fn register_host(
            &mut self,
            registration: &HostRegistration,
            desired_state: DesiredState,
        ) -> Result<JsonValue, DaemonClientError> {
            self.record(format!(
                "register:{}:{desired_state:?}",
                registration.host_name()
            ))
        }

        fn list_hosts(&mut self) -> Result<JsonValue, DaemonClientError> {
            self.record("agents".to_string())
        }

        fn set_desired_state(
            &mut self,
            host_name: &HostName,
            desired_state: DesiredState,
        ) -> Result<JsonValue, DaemonClientError> {
            self.record(format!("state:{host_name}:{desired_state:?}"))
        }

        fn reconcile_once(&mut self) -> Result<JsonValue, DaemonClientError> {
            self.record("reconcile".to_string())
        }

        fn health_check(&mut self, host_name: &HostName) -> Result<JsonValue, DaemonClientError> {
            self.record(format!("doctor:{host_name}"))
        }

        fn deregister_host(
            &mut self,
            host_name: &HostName,
        ) -> Result<JsonValue, DaemonClientError> {
            self.record(format!("deregister:{host_name}"))
        }

        fn wire_host_pipeline(
            &mut self,
            binding: &HostPipelineBinding,
        ) -> Result<JsonValue, DaemonClientError> {
            self.record(format!("wire:{}", binding.registration().host_name()))
        }

        fn unwire_host_pipeline(
            &mut self,
            host_name: &HostName,
        ) -> Result<JsonValue, DaemonClientError> {
            self.record(format!("unwire:{host_name}"))
        }
    }

    #[test]
    fn execute_routes_every_command_and_preserves_typed_daemon_errors() {
        let mut client = RecordingClient {
            calls: Vec::new(),
            fail_next: false,
        };
        let hash = "0".repeat(64);
        let pipeline_id = "00000000-0000-7000-8000-000000000000";
        let invocations = vec![
            argv(&["chief", "agents"]),
            argv(&["chief", "doctor", "alpha-host"]),
            argv(&["chief", "register", "alpha-host", "pkg/alpha", &hash]),
            argv(&["chief", "start", "alpha-host"]),
            argv(&["chief", "stop", "alpha-host"]),
            argv(&["chief", "reconcile"]),
            argv(&["chief", "deregister", "alpha-host"]),
            argv(&[
                "chief",
                "wire",
                "alpha-host",
                "pkg/alpha",
                &hash,
                pipeline_id,
                "00",
            ]),
            argv(&["chief", "unwire", "alpha-host"]),
        ];
        for invocation in invocations {
            let CliAction::Command(command) = parse_argv(&invocation).unwrap() else {
                panic!("expected command");
            };
            assert_eq!(
                execute(&mut client, command).unwrap(),
                JsonValue::String("ok".into())
            );
        }
        assert_eq!(
            client.calls,
            [
                "agents",
                "doctor:alpha-host",
                "register:alpha-host:Stopped",
                "state:alpha-host:Running",
                "state:alpha-host:Stopped",
                "reconcile",
                "deregister:alpha-host",
                "wire:alpha-host",
                "unwire:alpha-host",
            ]
        );

        client.fail_next = true;
        let error = execute(&mut client, CliCommand::Agents).unwrap_err();
        assert!(matches!(
            &error,
            CliError::Daemon(DaemonClientError::Remote { code, message })
                if code == "forbidden" && message == "sensitive adapter detail"
        ));
        assert_eq!(error.to_string(), "chief daemon rejected the request");
        assert!(!error.to_string().contains("sensitive"));
    }

    #[test]
    fn result_rendering_is_sorted_pretty_json_with_newline() {
        let value = JsonValue::Object(vec![
            ("z".to_string(), JsonValue::Number(JsonNumber::Integer(2))),
            ("a".to_string(), JsonValue::Bool(true)),
        ]);
        assert_eq!(
            render_result(&value).unwrap(),
            "{\n  \"a\": true,\n  \"z\": 2\n}\n"
        );
    }

    #[test]
    fn root_invocation_requires_a_supported_subcommand() {
        assert!(matches!(
            parse_argv(&argv(&["chief"])),
            Err(CliError::InvalidInput {
                field: "command",
                ..
            })
        ));
    }

    // ── vault (D18U "Provisioning commands") ─────────────────────────────────

    fn vault(values: &[&str]) -> Result<VaultCommand, CliError> {
        match parse_argv(&argv(values))? {
            CliAction::Vault(command) => Ok(command),
            other => panic!("expected a vault action, got {other:?}"),
        }
    }

    fn put(values: &[&str]) -> Result<VaultPut, CliError> {
        let mut full = vec!["chief", "vault", "put"];
        full.extend_from_slice(values);
        match vault(&full)? {
            VaultCommand::Put(put) => Ok(put),
            other => panic!("expected put, got {other:?}"),
        }
    }

    fn invalid_field(error: CliError) -> &'static str {
        match error {
            CliError::InvalidInput { field, .. } => field,
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn vault_put_parses_a_fully_stated_policy() {
        let parsed = put(&[
            "weather-key",
            "--mode",
            "leased",
            "--destination",
            "api.weather.gov:443",
            "--tier",
            "2",
            "--allow-agent",
            "weather-host",
            "--allow-agent",
            "alpha-host",
        ])
        .unwrap();
        assert_eq!(parsed.name.as_str(), "weather-key");
        assert_eq!(parsed.privilege_tier, 2);
        assert_eq!(parsed.allowed_mode, VaultDeliveryMode::Leased);
        assert_eq!(
            parsed.allowed_agents,
            AllowedAgents::only(["alpha-host", "weather-host"])
        );
        assert!(!parsed.raw);
        let policy = parsed.policy_at(1_234);
        assert_eq!(policy.rotated_at_ms, 1_234);
        assert_eq!(policy.allowed_mode, VaultDeliveryMode::Leased);
    }

    #[test]
    fn vault_put_any_agent_direct_and_raw() {
        let parsed = put(&[
            "bank",
            "--mode",
            "direct",
            "--tier",
            "0",
            "--any-agent",
            "--raw",
        ])
        .unwrap();
        assert_eq!(parsed.allowed_agents, AllowedAgents::Any);
        assert_eq!(parsed.allowed_mode, VaultDeliveryMode::Direct);
        assert!(parsed.raw);
        let both = put(&[
            "x",
            "--mode",
            "both",
            "--destination",
            "api.weather.gov:443",
            "--tier",
            "3",
            "--any-agent",
        ])
        .unwrap();
        assert_eq!(both.allowed_mode, VaultDeliveryMode::Both);
        assert_eq!(both.privilege_tier, 3);
    }

    #[test]
    fn vault_put_requires_every_policy_flag() {
        // U-C4: no flag has a default; each omission is refused.
        for missing in [
            &["k", "--tier", "1", "--any-agent"][..],
            &[
                "k",
                "--mode",
                "leased",
                "--destination",
                "api.weather.gov:443",
                "--any-agent",
            ][..],
            &[
                "k",
                "--mode",
                "leased",
                "--destination",
                "api.weather.gov:443",
                "--tier",
                "1",
            ][..],
        ] {
            assert!(put(missing).is_err(), "{missing:?} must be refused");
        }
    }

    #[test]
    fn vault_put_refuses_both_agent_forms_together() {
        assert!(put(&[
            "k",
            "--mode",
            "leased",
            "--destination",
            "api.weather.gov:443",
            "--tier",
            "1",
            "--any-agent",
            "--allow-agent",
            "alpha-host",
        ])
        .is_err());
    }

    #[test]
    fn vault_put_refuses_out_of_range_tiers_and_bad_modes() {
        for tier in ["4", "-1", "300"] {
            let error = put(&[
                "k",
                "--mode",
                "leased",
                "--destination",
                "api.weather.gov:443",
                "--tier",
                tier,
                "--any-agent",
            ])
            .unwrap_err();
            assert_eq!(invalid_field(error), "tier", "tier {tier}");
        }
        assert!(matches!(
            put(&["k", "--mode", "sometimes", "--tier", "1", "--any-agent"]),
            Err(CliError::Parse(_))
        ));
    }

    #[test]
    fn vault_put_allow_agent_uses_the_host_name_grammar() {
        // U-C5: a value no attested caller could ever carry is refused.
        for bad in ["Upper", "a", "has space", "under_score", "1digit"] {
            assert!(
                matches!(
                    put(&[
                        "k",
                        "--mode",
                        "leased",
                        "--destination",
                        "api.weather.gov:443",
                        "--tier",
                        "1",
                        "--allow-agent",
                        bad
                    ]),
                    Err(CliError::Registry(_))
                ),
                "{bad:?} must be refused"
            );
        }
    }

    #[test]
    fn vault_put_bounds_the_allow_list_before_reading_any_secret() {
        let mut values = vec![
            "k",
            "--mode",
            "leased",
            "--destination",
            "api.weather.gov:443",
            "--tier",
            "1",
        ];
        let hosts: Vec<String> = (0..65).map(|i| format!("host-{i}")).collect();
        for host in &hosts {
            values.push("--allow-agent");
            values.push(host);
        }
        assert_eq!(invalid_field(put(&values).unwrap_err()), "allow_agent");
    }

    #[test]
    fn vault_secret_names_follow_d18u() {
        let error = put(&[
            "Bad/Name",
            "--mode",
            "leased",
            "--destination",
            "api.weather.gov:443",
            "--tier",
            "1",
            "--any-agent",
        ])
        .unwrap_err();
        assert!(matches!(error, CliError::SecretName(NameError::BadStart)));
        assert!(error.to_string().contains("invalid secret name"));
        assert!(std::error::Error::source(&error).is_some());
        assert!(matches!(
            vault(&["chief", "vault", "delete", "a..b"]),
            Err(CliError::SecretName(NameError::DotDot))
        ));
    }

    #[test]
    fn vault_delete_and_list_parse() {
        assert!(matches!(
            vault(&["chief", "vault", "delete", "weather-key"]).unwrap(),
            VaultCommand::Delete(name) if name.as_str() == "weather-key"
        ));
        assert_eq!(
            vault(&["chief", "vault", "list"]).unwrap(),
            VaultCommand::List
        );
        assert_eq!(
            invalid_field(vault(&["chief", "vault"]).unwrap_err()),
            "command"
        );
    }

    #[test]
    fn vault_put_never_takes_the_secret_on_argv() {
        // There is no flag or argument for the value: an extra positional is a
        // parse error rather than a silently accepted secret.
        assert!(matches!(
            parse_argv(&argv(&[
                "chief",
                "vault",
                "put",
                "k",
                "s3cret",
                "--mode",
                "leased",
                "--destination",
                "api.weather.gov:443",
                "--tier",
                "1",
                "--any-agent",
            ])),
            Err(CliError::Parse(_))
        ));
        let CliAction::Help(help) =
            parse_argv(&argv(&["chief", "vault", "put", "--help"])).unwrap()
        else {
            panic!("expected help");
        };
        assert!(help.contains("stdin"));
    }

    #[test]
    fn secret_bytes_strips_exactly_one_newline_unless_raw() {
        let cooked = put(&[
            "k",
            "--mode",
            "leased",
            "--destination",
            "api.weather.gov:443",
            "--tier",
            "1",
            "--any-agent",
        ])
        .unwrap();
        let raw = put(&[
            "k",
            "--mode",
            "leased",
            "--destination",
            "api.weather.gov:443",
            "--tier",
            "1",
            "--any-agent",
            "--raw",
        ])
        .unwrap();
        let table: [(&[u8], &[u8], &[u8]); 6] = [
            (b"abc", b"abc", b"abc"),
            (b"abc\n", b"abc", b"abc\n"),
            (b"abc\r\n", b"abc", b"abc\r\n"),
            (b"abc\n\n", b"abc\n", b"abc\n\n"),
            (b"abc\r", b"abc\r", b"abc\r"),
            (b"\n", b"", b"\n"),
        ];
        for (input, default, kept) in table {
            assert_eq!(cooked.secret_bytes(input), default, "{input:?}");
            assert_eq!(raw.secret_bytes(input), kept, "{input:?} raw");
        }
    }

    #[test]
    fn leasable_secrets_require_destinations_and_direct_ones_refuse_them() {
        // D18U U-C4a.
        let missing = put(&["k", "--mode", "leased", "--tier", "1", "--any-agent"]).unwrap_err();
        assert_eq!(invalid_field(missing), "destination");
        let direct = put(&[
            "k",
            "--mode",
            "direct",
            "--tier",
            "1",
            "--any-agent",
            "--destination",
            "a.example:443",
        ])
        .unwrap_err();
        assert_eq!(invalid_field(direct), "destination");
        for bad in [
            "127.0.0.1:443",
            "a.example",
            "a.example:0",
            "a_b.example:443",
        ] {
            let error = put(&[
                "k",
                "--mode",
                "both",
                "--tier",
                "1",
                "--any-agent",
                "--destination",
                bad,
            ])
            .unwrap_err();
            assert_eq!(invalid_field(error), "destination", "{bad}");
        }
        let ok = put(&[
            "k",
            "--mode",
            "leased",
            "--tier",
            "1",
            "--any-agent",
            "--destination",
            "API.Weather.gov:443",
            "--destination",
            "b.example:8443",
        ])
        .unwrap();
        assert_eq!(
            ok.allowed_destinations,
            BTreeSet::from([
                "api.weather.gov:443".to_string(),
                "b.example:8443".to_string()
            ]),
            "lowercased and canonical"
        );
        assert_eq!(
            ok.policy_at(5).allowed_destinations,
            ok.allowed_destinations
        );
    }
}
