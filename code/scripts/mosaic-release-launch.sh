#!/usr/bin/env bash
# mosaic-release-launch.sh -- launch a release payload and require it to stay up.
#
#   usage: mosaic-release-launch.sh SECONDS LOG TARGET [ARG...]
#
# The release lanes' upgrade/corrupt-recovery gate
# (code/specs/mosaic-release-state-gate-v1.md, "Launching a payload") needs one
# question answered about a freshly extracted app: did it start and keep
# running? An app that exits early either crashed or refused to start without
# its Rust runtime, and either way the state it left behind proves nothing.
#
# On Linux the lanes answer it with `timeout`, whose exit 124 means "it was
# still running when I killed it". macOS runners have no GNU `timeout`, so
# this script is the macOS answer, shared by every app's release lane so the
# rule cannot drift between them:
#
#   1. Run TARGET from `/` (so nothing resolves relative to the checkout),
#      stdout and stderr into LOG. The environment is the caller's: callers
#      `env -u MOSAIC_APP_LIBRARY -u MOSAIC_APP_STATE_PATH` in front of us.
#   2. Once a second for SECONDS seconds, check the process is alive.
#   3. Stop it -- TERM, then KILL if it is still there 10 seconds later.
#   4. Print the log, and fail if it says the runtime did not load.
#
# TARGET may be an executable or a macOS `.app` bundle. A bundle is resolved
# through `Contents/Info.plist`'s CFBundleExecutable, which must be a bare file
# name: a plist naming `../../elsewhere` does not get to choose what runs.
#
# Exit status -- distinct, so a caller can run crash diagnostics (lldb, crash
# reports) only when there was a crash:
#
#   0   alive for the whole window, stopped, log has no runtime-unavailable
#   1   usage error, unresolvable target, or "Mosaic Rust runtime unavailable"
#   2   the app exited before its time was up
#
# Lines starting `::` are GitHub Actions workflow commands; elsewhere they are
# just text.

set -uo pipefail

RUNTIME_UNAVAILABLE="Mosaic Rust runtime unavailable"

usage() {
  echo "usage: mosaic-release-launch.sh SECONDS LOG TARGET [ARG...]" >&2
  exit 1
}

[[ $# -ge 3 ]] || usage
seconds="$1"
log="$2"
target="$3"
shift 3

# A positive whole number of seconds. `0` would test nothing: any app is
# "alive" before it has had a chance to crash.
if [[ ! "$seconds" =~ ^[1-9][0-9]*$ ]]; then
  echo "::error::mosaic-release-launch: SECONDS must be a positive integer, got '$seconds'" >&2
  exit 1
fi

# Resolve a `.app` bundle to the executable it names.
#
#   Trestle.app/Contents/Info.plist   CFBundleExecutable = Trestle
#   -> Trestle.app/Contents/MacOS/Trestle
#
# Python's plistlib reads both XML and binary plists on any OS, which keeps
# this testable off macOS; every release job sets Python up already.
executable="$target"
if [[ -d "$target" && "$target" == *.app ]]; then
  plist="$target/Contents/Info.plist"
  if [[ ! -f "$plist" ]]; then
    echo "::error::mosaic-release-launch: $target has no Contents/Info.plist" >&2
    exit 1
  fi
  python_bin="${PYTHON:-python3}"
  if ! name="$("$python_bin" -I -c '
import plistlib, sys
with open(sys.argv[1], "rb") as handle:
    value = plistlib.load(handle).get("CFBundleExecutable")
if not isinstance(value, str):
    sys.exit("CFBundleExecutable is missing or not a string")
print(value)
' "$plist")"; then
    echo "::error::mosaic-release-launch: could not read CFBundleExecutable from $plist" >&2
    exit 1
  fi
  if [[ -z "$name" || "$name" == *"/"* || "$name" == "." || "$name" == ".." ]]; then
    echo "::error::mosaic-release-launch: CFBundleExecutable '$name' is not a bare file name" >&2
    exit 1
  fi
  executable="$target/Contents/MacOS/$name"
fi

if [[ ! -f "$executable" || ! -x "$executable" ]]; then
  echo "::error::mosaic-release-launch: $executable is not an executable file" >&2
  exit 1
fi

label="$(basename "$log")"
mkdir -p "$(dirname "$log")"

# `exec` makes the subshell BECOME the app, so `$!` is the app's own pid and
# `kill -0` asks about the app, not about a shell waiting on it.
(cd / && exec "$executable" "$@") >"$log" 2>&1 &
pid=$!

# If this script is itself interrupted, do not leave the app running on the
# runner to hold files open for the next step.
trap 'kill -KILL "$pid" 2>/dev/null || true' EXIT

# The log is the app's own output. GitHub reads any line of the form
# `::command::` on stdout as a workflow command, so a stray `::endgroup::`
# or `::add-mask::` printed by the app would be obeyed. `::stop-commands::`
# with a one-off token turns command parsing off until the same token is
# echoed back, so the log is shown as plain text.
show_log() {
  local token
  token="mosaic-log-$$-$RANDOM$RANDOM"
  echo "::group::$label"
  echo "::stop-commands::$token"
  cat "$log"
  echo "::$token::"
  echo "::endgroup::"
}

elapsed=0
while [[ "$elapsed" -lt "$seconds" ]]; do
  sleep 1
  elapsed=$((elapsed + 1))
  if ! kill -0 "$pid" 2>/dev/null; then
    wait "$pid"
    status=$?
    trap - EXIT
    show_log
    echo "::error::$label: the app exited with status $status after ${elapsed}s, before its ${seconds}s were up" >&2
    exit 2
  fi
done

# Alive for the whole window: the pass condition. Now stop it.
kill -TERM "$pid" 2>/dev/null || true
for _ in 1 2 3 4 5 6 7 8 9 10; do
  kill -0 "$pid" 2>/dev/null || break
  sleep 1
done
kill -KILL "$pid" 2>/dev/null || true
wait "$pid" 2>/dev/null || true
trap - EXIT

show_log
if grep -Fq "$RUNTIME_UNAVAILABLE" "$log"; then
  echo "::error::$label: the app ran, but could not load its bundled runtime" >&2
  exit 1
fi
echo "$label: still running after ${seconds}s; stopped"
exit 0
