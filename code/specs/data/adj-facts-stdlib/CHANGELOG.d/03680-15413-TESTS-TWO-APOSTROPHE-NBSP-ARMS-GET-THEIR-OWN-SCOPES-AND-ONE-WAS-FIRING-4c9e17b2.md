- **#15413: two e2e tests get the scoped apostrophe/nbsp arms.** `facts_eyepartproperty_e2e.rs`
  (U+00A0) and `facts_figurativelanguagetype_e2e.rs` (U+2019) both read the whole `table … { … }`
  block in their negative arm. That is the pre-correction form, corrected for `plant-parts` in #15337
  (shard 03560) and `solar-eclipse-type` in #15338 (shard 03670). Test-only — no `.adj` change, no
  shipped data change.

  Each negative arm now reads `source` lines at any indent, never the block, because a `%` comment
  quoting a span is not a shipped citation. Neither positive half needed scoping: eye-part-property
  pins all three `row (part, property) { source "…" }` blocks verbatim and counts the row sources,
  and figurative-language-type already anchors its metaphor row at eight spaces.

  ### One was firing, the other was not

  This is the difference between the two files, and it is the reason to state severity per file
  rather than per defect class:

      table                      % comments inside the block   false alarm
      eye-part-property.adj                    4               ACTIVE
      figurative-language-type.adj             0               available, not active

  `shipped_table()` slices from `table …` to end of file in both, so a comment inside the block is
  inside `body`. Measured on the shipped eye-part-property block with the ordinary-space form planted
  in a comment: the unscoped arm sees it and fails a correct file; the scoped arm does not.

  ### Both arms observed firing, at their own assert lines

      mutant                     eye-part-property (185)    figurative (202)
      variant on its OWN row     KILL, arm fired            KILL, arm NOT reached
      variant on ANOTHER row     KILL, arm fired            KILL, arm fired
      variant in a % comment     SURVIVE                    SURVIVE

  The asymmetry is not noise. `the_retina_row_carries_the_pages_no_break_spaces` holds only the
  U+00A0 count and this arm, so either row mutant reaches it. In figurative-language-type the arm
  sits inside `the_table_shape_matches_the_measured_rows` behind three guards, and Rust
  short-circuits: an ASCII form on the metaphor row trips the row-keeps-U+2019 assert, at table level
  trips not-metaphor-as-envelope, and on an added fourth row trips the row-source count of 3. Only
  planting it on an existing NON-metaphor row satisfies all three and reaches the arm. Three routes
  were dominated before the fourth was found; concluding "unreachable" from the first three would
  have shipped an assertion never observed to fire.

  ### The trade

  Scoping lets a comment-borne variant survive. Nothing now pins "no variant anywhere in the block",
  only "no `source` line carries one" — the same trade recorded in 03560 and 03670. That is the right
  trade, because the shipped string is what provenance means, but it is a gap, so it is written down.

  ### Non-vacuity

  A negative arm over an empty filter passes trivially. Each filter reads **4** `source` lines in its
  shipped table, and each table has **1** four-space envelope line.

  ### How the census found them, and why two earlier screens would not have

  #15338's scope search and its follow-up both keyed on the identifier `ascii_variant`; neither file
  uses that name, inlining `.replace(…)` at the assertion instead. A behaviour-keyed census — any test
  building an apostrophe/nbsp variant of a shipped span — finds 13 such files across 527 in
  `code/packages/rust/adj-lang-cli/tests/`.

  Screening those 13 on "no source-line filter at all" is also wrong, and disagrees with classifying
  by what the arm reads on three of four candidates: the screen yields `constants`, `eyepartproperty`,
  `eyeparts`; the classifier yields `figurativelanguagetype`, `eyepartproperty`. `eyeparts` reads a
  pinned `.adj` slice and `constants` reads CLI output, neither reachable by an in-table comment;
  `figurativelanguagetype` is excluded by the screen because it already carries one eight-space row
  pin. A filter count is not a classifier; what the arm reads is.

  Gates: `cargo test --test facts_eyepartproperty_e2e` and `--test facts_figurativelanguagetype_e2e`
  under `RUSTFLAGS="-Dwarnings"` → 6 passed, 0 failed each. `cargo clippy --all-targets -- -D warnings`
  → clean, run separately. Every mutant was written to the tracked `.adj` inside a try/finally and
  restored byte-identical, verified clean in git after each run.
