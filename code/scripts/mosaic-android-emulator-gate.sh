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
  eventually "test -s $state" \
    "no $state in $package's storage: the engine never answered an event, or the host persisted elsewhere"
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
