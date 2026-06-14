#![forbid(unsafe_code)]

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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

const MACOS_SANDBOX_EXEC: &str = "/usr/bin/sandbox-exec";
const MACOS_KERNEL_PRIMITIVE: &str = "macos.sandbox-exec.seatbelt";
const MACOS_MDNSRESPONDER_SOCKET: &str = "/private/var/run/mDNSResponder";

/// Whether the current host has a kernel sandbox applier for the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelSandboxSupport {
    pub os: OsFamily,
    pub primitive: String,
    pub available: bool,
    pub reason: String,
}

/// Process output from a command launched under an OS kernel sandbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelSandboxCommandOutput {
    pub os: OsFamily,
    pub primitive: String,
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl KernelSandboxCommandOutput {
    pub fn success(&self) -> bool {
        self.status_code == Some(0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelSandboxError {
    Unsupported(KernelSandboxSupport),
    InvalidPlan(String),
    Io(String),
}

impl fmt::Display for KernelSandboxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(support) => write!(
                f,
                "kernel sandbox unsupported on {} via {}: {}",
                support.os, support.primitive, support.reason
            ),
            Self::InvalidPlan(message) => write!(f, "invalid kernel sandbox plan: {message}"),
            Self::Io(message) => write!(f, "kernel sandbox launcher I/O error: {message}"),
        }
    }
}

impl std::error::Error for KernelSandboxError {}

/// Report whether this host can apply the current OS plan in the kernel.
pub fn current_kernel_sandbox_support() -> KernelSandboxSupport {
    match OsFamily::current() {
        OsFamily::Macos => {
            let available = Path::new(MACOS_SANDBOX_EXEC).exists();
            KernelSandboxSupport {
                os: OsFamily::Macos,
                primitive: MACOS_KERNEL_PRIMITIVE.to_string(),
                available,
                reason: if available {
                    "sandbox-exec is available for Seatbelt profile application".to_string()
                } else {
                    "sandbox-exec was not found at /usr/bin/sandbox-exec".to_string()
                },
            }
        }
        os => KernelSandboxSupport {
            os,
            primitive: "not-yet-implemented".to_string(),
            available: false,
            reason: "this crate currently applies kernel sandboxes only through macOS Seatbelt"
                .to_string(),
        },
    }
}

/// Generate the macOS Seatbelt profile used to enforce a macOS sandbox plan.
///
/// V1 enforces filesystem write/create/delete capabilities as exact absolute
/// path literals and outbound network capabilities as resolver-socket plus
/// TCP-port policy. macOS Seatbelt cannot target arbitrary remote hostnames,
/// so host-exact checks remain paired with TLS/application validation.
pub fn macos_seatbelt_profile_for_plan(plan: &SandboxPlan) -> Result<String, KernelSandboxError> {
    if plan.os != OsFamily::Macos {
        return Err(KernelSandboxError::InvalidPlan(format!(
            "expected a macOS plan, got {}",
            plan.os
        )));
    }

    let mut writable_paths = writable_path_literals(plan)?;
    writable_paths.sort();
    writable_paths.dedup();

    let mut profile = String::from("(version 1)\n(allow default)\n");
    match writable_paths.as_slice() {
        [] => profile.push_str("(deny file-write*)\n"),
        [path] => {
            profile.push_str("(deny file-write* (require-not (literal \"");
            profile.push_str(&seatbelt_escape(path)?);
            profile.push_str("\")))\n");
        }
        paths => {
            profile.push_str("(deny file-write* (require-not (require-any");
            for path in paths {
                profile.push_str(" (literal \"");
                profile.push_str(&seatbelt_escape(path)?);
                profile.push_str("\")");
            }
            profile.push_str(")))\n");
        }
    }
    match macos_network_policy(plan)? {
        MacosNetworkPolicy::Unrestricted => {}
        MacosNetworkPolicy::DenyAll => profile.push_str("(deny network-outbound)\n"),
        MacosNetworkPolicy::Restricted(filters) => {
            if filters.len() == 1 {
                profile.push_str("(deny network-outbound (require-not ");
                profile.push_str(&filters[0]);
                profile.push_str("))\n");
            } else {
                profile.push_str("(deny network-outbound\n  (require-not\n    (require-any");
                for filter in filters {
                    profile.push_str("\n      ");
                    profile.push_str(&filter);
                }
                profile.push_str(")))\n");
            }
        }
    }
    Ok(profile)
}

/// Launch a child process through the current host's kernel sandbox primitive.
pub fn run_with_kernel_sandbox<I, S>(
    plan: &SandboxPlan,
    program: impl AsRef<OsStr>,
    args: I,
) -> Result<KernelSandboxCommandOutput, KernelSandboxError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let program = program.as_ref().to_os_string();
    let args = args
        .into_iter()
        .map(|arg| arg.as_ref().to_os_string())
        .collect::<Vec<OsString>>();

    if plan.os != OsFamily::current() {
        return Err(KernelSandboxError::InvalidPlan(format!(
            "plan targets {}, but current host is {}",
            plan.os,
            OsFamily::current()
        )));
    }

    match OsFamily::current() {
        OsFamily::Macos => run_macos_seatbelt(plan, program, args),
        _ => Err(KernelSandboxError::Unsupported(
            current_kernel_sandbox_support(),
        )),
    }
}

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

fn run_macos_seatbelt(
    plan: &SandboxPlan,
    program: OsString,
    args: Vec<OsString>,
) -> Result<KernelSandboxCommandOutput, KernelSandboxError> {
    let support = current_kernel_sandbox_support();
    if !support.available {
        return Err(KernelSandboxError::Unsupported(support));
    }

    let profile = macos_seatbelt_profile_for_plan(plan)?;
    let output = Command::new(MACOS_SANDBOX_EXEC)
        .arg("-p")
        .arg(profile)
        .arg(program)
        .args(args)
        .output()
        .map_err(|error| KernelSandboxError::Io(error.to_string()))?;

    Ok(KernelSandboxCommandOutput {
        os: OsFamily::Macos,
        primitive: MACOS_KERNEL_PRIMITIVE.to_string(),
        status_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

fn writable_path_literals(plan: &SandboxPlan) -> Result<Vec<String>, KernelSandboxError> {
    let mut paths = Vec::new();
    for rule in &plan.rules {
        if rule.capability.category != Category::Fs
            || !matches!(
                rule.capability.action,
                Action::Write | Action::Create | Action::Delete
            )
        {
            continue;
        }
        let target = rule.capability.target.as_str();
        if target == "*" || has_glob_syntax(target) {
            return Err(KernelSandboxError::InvalidPlan(format!(
                "macOS kernel enforcement requires exact absolute filesystem write targets, got '{target}'"
            )));
        }
        paths.push(
            canonicalize_literal_target(target)?
                .to_string_lossy()
                .to_string(),
        );
    }
    Ok(paths)
}

enum MacosNetworkPolicy {
    Unrestricted,
    Restricted(Vec<String>),
    DenyAll,
}

fn macos_network_policy(plan: &SandboxPlan) -> Result<MacosNetworkPolicy, KernelSandboxError> {
    let mut filters = Vec::new();
    for rule in &plan.rules {
        if rule.capability.category != Category::Net {
            continue;
        }
        match rule.capability.action {
            Action::Dns => filters.push(format!(
                "(literal \"{}\")",
                seatbelt_escape(MACOS_MDNSRESPONDER_SOCKET)?
            )),
            Action::Connect => {
                let target = rule.capability.target.as_str();
                if target == "*" {
                    return Ok(MacosNetworkPolicy::Unrestricted);
                }
                let port = network_target_port(target)?;
                filters.push(format!("(remote tcp \"*:{port}\")"));
            }
            Action::Listen => {}
            other => {
                return Err(KernelSandboxError::InvalidPlan(format!(
                    "unsupported macOS network action '{other}'"
                )))
            }
        }
    }

    filters.sort();
    filters.dedup();

    if filters.is_empty() {
        Ok(MacosNetworkPolicy::DenyAll)
    } else {
        Ok(MacosNetworkPolicy::Restricted(filters))
    }
}

fn network_target_port(target: &str) -> Result<u16, KernelSandboxError> {
    let port = target
        .rsplit_once(':')
        .map(|(_, port)| port)
        .ok_or_else(|| {
            KernelSandboxError::InvalidPlan(format!(
                "macOS network kernel enforcement requires host:port targets, got '{target}'"
            ))
        })?;
    port.parse::<u16>().map_err(|error| {
        KernelSandboxError::InvalidPlan(format!("invalid network port in '{target}': {error}"))
    })
}

fn has_glob_syntax(target: &str) -> bool {
    target.chars().any(|ch| matches!(ch, '*' | '?' | '[' | ']'))
}

fn canonicalize_literal_target(target: &str) -> Result<PathBuf, KernelSandboxError> {
    let path = Path::new(target);
    if !path.is_absolute() {
        return Err(KernelSandboxError::InvalidPlan(format!(
            "kernel file targets must be absolute paths, got '{target}'"
        )));
    }
    if let Ok(canonical) = fs::canonicalize(path) {
        return Ok(canonical);
    }

    let parent = path.parent().ok_or_else(|| {
        KernelSandboxError::InvalidPlan(format!("path '{target}' has no parent directory"))
    })?;
    let file_name = path.file_name().ok_or_else(|| {
        KernelSandboxError::InvalidPlan(format!("path '{target}' has no final path component"))
    })?;
    let parent = fs::canonicalize(parent).map_err(|error| {
        KernelSandboxError::InvalidPlan(format!(
            "parent directory for '{target}' must exist before kernel enforcement: {error}"
        ))
    })?;
    Ok(parent.join(file_name))
}

fn seatbelt_escape(input: &str) -> Result<String, KernelSandboxError> {
    let mut escaped = String::new();
    for ch in input.chars() {
        match ch {
            '\n' | '\r' | '\0' => {
                return Err(KernelSandboxError::InvalidPlan(
                    "Seatbelt profile literals cannot contain control characters".to_string(),
                ))
            }
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            other => escaped.push(other),
        }
    }
    Ok(escaped)
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
        Category::Net if capability.action == Action::Dns => (
            "macos.seatbelt.profile",
            SandboxCoverage::Direct,
            "Seatbelt can gate resolver socket access; hostname policy is paired with TLS/application checks",
        ),
        Category::Net => (
            "macos.seatbelt.profile",
            SandboxCoverage::Direct,
            "Seatbelt can constrain outbound sockets by protocol and port; arbitrary remote hostnames are not kernel-matchable",
        ),
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
    use std::net::TcpListener;
    use std::time::{SystemTime, UNIX_EPOCH};

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

    #[test]
    fn macos_seatbelt_profile_limits_file_writes_to_manifest_targets() {
        let dir = unique_temp_dir("profile");
        let allowed = dir.join("umbrella-today.txt");
        let manifest = weather_manifest_for_path(&allowed);
        let plan = plan_from_json(&manifest, OsFamily::Macos).unwrap();

        let profile = macos_seatbelt_profile_for_plan(&plan).unwrap();
        let allowed = fs::canonicalize(&dir)
            .unwrap()
            .join("umbrella-today.txt")
            .to_string_lossy()
            .to_string();

        assert!(profile.contains("(allow default)"));
        assert!(profile.contains("(deny file-write*"));
        assert!(profile.contains("(deny network-outbound"));
        assert!(profile.contains("(literal \"/private/var/run/mDNSResponder\")"));
        assert!(profile.contains("(remote tcp \"*:443\")"));
        assert!(profile.contains("(require-not"));
        assert!(profile.contains(&allowed));
        fs::remove_dir_all(dir).ok();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_kernel_sandbox_blocks_undeclared_file_writes() {
        if !Path::new(MACOS_SANDBOX_EXEC).exists() {
            return;
        }

        let dir = unique_temp_dir("kernel");
        let allowed = dir.join("allowed.txt");
        let denied = dir.join("denied.txt");
        let manifest = weather_manifest_for_path(&allowed);
        let plan = plan_from_json(&manifest, OsFamily::Macos).unwrap();
        let allowed_arg = fs::canonicalize(&dir)
            .unwrap()
            .join("allowed.txt")
            .to_string_lossy()
            .to_string();
        let denied_arg = fs::canonicalize(&dir)
            .unwrap()
            .join("denied.txt")
            .to_string_lossy()
            .to_string();

        let output = run_with_kernel_sandbox(
            &plan,
            "/bin/sh",
            [
                "-c",
                "printf allowed > \"$1\"; printf denied > \"$2\"",
                "sh",
                &allowed_arg,
                &denied_arg,
            ],
        )
        .unwrap();

        assert_eq!(fs::read_to_string(&allowed).unwrap(), "allowed");
        assert!(!denied.exists());
        assert!(!output.success());
        assert!(output.stderr.contains("Operation not permitted"));
        fs::remove_dir_all(dir).ok();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_kernel_sandbox_blocks_undeclared_network_ports() {
        if !Path::new(MACOS_SANDBOX_EXEC).exists() {
            return;
        }

        let allowed_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let denied_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let allowed_port = allowed_listener.local_addr().unwrap().port();
        let denied_port = denied_listener.local_addr().unwrap().port();
        let manifest = network_manifest_for_port(allowed_port);
        let plan = plan_from_json(&manifest, OsFamily::Macos).unwrap();
        let allowed_port = allowed_port.to_string();
        let denied_port = denied_port.to_string();

        let output = run_with_kernel_sandbox(
            &plan,
            "/bin/sh",
            [
                "-c",
                "/usr/bin/nc -z -G 1 localhost \"$1\"; /usr/bin/nc -z -G 1 localhost \"$2\"",
                "sh",
                &allowed_port,
                &denied_port,
            ],
        )
        .unwrap();

        assert!(!output.success());
        assert!(output
            .stderr
            .contains(&format!("localhost port {allowed_port}")));
        assert!(output.stderr.contains("succeeded"));
    }

    fn weather_manifest_for_path(path: &Path) -> String {
        let path = path.to_string_lossy();
        format!(
            r#"{{
              "version": 1,
              "package": "rust/weather-agent-e2e",
              "capabilities": [
                {{
                  "category": "net",
                  "action": "dns",
                  "target": "api.weather.gov",
                  "justification": "Resolve Weather.gov for the live umbrella forecast."
                }},
                {{
                  "category": "net",
                  "action": "connect",
                  "target": "api.weather.gov:443",
                  "justification": "Fetch the Weather.gov points and forecast resources over TLS."
                }},
                {{
                  "category": "fs",
                  "action": "write",
                  "target": "{path}",
                  "justification": "Write the umbrella decision text file."
                }}
              ],
              "justification": "Weather Agent E2E fetches live weather and writes one report."
            }}"#
        )
    }

    fn network_manifest_for_port(port: u16) -> String {
        format!(
            r#"{{
              "version": 1,
              "package": "rust/network-probe",
              "capabilities": [
                {{
                  "category": "net",
                  "action": "connect",
                  "target": "localhost:{port}",
                  "justification": "Connect to the declared local TCP test port."
                }}
              ],
              "justification": "Network probe validates Seatbelt outbound port enforcement."
            }}"#
        )
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "capability-os-sandbox-{label}-{}-{now}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
