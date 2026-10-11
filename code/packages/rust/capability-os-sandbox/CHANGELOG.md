# Changelog

## Unreleased

### Added

- **The base-deny plan model** (D18S S-I1a, S-P2, S-P3; #13980 P2.1).
  - `SandboxPlan.base` is a `BasePolicy` built from the OS alone, never from
    the manifest. It holds the deny-all primitives in install order, plus
    `principal_model` and `broker_topology`. A zero-capability agent
    therefore gets the full base, not an empty plan, and an empty plan would
    have meant "install nothing".
  - `SandboxCoverage::Unsupported` exists.
  - `SandboxPlan::launch_preconditions()` reports every reason a plan may not
    launch, all at once. It re-derives the plan rather than trusting its
    public fields:
    - the base must equal `BasePolicy::for_os(plan.os)`;
    - every rule must be for the plan's OS, and equal to its capability's
      lowering;
    - the coverage verdict comes from the re-derived rule.

    The refusals are `MissingBasePolicy`, `BasePolicyMismatch`,
    `SharedBroker`, `AdvisoryRule`, `UnsupportedRule`, `RuleForAnotherOs`,
    `RuleMismatch` and `NoKernelBoundary` (the portable target).
    `run_with_kernel_sandbox` refuses before installing anything. `Ok` means
    launchable as written, not enforced (S-P4). The set of rules is trusted,
    so the enforcer must lower from the signed manifest itself.
  - `SandboxPlanSummary` gains `unsupported_rules`,
    `base_primitives_required`, `base_installed` (always `false` until the
    appliers land), `principal_model` and `broker_topology`.

### Changed

- No lowering produces `Advisory`, and none rounds up to `Direct` (S-P2).
  `Direct` now means a kernel primitive names the exact declared target.
  - `ffi:*` is `Unsupported` on every platform, Portable included. Native
    code runs in the agent's address space, so nothing can scope or broker
    it. It was `Direct` on Linux, macOS, FreeBSD and OpenBSD.
  - `proc:*` is brokered on every platform, which includes `proc:exec`: the
    supervisor spawns and the agent never execs. It was `Direct` through
    seccomp, Seatbelt, job objects and jails.
  - `net:*` is brokered on every platform. It was `Direct` through
    `cgroup_bpf.sock_addr` (privileged), Seatbelt (ports, not hosts),
    AppContainer `internetClient` (all or nothing) and jails (root).
  - A filesystem target is `Direct` only when it names one normalised
    absolute path. These are all brokered:
    - glob syntax (`*`, `**`, `/tmp/*`, `?`, `[..]`);
    - the root `/` and any target ending in a separator, since a directory
      grants its whole subtree;
    - relative paths, `~`, `.` and `..` components, and empty components;
    - control characters;
    - a path in another OS family's syntax: a drive path on Unix, a `/`
      path on Windows, and a backslash inside a Unix path;
    - on Windows, a component ending in a dot or a space (Windows strips
      them) or containing `:` (an alternate data stream).
  - Time grants are brokered on every platform.
  - Primitive names follow `<os>.host_broker.<category>`. The Linux resolver
    is `linux.host_broker.resolver` (it was `linux.seccomp.brokered_resolver`).
  - `SandboxRule::is_native` and the summary's `native_rules` no longer count
    `Unsupported` rules as native coverage.

### Not yet

- The base is modelled, not installed. The macOS Seatbelt applier still
  writes an `(allow default)` profile, and still allows the declared ports
  even though network grants are now labelled brokered. D18S build steps 3,
  4, 7 and 8 make each applier install its base.

## [0.1.0] - 2026-05-14

### Added

- Added manifest-to-sandbox lowering for Linux, macOS, Windows, FreeBSD,
  OpenBSD, and portable host-broker plans.
- Added coverage labels for direct OS enforcement, brokered mediation,
  launch-time setup, and advisory narrowing.
