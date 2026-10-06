---
category: CI & GitHub Actions
---

# avdmanager and the Android emulator can disagree on where virtual devices live

The first CI run of UI89's emulator gate (PR #16275) created the AVD with
`avdmanager create avd` (exit 0, output discarded) and then
`emulator -avd mosaic-gate` failed with "Unknown AVD name [mosaic-gate] ...
there is no file mosaic-gate.ini in $HOME/.android/avd"; the script then
waited two full minutes for adb before failing. The emulator searches only
$ANDROID_AVD_HOME, $ANDROID_SDK_HOME/avd and $HOME/.android/avd, while
avdmanager can write elsewhere (e.g. under $XDG_CONFIG_HOME, which GitHub's
runners set).

Fix (code/scripts/start-mosaic-android-emulator.sh): export ANDROID_AVD_HOME
for both tools; show avdmanager's output; require `emulator -list-avds` to
list the device before booting (adopting the directory avdmanager actually
wrote to, found under $HOME, if it ignored the variable); stop waiting as soon
as the emulator process exits.

Do differently: when one tool creates what another consumes, name the shared
location explicitly and have the consumer confirm it sees the result before a
long wait -- and never send the creating tool's output to /dev/null.
