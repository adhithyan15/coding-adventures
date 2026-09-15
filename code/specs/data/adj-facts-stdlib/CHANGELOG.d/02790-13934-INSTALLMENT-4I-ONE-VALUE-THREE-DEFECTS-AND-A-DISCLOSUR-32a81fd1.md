- **#13934 installment 4i: one value, three defects, and a disclosure the header had already
  promised.** 1 value line, 1 `cites` line and 40 comment lines in **1** `.adj` file (43 added
  lines; the remaining one is the `trust consensus` line re-emitted unchanged, having lost its
  closing brace to the new `cites`) — `geometry/shape-composition` — plus one e2e test file. **Six
  mutations redden.**

  This is the sixth and last of 4g's shape: a real page sentence with other material welded on.
  `shape-composition:57` carried THREE sentences in one `source` under ONE `locator`, and measured
  against the pages, each was a different kind of thing:

  | sentence | where it actually lives | verdict |
  |---|---|---|
  | Triangulation | `Triangulation.html` — the cited page | a real span, 1 match |
  | polygon diagonal | **`PolygonDiagonal.html`** — a different page | one word altered |
  | quadrilateral | **neither page** | authored |

  **The header had already promised the disclosure the field never made.** It said the rule's
  `source` "states plainly which part is the verbatim MathWorld quote and which part is the
  elementary reasoning step built on top of it" — and the field marked nothing at all. It also
  called both definitions "quoted verbatim from MathWorld in the rule's own `source`", which was
  false of the diagonal sentence twice over.

  ### The one-word defect

  The page says *"A **polygon** diagonal is a line segment connecting two nonadjacent polygon
  vertices of a polygon."* The field said *"A **polygonal** diagonal…"* — the page's own name for
  the concept, changed, inside quotation marks. And that sentence is prose from a page the `locator`
  did not name, so even corrected it was cited to the wrong document.

  It is now a `cites` carrying its **own** locator, which the grammar requires. Naming the right
  page decides nothing about #14111's canonical-vs-fetchable question: the sentence is simply on the
  page it is on. `shape-composition` is the first `rule` in this package to use `cites`; the
  annotation production is shared with `table`, so it was legal all along.

  ### Two claim strengths, kept apart

  The diagonal sentence **is byte-exact against the page's tag-stripped text** — which is what this
  package's contiguity contract has always measured. Against the page's PROSE neither sentence is
  byte-exact: both are broken by `<a>` tags, and the triangulation sentence is wrapped across a
  newline as well. Both sentences do occur byte-exact four times each in their page's raw HTML,
  inside the `<meta>` description attributes — which is why naming the text being measured is not
  pedantry here. The imprecision ran in the self-deprecating direction, but #13934's subject is
  precision about claim strength. The triangulation sentence is verbatim only under whitespace
  collapse, because MathWorld wraps it across a newline — the weaker claim installment 4h learned to
  state rather than blur. Both are in the header, labelled.

  ### A population that grew

  Contiguity **58 → 57**, and the arithmetic is not a simple subtraction: splitting one welded
  citation into a `source` plus a `cites` **adds a value to the population** (563 → 564), because a
  corroboration is a citation in its own right and the sweep judges it separately. Both of this
  file's values are now verbatim. Diffed as SETS, member for member.

  ### Mutants

  Six, one per thing the repair asserts: WELD (restore the three-sentence string), WORD (put
  `polygonal` back — the original defect), PAGE (point the corroboration's locator back at the wrong
  page, text untouched), AUTHORED (weld the invented sentence back on), and RULE-LOCATOR (repoint
  the rule's OWN locator, text untouched), EXTRA-CITES — append a second `cites` carrying the exact
  authored sentence this installment deleted, under another page's locator.

  **That last one is the strongest finding of the series, and it is the reviewer's.** Every pin was
  an open-ended `contains`, so nothing bounded the `corroborations` array: the defect class this
  installment repaired could be RE-ADDED alongside the repair and ship green. All three tests passed
  against it. The corroboration pin now CLOSES the array (`...}]`), which kills the attack in both
  orderings because it is anchored at both ends. The pin therefore asserts something stronger than
  before — that the rule cites this and nothing else — so a future genuine corroboration will redden
  it and require a deliberate test edit. That is the right trade for a provenance pin. **The PAGE
  mutant is why text and locator are pinned together**: a corroboration with pinned text and an
  unpinned page could drift back to citing the wrong document silently, which is half of what this
  installment repaired.

  Security review then turned that argument on the rest of the test. The rule's own citation and the
  underlying fact's were both asserted UNJOINED — `contains("kind":"rule") &&
  contains("…Triangulation.html")`, two independent probes over the whole output, satisfiable by an
  occurrence in any citation from any imported file. Nothing else in the import set names either
  page today, so a locator-only mutation did still redden; the protection was incidental rather than
  deliberate. Both are now joined the way the corroboration pin is, and RULE-LOCATOR is written down
  so it stays checked.

  The harness caught one of its own mutants being vacuous first. `WORD` used `replace(..., 1)`, and
  4i's new header prose QUOTES the string it was searching for while explaining the defect — so the
  mutation edited a comment, no pin could fire, and it reported SURVIVED rather than passing. A
  mutant that does not change the artifact proves nothing about the pins.

