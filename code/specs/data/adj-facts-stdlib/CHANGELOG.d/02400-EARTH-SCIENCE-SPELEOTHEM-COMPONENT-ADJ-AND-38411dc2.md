- `earth-science/speleothem-component.adj` and
  `earth-science/compound-speleothem-surface.adj` (new, shipped together) — the FIFTH and SIXTH
  cave/karst libraries, which exist to CLOSE A RECORDED ABSTENTION and to show why recording it was
  right.

  `speleothem-growth-surface.adj` refused to give `column` a growth surface, and said why in the
  source's own words: "Columns are not stalactites nor are they stalagmites; they are both,
  together." A column is produced by two speleothems JOINING rather than by growing from one
  surface, so it has no single growth surface to bind; that header states plainly that recording
  `cave_ceiling` or `cave_floor` "would be false, and recording both would misrepresent the relation
  this table holds."

  That abstention was correct, and was never meant to be permanent. "What is a column made of?" has
  a perfectly good grounded answer — it simply is not a question about growth surfaces, and it
  needed a DIFFERENT RELATION. The table supplies it: `speleothem_component(compound_speleothem,
  component)`, with `column` → `stalactite` and `column` → `stalagmite`, neither outranking the
  other. This is the same move `civics/congress-chamber.adj` made for
  `government-branch-member.adj`'s abstention on the two chambers: when a table declines, the fix is
  a new relation shaped to the question, never a loosened row in the old one.

  The rule then recovers what the growth-surface table declined:
  `compound_speleothem_surface(compound_speleothem, surface)` chains the component relation with the
  growth-surface relation, deriving `column` → `cave_ceiling` AND `column` → `cave_floor`. TWO
  ANSWERS ARE CORRECT HERE WHERE TWO ROWS WOULD HAVE BEEN A MISREPRESENTATION THERE, because the
  relations are not the same relation: one means "the surface this speleothem grows from", the other
  "the surfaces reached by this speleothem's components". That distinction is the whole argument for
  abstaining rather than fudging. A table that had quietly bound both surfaces would have been
  approximately useful and precisely wrong, and nothing downstream could have recovered the
  difference. Because it declined, the honest answer remained available — as a composition, in its
  own name, with its provenance intact.

  Provenance: the table's `source` envelope carries the formation sentence ("When a stalagmite grows
  together with its counterpart feeder stalactite, a new speleothem is formed: a column or pillar.")
  and its `cites` corroboration the identity sentence ("Columns are not stalactites nor are they
  stalagmites; they are both, together."). The second is doing real work rather than decorating: on
  its own the first could be read as a column being a stalagmite that reached the ceiling — ONE
  component — and "they are both, together" is what makes two rows correct. The rule's envelope
  cites the identity sentence, and each derived answer additionally carries its two premises'
  citations, so nothing it relied on goes uncited. All from the U.S. National Park Service's
  "Speleothems" article, `trust authoritative`, CONTENT-verified.

  A LIMITATION, RECORDED RATHER THAN LEFT TO BE DISCOVERED: provenance in this stdlib is attached at
  TABLE level — a table's `source` envelope and `cites` corroborations cover EVERY row, not the row
  that matched — so the two derived answers are provenanced IDENTICALLY. The `cave_floor` answer
  carries the same citation list as the `cave_ceiling` one, with the stalactite sentence as its
  primary citation, though that row played no part in deriving it. This is worth recording because
  the tempting claim is the opposite: an earlier draft of this library's test asserted per-answer
  attribution (ceiling→stalactite, floor→stalagmite) and the assertions PASSED, because both
  sentences appear in the output of any successful use of the growth-surface table — the test proved
  nothing while reading as though it proved attribution. A library about provenance honesty does not
  get to overclaim its own provenance. What holds is that both premises are cited; what does not
  hold is that each answer is cited with the sentence that produced it. The shipped test pins the
  real behaviour with a floor-only query, which under row-level provenance would carry only the
  stalagmite sentence and today carries the stalactite one too, so it will fail loudly if row-level
  provenance ever lands.

  Honest abstention. `pillar` is NOT a row: the source offers it as an alternative NAME for the same
  speleothem ("a column or pillar"), not as a different thing with components of its own, so tabling
  it would double-count one speleothem as two; a synonym belongs in an alt-name relation, the shape
  `astronomy/space-rock-alt-name.adj` already uses. `soda_straw` is NOT a row: the source says every
  stalactite begins its growth as a hollow soda straw, which is a developmental stage of ONE
  speleothem rather than two joining — and, checked directly against the page, it is a stage whose
  SUCCESSOR the source never describes, since it never states how a soda straw becomes a thicker
  stalactite. A `stage → next_stage` table would have to invent that step, so none was built. The
  rule additionally abstains on `helictite`, absent from BOTH premises (the growth-surface table
  declined it because the source gives three surfaces with a frequency hedge no two-column row can
  carry) — a derived relation cannot repair a premise it inherits, and composition propagates
  abstention exactly as it propagates provenance.

  New `speleothem-component.query.adj` and `compound-speleothem-surface.query.adj`; new
  `facts_speleothemcomponent_e2e.rs` (5 tests: both components with the formation sentence and
  citation, the identity sentence reaching the reader as a corroboration, backward recall, the
  synonym/non-compound abstentions, and the developmental-stage abstention) and
  `facts_compoundspeleothemsurface_e2e.rs` (5 tests: both derived surfaces, both premises' citations
  surviving composition WITH the table-level-provenance limitation pinned, the proof trace naming both premise goals, backward recall, and
  abstention propagating from the premises). New manifest objectives
  `adj.science.3to5.speleothem_component` (recall) and
  `adj.science.3to5.compound_speleothem_surface` (infer, with both premises as prerequisites).

