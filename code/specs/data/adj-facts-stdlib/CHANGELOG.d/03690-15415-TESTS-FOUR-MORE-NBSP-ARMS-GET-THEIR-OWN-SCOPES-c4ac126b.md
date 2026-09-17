- **#15415: four more e2e tests get scoped U+00A0 arms.** `facts_cloudsignal_e2e.rs`,
  `facts_cloudtype_e2e.rs`, `facts_longboneparts_e2e.rs` and `facts_musclegroups_e2e.rs` each asserted
  a page-absent ordinary-space constant against `shipped_table()`, which slices from `table …` to end
  of file. Each now asserts it against `source` lines at any indent. Test-only — no `.adj` change.

  These four were not found by #15417's census or the identifier census before it. Each builds the
  ordinary-space form once, compares it to a named constant (`STRATUS_PLAIN`, `PERIOSTEUM_BEFORE`,
  `PECT_PLAIN`), and asserts the constant — so the arm itself contains no `.replace`.

  ### Before the fix, at `7d6819c879`

      file             arm   % comments in block   ordinary-space form planted in a comment
      cloudsignal      122            6             KILL, only at 122
      cloudtype        119            4             KILL, only at 119
      longboneparts    146            4             KILL, only at 146
      musclegroups     202            4             KILL, only at 202

  Each plant was the target row's shipped sentence with U+00A0 replaced by a space. Before any mutant
  ran, the harness checked that the planted text contains the test's constant and that the constant
  was absent from the block.

  ### After the fix

      mutant                   cloudsignal (135)  cloudtype (132)  longboneparts (158)  musclegroups (216)
      plant in a % comment     SURVIVE            SURVIVE          SURVIVE              SURVIVE
      plant on another row     KILL, arm fired    KILL, arm fired  KILL, arm fired      KILL, arm fired
      plant on its own row     KILL, unreached    KILL, unreached  KILL, unreached      KILL, unreached

  Within the test under study, the own-row mutants stop at `cloudtype:117`, `longboneparts:143` and
  `musclegroups:200` — each the check that the answer carries the row's own citation. `cloudsignal`
  passes that check at 120 and stops at 121, the check that the ordinary-space string reaches no
  answer. `cloud-signal.adj` ships the stratus sentence on three rows (lines 82, 86 and 90), and the
  mutant rewrote one of them.

  The scoped filters read 6, 4, 6 and 10 `source` lines, each table contributing one four-space line.

  `PECT_PLAIN` is a fragment — `from Latin pectus 'breast'` — so the scoped arm in `musclegroups` still
  covers every `source` line in the table, not only the pectoralis row.

  ### The trade

  As in 03560, 03670 and 03680: an ordinary-space form in a `%` comment now survives. Nothing pins "not
  anywhere in the block", only "no `source` line carries it".

  ### A harness defect its own input check stopped

  The first run required exactly one row carrying the target sentence, and aborted on `cloud-signal`,
  which has three. Its "another row" selector was `line != target`; on three identical lines that
  selects a second stratus row, and a first-occurrence replace would then have written onto a stratus
  row. The selector now excludes every row carrying the target sentence, and the run prints the row it
  hits: Cirrus, Cirrus, Diaphysis, Biceps.

  Gates: after `cargo clean -p adj-lang-cli`, `cargo test` under `RUSTFLAGS="-Dwarnings"` → 6, 6, 4 and
  8 passed, 0 failed. `cargo clippy --all-targets -- -D warnings` → exit 0, with one
  `Checking adj-lang-cli` line. Every mutant was written to the tracked `.adj` inside a try/finally and
  restored byte-identical by SHA-256.
