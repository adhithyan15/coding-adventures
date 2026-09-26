### Changed — Gujarati runway pins move with its anchor words (HL-C443)

Gujarati's opening runways (chapters 1 and 3-7) now open with thirteen anchor
words. Each position pin moves, with a comment. Every window those pins assert
still closes.

- **Position pins.** The R4 bridges A-D move 111-133 → 124-146. The doorway
  checkpoint moves 134 → 147, and its distances grow by 9, still inside R4. The
  doorway R3 distances grow by 9, still inside R3. Runway B's indices each
  move +13, with unchanged distances. The four bridge case files and the
  doorway file are renamed so their names carry the new positions.
- **Missed-window counts.** The whole-track count goes 874 → 900. The anchor
  words' own atoms add 27, and namaste's R1 closes (-1). No pre-existing
  window is lost.
- **Opening spine and chapter sizes:** 1: 12, 3: 13, 4-6: 7, 7: 8.
- **Lesson-content budget:** 519 → 532.
- **Letter-anchoring ceiling:** Gujarati [34, 5, 0] → [4, 4, 0].

No library code changed.
