---
category: BUILD files & dependency management
---

# Test for the file a tool creates, not for the directory it creates it in

Venture's `scripts/build-all.sh` ran `flutter create --platforms=macos .`
only `if [[ ! -d macos ]]`. Then #16588 made the package builder write
`macos/Runner/*.entitlements` into every generated Flutter project, so that
`flutter create` would keep them. The directory now always existed,
`flutter create` never ran, and `flutter build macos` failed with "No macOS
desktop project configured". Nothing caught it until an unrelated
changelog edit rebuilt Venture on macOS (#17430). The failing step printed
only that one line, ahead of the passing tests in the log.

Fix: run `flutter create` every time. It only adds the files a project
lacks. Engram's script already did this.

Do differently: when a script skips a generator, test for the file the
generator makes (here `macos/Runner.xcodeproj`), or run an idempotent
generator every time. And when you change what a generator writes, grep the
repo for scripts that test for the directories you now populate.
