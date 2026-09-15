- `transportation/sign-element-color.adj` (new) — a THREE-column `table` recording the colour the
  federal standard specifies for each PART of a traffic sign: `sign_element_color(sign, element,
  color)`, nine rows from three sentences of the FHWA's Manual on Uniform Traffic Control Devices.
  `stop_sign`: background red, legend white, border white. `yield_sign`: background WHITE, legend
  red, border red. `warning_sign`: background yellow, legend black, border black. Fourth
  transportation library.

  WHICH PART OF THE SIGN IS PART OF THE FACT, AND THAT IS THE WHOLE DESIGN. "A YIELD sign is red" is
  FALSE as stated: its BACKGROUND is white, and red is its border and its legend. A two-column
  `sign_color(sign, colour)` would flatten three different claims into one and would get the most
  recognisable sign in the country wrong — while carrying a federal citation. With the element as a
  column, `sign_element_color(yield_sign, background, $C)` returns `white`, and
  `sign_element_color(yield_sign, $E, red)` returns BOTH the legend and the border.

  This is the same structural lesson `water-share.adj` needed for its denominators, arriving
  independently in a different domain: WHEN A SOURCE STATES A VALUE RELATIVE TO SOMETHING, THAT
  SOMETHING IS A COLUMN. A share without its base is not a fact, and a colour without its element is
  not one either.

  SIBLING TO `sign-shape.adj`, WHICH DELIBERATELY LEFT THIS AXIS ALONE. That library's header records
  that colour "is a different axis and deserves its own table rather than being smuggled into a shape
  column", and this is the follow-through. The two tables quote THE SAME THREE SENTENCES for
  different facts, which is honest rather than duplicative: a sentence stating both a shape and a
  colour grounds a row in each relation. It is also NOT the same relation as `traffic-lights.adj`,
  which holds `traffic_light_meaning(color, meaning)` — the colour of a SIGNAL and what a driver must
  do. They share the atoms `red` and `yellow` and disagree about nothing, because they answer
  different questions about them.

  Provenance: STOP from Section 2B.05 and YIELD from 2B.08, both in Chapter 2B ("Regulatory Signs,
  Barricades, and Gates"); the warning default from Section 2C.03 in Chapter 2C ("Warning Signs And
  Object Markers"), which carries its own locator. Every quoted sentence was confirmed verbatim
  against the RAW HTML rather than merely against the extraction — the sibling library shipped a
  draft with "(see Figure 2B-1 )" before review caught that tag-stripping had inserted a space the
  manual does not contain, so the raw-HTML check is now the standard for any sentence containing a
  linked term or parenthetical.

  Honest abstention. `no_passing_zone_sign` gets no colour rows: its sentence specifies a pennant
  shape and says nothing about colour, and inferring "warning signs are yellow, so this one is
  yellow" would be unsound as well as ungrounded, because that warning sentence is itself defeasible.
  `regulatory_sign` as a class gets none either — the rectangular default says nothing about colour,
  and STOP and YIELD prove regulatory signs share no single scheme. The "WIDE" in YIELD's "wide red
  border" is a WIDTH rather than a colour and is not recorded; the border's colour is red, and how
  wide it is belongs to a dimensions relation this library does not hold. Retroreflectivity,
  fluorescent alternatives and the reserved colours coral and light blue are discussed in Chapter 2A,
  largely inside HTML TABLES that tag-stripping cannot quote byte-faithfully — a figure this library
  cannot cite byte-faithfully is one it does not state.

  New `sign-element-color.query.adj` and `facts_signelementcolor_e2e.rs` (6 tests: the direct lookup
  with its verbatim sentence, the YIELD-background-is-white test that pins the misconception the
  third column exists to prevent, both red parts of a YIELD sign with a negative that the background
  is not among them, the reverse lookup on a red background naming STOP alone, sentence-to-locator
  PAIRS for both chapters, and the two no-colour abstentions with a positive control). ALL THREE
  DEFECT CLASSES THIS SERIES HAS BEEN CAUGHT ON WERE APPLIED PREEMPTIVELY: the joint binding form
  throughout, positive controls on the abstention test, and locator PAIRS rather than a bare locator
  scan. All four negative assertions were mutation-tested and each fires under exactly its own
  mutation.

  TWO COVERAGE GAPS WERE CLOSED AFTER REVIEW, and the second is the reason mutation testing has to
  cover ROWS and not just assertions. Neither was a false-passing assertion — they were rows and
  properties that simply nothing guarded.

  (i) FOUR OF THE NINE ROWS were unpinned: mutating STOP's legend from white to black passed the
  entire suite. Those four are exactly the ones stated by the COMPRESSED constructions ("a white
  legend and border", "a black legend and border"), where one colour distributes over two nouns —
  the easiest rows in the table to misread. A new test pins both halves of each compressed phrase.

  (ii) THE YIELD SENTENCE WAS NEVER ASSERTED AT ALL, neither its text nor its locator, so two
  regressions passed the whole suite: retargeting its locator to Chapter 2C, and — worse —
  reinserting the spurious tag-stripping space in its figure reference, WHICH IS THE EXACT DEFECT THE
  SIBLING `sign-shape.adj` SHIPPED IN A DRAFT ONE SLICE EARLIER. A third sentence-to-locator pair
  closes both at once, because the needle is byte-exact including the parenthesis.

  All three regressions were verified to fail after the fix. New manifest objective
  `adj.socialstudies.k2.sign_element_color`.

