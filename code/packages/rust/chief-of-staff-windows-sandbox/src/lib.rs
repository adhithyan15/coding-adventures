//! # The Windows applier (D18S build step 8)
//!
//! `capability-os-sandbox` says what an agent may do. This crate makes the
//! Windows kernel enforce it, for a compiled agent, from its first
//! instruction.
//!
//! Windows needs no filter and no shim. Every process runs under an access
//! token, and the kernel checks that token on every open of every object.
//! An **AppContainer** token is one whose principal is a SID made for one
//! application. It starts with access to almost nothing, and gains only
//! what an access control list (ACL) names that SID for. So the work here
//! is to create the agent's process with that token, and to prove it has
//! it before the agent runs.
//!
//! What stands in the way is not Windows but stable Rust: everything below
//! is set through a *process-thread attribute list* on `CreateProcessW`,
//! and std's `Command` can only take one on nightly (`raw_attribute`). So
//! this crate creates the process itself, and returns its own
//! `ConfinedChild` rather than a `std::process::Child`.
//!
//! ```text
//!   supervisor                                          agent
//!   --------------------------------------------------  ---------------
//!   WindowsConfinement::prepare_verified(plan, agent, agent_id)
//!     re-verifies the agent through its held handle
//!     the path is exactly that handle's final path
//!     the agent's own AppContainer SID (no profile)
//!     one ACE on the image: that SID may read and run it
//!   spawn(arguments, environment)
//!     a job: kill on close, one process, no UI
//!     pipes for stdin and stdout; NUL for stderr
//!     CreateProcessW(SUSPENDED, the attributes below)
//!     launch checks (S-P4), from outside the process:
//!       token: AppContainer, LPAC, this SID,
//!              no capabilities, Low integrity
//!       the process is in the job
//!       its mitigation policies are the ones asked for
//!       its image is the held file's name
//!     ResumeThread ────────────────────────────────────►  first
//!                                                         instruction,
//!                                                         confined
//! ```
//!
//! A process created suspended has run nothing, not even its loader. So
//! every check above happens before any code of the agent's, and a failed
//! one ends the process through its job and refuses the launch (S-P3).
//!
//! ## The attributes
//!
//! ```text
//!   attribute                        value
//!   -------------------------------  ----------------------------------------
//!   (creation flags)                 suspended, detached: no console, whose
//!                                    host would be a child of the agent
//!   SECURITY_CAPABILITIES            the agent's AppContainer SID, with no
//!                                    capabilities: no network, no libraries,
//!                                    no devices
//!   ALL_APPLICATION_PACKAGES_POLICY  opt out: a "less privileged"
//!                                    AppContainer (LPAC). Every AppContainer
//!                                    token carries ALL APPLICATION PACKAGES
//!                                    (S-1-15-2-1), which many files and keys
//!                                    grant; an LPAC token does not (S-I5)
//!   HANDLE_LIST                      stdin, stdout, stderr, and the handles
//!                                    passed on: the only handles inherited
//!                                    (S-I3)
//!   JOB_LIST                         the job, so the process is in it from
//!                                    its creation
//!   CHILD_PROCESS_POLICY             restricted: it can create no process
//!   MITIGATION_POLICY                [`MITIGATIONS`]
//! ```
//!
//! ## The job
//!
//! The job kills its process when its last handle closes. The supervisor
//! holds the only one, inside the `ConfinedChild`. It allows one active
//! process, kills on an unhandled exception rather than reporting it, and
//! has every UI restriction: no desktop, no display settings, no exit of
//! Windows, no global atoms, no other process's user handles, and no
//! clipboard or system parameters.
//!
//! ## Exec by path, pinned to a held handle (S-K1)
//!
//! `CreateProcessW` takes a path, never a handle. `VerifiedExecutable`
//! holds the agent open **sharing read only**, so while it is held no one
//! can write, rename or delete the file. Then:
//!
//! - the file is owned by this user, Administrators, SYSTEM or
//!   TrustedInstaller, and no one else may write it by its access list (the
//!   share mode does not keep out a writable mapping made earlier);
//! - the path given must be exactly the handle's final path
//!   (`GetFinalPathNameByHandleW`), case included;
//! - after the suspended creation, the child's image name
//!   (`QueryFullProcessImageNameW`, the path the image was opened by) and
//!   the held handle's final path must both still be that path.
//!
//! The held file cannot be renamed, and a directory above it cannot be
//! renamed while it is open without delete sharing; the last two checks
//! confirm the path led to the held file at the creation.
//!
//! ## The principal
//!
//! Each agent gets its own SID, derived from [`container_name`]:
//! `chief-of-staff.` and 32 hex digits, the first 16 bytes of SHA-256 over
//! the attested `agent_id`. The derivation is deterministic, so the agent
//! has the same principal on every launch, and two agents share one only
//! through a SHA-256 collision. No profile is registered: a profile would
//! give the agent a storage folder and a registry key of its own, kept
//! across runs, which is state S-I1 does not allow.
//!
//! An LPAC token reads nothing ALL APPLICATION PACKAGES may, its own image
//! included. So `WindowsConfinement::prepare_verified` adds one ACE to the
//! image: this SID may read and execute it. The working directory is the
//! system directory, which ALL RESTRICTED APPLICATION PACKAGES (S-1-15-2-2,
//! which LPAC keeps) may read.
//!
//! ## The environment
//!
//! The same closed set as the macOS applier ([`GRANTABLE_ENVIRONMENT`]):
//! only names set by the caller, each in the set, with plain values. To it
//! the applier adds `SystemRoot`, which Windows' own libraries read, and
//! nothing else. It is passed as an explicit block, never inherited.

#![deny(unsafe_op_in_unsafe_fn)]

use capability_os_sandbox::{OsFamily, PlanRejection, SandboxCoverage, SandboxPlan};
use std::fmt;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{ConfinedChild, SpawnRequest, WindowsConfinement};

/// The variable that carries the values of the handles passed on to the
/// agent, in order, comma-separated (P2.8b: a broker's keys).
pub const HANDLES_VARIABLE: &str = "CHIEF_OF_STAFF_HANDLES";

/// Why an agent cannot be confined. Every one of these refuses the launch
/// (S-P3): nothing is ever started with less.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfinementError {
    /// The plan itself may not launch.
    Plan(Vec<PlanRejection>),
    /// The plan is for another OS.
    NotWindowsPlan,
    /// The plan has a `Direct` grant, which this applier does not express
    /// yet (P2.8c). Refused rather than run without it.
    Unsupported(String),
    /// The agent failed its digest, or its path is not the canonical path
    /// of the file that was verified.
    Executable(String),
    /// The agent's AppContainer profile or SID could not be made or found.
    Container(String),
    /// The ACE that lets the agent read its own image could not be set.
    ImageAccess(String),
    /// The caller sets an environment variable outside the closed set
    /// (S-I4a), or a value that is not plain.
    Environment(String),
    /// An argument or variable holds a NUL, so it cannot be passed.
    Argument(String),
    /// A Win32 call needed to create the process failed.
    Create(String),
    /// The created process is not what was asked for (S-P4). It has been
    /// ended without running.
    Verification(String),
}

impl fmt::Display for ConfinementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Plan(rejections) => write!(
                f,
                "sandbox plan cannot launch: {} reason(s)",
                rejections.len()
            ),
            Self::NotWindowsPlan => f.write_str("sandbox plan is not for Windows"),
            Self::Unsupported(key) => {
                write!(f, "Direct grants are not supported on Windows yet: {key}")
            }
            Self::Executable(why) => write!(f, "executable refused: {why}"),
            Self::Container(why) => write!(f, "AppContainer refused: {why}"),
            Self::ImageAccess(why) => write!(f, "image access entry refused: {why}"),
            Self::Environment(why) => write!(f, "agent environment refused: {why}"),
            Self::Argument(why) => write!(f, "argument refused: {why}"),
            Self::Create(why) => write!(f, "process creation failed: {why}"),
            Self::Verification(why) => write!(f, "launch check failed: {why}"),
        }
    }
}

impl std::error::Error for ConfinementError {}

/// The plan is one this applier can run: for Windows, launchable, and with
/// no `Direct` grant (those are P2.8c).
pub fn check_plan(plan: &SandboxPlan) -> Result<(), ConfinementError> {
    if plan.os != OsFamily::Windows {
        return Err(ConfinementError::NotWindowsPlan);
    }
    plan.launch_preconditions()
        .map_err(ConfinementError::Plan)?;
    if let Some(rule) = plan
        .rules
        .iter()
        .find(|rule| rule.coverage == SandboxCoverage::Direct)
    {
        return Err(ConfinementError::Unsupported(rule.capability_key()));
    }
    Ok(())
}

/// The AppContainer profile name for `agent_id`: `chief-of-staff.` and the
/// first 16 bytes of its SHA-256 in lowercase hex. 47 characters, within
/// the 64 a profile name may have, and only characters a name may hold.
///
/// ```text
///   agent_id "weather-reporter"  →  chief-of-staff.<32 hex digits>
/// ```
pub fn container_name(agent_id: &str) -> String {
    let digest = coding_adventures_sha256::sha256(agent_id.as_bytes());
    let mut name = String::from("chief-of-staff.");
    for byte in &digest[..16] {
        name.push_str(&format!("{byte:02x}"));
    }
    name
}

/// The process mitigation policy, as `PROCESS_CREATION_MITIGATION_POLICY_*`
/// flags. `windows-sys` names only the first three, so the rest are written
/// from their documented positions (`WinBase.h`). From bit 8 each policy is
/// a two-bit field whose value `01` means "always on".
///
/// ```text
///   bits    policy                        effect
///   ------  ----------------------------  -----------------------------------
///   0       DEP (32-bit only)             no execution from data pages
///   2       SEHOP (32-bit only)           exception chains are validated
///   12-13   heap terminate                heap corruption ends the process
///   16-17   bottom-up ASLR                randomised bottom-up allocations
///   20-21   high-entropy ASLR             64-bit randomisation
///   24-25   strict handle checks          an invalid handle is fatal
///   28-29   win32k system calls off       no window manager or GDI at all
///   32-33   extension points off          no AppInit DLLs, hooks or IMEs
///   36-37   dynamic code prohibited       no new executable memory (ACG)
///   44-45   Microsoft-signed DLLs only    no third-party library loads
///   48-49   non-system fonts off          no font parsing from files
///   52-53   no remote images              no image from a network share
///   56-57   no low-label images           no image a low process wrote
///   60-61   system32 first                library search starts there
/// ```
pub const MITIGATIONS: u64 = THIRTY_TWO_BIT_ONLY
    | always_on(12)
    | always_on(16)
    | always_on(20)
    | always_on(24)
    | always_on(28)
    | always_on(32)
    | always_on(36)
    | always_on(44)
    | always_on(48)
    | always_on(52)
    | always_on(56)
    | always_on(60);

/// DEP and SEHOP, which mean something only to a 32-bit process: a 64-bit
/// one always has DEP, and its exception handling has no chain to forge.
const THIRTY_TWO_BIT_ONLY: u64 = if cfg!(target_pointer_width = "32") {
    MITIGATION_DEP | MITIGATION_SEHOP
} else {
    0
};

/// `PROCESS_CREATION_MITIGATION_POLICY_DEP_ENABLE`.
pub const MITIGATION_DEP: u64 = 0x01;
/// `PROCESS_CREATION_MITIGATION_POLICY_SEHOP_ENABLE`.
pub const MITIGATION_SEHOP: u64 = 0x04;

/// The "always on" value (`01`) of the two-bit policy field at `shift`.
pub const fn always_on(shift: u32) -> u64 {
    1u64 << shift
}

/// The variables an agent may be given, as on macOS: the locale, the time
/// zone and `NO_COLOR`. A launch that sets anything else is refused, never
/// filtered (S-I4a).
pub const GRANTABLE_ENVIRONMENT: [&str; 11] = [
    "TZ",
    "LANG",
    "LANGUAGE",
    "LC_ALL",
    "LC_COLLATE",
    "LC_CTYPE",
    "LC_MESSAGES",
    "LC_MONETARY",
    "LC_NUMERIC",
    "LC_TIME",
    "NO_COLOR",
];

/// Every variable is in [`GRANTABLE_ENVIRONMENT`] and has a plain value:
/// letters, digits and `._+-@,/`, not starting with `/`, with no `..`. So no
/// value is a Windows path (no `\` or `:`) or a Unix one.
pub fn check_environment<'a>(
    variables: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Result<(), ConfinementError> {
    for (name, value) in variables {
        if !GRANTABLE_ENVIRONMENT.contains(&name) {
            return Err(ConfinementError::Environment(format!(
                "{name}: not in the grantable set"
            )));
        }
        let plain = value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._+-@,/".contains(&byte));
        if !plain || value.starts_with('/') || value.contains("..") {
            return Err(ConfinementError::Environment(format!(
                "{name}: its value must be a plain name, not a path"
            )));
        }
    }
    Ok(())
}

/// The environment block `CreateProcessW` takes with
/// `CREATE_UNICODE_ENVIRONMENT`: `NAME=VALUE` strings, each ending in a
/// NUL, sorted by name without regard to case, and one more NUL. An empty
/// block is two NULs.
///
/// ```text
///   [("TZ","UTC"), ("SystemRoot","C:\Windows")]
///     →  "SystemRoot=C:\Windows\0TZ=UTC\0\0"   (as UTF-16)
/// ```
pub fn environment_block(variables: &[(String, String)]) -> Result<Vec<u16>, ConfinementError> {
    let mut sorted: Vec<&(String, String)> = variables.iter().collect();
    sorted.sort_by_key(|(name, _)| name.to_uppercase());
    let mut block = Vec::new();
    for (name, value) in sorted {
        if name.is_empty() || name.contains(['=', '\0']) || value.contains('\0') {
            return Err(ConfinementError::Environment(format!(
                "{name}: not a variable that can be passed"
            )));
        }
        block.extend(name.encode_utf16());
        block.push(u16::from(b'='));
        block.extend(value.encode_utf16());
        block.push(0);
    }
    if block.is_empty() {
        block.push(0);
    }
    block.push(0);
    Ok(block)
}

/// The command line for `program` and `arguments`, quoted so that the
/// Microsoft C runtime's parser (and Rust's `std::env::args`) gives back
/// exactly `arguments`. The program is always quoted, and may not hold a
/// `"`, which no Windows path can.
///
/// An argument with no space, tab, newline or `"`, and not empty, is
/// written as it is. Any other is wrapped in quotes. Inside the quotes,
/// backslashes count only before a `"` or the closing quote, where they
/// are doubled:
///
/// ```text
///   argument       written
///   -------------  ------------------
///   plain          plain
///   two words      "two words"
///   (empty)        ""
///   say "hi"       "say \"hi\""
///   C:\dir\        C:\dir\
///   C:\my dir\     "C:\my dir\\"
/// ```
pub fn command_line(program: &str, arguments: &[String]) -> Result<Vec<u16>, ConfinementError> {
    if program.contains(['"', '\0']) {
        return Err(ConfinementError::Argument(
            "the program path holds a quote or a NUL".to_string(),
        ));
    }
    let mut line: Vec<u16> = Vec::new();
    line.push(u16::from(b'"'));
    line.extend(program.encode_utf16());
    line.push(u16::from(b'"'));
    for argument in arguments {
        if argument.contains('\0') {
            return Err(ConfinementError::Argument(
                "an argument holds a NUL".to_string(),
            ));
        }
        line.push(u16::from(b' '));
        append_quoted(&mut line, argument);
    }
    line.push(0);
    Ok(line)
}

fn append_quoted(line: &mut Vec<u16>, argument: &str) {
    let needs_quotes = argument.is_empty() || argument.contains([' ', '\t', '\n', '\u{b}', '"']);
    if !needs_quotes {
        line.extend(argument.encode_utf16());
        return;
    }
    line.push(u16::from(b'"'));
    let mut backslashes = 0usize;
    for unit in argument.encode_utf16() {
        if unit == u16::from(b'\\') {
            backslashes += 1;
            continue;
        }
        if unit == u16::from(b'"') {
            // Each backslash before a quote is doubled, and the quote is
            // escaped with one more.
            line.extend(std::iter::repeat_n(u16::from(b'\\'), backslashes * 2 + 1));
        } else {
            line.extend(std::iter::repeat_n(u16::from(b'\\'), backslashes));
        }
        backslashes = 0;
        line.push(unit);
    }
    // Before the closing quote, backslashes are doubled too.
    line.extend(std::iter::repeat_n(u16::from(b'\\'), backslashes * 2));
    line.push(u16::from(b'"'));
}

/// The final path of a handle as `GetFinalPathNameByHandleW` gives it,
/// `\\?\C:\...`, as the plain `C:\...` a caller writes. A UNC final path
/// (`\\?\UNC\...`) is left as it is, so it can never equal a drive path.
pub fn strip_verbatim_prefix(path: &str) -> &str {
    match path.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC\\") => rest,
        _ => path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(units: &[u16]) -> String {
        String::from_utf16(units).unwrap()
    }

    #[test]
    fn a_container_name_is_the_agent_id_s_digest_and_fits() {
        let name = container_name("weather-reporter");
        assert!(name.starts_with("chief-of-staff."));
        assert_eq!(name.len(), 47);
        assert!(name[15..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
        assert_eq!(name, container_name("weather-reporter"));
        assert_ne!(name, container_name("weather-reporter2"));
        // Nothing in the id reaches the name but its digest.
        assert!(!container_name("..\\x").contains('\\'));
    }

    #[test]
    fn the_mitigation_bits_are_the_documented_ones() {
        // WinBase.h: PROCESS_CREATION_MITIGATION_POLICY_*_ALWAYS_ON.
        assert_eq!(always_on(12), 0x0000_0000_0000_1000); // HEAP_TERMINATE
        assert_eq!(always_on(16), 0x0000_0000_0001_0000); // BOTTOM_UP_ASLR
        assert_eq!(always_on(20), 0x0000_0000_0010_0000); // HIGH_ENTROPY_ASLR
        assert_eq!(always_on(24), 0x0000_0000_0100_0000); // STRICT_HANDLE_CHECKS
        assert_eq!(always_on(28), 0x0000_0000_1000_0000); // WIN32K_SYSTEM_CALL_DISABLE
        assert_eq!(always_on(32), 0x0000_0001_0000_0000); // EXTENSION_POINT_DISABLE
        assert_eq!(always_on(36), 0x0000_0010_0000_0000); // PROHIBIT_DYNAMIC_CODE
        assert_eq!(always_on(44), 0x0000_1000_0000_0000); // BLOCK_NON_MICROSOFT_BINARIES
        assert_eq!(always_on(48), 0x0001_0000_0000_0000); // FONT_DISABLE
        assert_eq!(always_on(52), 0x0010_0000_0000_0000); // IMAGE_LOAD_NO_REMOTE
        assert_eq!(always_on(56), 0x0100_0000_0000_0000); // IMAGE_LOAD_NO_LOW_LABEL
        assert_eq!(always_on(60), 0x1000_0000_0000_0000); // IMAGE_LOAD_PREFER_SYSTEM32
                                                          // No field is ever set to "always off" (10) or the reserved 11.
        for shift in (8..64).step_by(4) {
            assert_ne!((MITIGATIONS >> shift) & 0b11, 0b10, "bit {shift}");
            assert_ne!((MITIGATIONS >> shift) & 0b11, 0b11, "bit {shift}");
        }
        // Control flow guard (40) is left to the image: forcing it would
        // refuse an image built without it.
        assert_eq!((MITIGATIONS >> 40) & 0b11, 0);
    }

    #[test]
    fn arguments_come_back_exactly() {
        let line = |arguments: &[&str]| {
            let owned: Vec<String> = arguments.iter().map(|s| s.to_string()).collect();
            text(&command_line(r"C:\agents\probe.exe", &owned).unwrap())
        };
        assert_eq!(line(&[]), "\"C:\\agents\\probe.exe\"\0");
        assert_eq!(line(&["plain"]), "\"C:\\agents\\probe.exe\" plain\0");
        assert_eq!(
            line(&["two words"]),
            "\"C:\\agents\\probe.exe\" \"two words\"\0"
        );
        assert_eq!(line(&[""]), "\"C:\\agents\\probe.exe\" \"\"\0");
        assert_eq!(
            line(&["say \"hi\""]),
            "\"C:\\agents\\probe.exe\" \"say \\\"hi\\\"\"\0"
        );
        assert_eq!(line(&[r"C:\dir\"]), "\"C:\\agents\\probe.exe\" C:\\dir\\\0");
        assert_eq!(
            line(&[r"C:\my dir\"]),
            "\"C:\\agents\\probe.exe\" \"C:\\my dir\\\\\"\0"
        );
        assert_eq!(
            line(&[r#"a\"b"#]),
            "\"C:\\agents\\probe.exe\" \"a\\\\\\\"b\"\0"
        );
    }

    #[test]
    fn a_nul_or_a_quoted_program_is_refused() {
        assert!(command_line("C:\\a\"b.exe", &[]).is_err());
        assert!(command_line("C:\\a.exe", &["x\0y".to_string()]).is_err());
    }

    #[test]
    fn the_environment_is_closed_and_plain() {
        assert!(check_environment([("TZ", "Europe/London"), ("LANG", "en_GB.UTF-8")]).is_ok());
        for (name, value) in [
            ("PATH", "C:\\Windows"),
            ("CHIEF_OF_STAFF_SECRET", "x"),
            ("TZ", "C:\\zoneinfo"),
            ("TZ", "/etc/localtime"),
            ("LANG", "..\\x"),
            ("LANG", "a:b"),
        ] {
            assert!(
                check_environment([(name, value)]).is_err(),
                "{name}={value}"
            );
        }
    }

    #[test]
    fn the_block_is_sorted_and_terminated() {
        let block = environment_block(&[
            ("TZ".to_string(), "UTC".to_string()),
            ("SystemRoot".to_string(), r"C:\Windows".to_string()),
        ])
        .unwrap();
        assert_eq!(text(&block), "SystemRoot=C:\\Windows\0TZ=UTC\0\0");
        assert_eq!(text(&environment_block(&[]).unwrap()), "\0\0");
        assert!(environment_block(&[("A=B".to_string(), "c".to_string())]).is_err());
        assert!(environment_block(&[("A".to_string(), "c\0".to_string())]).is_err());
    }

    #[test]
    fn final_paths_lose_only_a_drive_prefix() {
        assert_eq!(strip_verbatim_prefix(r"\\?\C:\a\b.exe"), r"C:\a\b.exe");
        assert_eq!(
            strip_verbatim_prefix(r"\\?\UNC\server\share\b.exe"),
            r"\\?\UNC\server\share\b.exe"
        );
        assert_eq!(strip_verbatim_prefix(r"C:\a.exe"), r"C:\a.exe");
    }

    #[test]
    fn only_a_launchable_windows_plan_without_direct_grants_runs() {
        let plan = |os| {
            capability_os_sandbox::plan_from_json(
                r#"{"version":1,"package":"rust/probe","capabilities":[],"justification":"none"}"#,
                os,
            )
            .unwrap()
        };
        assert!(check_plan(&plan(OsFamily::Windows)).is_ok());
        assert_eq!(
            check_plan(&plan(OsFamily::Linux)),
            Err(ConfinementError::NotWindowsPlan)
        );
        let with_grant = capability_os_sandbox::plan_from_json(
            r#"{"version":1,"package":"rust/probe","capabilities":[{"category":"fs","action":"read","target":"C:\\data\\in.txt","justification":"input"}],"justification":"reads"}"#,
            OsFamily::Windows,
        )
        .unwrap();
        assert!(matches!(
            check_plan(&with_grant),
            Err(ConfinementError::Unsupported(_))
        ));
    }
}
