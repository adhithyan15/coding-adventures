//! The supervisor's half of the macOS applier: check everything, write the
//! profile, and hand the shim its instructions. The install itself is the
//! shim's (`shim.rs`), in a fresh image.

use crate::{
    check_environment, check_executable_path, check_grant, direct_file_grants, seatbelt_profile,
    ConfinementError, LaunchVerification, ShimInstructions, INSTRUCTIONS_VARIABLE, MAX_INHERITED,
    VERIFICATION,
};
use capability_os_sandbox::{OsFamily, SandboxPlan};
use chief_of_staff_spawn_isolation::VerifiedExecutable;
use std::ffi::{c_char, CString};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Parked inherited descriptors sit at this number or above until the
/// shim moves them. Any free number will do: the shim stages them above
/// every source and target before placing them. A higher floor would fail
/// under macOS's default soft `RLIMIT_NOFILE` of 256.
const PARK_FLOOR: i32 = 3;

/// A prepared confinement: the profile and the shim's instructions, built
/// and checked in the supervisor, ready for each spawn.
#[derive(Debug, Clone)]
pub struct MacosConfinement {
    shim: PathBuf,
    instructions: ShimInstructions,
}

impl MacosConfinement {
    /// Check `plan`, `agent` and `shim`, and write the profile for running
    /// `agent` under `plan`.
    ///
    /// Both binaries are verified again through their descriptors, and
    /// both paths must pass [`check_executable_path`] against those same
    /// descriptors: the path is what runs. Each `Direct` grant must pass
    /// [`check_grant`], and may name neither binary. A launcher prepares a
    /// confinement for each launch, so the digests are checked each time.
    pub fn prepare_verified(
        plan: &SandboxPlan,
        agent: &VerifiedExecutable,
        shim: &VerifiedExecutable,
    ) -> Result<Self, ConfinementError> {
        if plan.os != OsFamily::Macos {
            return Err(ConfinementError::NotMacosPlan);
        }
        plan.launch_preconditions()
            .map_err(ConfinementError::Plan)?;
        let grants = direct_file_grants(plan)?;
        let (agent_dev, agent_ino) = verified_path(agent)?;
        let shim_identity = verified_path(shim)?;
        if (agent_dev, agent_ino) == shim_identity {
            return Err(ConfinementError::Executable(
                "the agent is the shim".to_string(),
            ));
        }
        let agent_path = utf8(agent.path())?;
        let shim_path = utf8(shim.path())?;
        if crate::under_system_read_root(&shim_path) {
            // Its read probe would pass whether or not the profile is in
            // force.
            return Err(ConfinementError::Executable(
                "the shim is inside a system library directory".to_string(),
            ));
        }
        let mut pinned = Vec::with_capacity(grants.len());
        for grant in &grants {
            let (dev, ino) = check_grant(grant)?;
            if (dev, ino) == (agent_dev, agent_ino) || (dev, ino) == shim_identity {
                // A writable image would let the agent rewrite its code
                // or the shim (S-I6).
                return Err(ConfinementError::Grant(format!(
                    "{}: names the agent or the shim",
                    grant.path
                )));
            }
            pinned.push((grant.path.clone(), dev, ino));
        }
        let profile = seatbelt_profile(&agent_path, &grants)?;
        Ok(Self {
            shim: shim.path().to_path_buf(),
            instructions: ShimInstructions {
                profile,
                agent: agent_path,
                agent_dev,
                agent_ino,
                shim: shim_path,
                inherited: Vec::new(),
                grants: pinned,
            },
        })
    }

    /// The profile the shim will install, for the audit record and tests.
    pub fn profile(&self) -> &str {
        &self.instructions.profile
    }

    /// Isolate `command`'s descriptors (`chief-of-staff-spawn-isolation`),
    /// then confine it. Call it last, after stdin, stdout, the arguments and
    /// the environment are set.
    ///
    /// What runs is always the shim, and then the agent `prepare_verified`
    /// checked; the command's program is ignored, and the agent's argv\[0\]
    /// is its own path. argv and the environment are taken when `apply`
    /// runs, as on Linux: arguments or variables added afterwards are not
    /// passed. Only what was set on the command is passed, never the
    /// supervisor's own environment, and it refuses (S-I4a) any name
    /// outside `GRANTABLE_ENVIRONMENT`.
    ///
    /// On `Err` the command is poisoned: spawning it anyway fails, so an
    /// ignored error can never launch anything unconfined.
    pub fn apply<'a>(&self, command: &'a mut Command) -> Result<&'a mut Command, ConfinementError> {
        self.install(command, Vec::new())
    }

    /// [`Self::apply`], and the agent also holds exactly `inherited`, at
    /// descriptors 3, 4, ... in order, and nothing else above stderr
    /// (D18S S-I3: a broker's keys). At most 64.
    ///
    /// They are parked now, as close-on-exec duplicates, and the caller's
    /// copies closed. In the forked child, after spawn isolation, the hook
    /// clears close-on-exec on exactly the parked ones, so they reach the
    /// shim, which moves them into place. The command holds them until it
    /// is dropped: spawn it once, since every spawn of it would receive
    /// them.
    pub fn apply_inheriting<'a>(
        &self,
        command: &'a mut Command,
        inherited: Vec<OwnedFd>,
    ) -> Result<&'a mut Command, ConfinementError> {
        self.install(command, inherited)
    }

    /// Which enforcement classes each launch confirms, and which only CI
    /// does (S-P4), for the audit record.
    pub fn launch_verification(&self) -> LaunchVerification {
        VERIFICATION
    }

    fn install<'a>(
        &self,
        command: &'a mut Command,
        inherited: Vec<OwnedFd>,
    ) -> Result<&'a mut Command, ConfinementError> {
        match self.prepare_exec(command, inherited) {
            Ok(exec) => {
                chief_of_staff_spawn_isolation::isolate(command);
                // SAFETY: the closure runs in the forked child before exec,
                // after spawn isolation's hook. It calls only `fcntl` and
                // `execve`, both async-signal-safe, allocates nothing, and
                // reads only `exec`, which it owns: every buffer was built
                // here, in the parent.
                unsafe { command.pre_exec(move || exec.run()) };
                Ok(command)
            }
            Err(error) => {
                poison(command);
                Err(error)
            }
        }
    }

    /// Check the command and build everything the child's hook needs.
    fn prepare_exec(
        &self,
        command: &Command,
        inherited: Vec<OwnedFd>,
    ) -> Result<ShimExec, ConfinementError> {
        if inherited.len() > MAX_INHERITED {
            return Err(ConfinementError::Inherited(format!(
                "{} descriptors; at most {MAX_INHERITED}",
                inherited.len()
            )));
        }
        let variables: Vec<(Vec<u8>, Vec<u8>)> = command
            .get_envs()
            .filter_map(|(name, value)| {
                value.map(|value| (name.as_bytes().to_vec(), value.as_bytes().to_vec()))
            })
            .collect();
        check_environment(
            variables
                .iter()
                .map(|(name, value)| (name.as_slice(), value.as_slice())),
        )?;
        let parked = park(inherited)?;
        let instructions = ShimInstructions {
            inherited: parked.iter().map(AsRawFd::as_raw_fd).collect(),
            ..self.instructions.clone()
        };

        let mut argv = vec![cstring(self.shim.as_os_str().as_bytes())?];
        for argument in command.get_args() {
            argv.push(cstring(argument.as_bytes())?);
        }
        let mut envp = Vec::with_capacity(variables.len() + 1);
        for (name, value) in &variables {
            envp.push(cstring(
                &[name.as_slice(), b"=", value.as_slice()].concat(),
            )?);
        }
        envp.push(cstring(
            format!("{INSTRUCTIONS_VARIABLE}={}", instructions.encode()).as_bytes(),
        )?);
        Ok(ShimExec::new(
            cstring(self.shim.as_os_str().as_bytes())?,
            argv,
            envp,
            parked,
        ))
    }
}

/// Everything the child's hook uses, built in the parent: the shim's path,
/// argv and envp as C strings with their pointer arrays, and the parked
/// descriptors.
struct ShimExec {
    program: CString,
    _argv: Vec<CString>,
    _envp: Vec<CString>,
    argv_pointers: Vec<*const c_char>,
    envp_pointers: Vec<*const c_char>,
    parked: Vec<OwnedFd>,
}

// SAFETY: the raw pointers point into the CStrings this struct owns, whose
// heap buffers never move or change after construction, and nothing
// writes through them. Sharing or sending the struct shares only reads.
unsafe impl Send for ShimExec {}
// SAFETY: as above.
unsafe impl Sync for ShimExec {}

impl ShimExec {
    fn new(program: CString, argv: Vec<CString>, envp: Vec<CString>, parked: Vec<OwnedFd>) -> Self {
        let pointers = |strings: &[CString]| {
            let mut pointers: Vec<*const c_char> =
                strings.iter().map(|string| string.as_ptr()).collect();
            pointers.push(std::ptr::null());
            pointers
        };
        Self {
            argv_pointers: pointers(&argv),
            envp_pointers: pointers(&envp),
            program,
            _argv: argv,
            _envp: envp,
            parked,
        }
    }

    /// In the forked child: let the parked descriptors survive the exec,
    /// and exec the shim. Returns only on failure, which fails the spawn.
    fn run(&self) -> std::io::Result<()> {
        for fd in &self.parked {
            let fd = fd.as_raw_fd();
            // SAFETY: F_GETFD/F_SETFD read and write only this descriptor's
            // flags.
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
            if flags == -1
                // SAFETY: as above.
                || unsafe { libc::fcntl(fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC) } == -1
            {
                return Err(std::io::Error::last_os_error());
            }
        }
        // SAFETY: every pointer is to a NUL-terminated string owned by
        // `self`, and both arrays end with a null pointer.
        unsafe {
            libc::execve(
                self.program.as_ptr(),
                self.argv_pointers.as_ptr(),
                self.envp_pointers.as_ptr(),
            )
        };
        Err(std::io::Error::last_os_error())
    }
}

/// Duplicate each descriptor close-on-exec at [`PARK_FLOOR`] or above, and
/// close the caller's copies: nothing spawned meanwhile, by any path, can
/// inherit them.
fn park(inherited: Vec<OwnedFd>) -> Result<Vec<OwnedFd>, ConfinementError> {
    inherited
        .into_iter()
        .map(|fd| {
            // SAFETY: F_DUPFD_CLOEXEC duplicates an open descriptor at or
            // above the floor, or fails.
            let parked = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_DUPFD_CLOEXEC, PARK_FLOOR) };
            if parked == -1 {
                return Err(ConfinementError::Inherited(format!(
                    "cannot park descriptor {}: {}",
                    fd.as_raw_fd(),
                    std::io::Error::last_os_error()
                )));
            }
            // SAFETY: a new descriptor this process now owns alone.
            Ok(unsafe { OwnedFd::from_raw_fd(parked) })
        })
        .collect()
}

/// Make `command` fail to spawn, whatever the caller does next.
fn poison(command: &mut Command) {
    // SAFETY: the closure only builds an error value, which does not
    // allocate.
    unsafe { command.pre_exec(|| Err(std::io::Error::from_raw_os_error(libc::EPERM))) };
}

fn cstring(bytes: &[u8]) -> Result<CString, ConfinementError> {
    CString::new(bytes).map_err(|_| ConfinementError::Argument("a NUL byte".to_string()))
}

/// Re-verify `executable` through its descriptor, then check its path
/// against that descriptor.
fn verified_path(executable: &VerifiedExecutable) -> Result<(u64, u64), ConfinementError> {
    executable
        .verify()
        .map_err(|error| ConfinementError::Executable(error.to_string()))?;
    let held = executable
        .descriptor()
        .map_err(|error| ConfinementError::Executable(error.to_string()))?;
    check_executable_path(executable.path(), Some(&held))
}

fn utf8(path: &Path) -> Result<String, ConfinementError> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| ConfinementError::Literal(format!("{} is not UTF-8", path.display())))
}
