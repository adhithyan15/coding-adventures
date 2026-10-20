import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "german",
  withoutHook: 0,
  withoutVisible: 0,
  rationale: [
    "The 988 / 1,071 baseline fell to 0 / 0. Every German word lesson now carries an etymology hook and learner-visible taken-apart block.",
    "EtymDB 2.1 (Fourrier and Sagot 2020), CC BY-SA 4.0, supports the cited rows; well-established origins without a supporting row carry no source tag, and twelve genuinely uncertain origins say so.",
  ],
} as const satisfies EtymologyGapPinOwner;
