## [0.1.0] - UNRELEASED

### Added

- `internal/record` (KILN04 stage 4a, the part of the shard recorder that needs
  no sandbox):
  - `VerifyTree(root)` (G5): proves the working tree is exactly `HEAD`.
    `git ls-tree -r -z HEAD` must equal `git ls-files --stage -z` (path, mode,
    id, stage 0 only); `git ls-files -v` must show no skip-worktree or
    assume-unchanged bit; `core.sparseCheckout` must not be true. Every entry
    is then `lstat`ed: symlink targets hash to their blob, regular files are
    opened `O_NOFOLLOW|O_NONBLOCK|O_NOCTTY` and hashed as Git blobs (SHA-1 or
    SHA-256 by id length) with the exec bit compared, ancestor directories must
    be real directories, gitlinks are returned, anything else is `non-regular`.
    1 GiB per file, 20 minutes, 8 workers, at most 16 paths in an error. A FIFO
    in place of a file cannot hang it.
  - `Inventory(root)`: the untracked inventory in the `recfmt.Inventory`
    format. A directory with no tracked file collapses to one entry, the root
    `.git` is skipped, symlinks are never followed, order is byte order, and a
    name the formats refuse fails the call.
- `legacy.DecodeSteps`, `legacy.ReadStepsFile`, `legacy.Steps` (G6): the strict
  decoder of the build tool's `-emit-kiln-steps` file (`kiln-legacy-steps/1`).
  Refuses unknown, duplicate and case-variant keys, trailing data, null lists,
  an unknown `os`, unsorted or repeated packages, bad paths and NUL in a line.
- `legacy.RecordTmpdir` (G13): `/tmp/kiln-tmp`, the `TMPDIR` keys are derived
  under.
- `source.Git.Output`: one hardened, shell-free git invocation for the
  plumbing `internal/record` needs.
- KILN04 section 4.8 "Recording decisions (stage 4a)": G5, G6 and G13, and the
  section 4.6 amendment for `-emit-kiln-steps`. G7-G9 are pending the sandbox
  PR.

### Notes

- `HostScan`, sandbox configuration, the supervisor, `collect` and the
  `kiln record` commands are not here: they need the sandbox (#19291).
- On Windows the exec-bit comparison and `O_NOFOLLOW` do not exist; the check
  compares bytes and symlinks only. The recorder itself is Linux-only.
