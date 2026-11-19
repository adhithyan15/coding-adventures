#!/usr/bin/env bash
# Launch a Mosaic Android app on an emulator and prove it keeps its state
# (UI89 §3.4 "The gate", §3.7). The Android shape of the iOS simulator gate.
#
#   mosaic-android-emulator-gate.sh <apk> <android-package> <mosaic-application-id> [<activity>]
#
# e.g. for Trestle (TaskApp):
#
#   mosaic-android-emulator-gate.sh app-debug.apk dev.codingadventures.trestle task-app
#
# <activity> is the launcher activity's class name, never a whole component:
# mosaic.android.MosaicActivity (Compose, the default), or .MainActivity for
# a Flutter app (UI89 §7.5). The gate builds the component itself, as
# <android-package>/<activity>.
#
# The emulator must already be booted and the only device adb sees
# (start-mosaic-android-emulator.sh does that). The APK must be a DEBUG build:
# `run-as`, which reads and seeds the app's private files, only works for a
# debuggable app.
#
# Why no UI automation: the host writes its state after every event, and the
# first event needs no finger -- the activity reports the window's
# environment (UI48 ENV4) as soon as it lays out. So a state file appearing
# under the app's `filesDir` proves, in one observation, that the activity
# started, JNA loaded `libmosaic_app.so`, the engine answered an event, and
# the host persisted where MosaicActivity told it to (`filesDir/<application
# id>/mosaic-state.v1.json`; a Flutter app's `main()` finds the same
# directory through `path_provider`). Then three launches, each of which must still be
# running ten seconds later with no uncaught exception:
#
#   launch   | state before           | must see afterwards
#   ---------+------------------------+-------------------------------------
#   1 fresh  | none                   | state written
#   2 again  | what launch 1 wrote    | nothing quarantined, nothing refused
#   3 seeded | `{}` (runtime rejects) | `.corrupt` holding `{}`, fresh state
#
# Launch 2 is the restore test: the runtime took back its own snapshot. The
# host reports a refused snapshot on stderr (logcat's System.err) and moves it
# aside, so both are checked. Launch 3 is the other half: state the runtime
# refuses is quarantined inside the app's own storage and the app still runs.
set -euo pipefail

if [[ $# -ne 3 && $# -ne 4 ]]; then
  echo "usage: $0 <apk> <android-package> <mosaic-application-id> [<activity>]" >&2
  exit 2
fi
apk="$1"
package="$2"
application_id="$3"
# Unset means the default; an empty argument is refused below, not defaulted.
activity_class="${4-mosaic.android.MosaicActivity}"
if [[ ! -f "$apk" ]]; then
  echo "no APK at $apk" >&2
  exit 2
fi
# Both names reach a remote shell command line below, so refuse anything that
# is not the plain identifier each one is.
if [[ ! "$package" =~ ^[A-Za-z][A-Za-z0-9_]*(\.[A-Za-z][A-Za-z0-9_]*)+$ ]]; then
  echo "invalid Android package name: $package" >&2
  exit 2
fi
if [[ ! "$application_id" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]]; then
  echo "invalid Mosaic application id: $application_id" >&2
  exit 2
fi
# A class name only: `adb shell` joins its arguments into one device command
# line, where quoting here does not protect them.
if [[ ! "$activity_class" =~ ^\.?[A-Za-z][A-Za-z0-9_]*(\.[A-Za-z][A-Za-z0-9_]*)*$ ]]; then
  echo "invalid activity class name: $activity_class" >&2
  exit 2
fi

activity="$package/$activity_class"
# Relative to the app's data directory, which is where `run-as` starts.
state="files/$application_id/mosaic-state.v1.json"
corrupt="$state.corrupt"

fail() {
  echo "::error::$*"
  echo "---- logcat (this launch) ----"
  adb logcat -d -v brief 2>/dev/null | tail -n 200 || true
  exit 1
}

# Failure-only, bounded context for the opt-in Flutter Trestle diagnostic APK.
# This runs only after the unchanged state deadline has failed, so it cannot
# give the app additional time to satisfy the gate.
diagnostic_state_failure() {
  [[ "${MOSAIC_NATIVE_DIAGNOSTICS_CAPTURE:-}" == 1 ]] || return 0
  [[ "$package" == "dev.codingadventures.trestle" && "$application_id" == "task-app" ]] || return 0
  [[ "$apk" == *"/mosaic-flutter-trestle-android/flutter/build/app/outputs/flutter-apk/app-debug.apk" ]] || return 0

  echo "---- bounded Flutter Trestle diagnostics (failure only) ----"
  local deadline=$((SECONDS + 6))
  local apk_hash="unavailable"
  if [[ -f "$apk" && $SECONDS -lt $deadline ]] && command -v sha256sum >/dev/null 2>&1 && command -v timeout >/dev/null 2>&1; then
    apk_hash="$(timeout 1 sha256sum "$apk" | cut -c1-64)" || apk_hash="unavailable"
    [[ "$apk_hash" =~ ^[0-9a-f]{64}$ ]] || apk_hash="unavailable"
  fi
  printf 'diagnostic_apk_sha256=%s\n' "$apk_hash"

  local project_root=""
  local lock_hash="unavailable"
  if project_root="$(cd "$(dirname "$apk")/../../../.." 2>/dev/null && pwd)" && [[ -f "$project_root/pubspec.lock" && $SECONDS -lt $deadline ]] && command -v sha256sum >/dev/null 2>&1 && command -v timeout >/dev/null 2>&1; then
    lock_hash="$(timeout 1 sha256sum "$project_root/pubspec.lock" | cut -c1-64)" || lock_hash="unavailable"
    [[ "$lock_hash" =~ ^[0-9a-f]{64}$ ]] || lock_hash="unavailable"
  fi
  printf 'diagnostic_pubspec_lock_sha256=%s\n' "$lock_hash"

  local aapt2=""
  if [[ -n "${ANDROID_HOME:-}" ]]; then
    for candidate in "$ANDROID_HOME"/build-tools/*/aapt2; do
      [[ -x "$candidate" ]] && aapt2="$candidate"
    done
  fi
  local apk_metadata="unavailable"
  if [[ -n "$aapt2" && $SECONDS -lt $deadline ]] && command -v timeout >/dev/null 2>&1; then
    local raw_metadata=""
    raw_metadata="$(timeout 1 "$aapt2" dump badging "$apk" 2>/dev/null | sed -n "s/^package: name='\\([A-Za-z0-9._-]*\\)' versionCode='\\([0-9]*\\)'.*/\\1 \\2/p" | head -n 1)" || raw_metadata=""
    if [[ "$raw_metadata" =~ ^[A-Za-z0-9._-]+\ [0-9]+$ ]]; then
      apk_metadata="$raw_metadata"
    fi
  fi
  printf 'diagnostic_apk_package_version=%s\n' "$apk_metadata"

  local probe_result="probe_failed"
  if [[ $SECONDS -lt $deadline ]] && command -v timeout >/dev/null 2>&1; then
    local probe_output=""
    local probe_status=0
    probe_output="$(timeout 1 adb shell "run-as $package sh -c 'if test -e $state; then if test -s $state; then printf present_nonempty; else printf present_empty; fi; else printf absent; fi'" 2>/dev/null)" || probe_status=$?
    probe_output="${probe_output//$'\r'/}"
    if [[ $probe_status -eq 0 && "$probe_output" =~ ^(absent|present_empty|present_nonempty)$ ]]; then
      probe_result="$probe_output"
    fi
  fi
  printf 'diagnostic_state_probe=%s\n' "$probe_result"

  local pid=""
  local pid_status=0
  if [[ $SECONDS -lt $deadline ]] && command -v timeout >/dev/null 2>&1; then
    pid="$(timeout 1 adb shell pidof "$package" 2>/dev/null | tr -d '\r')" || pid_status=$?
    [[ "$pid" =~ ^[0-9]+$ ]] || pid=""
  else
    pid_status=124
  fi

  local log_status="failed"
  local raw_log=""
  local max_log_bytes=49152
  if [[ $SECONDS -lt $deadline ]] && command -v timeout >/dev/null 2>&1; then
    raw_log="$(mktemp)" || raw_log=""
    if [[ -n "$raw_log" ]]; then
      set +e
      timeout 1 adb logcat -d -v brief -t 4000 2>/dev/null | head -c "$max_log_bytes" > "$raw_log"
      local pipeline_status=("${PIPESTATUS[@]}")
      set -e
      local log_bytes
      log_bytes="$(wc -c < "$raw_log" | tr -d ' ')"
      if [[ "$log_bytes" -ge "$max_log_bytes" ]]; then
        log_status="clipped"
      elif [[ "${pipeline_status[0]:-1}" -eq 124 ]]; then
        log_status="timed_out"
      elif [[ "${pipeline_status[0]:-1}" -ne 0 ]]; then
        log_status="failed"
      else
        log_status="complete"
      fi
      printf 'diagnostic_log_capture=%s\n' "$log_status"
      if [[ -n "$pid" && "$log_status" != failed && "$log_status" != timed_out ]]; then
        local marker_lines
        marker_lines="$(awk -v pid="$pid" 'match($0, "\\([[:space:]]*" pid "\\)") && index($0, "MOSAIC_NATIVE_DIAG v=1 ") { print }' "$raw_log" \
          | grep -Eo 'MOSAIC_NATIVE_DIAG v=1 stage=[a-z_]+ result=[a-z_]+ elapsed_us=[0-9]+' \
          | head -n 24 | cut -c1-240 || true)"
        if [[ -n "$marker_lines" ]]; then
          printf '%s\n' "$marker_lines"
        else
          printf 'diagnostic_host_markers=none_observed_for_current_pid\n'
        fi
      else
        printf 'diagnostic_host_markers=unavailable_pid_or_log\n'
      fi
      rm -f -- "$raw_log"
    else
      printf 'diagnostic_log_capture=temporary_file_failed\n'
    fi
  else
    printf 'diagnostic_log_capture=deadline_or_timeout_tool_unavailable\n'
  fi
  [[ -n "$pid" ]] || printf 'diagnostic_pid_probe=%s\n' "$([[ $pid_status -eq 124 ]] && echo timed_out || echo unavailable)"
}

# `adb shell` runs its arguments as ONE command line on the device; every
# value spliced in here was validated above.
in_app() {
  adb shell "run-as $package sh -c '$1'"
}

# This launch's log, read whole. A failed read must fail the gate: an empty
# log would read as "no crash" and "nothing refused". Captured rather than
# piped into `grep -q`, which can close the pipe early and, under pipefail,
# turn a match into a failure.
launch_log() {
  local log
  log="$(adb logcat -d -v brief)" || fail "could not read $package's logcat"
  printf '%s\n' "$log"
}

launch() {
  adb logcat -c
  adb shell am start -W -n "$activity" > /dev/null
}

# Still running `seconds` later, and it never threw: an uncaught exception
# kills the process, but a caught-and-shown startup failure would not, which
# is why every launch also checks the state file.
expect_running() {
  sleep "$1"
  if ! adb shell pidof "$package" > /dev/null; then
    fail "$package is not running $1 seconds after launch"
  fi
  local log
  log="$(launch_log)"
  if grep -F "FATAL EXCEPTION" <<< "$log" > /dev/null; then
    fail "$package threw an uncaught exception"
  fi
}

# Wait up to 30 seconds for a file test on the device to pass: the host reads
# state as it loads and persists after the environment report, which follows
# the first layout -- a few seconds on a software-rendered emulator.
eventually() {
  local check="$1"
  shift
  for _ in $(seq 1 30); do
    if in_app "$check"; then
      return 0
    fi
    sleep 1
  done
  fail "$*"
}

expect_state_written() {
  for _ in $(seq 1 30); do
    if in_app "test -s $state"; then
      return 0
    fi
    sleep 1
  done
  diagnostic_state_failure || true
  fail "no nonempty state check succeeded for $state in $package's storage before the existing deadline"
}

stop_app() {
  adb shell am force-stop "$package"
}

adb install -r -t "$apk" > /dev/null

echo "launch 1: a fresh install writes its state under filesDir"
in_app "rm -rf files/$application_id"
launch
expect_state_written
expect_running 10

echo "launch 2: the same state is restored, nothing quarantined or refused"
stop_app
launch
expect_running 10
if in_app "test -e $corrupt"; then
  fail "the state launch 1 wrote was quarantined on launch 2"
fi
log="$(launch_log)"
if grep -E "rejected persisted state|Ignored invalid Mosaic state" <<< "$log" > /dev/null; then
  fail "the host refused the state launch 1 wrote"
fi
in_app "test -s $state" || fail "launch 2 left no state behind"

echo "launch 3: state the runtime refuses is quarantined, and the app still runs"
stop_app
in_app "rm -f $corrupt && printf {} > $state"
launch
# The quarantine first: until it happens, the seeded `{}` would pass for
# written state.
eventually "test -e $corrupt" "the refused state was not moved to $corrupt"
if [[ "$(in_app "cat $corrupt" | tr -d '\r')" != "{}" ]]; then
  fail "$corrupt does not hold the refused state"
fi
expect_state_written
# Written afresh, not the seed left in place beside a copy.
if [[ "$(in_app "cat $state" | tr -d '\r')" == "{}" ]]; then
  fail "launch 3 left the refused state in place"
fi
expect_running 10

stop_app
echo "$package launched, restored its state and quarantined refused state on the emulator"
