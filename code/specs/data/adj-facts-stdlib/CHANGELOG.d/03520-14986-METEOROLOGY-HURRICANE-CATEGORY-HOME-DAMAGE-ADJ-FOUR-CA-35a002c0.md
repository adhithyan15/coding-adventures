- **#14986: `meteorology/hurricane-category-home-damage.adj` — four categories stop being warranted by the category-1 sentence.**
  The envelope was the category-1 span, so asking what a category 5 does to a well-built home returned
  `total_roof_failure_and_wall_collapse` evidenced by *"Very dangerous winds will produce some
  damage…"* — a sentence about category 1. The other four NHC sentences were header prose that reached
  no answer. Same defect, same page and same locator as its sibling `hurricane-categories.adj`, which
  this batch converted immediately before.

  ### What each row carries now

  Measured 2026-09-16 **UTC** by **this table's own fetch**, not by carrying the sibling's figures across
  (HTTP 200, 27 908 bytes; a nonsense path on the same host returns a real 404 of 16 bytes holding
  none of these spans), with inline tags removed without a space and only ASCII whitespace collapsed.
  - **Each row takes the NHC sentence stating its own structural effect.** Each occurs **exactly
    once**, inside one `<td>`, scripts set aside and over the whole file, zero times on the nonsense
    page, and zero times in `<head>`.
  - **The envelope** is the page's scale-defining sentence — *"The Saffir-Simpson Hurricane Wind Scale
    estimates potential property damage."* — once, inside one `<p>`, naming no category and no damage
    value. (It does contain the word *damage*; what it names no instance of is a damage **value** —
    the five row atoms. The test's word list pins that, and deliberately does not pin "damage".)

  ### No duplicate value here, and every value is stated by its own span

  Unlike the sibling table, whose `descriptor` column honestly duplicates `catastrophic_damage` across
  categories 4 and 5, **all five values here are distinct** — the page's home-damage detail differs
  category by category even where its summary word repeats. So no merge-direction test is needed: the
  ordinary forward binds already discriminate.

  What this table needs instead is the **#15318 check** — that each row's *value* is actually stated
  by the span that row carries. A new test normalizes each span and asserts the de-underscored atom
  occurs in it. All five: yes.

  ### Pins

  - **Kept:** the forward binds, the backward bind, and the `category_6` abstention.
  - **Inverted:** `contains("nhc.noaa.gov") && contains("\"trust\":\"authoritative\"")` — satisfied by
    any NHC citation, and previously satisfied by the category-1 span riding on every answer. Each
    answer is now pinned to its own whole citations array, closing on both the corroborations `]` and
    the citations `]` (#14735).
  - **Added:** a per-category check; a table-shape test (five row sources, keyword-anchored `cites`
    absence, **exactly one** `locator` and **exactly one** `trust` line, and a bare-word `category`
    needle on the envelope); and the value-supported-by-its-own-span test above.

  ### The mutation run scored 6 of 10 first, and the harness was what was wrong

  **10 of 10 mutants are now killed by their named assertion**, with both controls green and the files
  byte-identical afterwards. But the first run scored **6 of 10**, with four "killed for the wrong
  reason" — and the diagnosis is worth recording, because three of the four were **harness** faults,
  not weak tests:

  - **A file-only mutation cannot exercise an arm whose expectation lives in the test.** Rebinding a
    row's value atom in the `.adj` left `scale()` still holding the original atom, so the
    value↔span arm compared the *old* atom against an unchanged span and passed, while the CLI binds
    died on the mismatch. Same for the envelope mutant: editing only the `.adj` tripped the verbatim
    envelope-block arm before the word list it was aimed at. Both now co-edit the test.
  - **The fourth was a wrong expectation, not a wrong mutant.** A swapped span is caught first by
    `only_citation(span)`, not by the per-span negative loop — so **that loop is redundant for the
    span-swap class**, and this entry does not claim it as the killer.

  Finding this meant reading the panic **line numbers**: the first diagnostic regex assumed a panic
  format cargo does not emit and silently matched nothing, reporting which tests failed but not which
  assertion fired. **A diagnostic that parses nothing must say so rather than print an empty result.**

  It's a local scratch harness, so that count can't be reproduced from the repo.

  ### The query example, reported as it actually runs

  3 answers and 1 abstention; the envelope's wording occurs **zero** times in the output, there are 3
  citations arrays and **no** non-empty corroborations. The example queries only categories 1 and 5
  plus the backward bind, so `siding damage`, `gable ends` and `exterior walls` occur **zero** times
  in its output — those rows are exercised by the test suite, not by this example.

  Its stale line said the engine returns the effect plus "the table's source/locator/trust"; it now
  describes the row's own NHC sentence with the locator and trust inherited.

  ### Three header claims retired

  The header asserted the file "reproduces, byte-for-byte, the SAME five spans already quoted inside
  `hurricane-categories.adj`'s own provenance block … **no new WebFetch**", and that an ADJ table
  carries one envelope "**per `hurricane-categories.adj`'s own convention**". The convention claim dies
  with the sibling's own RS-5e conversion, and the no-new-WebFetch claim is deliberately falsified —
  this conversion re-measured the page rather than inherit figures. A third clause called the spans
  "already-quoted … that never made it into a row", which the conversion makes false in both halves:
  it survived the converter and was caught by reading the spliced header as prose, the same way the
  sibling's stale paragraph was.
