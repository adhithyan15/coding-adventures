#!/usr/bin/env bash
# disk-watch.sh -- say where a CI runner's disk went, before it is gone.
#
# Main's full build has died with "No space left on device" on a different
# shard most days (X-7, #17652). When the disk fills, the runner cannot even
# write its own log: shard 5 of run 39139594621 lost its entire Build Report,
# and the only line left was the runner failing to write GITHUB_STEP_SUMMARY.
# So the evidence has to be printed while there is still room to print it.
#
#   disk-watch.sh report <label>      one full report now
#   disk-watch.sh watch [seconds]     run until killed: a one-line `df` every
#                                     interval (default 300), and a full
#                                     report the first time usage crosses
#                                     70%, 80%, 90% and 95%
#
# A full report is `df` for the root filesystem, the size of every cache and
# build-output directory this repo is known to grow, and the 25 largest
# directories under code/ down to depth 5 (deep enough to separate
# code/packages/rust/target/debug from .../release, and one program's
# target/ from another's).
#
# Everything here is best effort and read-only: a measurement that failed
# must never fail the build it is measuring, so no command's status escapes.

set -u

ROOT_FS=/

# The directories this repo's builds are known or suspected to fill. Missing
# ones are skipped silently: not every shard installs every toolchain.
candidates() {
  local workspace="${GITHUB_WORKSPACE:-$PWD}"
  printf '%s\n' \
    "$workspace/code/packages/rust/target" \
    "$HOME/.cargo" \
    "$HOME/.rustup" \
    "$HOME/.gradle" \
    "$HOME/.android" \
    "$HOME/.m2" \
    "$HOME/.cache" \
    "$HOME/.npm" \
    "$HOME/.pub-cache" \
    "$HOME/.opam" \
    "$HOME/.cabal" \
    "$HOME/.ghcup" \
    "$HOME/.nuget" \
    "$HOME/.local" \
    "${RUNNER_TEMP:-/home/runner/work/_temp}" \
    "${RUNNER_TOOL_CACHE:-/opt/hostedtoolcache}" \
    /tmp
}

used_percent() {
  df -P "$ROOT_FS" 2>/dev/null | awk 'NR == 2 { sub("%", "", $5); print $5 }'
}

report() {
  local label="$1"
  local workspace="${GITHUB_WORKSPACE:-$PWD}"
  echo "== disk-watch: $label ($(date -u +%H:%M:%SZ)) =="
  df -h "$ROOT_FS" 2>/dev/null
  echo "-- known build and cache directories --"
  candidates | while IFS= read -r dir; do
    [ -d "$dir" ] && du -xsh "$dir" 2>/dev/null
  done | sort -rh
  # Toolchain setup actions unpack into RUNNER_TEMP before the build starts
  # (18 GB on PR #17759's first run), so break it down too.
  echo "-- largest entries in RUNNER_TEMP --"
  du -xsh "${RUNNER_TEMP:-/home/runner/work/_temp}"/* "${RUNNER_TEMP:-/home/runner/work/_temp}"/.[!.]* 2>/dev/null | sort -rh 2>/dev/null | head -10
  echo "-- largest directories under code/ (depth 5) --"
  # sort's stderr is dropped: head closing the pipe early is expected.
  du -xh -d 5 "$workspace/code" 2>/dev/null | sort -rh 2>/dev/null | head -25
  echo "== end disk-watch: $label =="
}

watch() {
  local interval="${1:-300}"
  local crossed=""
  while :; do
    local used
    used="$(used_percent)"
    echo "disk-watch $(date -u +%H:%M:%SZ): / ${used:-?}% used; $(df -h "$ROOT_FS" 2>/dev/null | awk 'NR == 2 { print $4 " free" }')"
    for threshold in 70 80 90 95; do
      case " $crossed " in *" $threshold "*) continue ;; esac
      if [ -n "$used" ] && [ "$used" -ge "$threshold" ]; then
        crossed="$crossed $threshold"
        report "crossed ${threshold}%"
      fi
    done
    sleep "$interval"
  done
}

case "${1:-}" in
  report) report "${2:-now}" ;;
  watch) watch "${2:-300}" ;;
  *)
    echo "usage: $0 report <label> | watch [seconds]" >&2
    exit 2
    ;;
esac
exit 0
