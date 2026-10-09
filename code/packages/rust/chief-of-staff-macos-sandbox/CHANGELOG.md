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
  - `command()`, `apply(&mut command)` and
    `apply_inheriting(&mut command, descriptors)`:
    - check the environment against the closed set;
    - pass only what was set on the command;
    - isolate the descriptors;
    - hand the shim its instructions in `CHIEF_OF_STAFF_SEATBELT_SHIM`.
  - `chief-of-staff-seatbelt-shim`, in a fresh image:
    - places the inherited descriptors;
    - refuses any other open descriptor;
    - zeroes `RLIMIT_CORE`;
    - calls `sandbox_init`;
    - runs the S-P4 probes;
    - re-checks the agent's inode and execs it.

    Any failure exits 127.
  - `launch_verification()`, for the audit record.
  - `macos-sandbox-probe` and `tests/confinement.rs` (macOS only), plus
    unit tests for the profile, literals, grants, paths, the environment
    and the instruction encoding.
