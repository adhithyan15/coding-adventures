## [0.1.0] - UNRELEASED

### Added

- `key.LegacyRecipeFiles` encodes a legacy recipe from an ordered list of named
  files (KILN02 3.4 generalised); it refuses an empty list, empty or non-UTF-8
  names, names containing `/` or `\`, and duplicate names. `LegacyRecipe` is now
  a thin wrapper, byte-identical to before.
- Golden vectors `legacy-build-mac-and-linux`,
  `legacy-semantics0-consumed-digests` and `legacy-same-text-different-name`.
