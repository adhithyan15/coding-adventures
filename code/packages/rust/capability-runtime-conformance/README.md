# capability-runtime-conformance

The VER08b **runtime capability conformance gate**: it proves, by running the
package's public-API test program, that a library does only what its
`required_capabilities.json` says, and fails on anything else.

The static analyzer (VER08a) reads source and can be evaded by macros, FFI,
`build.rs` and dependencies. This gate asks the kernel what actually happened,
with two mechanisms that must agree:

```text
 required_capabilities.json
        |  capability_os_sandbox::plan_from_json(Linux)
        v
   RuntimePolicy  = minimal baseline  U  allowances of each declared capability
        |
        +--> ENFORCEMENT  no_new_privs + Landlock + seccomp-BPF(KILL by default),
        |                 armed in-process right before the first library call
        +--> OBSERVATION  parent runs the child under `strace -f -v`, classifies
        |                 every syscall after the ARMED marker
        +--> CONTAINER    docker run --network none --read-only --cap-drop ALL
                          --security-opt seccomp=<generated profile> (no mounts, no env)
```

Negative controls (18 forbidden operations, each run enforce-only,
observe-only and both) prove the gate fails when it should. Results go to
`capability_runtime_evidence.json`. See
[`code/specs/VER08b-runtime-capability-conformance.md`](../../../specs/VER08b-runtime-capability-conformance.md).

## Usage

A package adds one `harness = false` test target. Phase A builds the inputs;
the closure it returns runs only after the sandbox is armed:

```rust
use capability_runtime_conformance::{run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings};

fn prepare() -> Exercise {
    let vectors = load_vectors();                    // phase A: unconstrained
    Box::new(move || {                               // phase B: armed
        vec![Check { name: "ct_eq".into(), ok: run(&vectors) }]
    })
}

fn main() {
    std::process::exit(run_package_gate(&GateConfig {
        package: "rust/ct-compare",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir: option_env!("CARGO_TARGET_TMPDIR").map(Into::into),
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    }));
}
```

The pilot is `code/packages/rust/ct-compare/tests/runtime_capability.rs`.

## Platforms

| Platform | Result |
|---|---|
| Linux x86_64 / aarch64 | live (needs `strace`; Docker ring needs a reachable daemon) |
| macOS, Windows | **NOT-LIVE** (`RCG010`), reported in output and evidence, never a pass |
| Linux without strace/ptrace | **NOT-LIVE** (`RCG013`) |
| strace present but an observed run leaves no trace (absent, unreadable or empty) | **FAIL** (`RCG013`), strict or not: a broken observer is not a clean observation |

Environment switches: `RCG_STRICT=1` turns any not-live into a failure;
`RCG_REQUIRE_DOCKER=1` makes an unavailable Docker ring a failure.

## Modules

| Module | Role |
|---|---|
| `policy` | manifest -> plan -> `RuntimePolicy`; the baseline and its justifications |
| `bpf` | policy -> classic-BPF seccomp program, plus a software evaluator for tests |
| `arm` | installs no_new_privs, Landlock and seccomp into this process |
| `observe` | parses `strace -f -v` output and classifies post-arm syscalls |
| `harness` | the parent/child driver, judgement table, controls, evidence |
| `sabotage` | the negative controls |
| `container` | Docker invocation and seccomp profile generation |
| `failure` | the `RCG001`..`RCG014` registry |

## What it cannot prove

Only the exercised paths; not environment reads, vDSO time, or timing channels;
host:port scoping for `net` is observed rather than enforced. It complements the
static analyzer and does not replace it. The spec's section 14 lists the limits.

## Development

```bash
cargo test -p capability-runtime-conformance -- --nocapture
cargo clippy -p capability-runtime-conformance --all-targets -- -D warnings
```
