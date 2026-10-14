import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "chinese",
  withoutHook: 0,
  withoutVisible: 62,
  rationale: [
    "HL24 found an etymology hook on every Chinese word lesson because the hooks take characters apart, while 62 lessons still did not show that origin in the learner-visible body.",
  ],
} as const satisfies EtymologyGapPinOwner;
