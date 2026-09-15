- `transportation/sign-shape.adj` (new) — a `table` recording the shape the federal standard
  specifies for a traffic sign: `sign_shape(sign, shape)`, five rows from the FHWA's Manual on
  Uniform Traffic Control Devices. `stop_sign` → `octagon`; `yield_sign` →
  `downward_pointing_equilateral_triangle`; `no_passing_zone_sign` →
  `pennant_shaped_isosceles_triangle`; plus the two shape DEFAULTS, each carrying its own
  defeasibility. THIRD transportation library, joining `traffic-lights.adj` and
  `green-signal-permitted-movement.adj`, which cite the same publisher. A deliberate move for breadth
  after six consecutive earth-science water libraries; `ADJ-STDLIB-COVERAGE.md` 5.1 names safety as a
  K-2 gap.

  The BACKWARD query is the one a child actually asks: `sign_shape($Sign, octagon)` answers "which
  one is the eight-sided sign?".

  THIS TABLE HOLDS A DEFEASIBLE DEFAULT ALONGSIDE ITS OWN COUNTEREXAMPLES, AND THAT IS WHAT MAKES THE
  HEDGES LOAD-BEARING RATHER THAN DECORATIVE. The manual states two defaults, each explicitly
  defeasible — "Regulatory signs shall be rectangular unless specifically designated otherwise."
  and "Except as provided in Paragraph 2 or unless specifically designated otherwise, all warning
  signs shall be diamond-shaped..." — and then designates otherwise, repeatedly. STOP and YIELD
  are BOTH regulatory signs and NEITHER is rectangular; NO PASSING ZONE is a warning sign that is not
  a diamond. Recorded flatly, those rows would CONTRADICT EACH OTHER. They do not, because each
  default carries its defeasibility inside its own atom, following the placement rule
  `veto-override.adj` and `karst-process-zone.adj` already apply. A query for a bare unqualified
  `rectangular` therefore ABSTAINS, which is correct: the MUTCD never says regulatory signs ARE
  rectangular, and its own next sections are the proof. This is the clearest case yet for that rule
  — in earlier libraries a dropped hedge would have cost a shade of confidence, whereas here it
  would make the library assert a falsehood the same document refutes two sections later.

  `speed_limit_sign` abstains for the adjacent reason: it IS a regulatory sign governed by the
  rectangular default, but the default is defeasible and STOP and YIELD are standing proof that
  inferring a named sign's shape from it is unsound. Deriving "rectangular" for it would be reasoning
  presented as recall.

  THE WARNING-SIGN CITATION BEGINS WITH ITS EXCEPTION CLAUSE, DELIBERATELY. The manual's sentence
  opens "Except as provided in Paragraph 2 or unless specifically designated otherwise, ..." and only
  then reaches "all warning signs shall be diamond-shaped". Quoting from "all warning signs" would
  have been a shorter, tidier citation that DROPPED THE HEDGE FROM THE EVIDENCE ITSELF — leaving
  an atom that says "unless designated otherwise" backed by a quotation appearing to say no such
  thing. A citation has to carry the qualifier it is being used to justify.

  Provenance: five sentences spanning MUTCD CHAPTERS 2B ("Regulatory Signs, Barricades, and Gates")
  and 2C ("Warning Signs And Object Markers"), both within Part 2 ("Signs"), each `cites` carrying its
  own locator. The chapter placement is load-bearing rather than decorative: STOP (2B.05) and YIELD
  (2B.08) sit inside the REGULATORY chapter, which is what makes them counterexamples to the
  regulatory default rather than unrelated signs, and NO PASSING ZONE (2C.45) sits inside the WARNING
  chapter for the same reason. An earlier draft called these "Parts 2B and 2C"; the manual's own
  headings say Chapter, and "Part 2B" appears zero times on either page.
  Both pages were content-verified by raw text extraction (HTTP 200, 589 and 406 substantive
  sentences, zero soft-404 markers) and every quoted string confirmed to appear verbatim in that
  extraction. TWO OF THE FIVE QUOTES WERE WRONG IN AN EARLIER DRAFT and were corrected before merge:
  they read "(see Figure 2B-1 )" and "(see Figure 2C-8 )" with a space before the parenthesis, which
  the manual does not contain. The space was a TAG-STRIPPING ARTIFACT — the pages wrap figure
  references in `<a>` tags, and the extractor's punctuation normaliser covered `,.;:!?` but not `)`.
  The draft's claim that every sentence "was confirmed to appear verbatim in that extraction" was
  therefore true of the extraction and false of the manual, which is the one thing a
  byte-faithful-quotation library cannot afford. The normaliser now covers brackets on both sides,
  and all five quotes were re-verified against a corrected extraction. `trust authoritative` — the
  FHWA is the first-party federal publisher and this is the standard itself rather than a description
  of it. Honest abstention also on the railroad
  crossbuck, the school-zone pentagon and the interstate shield, which are specified in sections this
  slice does not quote, and on COLOR, which the quoted sentences state but which is a different axis
  deserving its own table rather than being smuggled into a shape column.

  New `sign-shape.query.adj` and `facts_signshape_e2e.rs` (5 tests: the direct lookup with its
  verbatim sentence and citation, the reverse lookup with an exclusivity negative, both defaults
  keeping their defeasibility plus an assertion that the quoted evidence includes the exception
  clause it justifies, the two unqualified-shape abstentions with a positive control, and the
  inferred-from-default abstention with a positive control). Every assertion uses the JOINT binding
  form rather than independent substring scans, and all four negative assertions were mutation-tested
  — dropping either hedge makes the bare-shape queries bind, adding an inferred
  `speed_limit_sign` row breaks that abstention, and a second octagon row breaks the exclusivity
  negative.

  TWO TEST GAPS WERE CLOSED AFTER REVIEW. The exclusivity negative hand-listed only two of the four
  other signs, so adding `row (warning_sign, octagon)` passed every test while the assertion message
  claimed "no other sign may be returned as an octagon" — the message asserting more than the check;
  it now loops over all four. And the two-distinct-locator property, which is this library's
  structural novelty, was never actually asserted: the only locator check was a bare scan for
  `part2b.htm`, which survives swapping every locator and never examines `part2c.htm` at all. A sixth
  test now pins sentence-to-locator PAIRS for both chapters. Both gaps are the same failure mode this
  series has hit repeatedly — asserting on the stdout blob rather than the structural property — and
  both were found by mutation rather than by reading. New manifest objective
  `adj.socialstudies.k2.sign_shape`.

