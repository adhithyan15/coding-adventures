import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "spanish",
  withoutHook: 0,
  withoutVisible: 0,
  rationale: [
    "The 486 / 209 baseline fell to 0 / 0. A first pass added matching hooks to 287 body explanations, blocks and hooks to 199 lessons, and visible origin text to ten hook-only lessons. It used EtymDB 2.1 (Fourrier and Sagot 2020), CC BY-SA 4.0, Lewis and Short where needed, and named Arabic donor words; uncertain origins were stated plainly.",
    "A full review of all 497 word lessons then corrected loan routes, etyma, hedges, and labels, and gave 26 blocks that named only Spanish parts a sentence explaining where those parts came from.",
  ],
} as const satisfies EtymologyGapPinOwner;
