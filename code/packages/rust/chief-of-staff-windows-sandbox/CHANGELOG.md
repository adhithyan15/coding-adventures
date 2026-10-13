# Changelog

## Unreleased

### Added

- The D18S Windows applier (build step 8, P2.8a).
  - `WindowsConfinement::prepare_verified(plan, &agent, agent_id)` checks
    the plan (for Windows, launchable, no `Direct` grant yet) and
    re-verifies the agent. The path must be exactly the held handle's
    final path. It then finds or makes the agent's AppContainer profile,
    and adds one ACE so that SID may read and execute the image. That
    read-modify-write is serialized within the process.
  - `spawn(SpawnRequest)` creates the process suspended, through one
    attribute list: security capabilities (no capabilities), LPAC, an
    explicit handle list, the job list, a restricted child-process policy
    and `MITIGATIONS`. The job is kill-on-close, one process, die on an
    unhandled exception, with every UI limit. It then checks the token,
    the job, every mitigation policy and the image name before
    `ResumeThread`. On any failure it ends the process through the job.
  - `ConfinedChild`: `id`, `wait`, `try_wait`, `kill`, and the `stdin` and
    `stdout` pipes. Dropping it kills the agent.
  - `remove_profile(agent_id)`.
  - Portable helpers, unit-tested everywhere: `container_name`,
    `command_line`, `environment_block`, `check_environment`, `check_plan`,
    `strip_verbatim_prefix`, `MITIGATIONS`.
  - `windows-sandbox-probe` and `tests/confinement.rs`, which run on
    `windows-2025`.
