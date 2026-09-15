- `chemistry/meniscus-reading-point.adj` (new) — a `table` naming the two basic shapes a
  liquid's meniscus can take inside a laboratory measuring vessel (concave, convex) and which
  point of its curve (lowest, highest) is actually used to take the reading, quoted verbatim
  from NIST's "Good Measurement Practice for Method of Reading a Meniscus" (GMP 3, NIST
  Interagency Report NIST.IR.7383-2019; WebFetch-verified against the raw document text before
  writing this file, a new source never before cited in this stdlib):
  `meniscus_reading_point(meniscus_shape, reading_point)`, concave → lowest_point, convex →
  highest_point. This is the SECOND instance of the "observation and measurement" Major Gap
  (ADJ-STDLIB-COVERAGE.md §5.1/§5.2), after `chemistry/measuring-tool-si-unit.adj`, and the
  first to ground it with a fresh source rather than composing two already-shipped tables —
  composition was confirmed exhaustively dry for this gap two sessions prior. Distinct from the
  already-shipped tool→quantity and tool→SI-unit tables: this covers the actual reading
  TECHNIQUE (which point of a curved liquid surface to read), a real, named source of
  systematic measurement error the same NIST document later quantifies in its own
  "Uncertainty and Error Analysis in Meniscus Readings" section. Runs the relation BACKWARD as
  a genuine recall (binding a reading point recalls its meniscus shape). Honest abstention on
  `flat` (the cited document discusses exactly two meniscus shapes, concave and convex, and
  names no third shape). The document's own named example of a convex-meniscus liquid,
  mercury, is deliberately kept in the header's prose and in the table's `cites` span rather
  than promoted to an unstated new column, since the source's own two topic sentences key the
  reading rule on the SHAPE, not on a specific liquid. New e2e test
  `facts_meniscusreadingpoint_e2e.rs`; new manifest objective
  `adj.science.6to8.meniscus_reading_point`.

