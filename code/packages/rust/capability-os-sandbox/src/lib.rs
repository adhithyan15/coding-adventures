#![forbid(unsafe_code)]

use std::fmt;

use capability_cage::{Action, Capability, Category, Manifest};
use coding_adventures_json_value::{parse, JsonValue};
use operation_primitives::{start_new, OperationError};

/// Host operating system family for sandbox lowering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OsFamily {
    Linux,
    Macos,
    Windows,
    FreeBsd,
    OpenBsd,
    Portable,
}

impl OsFamily {
    pub fn current() -> Self {
        if cfg!(target_os = "linux") {
            Self::Linux
        } else if cfg!(target_os = "macos") {
            Self::Macos
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "freebsd") {
            Self::FreeBsd
        } else if cfg!(target_os = "openbsd") {
            Self::OpenBsd
        } else {
            Self::Portable
        }
    }

    pub fn all_supported() -> [Self; 6] {
        [
            Self::Linux,
            Self::Macos,
            Self::Windows,
            Self::FreeBsd,
            Self::OpenBsd,
            Self::Portable,
        ]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Linux => "linux",
            Self::Macos => "macos",
            Self::Windows => "windows",
            Self::FreeBsd => "freebsd",
            Self::OpenBsd => "openbsd",
            Self::Portable => "portable",
        }
    }
}

impl fmt::Display for OsFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How strong the lowered primitive is for the manifest target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SandboxCoverage {
    Direct,
    Brokered,
    LaunchTime,
    Advisory,
}

impl SandboxCoverage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Brokered => "brokered",
            Self::LaunchTime => "launch_time",
            Self::Advisory => "advisory",
        }
    }
}

impl fmt::Display for SandboxCoverage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One OS-level primitive selected for one capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxRule {
    pub capability: Capability,
    pub os: OsFamily,
    pub primitive: String,
    pub expression: String,
    pub coverage: SandboxCoverage,
    pub note: String,
}

impl SandboxRule {
    pub fn capability_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.capability.category, self.capability.action, self.capability.target
        )
    }

    pub fn is_native(&self) -> bool {
        self.coverage != SandboxCoverage::Brokered || !self.primitive.contains("host_broker")
    }
}

/// Full lowering plan for one OS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxPlan {
    pub package: String,
    pub os: OsFamily,
    pub rules: Vec<SandboxRule>,
}

impl SandboxPlan {
    pub fn empty(package: impl Into<String>, os: OsFamily) -> Self {
        Self {
            package: package.into(),
            os,
            rules: Vec::new(),
        }
    }

    pub fn summary(&self) -> SandboxPlanSummary {
        let mut summary = SandboxPlanSummary {
            package: self.package.clone(),
            os: self.os,
            total_rules: self.rules.len(),
            ..SandboxPlanSummary::default()
        };
        for rule in &self.rules {
            match rule.coverage {
                SandboxCoverage::Direct => summary.direct_rules += 1,
                SandboxCoverage::Brokered => summary.brokered_rules += 1,
                SandboxCoverage::LaunchTime => summary.launch_time_rules += 1,
                SandboxCoverage::Advisory => summary.advisory_rules += 1,
            }
            if rule.primitive.contains("host_broker") {
                summary.host_broker_rules += 1;
            } else {
                summary.native_rules += 1;
            }
        }
        summary
    }

    pub fn has_primitive(&self, primitive: &str) -> bool {
        self.rules.iter().any(|rule| rule.primitive == primitive)
    }
}

/// Payload-light summary for end-to-end tests and audit records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxPlanSummary {
    pub package: String,
    pub os: OsFamily,
    pub total_rules: usize,
    pub direct_rules: usize,
    pub brokered_rules: usize,
    pub launch_time_rules: usize,
    pub advisory_rules: usize,
    pub native_rules: usize,
    pub host_broker_rules: usize,
}

impl Default for SandboxPlanSummary {
    fn default() -> Self {
        Self {
            package: String::new(),
            os: OsFamily::Portable,
            total_rules: 0,
            direct_rules: 0,
            brokered_rules: 0,
            launch_time_rules: 0,
            advisory_rules: 0,
            native_rules: 0,
            host_broker_rules: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxPlanError {
    Manifest(String),
    Operation(OperationError),
}

impl fmt::Display for SandboxPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manifest(message) => write!(f, "sandbox manifest error: {message}"),
            Self::Operation(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SandboxPlanError {}

/// Lower one manifest JSON document into one OS plan.
pub fn plan_from_json(manifest_json: &str, os: OsFamily) -> Result<SandboxPlan, SandboxPlanError> {
    let fallback = SandboxPlan::empty("<invalid>", os);
    start_new(
        "capability-os-sandbox.plan_from_json",
        fallback,
        |op, rf| {
            op.add_property("os", os.as_str());
            match build_plan_from_json(manifest_json, os) {
                Ok(plan) => {
                    op.add_property("package", &plan.package);
                    op.add_property("rules", plan.rules.len());
                    rf.succeed(plan)
                }
                Err(error) => rf.fail(SandboxPlan::empty("<invalid>", os), error.to_string()),
            }
        },
    )
    .get_result()
    .map_err(SandboxPlanError::Operation)
}

/// Lower one manifest JSON document for every supported OS family.
pub fn plan_all_supported(manifest_json: &str) -> Result<Vec<SandboxPlan>, SandboxPlanError> {
    OsFamily::all_supported()
        .into_iter()
        .map(|os| plan_from_json(manifest_json, os))
        .collect()
}

/// Lower one manifest JSON document for the current host OS.
pub fn plan_for_current_os(manifest_json: &str) -> Result<SandboxPlan, SandboxPlanError> {
    plan_from_json(manifest_json, OsFamily::current())
}

fn build_plan_from_json(
    manifest_json: &str,
    os: OsFamily,
) -> Result<SandboxPlan, SandboxPlanError> {
    let package = manifest_package(manifest_json)?;
    let manifest = Manifest::load_from_str(manifest_json)
        .map_err(|error| SandboxPlanError::Manifest(error.to_string()))?;
    let rules = manifest
        .capabilities()
        .iter()
        .map(|capability| lower_capability(capability, os))
        .collect();
    Ok(SandboxPlan { package, os, rules })
}

fn manifest_package(manifest_json: &str) -> Result<String, SandboxPlanError> {
    let root =
        parse(manifest_json).map_err(|error| SandboxPlanError::Manifest(error.to_string()))?;
    let object = match root {
        JsonValue::Object(pairs) => pairs,
        _ => {
            return Err(SandboxPlanError::Manifest(
                "top-level manifest value must be an object".to_string(),
            ))
        }
    };
    object
        .iter()
        .find_map(|(key, value)| {
            if key == "package" {
                match value {
                    JsonValue::String(package) => Some(package.clone()),
                    _ => Some(String::new()),
                }
            } else {
                None
            }
        })
        .filter(|package| !package.is_empty())
        .ok_or_else(|| {
            SandboxPlanError::Manifest("manifest package field must be a string".to_string())
        })
}

fn lower_capability(capability: &Capability, os: OsFamily) -> SandboxRule {
    let (primitive, coverage, note) = match os {
        OsFamily::Linux => lower_linux(capability),
        OsFamily::Macos => lower_macos(capability),
        OsFamily::Windows => lower_windows(capability),
        OsFamily::FreeBsd => lower_freebsd(capability),
        OsFamily::OpenBsd => lower_openbsd(capability),
        OsFamily::Portable => lower_portable(capability),
    };
    SandboxRule {
        capability: capability.clone(),
        os,
        primitive: primitive.to_string(),
        expression: expression_for(capability, primitive),
        coverage,
        note: note.to_string(),
    }
}

fn lower_linux(capability: &Capability) -> (&'static str, SandboxCoverage, &'static str) {
    match capability.category {
        Category::Fs if capability.target == "*" => (
            "linux.mount_namespace",
            SandboxCoverage::Advisory,
            "wildcard filesystem grants need a narrowed root or a host broker to become target-exact",
        ),
        Category::Fs => (
            "linux.landlock.path_beneath",
            SandboxCoverage::Direct,
            "Landlock can restrict file actions to declared paths after process start",
        ),
        Category::Net if capability.action == Action::Dns => (
            "linux.seccomp.brokered_resolver",
            SandboxCoverage::Brokered,
            "hostname resolution needs a broker when the manifest target is a DNS name",
        ),
        Category::Net => (
            "linux.cgroup_bpf.sock_addr",
            SandboxCoverage::Direct,
            "cgroup socket-address filters can restrict connect/listen targets",
        ),
        Category::Proc => (
            "linux.seccomp.process_syscalls",
            SandboxCoverage::Direct,
            "seccomp plus pid/user namespaces bound process creation and signalling",
        ),
        Category::Env => (
            "linux.execve.env_allowlist",
            SandboxCoverage::LaunchTime,
            "environment capabilities are enforced by constructing the child env block",
        ),
        Category::Ffi => (
            "linux.mount_namespace.library_view",
            SandboxCoverage::Direct,
            "library loading is reduced to the visible filesystem and executable mappings",
        ),
        Category::Time => (
            "linux.seccomp.clock_syscalls",
            SandboxCoverage::Advisory,
            "time reads can be syscall-shaped but not meaningfully target-scoped",
        ),
        Category::Stdin | Category::Stdout => (
            "linux.fd_table",
            SandboxCoverage::LaunchTime,
            "stdio authority is selected by the file descriptors inherited at spawn",
        ),
    }
}

fn lower_macos(capability: &Capability) -> (&'static str, SandboxCoverage, &'static str) {
    match capability.category {
        Category::Env => (
            "macos.posix_spawn.env_allowlist",
            SandboxCoverage::LaunchTime,
            "environment authority is selected when spawning the child",
        ),
        Category::Stdin | Category::Stdout => (
            "macos.posix_spawn.file_actions",
            SandboxCoverage::LaunchTime,
            "stdio authority is selected through inherited descriptors",
        ),
        Category::Time => (
            "macos.seatbelt.profile",
            SandboxCoverage::Advisory,
            "Seatbelt profiles help shape system access but do not target-scope time reads",
        ),
        _ => (
            "macos.seatbelt.profile",
            SandboxCoverage::Direct,
            "Seatbelt/App Sandbox profiles carry filesystem, network, process, and mapping policy",
        ),
    }
}

fn lower_windows(capability: &Capability) -> (&'static str, SandboxCoverage, &'static str) {
    match capability.category {
        Category::Fs => (
            "windows.appcontainer.acl",
            SandboxCoverage::Direct,
            "AppContainer identity plus ACLs can restrict file object access",
        ),
        Category::Net => (
            "windows.appcontainer.network_capability",
            SandboxCoverage::Direct,
            "AppContainer network capabilities and firewall policy shape socket authority",
        ),
        Category::Proc => (
            "windows.job_object.restricted_token",
            SandboxCoverage::Direct,
            "restricted tokens and job objects constrain child processes and signalling",
        ),
        Category::Env => (
            "windows.createprocess.environment_block",
            SandboxCoverage::LaunchTime,
            "the child environment block is built from the manifest allowlist",
        ),
        Category::Ffi => (
            "windows.process_mitigation.dll_policy",
            SandboxCoverage::Advisory,
            "DLL loading needs search-path and mitigation policy plus broker review",
        ),
        Category::Time => (
            "windows.restricted_token",
            SandboxCoverage::Advisory,
            "time reads are not target-scoped by Windows sandbox primitives",
        ),
        Category::Stdin | Category::Stdout => (
            "windows.handle_inheritance",
            SandboxCoverage::LaunchTime,
            "stdio authority is selected by inheritable handles at spawn",
        ),
    }
}

fn lower_freebsd(capability: &Capability) -> (&'static str, SandboxCoverage, &'static str) {
    match capability.category {
        Category::Fs | Category::Ffi => (
            "freebsd.capsicum.cap_rights",
            SandboxCoverage::Direct,
            "Capsicum preopens handles and limits rights before capability mode",
        ),
        Category::Net if capability.action == Action::Dns => (
            "freebsd.host_broker.resolver",
            SandboxCoverage::Brokered,
            "DNS names require a resolver broker when using Capsicum capability mode",
        ),
        Category::Net => (
            "freebsd.jail.vnet_firewall",
            SandboxCoverage::Direct,
            "jails with VNET/firewall rules shape socket authority",
        ),
        Category::Proc => (
            "freebsd.jail.rctl",
            SandboxCoverage::Direct,
            "jail and rctl policy constrain process authority",
        ),
        Category::Env => (
            "freebsd.posix_spawn.env_allowlist",
            SandboxCoverage::LaunchTime,
            "the child environment is built from the manifest allowlist",
        ),
        Category::Time => (
            "freebsd.capsicum.capability_mode",
            SandboxCoverage::Advisory,
            "capability mode narrows global access but not target-scoped time reads",
        ),
        Category::Stdin | Category::Stdout => (
            "freebsd.capsicum.fd_rights",
            SandboxCoverage::LaunchTime,
            "stdio authority is selected by inherited descriptors and Capsicum rights",
        ),
    }
}

fn lower_openbsd(capability: &Capability) -> (&'static str, SandboxCoverage, &'static str) {
    match capability.category {
        Category::Fs | Category::Ffi => (
            "openbsd.unveil",
            SandboxCoverage::Direct,
            "unveil can restrict visible filesystem paths and rights",
        ),
        Category::Env => (
            "openbsd.execve.env_allowlist",
            SandboxCoverage::LaunchTime,
            "the child environment is built from the manifest allowlist",
        ),
        Category::Stdin | Category::Stdout => (
            "openbsd.fd_inheritance",
            SandboxCoverage::LaunchTime,
            "stdio authority is selected by inherited descriptors",
        ),
        _ => (
            "openbsd.pledge",
            SandboxCoverage::Advisory,
            "pledge narrows syscall classes but does not target-scope hosts or PIDs",
        ),
    }
}

fn lower_portable(_capability: &Capability) -> (&'static str, SandboxCoverage, &'static str) {
    (
        "host_broker.json_rpc",
        SandboxCoverage::Brokered,
        "portable defense-in-depth routes the operation through a manifest-checking host broker",
    )
}

fn expression_for(capability: &Capability, primitive: &str) -> String {
    format!(
        "{} allow {}:{} target={}",
        primitive, capability.category, capability.action, capability.target
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const WEATHER_MANIFEST: &str = r#"{
      "version": 1,
      "package": "rust/weather-agent-e2e",
      "capabilities": [
        {
          "category": "net",
          "action": "dns",
          "target": "api.weather.gov",
          "justification": "Resolve Weather.gov for the live umbrella forecast."
        },
        {
          "category": "net",
          "action": "connect",
          "target": "api.weather.gov:443",
          "justification": "Fetch the Weather.gov points and forecast resources over TLS."
        },
        {
          "category": "fs",
          "action": "write",
          "target": "/tmp/umbrella-today.txt",
          "justification": "Write the umbrella decision text file."
        }
      ],
      "justification": "Weather Agent E2E fetches live weather and writes one report."
    }"#;

    #[test]
    fn empty_manifest_lowers_to_empty_plan() {
        let plan = plan_from_json(
            r#"{
              "version": 1,
              "package": "rust/pure",
              "capabilities": [],
              "justification": "Pure computation."
            }"#,
            OsFamily::Linux,
        )
        .unwrap();

        assert_eq!(plan.package, "rust/pure");
        assert_eq!(plan.rules.len(), 0);
        assert_eq!(plan.summary().total_rules, 0);
    }

    #[test]
    fn weather_manifest_lowers_to_linux_primitives() {
        let plan = plan_from_json(WEATHER_MANIFEST, OsFamily::Linux).unwrap();

        assert!(plan.has_primitive("linux.landlock.path_beneath"));
        assert!(plan.has_primitive("linux.cgroup_bpf.sock_addr"));
        assert!(plan.has_primitive("linux.seccomp.brokered_resolver"));
        assert_eq!(plan.summary().total_rules, 3);
        assert_eq!(plan.summary().direct_rules, 2);
        assert_eq!(plan.summary().brokered_rules, 1);
    }

    #[test]
    fn weather_manifest_lowers_to_macos_windows_and_portable() {
        let macos = plan_from_json(WEATHER_MANIFEST, OsFamily::Macos).unwrap();
        assert!(macos.has_primitive("macos.seatbelt.profile"));

        let windows = plan_from_json(WEATHER_MANIFEST, OsFamily::Windows).unwrap();
        assert!(windows.has_primitive("windows.appcontainer.network_capability"));
        assert!(windows.has_primitive("windows.appcontainer.acl"));

        let portable = plan_from_json(WEATHER_MANIFEST, OsFamily::Portable).unwrap();
        assert_eq!(portable.summary().brokered_rules, 3);
        assert_eq!(portable.summary().host_broker_rules, 3);
    }

    #[test]
    fn invalid_manifest_reports_operation_error_with_context() {
        let error = plan_from_json(
            r#"{"version":1,"package":"rust/bad","capabilities":"oops"}"#,
            OsFamily::Linux,
        )
        .unwrap_err();

        let message = error.to_string();
        assert!(message.contains("capabilities must be an array"));
        match error {
            SandboxPlanError::Operation(operation) => {
                assert_eq!(
                    operation.properties.get("os").map(String::as_str),
                    Some("linux")
                );
            }
            other => panic!("expected operation error, got {other:?}"),
        }
    }

    #[test]
    fn all_supported_plans_cover_six_os_families() {
        let plans = plan_all_supported(WEATHER_MANIFEST).unwrap();
        assert_eq!(plans.len(), 6);
        assert_eq!(plans[0].os, OsFamily::Linux);
        assert_eq!(plans[5].os, OsFamily::Portable);
    }
}
