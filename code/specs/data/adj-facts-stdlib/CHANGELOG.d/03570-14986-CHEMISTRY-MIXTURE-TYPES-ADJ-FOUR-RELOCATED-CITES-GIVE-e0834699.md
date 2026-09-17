- **#14986: `chemistry/mixture-types.adj` -- four relocated `cites` give every mixture kind its own sentence.**
  The envelope was the `homogeneous` span and the other FOUR sentences sat in table-level `cites`, so
  asking what a colloid is returned `milk` warranted primarily by a sentence about homogeneous salt
  water. This is the largest relocation in the batch: `comet-tail-type` moved one `cites`, this moves
  four, and the corroborations array on every answer goes from non-empty to **empty**.

  **The file stated its own defect.** Its provenance paragraph read: *"The `source` below holds ONE of
  those blocks verbatim and the other four ride as `cites` corroborations on the same envelope, so
  every row's evidence is independently checkable."* That paragraph is rewritten, not left standing
  beside contradicting data.

  **TWO supersessions, and both earlier rationales were accurate when written.** #14070 split a joined
  five-clause `source` (a span no page displays) into one `source` plus four `cites`; that repair's
  *method* stands and is kept in the header. Its *conclusion* does not: a row's evidence sitting in a
  table-level corroboration still never reached that row's own answer.

  ### What each row carries now

  Measured 2026-09-16 UTC by this table's own fetch of the LibreTexts CHM 200 "9.1: Mixtures" page,
  inline tags removed without inserting a space, only ASCII whitespace collapsed.
  - Five spans, five rows, **one-to-one**. Each occurs **exactly once** on the page and **zero** times
    against a real 404 control (a nonsense path on the same host returns **404**, with a real error
    page rather than an empty body). Each is tightest-held in a `<p>`. The control's byte count is
    deliberately not quoted: two fetches of it in one session returned 99 203 and 99 265 bytes, so that
    figure is point-in-time and not reproducible. The load-bearing facts are the 404 status and the
    zero span hits; the article itself measured 148 984 bytes on both fetches.
  - Each span names **exactly one** row key -- envelope→`homogeneous`, and the four cites→`solution`,
    `heterogeneous`, `suspension`, `colloid`.
  - The envelope is now the page's framing definition of a mixture, which names no row key.

  ### The envelope's spaces are U+00A0

  The page writes that sentence's first two gaps as NON-BREAKING spaces. Counted on the fetched page:
  that spelling occurs **once**; the same sentence with ASCII spaces occurs **zero** times, so shipping
  the ASCII form would cite a sentence the source does not contain (#15324's defect for
  `soil-texture-class`). The envelope is extracted from page bytes by slicing backwards from a stable
  ASCII tail, never retyped, and is deliberately **not** quoted in full in the header -- an ASCII-spaced
  copy in a comment is the very twin the tests look for.

  **Three display layers rendered those bytes as ordinary spaces**: the terminal, a ripgrep result, and
  a diff view. The ripgrep output was indistinguishable from the defect; "fixing" it would have
  manufactured exactly the error this batch repairs. Only `repr()` on the file settled it. This is not
  a first -- 15 other stdlib tables already carry U+00A0, **31 characters** across them (the same
  census reads **22 lines**, and **28** characters if restricted to `source`/`cites` lines -- an
  unscoped "occurrences" is not a measurement), `chemistry/reaction-types.adj` among them -- checked
  before writing "first".

  **The "zero times" margin is one letter wide, and saying so is part of the claim.** Counted on the
  page: the shipped NBSP form occurs **1**, the ASCII-spaced form of the same sentence **0** -- but the
  ASCII-spaced sentence ending *"in any **proportions**."* occurs **1**, in the Summary section. So the
  negative arm would not fire on someone shipping that Summary sentence; it is a different, and still
  verbatim, page span. The arm pins the exact envelope, not "no ASCII mixture-definition anywhere".

  ### Negative arms name whole spans

  Measured, not assumed: no span is a substring of another, but `homogeneous`/`solution` share "salt"
  and "water", and `homogeneous`/`heterogeneous` share "composition", "mixture", "throughout" and
  "uniform". No single word separates them, so every negative arm names a WHOLE SPAN -- the hazard
  `plant-parts` has with "roots" and `comet-part` with "nucleus", reached by a third route.

  ### A property the header argued for and nothing checked

  `homogeneous` and `solution` both bind `salt_water` -- the source's own point, since a solution IS a
  homogeneous mixture. The header devotes a paragraph to it. `grep salt_water` over the e2e test
  returned **nothing**, and the query example never binds it either (its reverse query binds
  `vegetable_soup`; measured `Kind=homogeneous` 0, `Kind=solution` 0). The property defended hardest was
  the one with no coverage. `the_two_salt_water_kinds_cite_different_sentences` now covers it, and
  measurement confirms the engine returns **both** rows, each carrying its **own** sentence -- so no arm
  there may assert one citations array or that either span is absent.

  ### Pins

  - **Replaced, not edited:** `MT_SUSP_PIN` and `MT_ALL_PIN` each spelled out the now-dead non-empty
    `corroborations` array as their entire middle, and the two tests around them existed only to assert
    that shape. The envelope pin asserted the `homogeneous` span *as* the envelope. All three are gone.
  - **The pre-existing pin has now defended two different defects.** Its methodology was right both
    times -- anchor on the JSON key, close on the terminating quote, never pin a fragment -- and it
    caught a withdrawn repair. But an anchored pin defends whatever it points at: first a constructed
    span (#14070), then the wrong row's real sentence (#14986).
  - **Added:** a per-kind loop with whole-span negative arms; the duplicate-value test; a U+00A0 test in
    two scopes (positive arm reads 4-space `source` lines only, negative arm any indent); a shape test
    with keyword-anchored `cites` absence and exactly-one `locator`/`trust`; and a
    span-supports-its-own-row test with per-row needles and no catch-all arm.
  - The shape test's whole-word check includes **plurals**. A singular-only check has a hole: the token
    "solutions" never equals the key "solution", and the page carries *"…three different types:
    solutions, suspensions, and colloids."* That sentence passed my own eligibility filter as an
    envelope candidate until the check was widened.

  ### 10 of 11, with one expected survivor -- and the tenth kill was EARNED

  Both controls green, both files byte-identical after restoration, and all six mutation anchors
  verified to match exactly once beforehand, so no mutant was silently refused (a refusal shrinks the
  denominator without announcing itself).

  **The survivor is kept, not deleted.** An ASCII-spaced envelope inside a `%` comment in the table
  block: comments are lexer-skipped, and the U+00A0 arm is scoped to `source "` lines because a comment
  is not a shipped citation. Correct behaviour, and an honest gap -- nothing pins "no ASCII envelope
  anywhere in the table block".

  **The first run scored 9/11, and the misfire was a real finding.** `M07` swaps the two `salt_water`
  spans so each row carries the other's sentence. The duplicate-value test asserts both spans are
  present and that the two consts differ -- all **symmetric under a swap**, so it correctly passed.
  Nothing in the file-shape arms bound a span to *its own row*; the needle
  `        source "<span>"` proves only that a span sits among the eight-space source lines somewhere.

  **The fix was to strengthen the test, not to relabel the mutant.** The needle is now anchored on the
  row header -- `row ({kind}, {example}) {{\n        source "{span}"` -- so it pins the assignment. M07
  then died by its named assertion. Re-pointing M07 at the per-kind engine test, which already caught
  the swap, would have bought the same 10/11 while leaving the file-shape arms symmetric. The residue
  is stated rather than scored away: `assert_ne!(HOMOGENEOUS, SOLUTION)` compares two literals and can
  never fail for any `.adj` edit, and the token loops in that test read a const, so the row-header
  anchor above is what ties them to shipped data.

  Mutants used **real page text** where possible: the plural-types sentence, and the page's Q&A line
  naming salad dressing without ever saying "suspension" -- the decoy shard 02660 records, found by
  reading **both** salad-dressing sentences rather than the first match.

  It's a local scratch harness, so the count can't be reproduced from the repo.

  ### Verified locally

  `facts_mixturetypes_e2e` 6/6 with `RUSTFLAGS=-Dwarnings`; clippy `--all-targets -D warnings` clean;
  `git diff --check` clean. All three validators CI runs pass, read by **content** and not by exit code
  alone: `adj_stdlib_manifest --validate-json-schema` → `valid: true, errors: []` (222 objectives);
  `adj_stdlib_provenance verify` → `valid: true` (11 bundles, 125 objects); `adj_stdlib_report
  --fail-on-unreferenced-tests` → `gaps.missing_test_reference: []`; and the
  `test_adj_stdlib_provenance` unittest, 227 tests OK (7 skipped).

  The query example runs: 4 answers, 1 abstention, **4** citations arrays, **0** non-empty
  corroborations, and the envelope's wording in **zero** answers. Note **8** empty-corroboration
  occurrences against 4 answers, because provenance renders on two surfaces -- `citations` and the
  `steps` array's `"kind":"fact"` entries. `only_citation()` pins the first; the second is guarded only
  by the cross-span negative arms.

  `homogeneous` and `solution` score **0** in that example under every instrument, because it never
  queries those rows; they are exercised by the suite.

  The query companion's "the table's source/locator/trust" line is re-pointed, with the old wording
  quoted in place because it was accurate before this conversion. The README's row for this table --
  *mixture kind → the everyday example the source names* -- stays true under per-row provenance and is
  left alone; checked against the line, not assumed.

  Header line 17-18 also carried the retired "carrying the table's citation" boilerplate (#15336),
  wrapped across a line break so a grep for the phrase finds nothing here. Fixed in passing.
