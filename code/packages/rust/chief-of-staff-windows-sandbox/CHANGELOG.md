# Changelog

## Unreleased

### Added

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
    serialized within the process. A failed removal ends the agent unrun. The caller's handles are consumed
    even when the spawn fails.
  - `ConfinedChild`: `id`, `wait`, `try_wait`, `kill`, and the `stdin` and
    `stdout` pipes. Dropping it kills the agent.
  - Portable helpers, unit-tested everywhere: `container_name`,
    `command_line`, `environment_block`, `check_environment`, `check_plan`,
    `strip_verbatim_prefix`, `MITIGATIONS`.
  - `windows-sandbox-probe` and `tests/confinement.rs`, which run on
    `windows-2025`.
