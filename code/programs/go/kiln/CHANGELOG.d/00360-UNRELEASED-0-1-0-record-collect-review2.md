## [0.1.0] - UNRELEASED

### Fixed (second review of `kiln record collect`)

- `kiln record step` refuses when it cannot tell whether `traces/.collected` exists
  (any answer but "not there"), instead of running.
- `--results` needs a canonical parent directory, like `--out`: a symlinked parent
  that leads into the checkout is refused.
- `writeAtomic` takes the link back when the directory sync fails, so a failed
  `--out` or `record.json` write leaves nothing (the epilogue then leaves
  `record.failed`). A failure after the tombstone burns the record directory
  (documented: a missing shard is run-void in the merge).
- `legacy.PolicyDecode` skips the per-element walk of lists whose elements cannot
  hold keys (strings, numbers), so a record of tens of thousands of paths is not
  copied element by element; duplicate and case-variant keys are still refused.
- `scanAction` keeps at most 64 reasons per directory and counts the rest.
- Test fixture: a borrowed prepared world is checked after reset (the checkout is
  the commit, the untracked paths are the sealed inventory, the record directory
  holds exactly `pre/` and `traces/`) and fails the test loudly if not.
