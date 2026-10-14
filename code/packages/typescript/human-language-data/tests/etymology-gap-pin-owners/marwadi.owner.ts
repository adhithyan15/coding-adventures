import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "marwadi",
  withoutHook: 0,
  withoutVisible: 0,
  rationale: [
    "The 552 / 716 baseline fell to 0 / 0. Every one of the 718 Marwadi word lessons now has an etymology_hook and says where its word comes from: 717 new \"taken apart\" blocks, plus one origin sentence in MW-C05-hear-naam, whose section already had a block. All 166 older hooks were rewritten; most were spelling or usage notes rather than origins, and 2 (igyārā, aṭhārā) described their sound changes wrongly.",
    "Inherited words and Sanskrit loans cite Turner's CDIAL, naming the Marwari or Old Marwari row where CDIAL has one and the Hindi or Gujarati row otherwise, and keeping its \"prob.\", doubtful, Dravidian, Munda and Austroasiatic notes; 4 lessons also cite DEDR. Arabic loans name their root in Lane's Lexicon and reach Marwadi through Persian; Persian, English and Portuguese loans name the donor, Persian ones as \"usually traced to\". About 50 words say the origin is not settled or only partly known. 3 max_seconds rose to the computed length (172, 174, 197 s), all under 300 s.",
  ],
} as const satisfies EtymologyGapPinOwner;
