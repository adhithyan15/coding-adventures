## Fixed — every taken-apart block now says where its word comes from

Seven Italian word lessons had a "taken apart" block that split the word into parts, or explained its grammar, and never said where it came from (human-language-data `hasEmptyEtymologyBlock`, HL24 §9.4). Each block now says it. Source: EtymDB 2.1 (Fourrier & Sagot 2020, CC BY-SA 4.0), read locally; every line is our own sentence. Empty etymology blocks 7 → 0.

- **New origin sentences (5).** IT-C01-buongiorno (*bonus*, *diurnum*), IT-C02-come-stai (*quōmodo*, *stāre*), IT-C26-terzo (*tertius*, from *trēs*), IT-C30-anche (*né* ← *nec*; the origin of *anche* is uncertain and EtymDB has none) and IT-C32-perche (*per* and *che* ← *quid*).
- **Rewordings a signal can read (2).** IT-C28-prima says "comes from Latin"; IT-C33-questo says *questo* "grew from" *eccu(m) istum*.
- **Hook.** IT-C30-anche's hook described the dialogue, not the word; it now gives *né* ← Latin *nec* and says *anche* is uncertain.
- **Durations.** IT-C32-perche computes above 270 s; its block was rewritten without growing. No `max_seconds` was raised.

