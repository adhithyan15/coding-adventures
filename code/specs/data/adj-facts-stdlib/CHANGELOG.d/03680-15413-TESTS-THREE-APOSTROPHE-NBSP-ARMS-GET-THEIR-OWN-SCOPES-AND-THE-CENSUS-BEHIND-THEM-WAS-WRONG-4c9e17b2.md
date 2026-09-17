- **#15413: three e2e tests get the scoped apostrophe/nbsp arms, and the census behind them was
  wrong.** `facts_eyepartproperty_e2e.rs` (U+00A0), `facts_figurativelanguagetype_e2e.rs` (U+2019)
  and `facts_eyeparts_e2e.rs` (U+00A0) all read the whole table block in their negative arm. That is
  the pre-correction form, corrected for `plant-parts` in #15337 (shard 03560) and
  `solar-eclipse-type` in #15338 (shard 03670). Test-only — no `.adj` change, no shipped data change.

  Each negative arm now reads `source` lines at any indent, never the block, because a `%` comment
  quoting a span is not a shipped citation. No positive half needed scoping: each already pins its
  row at eight spaces or pins the row block verbatim.

  ### The third file was found by a review pushing back on my own exclusion

  This change shipped as two files. Its issue, its commit message and its PR body all named
  `facts_eyeparts_e2e.rs` and ruled it out — "reads a pinned `.adj` slice … not reachable by an
  in-table comment". That was false. `facts_eyeparts_e2e.rs:167` slices
  `adj[adj.find("table eye_part_function")..]` — **to end of file**, the same shape `shipped_table()`
  has. The phrase described where the bytes came from; the question was how far the arm reads.

  `eye-parts.adj` carries **twelve** `%` comment lines inside that block. With the ordinary-space
  forms planted in them, the unscoped arm **failed a correct file at line 170**.

  ### Three arms observed firing, each at its own assert line

  Every cell is a panic line from a run at the bytes being shipped, not a deduction:

      mutant                        eye-part-property (189)  figurative (218)  eye-parts (194)
      variant on its OWN row        KILL, arm fired          KILL, unreached   KILL, unreached
      variant on ANOTHER row        KILL, arm fired          KILL, arm fired   KILL, arm fired
      variant in a % comment        SURVIVE                  SURVIVE           SURVIVE

  Where an own-row mutant does not reach the arm, an earlier guard takes it: the positive pin at
  `eye-parts:193`, and `figurative:173`, the row-keeps-U+2019 assert.

  ### Five claims this change had to retract

  Four were mine; the first is the one that would have left a defect in the tree.

  1. **`eyeparts` is not this defect** — it was, and it is fixed here.
  2. **"13 such files across 527"** — re-derived on `origin/main` and on the working tree, it is
     **12**. The 13th was `facts_idiommeaning_e2e.rs`, admitted only by a looser predicate accepting
     any `replace(` plus any curly escape anywhere; its only `replace` is `key.replace('_', " ")`.
  3. **The count screen yields three names** — it yields **four** (`constants`, `eyepartproperty`,
     `eyeparts`, `figurativelanguagetype`), and its stated reason for excluding the fourth cannot
     separate it, because the other two carry eight-space pins too.
  4. **"At table level it trips not-metaphor-as-envelope"** — measured, replacing the envelope trips
     **envelope-is-framing-sentence at :163**. It *cannot* trip the other: that needle is built from
     the curly `METAPHOR`, so an ASCII twin never matches it.
  5. **"Only a NON-metaphor row reaches the figurative arm"** — an **added four-space `source` line
     reaches it too** (panic at 218). This followed from the fix's own shape: the scoped filter is
     indent-agnostic by design. The claim contradicted the thing it was describing.

  The figurative routes, all measured:

      route                             panic   what it trips
      ASCII on the metaphor row          173    row-keeps-U+2019
      ASCII replacing the envelope       163    envelope-is-framing-sentence
      ASCII on an added fourth row       154    the row-source count of 3
      ASCII on another row's source      218    THE ARM
      an added four-space source line    218    THE ARM

  ### What the comment counts do and do not say

      table                          % comments inside the block
      anatomy/eye-parts.adj                     12
      anatomy/eye-part-property.adj              4
      language/figurative-language-type.adj      0

  An earlier wording called the first two cases ACTIVE and the third merely available. **All three
  tests are green at HEAD**, so nothing was firing; what the count distinguishes is whether the
  hazard needs a new comment zone or only one more comment line. That is the smaller claim, and it
  is the one the measurement carries.

  ### Non-vacuity

  A negative arm over an empty filter passes trivially. The filters read **4**, **4** and **7**
  `source` lines respectively, each table contributing exactly **1** four-space envelope line.

  ### The census, re-derived, and what it still missed

  #15338's scope search and its follow-up both keyed on the identifier `ascii_variant`; none of these
  files uses that name. A behaviour-keyed census — any test applying `.replace('\u{XXXX}', …)` to a
  shipped span — finds **12** files across **527**. Classifying all twelve by the receiver their arm
  reads, item by item: **7** scoped (these three plus `mixturetypes`, `plantparts`,
  `solareclipsetype`, `sunlayer`), **1** reading CLI output only (`constants`), and **4** still
  reading the whole block — `cloudsignal:122`, `cloudtype:119`, `longboneparts:146`,
  `musclegroups:202`, filed as #15415. Those four hid from both censuses because they build the
  variant once, compare it to a named constant, and assert the **constant** against the block, so the
  arm's needle is not a `.replace` at all. A filter count is not a classifier; what the arm reads is.

  The classifier written to find them reproduced the same fault in miniature. It looked for a needle
  on the assertion's own line, matching either a `.replace` or an upper-case `_PLAIN`/`_BEFORE`
  constant; in `mixturetypes`, `plantparts`, `solareclipsetype` and `sunlayer` the needle is a
  **lower-case local** (`ascii_variant`, `ascii`) bound a line or two above, so it reported "no
  `.contains` arm" for all four. Those four are correctly scoped — verified one at a time — so the
  census reached the right answer about them for the wrong reason, and a census that is right for the
  wrong reason is one input away from being wrong.

  ### The trade

  Scoping lets a comment-borne variant survive. Nothing now pins "no variant anywhere in the block",
  only "no `source` line carries one" — the same trade recorded in 03560 and 03670. That is the right
  trade, because the shipped string is what provenance means, but it is a gap, so it is written down.

  Gates at the shipped bytes: `cargo test` under `RUSTFLAGS="-Dwarnings"` → 6, 6 and 5 passed, 0
  failed. `cargo clippy --all-targets -- -D warnings` → 0 warnings, run separately. Every mutant was
  written to the tracked `.adj` inside a try/finally, restored, and verified byte-identical by
  SHA-256 with a clean `git status` after each run.
