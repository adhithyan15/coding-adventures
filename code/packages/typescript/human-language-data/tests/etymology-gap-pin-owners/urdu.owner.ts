import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "urdu",
  withoutHook: 0,
  withoutVisible: 0,
  rationale: [
    "The 1,123 / 1,137 baseline fell to 0 / 0. Every Urdu word lesson now has an etymology hook and states its origin: 1,139 gained taken-apart blocks, 58 existing sections were corrected or gained an origin sentence (one of them the empty ṭhīk block), and 77 of the 79 hand-written lessons got shorter origin hooks.",
    "Inherited words and Sanskrit loans follow Turner's CDIAL with its probability, uncertainty, Dravidian, and Munda notes preserved, matching the Hindi lessons except where CDIAL files a word under another form; Arabic loans name their root in Lane's Lexicon and the Persian road they took; Persian, English, Portuguese, and Turkic donors are named. About 47 origins are explicitly not settled. Older blocks that disagreed with CDIAL, such as ānā from āyāti and yih from iyam, were corrected, and no max_seconds value was raised.",
  ],
} as const satisfies EtymologyGapPinOwner;
