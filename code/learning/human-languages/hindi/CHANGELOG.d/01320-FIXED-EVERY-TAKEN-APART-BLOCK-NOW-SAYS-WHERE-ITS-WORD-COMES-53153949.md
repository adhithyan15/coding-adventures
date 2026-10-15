## Fixed — every taken-apart block now says where its word comes from

Seven Hindi word lessons had a "taken apart" block or origin heading that took the phrase or the grammar apart and never said where the word came from (human-language-data `hasEmptyEtymologyBlock`, HL24 §9.4). Each block now says it, from Turner's CDIAL in the moli-mandala/data files; no source text is copied. Empty etymology blocks 7 → 0.

- **New origin sentences (3).** HI-C04-phir-milenge: *phir* ← *\*phirati*, "turns" (CDIAL 9078), *milenge* ← Sanskrit *milati* (CDIAL 10133), as the hook already said. HI-C30-dopahar-widened: *pahar* ← Sanskrit *prahara*, "a watch" (CDIAL 8900). HI-C93-karne-ke-liye: *karnā* ← Sanskrit *karoti* (CDIAL 2814).
- **Rewordings a signal can read (4).** HI-C37-chai says Hindi borrowed *chāy* from Persian. HI-C40-there and HI-C40-where say their older history is uncertain (CDIAL has no entry; 1605 records only *yahā̃*). HI-C91-aadmi-unchanged says *ādmī* is an Arabic word borrowed through Persian, as its hook does.
- **Durations.** HI-C37-chai computes above 270 s and was only reworded. No `max_seconds` was raised.
