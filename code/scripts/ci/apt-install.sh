#!/usr/bin/env bash
#
# Install Ubuntu-archive packages on a CI runner, without letting an unrelated
# third-party repository fail the job, and without letting a stalled mirror
# hang it (see "Bounding apt in time" near the end).
#
# ## Why this exists
#
# `apt-get update` exits 100 if *any* configured repository fails, and the
# GitHub-hosted runner images ship with vendor repositories preinstalled that
# this repository never installs from. When `packages.microsoft.com` returned
# 403, a docs-only pull request -- four Markdown files, no source, no CI change
# -- failed its required `build (ubuntu-latest)` job:
#
#     Err:6 https://packages.microsoft.com/repos/azure-cli noble InRelease
#       403  Forbidden [IP: 13.107.213.41 443]
#     E: The repository '...' is no longer signed.
#     ##[error]Process completed with exit code 100.
#
# Nothing here depends on those repositories. The fix is to drop them before
# updating, so `apt-get update`'s exit status once again means something about
# the archive we actually use.
#
# ## What this deliberately does NOT do
#
# It is not `apt-get update || true`. That would hide a genuinely unavailable
# Ubuntu archive and turn a loud failure into a confusing one later, when the
# install fails for a reason that no longer names the cause. Update still has to
# succeed, and install still fails if a package cannot be found.
#
# ## The trap
#
# On Ubuntu 24.04 the **main archive itself** is declared in
# `/etc/apt/sources.list.d/ubuntu.sources`, in deb822 format -- not in
# `/etc/apt/sources.list`, which is a stub. So "delete everything in
# sources.list.d" removes the archive every package here comes from, and the
# job then fails to find `libcairo2-dev` rather than failing to reach Microsoft.
# Pruning is therefore by explicit vendor pattern, and the result is asserted
# below before anything is installed.
#
# Usage:
#   apt-install.sh <apt-get install args...>
#   apt-install.sh libcairo2-dev
#   apt-install.sh --no-install-recommends texlive-xetex latexmk
#
# Testing hooks:
#   APT_SOURCES_DIR   where the source lists live (default the real path)
#   APT_PRUNE_ONLY=1  prune, report, and stop -- do not touch apt at all

set -euo pipefail

# This prunes system package sources with `sudo rm`. On a runner that is
# correct -- the VM is discarded after the job. On a developer's machine it
# would silently delete their VS Code, dotnet, and moby repository config, and
# they would find out days later when an update stopped seeing them. So it
# refuses to run outside CI unless someone says otherwise in as many words.
if [[ -z "${CI:-}" && -z "${APT_INSTALL_FORCE:-}" && -z "${APT_PRUNE_ONLY:-}" ]]; then
  echo "error: this script removes system apt sources and is meant for CI." >&2
  echo "       Set APT_INSTALL_FORCE=1 if you really mean to run it here." >&2
  exit 3
fi

SOURCES_DIR="${APT_SOURCES_DIR:-/etc/apt/sources.list.d}"
SOURCES_LIST="${APT_SOURCES_LIST:-/etc/apt/sources.list}"

# Overridable so the tests can exercise the pruning without root and without a
# real apt. On a runner this is plain `sudo`.
SUDO="${APT_SUDO-sudo}"

# Which source lists to KEEP. An allowlist, not a list of vendors to drop.
#
# The first version enumerated vendors -- microsoft, azure-cli -- and shipped.
# The diagnostic below then reported what actually survived on the runner:
#
#     pruned 2 vendor source list(s): microsoft-prod.list azure-cli.sources
#     apt sources remaining: google-chrome.sources ubuntu.sources
#
# `google-chrome.sources` is a repository nothing here installs from, and it
# can 403 exactly the way Microsoft's did. A denylist over repositories someone
# else decides to preinstall can only ever be as current as the last time
# somebody looked at a runner image, and every miss is silent until an outage.
#
# What this project depends on is a closed set: every package installed across
# all six workflows comes from the Ubuntu archive, and no workflow runs
# `add-apt-repository`. So the safe shapes are named instead, and a vendor
# repository added to some future image cannot fail a job here.
#
# Same correction, and the same reasoning, as the release archiver's member
# names -- see "a denylist over filename hazards is unwinnable" in lessons.md.
KEEP_PATTERNS=(
  "ubuntu.sources"
  "ubuntu.list"
  "ubuntu-*.sources"
  "ubuntu-*.list"
)

# The escape hatch. Nothing needs it today -- no workflow adds a repository --
# but the day one does, it will add the PPA and then call this script, which
# would delete it a line later and fail with "unable to locate package"
# pointing at the package rather than at us. Space-separated globs.
#
#   APT_KEEP="deadsnakes*" bash apt-install.sh python3.13
# `${arr[@]}` on an EMPTY array is an unbound-variable error under `set -u` in
# bash 3.2, which is what macOS ships -- the runners have bash 5, where it is
# fine, so this would have passed CI and failed for anyone running it locally.
read -r -a extra_keep <<< "${APT_KEEP:-}"
if [[ ${#extra_keep[@]} -gt 0 ]]; then
  KEEP_PATTERNS+=("${extra_keep[@]}")
fi

keep_this() {
  local name="$1" pattern
  for pattern in "${KEEP_PATTERNS[@]}"; do
    # Unquoted on purpose: the patterns ARE globs (`ubuntu-*.sources`).
    # shellcheck disable=SC2053
    [[ -n "$pattern" && "$name" == $pattern ]] && return 0
  done
  return 1
}

pruned=()
if [[ -d "$SOURCES_DIR" ]]; then
  shopt -s nullglob
  for path in "$SOURCES_DIR"/*.list "$SOURCES_DIR"/*.sources; do
    [[ -f "$path" ]] || continue
    name="$(basename "$path")"
    if keep_this "$name"; then
      continue
    fi
    $SUDO rm -f "$path"
    pruned+=("$name")
  done
  shopt -u nullglob
fi

if [[ ${#pruned[@]} -gt 0 ]]; then
  echo "pruned ${#pruned[@]} unused source list(s): ${pruned[*]}"
else
  echo "no unused source lists to prune"
fi

# The guard on the pruning above: did we just delete everything?
#
# Deliberately NOT "does a known Ubuntu mirror hostname appear somewhere".
# The first version of this asked exactly that, and failed on the real runner
# while passing every local fixture -- because it encoded my guess about which
# mirror and which file layout the image uses, and the guess was wrong. The
# invariant that actually matters does not need to know any of that: pruning
# must leave at least one source behind that we did not prune.
remaining=()
if [[ -d "$SOURCES_DIR" ]]; then
  shopt -s nullglob
  for path in "$SOURCES_DIR"/*.list "$SOURCES_DIR"/*.sources; do
    [[ -f "$path" ]] && remaining+=("$(basename "$path")")
  done
  shopt -u nullglob
fi
if [[ -f "$SOURCES_LIST" ]] && grep -Eq '^[[:space:]]*(deb|deb-src|Types:)' "$SOURCES_LIST" 2>/dev/null; then
  remaining+=("$(basename "$SOURCES_LIST")")
fi

# Printed unconditionally, not only on failure. When this went wrong in CI the
# error said what it concluded and not what it saw, so diagnosing it needed
# another run. One line now saves that round trip.
echo "apt sources remaining: ${remaining[*]:-(none)}"

if [[ ${#remaining[@]} -eq 0 ]]; then
  echo "error: pruning removed every configured apt source." >&2
  echo "       Every package this script installs comes from the Ubuntu" >&2
  echo "       archive, so continuing would fail with a misleading 'unable to" >&2
  echo "       locate package'. Check KEEP_PATTERNS in $0 -- the Ubuntu" >&2
  echo "       archive's source list is not being matched by any of them." >&2
  exit 1
fi

if [[ -n "${APT_PRUNE_ONLY:-}" ]]; then
  echo "APT_PRUNE_ONLY set; stopping before apt-get"
  exit 0
fi

if [[ $# -eq 0 ]]; then
  echo "error: no packages given" >&2
  exit 2
fi

# ## Bounding apt in time (#12163, #12181)
#
# Pruning answers "a repository we do not use is down". It does nothing for
# the other way apt fails a job: a mirror we DO use that accepts the
# connection and then stops talking. That happened on `build (ubuntu-latest)`
# repeatedly --
#
#     Get:5 https://archive.ubuntu.com/ubuntu noble-security InRelease [126 kB]
#     <nothing for 5h59m>
#     ##[error]The operation was canceled.
#
# -- and because nothing bounded the step, each stall burned the full six-hour
# job limit and surfaced as `cancelled`, which reads as "superseded", not as
# "re-run me". Three layers now bound it, from the innermost out:
#
#   1. apt's own knobs (APT_OPTS below). They make a dead connection fail in
#      seconds and retry the individual download, which is what fixes the
#      common case without anyone noticing.
#   2. A wall-clock cap on each apt-get invocation (`timeout`), so a stall the
#      knobs do not catch -- a mirror trickling one byte at a time never trips
#      an inactivity timeout -- becomes a failed ATTEMPT, not a hung job.
#   3. A short retry loop over those attempts, because a mirror stall is
#      transient by nature and the next attempt usually lands on a healthy
#      mirror connection.
#
# The workflow steps that call this script also carry `timeout-minutes`, the
# outermost backstop: if every layer here were somehow wrong, the step still
# fails in minutes rather than hours.
#
# Testing hooks for this part:
#   APT_GET              the apt-get binary (default `apt-get`)
#   APT_ATTEMPTS         attempts per apt-get command (default 3)
#   APT_RETRY_DELAY      base backoff in seconds, doubled each retry (default 10)
#   APT_UPDATE_TIMEOUT   wall-clock seconds per `update` attempt (default 120)
#   APT_INSTALL_TIMEOUT  wall-clock seconds per `install` attempt (default 600)

APT_GET="${APT_GET:-apt-get}"
APT_ATTEMPTS="${APT_ATTEMPTS:-3}"
APT_RETRY_DELAY="${APT_RETRY_DELAY:-10}"

# `update` fetches a handful of index files and normally takes 2-15 seconds on
# a runner; two minutes is an order of magnitude of headroom, and still short
# enough that three attempts fit inside a 10-minute step.
APT_UPDATE_TIMEOUT="${APT_UPDATE_TIMEOUT:-120}"

# `install` is where the size varies: libcairo2-dev takes ~15s, the TeX Live
# install in human-languages-books.yml up to ~2 minutes. Ten minutes covers
# the largest with ~5x headroom. For the small installs the step's own
# `timeout-minutes` is the tighter bound, and that is fine -- the stalls seen
# so far were all in `update`, which the tighter cap above covers.
APT_INSTALL_TIMEOUT="${APT_INSTALL_TIMEOUT:-600}"

# Bash evaluates the operands of `-ge` and `$(( ))` as arithmetic, and an
# arithmetic "number" like `a[$(cmd)]` runs cmd. Only someone who already
# controls the runner's environment could set these, but a plain-digits check
# costs nothing and makes the knobs mean only what they say.
for knob in APT_ATTEMPTS APT_RETRY_DELAY APT_UPDATE_TIMEOUT APT_INSTALL_TIMEOUT; do
  if [[ ! ${!knob} =~ ^[0-9]+$ ]]; then
    echo "error: $knob must be a non-negative integer, got '${!knob}'." >&2
    exit 2
  fi
done

APT_OPTS=(
  # Retry each individual download up to three times before giving up on it.
  # Newer apt defaults to this already; saying it keeps the behaviour the same
  # on an older image and documents that we rely on it.
  -o Acquire::Retries=3
  # How long apt waits on a connect, or on a connection that has gone silent,
  # before treating the fetch as failed (apt's default is 120s). A healthy
  # mirror answers in well under a second, so 30s only ever trims a stall.
  -o Acquire::http::Timeout=30
  -o Acquire::https::Timeout=30
  # Without this apt fails IMMEDIATELY if something else (an image's
  # unattended-upgrades, say) holds the dpkg lock; with it apt waits for the
  # lock -- but only for a minute, so a stuck holder cannot become our hang.
  -o DPkg::Lock::Timeout=60
)

# Run one apt-get command under a wall-clock cap, retrying with backoff.
#
#   apt_with_retry <seconds-per-attempt> <apt-get args...>
#
# `timeout` runs INSIDE sudo, as root, so that its SIGKILL (sent 15s after the
# polite SIGTERM, if apt ignores that) reaches apt-get itself; run outside sudo
# it could only signal sudo. `env DEBIAN_FRONTEND=noninteractive` is likewise
# inside sudo because sudo resets the environment: a package whose postinst
# asks a debconf question must take the default, not wait on a terminal that
# CI does not have.
apt_with_retry() {
  local budget="$1"
  shift
  local attempt=1 status delay="$APT_RETRY_DELAY"
  while true; do
    status=0
    $SUDO env DEBIAN_FRONTEND=noninteractive \
      timeout --kill-after=15 "$budget" \
      "$APT_GET" "${APT_OPTS[@]}" "$@" || status=$?
    if [[ $status -eq 0 ]]; then
      return 0
    fi
    # 124 is timeout(1)'s "the cap fired"; 137 is SIGKILL after it did. Named
    # in the log so a stall reads as a stall, not as a package problem.
    local why="exit $status"
    if [[ $status -eq 124 || $status -eq 137 ]]; then
      why="no result after ${budget}s; treated as a stalled mirror"
    fi
    if [[ $attempt -ge $APT_ATTEMPTS ]]; then
      echo "error: apt-get $1 failed on all $APT_ATTEMPTS attempt(s) ($why)." >&2
      return "$status"
    fi
    echo "apt-get $1 attempt $attempt/$APT_ATTEMPTS failed ($why); retrying in ${delay}s" >&2
    sleep "$delay"
    attempt=$((attempt + 1))
    delay=$((delay * 2))
  done
}

# Neither of these is allowed to fail quietly: the whole point is that update
# keeps meaning "the archive we depend on is reachable". Retrying is not
# swallowing -- the last attempt's failure is still this script's exit status,
# and a package that genuinely does not exist fails every attempt.
apt_with_retry "$APT_UPDATE_TIMEOUT" update
apt_with_retry "$APT_INSTALL_TIMEOUT" install -y "$@"
