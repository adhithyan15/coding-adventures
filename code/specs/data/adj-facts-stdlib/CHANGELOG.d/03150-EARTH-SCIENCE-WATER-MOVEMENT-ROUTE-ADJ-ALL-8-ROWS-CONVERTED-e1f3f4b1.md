- `earth-science/water-movement-route.adj` — all 8 rows converted to per-row provenance
  (RS-5e, #14986). Three USGS sentences grounded eight rows, and every row carried all three:
  the ATMOSPHERE sentence as the envelope `source`, the other two as `cites`. So
  `? water_movement_route(runoff, $R)` came back proved, as its **primary** source, by *"Water
  moves between the atmosphere and the surface through evaporation, evapotranspiration, and
  precipitation."* — **a sentence that does not mention runoff** — with the sentence that does
  mention it demoted to a corroboration. In `--explain`, which renders the primary source and
  drops corroborations, the runoff row displayed under a sentence that does not ground it.

  Each row now carries the sentence that **lists it by name**: 3 rows on the atmosphere
  sentence, 3 on the surface sentence, 2 on the ground sentence. Three rows sharing a span is
  honest here rather than a compromise — each sentence names each of its own processes
  explicitly. No row restates `locator`: all three sentences are on the one page the envelope
  names, which is the rule `geography/reference-lines.adj` states.

  **The envelope becomes the page's own framing sentence** — *"The water cycle describes where
  water is on Earth and how it moves."* — chosen because it names **none** of the eight row
  keys and so cannot mis-warrant any of them. `source` is a required envelope field, so the
  slot has to be filled by something; filling it with one of the three route sentences is
  exactly what produced the defect. All eight rows override it, so **the framing sentence now
  reaches zero answers**, which the suite asserts as a count — it is the sharpest form of
  "mis-warrants no row", and it reddens if any row loses its block and falls back.

  **A mutant survived the first harness run, and the fix was to widen what is pinned, not to
  narrow the mutant.** Truncating the SNOWMELT row's citation at a comma left the whole suite
  green: the only full-sentence pin in the file was on `runoff`, and a **prefix** needle —
  `"Water moves across the surface through"` — in this change's own first draft of the row
  counts was satisfied by the truncated span. (That draft never shipped; the count is new here,
  so this is a defect caught inside the change, not one being reported against `main`.) Same
  class as #13916/#13918, one row over. The counts now use whole sentences, and a new
  structural test pins the row spans **set-wise against the shipped `.adj`**: the distinct row
  `source` values must be exactly the three sentences, carried by exactly 3 / 3 / 2 rows, and
  nothing else.

  **Security review then found the one field none of that covered:** the envelope sentence
  itself had no assertion on its VALUE — one needle was a prefix inside an `assert_eq!(…, 0)`,
  which stays 0 for any string, and the other only checked what the sentence does *not* say. A
  fabricated framing sentence naming no process left all eight tests green. It is now pinned
  verbatim. Review also found `contains("\"corroborations\":[]")` was satisfiable by any
  unprovenanced block in stdout (the CLI's `UNRESOLVED_PROV` ends with exactly that text); the
  runoff warrant is now pinned as one contiguous span of source + locator + trust + empty
  corroborations.

  A local mutant harness (scratch, not shipped — so **this count is not reproducible from the
  repo**) runs 11 mutants against the suite, all killed, with a green baseline before and
  after. It did not cover the envelope-span case above, which is why review found it and the
  harness did not.

  A second review round found four more, all now applied: the table-level source was taken
  positionally while the locator check was total, so a second envelope source would have been
  invisible; the envelope was lowercased while the row key it is compared against was not, so
  the no-row-key check would have gone vacuous on the first capitalised key; a nine-space
  filter was documented as load-bearing when it could never exclude anything; and **a
  re-measured sentence count was deleted rather than defended** -- it had been offered to fault
  the header original 84-substantive-sentences figure for having no recorded rule, and then
  turned out to depend on an unrecorded rule of its own that review could not reproduce (six
  plausible rules give 73 to 77). Nothing replaces it; the sentence count is simply not
  something this change measured.

  Source re-verified by **raw extraction** (2026-09-14, HTTP 200; 95,733 bytes on disk, 95,701
  Unicode characters), not a fetch summary: all three route sentences occur verbatim after tag-stripping that reproduces the
  page's punctuation, all three inside `<p>` blocks, and a fabricated control sentence is
  reported absent by the same comparison. The three are **consecutive sentences of one `<p>`** —
  which the header's "three parallel sentences" had asserted and never shown.

  Scope: this file. Issue #13898 (`--explain` drops corroborations) no longer reaches this
  table, because no row has corroborations any more; nothing here is claimed about other tables.

