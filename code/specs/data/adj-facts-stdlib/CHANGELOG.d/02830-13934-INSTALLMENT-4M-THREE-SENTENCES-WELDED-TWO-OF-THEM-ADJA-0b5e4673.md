- **#13934 installment 4m: three sentences welded, two of them adjacent — and a debt from 4l.**
  **1** value line rewritten into **2** `cites`, alongside **41** comment lines added against
  **12** removed, in **2** `.adj` files (43 added / 13 removed lines in total) —
  `geography/reference-lines` and `geography/ocean-deepest` — plus one e2e test file.
  **Six mutations redden.**

  ### The candidate was not the value I went looking for

  The backlog names `geography/reference-lines:109`. Line 109 is the NESDIS **`cites`**, not the
  equator `source` four lines above it. The `source` was measured anyway: `"The equator is the most
  well known parallel. At 0 degrees latitude, it equally divides the Earth into the Northern and
  Southern hemispheres."` occurs **once, byte-exact in the raw HTML**, inside a single `<p>`, with
  no tag between its two sentences. It is a contiguous span and needed no repair. The file has one
  commit in its history and is 110 lines long in both revisions, so the backlog's line number and
  today's line 109 are the same line.

  ### What line 109 actually held

  Three sentences from the NOAA NESDIS "What Is a Solstice?" page under one locator:

  | | occurrences | where |
  |---|---|---|
  | S1 "In the Northern Hemisphere, the Summer Solstice occurs … usually June 21." | 1 | one `<p>` … |
  | S2 "In the Southern Hemisphere, … usually December 21." | 1 | … the same `<p>`, no tag between them |
  | S3 "Two other significant lines of latitude are the Arctic Circle (around the North Pole) and the Antarctic Circle (around the South Pole)." | 1 | a different `<p>`, **3,665 raw characters** later, with **48** block-element tags in between |
  | **S1 + S2 as one string** | **1** | the contiguous pair |
  | S2 + S3 as one string | **0** | — |
  | **the shipped 392-character value** | **0** | — |

  Counted under both extractors — text nodes breaking only on block elements, and the crude
  every-tag-is-a-break one — with HTML comments stripped in both. The pair is now one `cites` and
  S3 is a second `cites` under the same locator. **Both are needed:** the pair grounds the two
  tropic rows and S3 grounds the two polar-circle rows, so this is not one span being trimmed.

  **The header already had it right.** Its evidence block lists S3 separately for
  `arctic_circle`/`antarctic_circle`, and marks the tropics quote's own join with `— and —`. Only
  the machine-readable field welded. That asymmetry is recorded rather than smoothed over: the
  comment a human reads and the field a program reads disagreed, and the program's was wrong.

  ### Why it survived: the pin was a host and a trust tier

  `contains("oceanservice.noaa.gov") && contains(trust)` — the same shape as 4l's oceans pin, and
  satisfied by the welded string exactly as happily as by the two real spans. The needle is now the
  whole citation object as the serialiser emits it, closing on the corroborations `]`, so it bounds
  the source text, the locator, the trust tier and the corroboration set at once.

  Its string appears **four** times in the test's output, at four distinct JSON paths —
  `recall/[0]/answers/[0]/citations/[0]` and `…/steps/[0]`, and the same pair under `recall/[1]`
  for the reverse query. That was counted before the needle was trusted, and each of the six
  mutations drove all four to zero together: echoes of one field, not the independently-driftable
  copies of #14745.

  ### A finding about the test itself

  A second assertion was added saying the weld must not come back. Run through `cargo test`, **all
  six mutants were caught by the citation-object assertion and that one never fired** — it sat
  second, so the test panicked before reaching it. An assertion never observed to fire is
  decoration. It was moved **first**, and now `RESTORE` is caught by it while the other five are
  caught by the citation object. Both have a live positive control instead of one having none.

  ### 4l's debt, repaid

  `ocean-deepest.adj` described the Atlantic sentence as "the rest of the span" of `oceans.adj`'s
  `source` — true while one `source` held both sentences, stale since 4l split them. Two more
  claims in that header had the same cause: "the ALREADY-cited span" (singular; that table now
  cites two) and "the SAME **opening** sentence" (there is no longer a weld for it to open).
  Nothing the file ships was wrong — its own `source` is the Pacific sentence and its one row is
  grounded by it. The 805-character gap was **re-measured this session** from freshly fetched
  bytes under both extractors rather than copied from 4l's note.

  ### Mutants

  `RESTORE` (the 392-character weld put back), `DROP-S3`, `SWAP` (the two corroborations
  exchanged), `WORD` (`June 21` → `July 21`), `LOCATOR-TAIL`, `EXTRA-CITES` (an extra corroboration
  under `https://evil.example/facts`). Each turns the real suite red; none is vacuous; none crashes
  the CLI, so none reddens for the wrong reason. The shipped `.adj` was restored byte-identically
  afterwards (sha256 checked).

