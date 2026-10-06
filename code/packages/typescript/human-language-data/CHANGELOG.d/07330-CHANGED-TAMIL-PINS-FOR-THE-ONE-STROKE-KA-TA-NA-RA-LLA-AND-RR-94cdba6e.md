### Changed — Tamil pins for the one-stroke ka, ta, na, ra, lla and rra

- **Evidence files.** `tests/script-inventories/tamil/letters/` has new
  digests for `0080-U-B95`, `0140-U-BA4`, `0190-U-BA8`, `0210-U-BB0`,
  `0230-U-BB3` and `0250-U-BB1`, because the records now have `penLifts: 0`.
  All six files now assert the same things:
  - the whole `strokeOrder`, with every step after the first starting
    "without lifting";
  - zero pen lifts;
  - the citation;
  - the phrases of the variation note, including the HP Labs India LipiTk
    4.0 share of one-stroke prototypes (88%, 88%, 85%, 90%, 90% and 99%).

  Before, `U-B95` and `U-BB1` pinned only their digest. The other four
  asserted two or three lifts and "pen-down runs".
- **Regenerated owner evidence.** The six script-owner-evidence digests were
  regenerated, because the record bytes changed. The owner declarations
  stay the same.
- **Regenerated curriculum outputs.** The filmstrip-geometry ledger, the
  Tamil figure hashes (six filmstrips), chapters 8, 10, 13 and 110's book
  and narration hashes, and four lessons' modality records were all
  regenerated. The lessons are TA-S04, TA-S05, TA-S06 and TA-S131. Their
  writing steps, their pen-lift line and two wrap-up answers changed.
- **Unchanged.** The curriculum digest, the lesson count, the reinforcement
  counts and the filmstrip target counts do not move, because no lesson was
  added, moved or retagged.
