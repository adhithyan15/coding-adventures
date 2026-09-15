- `anatomy/eye-parts.adj` — all 6 rows converted to per-row provenance (RS-5e, #14986). The
  CORNEA sentence was this table's `source`, the field that carries the tier, for every row, so
  `? eye_part_function(optic_nerve, $F)` came back proved by *"The cornea is shaped like a dome
  and bends light to help the eye focus."* Every sentence is on the same NEI page, so the
  locator was never wrong and a hostname assertion could never have caught it — only the span
  was.

  **No row carries a `locator`**: six sentences, one page, so every row inherits the envelope's,
  which is the form ADJ-TABLES RS-5e's own example uses and the one #15197 asks whether the
  cascade should adopt everywhere. A visible consequence: this table has **no equivalent
  mutant to declare** — the "drop a row's locator" mutation does not exist, and the locator arm
  is instead "ADD one", which the structural test kills.

  **Two sentences name more than one part**, which is why "the span mentions the key" is not a
  sufficient pairing test on its own: the LENS sentence names the cornea and the retina, and
  the OPTIC-NERVE sentence names the retina. Giving the cornea row the lens sentence passes a
  mention test — it is caught by the source-equality pin, and both swaps are mutants that die.

  Block structure checked first: 6 `<p>` and 29 `<li>`, with every sentence used here coming
  from one of three narrative paragraphs; the list items are site navigation. All seven spans
  confirmed verbatim, each occurring exactly once, each with a negative arm absent, all six row
  spans distinct.

  `facts_eyeparts_e2e.rs`: adds 3 tests (4 total), including the #15193 structural check — six
  distinct spans and **zero** row-level locators. 15 of 15 mutants killed, baseline green.
