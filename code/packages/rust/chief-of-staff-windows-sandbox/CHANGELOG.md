# Changelog

## Unreleased

### Added

- **P2.8b-1.** `WindowsConfinement::prepare_verified_broker(plan, broker,
  agent_id)` and `broker_container_name`: a broker runs in a
  `chief-of-staff-broker.<32 hex>` container, a namespace no agent's
  container can reach, so a broker and its agent are always different
  principals with independent one-instance rules. `container_name` now
  takes any bytes. `spawn` holds `chief_of_staff_spawn_isolation::spawn_lock`
  from marking the listed handles inheritable until its copies are closed
  (on every path). The test control spawns go through the same lock. New
  native test: an agent and its broker run side by side.
- The D18S Windows applier (build step 8, P2.8a).
  - `WindowsConfinement::prepare_verified(plan, &agent, agent_id)` checks
    the plan (for Windows, launchable, no `Direct` grant yet) and
    re-verifies the agent. The path must be exactly the held handle's
    final path. It then derives the agent's AppContainer SID. It adds
    one ACE so that SID may read and execute the image. That
    read-modify-write is serialized within the process.
  - `spawn(SpawnRequest)` creates the process suspended and detached (no
    console, whose host would be a child of the agent), through one
    attribute list: security capabilities (no capabilities), LPAC, an
    explicit handle list, the job list, a restricted child-process policy
    and `MITIGATIONS`. The job is kill-on-close, one process, die on an
    unhandled exception, with every UI limit. It then checks the token,
    the job, every mitigation policy, the image name and the held file's
    final path before `ResumeThread`. On any failure it ends the process
    through the job. The listed handles are made inheritable only
    immediately before the creation.
  - The agent's AppContainer profile exists only around the creation.
    `CreateProcessW` refuses a container with no registered profile
    (`ERROR_FILE_NOT_FOUND`, the first time these tests ran on
    windows-2025), but a profile is a storage folder and registry key the
    agent could keep state in (S-I1). So `spawn` registers it, creates the
    process suspended, and removes it before any check or instruction,
    serialized within the process. The agent's environment gains
    `LOCALAPPDATA`, set to the container's `AC` folder: without it
    `CreateProcessW` fails with `ERROR_ENVVAR_NOT_FOUND` (the second
    windows-2025 run). The profile's storage folder (as
    `GetAppContainerFolderPath` reported it) must be gone afterwards. A
    failed removal, or a folder left behind, ends the agent unrun.
  - `is_less_privileged_app_container(token)`: the launch check's LPAC
    test, public so the probe reports it from inside the same way. The
    third windows-2025 run failed the launch check with an unlabelled
    `ERROR_INVALID_PARAMETER`, most likely `TokenIsLessPrivilegedAppContainer`
    being unqueryable there; the token's
    `WIN://NOALLAPPPKG` security attribute answers instead, read with
    every pointer bounds- and alignment-checked. Each token query in the
    launch check now names itself in its error.
  - The probe starts Winsock itself in `connect` (the fourth windows-2025
    run's confined `connect` printed nothing: std panics when
    `WSAStartup` fails, and inside the container that failure is likely
    the denial), and prints any panic on stdout; the network test reports the
    exit code with an unexpected output.
  - One instance of an agent at a time: `spawn` refuses an agent whose
    previous `ConfinedChild` is still alive (`Container`), and dropping a
    child ends it and releases the claim once it has exited. Instances
    share a SID, so either could have written into the profile the other's
    spawn registered. The caller's handles are consumed
    even when the spawn fails.
  - `ConfinedChild`: `id`, `wait`, `try_wait`, `kill`, and the `stdin` and
    `stdout` pipes. Dropping it kills the agent.
  - Portable helpers, unit-tested everywhere: `container_name`,
    `command_line`, `environment_block`, `check_environment`, `check_plan`,
    `strip_verbatim_prefix`, `MITIGATIONS`.
  - `windows-sandbox-probe` and `tests/confinement.rs`, which run on
    `windows-2025`.
