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
use std::os::fd::{AsRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    /// [`check_grant`], and may name neither binary.
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
        for grant in &grants {
            let identity = check_grant(grant)?;
            if identity == (agent_dev, agent_ino) || identity == shim_identity {
                // A writable image would let the agent rewrite its code
                // or the shim (S-I6).
                return Err(ConfinementError::Grant(format!(
                    "{}: names the agent or the shim",
                    grant.path
                )));
            }
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
            },
        })
    }

    /// A command that runs the shim. Add the agent's arguments, stdin,
    /// stdout and environment, then call [`Self::apply`] last. The agent's
    /// argv\[0\] is its own path; the arguments follow it.
    pub fn command(&self) -> Command {
        Command::new(&self.shim)
    }

    /// The profile the shim will install, for the audit record and tests.
    pub fn profile(&self) -> &str {
        &self.instructions.profile
    }

    /// Isolate `command`'s descriptors (`chief-of-staff-spawn-isolation`),
    /// check its environment, and give it the shim's instructions. Call it
    /// last.
    ///
    /// It refuses a command not made by [`Self::command`], and (S-I4a) one
    /// whose environment names anything outside `GRANTABLE_ENVIRONMENT`.
    /// The parent's own environment is never inherited: only what was set
    /// on the command is passed. On `Err` the command is unchanged; were it
    /// spawned anyway, the shim would find no instructions and exit 127.
    pub fn apply<'a>(&self, command: &'a mut Command) -> Result<&'a mut Command, ConfinementError> {
        self.install(command, Vec::new())
    }

    /// [`Self::apply`], and the agent also holds exactly `inherited`, at
    /// descriptors 3, 4, ... in order, and nothing else above stderr
    /// (D18S S-I3: a broker's keys). At most 64.
    ///
    /// The command keeps the descriptors until it is dropped. In the
    /// forked child, after spawn isolation has marked everything above 2
    /// close-on-exec, a hook clears that flag on exactly these, so they
    /// reach the shim, which moves them into place.
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
        // Every check first; the command is touched only once all pass.
        if Path::new(command.get_program()) != self.shim {
            return Err(ConfinementError::Program);
        }
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
        let numbers: Vec<i32> = inherited.iter().map(AsRawFd::as_raw_fd).collect();
        if numbers.iter().any(|fd| *fd <= 2) {
            return Err(ConfinementError::Inherited(
                "a descriptor below 3 cannot be inherited".to_string(),
            ));
        }
        let instructions = ShimInstructions {
            inherited: numbers.clone(),
            ..self.instructions.clone()
        };

        // Only what was set on the command, never the supervisor's own
        // environment.
        command.env_clear();
        for (name, value) in &variables {
            command.env(
                std::ffi::OsStr::from_bytes(name),
                std::ffi::OsStr::from_bytes(value),
            );
        }
        command.env(INSTRUCTIONS_VARIABLE, instructions.encode());
        chief_of_staff_spawn_isolation::isolate(command);
        if !inherited.is_empty() {
            // SAFETY: the closure runs in the forked child before exec, so
            // it calls only `fcntl`, which is async-signal-safe, allocates
            // nothing, and reads only `numbers`, which it owns. `held`
            // keeps the descriptors open, and so their numbers ours, for
            // as long as the command lives.
            let held = inherited;
            unsafe {
                command.pre_exec(move || {
                    let _ = &held;
                    for &fd in &numbers {
                        let flags = libc::fcntl(fd, libc::F_GETFD);
                        if flags == -1
                            || libc::fcntl(fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC) == -1
                        {
                            return Err(std::io::Error::last_os_error());
                        }
                    }
                    Ok(())
                });
            }
        }
        Ok(command)
    }
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
