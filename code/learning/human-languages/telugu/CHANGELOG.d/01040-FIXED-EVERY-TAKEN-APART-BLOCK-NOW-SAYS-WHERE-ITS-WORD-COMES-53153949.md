## Fixed — every taken-apart block now says where its word comes from

Eight Telugu word lessons had a "taken apart" block that compared words across the family or described sourcing, and never said where the word came from (human-language-data `hasEmptyEtymologyBlock`, HL24 §9.4). Each block now says it, from DEDR and CDIAL in the moli-mandala/data files; no source text is copied. Empty etymology blocks 8 → 0.

- **New origin sentences (4).** TE-C06-dative-subject: *vaccu* is native (DEDR 5270). TE-C12-kutumbam: the sibling words are native, and DEDR 2783 files *celli* with *celi*, "female companion". TE-C15-neellu-biyyam: *nīḷḷu* is the native word of DEDR 3690; *biyyam* is in DEDR's appendix of words taken from Indo-Aryan (App. 60), under Sanskrit *bīja*, "seed" (CDIAL 9250). TE-C16-nelalu: the month names are Sanskrit, *caitra* named for the asterism Citrā (CDIAL 4915).
- **Rewordings a signal can read (4).** TE-C28-madhyahnam-widened calls *aparāhnam* a tatsama. TE-C92-hospital says "traced to English *hospital*". TE-C225-boppayi says *papaya* was borrowed with the fruit. TE-C293-pancayati says its route into Telugu is uncertain, since neither dictionary lists it.
- **Corrected.** TE-C15-neellu-biyyam called *annam* "likely Sanskrit-influenced"; it is Sanskrit *anna* (CDIAL 395).
- **Durations.** TE-C06-dative-subject computes above 270 s and was trimmed so it did not grow. No `max_seconds` was raised.
