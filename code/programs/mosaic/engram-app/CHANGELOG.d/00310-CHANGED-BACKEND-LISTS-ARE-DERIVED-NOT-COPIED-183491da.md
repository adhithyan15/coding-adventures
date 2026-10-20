### Changed — backend lists are derived, not copied

- Tests read the backend list from `mosaic_package_artifact_builder::Backend` (`Backend::NATIVE` / `Backend::ALL`) instead of a hand-kept copy, so a backend added to Mosaic is gated here automatically. `scripts/build-all.ps1` and `scripts/build-native.sh` ask `mosaic-compile backends` for their matrix instead of keeping their own list.
