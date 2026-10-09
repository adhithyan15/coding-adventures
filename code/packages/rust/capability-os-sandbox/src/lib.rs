#![forbid(unsafe_code)]
// Platform-conditional: code for the non-native platform is intentionally inactive; allow the resulting dead_code/unused lints only where it does not compile in.
#![allow(dead_code, unused_imports)] // platform-conditional network helpers/imports are inactive on some targets

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
///
/// `Advisory` and `Unsupported` exist so a plan can *say* it is weak. Neither
/// may appear in a plan that launches (D18S S-P2, S-P3; see
/// [`SandboxPlan::launch_preconditions`]). Lowering in this crate never
/// produces `Advisory` any more: what used to be advisory is now routed
/// through the broker, or reported as unsupported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SandboxCoverage {
    Direct,
    Brokered,
    LaunchTime,
    /// Narrows a class of behaviour without constraining the exact target.
    /// A defence, not an enforcement claim, so never a grant (S-P2).
    Advisory,
    /// The platform cannot enforce this capability at all. A launch failure,
    /// never a quiet downgrade (S-P3).
    Unsupported,
}

impl SandboxCoverage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Brokered => "brokered",
            Self::LaunchTime => "launch_time",
            Self::Advisory => "advisory",
            Self::Unsupported => "unsupported",
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
        self.coverage != SandboxCoverage::Unsupported && !self.primitive.contains("host_broker")
    }
}

/// How agent processes are separated from each other at the OS level
/// (D18S S-I1a, S-I5). Recorded, so a deployment can see which model it got.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrincipalModel {
    /// Every agent runs as the supervisor's user. Isolation rests on the
    /// sandbox alone; distinct principals are defence in depth (S-I5).
    SharedUid,
    /// Each agent runs as its own user.
    DistinctUid,
    /// Each agent gets its own AppContainer SID (Windows).
    AppcontainerSid,
}

impl PrincipalModel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SharedUid => "shared_uid",
            Self::DistinctUid => "distinct_uid",
            Self::AppcontainerSid => "appcontainer_sid",
        }
    }
}

/// Where an agent's broker runs (D18S S-I1a, S-K7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BrokerTopology {
    /// One broker per agent. The only topology that launches.
    PerAgent,
    /// One broker shared by a supervisor's agents. A launch failure (S-K7).
    PerSupervisor,
}

impl BrokerTopology {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PerAgent => "per_agent",
            Self::PerSupervisor => "per_supervisor",
        }
    }
}

/// The unconditional deny-all base policy (D18S S-I1, S-I1a).
///
/// It is installed identically for a zero-capability agent and for a
/// maximally capable one, and it is **not derived from the manifest**.
/// Manifest lowering may only subtract from it.
///
/// That is the whole point of making it a term of its own. A plan built as
/// `manifest.capabilities().map(lower)` gives the *strictest* agent in the
/// system, one with no capabilities, an **empty** plan, and "empty plan"
/// becomes "install nothing", which is allow-all. Landlock happens to fail
/// closed on an empty ruleset; seccomp, the environment block and descriptor
/// inheritance all fail open. With the base as a field, an empty manifest
/// still yields every primitive below.
///
/// ```text
///   plan = base_policy(os)            always, from the OS alone
///        + rules(manifest, os)        subtractions from the base, per grant
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasePolicy {
    pub os: OsFamily,
    /// The primitives the base installs, in install order.
    pub primitives: Vec<&'static str>,
    pub principal_model: PrincipalModel,
    pub broker_topology: BrokerTopology,
}

impl BasePolicy {
    /// The base policy for `os`. Nothing about the manifest is an input.
    ///
    /// The primitive lists are S-I1's deny-all classes as each platform
    /// expresses them. They describe what an enforcer must install. P2.1
    /// models them; the per-platform enforcers (P2.3, P2.4, P2.7, P2.8)
    /// install them.
    pub fn for_os(os: OsFamily) -> Self {
        let primitives: Vec<&'static str> = match os {
            OsFamily::Linux => vec![
                "linux.prctl.no_new_privs",
                "linux.seccomp.arch_check",
                "linux.seccomp.allowlist_deny_default",
                "linux.seccomp.deny_io_uring",
                "linux.landlock.deny_all_fs",
                "linux.execve.empty_env",
                "linux.close_range.inherited_fds",
            ],
            OsFamily::Macos => vec![
                "macos.seatbelt.deny_default",
                "macos.posix_spawn.empty_env",
                "macos.posix_spawn.close_other_fds",
            ],
            OsFamily::Windows => vec![
                "windows.appcontainer.no_capabilities",
                "windows.job_object.no_child_processes",
                "windows.createprocess.empty_environment_block",
                "windows.handle_inheritance.explicit_list",
            ],
            OsFamily::FreeBsd => vec![
                "freebsd.capsicum.capability_mode",
                "freebsd.posix_spawn.empty_env",
                "freebsd.closefrom.inherited_fds",
            ],
            OsFamily::OpenBsd => vec![
                "openbsd.pledge.stdio_only",
                "openbsd.unveil.lock_empty",
                "openbsd.execve.empty_env",
                "openbsd.closefrom.inherited_fds",
            ],
            OsFamily::Portable => vec![
                "host_broker.json_rpc.deny_default",
                "portable.spawn.empty_env",
                "portable.spawn.stdio_only",
            ],
        };
        let principal_model = match os {
            OsFamily::Windows => PrincipalModel::AppcontainerSid,
            _ => PrincipalModel::SharedUid,
        };
        Self {
            os,
            primitives,
            principal_model,
            broker_topology: BrokerTopology::PerAgent,
        }
    }
}

/// Full lowering plan for one OS: the base deny plus the manifest's grants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxPlan {
    pub package: String,
    pub os: OsFamily,
    /// The unconditional base (S-I1a). Never derived from `rules`.
    pub base: BasePolicy,
    pub rules: Vec<SandboxRule>,
}

/// Why a plan may not launch (D18S S-I1a, S-P2, S-P3, S-K7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanRejection {
    /// The base policy installs nothing.
    MissingBasePolicy,
    /// The base policy is not exactly [`BasePolicy::for_os`] for the plan's
    /// OS: another OS's base, or one with primitives added or removed.
    BasePolicyMismatch,
    /// A rule is advisory: it would narrow behaviour without enforcing the
    /// target, and advisory is not a grant (S-P2).
    AdvisoryRule(String),
    /// The platform cannot enforce a declared capability (S-P3).
    UnsupportedRule(String),
    /// One broker shared across a supervisor's agents (S-K7).
    SharedBroker,
    /// The plan is for the portable target, which has no kernel boundary:
    /// the broker is all there is, and a process that ignores it is
    /// unconstrained. Never a launch target for untrusted code.
    NoKernelBoundary,
    /// A rule lowered for another OS than the plan's.
    RuleForAnotherOs(String),
    /// A rule is not what this crate lowers its capability to. Plans are
    /// plain data with public fields; the check re-derives every rule, so an
    /// edited coverage label or primitive cannot reach an enforcer.
    RuleMismatch(String),
}

impl fmt::Display for PlanRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingBasePolicy => f.write_str("sandbox plan has no base deny policy"),
            Self::BasePolicyMismatch => {
                f.write_str("sandbox plan base policy is not the OS's base deny")
            }
            Self::AdvisoryRule(key) => write!(f, "advisory coverage is not a grant: {key}"),
            Self::UnsupportedRule(key) => {
                write!(f, "platform cannot enforce declared capability: {key}")
            }
            Self::SharedBroker => f.write_str("a per-supervisor broker cannot launch"),
            Self::NoKernelBoundary => {
                f.write_str("the portable target has no kernel boundary and cannot launch")
            }
            Self::RuleForAnotherOs(key) => write!(f, "rule lowered for another OS: {key}"),
            Self::RuleMismatch(key) => {
                write!(f, "rule differs from its capability's lowering: {key}")
            }
        }
    }
}

impl SandboxPlan {
    /// A plan with no grants. It still carries the full base policy: no
    /// grants means deny everything, never install nothing (S-I1a).
    pub fn empty(package: impl Into<String>, os: OsFamily) -> Self {
        Self {
            package: package.into(),
            os,
            base: BasePolicy::for_os(os),
            rules: Vec::new(),
        }
    }

    /// Every reason this plan may not launch, or `Ok` if there are none.
    ///
    /// An enforcer calls this before installing anything (S-P3): a sandbox
    /// that quietly becomes advisory is worse than none, because the
    /// deployment believes it is contained. All reasons are reported, not
    /// just the first, so one look at a refused launch shows the whole gap.
    ///
    /// No rule's label is taken on trust. The base must equal
    /// [`BasePolicy::for_os`] for the plan's OS, and every rule must equal
    /// what this crate lowers its capability to, so an edited coverage label
    /// or primitive is caught.
    ///
    /// **The set of rules is trusted.** Each rule is checked against its own
    /// capability, not against a manifest, so a correctly lowered rule for an
    /// undeclared capability passes. The enforcer (`spawn_verified`) must
    /// lower from the signed manifest itself, never accept a plan from
    /// elsewhere.
    ///
    /// **`Ok` is not proof of enforcement (S-P4).** It says the plan is
    /// launchable as written. Whether the platform applier installed the
    /// base and every rule is the applier's report, and until D18S steps 3,
    /// 4, 7 and 8 land no applier installs the base at all.
    pub fn launch_preconditions(&self) -> Result<(), Vec<PlanRejection>> {
        let mut rejections = Vec::new();
        if self.os == OsFamily::Portable {
            rejections.push(PlanRejection::NoKernelBoundary);
        }
        if self.base.primitives.is_empty() {
            rejections.push(PlanRejection::MissingBasePolicy);
        }
        if self.base.broker_topology == BrokerTopology::PerSupervisor {
            rejections.push(PlanRejection::SharedBroker);
        }
        if !self.base.primitives.is_empty()
            && self.base.broker_topology == BrokerTopology::PerAgent
            && self.base != BasePolicy::for_os(self.os)
        {
            rejections.push(PlanRejection::BasePolicyMismatch);
        }
        for rule in &self.rules {
            let key = rule.capability_key();
            if rule.os != self.os {
                rejections.push(PlanRejection::RuleForAnotherOs(key));
                continue;
            }
            let expected = lower_capability(&rule.capability, self.os);
            if *rule != expected {
                rejections.push(PlanRejection::RuleMismatch(key.clone()));
            }
            // The coverage verdict comes from the re-derived rule, never the
            // label on the one supplied.
            match expected.coverage {
                SandboxCoverage::Advisory => rejections.push(PlanRejection::AdvisoryRule(key)),
                SandboxCoverage::Unsupported => {
                    rejections.push(PlanRejection::UnsupportedRule(key))
                }
                SandboxCoverage::Direct
                | SandboxCoverage::Brokered
                | SandboxCoverage::LaunchTime => {}
            }
        }
        if rejections.is_empty() {
            Ok(())
        } else {
            Err(rejections)
        }
    }

    pub fn summary(&self) -> SandboxPlanSummary {
        let mut summary = SandboxPlanSummary {
            package: self.package.clone(),
            os: self.os,
            total_rules: self.rules.len(),
            base_primitives_required: self.base.primitives.len(),
            base_installed: false,
            principal_model: self.base.principal_model,
            broker_topology: self.base.broker_topology,
            ..SandboxPlanSummary::default()
        };
        for rule in &self.rules {
            match rule.coverage {
                SandboxCoverage::Direct => summary.direct_rules += 1,
                SandboxCoverage::Brokered => summary.brokered_rules += 1,
                SandboxCoverage::LaunchTime => summary.launch_time_rules += 1,
                SandboxCoverage::Advisory => summary.advisory_rules += 1,
                SandboxCoverage::Unsupported => summary.unsupported_rules += 1,
            }
            if rule.primitive.contains("host_broker") {
                summary.host_broker_rules += 1;
            } else if rule.coverage != SandboxCoverage::Unsupported {
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
    pub unsupported_rules: usize,
    pub native_rules: usize,
    pub host_broker_rules: usize,
    /// How many primitives the base deny requires (S-I1a). Never zero for a
    /// plan this crate built. A requirement, not a report of what is live.
    pub base_primitives_required: usize,
    /// Whether an applier has installed the base. Always `false` until the
    /// platform appliers land (D18S steps 3, 4, 7, 8); a summary never
    /// claims enforcement the code does not perform (S-P4).
    pub base_installed: bool,
    pub principal_model: PrincipalModel,
    pub broker_topology: BrokerTopology,
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
            unsupported_rules: 0,
            native_rules: 0,
            host_broker_rules: 0,
            base_primitives_required: 0,
            base_installed: false,
            principal_model: PrincipalModel::SharedUid,
            broker_topology: BrokerTopology::PerAgent,
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
///
/// The plan's launch preconditions are checked first (S-P3): an advisory or
/// unsupported rule, a missing base, or a shared broker refuses the launch
/// before anything is installed.
///
/// **What this does not do yet: install the base deny.** The only applier
/// today is macOS Seatbelt, and its profile still starts `(allow default)`
/// and denies only what the grants do not cover. The plan's base term says
/// what *must* be installed (`macos.seatbelt.deny_default`). Making the
/// applier install it is D18S build step 7 (P2.7), and doing it for each
/// other platform is steps 3, 4 and 8. Until then, a launch through here is
/// grant-shaped, not deny-all.
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
    // S-P3: refuse before installing anything, never after.
    if let Err(rejections) = plan.launch_preconditions() {
        let reasons = rejections
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(KernelSandboxError::InvalidPlan(reasons));
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
    Ok(SandboxPlan {
        package,
        os,
        base: BasePolicy::for_os(os),
        rules,
    })
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
    let (primitive, coverage, note) = lower_rule(capability, os);
    SandboxRule {
        capability: capability.clone(),
        os,
        expression: expression_for(capability, &primitive),
        primitive,
        coverage,
        note: note.to_string(),
    }
}

/// The lowering, cross-platform rules first.
///
/// A `Direct` label is an enforcement claim: the kernel primitive names the
/// exact target the manifest declared (S-P2). Three classes of grant can
/// never meet that on any platform, so they are settled here, before any
/// platform table is consulted, and no table can round them up:
///
/// ```text
///   category / target        every OS                 why
///   ----------------------   ----------------------   ------------------------------
///   ffi:*                    Unsupported              in-process code; nothing to broker (S-K6, S-I6)
///   proc:*                   Brokered (host_broker)   no primitive names a program or a PID;
///                                                     the supervisor spawns (spawn_verified)
///   net:*                    Brokered (host_broker)   no unprivileged primitive names a host
///   Direct + wildcard        Brokered (host_broker)   "*", "**", globs: nothing exact to name
/// ```
///
/// Portable has no kernel boundary at all, so everything else on it is the
/// broker; `launch_preconditions` refuses it anyway (`NoKernelBoundary`).
fn lower_rule(capability: &Capability, os: OsFamily) -> (String, SandboxCoverage, &'static str) {
    if capability.category == Category::Ffi {
        return (
            format!("{os}.ffi.unenforceable"),
            SandboxCoverage::Unsupported,
            "native code runs inside the agent's own address space; no sandbox can target-scope it and no broker can perform it",
        );
    }
    if os == OsFamily::Portable {
        let (primitive, coverage, note) = lower_portable(capability);
        return (primitive.to_string(), coverage, note);
    }
    match capability.category {
        Category::Proc => {
            return (
                format!("{os}.host_broker.process"),
                SandboxCoverage::Brokered,
                "no platform primitive names a program or a PID, so the supervisor spawns (spawn_verified) and signals; the agent never execs",
            )
        }
        Category::Net if capability.action == Action::Dns => {
            return (
                format!("{os}.host_broker.resolver"),
                SandboxCoverage::Brokered,
                "no unprivileged primitive can scope name resolution to one hostname, so the broker resolves",
            )
        }
        Category::Net => {
            return (
                format!("{os}.host_broker.net"),
                SandboxCoverage::Brokered,
                "no unprivileged primitive can scope a socket to one host, so the broker makes the connection",
            )
        }
        _ => {}
    }
    let (primitive, coverage, note) = match os {
        OsFamily::Linux => lower_linux(capability),
        OsFamily::Macos => lower_macos(capability),
        OsFamily::Windows => lower_windows(capability),
        OsFamily::FreeBsd => lower_freebsd(capability),
        OsFamily::OpenBsd => lower_openbsd(capability),
        OsFamily::Portable => lower_portable(capability),
    };
    if coverage == SandboxCoverage::Direct && !is_exact_file_target(&capability.target, os) {
        return (
            format!("{os}.host_broker.{}", capability.category),
            SandboxCoverage::Brokered,
            "the target is a pattern, a directory, or not a normalised absolute path, so no kernel rule can carry it exactly; the broker checks each operation",
        );
    }
    (primitive.to_string(), coverage, note)
}

/// Whether a filesystem target names exactly one normalised absolute path
/// on `os`.
///
/// Only such a target may be `Direct`. A directory is a wildcard in
/// disguise: Landlock `path_beneath`, `unveil`, a preopened Capsicum
/// directory and inheritable Windows ACEs all grant the whole subtree, so
/// `fs:read:/` would be the entire filesystem with a Direct label.
///
/// ```text
///   target              exact?   why not
///   -----------------   ------   ------------------------------------
///   /etc/hosts          yes      (Unix only)
///   C:\Users\a\x.txt     yes      (Windows only)
///   /                   no       the root: every path beneath it
///   /tmp/               no       trailing separator names a directory
///   .  ./x  a/b         no       relative: resolves against the cwd
///   ~  ~/notes          no       shell expansion, not a path
///   /a/../etc/shadow    no       `..` or `.` components hide the target
///   /a//b               no       empty component
///   /tmp/*  **  [ab]    no       glob syntax
///   /a/\0b              no       control characters
///   /a\b                no       on Unix, a backslash is a filename byte
///                                that reads like a separator
///   C:\a.  C:\a\b␠      no       Windows strips trailing dots and spaces,
///                                so the name is not the file opened
///   C:\a:stream         no       an NTFS alternate data stream
///   /etc/hosts          no       on Windows: relative to the current drive
///   C:\x                no       on Unix: a file in the cwd named `C:\x`
/// ```
///
/// A syntactically exact path can still be a directory on disk. The applier
/// resolves each Direct target at launch and refuses one that is a
/// directory or intersects the never-grantable set (S-I6, S-P3).
fn is_exact_file_target(target: &str, os: OsFamily) -> bool {
    if has_glob_syntax(target) || target.contains('~') || target.chars().any(char::is_control) {
        return false;
    }
    let plain = |component: &str| !component.is_empty() && component != "." && component != "..";
    match os {
        OsFamily::Linux | OsFamily::Macos | OsFamily::FreeBsd | OsFamily::OpenBsd => {
            match target.strip_prefix('/') {
                Some(rest) => {
                    !rest.is_empty() && !rest.contains('\\') && rest.split('/').all(plain)
                }
                None => false,
            }
        }
        OsFamily::Windows => match windows_drive_rest(target) {
            Some(rest) => {
                !rest.is_empty()
                    && rest.split(['/', '\\']).all(|component| {
                        plain(component)
                            && !component.ends_with('.')
                            && !component.ends_with(' ')
                            && !component.contains(':')
                    })
            }
            None => false,
        },
        OsFamily::Portable => false,
    }
}

/// The part after `C:\` or `C:/` in an absolute Windows path.
fn windows_drive_rest(target: &str) -> Option<&str> {
    let bytes = target.as_bytes();
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
    {
        Some(&target[3..])
    } else {
        None
    }
}

fn lower_linux(capability: &Capability) -> (&'static str, SandboxCoverage, &'static str) {
    match capability.category {
        Category::Fs => (
            "linux.landlock.path_beneath",
            SandboxCoverage::Direct,
            "Landlock restricts file actions to declared paths; the applier must probe the Landlock ABI (no direct fs below v3), and refuse the launch when the kernel cannot express the rule, the target is a directory, or it intersects the never-grantable set (S-I6, S-P3)",
        ),
        Category::Env => (
            "linux.execve.env_allowlist",
            SandboxCoverage::LaunchTime,
            "environment capabilities are enforced by constructing the child env block",
        ),
        Category::Time => (
            "linux.host_broker.clock",
            SandboxCoverage::Brokered,
            "time reads cannot be target-scoped by seccomp, so the broker answers them",
        ),
        Category::Stdin | Category::Stdout => (
            "linux.fd_table",
            SandboxCoverage::LaunchTime,
            "stdio authority is selected by the file descriptors inherited at spawn",
        ),
        Category::Net | Category::Proc | Category::Ffi => unreachable_lowering(),
    }
}

/// The arm for categories `lower_rule` settles before any platform table.
fn unreachable_lowering() -> (&'static str, SandboxCoverage, &'static str) {
    (
        "unreachable",
        SandboxCoverage::Unsupported,
        "lowered before the platform table; reaching here fails closed",
    )
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
            "macos.host_broker.clock",
            SandboxCoverage::Brokered,
            "Seatbelt cannot target-scope time reads, so the broker answers them",
        ),
        Category::Fs => (
            "macos.seatbelt.profile",
            SandboxCoverage::Direct,
            "Seatbelt carries filesystem policy as exact path literals",
        ),
        Category::Net | Category::Proc | Category::Ffi => unreachable_lowering(),
    }
}

fn lower_windows(capability: &Capability) -> (&'static str, SandboxCoverage, &'static str) {
    match capability.category {
        Category::Fs => (
            "windows.appcontainer.acl",
            SandboxCoverage::Direct,
            "AppContainer identity plus ACLs can restrict file object access",
        ),
        Category::Env => (
            "windows.createprocess.environment_block",
            SandboxCoverage::LaunchTime,
            "the child environment block is built from the manifest allowlist",
        ),
        Category::Time => (
            "windows.host_broker.clock",
            SandboxCoverage::Brokered,
            "Windows sandbox primitives cannot target-scope time reads, so the broker answers them",
        ),
        Category::Stdin | Category::Stdout => (
            "windows.handle_inheritance",
            SandboxCoverage::LaunchTime,
            "stdio authority is selected by inheritable handles at spawn",
        ),
        Category::Net | Category::Proc | Category::Ffi => unreachable_lowering(),
    }
}

fn lower_freebsd(capability: &Capability) -> (&'static str, SandboxCoverage, &'static str) {
    match capability.category {
        Category::Fs => (
            "freebsd.capsicum.cap_rights",
            SandboxCoverage::Direct,
            "Capsicum preopens handles and limits rights before capability mode",
        ),
        Category::Env => (
            "freebsd.posix_spawn.env_allowlist",
            SandboxCoverage::LaunchTime,
            "the child environment is built from the manifest allowlist",
        ),
        Category::Time => (
            "freebsd.host_broker.clock",
            SandboxCoverage::Brokered,
            "capability mode cannot target-scope time reads, so the broker answers them",
        ),
        Category::Stdin | Category::Stdout => (
            "freebsd.capsicum.fd_rights",
            SandboxCoverage::LaunchTime,
            "stdio authority is selected by inherited descriptors and Capsicum rights",
        ),
        Category::Net | Category::Proc | Category::Ffi => unreachable_lowering(),
    }
}

fn lower_openbsd(capability: &Capability) -> (&'static str, SandboxCoverage, &'static str) {
    match capability.category {
        Category::Fs => (
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
        Category::Time => (
            "openbsd.host_broker.clock",
            SandboxCoverage::Brokered,
            "pledge cannot target-scope time reads, so the broker answers them",
        ),
        Category::Net | Category::Proc | Category::Ffi => unreachable_lowering(),
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

    const PURE_MANIFEST: &str = r#"{
      "version": 1,
      "package": "rust/pure",
      "capabilities": [],
      "justification": "Pure computation."
    }"#;

    #[test]
    fn an_empty_manifest_still_installs_the_whole_base_deny() {
        // S-I1a: the strictest agent in the system gets no grants, and must
        // NOT get an empty plan, because "install nothing" is allow-all.
        for os in OsFamily::all_supported() {
            let plan = plan_from_json(PURE_MANIFEST, os).unwrap();
            assert_eq!(plan.package, "rust/pure");
            assert!(plan.rules.is_empty(), "{os}");
            assert_eq!(plan.base, BasePolicy::for_os(os), "{os}");
            assert!(!plan.base.primitives.is_empty(), "{os}");
            let summary = plan.summary();
            assert_eq!(summary.base_primitives_required, plan.base.primitives.len());
            assert!(
                !summary.base_installed,
                "{os}: no applier installs the base yet"
            );
            if os == OsFamily::Portable {
                assert_eq!(
                    plan.launch_preconditions(),
                    Err(vec![PlanRejection::NoKernelBoundary])
                );
            } else {
                assert_eq!(plan.launch_preconditions(), Ok(()), "{os}");
            }
        }
        // The Linux base carries every S-P1 filter-shape requirement.
        let linux = BasePolicy::for_os(OsFamily::Linux).primitives;
        for required in [
            "linux.prctl.no_new_privs",
            "linux.seccomp.arch_check",
            "linux.seccomp.allowlist_deny_default",
            "linux.seccomp.deny_io_uring",
        ] {
            assert!(linux.contains(&required), "{required}");
        }
    }

    #[test]
    fn the_base_does_not_depend_on_the_manifest() {
        for os in OsFamily::all_supported() {
            let pure = plan_from_json(PURE_MANIFEST, os).unwrap();
            let weather = plan_from_json(WEATHER_MANIFEST, os).unwrap();
            assert_eq!(pure.base, weather.base, "{os}");
            assert_eq!(
                SandboxPlan::empty("x", os).base,
                weather.base,
                "{os}: even the empty constructor carries the base"
            );
        }
    }

    /// Every valid (category, action) pair, with a target that is not a
    /// wildcard, plus the wildcard filesystem case.
    fn every_capability() -> Vec<Capability> {
        use Action::*;
        use Category::*;
        let pairs = [
            (Fs, Read),
            (Fs, Write),
            (Fs, Create),
            (Fs, Delete),
            (Fs, List),
            (Net, Connect),
            (Net, Listen),
            (Net, Dns),
            (Proc, Exec),
            (Proc, Fork),
            (Proc, Signal),
            (Env, Read),
            (Env, Write),
            (Ffi, Call),
            (Ffi, Load),
            (Time, Read),
            (Time, Sleep),
            (Stdin, Read),
            (Stdout, Write),
        ];
        // `Capability::new` refuses pairs capability-cage does not allow,
        // so the list cannot drift into invalid combinations.
        let mut capabilities: Vec<Capability> = pairs
            .iter()
            .filter_map(|(category, action)| {
                Capability::new(*category, *action, "target", "every lowering arm").ok()
            })
            .collect();
        capabilities.push(Capability::new(Fs, Read, "*", "wildcard").unwrap());
        capabilities
    }

    #[test]
    fn no_lowering_on_any_platform_is_advisory() {
        // S-P2: what would be advisory is brokered or unsupported instead.
        let capabilities = every_capability();
        assert!(capabilities.len() >= 18);
        for os in OsFamily::all_supported() {
            for capability in &capabilities {
                let rule = lower_capability(capability, os);
                assert_ne!(
                    rule.coverage,
                    SandboxCoverage::Advisory,
                    "{os}: {}",
                    rule.capability_key()
                );
            }
        }
    }

    /// Every OS that has a kernel boundary.
    fn kernel_oses() -> impl Iterator<Item = OsFamily> {
        OsFamily::all_supported()
            .into_iter()
            .filter(|os| *os != OsFamily::Portable)
    }

    #[test]
    fn an_unenforceable_capability_fails_the_launch_loudly() {
        // S-P3: no platform can broker or enforce in-process code loading.
        let ffi =
            Capability::new(Category::Ffi, Action::Load, "plugin.dll", "load a plugin").unwrap();
        let rule = lower_capability(&ffi, OsFamily::Windows);
        assert_eq!(rule.coverage, SandboxCoverage::Unsupported);
        assert!(!rule.is_native());
        let mut plan = SandboxPlan::empty("rust/plugin", OsFamily::Windows);
        plan.rules.push(rule);
        let summary = plan.summary();
        assert_eq!(summary.unsupported_rules, 1);
        assert_eq!(
            summary.native_rules, 0,
            "unsupported is not native coverage"
        );
        assert_eq!(
            plan.launch_preconditions(),
            Err(vec![PlanRejection::UnsupportedRule(
                "ffi:load:plugin.dll".to_string()
            )])
        );
    }

    #[test]
    fn ffi_is_unsupported_on_every_platform() {
        // S-K6, S-I6: native code runs in the agent's address space. Neither
        // a kernel rule nor the broker can scope it, on any OS.
        for os in OsFamily::all_supported() {
            for action in [Action::Call, Action::Load] {
                let Ok(ffi) = Capability::new(Category::Ffi, action, "libm.so", "math") else {
                    continue;
                };
                let rule = lower_capability(&ffi, os);
                assert_eq!(rule.coverage, SandboxCoverage::Unsupported, "{os}");
                let mut plan = SandboxPlan::empty("rust/ffi", os);
                plan.rules.push(rule);
                let rejections = plan.launch_preconditions().unwrap_err();
                assert!(
                    rejections
                        .iter()
                        .any(|r| matches!(r, PlanRejection::UnsupportedRule(_))),
                    "{os}: {rejections:?}"
                );
            }
        }
    }

    #[test]
    fn process_authority_is_brokered_everywhere() {
        // No primitive names a program or a PID. proc:exec in particular must
        // never be Direct: a Direct exec would let the agent run an
        // unverified binary; the supervisor's spawn_verified runs it instead.
        for os in kernel_oses() {
            for action in [Action::Exec, Action::Fork, Action::Signal] {
                let Ok(proc) = Capability::new(Category::Proc, action, "/usr/bin/true", "x") else {
                    continue;
                };
                let rule = lower_capability(&proc, os);
                assert_eq!(rule.coverage, SandboxCoverage::Brokered, "{os} {action}");
                assert!(rule.primitive.contains("host_broker"), "{os} {action}");
            }
        }
    }

    #[test]
    fn network_authority_is_never_labelled_direct() {
        // No unprivileged primitive scopes a socket or a lookup to one host:
        // cgroup BPF needs privilege, Seatbelt matches ports not names,
        // AppContainer's internetClient is all or nothing, jails need root.
        for os in kernel_oses() {
            for (action, target) in [
                (Action::Connect, "api.weather.gov:443"),
                (Action::Listen, "127.0.0.1:8080"),
                (Action::Dns, "api.weather.gov"),
            ] {
                let net = Capability::new(Category::Net, action, target, "x").unwrap();
                let rule = lower_capability(&net, os);
                assert_eq!(rule.coverage, SandboxCoverage::Brokered, "{os} {action}");
                assert!(rule.primitive.contains("host_broker"), "{os} {action}");
            }
        }
    }

    #[test]
    fn a_wildcard_target_is_never_labelled_direct() {
        for os in OsFamily::all_supported() {
            for target in [
                "*",
                "**",
                "/**",
                "/tmp/*",
                "/home/?/notes",
                "/[ab]/x",
                "/",
                "/home/user/",
                "/tmp/",
                ".",
                "./notes",
                "notes/today",
                "~",
                "~/notes",
                "/a/../etc/shadow",
                "/a/./b",
                "/a//b",
                "C:\\",
                "C:\\Users\\",
                "C:\\a\\..\\b",
                "",
                "/a/\u{0}b",
                "/a/b\n",
                "C:\\a\u{1f}",
            ] {
                assert!(!is_exact_file_target(target, os), "{os} {target}");
                for action in [Action::Read, Action::Write, Action::List] {
                    // capability-cage refuses some of these outright; the
                    // rest must not come out Direct.
                    let Ok(fs) = Capability::new(Category::Fs, action, target, "x") else {
                        continue;
                    };
                    let rule = lower_capability(&fs, os);
                    assert_ne!(rule.coverage, SandboxCoverage::Direct, "{os} {target}");
                }
            }
        }
        // An exact path stays Direct where the platform can name it, and
        // only there.
        let unix = ["/etc/hosts", "/tmp/umbrella-today.txt"];
        let windows = ["C:\\Users\\a\\notes.txt", "d:/data/report.csv"];
        for os in kernel_oses() {
            for exact in unix.iter().chain(windows.iter()) {
                let fs = Capability::new(Category::Fs, Action::Read, *exact, "x").unwrap();
                let direct = lower_capability(&fs, os).coverage == SandboxCoverage::Direct;
                let native = (os == OsFamily::Windows) == windows.contains(exact);
                assert_eq!(direct, native, "{os} {exact}");
            }
        }
    }

    #[test]
    fn a_target_the_os_would_rename_is_not_exact() {
        // Windows strips trailing dots and spaces and treats `:` as a stream
        // separator; Unix treats `\` as an ordinary byte.
        for target in [
            "C:\\Users\\a.",
            "C:\\a\\b ",
            "C:\\a:secret",
            "C:\\a\\...",
            "C: \\x",
        ] {
            assert!(!is_exact_file_target(target, OsFamily::Windows), "{target}");
        }
        for target in ["/a\\..\\b", "/a\\b"] {
            assert!(!is_exact_file_target(target, OsFamily::Linux), "{target}");
        }
        assert!(is_exact_file_target("/a/b.txt", OsFamily::Linux));
        assert!(is_exact_file_target("C:\\a\\b.txt", OsFamily::Windows));
    }

    #[test]
    fn every_built_plan_passes_its_own_rederivation() {
        for os in kernel_oses() {
            for manifest in [WEATHER_MANIFEST, PURE_MANIFEST] {
                let plan = plan_from_json(manifest, os).unwrap();
                assert_eq!(plan.launch_preconditions(), Ok(()), "{os}");
            }
        }
    }

    #[test]
    fn the_portable_target_never_launches() {
        let plan = plan_from_json(WEATHER_MANIFEST, OsFamily::Portable).unwrap();
        assert_eq!(
            plan.launch_preconditions(),
            Err(vec![PlanRejection::NoKernelBoundary])
        );
    }

    #[test]
    fn a_relabelled_rule_cannot_reach_an_enforcer() {
        // The plan's fields are public. Relabelling an unsupported rule as
        // Direct, or a brokered rule as Direct, is caught by re-derivation,
        // and the verdict comes from the re-derived coverage.
        let ffi = Capability::new(Category::Ffi, Action::Load, "libx.so", "x").unwrap();
        let mut plan = SandboxPlan::empty("rust/plugin", OsFamily::Linux);
        let mut rule = lower_capability(&ffi, OsFamily::Linux);
        rule.coverage = SandboxCoverage::Direct;
        rule.primitive = "linux.landlock.path_beneath".to_string();
        plan.rules.push(rule);
        let rejections = plan.launch_preconditions().unwrap_err();
        assert!(rejections.contains(&PlanRejection::RuleMismatch("ffi:load:libx.so".into())));
        assert!(rejections.contains(&PlanRejection::UnsupportedRule("ffi:load:libx.so".into())));

        let mut weather = plan_from_json(WEATHER_MANIFEST, OsFamily::Linux).unwrap();
        weather.rules[1].coverage = SandboxCoverage::Direct;
        weather.rules[1].primitive = "linux.cgroup_bpf.sock_addr".to_string();
        assert_eq!(
            weather.launch_preconditions(),
            Err(vec![PlanRejection::RuleMismatch(
                "net:connect:api.weather.gov:443".into()
            )])
        );
    }

    #[test]
    fn the_base_must_be_exactly_the_os_base() {
        let mut other_os = SandboxPlan::empty("x", OsFamily::Linux);
        other_os.base = BasePolicy::for_os(OsFamily::Macos);
        assert_eq!(
            other_os.launch_preconditions(),
            Err(vec![PlanRejection::BasePolicyMismatch])
        );
        // A base with one primitive quietly removed is not the base.
        let mut thinned = SandboxPlan::empty("x", OsFamily::Linux);
        thinned
            .base
            .primitives
            .retain(|p| *p != "linux.seccomp.deny_io_uring");
        assert_eq!(
            thinned.launch_preconditions(),
            Err(vec![PlanRejection::BasePolicyMismatch])
        );
    }

    #[test]
    fn every_launch_precondition_is_reported_at_once() {
        let mut plan = plan_from_json(WEATHER_MANIFEST, OsFamily::Linux).unwrap();
        assert_eq!(plan.launch_preconditions(), Ok(()));
        plan.rules[0].coverage = SandboxCoverage::Advisory;
        plan.rules[1].os = OsFamily::Macos;
        plan.base.primitives.clear();
        plan.base.broker_topology = BrokerTopology::PerSupervisor;
        let rejections = plan.launch_preconditions().unwrap_err();
        assert_eq!(rejections.len(), 4, "{rejections:?}");
        assert!(rejections.contains(&PlanRejection::MissingBasePolicy));
        assert!(rejections.contains(&PlanRejection::SharedBroker));
        assert!(rejections
            .iter()
            .any(|rejection| matches!(rejection, PlanRejection::RuleMismatch(_))));
        assert!(rejections
            .iter()
            .any(|rejection| matches!(rejection, PlanRejection::RuleForAnotherOs(_))));
        let mut all = rejections;
        all.extend([
            PlanRejection::BasePolicyMismatch,
            PlanRejection::AdvisoryRule("k".into()),
            PlanRejection::UnsupportedRule("k".into()),
            PlanRejection::NoKernelBoundary,
        ]);
        for rejection in &all {
            assert!(!rejection.to_string().is_empty());
        }
    }

    #[test]
    fn the_principal_and_broker_models_are_recorded() {
        let windows = plan_from_json(WEATHER_MANIFEST, OsFamily::Windows).unwrap();
        assert_eq!(
            windows.summary().principal_model,
            PrincipalModel::AppcontainerSid
        );
        let linux = plan_from_json(WEATHER_MANIFEST, OsFamily::Linux)
            .unwrap()
            .summary();
        assert_eq!(linux.principal_model, PrincipalModel::SharedUid);
        assert_eq!(linux.broker_topology, BrokerTopology::PerAgent);
        assert_eq!(PrincipalModel::DistinctUid.as_str(), "distinct_uid");
        assert_eq!(BrokerTopology::PerSupervisor.as_str(), "per_supervisor");
        assert_eq!(SandboxCoverage::Unsupported.as_str(), "unsupported");
    }

    #[test]
    fn openbsd_network_is_brokered_not_pledged() {
        // The OpenBSD lowering was the most advisory in the tree (S-P2).
        let plan = plan_from_json(WEATHER_MANIFEST, OsFamily::OpenBsd).unwrap();
        assert!(plan.has_primitive("openbsd.host_broker.net"));
        assert!(!plan.has_primitive("openbsd.pledge"));
        assert_eq!(plan.launch_preconditions(), Ok(()));
    }

    #[test]
    fn weather_manifest_lowers_to_linux_primitives() {
        let plan = plan_from_json(WEATHER_MANIFEST, OsFamily::Linux).unwrap();

        assert!(plan.has_primitive("linux.landlock.path_beneath"));
        assert!(plan.has_primitive("linux.host_broker.net"));
        assert!(plan.has_primitive("linux.host_broker.resolver"));
        assert!(!plan.has_primitive("linux.cgroup_bpf.sock_addr"));
        assert_eq!(plan.summary().total_rules, 3);
        assert_eq!(plan.summary().direct_rules, 1);
        assert_eq!(plan.summary().brokered_rules, 2);
    }

    #[test]
    fn weather_manifest_lowers_to_macos_windows_and_portable() {
        let macos = plan_from_json(WEATHER_MANIFEST, OsFamily::Macos).unwrap();
        assert!(macos.has_primitive("macos.seatbelt.profile"));
        assert!(macos.has_primitive("macos.host_broker.net"));

        let windows = plan_from_json(WEATHER_MANIFEST, OsFamily::Windows).unwrap();
        assert!(windows.has_primitive("windows.host_broker.net"));
        // `/tmp/umbrella-today.txt` is not an absolute Windows path, so the
        // file rule is brokered rather than an ACL.
        assert!(windows.has_primitive("windows.host_broker.fs"));
        assert!(!windows.has_primitive("windows.appcontainer.acl"));

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

    // A Seatbelt profile is macOS policy over POSIX paths; on Windows the
    // canonical temp path is a `\\?\` verbatim path no profile would hold.
    #[cfg(unix)]
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
        // A JSON string: a Windows path's backslashes must be escaped.
        let path = path
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
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
