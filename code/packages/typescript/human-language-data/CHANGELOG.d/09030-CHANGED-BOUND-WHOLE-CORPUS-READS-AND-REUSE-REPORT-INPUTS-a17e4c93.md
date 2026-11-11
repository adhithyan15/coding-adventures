### Changed — bound whole-corpus reads and reuse report inputs on Windows

- Add `loadEverythingAsync`, an awaitable equivalent of the complete loader
  that uses the existing 16-read bounded lesson path while retaining the same
  sorted order, source-byte cache key, metadata guards, and result shape.
- Let the gentle-ramp compatibility derivation accept already-loaded canonical
  inputs for its second independent full-report calculation. The historical
  no-argument loader still checks the retired snapshot path and reads all
  canonical owners itself.
- Use registry-only loading in the shared progression identity test instead
  of constructing a 63,000-lesson corpus that the assertion never reads.
- Share one bounded lesson load between the read-only literal-markup gate and
  its planted non-vacuity control. The complete rendered-book and source
  assertions remain in place.
- Scope the Persian writing-ladder assertion to all Persian lessons and its
  canonical curriculum, retaining the shared assessment policy and spine;
  unrelated tracks no longer consume its 30-second case budget.
- Scope six Kannada, Tamil, and Telugu A1 exam assertions to their complete
  canonical track lessons. The exam functions already filter by language, so
  every probe, coverage count, and continuity assertion remains unchanged.
- Use a shared bounded source load for the read-only lost-script gate and its
  planted damage control, and for both committed metalanguage measurements.
  Their complete source and rendered-book scans are retained.
- Make the completion-plan runner load only the registry, lessons, curricula,
  and spine it consumes. Focused validation still shows high Windows I/O cost;
  the complete suite remains the acceptance gate for this optimization.
- Add an anchored, bounded reader for every direct curriculum-membership owner
  and an awaitable curriculum loader. Fixture parity, corruption, and
  redirected-directory tests cover the new path. The track-progress command
  uses it. Its read-only card, check, and report tests now share one complete
  real-corpus derivation while invoking the same final mode gate as the CLI;
  the unchanged full Windows suite remains the release acceptance gate.
- Let the direct completion-plan command await the same bounded canonical
  lesson and membership reads while retaining the synchronous library API and
  byte-identical planning tail. The clean-corpus equivalence test remains the
  gate for this path.
- Share a bounded lesson read across the two verb baseline assertions, and
  overlap the independent committed-modality read with the shared lesson read.
  Both still compare the complete canonical reports; neither test budget nor
  assertion changes.
- Derive the async committed-modality narration identity index through the
  pinned generated-hash batch reader, requesting narration owners only. Keep
  exact registry language closure, chapter validation, and global lesson-ID
  collision checks; the synchronous index remains unchanged.
- Keep the 30-second default, explicit per-case timeouts, every corpus
  assertion, and the 85% line-coverage threshold unchanged. Full Windows
  coverage and final-head CI remain required before release.
