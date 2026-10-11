### Changed — five Tamil letters lose their pen lifts: அ, ஆ, எ, ங and ஷ

- **Records.** `tamil.d/letters` for அ, ஆ, எ, ங and ஷ: `penLifts` 0 (was 1,
  1, 1, 1 and 3), one `strokeOrder` step per movement of the new one-stroke
  ductus (6, 7, 4, 6 and 6), a new `strokeOrderNote`, the LipiTk class added
  to each citation, and the same variation note as the ductus (counts and
  shares from HP Labs India's LipiTk 4.0 Tamil recognizer, and where they
  override Radhakrishnan's Frames 4, 5 and 2 or Narale's chart). எ's and ங's
  components and notes now describe the native writers' order. `_meta.json`
  lists the five with the one-stroke letters and drops the "still due
  review" list: ஊ is now the only Tamil letter with a lift.
- **Lessons.** TA-S119 (அ), TA-S120 (ஆ), TA-S130 (ங) and TA-S140 (ஷ) show the
  new steps and "Pen lifts: 0. The pen never leaves the paper." (were 1, 1, 1
  and 3), with a line on the LipiTk counts; TA-S130 says Frame 2 draws the
  upright first. TA-S112 (எ) follows the strip and needs no prose change.
- **Pins.** The five Tamil inventory-evidence digests, each now asserting
  the lift count, the step count, key steps, the LipiTk citation and the
  override phrase (அ and ஆ had a digest only). Regenerated: the five
  script-owner-evidence records; nine Tamil filmstrips and their figure
  hashes (the five letters, and the words ஆம், என், நீங்கள், ஆனால்); Tamil
  chapters 15, 16, 110 and 112 with their book and narration hashes; four
  lesson-modality records. Filmstrip target counts do not move.
- **Python.** `data/scripts/test_tamil_sharded_consumers.py` pinned 25 Tamil
  letters while the inventory has 30. It now reads the expected letters and
  marks from the shard files themselves and checks the drizzle author loads
  exactly those, in shard order.
