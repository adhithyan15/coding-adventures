### Changed — Tamil pins for the one-stroke i, ai, o and oo and the two-stroke uu

- **Evidence files.** `tests/script-inventories/tamil/letters/` has new
  digests for `0030-U-B87`, `0055-U-B90`, `0060-U-B92`, `0065-U-B93` and
  `0050-U-B8A`, because the records now have `penLifts: 0` (ஊ: 1). All
  five files now assert:
  - the whole `strokeOrder`, with every step after the first starting
    "without lifting" (ஊ's fourth step starts ள after its one lift);
  - the pen-lift count;
  - the citation;
  - the phrases of the variation note, including the HP Labs India LipiTk
    4.0 share of one-stroke prototypes (100%, 99%, 99% and 97%) or, for ஊ,
    of two-stroke prototypes (93%).

  `U-B90`, `U-B92`, `U-B93` and `U-B8A` keep their sound and role checks;
  `U-B8A` also pins its two-stroke `strokeOrderNote`. Before, `U-B87`
  pinned only its digest, and the other four asserted one, three or four
  lifts.
- **Regenerated owner evidence.** The five script-owner-evidence digests
  were regenerated, because the record bytes changed. The owner
  declarations stay the same.
- **Regenerated curriculum outputs.** The filmstrip-geometry ledger, the
  Tamil figure hashes (five filmstrips), chapters 109, 111 and 112's book
  and narration hashes, and four lessons' modality records were
  regenerated. The lessons are TA-S127, TA-S134, TA-S138 and TA-S139; their
  writing steps and pen-lift lines changed. The modality records were
  written by the package's own `generatedModalityOutputs`, writing only the
  files whose bytes changed, because the disk is too full for the CLI's
  staging copy.
- **Unchanged.** எ's record and pins, the curriculum digest, the lesson
  count, the reinforcement counts and the filmstrip target counts do not
  move, because no lesson was added, moved or retagged.
