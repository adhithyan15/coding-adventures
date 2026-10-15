## Fixed — every taken-apart block now says where its word comes from

Eight Kannada word lessons had a "taken apart" block that compared the word with Tamil or split it into parts and never said where it came from (human-language-data `hasEmptyEtymologyBlock`, HL24 §9.4). Each block now says it, from DEDR and CDIAL in the moli-mandala/data files; no source text is copied. Empty etymology blocks 8 → 0.

- **New origin sentences (4).** KA-C12-kutumba: *appa*, *amma* (DEDR 156, 183) and the four sibling words (DEDR 131, 3085, 23, 3015) are native. KA-C15-niiru-akki: *nīru* (DEDR 3690) and *akki* (DEDR 215) are native. KA-C31-shubha-madhyahna: both words are Sanskrit tatsamas (CDIAL 12532, 9818). KA-C37-baayi: *bāyi* is native (DEDR 5352).
- **Rewordings a signal can read (4).** KA-C32-hoogu names the root (DEDR 4572, which lists Old Kannada *pōgu*). KA-C34-keelu cites DEDR 2017 for the root with both senses. KA-C95-oh and KA-C300-kadambari say "comes from" where they already gave the origin.
- **Corrected.** KA-C15-niiru-akki called *anna* "likely Sanskrit-influenced". It is Sanskrit *anna*, "food, cooked rice" (CDIAL 395), and the block now says so.
- **Durations.** KA-C31-shubha-madhyahna, KA-C32-hoogu and KA-C34-keelu compute above 270 s; each was trimmed so it did not grow. No `max_seconds` was raised.
