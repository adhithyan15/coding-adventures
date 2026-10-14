import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "bengali",
  withoutHook: 0,
  withoutVisible: 0,
  rationale: [
    "The 1,136 / 1,144 baseline fell to 0 / 0. Every Bengali word lesson now has an etymology hook and states its origin: 1,144 gained a taken-apart block or an origin sentence in an existing section, 23 lessons whose text already named an origin gained a hook, and the three empty taken-apart blocks now name a source.",
    "Inherited words and Sanskrit loans follow Turner's CDIAL, the Bengali form checked against the entry's rows (Old and Middle Bengali, Assamese, or Oriya named where no Bengali row exists), with its probability, query, Dravidian, Munda, and Austroasiatic notes preserved and tatsama told from tadbhava by sound change; Arabic loans name their root in Lane's Lexicon and say they came through Persian; Persian loans are hedged as usually traced; English and Portuguese loans name the donor. 46 origins, and parts of 32 more, are explicitly not settled after a search of CDIAL by form and by gloss. Older passages that contradicted CDIAL were corrected (আমি is listed under asmad 'we', not asmi 'I am'; কাল 'tomorrow' is from kalya, not kāla), and the nine raised max_seconds values equal their computed length, all below 300 seconds.",
  ],
} as const satisfies EtymologyGapPinOwner;
