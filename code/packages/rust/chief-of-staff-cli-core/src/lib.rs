//! Declarative operator command core for the authenticated D18 Chief daemon.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_daemon_api::{DaemonClient, DaemonClientError};
use chief_of_staff_service_registry::{
    DesiredState, HostName, HostRegistration, PackagePath, RegistryError, RestartPolicy,
};
use cli_builder::{load_spec_from_str, CliBuilderError, ParseResult, Parser, ParserOutput};
use coding_adventures_json_serializer::{serialize_pretty, JsonSerializerError, SerializerConfig};
use coding_adventures_json_value::JsonValue;
use core::fmt::{self, Display, Formatter};

const CLI_SPEC: &str = r#"{
  "cli_builder_spec_version": "1.0",
  "name": "chief",
  "description": "Operate the local D18 Chief daemon.",
  "version": "0.1.0",
  "commands": [
    {
      "id": "agents",
      "name": "agents",
      "description": "List registered host agents."
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
    }
  ]
}"#;

/// One complete successful parser outcome.
#[derive(Debug)]
pub enum CliAction {
    /// Generated help text; no daemon connection is needed.
    Help(String),
    /// Generated version text; no daemon connection is needed.
    Version(String),
    /// One validated operation for an already-authenticated daemon client.
    Command(CliCommand),
}

/// One typed operator command supported by the current daemon API.
#[derive(Clone, Debug, PartialEq, Eq)]
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
        _ => Err(CliError::InvalidInput {
            field: "command",
            message: "a supported subcommand is required",
        }),
    }
}

fn parse_register(result: &ParseResult) -> Result<CliCommand, CliError> {
    let host_name = host_name(result)?;
    let package_path =
        PackagePath::new(argument(result, "package_path")?).map_err(CliError::Registry)?;
    let package_hash = decode_package_hash(argument(result, "package_hash")?)?;
    let restart_policy = match flag(result, "restart")? {
        "always" => RestartPolicy::Always,
        "on_failure" => RestartPolicy::OnFailure,
        "never" => RestartPolicy::Never,
        _ => return Err(invalid_spec_value("restart")),
    };
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

fn invalid_spec_value(field: &'static str) -> CliError {
    CliError::InvalidInput {
        field,
        message: "declarative parser returned an unexpected value",
    }
}

fn decode_package_hash(value: &str) -> Result<[u8; 32], CliError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(CliError::InvalidInput {
            field: "package_hash",
            message: "must be exactly 64 lowercase hexadecimal characters",
        });
    }
    let mut hash = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        hash[index] = (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]);
    }
    Ok(hash)
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
    }

    #[test]
    fn execute_routes_every_command_and_preserves_typed_daemon_errors() {
        let mut client = RecordingClient {
            calls: Vec::new(),
            fail_next: false,
        };
        let hash = "0".repeat(64);
        let invocations = vec![
            argv(&["chief", "agents"]),
            argv(&["chief", "doctor", "alpha-host"]),
            argv(&["chief", "register", "alpha-host", "pkg/alpha", &hash]),
            argv(&["chief", "start", "alpha-host"]),
            argv(&["chief", "stop", "alpha-host"]),
            argv(&["chief", "reconcile"]),
            argv(&["chief", "deregister", "alpha-host"]),
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
}
