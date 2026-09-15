- **#14070 installment 3a: four glyph-flattened `source` values repaired, five header quote lines
  synced across all four files.** `astronomy/lunar-eclipse-type`,
  `civics/electoral-college-count`, `earth-science/seismic-wave-arrival-order`,
  `geography/map-type-classification`.

  Each shipped an ASCII apostrophe where its page renders U+2019, so **the citation did not appear
  on its own page** — the premise being that a caller can check a citation against its locator.
  All four now verify verbatim, and the old forms appear on none of the pages.

  **The replacement text was taken from the page, not hand-curled.** A candidate swap was applied
  and the result confirmed present in a rendered block. Hand-curling is how the earlier undercount
  happened: assuming the contraction rule covered every case left eight quoted-word values parked
  in an unexplained NEITHER bucket that a contraction-only swap could not resolve. They were never
  called clean — the note on that run said they were "likely glyph issues plus a second
  difference" — but the reason given for holding them back was wrong: the page opens a quoted word
  with U+2018, not U+2019. Keeping them separate was right; the explanation was not.

  **All four carried the same flattened string in their header; all four headers are fixed here.**
  The first pass fixed only two — the security review caught the other two, whose headers wrap the
  quote across two lines, which the screen could not see. Installment 1 repaired header quote
  blocks in all four of its libraries for the same reason; the installment-2 entry calls that “the
  header/data desync caught in installment 1”, singular, so treat the count as this entry’s
  recollection rather than something the log corroborates.

  A PRE-EXISTING PIN IN `electoral-college-count` FAILED, CORRECTLY. It asserted the whole
  citation, anchored on the JSON key and closed on the terminating quote, and its own comment
  cites #13916 and #13918 on why fragments are insufficient. It was right in form and **faithfully
  defended a value absent from its own page**. Repointed to the curly form. Second instance this
  effort of an *anchored* pin protecting a defect, after `mixture-types`' five-clause join. (The
  `oganesson` case belongs to the opposite category and was miscited here at first: that assertion
  was a bare **unanchored** substring, and what it obstructed was a correct repair.)

  Four restore-the-flattened-form mutations pass — each differs from the repaired value by **one
  character**, so a pin that survived would not be checking the glyph at all.

  **Three screening defects surfaced while shipping this; two of them were in the header screen.**
  It scanned `source`/`cites` lines and never looked at header quotes at all —
  `lunar-eclipse-type`'s header claims three verbatim quotes and the first pass repaired only the
  one that was also a `source`. A header screen written to close that gap then reported two of
  this PR's own files clean, because both wrap their quote across two `%` lines and it only
  recognised a complete `"..."` span on a single line. The security review caught both. A second,
  cell-based screen finds the wrapped ones; **neither screen is a superset of the other**, so the
  union — 31 sites across 19 files — is the work list, filed as #14111. Five header lines are
  repaired in this PR (the four matching a shipped `source`, plus lunar-eclipse-type’s third
  verbatim quote); the union above is measured after those. The same sentence repaired here in
  `map-type-classification` still sits flattened in sibling `map-type`'s header: header text
  propagates into derived libraries' shipped data, so this is a defect waiting to happen.

  `electoral-college-count`'s header claimed the page had been "read byte-for-byte", with the em
  dashes specifically confirmed as U+2014 "so the quote is byte-faithful" — while an ASCII
  apostrophe sat two rows above in the same table. **A byte-for-byte read that only looks for the
  character you already suspect is not a byte-for-byte read.** That note is now scoped honestly.

  **The NASA page is mixed, and a blanket curl would have broken a correct quote.** It renders the
  total and penumbral sentences with U+2019 and the partial one with an ASCII apostrophe. Curling
  all three would have made a currently-verbatim quote stop appearing on its own page. The file
  now carries a DO-NOT-NORMALIZE note so a future editor does not "fix" the odd one out.

  *Scope note:* this is **4 of 19** confirmed glyph repairs; **15 remain, and 4 of those are not
  `language/*`** — `biology/cell-division-daughter-cells:93`,
  `biology/cell-division-genetic-outcome:55`, `biology/heredity-term:122`,
  `physics/newton-laws:88`. The language group still ships separately, because a 19-site PR is
  harder to review than coherent ones.

  **This entry first said "4 of 15, the other 11 all `language/*`", and that was wrong in a way
  worth recording.** The collector wrapped its fetch in `except Exception: continue`. TLS to
  several hosts fails on this machine (a proxy root the Python bundle does not carry), so those
  sites raised, were skipped, and came out the far end **indistinguishable from sites that had
  been checked and found clean**. A screen that reports absence of evidence as evidence of absence
  does not fail loudly; it fails silently and confidently, and it had me telling the next
  installment to sweep `language/*` and stop — leaving four biology/physics citations shipping
  text absent from their own pages. The transport now goes through curl (system trust store,
  **verification not disabled**) and fetch failures are counted and printed separately from clean
  results. One site still cannot be reached (`biology/vitamin-deficiency-symptom:77`) and is now
  *visible* as unchecked rather than silently counted as fine.


  533 test binaries / 1622 tests green, clippy `-D warnings` clean; all four `.query.adj`
  companions parse, run and abstain correctly.

