## Fixed — every taken-apart block now says where its word comes from

Twelve Tamil word lessons had a "taken apart" block that split the word or phrase into parts, or compared it with its sisters, and never said where the word came from (human-language-data `hasEmptyEtymologyBlock`, HL24 §9.4). Each block now says it, from DEDR or CDIAL, read in the moli-mandala/data files; no source text is copied. Empty etymology blocks 12 → 0.

- **New origin sentences (6).** TA-C01-aam (*ām* worn down from *ākum*, DEDR 333), TA-C02-en-peyar (DEDR 5160, 4410), TA-C02-ungal-peyar-enna (DEDR 3688, 4410, 5151), TA-C04-poy-varugiren (*pō* DEDR 4572, *varu* DEDR 5270), TA-C07-numbers-6-10 (*oṉpatu*, DEDR 1025) and TA-C31-matiya-vanakkam (*matiya* ← Sanskrit *madhya*, CDIAL 9804; *vaṇakkam* native, DEDR 5236, as TA-C28 and TA-C30 already say).
- **Rewordings a signal can read (6).** Blocks that already gave the origin, or named "the Dravidian dictionary", now cite the entry: TA-C01-nandri (DEDR 3610), TA-C33-ezhutu (DEDR 853), TA-C34-pidi (DEDR 4148), TA-C37-nanban (DEDR 3588), TA-C38-udambu (DEDR 586). TA-C05-naan-tamizh-pesugiren says "disputed" for "debated" and cites DEDR 3080.
- **Hedged.** TA-C07-numbers-6-10 said nine "grew from one plus ten". DEDR 1025 compares *oṉpatu* with both "one" (990) and the older word for nine (3532), so the block now says it is usually read that way and that its make-up is uncertain.
- **Hook.** TA-C31-matiya-vanakkam's hook described its sources, not its word; it now gives the origin of both words.
- **Durations.** TA-C01-nandri, TA-C33-ezhutu, TA-C34-pidi, TA-C37-nanban and TA-C38-udambu compute above 270 s; each was reworded or trimmed so it did not grow. No `max_seconds` was raised.
