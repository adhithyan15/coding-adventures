import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "hindi",
  withoutHook: 0,
  withoutVisible: 0,
  rationale: [
    "The 1,126 / 967 baseline fell to 0 / 0. Every Hindi word lesson now has an etymology hook and states its origin: 962 gained taken-apart blocks and 43 existing sections gained one origin sentence.",
    "Inherited words and Sanskrit loans follow Turner's CDIAL with its probability, uncertainty, Dravidian, Munda, and Austroasiatic notes preserved; Arabic loans name their root in Lane's Lexicon; Persian, English, and Portuguese loans name the donor. About 48 origins are explicitly uncertain or partly known. Fifteen contradictory older blocks were corrected, and 41 max_seconds values rose to their computed length while remaining below 300 seconds.",
  ],
} as const satisfies EtymologyGapPinOwner;
