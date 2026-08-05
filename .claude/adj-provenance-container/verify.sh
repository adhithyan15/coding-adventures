#!/usr/bin/env bash
# Run the ADJ provenance verifier on a Linux host with a delegated cgroup v2 root.
#
# Mirrors .github/workflows/ci.yml: create a cgroup, hand it to the current
# process, export ADJ_PROVENANCE_CGROUP_ROOT, verify, clean up.
#
# NOT `--privileged`. That grants CAP_SYS_ADMIN and unmasks /proc and /sys, which
# is enough for the container to mount the host's virtiofs share itself and read
# the developer's entire home directory — SSH keys included. It matters here
# because `cargo build` below executes build scripts and proc macros from the
# crates.io dependency tree as root. All the guardian actually needs is ordinary
# file I/O on a writable cgroup tree, so it gets exactly that: a bind mount of
# /sys/fs/cgroup plus the host cgroup namespace.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
IMAGE="adj-provenance-verify:local"

docker build --pull -q -t "$IMAGE" "$REPO_ROOT/.claude/adj-provenance-container" >/dev/null

docker run --rm --cgroupns=host \
  -v /sys/fs/cgroup:/sys/fs/cgroup \
  -v "$REPO_ROOT":/repo -w /repo \
  "$IMAGE" bash -c '
set -euo pipefail

# The verifier execs these, so they must be Linux binaries. Built into a
# container-local target dir so a host build is never clobbered by a Linux one.
# These are TRUSTED binaries — the verifier believes their output — so their
# dependency set should not be re-resolved from the network on every run. But
# this repo gitignores Cargo.lock deliberately, so --locked cannot be
# unconditional: it would fail outright on a fresh clone. Use it when a lockfile
# is actually there, and say so when it is not, rather than silently resolving.
export CARGO_TARGET_DIR=/tmp/adj-target
LOCKED=""
if [ -f code/packages/rust/Cargo.lock ]; then
  LOCKED="--locked"
else
  echo "note: no Cargo.lock present; dependencies will be resolved fresh" >&2
fi
cargo build --quiet $LOCKED --manifest-path code/packages/rust/Cargo.toml \
  -p adj-lang-cli --bin adj-formula-inventory --bin adj-formula-audit

# A UUID, not $$: inside `docker run ... bash -c` the PID is always 1, so $$
# would name one fixed path and two concurrent runs (routine here — this repo is
# worked in several worktrees at once) would share a delegated root, with the
# first to finish rmdir-ing it out from under the second. Plain mkdir, no -p, so
# a collision or a stale root from a crashed run is an error rather than a silent
# adoption. CI takes the same care via GITHUB_RUN_ID/RUN_ATTEMPT.
ROOT="/sys/fs/cgroup/adj-provenance-$(cat /proc/sys/kernel/random/uuid)"
mkdir "$ROOT"

# Captured BEFORE moving: cleanup must return the shell to the cgroup it came
# from, not to the VM root. Restoring to the root would leave cargo, python and
# the guardian outside the container cgroup, so Docker limits and accounting
# would stop applying to them.
ORIGINAL="$(awk -F: '"'"'$1 == "0" { print $3 }'"'"' /proc/self/cgroup)"
cleanup() {
  echo $$ > "/sys/fs/cgroup${ORIGINAL}/cgroup.procs" 2>/dev/null || true
  rmdir "$ROOT" 2>/dev/null || true
}
# Registered immediately after mkdir, so a failure in the move below still
# removes the cgroup instead of leaking it into the VM.
trap cleanup EXIT

echo $$ > "$ROOT/cgroup.procs"
export ADJ_PROVENANCE_CGROUP_ROOT="$ROOT"

python3 code/scripts/adj_stdlib_provenance.py verify \
  --formula-inventory-binary /tmp/adj-target/debug/adj-formula-inventory \
  --formula-audit-binary /tmp/adj-target/debug/adj-formula-audit
'
