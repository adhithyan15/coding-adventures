## [0.1.0] - UNRELEASED

### Added

- `internal/traced` (KILN04 stage 1b-i): `Classify` turns the parsed traces of
  one action (one per BUILD step) into a `RawRecord`: the manifest entry names
  (`pkg:`, `path:`, `list:`), the write roots and the tracked paths written, the
  in-repository untracked paths observed, the traversed components, and the
  section 6 reasons (at most 16 per kind). It is pure; the tracked tree, the
  post-run disk, the package index, `host-roots.json`, the weak key's tools, the
  pinned ecosystems and the scrubbed `HOME`/`TMPDIR` are all injected.
  - Resolution walks each path component by component (40 hops, `..` on the
    resolved directory, every crossed link an observation), prefers the `-y`
    return path for successful opens and flags a disagreement, and applies the
    stability rule using what the trace itself knows (links, directories and
    removals the action made) before the post-run disk.
  - Stability is strict (KILN04 5.1): a traversed component is stable only if it is tracked and untouched, or this action created it once before the lookup began and nothing touched it or an ancestor since; outside the repository only untouched components are stable; HOME and TMPDIR are never read from the disk. Creation is an absence observation; the record also lists `AbsentObservations`. `Recipe` is required. A differential fuzzer (an in-memory filesystem model as oracle) checks every path the model really resolved is covered, own, or explained by a reason.
  - Exec is a read of the program plus its `#!` chain (4 hops) and the ELF
    `PT_INTERP`, read from the tracked tree or the post-run disk.
  - `/proc` follows the ephemeral list (`trace-visible` for
    `/proc/self/status` and `stat`, `trace-incomplete:proc-path` otherwise);
    out-of-repository reads follow `image` (ctime against the job-start stamp),
    `tool` (keyed tools only), `state` (always `host-read` until stage 5),
    `ephemeral` and `absent-ok`; reasons redact `$HOME` and `TMPDIR`.
  - Network is judged by address: `network:`, `daemon:`, loopback and Unix
    sockets the action bound, `ECONNREFUSED`/`ENOENT` allowed, other families
    refused.
  - `CheckRecipe` compares each step's exec argv with the expected `sh -c`
    lines (`trace-incomplete:recipe-mismatch`).
  - `SourceTree` adapts a `source.Source` to the tracked-tree view;
    `PackageIndex` finds the deepest enclosing package.
  - Tests use hand-built traces and in-memory views, plus one trace parsed from
    strace text; `FuzzClassify` checks for no panic, sorted unique output and
    clean entries and reasons.
- `internal/trace`: `Event.Argv` (the decoded argument vector of an exec) and
  `Trace.StepEvent()` (the step's first successful exec), which the recipe check
  needs.
- KILN04 section 5.5 records the classification decisions.
