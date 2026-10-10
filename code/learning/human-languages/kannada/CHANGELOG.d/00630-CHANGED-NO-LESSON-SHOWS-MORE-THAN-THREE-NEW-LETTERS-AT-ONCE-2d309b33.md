## Changed — no lesson shows more than three new letters at once

The script-ramp report (`ramp.script`, policy `maxNewGlyphsPerLesson: 3`)
counts every target-script shape the first time a lesson body shows it.
It listed four Kannada lessons; it now lists none. Script closure stays 0 and
the exposure-exempted glyph count is unchanged.

| lesson | before | after |
|---|---|---|
| KA-C01-namaskara | 7 ನ ಮ ಸ ್ ಕ ಾ ರ | 3 ನ ಮ ರ |
| KA-C01-dhanyavada | 4 ಧ ಯ ವ ದ | 3 ಧ ವ ದ |
| KA-C02-santosha | 4 ಂ ತ ೋ ಷ | 0 |
| KA-S122-letter-ca | 6 ಖ ಜ ಠ ಢ ಘ ಫ | 3 ಖ ಜ ಠ |

- **KA-C01-namaskara** reads three of its letters, ನ ಮ ರ, traces ನ, and says
  the whole word waits for the end of the chapter. **KA-C01-dhanyavada** reads
  ಧ beside plain ದ, and ವ. Both headings are romanized.
- The whole ನಮಸ್ಕಾರ and ಧನ್ಯವಾದ first appear in KA-S01-letter-na, which now
  says which three of their shapes (ಕ, ಯ, ಾ) the reader need not read yet.
  KA-C01-illa and KA-C01-sari each take one more shape (3 and 2).
- **KA-C02-santosha** stays spoken and says its written form waits a few
  chapters; KA-S05-letter-ta (chapter 5) already spells it (now 3 new).
- **KA-S122-letter-ca**'s twelve-month preview was split: ಚ's lesson keeps
  ಚೆನ್ನಾಗಿ and the first three months, and KA-S124-letter-pa previews the two
  months that carry ಪ plus three more. Every month is still on a script page
  before KA-C16-tingalugalu, which is unchanged.
- Regenerated book chapters 1, 2, 13 and 15, narration, modality and hashes.
