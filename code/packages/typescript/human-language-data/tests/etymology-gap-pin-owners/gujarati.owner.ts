import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "gujarati",
  withoutHook: 0,
  withoutVisible: 0,
  rationale: [
    "The 529 / 543 baseline fell to 0 / 0. Every Gujarati word lesson now has an etymology_hook and says where its word comes from: 545 new \"taken apart\" blocks, plus one origin sentence in 31 lessons whose block or section had none, among them all 24 \"taken apart\" blocks of C03-C37 that named no origin.",
    "Inherited words and Sanskrit loans cite Turner's CDIAL, keeping its \"prob.\", \"?\", Dravidian, Munda, Austroasiatic and \"H ->\" notes; Arabic loans name their root in Lane's Lexicon; Persian, English and Portuguese loans name the donor, Persian ones as \"usually traced to\". About 45 say the origin is not settled or only partly known. 8 older passages that contradicted CDIAL were corrected, and 4 max_seconds rose to the computed length, all under 300 s.",
  ],
} as const satisfies EtymologyGapPinOwner;
