### Changed — Tamil pins for the one-stroke ca, nna, nnna, llla and nya

- **Evidence files.** `tests/script-inventories/tamil/letters/` has new
  digests for `0100-U-B9A`, `0180-U-BA3`, `0200-U-BA9`, `0240-U-BB4` and
  `0110-U-B9E`, because the records now have `penLifts: 0`. All five files
  now assert the same things:
  - the whole `strokeOrder`, with every step after the first starting
    "without lifting";
  - zero pen lifts;
  - the citation;
  - the phrases of the variation note, including the HP Labs India LipiTk
    4.0 share of one-stroke prototypes (88%, 93%, 94%, 97% and 86%).

  `0110-U-B9E` keeps its sound and role checks and now also pins the
  one-stroke `strokeOrderNote`. Before, `U-BA3`, `U-BA9` and `U-BB4` pinned
  only their digest, and the other two asserted one or three lifts.
- **Regenerated owner evidence.** The five script-owner-evidence digests
  were regenerated, because the record bytes changed. The owner
  declarations stay the same.
- **Regenerated curriculum outputs.** The filmstrip-geometry ledger, the
  Tamil figure hashes (five filmstrips), chapters 6, 7 and 112's book and
  narration hashes, and three lessons' modality records were regenerated.
  The lessons are TA-S02, TA-S03 and TA-S137. Their writing steps and
  pen-lift line changed, and TA-S02's wrap-up answer. The modality records
  were written by the package's own `generatedModalityOutputs`, writing only
  the files whose bytes changed, because the disk is too full for the
  CLI's staging copy.
- **Unchanged.** ங's record and pins, the curriculum digest, the lesson
  count, the reinforcement counts and the filmstrip target counts do not
  move, because no lesson was added, moved or retagged.
