### Fixed — a rejected state file never replaces an earlier quarantine

- Every generated native host (Qt, SwiftUI, Compose, Flutter, XAML) kept one
  quarantine slot: on a rejected `mosaic-state.v1.json` it deleted any
  `<state>.corrupt` and moved the bad file there. A second rejection (the next
  launch, say) destroyed the first quarantine, which may be the user's real
  data, and a delete-then-move that failed halfway lost it too.
- A rejected file now moves to the first free name: `<state>.corrupt`, then
  `<state>.corrupt.1` to `<state>.corrupt.9`. A name is skipped when anything
  is there (file, directory or link), nothing is deleted first, and the move
  refuses to replace: `QFile::rename`, `FileManager.moveItem`, `Files.move`
  without `REPLACE_EXISTING`, and `File.Move(..., overwrite: false)`. `dart:io`
  has no such move (`renameSync` and `copySync` both replace), so Flutter
  claims the name with `createSync(exclusive: true)` and then moves onto that
  empty placeholder with the atomic replace a save already uses.
- If the move fails, or all ten names are taken, the rejected file is still at
  the live path. The host then sets a session flag that makes its save a no-op
  (including the delete-on-null-snapshot branch) and appends "This session
  will not be saved." to its persistence warning, which stays showing because
  no write clears it.
- The first quarantine on a fresh state directory is still exactly
  `<state>.corrupt`, so the release gates that compare against it are
  unchanged.
- Compose named the quarantine after the default file name, so an explicit
  `MOSAIC_APP_STATE_PATH` quarantined to `<dir>/mosaic-state.v1.json.corrupt`.
  It now uses `<state>.corrupt`, like the other four hosts.
- A text test pins each host's candidate loop, ten-name limit, skip-before-move
  order, no-replace move, flag and save guard. Each effect driver gains a
  `quarantine` case that runs the emitted host: an earlier `.corrupt` survives
  and the rejected file lands in `.corrupt.1`; with all ten names taken the
  rejected file is not written over after an event and the warning stays.
