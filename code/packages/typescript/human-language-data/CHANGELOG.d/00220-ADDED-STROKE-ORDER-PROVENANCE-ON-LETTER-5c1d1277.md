### Added — stroke-order provenance on `Letter`

- Add `StrokeOrderSource` and two optional `Letter` fields, `penLifts` and
  `strokeOrderSource`. A `strokeOrder` list names a letter's **parts** in writing
  order; it has never counted **pen-down runs**, but a numbered list of three
  reads to a learner as three strokes and two lifts. Tamil ம is the counter-
  example that forced the distinction: its prose listed three parts while the
  authored, font-checked pen path in Language Ladder's `strokes.ts` shows one
  unbroken stroke with zero lifts. `penLifts` records that number only where a
  verified path supports it — absent means *not verified*, never *none* — and
  `strokeOrderSource` carries the citation, URL, and the honest `variation` note
  for scripts (every Indic script, Arabic, Hebrew) that have no national
  standard. Both are optional, so every existing script file still validates.
- Document the parts-vs-strokes rule on `strokeOrder` itself, where the next
  author writing one will actually read it.

