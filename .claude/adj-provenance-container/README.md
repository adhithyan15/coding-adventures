# Local provenance verification

`adj_stdlib_provenance.py verify` only runs on Linux. `_PosixGuardian.__init__`
fails outright unless `sys.platform.startswith("linux")` — there is no flag and
no environment override. That is deliberate: ADJ-STDLIB-COVERAGE §13i chose to
*reject* off-Linux rather than "silently weaken the claim", because macOS and
generic POSIX have no equivalent unprivileged hard container for the cgroup-v2
containment the verifier depends on.

So on a Mac the whole Wave-1 provenance track — and every library migration —
is CI-only-verifiable, and each iteration costs a full CI round trip.

This harness removes that cost **without touching the containment code**. It
reproduces the Linux the verifier already expects, rather than relaxing what the
verifier will accept.

## Use

```
./.claude/adj-provenance-container/verify.sh
```

Expected on a clean tree:

```json
{ "bundles": 11, "objects": 125, "snapshots": 11, "valid": true }
```

Prerequisite: any Docker-compatible runtime. On macOS:

```
brew install colima docker && colima start --cpu 4 --memory 8
```

## Two things that are not obvious

**The verifier execs the trusted binaries.** `adj-formula-inventory` and
`adj-formula-audit` are launched as subprocesses, so a macOS build cannot be
reused — the script builds them *inside* the container. It uses a separate
`CARGO_TARGET_DIR=/tmp/adj-target` so a host build is never clobbered by a Linux
one (or vice versa).

**The guardian needs a writable cgroup tree, and nothing more.** It only ever
does `mkdir`, writes `cgroup.procs` and `cgroup.kill`, and `rmdir`s — ordinary
file I/O as uid 0 in the container. So it gets `--cgroupns=host` plus a bind
mount of `/sys/fs/cgroup`, and the script then mirrors
`.github/workflows/ci.yml`: create a cgroup, move the shell in, export
`ADJ_PROVENANCE_CGROUP_ROOT`, remove it on exit.

**Deliberately not `--privileged`.** An earlier version used it, and that was a
real hole rather than a theoretical one: `--privileged` grants CAP_SYS_ADMIN and
unmasks `/proc` and `/sys`, which is enough for the container to mount the host
share itself — under colima it reaches the developer's entire home directory,
SSH keys included, regardless of what is bind-mounted. That matters here
specifically because `cargo build` executes build scripts and proc macros from
the crates.io dependency tree as root. The narrower flags run the identical
workload to the identical result.

## What this does and does not change

It changes nothing about what the verifier accepts. The invocation matches CI's
apart from the binary paths, every strict default is left alone, and the only
environment variable the verifier reads is `ADJ_PROVENANCE_CGROUP_ROOT`, set to
a genuine delegated root that the guardian re-validates by `statfs`.

For that claim to hold, two things are pinned rather than left to drift:

- **The base image, by digest.** These binaries are trusted — the verifier
  believes their output — so the toolchain compiling them is inside the trust
  boundary.
- **`jsonschema==4.26.0`, matching CI.** Debian's `python3-jsonschema` is
  4.19.2, and validator behaviour changed between the two. Left alone, a
  manifest could validate here and fail in CI, which would make this harness
  actively misleading rather than merely different.

`Cargo.lock` is gitignored in this repo, so `--locked` is used when a lockfile is
present and the run says so when one is not.
