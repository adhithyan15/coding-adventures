# Changelog

## Unreleased

### Added

- The D18S macOS applier (build step 7, #13980 P2.7a).
  - `MacosConfinement::prepare_verified(plan, &agent, &shim)`:
    - checks the plan (`launch_preconditions`, a macOS plan, and only
      exact `Direct` fs read and write grants);
    - re-verifies both binaries;
    - checks each path is canonical, names the verified inode, and sits
      under directories no one else can write;
    - checks each grant;
    - writes a deny-default Seatbelt profile.
  - `apply(&mut command)` and
    `apply_inheriting(&mut command, descriptors)`:
    - check the environment against the closed set;
    - capture argv and envp, passing only what was set on the command;
    - park inherited descriptors as close-on-exec duplicates;
    - isolate the descriptors, then exec the shim from the hook, with its
      instructions in `CHIEF_OF_STAFF_SEATBELT_SHIM`;
    - poison the command on any refusal.
  - `chief-of-staff-seatbelt-shim`, in a fresh image:
    - places the inherited descriptors;
    - checks its environment;
    - zeroes `RLIMIT_CORE`;
    - checks the agent's and each grant's inode;
    - calls `sandbox_init`;
    - runs the S-P4 probes;
    - refuses any other open descriptor;
    - re-checks the inodes and execs the agent.

    Any failure exits 127.
  - The profile reads only a fixed list of sysctls. A grant, like the
    agent and the shim, must be safe from anyone else: owned by root or
    this user, writable by no one else, under directories no one else can
    write by mode or ACL, and on a local APFS or HFS volume.
  - `launch_verification()`, for the audit record.
  - `macos-sandbox-probe` and `tests/confinement.rs` (macOS only), plus
    unit tests for the profile, literals, grants, paths, the environment
    and the instruction encoding.
