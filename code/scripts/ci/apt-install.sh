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

# `--print-uris` (the .deb cache's listing, below) reads only the local index
# that `update` already fetched -- no network -- and returns in about a
# second. Thirty seconds bounds it without spending a step's budget on it.
APT_LIST_TIMEOUT="${APT_LIST_TIMEOUT:-30}"

# Bash evaluates the operands of `-ge` and `$(( ))` as arithmetic, and an
# arithmetic "number" like `a[$(cmd)]` runs cmd. Only someone who already
# controls the runner's environment could set these, but a plain-digits check
# costs nothing and makes the knobs mean only what they say.
for knob in APT_ATTEMPTS APT_RETRY_DELAY APT_UPDATE_TIMEOUT APT_INSTALL_TIMEOUT APT_LIST_TIMEOUT; do
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

# ## Not downloading at all: a verified .deb cache (#17191)
#
# The layers above assume a bad mirror is a STALLED one -- silent, or nearly
# so -- and that the next attempt lands somewhere healthy. The books job then
# met a mirror that was neither: it served the 180 MB TeX download at a steady
# ~150 KB/s on every attempt.
#
#     Get:6 ... texlive-plain-generic all 2023.20240207-1 [29.0 MB]
#     error: apt-get install failed on all 3 attempt(s) (no result after 270s)
#
# At that rate the download alone takes about twenty minutes, so no choice of
# per-attempt cap fits inside a 20-minute step: each attempt was making real
# progress when it was cut off. The remedy is to stop depending on mirror speed
# for packages that do not change between runs.
#
# APT_DEB_CACHE names a directory the workflow persists between runs (with
# actions/cache). Before `install`, every .deb found there is checked against
# the SHA512 (or SHA256) that the Packages index records for it -- the index
# `update` has just fetched and verified against the archive's signed Release
# file, read back with `apt-cache show` -- and only an exact match (hash and
# size) is copied into apt's archive directory, where apt then uses it instead
# of downloading. After a successful install the directory is refilled with
# exactly the archives this install used, so it never accumulates superseded
# versions.
#
# The cache is a file that came back from outside this run, so this script
# does not rely on whatever apt might or might not re-check about a file that
# is already sitting in its archive directory: nothing gets there from the
# cache unless its hash matches the signed index. That keeps the archive the
# only authority over what is installed.
#
# The cache can only ever save time. A missing, empty, stale or tampered cache
# means that package downloads as before; no failure in seeding or refilling
# the cache fails the install.
#
# Testing hooks for this part:
#   APT_DEB_CACHE     the persisted directory (unset: no caching at all)
#   APT_ARCHIVES_DIR  apt's archive directory (default /var/cache/apt/archives)
#   APT_LIST_TIMEOUT  wall-clock seconds for the listing (default 30, above)
#   APT_CACHE         the apt-cache binary (default `apt-cache`)

APT_DEB_CACHE="${APT_DEB_CACHE:-}"
APT_ARCHIVES_DIR="${APT_ARCHIVES_DIR:-/var/cache/apt/archives}"
APT_CACHE="${APT_CACHE:-apt-cache}"

# The local file name apt gives an archive: package_version_arch.deb, with an
# epoch's colon spelled %3a. Anything else -- a slash above all -- is refused,
# so a hostile line can never name a path outside the two directories.
DEB_NAME='^[A-Za-z0-9][A-Za-z0-9.+~%_-]*\.deb$'

# The strongest hash the signed index records for one archive, as "ALGO hex".
#
#   strong_hash_for <package> <decoded URI> <size>
#
# `--print-uris` names each archive's hash, but it prints the MD5 sum when the
# index has one -- and Ubuntu's always does -- so its own field cannot be the
# check. The same index carries SHA256 and SHA512 for every archive, and
# `apt-cache show` prints them. A package can have several stanzas (versions,
# pockets, sources), so the one chosen is the stanza whose whole pool path
# (`Filename:`, e.g. pool/main/l/latexmk/latexmk_4.83-1_all.deb) ends the URI
# apt would fetch, right after a slash, and whose size matches too.
strong_hash_for() {
  "$APT_CACHE" show -- "$1" 2>/dev/null | awk -v uri="$2" -v size="$3" '
    BEGIN { RS = ""; FS = "\n" }
    {
      file = ""; bytes = ""; sha256 = ""; sha512 = ""
      for (i = 1; i <= NF; i++) {
        if ($i ~ /^Filename: /) { file = substr($i, 11) }
        else if ($i ~ /^Size: /) { bytes = substr($i, 7) }
        else if ($i ~ /^SHA256: /) { sha256 = substr($i, 9) }
        else if ($i ~ /^SHA512: /) { sha512 = substr($i, 9) }
      }
      tail = length(uri) - length(file)
      if (file != "" && tail > 0 && substr(uri, tail) == "/" file && bytes == size) {
        if (sha512 != "") { print "SHA512 " sha512; exit }
        if (sha256 != "") { print "SHA256 " sha256; exit }
      }
    }'
}

seed_archives_from_cache() {
  [[ -n "$APT_DEB_CACHE" && -d "$APT_DEB_CACHE" ]] || return 0

  # One line per archive apt would download (the hash field is ignored; see
  # strong_hash_for above):
  #   'http://.../pool/main/l/latexmk/latexmk_4.83-1_all.deb' latexmk_1%3a4.83-1_all.deb 203324 MD5Sum:...
  local uris
  if ! uris=$($SUDO env DEBIAN_FRONTEND=noninteractive \
    timeout --kill-after=15 "$APT_LIST_TIMEOUT" \
    "$APT_GET" "${APT_OPTS[@]}" -qq --print-uris install -y "$@"); then
    echo "deb cache: could not list the archives to fetch; downloading all of them" >&2
    return 0
  fi

  local seeded=0 rejected=0 absent=0
  local uri name size _hash pool cached staged expected algo want got
  while read -r uri name size _hash; do
    [[ -n "${size:-}" ]] || continue
    if [[ ! $name =~ $DEB_NAME || ! $size =~ ^[0-9]+$ ]]; then
      rejected=$((rejected + 1))
      continue
    fi
    cached="$APT_DEB_CACHE/$name"
    if [[ ! -f "$cached" || -L "$cached" ]]; then
      absent=$((absent + 1))
      continue
    fi
    # Size first: it is free, and a wrong size is already a rejection, so a
    # huge or truncated file is never read end to end.
    if [[ "$(stat -c %s "$cached")" != "$size" ]]; then
      rejected=$((rejected + 1))
      continue
    fi
    # The index names the pool path (no epoch, `+` as is); the URI names the
    # same path percent-encoded. Decode it to compare like with like. The
    # result is only ever compared as a string, never used as a path.
    pool="${uri//\'/}"
    pool=$(printf '%b' "${pool//%/\\x}")
    expected=$(strong_hash_for "${name%%_*}" "$pool" "$size")
    algo="${expected%% *}"
    want="${expected#* }"
    case "$algo" in
      SHA256|SHA512) ;;
      # No strong hash in the index for this file: nothing to trust it on.
      *) rejected=$((rejected + 1)); continue ;;
    esac
    # Copy first, then hash the COPY: a root-owned file in apt's own partial
    # directory, which nothing else in the job can rewrite between the check
    # and apt's use of it. Only a match is moved into place.
    staged="$APT_ARCHIVES_DIR/partial/$name.from-cache"
    if ! $SUDO mkdir -p "$APT_ARCHIVES_DIR/partial" ||
      ! $SUDO cp -- "$cached" "$staged"; then
      rejected=$((rejected + 1))
      continue
    fi
    case "$algo" in
      SHA256) got=$(sha256sum < "$staged") ;;
      SHA512) got=$(sha512sum < "$staged") ;;
    esac
    got="${got%% *}"
    if [[ "$got" == "${want,,}" && "$(stat -c %s "$staged")" == "$size" ]] &&
      $SUDO mv -f -- "$staged" "$APT_ARCHIVES_DIR/$name"; then
      seeded=$((seeded + 1))
    else
      $SUDO rm -f -- "$staged"
      rejected=$((rejected + 1))
    fi
  done <<< "$uris"

  echo "deb cache: seeded $seeded verified archive(s); $absent not cached; $rejected rejected"
}

refill_cache_from_archives() {
  [[ -n "$APT_DEB_CACHE" ]] || return 0

  mkdir -p "$APT_DEB_CACHE"
  local deb kept=0
  shopt -s nullglob
  rm -f -- "$APT_DEB_CACHE"/*.deb
  for deb in "$APT_ARCHIVES_DIR"/*.deb; do
    [[ -f "$deb" && ! -L "$deb" ]] || continue
    cp -- "$deb" "$APT_DEB_CACHE/" && kept=$((kept + 1))
  done
  shopt -u nullglob

  echo "deb cache: kept $kept archive(s) in $APT_DEB_CACHE for the next run"
}

# Both cache steps run in `||` context, which switches `set -e` off inside
# them; every failure path in them is handled explicitly for that reason, so
# keep the `||` if this is ever refactored.
#
# Neither apt call is allowed to fail quietly: the whole point is that update
# keeps meaning "the archive we depend on is reachable". Retrying is not
# swallowing -- the last attempt's failure is still this script's exit status,
# and a package that genuinely does not exist fails every attempt. The cache
# steps around install are the opposite: they may fail, and only say so.
apt_with_retry "$APT_UPDATE_TIMEOUT" update
seed_archives_from_cache "$@" || echo "deb cache: seeding failed; downloading instead" >&2
apt_with_retry "$APT_INSTALL_TIMEOUT" install -y "$@"
refill_cache_from_archives || echo "deb cache: could not refill $APT_DEB_CACHE" >&2
