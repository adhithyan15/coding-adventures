# Changelog

## Unreleased

### Fixed

- No weak randomness. `random_hex()` no longer falls back to a clock/address
  mix when `getrandom` fails or reads short; it returns an error. The private
  directory creation reports NOT-LIVE `RCG013` ("no OS randomness"), and a
  nonce that cannot be drawn mid-run stops the gate with a panic rather than
  proceeding with a predictable value.
- Private working directories are now created exclusively and removed on every
  exit path. (1) The `rcg-scratch` / `rcg-container` directory name is
  `<name>-<pid>-<64 random bits>` (the same source as the run nonces) and is
  made with a non-recursive `mkdir` at mode 0700, never `create_dir_all`: with
  `evidence_dir == None` it sits under shared `/tmp`, where a pre-planted
  symlink at the predictable `<name>-<pid>` was followed and strace output
  could be redirected. An existing path of any kind (symlink, file,
  directory) is now refused and the gate reports NOT-LIVE `RCG013`; it is
  never reused. (2) An RAII `ScratchDir` guard deletes the directory on drop,
  so the early `NOT-LIVE` returns (no strace, strace cannot trace, no
  `current_exe`, child will not spawn), a panic and the container ring all
  clean up; only SIGKILL can still leave one. No control, policy, `RCG009` or
  `RCG013` behaviour changed. Tests: in-module unit tests in `harness.rs`
  and two new cases in `tests/selftest_fail_closed.rs`.
- Fail closed on a missing trace. An observed run whose strace trace was
  absent, unreadable or empty used to be analysed as an empty string, i.e.
  "observed nothing, saw no violations". The main run, every control run
  (full and observe-only), the env-leak run and the known-gap run now add an
  `RCG013` failure ("observer produced no trace (<which run>: <why>)"), in
  strict and non-strict mode alike. Regression test
  `tests/selftest_fail_closed.rs` (a `strace` shim that leaves no trace,
  either always or only for controls). `RCG013` keeps its id; its meaning now
  covers an unusable trace, documented in the README and VER08b.
- Scratch (`rcg-scratch`) and container (`rcg-container`) working directories
  are now private to each gate process (`<name>-<pid>`). They live under the
  adopter's `evidence_dir`, which for Rust adopters is the workspace-wide
  `CARGO_TARGET_TMPDIR`, and are removed with `remove_dir_all`. Two gates
  running in parallel (the build tool does this, e.g. `ct-compare` beside
  `gf256`) shared one directory: the first to finish deleted the directory the
  second was writing `strace -o` traces into, so every later observed run had
  no trace and `RCG009` reported the remaining controls (io-uring, bpf,
  env-leak on CI) as "NOT detected" while enforce-only runs still passed.
  Regression test: `tests/selftest_concurrent.rs` (two staggered gates, one
  shared `evidence_dir`; failed 3/3 before the fix). The jail itself is
  unchanged: no control, check or `RCG009` condition was touched.
- Launcher now closes every inherited descriptor above stderr between `fork`
  and `exec` (`close_range`, with a `close` loop fallback). `Command` only
  closes `O_CLOEXEC` descriptors, and the GitHub Actions runner leaks plain
  pipes into build steps, so the child's `RCG011` audit (correctly) refused to
  arm: "no checks ran" plus `RCG009` for all 18 controls on CI. The audit is
  unchanged and still fail-closed; the launcher now hands over exactly 0/1/2.

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
