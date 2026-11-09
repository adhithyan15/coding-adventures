# Changelog

## Unreleased

### Added

- Initial implementation of the VER08b runtime capability conformance gate.
  - `policy`: lowers a manifest through `capability-os-sandbox` and maps every
    category/action to syscall allowances and Landlock rights; the explicit,
    justified `BASELINE` (`rcg-baseline-1`: `write` fd 1/2, `exit_group`,
    wake-only `futex`, `mmap` without `PROT_EXEC`, `munmap`, `brk`); refuses
    `ffi`, `ui` and (by default) `thread` with `RCG006`.
  - `bpf`: renders the policy as a seccomp-BPF program (arch check, KILL by
    default, argument predicates) with a software evaluator for tests.
  - `arm`: no_new_privs + Landlock (all fs rights handled, rules only for
    declared paths) + seccomp, installed in-process.
  - `observe`: strace `-f -v` parser/classifier with the ARMED-marker boundary,
    execve environment extraction and the verdict table inputs.
  - `harness`: parent/child driver, enforcement/observation agreement table,
    18 syscall negative controls run three ways plus an env-leak control and a
    recorded known gap (vDSO time), the Docker ring (rootfs from `ldd`, KILL
    profile generated from the policy, container-level negative control), and
    `capability_runtime_evidence.json`.
  - `failure`: the `RCG001`..`RCG014` registry.
- Tests: manifest->policy->filter mapping for every category, BPF behaviour,
  classifier fixtures (hand-written and recorded from real strace), judgement
  table, container rendering, and three live self-test gates (pure, declared
  time + Landlock-scoped fs read, misbehaving).
- Registered in the Rust workspace. Non-Linux targets compile and report
  NOT-LIVE (`RCG010`).
- Coverage work: in-process unit tests for `arm` (rights arithmetic, Landlock
  probe, every installer failure path that returns before restricting the
  process), `sabotage` (every harmless control plus the unknown-name path) and
  `harness` (`not_live`, evidence write failure, manifest refusal,
  `unexpected_fds`); new `selftest_controls` target runs the full gate with all
  rings (18 negative controls, env-leak, time-vdso known gap, Docker ring
  NOT-LIVE when no daemon). Fixed a ruleset-fd leak when a Landlock path
  contained a NUL byte. Measured line coverage (llvm-cov, this crate sources,
  including in-source test modules): 82.0%; production lines only: 80.2%. The
  remainder is `child_main`, the successful `arm` path and the Docker success
  path, which run only inside a sandboxed child that ends in `_exit` and so
  cannot flush profile data; they are asserted end to end by the `selftest_*`
  targets and ct-compare gate instead.
