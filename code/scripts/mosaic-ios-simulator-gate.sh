#!/usr/bin/env bash
# Launch a Mosaic iOS app on a simulator and prove it keeps its state
# (UI89 §2.4). The iOS shape of mosaic-android-emulator-gate.sh.
#
#   mosaic-ios-simulator-gate.sh <app> <bundle-identifier> <mosaic-application-id> <simulator-udid>
#
# e.g. for Journal:
#
#   mosaic-ios-simulator-gate.sh Debug-iphonesimulator/App.app \
#     dev.codingadventures.journalapp journal-app "$udid"
#
# The simulator must already be booted (`xcrun simctl boot` + `bootstatus -b`).
#
# Why no UI automation: the SwiftUI shell reports the window's environment
# (UI48 ENV4) once it lays out, that report is an event, and the host persists
# after every event. So a state file appearing in the app's data container
# proves, in one observation, that the app started, the statically linked
# engine answered an event, and the host persisted where iOS keeps it
# (`Library/Application Support/<application id>/mosaic-state.v1.json`). Then
# three launches, each of which must still be running ten seconds later:
#
#   launch   | state before           | must see afterwards
#   ---------+------------------------+-------------------------------------
#   1 fresh  | none                   | state written
#   2 again  | what launch 1 wrote    | nothing quarantined, state still there
#   3 seeded | `{}` (runtime rejects) | `.corrupt` holding `{}`, fresh state
#
# The data container is a directory on the Mac, so unlike Android's `run-as`
# every check here reads and writes it directly.
set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "usage: $0 <app> <bundle-identifier> <mosaic-application-id> <simulator-udid>" >&2
  exit 2
fi
app="$1"
bundle="$2"
application_id="$3"
simulator="$4"
if [[ ! -d "$app" ]]; then
  echo "no .app bundle at $app" >&2
  exit 2
fi
# Each value reaches a command line or a path below, so refuse anything that
# is not the plain identifier it is.
if [[ ! "$bundle" =~ ^[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)+$ ]]; then
  echo "invalid bundle identifier: $bundle" >&2
  exit 2
fi
if [[ ! "$application_id" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]]; then
  echo "invalid Mosaic application id: $application_id" >&2
  exit 2
fi
if [[ ! "$simulator" =~ ^[0-9A-Fa-f-]+$ ]]; then
  echo "invalid simulator udid: $simulator" >&2
  exit 2
fi

fail() {
  echo "::error::$*"
  exit 1
}

# Each launch's stderr, where the host reports state it refused (as
# "rejected persisted state" or "Ignored invalid Mosaic state").
logs="$(mktemp -d)"
trap 'rm -rf -- "$logs"' EXIT

# A cold start every time: `--terminate-running-process` ends any instance a
# failed terminate left behind, which would otherwise just be brought to the
# front -- and a launch 2 that never reloaded state would pass.
launch() {
  xcrun simctl launch --terminate-running-process --stderr="$logs/$1.err" \
    "$simulator" "$bundle" > /dev/null
}

# Whether this launch's host refused the state it found.
refused_state() {
  grep -E "rejected persisted state|Ignored invalid Mosaic state" "$logs/$1.err" > /dev/null 2>&1
}

stop_app() {
  xcrun simctl terminate "$simulator" "$bundle" > /dev/null 2>&1 || true
}

# Still running `seconds` later: launchctl lists the app's own job (its label
# is the bundle id, or UIKit's `UIKitApplication:<bundle id>[...]`) with a
# process id, not `-`. Matched exactly, so a job whose label merely contains
# the bundle id (an extension's) does not count. launchctl's whole output is
# captured first: `grep -q` on a pipe can close it early and fail simctl with
# SIGPIPE.
expect_running() {
  sleep "$1"
  local services
  services="$(xcrun simctl spawn "$simulator" launchctl list)" || fail "could not list the simulator's services"
  if ! awk -v bundle="$bundle" '
      $1 ~ /^[0-9]+$/ && ($3 == bundle || index($3, "UIKitApplication:" bundle "[") == 1) { found = 1 }
      END { exit found ? 0 : 1 }
    ' <<< "$services"; then
    fail "$bundle is not running $1 seconds after launch"
  fi
}

# Wait up to 30 seconds for a test on a file to pass: the host persists after
# the environment report, which follows the first layout.
eventually() {
  local description="$1"
  shift
  for _ in $(seq 1 30); do
    if "$@"; then
      return 0
    fi
    sleep 1
  done
  fail "$description"
}

xcrun simctl install "$simulator" "$app"
container="$(xcrun simctl get_app_container "$simulator" "$bundle" data)"
if [[ -z "$container" || ! -d "$container" ]]; then
  fail "no data container for $bundle"
fi
state_dir="$container/Library/Application Support/$application_id"
state="$state_dir/mosaic-state.v1.json"
corrupt="$state.corrupt"

echo "launch 1: a fresh install writes its state in its container"
stop_app
rm -rf -- "$state_dir"
launch 1
eventually "no $state: the engine never answered an event, or the host persisted elsewhere" test -s "$state"
expect_running 10

echo "launch 2: the same state is restored, nothing quarantined"
stop_app
launch 2
expect_running 10
if [[ -e "$corrupt" ]]; then
  fail "the state launch 1 wrote was quarantined on launch 2"
fi
# The quarantine's move can fail silently; the host's report cannot.
if refused_state 2; then
  fail "the host refused the state launch 1 wrote: $(cat -- "$logs/2.err")"
fi
[[ -s "$state" ]] || fail "launch 2 left no state behind"

echo "launch 3: state the runtime refuses is quarantined, and the app still runs"
stop_app
rm -f -- "$corrupt"
[[ ! -L "$state" ]] || fail "$state is a symbolic link"
printf '{}' > "$state"
launch 3
# The quarantine first: until it happens, the seeded `{}` would pass for
# written state.
eventually "the refused state was not moved to $corrupt" test -e "$corrupt"
# The report follows the move, so it may land a moment later.
eventually "the host did not report the state it refused" refused_state 3
[[ "$(cat -- "$corrupt")" == "{}" ]] || fail "$corrupt does not hold the refused state"
eventually "launch 3 wrote no fresh state" test -s "$state"
# Written afresh, not the seed left in place beside a copy.
[[ "$(cat -- "$state")" != "{}" ]] || fail "launch 3 left the refused state in place"
expect_running 10

stop_app
echo "$bundle launched, restored its state and quarantined refused state on the simulator"
