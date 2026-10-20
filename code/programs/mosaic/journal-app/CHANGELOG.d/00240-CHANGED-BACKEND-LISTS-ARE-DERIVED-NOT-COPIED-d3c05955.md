- **Backend lists are derived, not copied.** Tests read the backend list from
  `mosaic_package_artifact_builder::Backend` (`Backend::NATIVE` /
  `Backend::ALL`) instead of a hand-kept copy, so a backend added to Mosaic is
  gated here automatically.
