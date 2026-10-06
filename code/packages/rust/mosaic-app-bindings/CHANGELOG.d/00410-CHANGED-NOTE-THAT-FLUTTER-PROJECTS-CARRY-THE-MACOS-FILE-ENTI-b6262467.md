### Changed — note that Flutter projects carry the macOS file entitlement

- The Flutter platform library's header comment now says generated projects
  carry the macOS file-dialog entitlement (the builder writes the runner's
  entitlements before `flutter create`).
