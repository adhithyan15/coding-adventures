- **#15338: `tests/facts_solareclipsetype_e2e.rs` — the apostrophe arms get their own scopes.**
  Both arms used to read the whole `table … { … }` block: `body.contains(ENVELOPE)` and
  `!body.contains(&ascii_variant)`. That is the **pre-correction** form. The same instrument was
  corrected for `plant-parts` in #15337 and recorded in shard 03560; this copy did not get the fix.
  Test-only — no `.adj` change, no shipped data change.

  ### Two arms, two claims, two scopes

  - **Positive arm** → `l.starts_with("    source \"")`, table-level indent only. Its claim is about
    *the envelope*. Read with `trim_start()` it would also see eight-space row sources, and would be
    satisfied by a mutant that deleted the envelope line and planted the string on a row — leaving its
    message claiming more than it checked.
  - **Negative arm** → `l.trim_start().starts_with("source \"")`, any indent. Its claim is about *any
    shipped citation*: a row carrying an ASCII-apostrophe envelope is just as wrong. Still `source`
    lines only, never the block, because a `%` comment quoting the sentence is not a shipped citation.

  ### Observed working, not merely observed silent

  Both arms passed before this change and after it, which proves nothing on its own — a filter seen
  only silent has not been seen working. `shipped_table()` reads a fixed path, so each mutant was
  **written to the tracked `.adj`** inside a `try`/`finally`, tested, and restored in the same call;
  the file was verified byte-identical afterwards and clean in git. A temp-directory copy cannot
  reach this test at all.

      mutant                                    want      got   kills at
      ASCII on the table-level envelope         KILL      KILL  positive arm
      envelope deleted, ASCII on a row source   KILL      KILL  positive arm
      ASCII inside a % comment only             SURVIVE   SURVIVE   —
      envelope intact, ASCII on a row source    KILL      KILL  NEGATIVE arm
      envelope deleted, U+2019 on a row source  KILL      KILL  positive arm

  The baseline passed first, so the run is interpretable. **Which arm fires matters.** Mutants one
  and two never reach the negative arm at all: Rust short-circuits, so a failing positive arm ends
  the test. Mutant three *does* reach it — it passes 6/6, and a passing test executes every
  statement.

  **Each scoping decision has its own isolating mutant**, and each was measured:

  - **m4** isolates the *negative* arm's `trim_start()`: its only ASCII plant sits at eight spaces
    with the envelope intact, so a four-space-only negative arm would spare it.
  - **m5** isolates the *positive* arm's four-space-only scoping: with the envelope deleted and the
    **U+2019** form planted on a row, the four-space filter does not see it and the arm kills, while
    a `trim_start()` filter would see it on the row and the arm would survive. m2 cannot do this job
    — it plants the *ASCII* form, so it kills through the positive arm under either scoping.
  - **m3** earns *"`source` lines, not the whole block"*: its ASCII envelope sits in the block but on
    **no `source` line**, so the old unscoped arm would have **killed** it and the new arm correctly
    spares it.

  **Not all mutants merely "fail the test".** The second one deletes the table-level `source`, which
  leaves `locator`/`trust` without provenance: the CLI rejects the file with `TableMissingProvenance`
  and **all six** tests fail, four of them on `cli should succeed` rather than on anything about
  apostrophes. It parses; it is refused at lowering. That does not weaken its kill — the apostrophe
  arm reads the file text directly and never consults the CLI.

  The fourth mutant, mutating the **total** row, fails **three** tests: the apostrophe arm plus two
  per-row citation tests. The shape test passes on it, but that does not make the negative arm the
  sole guard — two other assertions catch it as well. The count is a property of the row chosen, not
  of the mutant class: the same mutation on the *partial* row fails **two**, because one of those
  per-row tests only inspects the total row.

  **Neither arm is vacuous.** A negative arm over an empty filter result passes trivially, having
  observed nothing. Measured against the shipped file: the positive arm's filter matches **1** line
  (the table-level envelope), the negative arm's matches **4** (that envelope plus 3 row sources).

  ### The survivor is the trade, not a regression

  Scoping the negative arm lets a comment-borne ASCII envelope survive. Nothing now pins "no ASCII
  envelope anywhere in the table block", only "no `source` line carries one". Shard 03560 already
  records that trade for `plant-parts`; it is the same trade here, and it is the right one — the
  shipped string is what provenance means — but it is a gap, written down rather than scored away.

  ### Severity: the second defect was already backstopped, and that was measured

  The issue asked whether this file has the shape-test backstop `plant-parts` has before deciding
  severity. It does, quoted whole rather than elided:

      "\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust authoritative\n"

  The mutant run confirms it rather than asserting it: the shape test also failed on the first two
  killing mutants. So this change makes the arm honest about its own claim; it does not plug an open
  hole.

  **And the defect it backstops cannot ship anyway.** A table with no table-level `source` is refused
  at lowering, so the mutant the positive arm's scope guards against would never reach production.
  The backstop is real; the risk it manages is smaller than "backstopped" implies.

  ### Scope

  Searched `code/packages/rust/adj-lang-cli/tests/` (527 files) for `ascii_variant`: **three** carry
  it. `facts_plantparts_e2e.rs` and `facts_mixturetypes_e2e.rs` are already corrected — mixture-types
  is an nbsp (U+00A0) variant that gained the instrument in #15342, after this issue was filed, which
  is why the issue's scope note says two. `facts_solareclipsetype_e2e.rs` is the only uncorrected copy
  **among the three that carry this instrument by name** — the population was defined by grepping for
  the identifier `ascii_variant`, and mixture-types shows the hazard in reverse by using that name for
  a U+00A0 variant. The same defect under a different local name would be invisible to this census.

  That was classified **per function, not per file**. A file-level check for the corrected idiom
  reports this file as already corrected, because the idiom appears in a different test in it.

  Gates: `cargo test --test facts_solareclipsetype_e2e` under `RUSTFLAGS="-Dwarnings"` → 6 passed,
  0 failed. `cargo clippy --all-targets -- -D warnings` → clean, run separately.
