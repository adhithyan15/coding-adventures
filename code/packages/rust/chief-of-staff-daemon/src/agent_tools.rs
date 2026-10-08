//! # The agent tool source: `net.fetch` and `vault.request_lease`
//!
//! D18V's daemon composition rules (V-D2 to V-D6), in one model-tool source.
//!
//! The smart-home source offers the same ten tools to every host and leaves
//! authorization to capability grants at call time. This source is the
//! opposite: **what a host is offered is computed from that host's own signed
//! manifest**, so two hosts on the same daemon see different surfaces.
//!
//! ```text
//!   HostPipelineBinding
//!     └─ registration: host_name, package_path, package_hash
//!          │
//!          ▼  verify_agent_package(package_path, keyring)        V-D2.1
//!          ▼  digest == package_hash                             V-D2.2
//!          ▼  parse_manifest, tier <= signing key's ceiling      V-D2.3
//!          ▼  HostProfile::from_manifest
//!          ▼  check_registration(tool) for each candidate        V-D2.4
//!          │     net.fetch            + a net:connect capability
//!          │     vault.request_lease  + a vault + vault_access leased|both
//!          ▼
//!   AgentSurface { host_name, tools, allowlist, vault_access }   (cached)
//! ```
//!
//! ## Why the cache is safe
//!
//! Verifying a package reads and hashes every file in it, and `definitions`
//! runs on every completion turn. So the derived surface is cached, keyed by
//! `(host_name, package_path, package_hash)`.
//!
//! The hash in the key is what makes this sound rather than merely fast. An
//! entry is only ever inserted after the package on disk was verified to have
//! *exactly that digest*, so the entry is a pure function of the key. Editing
//! the package afterwards changes the disk, not the key, and so cannot change
//! the surface. That is V-A2's "a host cannot widen its own allowlist by
//! editing its package after it was registered", for free.
//!
//! Failures are not cached: a host whose package fails verification is
//! re-checked on its next call, and offered nothing in the meantime.
//!
//! ## Failing closed means adding nothing
//!
//! This source can only *add* tools to a host's surface. When anything about
//! the package is wrong it therefore offers nothing, rather than failing the
//! whole composite: smart-home authorization does not depend on this
//! manifest, and refusing it too would turn "this host cannot fetch" into
//! "this host cannot do anything" for no security gain.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use chief_of_staff_agent_manifest::{parse_manifest, VaultAccess};
use chief_of_staff_host_control_protocol::{
    DataPlaneFailure, ModelToolCall, ModelToolDefinition, ModelToolResult,
};
use chief_of_staff_host_data_plane::ModelToolDispatcher;
use chief_of_staff_host_runtime::{
    verify_agent_package, HostProfile, HostProfileRuntime, PackageKeyring,
};
use chief_of_staff_net_fetch::{
    parse_request, tool_definition as net_fetch_definition, CredentialSource, FetchError,
    NetAllowlist, NetFetch, Resolver, Transport, NET_FETCH_TOOL_ID,
};
use chief_of_staff_pipeline_bindings::HostPipelineBinding;
use chief_of_staff_tool_api::{
    builtin_tool_definition, InMemoryToolRuntime, RequestedBy, ToolCallError, ToolDefinition,
    ToolErrorKind, ToolExecutionContext, ToolHandlerOutput, ToolInvocationRequest, ToolResult,
};
use chief_of_staff_vault_dispatch::{VaultToolBridge, VAULT_REQUEST_LEASE_TOOL_ID};
use chief_of_staff_vault_runtime::{
    ChiefVaultRuntime, VaultDirectDelivery, VaultDirectDeliveryError, VaultDirectRequest,
};
use coding_adventures_json_serializer::serialize as serialize_json;
use coding_adventures_json_value::{parse as parse_json, JsonNumber, JsonValue};
use coding_adventures_vault_leases::LeasePayload;
use coding_adventures_zeroize::Zeroizing;
use smart_home_core::VaultRef;

use crate::UnixTimeClock;

/// The most surfaces held at once. One per registered host is the steady
/// state; the bound only matters if registrations churn, and then clearing
/// and re-deriving is correct, just slower.
const MAX_CACHED_SURFACES: usize = 256;

/// What one host may use from this source, derived from its signed manifest.
struct AgentSurface {
    /// The registration's host name: the identity every call runs as (V-D3).
    host_name: String,
    /// The D18D definitions offered, in a fixed order.
    tools: Vec<ToolDefinition>,
    /// `Some` exactly when `net.fetch` is offered.
    allowlist: Option<NetAllowlist>,
    /// `Some` exactly when `vault.request_lease` is offered.
    vault_access: Option<VaultAccess>,
}

impl AgentSurface {
    /// The surface of a host this source offers nothing to.
    fn empty(host_name: String) -> Self {
        Self {
            host_name,
            tools: Vec::new(),
            allowlist: None,
            vault_access: None,
        }
    }

    fn offers(&self, tool_id: &str) -> bool {
        self.tools.iter().any(|tool| tool.tool_id == tool_id)
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct SurfaceKey {
    host_name: String,
    package_path: String,
    package_hash: [u8; 32],
}

/// The `net.fetch` + `vault.request_lease` model-tool source.
pub(crate) struct AgentModelTools<R: Resolver + 'static, T: Transport + 'static> {
    keyring: Arc<PackageKeyring>,
    vault: Option<Arc<ChiefVaultRuntime>>,
    fetch: Arc<NetFetch<R, T>>,
    clock: Arc<dyn UnixTimeClock>,
    surfaces: Mutex<HashMap<SurfaceKey, Arc<AgentSurface>>>,
}

impl<R: Resolver + 'static, T: Transport + 'static> AgentModelTools<R, T> {
    /// Compose the source. `vault` is `None` when no vault is configured, and
    /// then `vault.request_lease` is offered to nobody (V-D1).
    pub(crate) fn new(
        keyring: Arc<PackageKeyring>,
        vault: Option<Arc<ChiefVaultRuntime>>,
        fetch: NetFetch<R, T>,
        clock: Arc<dyn UnixTimeClock>,
    ) -> Self {
        Self {
            keyring,
            vault,
            fetch: Arc::new(fetch),
            clock,
            surfaces: Mutex::new(HashMap::new()),
        }
    }

    /// The binding's surface, from the cache or freshly derived.
    fn surface(
        &self,
        binding: &HostPipelineBinding,
    ) -> Result<Arc<AgentSurface>, DataPlaneFailure> {
        let registration = binding.registration();
        let key = SurfaceKey {
            host_name: registration.host_name().as_str().to_string(),
            package_path: registration.package_path().as_str().to_string(),
            package_hash: *registration.package_hash(),
        };
        if let Some(hit) = self
            .surfaces
            .lock()
            .map_err(|_| DataPlaneFailure::Internal)?
            .get(&key)
        {
            return Ok(Arc::clone(hit));
        }
        // Derived outside the lock: verification reads the whole package, and
        // holding the lock across that would serialize every host behind the
        // slowest disk read. Two threads racing here derive the same value
        // from the same verified bytes, so either insert is correct.
        let Some(surface) = self.derive(&key) else {
            return Ok(Arc::new(AgentSurface::empty(key.host_name)));
        };
        let surface = Arc::new(surface);
        let mut surfaces = self
            .surfaces
            .lock()
            .map_err(|_| DataPlaneFailure::Internal)?;
        if surfaces.len() >= MAX_CACHED_SURFACES {
            surfaces.clear();
        }
        surfaces.insert(key, Arc::clone(&surface));
        Ok(surface)
    }

    /// V-D2: verify, pin, parse, and run every candidate through the host
    /// runtime's own registration check. `None` means "offer nothing".
    fn derive(&self, key: &SurfaceKey) -> Option<AgentSurface> {
        let package = verify_agent_package(Path::new(&key.package_path), &self.keyring).ok()?;
        if package.digest() != key.package_hash {
            return None;
        }
        let manifest = parse_manifest(std::str::from_utf8(package.manifest_bytes()).ok()?).ok()?;
        let profile = HostProfile::from_manifest("chief-daemon-agent-tools", &manifest).ok()?;
        // The signing key caps the tier a package may claim. The host runtime
        // refuses a profile above that ceiling at spawn; a tool check against
        // the manifest's own claim would quietly skip it.
        if profile.max_tier > package.maximum_tier() {
            return None;
        }
        let checker = HostProfileRuntime::new(profile).ok()?;
        // A malformed `net` target is an error in the signed manifest, not an
        // entry to skip (V-A1), so it withholds the whole source.
        let allowlist = NetAllowlist::from_capabilities(&manifest.capabilities).ok()?;

        let mut surface = AgentSurface::empty(key.host_name.clone());
        if let Some(allowlist) = allowlist {
            let definition = net_fetch_definition();
            if checker.check_registration(&definition).is_ok() {
                surface.tools.push(definition);
                surface.allowlist = Some(allowlist);
            }
        }
        let leasable = manifest
            .vault_access
            .as_ref()
            .filter(|access| matches!(access.mode.as_str(), "leased" | "both"));
        if let (Some(_), Some(access)) = (&self.vault, leasable) {
            let definition = builtin_tool_definition(VAULT_REQUEST_LEASE_TOOL_ID)?;
            if checker.check_registration(&definition).is_ok() {
                surface.tools.push(definition);
                surface.vault_access = Some(access.clone());
            }
        }
        Some(surface)
    }

    /// Run one call through a fresh agent-surface runtime holding exactly the
    /// called tool, so schema validation and the output walk apply as they
    /// do for every other D18D tool.
    fn invoke(
        &self,
        surface: &Arc<AgentSurface>,
        call: &ModelToolCall,
        arguments: JsonValue,
    ) -> Result<ToolResult, DataPlaneFailure> {
        let definition = surface
            .tools
            .iter()
            .find(|tool| tool.tool_id == call.name)
            .cloned()
            .ok_or(DataPlaneFailure::Unauthorized)?;
        let mut runtime = InMemoryToolRuntime::agent_surface();
        let registered = match call.name.as_str() {
            NET_FETCH_TOOL_ID => {
                runtime.register_handler(definition, self.net_fetch_handler(surface))
            }
            VAULT_REQUEST_LEASE_TOOL_ID => {
                let handler = self
                    .lease_handler(surface)
                    .ok_or(DataPlaneFailure::Unauthorized)?;
                runtime.register_handler(definition, handler)
            }
            _ => return Err(DataPlaneFailure::Unauthorized),
        };
        registered.map_err(|_| DataPlaneFailure::Internal)?;
        let requested_at = self.clock.now_ms().ok_or(DataPlaneFailure::Internal)?;
        Ok(runtime.invoke(&ToolInvocationRequest {
            call_id: call.call_id.clone(),
            tool_id: call.name.clone(),
            arguments,
            requested_by: RequestedBy::Agent,
            session_id: None,
            job_id: None,
            // V-D3: the registration's name, never anything from the call.
            agent_id: Some(surface.host_name.clone()),
            user_id: None,
            requested_at,
            deadline_at: None,
            idempotency_key: None,
        }))
    }

    /// `net.fetch`, bound to this host's allowlist and identity.
    ///
    /// The identity is captured here from the surface, not read from the
    /// execution context, so nothing on the call path can change whose
    /// leases this fetch may redeem.
    fn net_fetch_handler(
        &self,
        surface: &Arc<AgentSurface>,
    ) -> impl Fn(JsonValue, ToolExecutionContext) -> Result<ToolHandlerOutput, ToolCallError> + 'static
    {
        let fetch = Arc::clone(&self.fetch);
        let surface = Arc::clone(surface);
        let vault = self.vault.clone();
        move |arguments, _context| {
            let allowlist = surface
                .allowlist
                .as_ref()
                .ok_or_else(|| fetch_error(FetchError::Unauthorized))?;
            let request = parse_request(&arguments).map_err(fetch_error)?;
            let source = vault.as_ref().map(|vault| VaultCredentials {
                vault: Arc::clone(vault),
                host_name: surface.host_name.clone(),
            });
            let response = fetch
                .execute(
                    allowlist,
                    &request,
                    source
                        .as_ref()
                        .map(|source| source as &dyn CredentialSource),
                )
                .map_err(fetch_error)?;
            Ok(ToolHandlerOutput::new(response.to_json()))
        }
    }

    /// `vault.request_lease`, narrowed by the manifest's `vault_access`
    /// (V-D4) before the vault's own policy sees it.
    fn lease_handler(
        &self,
        surface: &Arc<AgentSurface>,
    ) -> Option<
        impl Fn(JsonValue, ToolExecutionContext) -> Result<ToolHandlerOutput, ToolCallError> + 'static,
    > {
        let vault = Arc::clone(self.vault.as_ref()?);
        let access = surface.vault_access.clone()?;
        let inner = VaultToolBridge::new(vault, Arc::new(NoDirectDelivery)).lease_handler();
        Some(move |arguments: JsonValue, context: ToolExecutionContext| {
            if !lease_declared(&access, &arguments) {
                return Err(ToolCallError::new(
                    ToolErrorKind::ToolPermissionDenied,
                    "the agent manifest does not declare this lease",
                ));
            }
            inner(arguments, context)
        })
    }
}

/// V-D4: the secret is declared in `vault_access.secrets` and the TTL is
/// within `max_lease_ttl` seconds.
///
/// Anything unreadable is "not declared". The schema has already been
/// checked by the time a handler runs, so this only refuses what the vault
/// would refuse too, but it must not be the step that lets a malformed call
/// through.
fn lease_declared(access: &VaultAccess, arguments: &JsonValue) -> bool {
    let JsonValue::Object(fields) = arguments else {
        return false;
    };
    let field = |name: &str| {
        fields
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    };
    let Some(JsonValue::String(secret_name)) = field("secret_name") else {
        return false;
    };
    let Some(JsonValue::Number(JsonNumber::Integer(ttl_ms))) = field("ttl_ms") else {
        return false;
    };
    let max_ttl_ms = i64::from(access.max_lease_ttl) * 1_000;
    access
        .secrets
        .iter()
        .any(|declared| declared == secret_name)
        && *ttl_ms <= max_ttl_ms
}

/// The D18V error, as a D18D tool error (V-D6).
///
/// `message` is fixed text and `details.reason` is the bounded D18V kind, so
/// the model can tell a refused URL from a timeout and nothing from the
/// request or response rides along.
fn fetch_error(error: FetchError) -> ToolCallError {
    let kind = match error {
        FetchError::Unauthorized | FetchError::CredentialRefused | FetchError::AddressRefused => {
            ToolErrorKind::ToolPermissionDenied
        }
        FetchError::InvalidRequest(_) => ToolErrorKind::ToolValidationError,
        FetchError::Timeout => ToolErrorKind::ToolTimeout,
        FetchError::Network | FetchError::Tls | FetchError::Protocol | FetchError::BinaryBody => {
            ToolErrorKind::ToolExecutionError
        }
    };
    let mut error_value = ToolCallError::new(kind, error.to_string());
    error_value.details = JsonValue::Object(vec![(
        "reason".to_string(),
        JsonValue::String(error.kind().to_string()),
    )]);
    error_value
}

/// V-D5: redemption through `consume_for`, as the registration's host.
struct VaultCredentials {
    vault: Arc<ChiefVaultRuntime>,
    host_name: String,
}

impl CredentialSource for VaultCredentials {
    fn redeem(&self, vault_ref: &str, destination: &str) -> Result<Zeroizing<Vec<u8>>, FetchError> {
        // Every refusal is the same kind: which one it was would tell the
        // model something about a lease it may not hold.
        let vault_ref = VaultRef::new(vault_ref).map_err(|_| FetchError::CredentialRefused)?;
        let payload: LeasePayload = self
            .vault
            .consume_for(&vault_ref, &self.host_name, destination)
            .map_err(|_| FetchError::CredentialRefused)?;
        Ok(Zeroizing::new(payload.as_bytes().to_vec()))
    }
}

/// The bridge needs a direct-delivery adapter to exist, and this source never
/// offers `vault.request_direct`. This one refuses everything, so even a
/// future wiring mistake that reached it could not deliver a secret.
struct NoDirectDelivery;

impl VaultDirectDelivery for NoDirectDelivery {
    fn deliver(
        &self,
        _request: VaultDirectRequest<'_>,
        _payload: LeasePayload,
    ) -> Result<(), VaultDirectDeliveryError> {
        Err(VaultDirectDeliveryError::Rejected)
    }
}

impl<R: Resolver + 'static, T: Transport + 'static> ModelToolDispatcher for AgentModelTools<R, T> {
    fn definitions(
        &self,
        binding: &HostPipelineBinding,
    ) -> Result<Vec<ModelToolDefinition>, DataPlaneFailure> {
        self.surface(binding)?
            .tools
            .iter()
            .map(|definition| {
                let schema = serialize_json(&definition.input_json_schema())
                    .map_err(|_| DataPlaneFailure::Internal)?;
                Ok(ModelToolDefinition {
                    name: definition.tool_id.clone(),
                    description: definition.description.clone(),
                    input_schema: serde_json::from_str(&schema)
                        .map_err(|_| DataPlaneFailure::Internal)?,
                })
            })
            .collect()
    }

    fn execute(
        &self,
        binding: &HostPipelineBinding,
        call: &ModelToolCall,
    ) -> Result<ModelToolResult, DataPlaneFailure> {
        let surface = self.surface(binding)?;
        if !surface.offers(&call.name) {
            return Err(DataPlaneFailure::Unauthorized);
        }
        let arguments_json =
            serde_json::to_string(&call.arguments).map_err(|_| DataPlaneFailure::InvalidRequest)?;
        let arguments =
            parse_json(&arguments_json).map_err(|_| DataPlaneFailure::InvalidRequest)?;
        let result = self.invoke(&surface, call, arguments)?;
        let (output, is_error) = if result.ok {
            (result.output.unwrap_or(JsonValue::Null), false)
        } else {
            let error = result.error.ok_or(DataPlaneFailure::Internal)?;
            (
                JsonValue::Object(vec![
                    (
                        "kind".to_string(),
                        JsonValue::String(error.kind.to_string()),
                    ),
                    ("message".to_string(), JsonValue::String(error.message)),
                    ("details".to_string(), error.details),
                ]),
                true,
            )
        };
        let output_json = serialize_json(&output).map_err(|_| DataPlaneFailure::Internal)?;
        Ok(ModelToolResult {
            call: call.clone(),
            output: serde_json::from_str(&output_json).map_err(|_| DataPlaneFailure::Internal)?,
            is_error,
        })
    }
}

#[cfg(test)]
mod tests {
    //! The surface rules (V-D2), identity (V-D3), the manifest's narrowing of
    //! leases (V-D4), redemption (V-D5) and error shape (V-D6), all through
    //! the public `ModelToolDispatcher` face, over a real signed package and
    //! a fake network.

    use super::*;
    use crate::SystemUnixTimeClock;
    use chief_of_staff_channel_endpoints::AgentId as ChannelAgentId;
    use chief_of_staff_host_control_protocol::{LaunchBindings, LevelOneModelBinding};
    use chief_of_staff_host_runtime::{sign_agent_package, PackageKeyType, TrustedPackageKey};
    use chief_of_staff_net_fetch::{read_limited, Limits};
    use chief_of_staff_pipeline_bindings::PipelineId;
    use chief_of_staff_service_registry::{HostName, HostRegistration, PackagePath, RestartPolicy};
    use chief_of_staff_tool_api::PrivilegeTier;
    use chief_of_staff_vault_runtime::{AllowedAgents, SecretPolicy, VaultDeliveryMode};
    use std::collections::BTreeSet;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const SECRET: &str = "s3cret-token-123";
    const PRODUCTION_KEY: &str = "agent-tools-prod";
    const DEVELOPER_KEY: &str = "agent-tools-dev";

    // ── Fixtures ─────────────────────────────────────────────────────────

    /// A signed package on disk, removed on drop.
    struct Package {
        path: PathBuf,
        digest: [u8; 32],
    }

    impl Drop for Package {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    static NEXT_PACKAGE: AtomicUsize = AtomicUsize::new(0);

    fn key_seed(key_id: &str) -> [u8; 32] {
        if key_id == PRODUCTION_KEY {
            [41; 32]
        } else {
            [42; 32]
        }
    }

    fn keyring() -> Arc<PackageKeyring> {
        let mut keyring = PackageKeyring::new();
        for (key_id, key_type, tier) in [
            (
                PRODUCTION_KEY,
                PackageKeyType::Production,
                PrivilegeTier::Tier3,
            ),
            (
                DEVELOPER_KEY,
                PackageKeyType::Developer,
                PrivilegeTier::Tier1,
            ),
        ] {
            let (public_key, _) = coding_adventures_ed25519::generate_keypair(&key_seed(key_id));
            keyring
                .trust(TrustedPackageKey::new(key_id, key_type, public_key, tier).unwrap())
                .unwrap();
        }
        Arc::new(keyring)
    }

    fn package(manifest: &str, key_id: &str) -> Package {
        let path = std::env::temp_dir().join(format!(
            "chief-agent-tools-{}-{}",
            std::process::id(),
            NEXT_PACKAGE.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("manifest.json"), manifest).unwrap();
        std::fs::write(path.join("SKILL.md"), "# Fixture\n").unwrap();
        let (_, secret_key) = coding_adventures_ed25519::generate_keypair(&key_seed(key_id));
        let digest = sign_agent_package(&path, key_id, &secret_key).unwrap();
        Package { path, digest }
    }

    /// A v4 manifest. Each argument is spliced in as raw JSON.
    fn manifest(tier: u8, tools: &str, tool_capabilities: &str, net: &str, vault: &str) -> String {
        let vault = if vault.is_empty() {
            String::new()
        } else {
            format!(r#""vault_access": {vault},"#)
        };
        format!(
            r#"{{
              "version": 4,
              "agent": "weather-agent",
              "description": "Reports the weather.",
              "privilege_tier": {tier},
              "channels": {{"reads": {{"weather-requests": 1}}, "writes": {{"weather-reports": 1}}}},
              {vault}
              "capabilities": {net},
              "allowed_tools": {tools},
              "tool_capabilities": {tool_capabilities},
              "justification": "Fetches a forecast."
            }}"#
        )
    }

    const WEATHER_NET: &str = r#"[
      {"category": "net", "action": "dns", "target": "api.weather.gov", "justification": "Reads the public forecast."},
      {"category": "net", "action": "connect", "target": "api.weather.gov:443", "justification": "Reads the public forecast."}
    ]"#;
    const LEASED: &str = r#"{"secrets": ["weather-key"], "mode": "leased", "max_lease_ttl": 60}"#;

    /// The fetch-only weather agent: Tier 1, developer-signed.
    fn fetch_only() -> Package {
        package(
            &manifest(1, r#"["net.fetch"]"#, r#"["net:connect"]"#, WEATHER_NET, ""),
            DEVELOPER_KEY,
        )
    }

    /// A Tier 2, production-signed agent that may also lease `weather-key`.
    fn leasing(vault: &str) -> Package {
        package(
            &manifest(
                2,
                r#"["net.fetch", "vault.request_lease"]"#,
                r#"["net:connect", "vault:lease"]"#,
                WEATHER_NET,
                vault,
            ),
            PRODUCTION_KEY,
        )
    }

    fn binding(host_name: &str, package: &Package) -> HostPipelineBinding {
        binding_with_hash(host_name, package, package.digest)
    }

    fn binding_with_hash(
        host_name: &str,
        package: &Package,
        hash: [u8; 32],
    ) -> HostPipelineBinding {
        let mut pipeline_id = [0; 16];
        pipeline_id[6] = 0x70;
        pipeline_id[8] = 0x80;
        HostPipelineBinding::new(
            PipelineId::new(pipeline_id).unwrap(),
            HostRegistration::new(
                HostName::new(host_name).unwrap(),
                PackagePath::new(package.path.to_str().unwrap()).unwrap(),
                hash,
                RestartPolicy::Always,
            ),
            ChannelAgentId::new(b"weather-agent".to_vec()).unwrap(),
            LaunchBindings::new(
                Vec::new(),
                Some(LevelOneModelBinding::new("test-model", 0.0, 128).unwrap()),
            )
            .unwrap(),
        )
    }

    struct PublicResolver;

    impl Resolver for PublicResolver {
        fn resolve(&self, _host: &str, port: u16) -> std::io::Result<Vec<SocketAddr>> {
            Ok(vec![SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
                port,
            )])
        }
    }

    /// Records every request and answers each with `response`.
    #[derive(Clone)]
    struct Recorder {
        response: Arc<Vec<u8>>,
        sent: Arc<Mutex<Vec<Vec<u8>>>>,
    }

    impl Recorder {
        fn echoing() -> Self {
            let body = format!("you sent Bearer {SECRET}");
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: {}\r\n\r\n{body}",
                body.len()
            );
            Self {
                response: Arc::new(response.into_bytes()),
                sent: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn requests(&self) -> Vec<String> {
            self.sent
                .lock()
                .unwrap()
                .iter()
                .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
                .collect()
        }
    }

    impl Transport for Recorder {
        fn exchange(
            &self,
            _server_name: &str,
            _address: SocketAddr,
            request: &[u8],
            limits: &Limits,
        ) -> Result<Zeroizing<Vec<u8>>, FetchError> {
            self.sent.lock().unwrap().push(request.to_vec());
            read_limited(&mut self.response.as_slice(), limits.max_wire_bytes)
        }
    }

    fn vault_with(destinations: &[&str], agents: &[&str]) -> Arc<ChiefVaultRuntime> {
        let vault = ChiefVaultRuntime::new();
        vault.register_secret(
            "weather-key",
            LeasePayload::new(SECRET.as_bytes().to_vec()),
            SecretPolicy {
                privilege_tier: 2,
                allowed_agents: AllowedAgents::only(agents.iter().copied()),
                allowed_mode: VaultDeliveryMode::Leased,
                rotated_at_ms: 0,
                allowed_destinations: destinations
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<BTreeSet<_>>(),
            },
        );
        Arc::new(vault)
    }

    fn tools(
        vault: Option<Arc<ChiefVaultRuntime>>,
        recorder: &Recorder,
    ) -> AgentModelTools<PublicResolver, Recorder> {
        AgentModelTools::new(
            keyring(),
            vault,
            NetFetch::new(PublicResolver, recorder.clone()),
            Arc::new(SystemUnixTimeClock),
        )
    }

    fn offered(
        tools: &AgentModelTools<PublicResolver, Recorder>,
        binding: &HostPipelineBinding,
    ) -> Vec<String> {
        tools
            .definitions(binding)
            .unwrap()
            .into_iter()
            .map(|definition| definition.name)
            .collect()
    }

    fn call(name: &str, arguments: serde_json::Value) -> ModelToolCall {
        ModelToolCall {
            call_id: "call-1".to_string(),
            name: name.to_string(),
            arguments,
        }
    }

    fn lease(
        tools: &AgentModelTools<PublicResolver, Recorder>,
        binding: &HostPipelineBinding,
    ) -> String {
        let result = tools
            .execute(
                binding,
                &call(
                    VAULT_REQUEST_LEASE_TOOL_ID,
                    serde_json::json!({"secret_name": "weather-key", "ttl_ms": 30_000}),
                ),
            )
            .unwrap();
        assert!(!result.is_error, "{:?}", result.output);
        result.output["vault_ref"].as_str().unwrap().to_string()
    }

    fn fetch(vault_ref: Option<&str>) -> ModelToolCall {
        let mut arguments = serde_json::json!({
            "url": "https://api.weather.gov/points/1,2",
            "method": "GET",
        });
        if let Some(vault_ref) = vault_ref {
            arguments["credential"] = serde_json::json!({
                "vault_ref": vault_ref,
                "header": "authorization",
                "scheme": "Bearer",
            });
        }
        call(NET_FETCH_TOOL_ID, arguments)
    }

    fn reason(result: &ModelToolResult) -> &str {
        assert!(result.is_error, "{:?}", result.output);
        result.output["details"]["reason"].as_str().unwrap()
    }

    // ── V-D2: the surface ────────────────────────────────────────────────

    #[test]
    fn a_fetching_agent_is_offered_net_fetch_and_nothing_else() {
        let recorder = Recorder::echoing();
        let package = fetch_only();
        let tools = tools(Some(vault_with(&[], &[])), &recorder);
        assert_eq!(
            offered(&tools, &binding("weather", &package)),
            [NET_FETCH_TOOL_ID]
        );
    }

    #[test]
    fn a_leasing_agent_is_offered_the_lease_tool_only_when_a_vault_exists() {
        let recorder = Recorder::echoing();
        let package = leasing(LEASED);
        let binding = binding("weather", &package);
        assert_eq!(
            offered(&tools(Some(vault_with(&[], &[])), &recorder), &binding),
            [NET_FETCH_TOOL_ID, VAULT_REQUEST_LEASE_TOOL_ID]
        );
        assert_eq!(
            offered(&tools(None, &recorder), &binding),
            [NET_FETCH_TOOL_ID]
        );
    }

    #[test]
    fn the_lease_tool_needs_a_leasable_vault_access_declaration() {
        let recorder = Recorder::echoing();
        let tools = tools(Some(vault_with(&[], &[])), &recorder);
        for vault in [
            "",
            r#"{"secrets": ["weather-key"], "mode": "direct", "max_lease_ttl": 60}"#,
        ] {
            let package = leasing(vault);
            assert_eq!(
                offered(&tools, &binding("weather", &package)),
                [NET_FETCH_TOOL_ID],
                "vault_access {vault:?}"
            );
        }
        let package =
            leasing(r#"{"secrets": ["weather-key"], "mode": "both", "max_lease_ttl": 60}"#);
        assert_eq!(offered(&tools, &binding("weather", &package)).len(), 2);
    }

    #[test]
    fn every_registration_check_withholds_net_fetch() {
        let recorder = Recorder::echoing();
        let tools = tools(None, &recorder);
        let cases = [
            // Not in allowed_tools.
            manifest(
                1,
                r#"["artifact.write"]"#,
                r#"["net:connect"]"#,
                WEATHER_NET,
                "",
            ),
            // Tool capability not granted.
            manifest(1, r#"["net.fetch"]"#, "[]", WEATHER_NET, ""),
            // Tier below the tool's Tier 1.
            manifest(0, r#"["net.fetch"]"#, r#"["net:connect"]"#, WEATHER_NET, ""),
            // No net:connect capability at all (V-A3).
            manifest(1, r#"["net.fetch"]"#, r#"["net:connect"]"#, "[]", ""),
        ];
        for manifest in cases {
            let package = package(&manifest, DEVELOPER_KEY);
            assert!(
                offered(&tools, &binding("weather", &package)).is_empty(),
                "{manifest}"
            );
        }
    }

    #[test]
    fn a_package_that_fails_verification_is_offered_nothing() {
        let recorder = Recorder::echoing();
        let tools = tools(Some(vault_with(&[], &[])), &recorder);

        // The registration pins a different digest (V-D2.2).
        let pinned = fetch_only();
        let pinned_elsewhere = binding_with_hash("weather", &pinned, [9; 32]);
        assert!(offered(&tools, &pinned_elsewhere).is_empty());
        assert_eq!(
            tools.execute(&pinned_elsewhere, &fetch(None)).unwrap_err(),
            DataPlaneFailure::Unauthorized
        );
        assert!(recorder.requests().is_empty());

        // A Tier 2 manifest signed with a developer key, capped at Tier 1.
        let over_ceiling = package(
            &manifest(2, r#"["net.fetch"]"#, r#"["net:connect"]"#, WEATHER_NET, ""),
            DEVELOPER_KEY,
        );
        assert!(offered(&tools, &binding("weather", &over_ceiling)).is_empty());

        // A malformed net target is an error, not a skipped entry (V-A1).
        let wildcard = package(
            &manifest(
                1,
                r#"["net.fetch"]"#,
                r#"["net:connect"]"#,
                r#"[{"category": "net", "action": "connect", "target": "*.weather.gov:443", "justification": "Reads any weather host."}]"#,
                "",
            ),
            DEVELOPER_KEY,
        );
        assert!(offered(&tools, &binding("weather", &wildcard)).is_empty());
    }

    #[test]
    fn an_edit_after_registration_cannot_change_a_cached_surface() {
        let recorder = Recorder::echoing();
        let tools = tools(None, &recorder);
        let package = fetch_only();
        let registered = binding("weather", &package);
        assert_eq!(offered(&tools, &registered), [NET_FETCH_TOOL_ID]);

        // Widen the manifest on disk. The registration still pins the old
        // digest, so the surface stays exactly what was verified.
        std::fs::write(
            package.path.join("manifest.json"),
            manifest(1, r#"["net.fetch"]"#, r#"["net:connect"]"#, "[]", ""),
        )
        .unwrap();
        assert_eq!(offered(&tools, &registered), [NET_FETCH_TOOL_ID]);
    }

    // ── V-D5, V-D3: redemption and identity ──────────────────────────────

    #[test]
    fn a_leased_credential_is_sent_once_and_masked_in_the_response() {
        let recorder = Recorder::echoing();
        let tools = tools(
            Some(vault_with(&["api.weather.gov:443"], &["weather"])),
            &recorder,
        );
        let package = leasing(LEASED);
        let binding = binding("weather", &package);

        let vault_ref = lease(&tools, &binding);
        let result = tools.execute(&binding, &fetch(Some(&vault_ref))).unwrap();
        assert!(!result.is_error, "{:?}", result.output);
        assert_eq!(result.output["status"], 200);
        let body = result.output["body"].as_str().unwrap();
        assert!(!body.contains(SECRET), "{body}");
        assert!(body.starts_with("you sent "));

        let requests = recorder.requests();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].contains(&format!("authorization: Bearer {SECRET}\r\n")));

        // Single use: the same reference is refused, and nothing is sent.
        let again = tools.execute(&binding, &fetch(Some(&vault_ref))).unwrap();
        assert_eq!(reason(&again), "credential_refused");
        assert_eq!(recorder.requests().len(), 1);
    }

    #[test]
    fn another_host_cannot_redeem_a_lease_and_the_owner_keeps_it() {
        let recorder = Recorder::echoing();
        let tools = tools(
            Some(vault_with(
                &["api.weather.gov:443"],
                &["weather", "intruder"],
            )),
            &recorder,
        );
        let package = leasing(LEASED);
        let owner = binding("weather", &package);
        let intruder = binding("intruder", &package);

        let vault_ref = lease(&tools, &owner);
        let stolen = tools.execute(&intruder, &fetch(Some(&vault_ref))).unwrap();
        assert_eq!(reason(&stolen), "credential_refused");
        assert!(recorder.requests().is_empty());

        let used = tools.execute(&owner, &fetch(Some(&vault_ref))).unwrap();
        assert!(!used.is_error, "{:?}", used.output);
    }

    #[test]
    fn a_secret_is_never_sent_to_a_destination_its_record_does_not_name() {
        let recorder = Recorder::echoing();
        let tools = tools(
            Some(vault_with(&["api.example.com:443"], &["weather"])),
            &recorder,
        );
        let package = leasing(LEASED);
        let binding = binding("weather", &package);

        let vault_ref = lease(&tools, &binding);
        let result = tools.execute(&binding, &fetch(Some(&vault_ref))).unwrap();
        assert_eq!(reason(&result), "credential_refused");
        assert!(recorder.requests().is_empty());
    }

    #[test]
    fn a_credential_without_a_vault_is_refused() {
        let recorder = Recorder::echoing();
        let tools = tools(None, &recorder);
        let package = fetch_only();
        let result = tools
            .execute(
                &binding("weather", &package),
                &fetch(Some("vault-lease:00")),
            )
            .unwrap();
        assert_eq!(reason(&result), "credential_refused");
        assert!(recorder.requests().is_empty());
    }

    // ── V-D4: the manifest narrows leases ────────────────────────────────

    #[test]
    fn a_lease_outside_the_manifest_declaration_is_refused() {
        let recorder = Recorder::echoing();
        let tools = tools(
            Some(vault_with(&["api.weather.gov:443"], &["weather"])),
            &recorder,
        );
        let package = leasing(LEASED);
        let binding = binding("weather", &package);
        for arguments in [
            serde_json::json!({"secret_name": "other-key", "ttl_ms": 30_000}),
            serde_json::json!({"secret_name": "weather-key", "ttl_ms": 60_001}),
        ] {
            let result = tools
                .execute(
                    &binding,
                    &call(VAULT_REQUEST_LEASE_TOOL_ID, arguments.clone()),
                )
                .unwrap();
            assert!(result.is_error, "{arguments}");
            assert_eq!(result.output["kind"], "ToolPermissionDenied", "{arguments}");
        }
        // The boundary itself is inside the declaration.
        let at_limit = tools
            .execute(
                &binding,
                &call(
                    VAULT_REQUEST_LEASE_TOOL_ID,
                    serde_json::json!({"secret_name": "weather-key", "ttl_ms": 60_000}),
                ),
            )
            .unwrap();
        assert!(!at_limit.is_error, "{:?}", at_limit.output);
    }

    // ── V-D6: refusals are tool results ──────────────────────────────────

    #[test]
    fn a_url_outside_the_manifest_is_a_tool_error_and_sends_nothing() {
        let recorder = Recorder::echoing();
        let tools = tools(None, &recorder);
        let package = fetch_only();
        let result = tools
            .execute(
                &binding("weather", &package),
                &call(
                    NET_FETCH_TOOL_ID,
                    serde_json::json!({"url": "https://evil.example/", "method": "GET"}),
                ),
            )
            .unwrap();
        assert_eq!(reason(&result), "unauthorized");
        assert_eq!(result.output["kind"], "ToolPermissionDenied");
        assert!(recorder.requests().is_empty());
    }

    #[test]
    fn a_tool_this_source_did_not_offer_is_unauthorized() {
        let recorder = Recorder::echoing();
        let tools = tools(Some(vault_with(&[], &[])), &recorder);
        let package = fetch_only();
        let binding = binding("weather", &package);
        for name in [
            VAULT_REQUEST_LEASE_TOOL_ID,
            "vault.request_direct",
            "smart_home.discover",
        ] {
            assert_eq!(
                tools
                    .execute(&binding, &call(name, serde_json::json!({})))
                    .unwrap_err(),
                DataPlaneFailure::Unauthorized,
                "{name}"
            );
        }
    }

    #[test]
    fn an_unauthenticated_fetch_returns_the_response() {
        let recorder = Recorder::echoing();
        let tools = tools(None, &recorder);
        let package = fetch_only();
        let result = tools
            .execute(&binding("weather", &package), &fetch(None))
            .unwrap();
        assert!(!result.is_error, "{:?}", result.output);
        assert_eq!(result.output["truncated"], false);
        let requests = recorder.requests();
        assert!(requests[0].starts_with("GET /points/1,2 HTTP/1.1\r\nhost: api.weather.gov\r\n"));
        assert!(!requests[0].contains("authorization"));
    }
}
