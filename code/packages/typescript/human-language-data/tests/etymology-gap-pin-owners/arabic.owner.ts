import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "arabic",
  withoutHook: 0,
  withoutVisible: 0,
  rationale: [
    "The 1,187 / 1,195 baseline fell to 0 / 0. Every Arabic word lesson now has an etymology hook and states its origin: 1,197 gained a taken-apart block, 20 older blocks gained an origin sentence (among them the 19 empty ones), and all 75 older hooks were cut to 120 characters or replaced, 12 of them because they described grammar, script or the calendar instead of origin or made a false claim (ʿashar beside English -teen, al-aḥad as an ordinal, a garbled Aramaic form).",
    "Native words, about 1,080 of them, name their root and its sense in Lane's Lexicon, each root and form checked by script against Lane's articles, with the pattern where it is plain (ma- places, mi- tools, participles, Forms II to X); Hebrew, Aramaic and Akkadian cousins are named only where standard. Loans name the donor that Lane's sources or EtymDB 2.1 give: Persian 26, modern European 24, Greek 17, Aramaic or Syriac 11, Latin 4, Coptic 2, Turkish 1. About 39 lessons keep a 'not settled' clause, most for one part or a disputed route, with competing accounts named, after an independent review of all 1,262 changed lessons fixed about 228 (18%), chiefly blocks that had put the noun's own meaning where the root's sense belongs; folk etymologies are flagged in place (sikkīn 'it stills', shiṭranj 'a hundred stratagems', English sheriff beside sharīf). No max_seconds value was raised; one new block was shortened to stay under 300 seconds.",
  ],
} as const satisfies EtymologyGapPinOwner;
