# `mv file.bak file` restores OLD mtime → cargo skips the rebuild (false "race")

While building WEB01b-1b I did a "does this test actually prove anything" check:
`sed -i.bak 's/ordered_responses: true/false/' lib.rs` (build+run → correctly
FAILED), then `mv lib.rs.bak lib.rs` to restore. The restored file had
`ordered_responses: true` again — but every subsequent `cargo test` kept FAILING
as if it were still `false`. Adding ANY `eprintln!` "fixed" it, which screamed
"timing race." It was not a race.

`mv` preserves the SOURCE file's mtime. The `.bak` was created at the moment of
the `sed` (before the false-build), so restoring it stamped `lib.rs` with an
mtime OLDER than the compiled artifact from the false build. Cargo's
mtime-based staleness check then judged `lib.rs` "older than the build" and
skipped recompiling — so the tests ran against the stale `ordered_responses:
false` binary. Adding an `eprintln` edited the file (fresh mtime) and forced a
real rebuild, which is why it "passed."

Lessons:
- To revert a quick experiment, restore from git (`git checkout -- file`) or
  `touch file` after a `cp`/`mv` — never trust `mv file.bak file` to trigger a
  rebuild; it can move the mtime backwards.
- A Heisenbug that disappears the instant you add a print, where the print is
  AFTER the observed effect, is almost never a real race — suspect a stale build
  artifact (or caching) first. Confirm by `touch`-ing the source and re-running
  clean BEFORE hunting for a concurrency bug.
