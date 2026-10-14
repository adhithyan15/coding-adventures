import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "punjabi",
  withoutHook: 0,
  withoutVisible: 0,
  rationale: [
    "The 549 / 524 baseline fell to 0 / 0. Every Punjabi word lesson now has an etymology hook and states its origin: 524 gained taken-apart blocks, the 31 empty taken-apart sections gained one origin sentence each, and 21 lessons that already stated their origin gained a hook.",
    "Inherited words and Sanskrit loans follow Turner's CDIAL Punjabi rows, with its probability, uncertainty, Dravidian, Munda, and Austroasiatic notes preserved and its notes on words Punjabi took from Hindi or gave to it kept; Arabic loans name their root in Lane's Lexicon; Persian, English, and Portuguese loans name the donor. 29 origins are stated as not settled, most of them words CDIAL does not list, and about 36 more carry CDIAL's own hedges. Three max_seconds values rose to their computed length while remaining below 300 seconds.",
  ],
} as const satisfies EtymologyGapPinOwner;
