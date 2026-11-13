# VER08b — Runtime Capability Conformance Gate

**Parent:** [VER00 §8](VER00-verified-shipping-program.md) (evidence rungs), [VER08](VER08-capability-analyzers.md), [VER08a](VER08a-rust-capability-analyzer.md) (the static analyzer this gate complements), [13-capability-security](13-capability-security.md), [D18S](D18S-chief-of-staff-agent-sandbox.md) (sandbox lowering), [D21](D21-capability-cage.md) (taxonomy).

**Pilot:** `code/packages/rust/ct-compare` (`"capabilities": []`, pure computation).
**Implementation:** `code/packages/rust/capability-runtime-conformance`.

---

## 1. The claim, and why a static check is not enough

> *The library's full public-API test program, run under an OS sandbox that is derived mechanically from `required_capabilities.json` and denies everything else, performs no operation outside the declared set — and an independent observer agrees.*

VER08a is a lexical analyzer. It can be evaded by macros, FFI, `build.rs`, and dependencies. This gate adds **runtime** evidence: it executes the code and asks the kernel what actually happened. Two mechanisms must agree:

```text
                        required_capabilities.json
                                   │  (capability-os-sandbox::plan_from_json)
                                   ▼
                              SandboxPlan ──► RuntimePolicy
                                   │              │  baseline ∪ declared allowances
          ┌────────────────────────┴──────┐       │
          ▼                               ▼       ▼
   ENFORCEMENT (ring 1: in-process)   OBSERVATION (independent)     CONTAINER (ring 0)
   no_new_privs + seccomp-BPF         parent runs the child under   docker run --network none
   (deny-by-default, KILL) +          `strace -f`, classifies every --read-only --cap-drop ALL
   Landlock deny-all-fs               syscall after the ARMED mark  generated seccomp profile
          │                               │                              │
          └──────────── must agree ───────┴───────── evidence JSON ──────┘
```

A gate that has never been seen to fail is not evidence. §8 therefore makes **negative controls mandatory**: the same machinery must be shown to catch each forbidden category.

## 2. Two-phase model

A process cannot start with the strict filter: the dynamic loader, the Rust runtime and the harness itself make dozens of syscalls (openat of `libc.so`, mprotect, arch_prctl, set_tid_address, rseq, prlimit64, sigaltstack, getrandom for hash seeds, …). Allowing all of those for the whole run would make the baseline huge and meaningless. Instead:

```text
 PHASE A  setup (unconstrained, untrusted-for-nothing: only test-harness code runs)
          load vectors, allocate, set panic hook, warm stdout, audit env + fds
 ───────── prctl(NO_NEW_PRIVS); landlock_restrict_self; seccomp(SET_MODE_FILTER) ─────────
          write(2, "RCG-ARMED <nonce>\n")      ◄── the boundary marker, first armed syscall
 PHASE B  armed: ONLY library code + TAP output. Any syscall outside baseline ∪ declared
          kills the whole process with SIGSYS.
          exit_group
```

Only phase B is evidence about the library. Phase A is harness code that never calls the library. The parent locates the marker by the per-run random nonce it passed in argv (the library never sees it).

Invariant: the filter is installed **immediately before** the first library call and no library code runs in phase A. The arming call is the last line of setup in the harness API (`run_package_gate(.., setup, exercise)` — `exercise` is only reachable after arming).

## 3. Capability → allowance mapping

Source of truth: the `SandboxPlan` that `capability-os-sandbox` produces for `OsFamily::Linux`. The gate refuses (failure `RCG006`) any plan whose `launch_preconditions()` fails, which today means any `ffi`, `ui` capability (unenforceable) or any advisory rule. `thread` is also `Unsupported` in the plan; the gate maps it only coarsely (§3.2) and labels it so.

### 3.1 Table

| Capability | Syscalls added to the armed allowlist | Other layers |
|---|---|---|
| *(none — baseline)* | §4 | Landlock handles **all** fs rights, no rules (no fs at all); no sockets; empty env; fds 0–2 only |
| `fs:read <path>` | `openat`, `open`, `read`, `pread64`, `readv`, `preadv`, `lseek`, `fstat`, `newfstatat`, `stat`, `lstat`, `statx`, `access`, `faccessat`, `faccessat2`, `readlink`, `readlinkat`, `fcntl`, `close` | Landlock `READ_FILE` beneath each exact target; target must be an absolute exact file (the plan already enforces) |
| `fs:write <path>` | read set's `openat/open/close/fstat/lseek` + `write` on any fd, `pwrite64`, `writev`, `pwritev`, `ftruncate`, `fsync`, `fdatasync` | Landlock `WRITE_FILE`, `TRUNCATE` beneath target |
| `fs:create` | `openat`, `open`, `creat`, `mkdir`, `mkdirat` | Landlock `MAKE_REG`, `MAKE_DIR` |
| `fs:delete` | `unlink`, `unlinkat`, `rmdir` | Landlock `REMOVE_FILE`, `REMOVE_DIR` |
| `fs:list` | `getdents`, `getdents64` | Landlock `READ_DIR` |
| `net:connect` | `socket`, `connect`, `getsockopt`, `setsockopt`, `getsockname`, `getpeername`, `sendto`, `recvfrom`, `sendmsg`, `recvmsg`, `poll`, `ppoll`, `epoll_*`, `shutdown` | container: network enabled (not `--network none`); host:port **not** scoped by seccomp, observed instead (§6) — label `partial` |
| `net:listen` | `socket`, `bind`, `listen`, `accept`, `accept4`, `getsockname`, `setsockopt`, `poll`, `epoll_*` | as above |
| `net:dns` | connect set + fs read of `/etc/resolv.conf`, `/etc/hosts`, `/etc/nsswitch.conf` | Landlock read rules for those files |
| `proc:exec` | `execve`, `execveat`, `wait4`, `waitid`, `pipe`, `pipe2`, `dup`, `dup2`, `dup3`, `clone`, `vfork` (child must be created to exec) | Landlock execute on the declared command (plan: brokered; the in-process gate treats as coarse) |
| `proc:fork` | `fork`, `vfork`, `clone`, `clone3`, `wait4`, `waitid` | — |
| `proc:signal` | `kill`, `tgkill`, `tkill`, `rt_sigqueueinfo` | — |
| `env:read`, `env:write` | none (environment access is memory, not syscalls) | launch-time `envp` allowlist; in-child audit (§5) |
| `time:read` | `clock_gettime`, `clock_getres`, `gettimeofday`, `time` | **limit:** the vDSO answers most of these with no syscall; neither seccomp nor strace can see that (§9) |
| `time:sleep` | `nanosleep`, `clock_nanosleep` (+ time:read set) | — |
| `stdin:read` | `read` on fd 0 only | fd table |
| `stdout:write` | `write` on fd 1 only (already inside baseline) | fd table |
| `rand:read` | `getrandom` | brokered in D18S; in-process only |
| `thread:spawn/message` | `clone`, `clone3`, `futex`, `set_robust_list`, `rseq`, `sched_yield`, `sigaltstack`, `rt_sigprocmask`, `mprotect` (no `PROT_EXEC`), `sched_getaffinity` | **coarse**: role/target not scoped. Plan marks it `Unsupported`, so the gate refuses (RCG006) unless the caller passes `allow_coarse_thread` |
| `ffi:*`, `ui:*` | *refused* | cannot be scoped; gate emits `RCG006` and no certificate |

The mapping is a pure function `RuntimePolicy::from_plan(&SandboxPlan)`; every row above has a unit test (§10).

### 3.2 Things deliberately never allowed by any capability

`ptrace`, `bpf`, `io_uring_*`, `mount`, `umount2`, `pivot_root`, `chroot`, `setns`, `unshare`, `perf_event_open`, `keyctl`, `add_key`, `request_key`, `init_module`, `finit_module`, `kexec_*`, `reboot`, `seccomp` (after arming; no filter loosening), `prctl` (after arming), `landlock_*` (after arming), `mprotect` with `PROT_EXEC`, `mmap` with `PROT_EXEC`. These match D18S's `deny_io_uring` and no_new_privs intent.

## 4. The runtime baseline (armed phase)

The baseline is explicit, minimal, and each entry carries its justification in code (`BASELINE` in `policy.rs`) and in the evidence file. **The list below was derived by measurement, then pruned**: a first draft (written before any measurement) also listed `exit`, `rt_sigreturn`, `mremap` and `madvise`; running the Rust pilot under `strace` showed the armed phase never makes them (the harness leaves through `_exit`, which is `exit_group`; there are no signals and no `realloc`), so they were removed. Entries that remain are there because the pilot demonstrably needs them, or (marked *structural*) because the allocator legitimately varies by libc build.

| Syscall | Constraint | Justification | Observed on pilot |
|---|---|---|---|
| `write` | arg0 ∈ {1, 2} only (any other fd kills) | TAP result lines on fd 1; the ARMED marker and panic text on fd 2. Nothing else may be written. | yes (10) |
| `exit_group` | — | Process termination (`_exit`). Without it a process cannot end. | yes (1) |
| `futex` | `op` ∈ {`FUTEX_WAKE`, `FUTEX_WAKE_PRIVATE`} only | glibc's dynamic-loader lock release during the first panic unwind (`ct_select_bytes` panics by contract on unequal lengths). Wake-only: in a single-threaded process a wake can never block or wait on anything; `FUTEX_WAIT` stays denied. Found by measurement: the first run of the pilot was killed here. | yes (1) |
| `mmap` | `PROT_EXEC` forbidden | Allocations above the mmap threshold (the pilot's 1 MiB select vector). W^X: an executable mapping is code loading, an FFI-class capability. | yes (1) |
| `munmap` | — | Returning mmap-backed allocations. | yes (3) |
| `brk` | — | *Structural.* glibc `malloc` main-arena growth/trim; not needed on the pilot run on glibc 2.39 (arena pre-warmed in phase A) but allocator-internal and libc-dependent, and the library allocates a `Vec` on every `ct_select_bytes`. Evidence lists baseline entries not observed (`unobserved_baseline`). | no |

Candidates measured as **not** needed, so absent: `read`, `openat`, `getrandom`, `clock_gettime`, `rt_sigprocmask`, `rt_sigaction`, `sigaltstack` (the harness skips `std`'s runtime cleanup via `_exit`), `mprotect`, `getpid`, `exit`, `rt_sigreturn`, `mremap`, `madvise`.

The measured armed-phase syscall set for the pilot is `{write, exit_group, futex(wake), mmap, munmap}`; a test asserts the recorded set stays inside the baseline, so adding an undeclared syscall to the library fails a test and not only the gate.

The baseline has a version string (`rcg-baseline-1`) recorded in evidence; changing the list requires bumping it.

## 5. Enforcement ring (in-process)

`arm(&RuntimePolicy)` performs, in order, with every step checked and a distinct exit code on failure (`RCG008`):

1. `prctl(PR_SET_NO_NEW_PRIVS, 1)` — unprivileged-safe; the filter cannot be undone by exec of setuid binaries.
2. **Landlock**: probe ABI; create a ruleset handling every filesystem right the ABI knows; add `path_beneath` rules only for declared `fs` targets; `landlock_restrict_self`. If Landlock is unavailable and the policy has `fs` capabilities, fail closed (`RCG012`). If it is unavailable and the policy has no fs capability, seccomp already denies every fs syscall, so Landlock is defense in depth: the evidence records `landlock_enforced:false` and the verdict stands.
3. **seccomp-BPF**: `seccomp(SECCOMP_SET_MODE_FILTER, SECCOMP_FILTER_FLAG_TSYNC)` with the program: arch check (`AUDIT_ARCH_X86_64` or `AUDIT_ARCH_AARCH64`, else kill); allowlisted numbers → `ALLOW` (with the argument predicates of §4); default → `SECCOMP_RET_KILL_PROCESS`. `KILL_PROCESS` rather than `ERRNO`: a library that gets `EACCES` can swallow it and carry on, hiding the attempt; a kill is loud and unambiguous (SIGSYS).

The harness leaves the process with `_exit` (straight to `exit_group`), not `std::process::exit`: `std`'s runtime cleanup would `sigaltstack`/`munmap` its handler stack and libc's atexit machinery would add calls that have nothing to do with the library. Measurement showed this is what keeps the baseline to the six entries of section 4.

Pre-arm audits (harness code, reported as `RCG005` / `RCG011`):
* **environment**: (the LLVM coverage runtime sets `__LLVM_PROFILE_RT_INIT_ONCE` for itself after `exec` in instrumented builds; that one name is exempt from the in-child audit, and the parent's `execve` check still sees what the launcher really passed) every variable in `environ` must be named by an `env:read`/`env:write` capability target (`*` allows all). Env reads are not syscalls, so the *only* defences are the empty launch environment and this audit.
* **file descriptors**: only 0, 1, 2 (plus the audit's own directory handle) may be open — no inherited descriptor can be used as ambient authority.

## 6. Observation ring (independent)

The parent re-executes the same binary as a child under `strace -f -v -qq -s 4096 -o <trace> <child>` (so threads and forks are followed). It then:

1. finds the **ARMED** marker: the `write(2, "RCG-ARMED <nonce>\n", …)` line;
2. extracts the syscall name of every subsequent line (ignoring `--- SIG… ---`, `<... x resumed>`, and exit/kill epilogues);
3. classifies each name via the same syscall→capability table used to build the filter, and compares with baseline ∪ declared;
4. applies the argument predicates it can see in text (`write` fd; `mmap` prot);
5. checks the first `execve` line's environment against the manifest;
6. fails with `RCG002` and lists every offending call, even if seccomp did not block it.

The parent also records the child's exit status. **Agreement table:**

| Child outcome | Observer finds violation | Verdict | Failure id |
|---|---|---|---|
| exit 0 | no | PASS | — |
| exit 0 | yes | FAIL — filter had a hole | `RCG002` + `RCG003` |
| SIGSYS | yes (that call) | FAIL — attempted violation (enforced) | `RCG001` |
| SIGSYS | no | FAIL — observer blind or filter mismatch | `RCG003` |
| exit ≠ 0, not SIGSYS | — | FAIL — library check failed | `RCG007` |
| marker missing | — | FAIL — never armed | `RCG004` |

The observer is deliberately **not** the seccomp log: ptrace sees the attempt before seccomp acts on it, so a kill still leaves the offending name in the trace.

## 7. Container ring (CI-portable)

`docker_invocation(policy, image, cmd)` renders, from the same manifest-derived policy:

```text
docker run --rm --network none           (omitted only when a net capability is declared)
  --read-only --cap-drop ALL --security-opt no-new-privileges
  --security-opt seccomp=<generated profile.json>
  --pids-limit 16 --memory 256m --user 65534:65534
  (no -v, no -e, no --env-file, environment is empty)  <image> /rcg/test --rcg-child …
```

The generated profile has `defaultAction: SCMP_ACT_KILL_PROCESS` and an allowlist of **startup set ∪ baseline ∪ declared**. The startup set (`execve`, loader `openat`/`read`/`mmap`/`mprotect`/`arch_prctl`/…) is larger than the in-process baseline because the container filter applies from the first instruction, which is exactly why ring 1 exists and is stricter. Ring 0 therefore proves "the container has exactly these permissions" at the kernel/container boundary; ring 1 proves the library *within* that process makes no more.

The image is built offline (`docker import` of a rootfs tarball containing the test binary and its `ldd` closure only — no shell, no package manager). The container run executes the test program in **container mode**: the in-process arming still happens (it is the same binary) and the process prints its TAP; the parent checks exit status. `strace` is not run inside the container (that would need `CAP_SYS_PTRACE`, contradicting `--cap-drop ALL`); observation is ring 2's job on the host. Implementation findings that shaped the ring (all measured against Docker 29 / runc on Linux 6.18):

* The container filter must also allow what `runc init` does after the filter is installed. Bisecting a `KILL_PROCESS` profile showed that, beyond the usual start-up calls, `openat2` is required (runc's safe `/proc` access). Without it the container dies before `main` and Docker reports a misleading `bind-mount /proc/<pid>/ns/net` error. `STARTUP_SYSCALLS` records this.
* The test binary installs ring 1 from *inside* the container, so ring 0 must also allow the arming calls (`prctl`, `seccomp`, `landlock_*`): `ARMING_SYSCALLS`. Ring 1 forbids them again for everything after arming.
* Docker injects `PATH`, `HOSTNAME` and `HOME` into every container no matter what. In container mode the child removes exactly those three in phase A (before the environment audit), so the library still sees an empty environment; the evidence records this.
* The container-ring negative control runs the sabotage `io-uring` with the in-process filter **off**; the container's profile alone must kill it (exit status 159 = 128 + SIGSYS). `socket` is deliberately not used for this: it is in the start-up set because runc needs it.
* The profile has no argument predicates (`write` fd, `PROT_EXEC`): those are ring 1's job.

If Docker or its daemon is unreachable the layer reports `not-live` with the reason (and fails if `RCG_REQUIRE_DOCKER=1`).

## 8. Negative controls (mandatory)

The same test binary accepts `--sabotage <name>`: after arming it attempts one forbidden operation. Each control is run **three ways**, so each ring is proven to catch it on its own:

| Run | Filter | Strace | Expected |
|---|---|---|---|
| full | on | on | SIGSYS and observer both flag it (`RCG001`) |
| observe-only | **off** | on | process survives the attempt; observer flags it (`RCG002`) |
| enforce-only | on | **off** | SIGSYS (`RCG001`) |

Controls (name → operation → expected primary syscall): `fs-open` (`openat` of `/etc/hostname`), `fs-create` (`openat O_CREAT`), `fs-delete` (`unlink`), `fs-list` (`getdents64`), `net-socket` (`socket`), `proc-spawn` (`std::process::Command` → `clone`/`vfork`/`execve`), `proc-exec` (`execve`), `proc-signal` (`kill`), `time-syscall` (raw `clock_gettime`), `time-sleep` (`nanosleep`), `rand-syscall` (`getrandom`), `stdin-read` (`read` fd 0), `bad-fd-write` (`write` fd 7), `thread-spawn` (`clone3`), `exec-map` (`mmap PROT_EXEC`), `mprotect-exec`, `io-uring` (`io_uring_setup`), `bpf` (`bpf`). Two controls are not syscalls: `env-leak` runs the child with a canary variable in its environment (a buggy launcher) and expects `RCG005` from both the in-child audit and the parent's `execve` check.

The *first* violating call is not always the one the control name suggests (a thread spawn is first caught at `rt_sigaction` inside `pthread_create`, a `Command::spawn` at `rt_sigprocmask`), so each control lists the set of calls the observer may report first (`expected` in `sabotage.rs`). The container ring has one more control of its own (section 7).

A control that is **not** detected makes the gate itself fail with `RCG009` — the gate is defective and its PASS is meaningless. `known_gaps` (e.g. `Instant::now()` served by the vDSO) are recorded, not asserted as detected; if a gap starts being detected the evidence says so and the spec should be updated.

## 9. Evidence file

`capability_runtime_evidence.json` (written next to the test target's build output; schema `capability_runtime_evidence.v1`):

```json
{
  "schema": "capability_runtime_evidence.v1",
  "package": "rust/ct-compare",
  "gate_version": "rcg-1",
  "manifest_sha256": "<hex>",
  "plan_sha256": "<hex>",
  "baseline_version": "rcg-baseline-1",
  "declared_capabilities": [],
  "live": true,
  "not_live_reason": null,
  "enforcement": { "verdict": "pass", "landlock_enforced": true, "seccomp_enforced": true,
                   "child_status": "exit 0", "failure_ids": [] },
  "observation": { "verdict": "pass", "observer": "strace -f -v", "armed_marker_seen": true,
                   "observed_syscalls": { "write": 10, "mmap": 1, "munmap": 3, "futex": 1, "exit_group": 1 },
                   "unobserved_baseline": ["brk"],
                   "violations": [], "failure_ids": [] },
  "agreement": true,
  "negative_controls": [
    { "name": "fs-open", "expected_syscall": "openat",
      "full": {"detected": true, "failure_id": "RCG001", "observed": "openat"},
      "observe_only": {"detected": true, "failure_id": "RCG002"},
      "enforce_only": {"detected": true, "failure_id": "RCG001"} }
  ],
  "known_gaps": [ { "name": "time-vdso", "detected": false } ],
  "container": { "live": true, "invocation": ["docker", "run", "…"], "verdict": "pass",
                 "negative_control_io_uring_killed_by_container_filter": true },
  "environment": { "kernel": "Linux 6.18…", "arch": "x86_64", "runner": "…", "strace": "…" },
  "overall": "pass"
}
```

`overall` is `pass` only if the gate is live, enforcement and observation both pass and agree, every negative control was detected on every ring, and (when live) the container ring passed. `not-live` is its own value, never `pass`.

**Integration (follow-up, not done here):** `ca-assurance-report` should consume this file as an evidence rung under VER00 §8 (manifest hash must equal the report's manifest hash; `overall` must be `pass`; `live:false` renders as "not-live", never "proved"). That crate is being rewritten elsewhere; this spec only fixes the schema.

## 10. Failure-id registry

| Id | Meaning |
|---|---|
| `RCG001` | Enforcement: process killed by SIGSYS after arming (attempted forbidden operation) |
| `RCG002` | Observation: syscall outside baseline ∪ declared seen after the ARMED marker |
| `RCG003` | Enforcement/observation disagree |
| `RCG004` | ARMED marker never seen (filter never installed or marker forged/missing) |
| `RCG005` | Environment exposure: variable present that no capability names |
| `RCG006` | Declared capability cannot be enforced at runtime (`ffi`, `ui`, coarse `thread`) |
| `RCG007` | Library/test check failed (non-zero exit not caused by SIGSYS) |
| `RCG008` | Sandbox installation failed (prctl/seccomp/landlock error) |
| `RCG009` | Negative control not detected — gate defect |
| `RCG010` | Platform not live (non-Linux) — reported, never silent |
| `RCG011` | Unexpected inherited file descriptor |
| `RCG012` | Landlock required for fs scoping but unavailable |
| `RCG013` | Observer unavailable, or an observed run left no usable trace (absent, unreadable, empty or unparsable) |
| `RCG014` | Container ring failed |

## 11. Tests (implementation contract)

* **mapping** (`tests/units.rs`): for every row of section 3.1, manifest JSON -> plan -> `RuntimePolicy` contains exactly the listed syscalls and nothing from another category; refused categories yield `RCG006`; no capability ever allows a `NEVER_ALLOWED` call. (The capability cage rejects manifests that combine untrusted inputs with external actuations, so the "every capability" tests use several valid manifests.)
* **filter**: BPF program structure (arch check first, KILL default, argument predicates), run through a software BPF evaluator for allowed/denied numbers, `write` fd scoping, `PROT_EXEC`, wake-only futex, x32 numbers, per-capability widening.
* **classifier**: hand-written strace fixtures (clean, violation, killed, multi-thread interleaving with `<unfinished ...>`/`<... resumed>`, missing marker, wrong nonce, unknown syscall names, leaked environment) **and traces recorded from the real gate binary** in `tests/fixtures/`.
* **verdict table**: every row of section 6.
* **arm** (`tests/selftest_arm.rs`, forked children): allowed baseline calls work, forbidden ones die with SIGSYS, arming cannot be repeated, Landlock refuses an undeclared file while seccomp allows `openat`.
* **live gates** (`selftest_pure`, `selftest_declared`, `selftest_misbehaving`): a clean pure package passes; a package declaring `time:read` and one readable file passes and Landlock is observed to scope it; a package that misbehaves after arming fails the whole gate with `RCG001` and the observer names `openat`.
* **negative controls**: all 18 syscall controls and `env-leak` executed for real on the pilot, three rings each (Linux).
* **container**: executed for real when Docker works (rootfs from `ldd`, generated profile, in-container run and container negative control); otherwise recorded as not-live.
* **baseline measurement**: the pilot's recorded post-arm syscall set is asserted to equal `{exit_group, futex, mmap, munmap, write}` and to be inside the baseline.
* **spec sync**: tests read this file and fail if a baseline entry or a failure id is missing from it.

## 12. Per-language strategy

The harness shape is the same everywhere: *parent* (strace + verdict) / *child* (setup, arm, exercise). The only language-specific part is **when it is safe to arm**.

| Lane | Arming point | Notes |
|---|---|---|
| Rust (pilot) | After `main` starts and vectors are loaded; arm via `libc` | Native allocator needs only §4. |
| Go | After `init`, GC start and `runtime.LockOSThread`-free warmup; arm with `SECCOMP_FILTER_FLAG_TSYNC` | Go runtime is multi-threaded and uses `futex`, `epoll`, `sigaltstack`, `rt_sigprocmask`, `clone` for Ms: a larger *Go runtime baseline* `rcg-baseline-go-1` is required and must be justified separately; TSYNC covers existing threads. |
| Python | After interpreter start and `import` of the package and test data (imports are phase A) | Needs `read`, `fstat`, `lseek`, `mmap` for lazy `.pyc` loads if any import is deferred — the test program must force-import everything first. Python baseline adds `getrandom`-free paths only after `PYTHONHASHSEED` is pinned. |
| TypeScript/Node | After the event loop is idle and modules are loaded; V8 spawns worker threads and uses `futex`, `epoll_*`, `eventfd2`, `mprotect` for JIT (violates the W^X rule) | Likely run with `--jitless`, `--single-threaded` and a Node-specific baseline; may be impractical — then the lane stays *static only* and says so. |
| Java | After VM startup and class preloading; arm via a JNI/Panama shim | Large baseline (GC threads, futex, mprotect for JIT); use `-Xint` and serial GC. |
| Others | Ruby/Lua/Perl like Python; C/C++/Swift/C# like Rust (C# needs a runtime baseline like Java) | |

Each lane gets its own baseline *version string* and justification table; baselines are never shared implicitly, so a loose interpreter baseline cannot weaken the Rust one.

## 13. Platform status

| Platform | Status |
|---|---|
| Linux (x86_64, aarch64) | **Live**: all three rings |
| macOS | **not-live**: the gate prints `NOT-LIVE (RCG010)` and writes evidence with `live:false`. A future ring may use Seatbelt (`sandbox-exec`) + `dtruss`/`fs_usage`, which need elevated privileges. |
| Windows | **not-live**: same reporting. Future: AppContainer + Job Object + ETW. |
| Linux without `strace`/ptrace | **not-live (observer)** unless `RCG_STRICT=1`, in which case `RCG013` fails. An observer that starts but leaves no trace for an observed run (main, any control, the env-leak run or the known-gap run) is a **failure** (`RCG013`) in both modes: the gate never reads "no trace" as "no violations". |

GitHub-hosted `ubuntu-24.04` restricts *unprivileged user namespaces* via AppArmor; this gate does not need them (seccomp + `no_new_privs` + Landlock are unprivileged; Docker is available on those runners).

## 14. What a runtime gate can and cannot prove (honest limits)

* **Only exercised paths.** It proves the *tested* execution made no undeclared syscalls — not that no code path can. Coverage of the test program bounds the claim. It complements the static analyzer (which reasons about all paths lexically); neither replaces the other.
* **Not syscall-visible effects:** environment reads (memory), `Instant::now()`/`SystemTime::now()` through the vDSO, CPU-timing side channels, reads of already-mapped memory, and anything done in phase A.
* **Scoping inside a category is partial.** seccomp cannot see paths or hosts; Landlock scopes filesystem paths, but network host:port scoping relies on observation of addresses, not enforcement.
* **Dependencies and build scripts** are covered only to the extent their *run-time* behaviour occurs in phase B; build-time behaviour (`build.rs`, proc macros) is not.
* **A compromised kernel, `strace`, or Docker** defeats the gate. The two rings agreeing reduces, but does not remove, the risk of a shared blind spot (both use the same syscall table; the table is tested against fixtures).
* **Platforms other than Linux are not covered at all** and say so.
