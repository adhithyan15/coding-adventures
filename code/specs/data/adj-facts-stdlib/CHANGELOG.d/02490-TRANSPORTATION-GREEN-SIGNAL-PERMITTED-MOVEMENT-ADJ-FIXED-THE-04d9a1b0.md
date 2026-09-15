- `transportation/green-signal-permitted-movement.adj` (fixed) — THE CITATION WAS TRUNCATED, and is
  now the full sentence. Closes issue #13916.

  The `source` envelope stopped at "…or make a U-turn movement", omitting the clause the MUTCD
  sentence actually ends with: "except as such movement is modified by lane-use signs, turn
  prohibition signs, lane markings, roadway design, separate turn signal indications, or other
  traffic control devices." Every row was therefore presented as an UNCONDITIONAL permission, backed
  by evidence with its qualifier cut off — the exact defect `sign-shape.adj` and
  `red-signal-permitted-movement.adj` both warn against in their own headers: A CITATION HAS TO CARRY
  THE QUALIFIER IT IS BEING USED TO JUSTIFY. The rows are unchanged and were never wrong; green does
  permit those four movements. What was wrong was the evidence attached to them.

  THE ROOT CAUSE WAS INHERITED PROVENANCE, NOT CARELESS READING. The header used to say the span
  "reproduces, byte-for-byte, the SAME … span already quoted inside `traffic-lights.adj`'s own header
  — NO NEW WebFetch". The quote was COPIED FROM A SIBLING LIBRARY'S PROSE to save a fetch, so a
  truncation in that header propagated silently into a `source` envelope, where it read as verified.
  A CITATION INHERITED FROM ANOTHER LIBRARY'S HEADER IS NOT VERIFIED PROVENANCE — re-extract from the
  page every time; one fetch costs far less than a citation that says less than the standard does.
  The replacement sentence was re-extracted and verified CHARACTER-EXACTLY against the page's RAW
  HTML (326 characters, pure ASCII, no escaping required).

  THE EXCEPTION STAYS IN THE CITATION RATHER THAN THE ATOMS, and review corrected the reason. A first
  draft called it a "whole-permission qualifier … not any one movement", which the grammar does not
  support: the clause reads "except as SUCH MOVEMENT is modified", a singular distributive anaphor,
  and every device it names is movement-specific. If anything it is MORE per-movement than the red
  sibling's exception, which is FRONTED and conditions the whole grant before stating it — so the
  draft applied the scope rule in the direction the text least supports, justifying a placement it
  had not earned.

  The right reason is about what a qualifier DOES to a value rather than where it sits. Red's "after
  stopping" is CONSTITUTIVE: it changes what the permitted act IS, so a bare `turn_right` row would
  assert something red's sentence never says. Green's exception is DEFEASIBILITY: it can override any
  of the four movements without changing what any of them IS, and applies uniformly to all four, so
  it carries NO row-discriminating information — pushed into the atoms it would decorate every row
  identically while making none more accurate. CONSTITUTIVE QUALIFIERS GO IN THE ATOM; DEFEASIBILITY
  GOES IN THE CITATION. That is a sharper statement of the rule `sign-shape.adj` and
  `karst-process-zone.adj` were already following.

  The header's note on the red sibling was also narrowed. Both permissions are device-modifiable, and
  FOR THE MOVEMENTS BOTH SIGNALS PERMIT the difference is the stopping requirement — but red permits
  STRICTLY FEWER movements, allowing neither `straight_through` nor `u_turn` and confining its left
  turn to one-way-into-one-way. The draft's "the stopping requirement and nothing else" is refuted by
  the red table's own rows, which matters because this header invites the reader to consult it.

  TWO GUARDS ADDED, being the two whose absence let this survive. The test now (i) asserts the WHOLE
  citation, anchored on the `"source":"` key and closed by its terminating quote, and (ii) asserts
  ANSWER CARDINALITY of exactly four, applying the standing rule from #13917, since pinning rows
  proves nothing is missing but cannot prove nothing was invented.

  The first guard was itself strengthened after review. It originally pinned only the TRAILING clause,
  which narrowed the hole rather than closing it: appending fabricated text after the clause, or
  corrupting the sentence HEAD — swapping CIRCULAR GREEN for FLASHING RED and rewriting the
  enumeration — both still passed, because `contains` on a tail cannot see what precedes or follows
  it. Anchoring the whole sentence pins head, tail, punctuation, internal spacing and length at once.
  Verified by mutation: re-truncation, a dropped final period, an injected double space, an appended
  fabricated sentence, and a corrupted head all fail it; adding a `reverse` row fails the cardinality
  assertion.

  THE PROPAGATION SOURCE IS FIXED IN THE SAME CHANGE. `traffic-lights.adj`'s header still carried the
  truncated green quote — the artifact this library copied from. Its own `source` envelope is the RED
  sentence, so nothing it emitted was ever wrong, but a header quote is read as a citation even though
  it is not one, and leaving the loaded artifact in place while fixing the copy would repeat the very
  mistake this entry documents. Its quote is now complete, with a note recording that it propagated.

  Related: issue #13918 measures how far this generalises — 86 of 362 libraries (24%) carry an
  inherited-citation claim, and only 37 of 361 (10%) have any test asserting any span of their own
  citation.

