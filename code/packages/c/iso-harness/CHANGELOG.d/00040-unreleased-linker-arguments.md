# Unreleased — optional linker arguments

### Changed

- Add `ISO_LINK_ARGS` to the POSIX and PowerShell ISO harness entry points.
  POSIX appends these tokens after source inputs; PowerShell appends them after
  `/link`. With no value, existing package commands keep their original link
  behavior.
