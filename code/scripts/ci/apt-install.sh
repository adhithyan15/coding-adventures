#!/usr/bin/env bash
#
# Install Ubuntu-archive packages on a CI runner, without letting an unrelated
# third-party repository fail the job.
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

# Neither of these is allowed to fail quietly: the whole point is that update
# keeps meaning "the archive we depend on is reachable".
$SUDO apt-get update
$SUDO apt-get install -y "$@"
