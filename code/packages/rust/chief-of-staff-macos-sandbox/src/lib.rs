//! # The macOS applier (D18S build step 7)
//!
//! `capability-os-sandbox` says what an agent may do. This crate makes the
//! macOS kernel enforce it, for a compiled agent, from its first
//! instruction. The Linux applier (`chief-of-staff-linux-sandbox`) installs
//! its filters between `fork` and `exec`. That cannot work on macOS, for
//! three reasons:
//!
//! - **There is no `execveat`.** Every exec is by path, so the file that
//!   runs is whatever the path names at that moment.
//! - **Seatbelt's install allocates.** `sandbox_init` compiles its profile
//!   from text. A forked child of the multi-threaded supervisor may only
//!   make async-signal-safe calls, because another thread may have held
//!   the allocator's lock at the fork.
//! - **There is nothing that can allow exactly one exec**, as Linux's
//!   seccomp listener does.
//!
//! So the install happens in a fresh process image of our own: **the
//! shim** (`chief-of-staff-seatbelt-shim`), a small binary pinned by
//! SHA-256 exactly as a broker is.
//!
//! ```text
//!   supervisor                           shim (fresh image)       agent
//!   -----------------------------------  -----------------------  -------
//!   MacosConfinement::prepare_verified
//!     re-verifies shim and agent digests
//!     checks both paths (see below)
//!     checks each Direct grant
//!     builds the profile text
//!   command()  -> Command::new(shim)
//!   apply(&mut command)
//!     checks the environment
//!     adds the shim's instructions
//!   spawn ──── fork, isolate, exec ───►  places inherited fds
//!                                        only 0-2 (+ them) open?
//!                                        RLIMIT_CORE = 0
//!                                        agent still that inode?
//!                                        sandbox_init(profile)
//!                                        launch probes (S-P4)
//!                                        agent still that inode?
//!                                        execve(agent) ─────────►  first
//!                                                                  instruction,
//!                                                                  confined
//! ```
//!
//! Seatbelt applies to the whole process (not one thread, as seccomp
//! does) and survives exec. So the agent's first instruction already runs
//! under the profile (S-I4c). Every failure in the shim exits with status
//! 127 without exec'ing (S-P3): an agent never runs unconfined.
//!
//! ## The profile: which operations exist at all
//!
//! It starts `(deny default)`. Then it allows only:
//!
//! ```text
//!   operation                           on
//!   ----------------------------------  ------------------------------------
//!   process-exec                        the agent's canonical path
//!   file-read*, file-map-executable     the agent; /usr/lib; /System/Library;
//!                                       /System/Volumes/Preboot/Cryptexes/OS;
//!                                       /System/Cryptexes/OS (dyld and its
//!                                       shared cache)
//!   file-read-data                      /dev/urandom, /dev/random
//!   file-read-data, file-write-data     /dev/null
//!   file-read-metadata                  any path (dyld stats its search path)
//!   sysctl-read                         libc reads hw.* and kern.* at start
//!   signal, process-info*               the agent itself only
//!   each Direct fs read grant           file-read-data on its literal path
//!   each Direct fs write grant          file-write-data on its literal path
//! ```
//!
//! Everything else is denied. That includes `mach-lookup` and
//! `mach-priv-task-port` (S-I5's macOS row), `process-fork`, `process-info*`
//! on any other process, `network*`, `ipc-posix*`, `iokit-open`, and every
//! other path. Every network grant is brokered (P2.1), so the profile has
//! no network rule at all.
//!
//! ## Exec by path, pinned to an inode
//!
//! The path is what runs, so [`check_executable_path`] requires three
//! things of it, for both the agent and the shim:
//!
//! - it is canonical: no symlink on the way, and no `.` or `..`;
//! - it names the same `(dev, ino)` as the descriptor whose digest was
//!   just verified;
//! - every directory above it is owned by root or this user, and no one
//!   else can write to it, unless it is sticky.
//!
//! The shim checks the agent's `(dev, ino)` again before installing the
//! profile, and once more immediately before `execve`. What remains is
//! recorded in the spec: between that last check and the exec, someone
//! able to write the directory could swap the file. The directory rule
//! leaves only root and an unsandboxed process of this same user, and both
//! are already the supervisor's equals.

#![deny(unsafe_op_in_unsafe_fn)]

use capability_cage::{Action, Category};
use capability_os_sandbox::{PlanRejection, SandboxCoverage, SandboxPlan};
use std::fmt;

#[cfg(unix)]
use std::path::Path;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::MacosConfinement;
#[cfg(target_os = "macos")]
#[doc(hidden)]
pub mod shim;

/// Why an agent cannot be confined. Every one of these refuses the launch
/// (S-P3): there is no "confine what we can" mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfinementError {
    /// The plan itself may not launch.
    Plan(Vec<PlanRejection>),
    /// The plan is for another OS.
    NotMacosPlan,
    /// A `Direct` grant this applier cannot express exactly, such as create
    /// or delete, which would need a rule over a whole directory.
    InexpressibleGrant(String),
    /// A `Direct` grant's path is not one this applier will allow.
    Grant(String),
    /// The agent or the shim failed its digest, or its path failed
    /// [`check_executable_path`].
    Executable(String),
    /// A string cannot be written as a Seatbelt literal.
    Literal(String),
    /// The command sets an environment variable outside the closed set
    /// (S-I4a).
    Environment(String),
    /// The command was not made by `MacosConfinement::command`, so it would
    /// not run the shim.
    Program,
    /// More descriptors to inherit than the shim places.
    Inherited(String),
}

impl fmt::Display for ConfinementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Plan(rejections) => write!(
                f,
                "sandbox plan cannot launch: {} reason(s)",
                rejections.len()
            ),
            Self::NotMacosPlan => f.write_str("sandbox plan is not for macOS"),
            Self::InexpressibleGrant(key) => {
                write!(f, "Seatbelt cannot express this grant exactly: {key}")
            }
            Self::Grant(why) => write!(f, "filesystem grant refused: {why}"),
            Self::Executable(why) => write!(f, "executable refused: {why}"),
            Self::Literal(why) => write!(f, "not a Seatbelt literal: {why}"),
            Self::Environment(why) => write!(f, "agent environment refused: {why}"),
            Self::Program => f.write_str("command does not run the Seatbelt shim"),
            Self::Inherited(why) => write!(f, "inherited descriptors refused: {why}"),
        }
    }
}

impl std::error::Error for ConfinementError {}

// ---------------------------------------------------------------------------
// Grants
// ---------------------------------------------------------------------------

/// A filesystem grant the profile will carry, taken from a `Direct` plan
/// rule: one exact path, read or written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileGrant {
    pub path: String,
    pub write: bool,
}

/// The plan's `Direct` filesystem grants, each one an exact path read or
/// written. Anything else `Direct` is refused: today only filesystem rules
/// can be `Direct` (capability-os-sandbox), and create, delete and list
/// would need rights over a whole directory. The same rule as Linux's.
pub fn direct_file_grants(plan: &SandboxPlan) -> Result<Vec<FileGrant>, ConfinementError> {
    let mut grants = Vec::new();
    for rule in &plan.rules {
        if rule.coverage != SandboxCoverage::Direct {
            continue;
        }
        let capability = &rule.capability;
        let write = match (capability.category, capability.action) {
            (Category::Fs, Action::Read) => false,
            (Category::Fs, Action::Write) => true,
            _ => return Err(ConfinementError::InexpressibleGrant(rule.capability_key())),
        };
        grants.push(FileGrant {
            path: capability.target.clone(),
            write,
        });
    }
    Ok(grants)
}

/// The directories every agent may read and map executable: dyld, the
/// system libraries, and the shared cache. A write grant may not land in
/// one, and a shim there would make its own read probe pass.
pub const SYSTEM_READ_ROOTS: [&str; 4] = [
    "/usr/lib",
    "/System/Library",
    "/System/Volumes/Preboot/Cryptexes/OS",
    "/System/Cryptexes/OS",
];

/// Whether `path` is one of [`SYSTEM_READ_ROOTS`] or beneath one. A
/// component-wise test: `/usr/library` is not under `/usr/lib`.
pub fn under_system_read_root(path: &str) -> bool {
    SYSTEM_READ_ROOTS.iter().any(|root| {
        path == *root
            || path
                .strip_prefix(root)
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

// ---------------------------------------------------------------------------
// The profile
// ---------------------------------------------------------------------------

/// Write `value` as the body of a Seatbelt string literal: `"` and `\` are
/// escaped, and a control character is refused rather than escaped, since
/// no path a grant may name contains one.
pub fn seatbelt_literal(value: &str) -> Result<String, ConfinementError> {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            ch if ch.is_control() => {
                return Err(ConfinementError::Literal(
                    "a control character in a path".to_string(),
                ))
            }
            ch => escaped.push(ch),
        }
    }
    Ok(escaped)
}

/// The Seatbelt profile for running the executable at `agent` (a canonical
/// path) with `grants` (canonical, checked paths): deny by default, and the
/// table in the crate documentation.
///
/// Pure text: the checks on the paths are [`check_executable_path`] and
/// `MacosConfinement`'s, made before this is called.
pub fn seatbelt_profile(agent: &str, grants: &[FileGrant]) -> Result<String, ConfinementError> {
    let agent = seatbelt_literal(agent)?;
    let mut profile = String::from("(version 1)\n(deny default)\n");
    // The one program this process may become. Exec'ing its own image
    // again gives it nothing: the profile survives the exec.
    profile.push_str(&format!("(allow process-exec (literal \"{agent}\"))\n"));
    profile.push_str(&format!(
        "(allow file-read* file-map-executable (literal \"{agent}\"))\n"
    ));
    profile.push_str("(allow file-read* file-map-executable");
    for root in SYSTEM_READ_ROOTS {
        profile.push_str(&format!(" (subpath \"{}\")", seatbelt_literal(root)?));
    }
    profile.push_str(")\n");
    profile.push_str("(allow file-read-metadata)\n");
    profile
        .push_str("(allow file-read-data (literal \"/dev/urandom\") (literal \"/dev/random\"))\n");
    profile.push_str("(allow file-read-data file-write-data (literal \"/dev/null\"))\n");
    profile.push_str("(allow sysctl-read)\n");
    profile.push_str("(allow signal (target self))\n");
    profile.push_str("(allow process-info* (target self))\n");
    for grant in grants {
        let operation = if grant.write {
            "file-write-data"
        } else {
            "file-read-data"
        };
        profile.push_str(&format!(
            "(allow {operation} (literal \"{}\"))\n",
            seatbelt_literal(&grant.path)?
        ));
    }
    Ok(profile)
}

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

/// Check that `path` is safe to execute by path, and return its
/// `(dev, ino)`.
///
/// - It is absolute and canonical: `fs::canonicalize` gives it back
///   unchanged, so no component is a symlink, `.` or `..`.
/// - It is a regular file, and if `opened` is given, the same file
///   (`(dev, ino)`) as that open descriptor.
/// - Every directory above it is owned by root or this process's user,
///   and is writable by no one else unless it is sticky. Someone who could
///   write a directory on the way could rename a different file into
///   place; a sticky directory lets no one rename an entry they do not own.
#[cfg(unix)]
pub fn check_executable_path(
    path: &Path,
    opened: Option<&std::fs::File>,
) -> Result<(u64, u64), ConfinementError> {
    use std::os::unix::fs::MetadataExt;
    let refuse = |why: &str| ConfinementError::Executable(format!("{}: {why}", path.display()));
    if !path.is_absolute() {
        return Err(refuse("not absolute"));
    }
    let canonical = std::fs::canonicalize(path).map_err(|_| refuse("cannot be resolved"))?;
    if canonical != path {
        return Err(refuse("not canonical (a symlink, `.` or `..` on the way)"));
    }
    let named = std::fs::symlink_metadata(path).map_err(|_| refuse("cannot be inspected"))?;
    if !named.file_type().is_file() {
        return Err(refuse("not a regular file"));
    }
    if let Some(file) = opened {
        let held = file.metadata().map_err(|_| refuse("cannot be inspected"))?;
        if (held.dev(), held.ino()) != (named.dev(), named.ino()) {
            return Err(refuse("the path no longer names the verified file"));
        }
    }
    check_ancestors(path).map_err(|why| refuse(&why))?;
    Ok((named.dev(), named.ino()))
}

/// Every directory above `path`: owned by root or this user, and writable
/// by no one else unless sticky.
#[cfg(unix)]
fn check_ancestors(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::MetadataExt;
    // SAFETY: geteuid has no preconditions and touches no memory.
    let me = unsafe { libc::geteuid() };
    for directory in path.ancestors().skip(1) {
        let metadata = std::fs::symlink_metadata(directory)
            .map_err(|_| format!("{} cannot be inspected", directory.display()))?;
        if !metadata.file_type().is_dir() {
            return Err(format!("{} is not a directory", directory.display()));
        }
        if metadata.uid() != 0 && metadata.uid() != me {
            return Err(format!("{} is owned by someone else", directory.display()));
        }
        let shared = metadata.mode() & 0o022 != 0;
        let sticky = metadata.mode() & 0o1000 != 0;
        if shared && !sticky {
            return Err(format!(
                "{} is writable by others and not sticky",
                directory.display()
            ));
        }
    }
    Ok(())
}

/// Check one `Direct` grant and return its `(dev, ino)`: an absolute,
/// canonical path naming an existing regular file. Seatbelt matches the
/// resolved path, so a literal that is not canonical (`/var/...` for
/// `/private/var/...`) would never match; refusing it says so at launch
/// rather than as a puzzling denial later. A directory would be a
/// wildcard in disguise, and is refused too.
#[cfg(unix)]
pub fn check_grant(grant: &FileGrant) -> Result<(u64, u64), ConfinementError> {
    use std::os::unix::fs::MetadataExt;
    let path = Path::new(&grant.path);
    let refuse = |why: &str| ConfinementError::Grant(format!("{}: {why}", grant.path));
    if !path.is_absolute() {
        return Err(refuse("not absolute"));
    }
    let canonical = std::fs::canonicalize(path).map_err(|_| refuse("does not exist"))?;
    if canonical != path {
        return Err(refuse("not canonical (a symlink, `.` or `..` on the way)"));
    }
    let metadata = std::fs::symlink_metadata(path).map_err(|_| refuse("cannot be inspected"))?;
    if !metadata.file_type().is_file() {
        return Err(refuse("not a regular file"));
    }
    if grant.write && under_system_read_root(&grant.path) {
        return Err(refuse("a write grant inside a system library directory"));
    }
    Ok((metadata.dev(), metadata.ino()))
}

// ---------------------------------------------------------------------------
// The environment (S-I4a)
// ---------------------------------------------------------------------------

/// The environment variables an agent may be given: a closed set, the same
/// as Linux's (`chief-of-staff-linux-sandbox`). Anything else, and above
/// all `DYLD_*`, which makes dyld load code into the shim and the agent, is
/// refused.
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

/// Refuse any name outside the closed set, and any value that is not a
/// plain name: letters, digits and `._+-@,`, with `/` allowed only inside
/// (`America/New_York`) and never `..`. A value with a path in it makes
/// libc read that file at start.
pub fn check_environment<'a>(
    variables: impl IntoIterator<Item = (&'a [u8], &'a [u8])>,
) -> Result<(), ConfinementError> {
    for (name, value) in variables {
        let shown = || String::from_utf8_lossy(name).into_owned();
        if !GRANTABLE_ENVIRONMENT
            .iter()
            .any(|granted| granted.as_bytes() == name)
        {
            return Err(ConfinementError::Environment(format!(
                "{}: not in the grantable set",
                shown()
            )));
        }
        let plain = value
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._+-@,/".contains(byte));
        if !plain || value.starts_with(b"/") || value.windows(2).any(|pair| pair == b"..") {
            return Err(ConfinementError::Environment(format!(
                "{}: its value must be a plain name, not a path",
                shown()
            )));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// The shim's instructions
// ---------------------------------------------------------------------------

/// The environment variable that carries the shim's instructions. The shim
/// removes it before anything else, so the agent never sees it. It is not
/// secret: the profile and the paths are all it holds.
pub const INSTRUCTIONS_VARIABLE: &str = "CHIEF_OF_STAFF_SEATBELT_SHIM";

/// At most this many inherited descriptors, as on Linux.
pub const MAX_INHERITED: usize = 64;

/// Everything the shim needs, written into [`INSTRUCTIONS_VARIABLE`].
///
/// ```text
///   1;<profile hex>;<agent path hex>;<dev>;<ino>;<shim path hex>;<fd>,<fd>,...
/// ```
///
/// Hex keeps the profile's quotes, newlines and semicolons out of the
/// framing. The version comes first so a shim and a supervisor of
/// different builds refuse each other rather than misread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShimInstructions {
    pub profile: String,
    pub agent: String,
    pub agent_dev: u64,
    pub agent_ino: u64,
    pub shim: String,
    /// The descriptors to place at 3, 4, ..., in order.
    pub inherited: Vec<i32>,
}

impl ShimInstructions {
    pub fn encode(&self) -> String {
        let fds = self
            .inherited
            .iter()
            .map(i32::to_string)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "1;{};{};{};{};{};{}",
            hex(self.profile.as_bytes()),
            hex(self.agent.as_bytes()),
            self.agent_dev,
            self.agent_ino,
            hex(self.shim.as_bytes()),
            fds
        )
    }

    /// The inverse of [`Self::encode`]. Anything malformed, including a
    /// descriptor below 3 or more than [`MAX_INHERITED`] of them, is `None`.
    pub fn decode(text: &str) -> Option<Self> {
        let fields: Vec<&str> = text.split(';').collect();
        let [version, profile, agent, dev, ino, shim, fds] = fields.as_slice() else {
            return None;
        };
        if *version != "1" {
            return None;
        }
        let inherited = if fds.is_empty() {
            Vec::new()
        } else {
            fds.split(',')
                .map(|fd| fd.parse::<i32>().ok().filter(|fd| *fd > 2))
                .collect::<Option<Vec<_>>>()?
        };
        if inherited.len() > MAX_INHERITED {
            return None;
        }
        Some(Self {
            profile: String::from_utf8(unhex(profile)?).ok()?,
            agent: String::from_utf8(unhex(agent)?).ok()?,
            agent_dev: dev.parse().ok()?,
            agent_ino: ino.parse().ok()?,
            shim: String::from_utf8(unhex(shim)?).ok()?,
            inherited,
        })
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[usize::from(byte >> 4)] as char);
        out.push(DIGITS[usize::from(byte & 0xf)] as char);
    }
    out
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    let digit = |byte: u8| match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    };
    text.as_bytes()
        .chunks(2)
        .map(|pair| Some(digit(pair[0])? << 4 | digit(pair[1])?))
        .collect()
}

/// Which enforcement classes each launch confirms, and which only CI
/// does (S-P4), for the audit record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchVerification {
    pub at_launch: &'static [&'static str],
    pub ci_only: &'static [&'static str],
}

/// What the shim confirms at every launch, and what only the
/// `macos-latest` tests do.
pub const VERIFICATION: LaunchVerification = LaunchVerification {
    at_launch: &[
        "seatbelt: reading the shim's own image is denied (EPERM)",
        "seatbelt: a TCP connect to 127.0.0.1:1 is denied (EPERM, not ECONNREFUSED)",
        "only fds 0-2 and the inherited ones survive the exec (S-I4d)",
        "RLIMIT_CORE is 0",
        "the agent's path names the verified inode",
    ],
    ci_only: &[
        "seatbelt: ungranted reads and writes, fork, a second exec, signals and process info for other processes, and every connect are denied",
        "seatbelt: Direct grants are readable or writable as granted",
    ],
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_profile_denies_by_default_and_names_each_grant() {
        let grants = [
            FileGrant {
                path: "/Users/a/in.txt".into(),
                write: false,
            },
            FileGrant {
                path: "/Users/a/out.txt".into(),
                write: true,
            },
        ];
        let profile = seatbelt_profile("/opt/agent", &grants).unwrap();
        assert!(profile.starts_with("(version 1)\n(deny default)\n"));
        assert!(profile.contains("(allow process-exec (literal \"/opt/agent\"))"));
        assert!(profile.contains("(allow file-read-data (literal \"/Users/a/in.txt\"))"));
        assert!(profile.contains("(allow file-write-data (literal \"/Users/a/out.txt\"))"));
        assert!(!profile.contains("allow default"));
        assert!(!profile.contains("network"));
        assert!(!profile.contains("mach-lookup"));
        assert!(!profile.contains("process-fork"));
        // A read grant is not writable, nor a write grant readable.
        assert!(!profile.contains("file-write-data (literal \"/Users/a/in.txt\")"));
        assert!(!profile.contains("file-read-data (literal \"/Users/a/out.txt\")"));
    }

    #[test]
    fn literals_are_escaped_and_control_characters_refused() {
        assert_eq!(seatbelt_literal("a\"b\\c").unwrap(), "a\\\"b\\\\c");
        for bad in ["a\nb", "a\0b", "a\rb", "a\u{7f}b"] {
            assert!(matches!(
                seatbelt_literal(bad),
                Err(ConfinementError::Literal(_))
            ));
        }
        // A quote cannot end the literal and start a rule of its own.
        let profile = seatbelt_profile("/x\") (allow default", &[]).unwrap();
        assert!(profile.contains("(literal \"/x\\\") (allow default\")"));
    }

    #[test]
    fn system_read_roots_match_whole_components() {
        assert!(under_system_read_root("/usr/lib"));
        assert!(under_system_read_root("/usr/lib/libSystem.B.dylib"));
        assert!(under_system_read_root("/System/Library/x"));
        assert!(!under_system_read_root("/usr/library"));
        assert!(!under_system_read_root("/usr/local/lib/x"));
    }

    #[test]
    fn instructions_round_trip_and_refuse_malformed_text() {
        let instructions = ShimInstructions {
            profile: "(version 1)\n(deny default)\n; \"quoted\"".into(),
            agent: "/a;b/agent".into(),
            agent_dev: 16_777_234,
            agent_ino: 99,
            shim: "/opt/shim".into(),
            inherited: vec![7, 300],
        };
        let text = instructions.encode();
        assert_eq!(ShimInstructions::decode(&text), Some(instructions.clone()));
        let none = ShimInstructions {
            inherited: Vec::new(),
            ..instructions
        };
        assert_eq!(ShimInstructions::decode(&none.encode()), Some(none));
        for bad in [
            "",
            "2;;;1;1;;",
            "1;zz;;1;1;;",
            "1;;;1;1;",
            "1;;;x;1;;",
            "1;;;1;1;;2",
            "1;;;1;1;;-4",
            "1;;;1;1;;5,,6",
            "1;a;;1;1;;",
        ] {
            assert_eq!(ShimInstructions::decode(bad), None, "{bad:?}");
        }
        let many = (3..3 + MAX_INHERITED as i32 + 1)
            .map(|fd| fd.to_string())
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(ShimInstructions::decode(&format!("1;;;1;1;;{many}")), None);
    }

    #[test]
    fn the_environment_is_a_closed_set_of_plain_values() {
        assert!(check_environment([(b"TZ".as_slice(), b"America/New_York".as_slice())]).is_ok());
        for (name, value) in [
            (b"DYLD_INSERT_LIBRARIES".as_slice(), b"x".as_slice()),
            (b"HOME", b"x"),
            (b"TZ", b"/etc/localtime"),
            (b"LANG", b"../../x"),
            (b"LANG", b"en US"),
        ] {
            assert!(
                check_environment([(name, value)]).is_err(),
                "{}",
                String::from_utf8_lossy(name)
            );
        }
    }

    #[test]
    fn only_exact_file_reads_and_writes_are_direct_grants() {
        let plan = |capabilities: &str| {
            capability_os_sandbox::plan_from_json(
                &format!(
                    r#"{{"version":1,"package":"rust/t","capabilities":[{capabilities}],"justification":"test"}}"#
                ),
                capability_os_sandbox::OsFamily::Macos,
            )
            .unwrap()
        };
        // One plan may not both read and write (read/write separation),
        // so each is its own.
        let read = direct_file_grants(&plan(
            r#"{"category":"fs","action":"read","target":"/a/in","justification":"t"}"#,
        ))
        .unwrap();
        assert_eq!(
            read,
            vec![FileGrant {
                path: "/a/in".into(),
                write: false
            }]
        );
        let write = direct_file_grants(&plan(
            r#"{"category":"fs","action":"write","target":"/a/out","justification":"t"}"#,
        ))
        .unwrap();
        assert_eq!(
            write,
            vec![FileGrant {
                path: "/a/out".into(),
                write: true
            }]
        );
        // Brokered rules are not Direct grants and are skipped.
        let brokered = direct_file_grants(&plan(
            r#"{"category":"net","action":"connect","target":"example.com:443","justification":"t"}"#,
        ))
        .unwrap();
        assert!(brokered.is_empty());
    }

    #[cfg(unix)]
    mod paths {
        use super::super::*;
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        use std::path::PathBuf;
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct Scratch(PathBuf);

        impl Scratch {
            fn new() -> Self {
                static NEXT: AtomicUsize = AtomicUsize::new(0);
                let base = fs::canonicalize(std::env::temp_dir()).unwrap();
                let path = base.join(format!(
                    "macos-sandbox-unit-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ));
                fs::create_dir_all(&path).unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
                Self(path)
            }
        }

        impl Drop for Scratch {
            fn drop(&mut self) {
                let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o700));
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        /// The CI checkout may sit under a directory another user can
        /// write. Then every path below it is refused, rightly, and the
        /// positive cases cannot be shown there.
        fn ancestors_are_private(path: &Path) -> bool {
            check_ancestors(&path.join("x")).is_ok()
        }

        #[test]
        fn an_executable_path_must_be_canonical_and_the_same_file() {
            let scratch = Scratch::new();
            if !ancestors_are_private(&scratch.0) {
                eprintln!("skipped: the scratch directory's ancestors are shared");
                return;
            }
            let file = scratch.0.join("agent");
            fs::write(&file, b"x").unwrap();
            let opened = fs::File::open(&file).unwrap();
            assert!(check_executable_path(&file, Some(&opened)).is_ok());

            // Through a symlink: not canonical.
            let link = scratch.0.join("link");
            std::os::unix::fs::symlink(&file, &link).unwrap();
            assert!(check_executable_path(&link, None).is_err());
            // With a `..`: not canonical.
            let dotted = scratch.0.join("sub/../agent");
            fs::create_dir(scratch.0.join("sub")).unwrap();
            assert!(check_executable_path(&dotted, None).is_err());
            // Relative: refused.
            assert!(check_executable_path(Path::new("agent"), None).is_err());
            // A directory: refused.
            assert!(check_executable_path(&scratch.0.join("sub"), None).is_err());

            // Replaced after it was opened: the path names another inode.
            fs::rename(&file, scratch.0.join("old")).unwrap();
            fs::write(&file, b"y").unwrap();
            assert!(matches!(
                check_executable_path(&file, Some(&opened)),
                Err(ConfinementError::Executable(why)) if why.contains("no longer")
            ));
        }

        #[test]
        fn a_directory_others_can_write_is_refused_unless_sticky() {
            let scratch = Scratch::new();
            if !ancestors_are_private(&scratch.0) {
                eprintln!("skipped: the scratch directory's ancestors are shared");
                return;
            }
            let shared = scratch.0.join("shared");
            fs::create_dir(&shared).unwrap();
            let file = shared.join("agent");
            fs::write(&file, b"x").unwrap();
            fs::set_permissions(&shared, fs::Permissions::from_mode(0o777)).unwrap();
            assert!(matches!(
                check_executable_path(&file, None),
                Err(ConfinementError::Executable(why)) if why.contains("not sticky")
            ));
            fs::set_permissions(&shared, fs::Permissions::from_mode(0o1777)).unwrap();
            assert!(check_executable_path(&file, None).is_ok());
            fs::set_permissions(&shared, fs::Permissions::from_mode(0o775)).unwrap();
            assert!(check_executable_path(&file, None).is_err());
        }

        #[test]
        fn a_grant_must_be_an_existing_canonical_regular_file() {
            let scratch = Scratch::new();
            let file = scratch.0.join("data");
            fs::write(&file, b"x").unwrap();
            let grant = |path: &Path, write: bool| FileGrant {
                path: path.to_str().unwrap().to_string(),
                write,
            };
            assert!(check_grant(&grant(&file, false)).is_ok());
            assert!(check_grant(&grant(&file, true)).is_ok());
            let link = scratch.0.join("link");
            std::os::unix::fs::symlink(&file, &link).unwrap();
            assert!(check_grant(&grant(&link, false)).is_err());
            assert!(check_grant(&grant(&scratch.0, false)).is_err());
            assert!(check_grant(&grant(&scratch.0.join("missing"), false)).is_err());
            assert!(check_grant(&grant(Path::new("data"), false)).is_err());
        }
    }
}
