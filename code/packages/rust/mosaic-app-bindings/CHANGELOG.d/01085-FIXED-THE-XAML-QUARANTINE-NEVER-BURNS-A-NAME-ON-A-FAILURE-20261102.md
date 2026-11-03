### Fixed — the XAML quarantine never burns a name on a failure

The emitted XAML host could switch saving off and report "All 10 quarantine
names beside it are taken" when nothing at all was at those names, so every
later edit in that session was lost:

- **An inspection error counted as taken.** `QuarantineNameTaken` caught every
  exception and answered `true`. The fault behind one such error usually
  affects all ten names, so the host walked every one and then blamed ten
  files that did not exist. It now reports the error and the name it hit
  (`Could not inspect <name>: ...`) and stops there; saving is still switched
  off, because the rejected file is still at the live path.
- **A half-finished move could leave a hard link behind.** On Linux and macOS
  .NET implements `File.Move(..., overwrite: false)` as link(2) then unlink(2).
  When the unlink failed, the quarantine name stayed behind as a second name
  for the live file, read as taken, and the next name got another link. The
  host now does what the Compose, SwiftUI, Flutter and Qt hosts do: it claims
  the name with `FileMode.CreateNew` (`O_CREAT | O_EXCL`, Windows
  `CREATE_NEW`), then moves onto that empty placeholder with
  `File.Move(..., overwrite: true)` (rename(2), or `MoveFileEx` with
  `MOVEFILE_REPLACE_EXISTING`). If that move fails, the placeholder is removed
  when it is still an empty regular file, so a failure never uses a name up.
  This also retires the check-then-rename fallback .NET used on file systems
  without hard links, which could replace a quarantine another instance made
  in between.

Tests:

- `tests/xaml_effect_driver` gains two cases, run by
  `tests/xaml_effect_completion.rs`. `quarantine-inspection-error` calls the
  move with a path .NET refuses to inspect (it holds a NUL) and fails on the
  old host, which answered "All 10 ... are taken". `quarantine-uninspectable`
  runs a whole launch whose quarantine names are too long for the file system
  and checks that the warning names `.corrupt`, stops there, and that nothing
  is written over the rejected file.
- `the_xaml_quarantine_never_burns_a_name_on_a_failure` pins the shape: no
  `return true` in the inspection, both looks return the error, the claim
  comes before the move, and a failed move removes only an empty placeholder.
- `xaml_effect_completion.rs` now finds the conformance runtime under
  `CARGO_TARGET_DIR` when that is set, instead of always under the
  workspace's `target/`.
