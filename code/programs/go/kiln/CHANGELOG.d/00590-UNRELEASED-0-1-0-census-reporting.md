## [0.1.0] - UNRELEASED

### Added (census reporting: triage of the first real census)

Census side channel and `kiln record census` summary only. No recording byte
changes: `record.json`, `records.jsonl`, the binding and everything sealed are
byte-identical, and `TestCensusDoesNotChangeTheRecord` pins that for every site
the census observer now watches (the observer is write-only).

- `refused:<syscall>`: parse reasons `refused:<name>` are reported with the name
  when it is a member of the parser's closed refused class
  (`trace.SyscallClass`), else `refused:other`, in both "parse reasons in
  record.json" and "parse refusals seen by this census".
- Stricter-only rows by name: `trace.ParseCrossCheckedDetail` returns the fixed
  row name and line of each allowed divergence (`ParseCrossChecked` is a wrapper
  with its old signature). The summary has `cross_check.allowed_rows` and one
  detail per row instead of "N allowed stricter-only divergence(s)".
- `trace.Shape`: a redacted token outline of the line the grammar stopped at (and
  the pid's pending `<unfinished ...>` half): token type names, character classes
  for words, closed-table syscall names, ASCII punctuation, the `lineFacts`
  booleans; never a byte of QUOTED or ANNOTATED text. At most 96 tokens a line, 8
  distinct outlines and 64 computations per census (`shapes`,
  `bounds.shapes_dropped`). Hostile-input tests put secrets in every token kind;
  the AST guard keeps `Shape`, `LineShape` and `ParseCrossCheckedDetail` out of
  every production file but the census.
- Sub-codes for `trace-incomplete:unstable-path` (`component-changed`,
  `absent-changed`, `final-changed`, `followed-final-changed`,
  `followed-absent-changed`, `gone-but-found`, `removed-before-made`,
  `file-changed`, `own-overlap`, `through-missing`, `dotdot`, `fd-path-differs`)
  and `trace-incomplete:exec-unreadable` (`modified-after-exec`, `virtual-exe`,
  `virtual-interpreter`, `under-tmp`, `in-repo`, `host-disk`), each with the
  redacted path, as census kinds `trace-incomplete:<reason>:<sub>`. The record's
  reason string is unchanged.
- host-read op classes: `census.json` gains an additive `ops` map and each
  `reasons[]` kind of the summary an `ops` breakdown (listing, absent-probe,
  open, stat, lookup, exec, chdir, write, other).
- Per language `built_ok`, `emittable`, `gated`, `unkeyed`, `writes_unknown`,
  `recordable_share_of_built_ok`, `emittable_share` and
  `emittable_share_of_built_ok`; per summary `actions.built_ok`,
  `actions.emittable`, `actions.gated`, `actions.unkeyed`,
  `actions.writes_unknown`, `actions.shard_void` and
  `actions.strict_merge_voids_shard` (Markdown: "strict merge would void this
  shard: yes/no"). `emittable` is an upper bound (per action, pre-merge) and
  requires a plan entry with no gate and a weak key.
- Shape budgets: `allowed:*` outlines have their own budget (8 slots, 32
  computations); refusals 8 slots, 32 computations, 2 per code; a (trace, line)
  is shaped once; 4 GiB scanned in all.
- Census per-kind caps fit the overall bound (sub-coded kinds hold 256 paths),
  and the stability pass's notes are given to the census sorted, so
  `census.json` is deterministic at a cap.

### Changed

- The invoker's home and temporary directory are labelled `$RUNNER_HOME` and
  `$RUNNER_TMP` unless they are the directories the sealed host facts name, so
  `$HOME` and `$TMPDIR` mean only the step's.
- The Markdown "Recordable share by language" section is "Share by language": it
  says "recordable (parse ok and writes complete, NOT necessarily emittable)",
  "no-reason" (was "clean": an empty census beside a recordable record) and
  "would emit" (zero reasons, not gated). JSON field names are unchanged and the
  schema stays `kiln-census-summary/1` (all additions are additive).
- SEMANTIC CHANGE to `census_clean`: a record that raised a read-side
  `unstable-path` or `exec-unreadable` now has a non-empty `census.json`, so the
  action counts as `census_nonempty` instead of `census_clean` (it was "clean"
  before, which was misleading). The JSON stays additive,
  `kiln-census-summary/1`; `ci/census_not_clean.py` is unaffected (tested).
- KILN04's census section documents every new field and label.
