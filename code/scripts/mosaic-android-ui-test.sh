#!/usr/bin/env bash
# Run a Mosaic app's instrumented UI test on an emulator, across two cold
# launches (UI89 §4.2).
#
#   mosaic-android-ui-test.sh <apk> <test-apk> <android-package> <test-class>
#
# e.g. for Journal:
#
#   mosaic-android-ui-test.sh app-debug.apk app-debug-androidTest.apk \
#     dev.codingadventures.journalapp \
#     dev.codingadventures.journalapp.uitest.JournalAndroidUiTest
#
# The emulator must already be booted and be the only device adb sees
# (start-mosaic-android-emulator.sh). The test drives the app's real
# MosaicActivity, so its state lands where users' state lands:
# filesDir/<application id>/mosaic-state.v1.json.
#
# Why not Gradle's connectedDebugAndroidTest: it uninstalls the app when the
# run ends, which erases the state the second launch exists to read back. So
# both APKs are installed here and run with `am instrument`, once per launch:
#
#   launch | before                     | the test is told
#   -------+----------------------------+---------------------------
#   1      | `pm clear`: no state       | -e mosaicLaunch 1
#   2      | what launch 1 left behind  | -e mosaicLaunch 2
#
# Each `am instrument` starts a new app process, so launch 2 reads the
# snapshot from storage, not from the engine's memory.
#
# `am instrument` exits 0 even when the test fails. So its raw output is
# read: the run must end with INSTRUMENTATION_CODE: -1 (finished), report
# `OK (1 test)`, and say neither FAILURES!!! nor "Process crashed". A test
# that was filtered out or never compiled in reports `OK (0 tests)`, which
# fails here rather than passing.
set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "usage: $0 <apk> <test-apk> <android-package> <test-class>" >&2
  exit 2
fi
apk="$1"
test_apk="$2"
package="$3"
test_class="$4"
for file in "$apk" "$test_apk"; do
  if [[ ! -f "$file" ]]; then
    echo "no APK at $file" >&2
    exit 2
  fi
done
# The package and the class reach `adb shell`, which joins its arguments
# into ONE command line on the device, so quoting here does not protect
# them. Refuse anything but the plain names they are: no `$` (a nested
# class, which the device shell would expand), no `#` (a method filter,
# which it would treat as a comment) and no whitespace.
if [[ ! "$package" =~ ^[A-Za-z][A-Za-z0-9_]*(\.[A-Za-z][A-Za-z0-9_]*)+$ ]]; then
  echo "invalid Android package name: $package" >&2
  exit 2
fi
if [[ ! "$test_class" =~ ^[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z_][A-Za-z0-9_]*)+$ ]]; then
  echo "invalid test class: $test_class" >&2
  exit 2
fi

runner="$package.test/androidx.test.runner.AndroidJUnitRunner"

# The APK paths go only to host-side `adb install`, as separate arguments.
# -t: debug builds carry testOnly; -r: replace what the gate installed.
adb install -r -t "$apk" > /dev/null
adb install -r -t "$test_apk" > /dev/null
# Launch 1 starts empty whatever the gate left in the app's storage.
adb shell pm clear "$package" > /dev/null

run_launch() {
  # `launch` is the literal 1 or 2 from the calls below, never input.
  local launch="$1"
  local output
  echo "==> $test_class, launch $launch"
  adb logcat -c || true
  # Captured whole rather than piped into grep, which could close the pipe
  # early and, under pipefail, turn a match into a failure.
  output="$(adb shell am instrument -w -r \
    -e class "$test_class" -e mosaicLaunch "$launch" "$runner" 2>&1)" || true
  printf '%s\n' "$output" | grep -E '^(INSTRUMENTATION_STATUS: (test|stack)=|INSTRUMENTATION_CODE|OK |Tests run|FAILURES)' || true
  if [[ "$output" == *"FAILURES!!!"* ]] \
    || [[ "$output" == *"Process crashed"* ]] \
    || [[ "$output" != *"INSTRUMENTATION_CODE: -1"* ]] \
    || [[ "$output" != *"OK (1 test)"* ]]; then
    echo "::error::$test_class failed on launch $launch"
    echo "---- am instrument ----"
    printf '%s\n' "$output" | tail -n 120
    echo "---- logcat (this launch) ----"
    adb logcat -d -v brief 2>/dev/null | tail -n 200 || true
    exit 1
  fi
}

run_launch 1
run_launch 2
echo "$test_class passed across two cold launches"
