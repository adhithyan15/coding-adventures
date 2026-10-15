## Fixed — every taken-apart block now says where its word comes from

Six Malayalam word lessons had a "taken apart" block that took the word apart or compared it with Tamil and never said where it came from (human-language-data `hasEmptyEtymologyBlock`, HL24 §9.4). Each block now says it, from DEDR and CDIAL in the moli-mandala/data files; no source text is copied. Empty etymology blocks 6 → 0.

- **New origin sentences (4).** ML-C07-numbers-6-10: *ompatu* is native (DEDR 1025), with its make-up hedged as in the Tamil lesson. ML-C12-kudumbam: DEDR has no entry for the four sibling words; *cēṭṭan* and *cēcci* are usually linked to Prakrit *jeṭṭha*, "eldest" (CDIAL 5286), and the source of *aniyan* is uncertain. ML-C20-pathinonnu-irupathu: *iru* and *pattŭ* are native (DEDR 474, 3918). ML-C75-eppol: *pōḷ* is the native word for "time" (DEDR 4559), as ML-C50-now already says.
- **Rewordings a signal can read (2).** ML-C34-edukkuka names DEDR 851, where the etymological dictionary files *eṭu*. ML-C322-porotta says "comes from Hindustani *parāṭhā*".
- **Durations.** ML-C34-edukkuka and ML-C75-eppol compute above 270 s; ML-C75-eppol dropped a redundant clause so neither grew. No `max_seconds` was raised.
