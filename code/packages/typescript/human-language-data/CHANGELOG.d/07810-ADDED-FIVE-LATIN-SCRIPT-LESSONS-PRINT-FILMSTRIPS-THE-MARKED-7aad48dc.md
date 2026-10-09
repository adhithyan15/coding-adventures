### Added — five Latin-script lessons print filmstrips, the marked letters drawn by analogy

- **5 lessons gain a strip** (848 -> 853 of 1,367 writing lessons; Latin-script
  tracks 16 -> 21): FR-W01-accents (é è ê), FR-W02-cedille (ç),
  FR-W03-trema (ï ë ü), GE-W02-umlauts (ä ö ü) and LA-W01-salve-delayed-copy
  (s a l v ē). v is cited to the Grundschrift-App; è ê ç ï ë ä ö and ē are
  drawn BY ANALOGY with the cited ü, acute and tilde (base letter, then the
  mark last), not separately sourced, and say so.
- **Inventory.** `latin.json` gives v, m and R components, an order, a lift
  count and a Grundschrift source, and adds eight rows (63 -> 71 letters) for
  è ê ë ï ä ö ē ç whose citation, note and variation say "by analogy, not
  separately sourced". Its `notes` state the analogy rule.
- **Still undrawn:** FR-C10-oe (œ is a ligature, outside the analogy);
  LA-W01-salve-guided-copy (a slash is not a list separator);
  ES-C03-como-acento and ES-W03-question-span (`?` and `,` have no ductus,
  and a word with punctuation inside it is refused, though ¿ is cited);
  ES-W02-enye-formas and GE-W03-capitalization (over MAX_SEQUENCE_PIECES).
- **Pins.** `tests/filmstrip-target-counts`: french 3 -> 6, german 3 -> 4,
  latin 0 -> 1. `latin.evidence.ts` pins 34 cited and 8 analogy rows.
  `the-real-corpus` lists the 21 Latin-script targets.
- **Regenerated:** figures and their hash manifests, and the French, German
  and Latin chapters that place the new strips. Lesson prose, narration and
  modality are unchanged. HL06 records the design and its result.
