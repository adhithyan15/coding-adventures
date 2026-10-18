- `scripts/build-all.sh` always runs `flutter create` before
  `flutter build`. It used to skip it whenever `macos/` existed. Since
  #16588 the generated project ships `macos/Runner/*.entitlements`, so the
  directory always existed. The runner then had no Xcode project, and
  `flutter build macos` failed with "No macOS desktop project configured".
  `flutter create` only adds missing files, so it keeps those entitlements.

