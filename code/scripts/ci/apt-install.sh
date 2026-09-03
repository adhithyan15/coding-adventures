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

# Vendor repositories the runner images preinstall and this repository never
# installs from. Matched as shell globs against the file NAME, in both the
# one-line `.list` format and the deb822 `.sources` format, because the runner
# images have been migrating between the two and a pattern that knew only one
# would silently stop pruning.
VENDOR_PATTERNS=(
  "microsoft*"
  "azure-cli*"
  "azure_cli*"
)

pruned=()
if [[ -d "$SOURCES_DIR" ]]; then
  for pattern in "${VENDOR_PATTERNS[@]}"; do
    # `nullglob` so a pattern that matches nothing expands to nothing rather
    # than to the literal pattern -- otherwise this would try to remove a file
    # named `microsoft*` and, under `set -e`, take the job down with it.
    shopt -s nullglob
    for path in "$SOURCES_DIR"/$pattern; do
      # `-f` FOLLOWS symlinks, so this does not exclude a link pointing at
      # a vendor name -- it only decides whether to unlink. That is still the
      # behaviour we want, because `rm -f` unlinks the link rather than its
      # target, so nothing outside this directory is removed either way.
      # Stated plainly because the earlier wording claimed the opposite, and a
      # comment that overstates what a check does invites someone to lean on a
      # guarantee that was never there.
      if [[ -f "$path" ]]; then
        $SUDO rm -f "$path"
        pruned+=("$(basename "$path")")
      fi
    done
    shopt -u nullglob
  done
fi

if [[ ${#pruned[@]} -gt 0 ]]; then
  echo "pruned ${#pruned[@]} vendor source list(s): ${pruned[*]}"
else
  echo "no vendor source lists to prune"
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
  echo "       locate package'. Check VENDOR_PATTERNS in $0 -- one of them is" >&2
  echo "       too broad." >&2
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
