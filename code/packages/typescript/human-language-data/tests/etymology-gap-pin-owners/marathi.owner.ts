import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "marathi",
  withoutHook: 0,
  withoutVisible: 0,
  rationale: [
    "The 1,163 / 1,116 baseline fell to 0 / 0. Every Marathi word lesson now has an etymology hook and states its origin: 1,116 gained taken-apart blocks and 58 lessons whose text already named an origin gained a hook, with a corrected or added sentence where needed.",
    "Inherited words and Sanskrit loans follow Turner's CDIAL, the Marathi form checked against the entry's rows, with its probability, query, Dravidian, Munda, and Austroasiatic notes preserved; the few Dravidian contact words cite DEDR; Arabic loans name their root in Lane's Lexicon; Persian loans are hedged as usually traced; English and Portuguese loans name the donor. About 137 origins are explicitly not settled after a search of CDIAL and DEDR by form and by gloss. Older passages that contradicted CDIAL were corrected (मी is from mayā, not aham), and raised max_seconds values equal their computed length, all below 300 seconds.",
  ],
} as const satisfies EtymologyGapPinOwner;
