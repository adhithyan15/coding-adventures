import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "latin",
  withoutHook: 0,
  withoutVisible: 0,
  rationale: [
    "The 1,095 / 1,105 baseline fell to 0 / 0. All 1,090 generated word lessons from LA-C63-bracchium through LA-C280-vallum gained a taken-apart block and matching hook; five Chapter 1 greetings gained hooks, and fifteen hand-written lessons gained short blocks.",
    "Rows were checked against EtymDB 2.1 (Fourrier and Sagot 2020), CC BY-SA 4.0, and Lewis and Short without carrying over its outdated comparisons. PIE and Proto-Italic forms are starred and hedged; 183 lessons state that the origin is uncertain and give the usual account where one exists.",
  ],
} as const satisfies EtymologyGapPinOwner;
