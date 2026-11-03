## [Unreleased]

### Fixed

- `BUILD_windows` stages the extension with the new
  `scripts/check_staged_extension.py` (the shape `window-native` uses) instead
  of sh one-liners (`$(...)`, `for ...; do`, `[ -f ... ]`) that `cmd /C`
  cannot run. The script stages into a fresh `mkdtemp` directory, runs ruff
  (now over `scripts` too) and pytest against it, and removes it.

