### Fixed - sight cues must be anchored to something on the page

- A sight cue is a pointing expression, and it now only counts as one when
  there is something on the page for it to point at. Each phrase in
  `SIGHT_CUE_RULES` declares its own anchor: `shown below` / `written above`
  carry their own deixis and always count; `the table` / `the chart` are
  definite references to a whole artifact and count only when the lesson
  actually contains a table or an image; `look at` / `see the` / `column` are
  instructions and count unless the occurrence is a quoted gloss or takes a
  wh-clause complement ("Look at **what** English built on that jar").
- Measured over the whole corpus, 96 lessons were `sight` on a prose cue alone
  — no script block, no unspeakable table, not `type: writing`. 25 of them were
  pointing at nothing on the page and are now `voice`: corpus `sight` 597 → 572
  and `voice` 2969 → 2994, across 12 tracks. Every one of the 25 was read by
  hand before being accepted.
- The bias is unchanged and deliberate: a lesson wrongly called drivable sends
  a driver to a page they cannot read, so a cue is dropped only where the
  document's own structure or the grammar of the sentence settles it. `column`
  is still never dropped on structural grounds — an author may call any aligned
  display a column, and `ES-C56-cion` does exactly that with no Markdown table
  in the lesson. Figurative uses with a concrete object ("Look at your
  collarbone") are still counted, and the tests record that as a decision.
- A block is now judged against its whole lesson rather than against its own
  text, so a paragraph saying "look at the table" still counts when the table
  itself lives in the next block.
- Cue patterns are compiled once at module load and matched case-insensitively
  against the original text rather than a lowercased copy, because
  `toLowerCase()` is not length-preserving for every script in this corpus and
  the offsets the anchoring tests use have to index the real string.
- The gloss rule treats only `"` and the curly pair as quote marks. An earlier
  draft also accepted `'`, which let any two contractions within sixty
  characters forge a gloss and swallow the cue inside it — "Don't look at the
  chart's third bar" lost `look at`, and `ES-W00-hola-observe` lost the
  narration line telling a listener the lesson points at something written down.
- `hasPageArtifact` recognises HTML `<img>`/`<table>`/`<figure>` and
  reference-style images as well as pipe tables, since the `artifact` anchor's
  safety rests on that test being complete; and its alt-text scan is bounded and
  atomic, because the unbounded form walked to end-of-text from every `!` in the
  body (49 s on 400 KB of `![`, now ~70 ms, with a test pinning the class).

