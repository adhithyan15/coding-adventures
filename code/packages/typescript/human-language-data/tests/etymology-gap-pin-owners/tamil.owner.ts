import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "tamil",
  withoutHook: 0,
  withoutVisible: 0,
  rationale: [
    "The 1,185 / 1,144 baseline first fell to 0 / 4. Native words use DEDR and Sanskrit loans use Turner's CDIAL, both read through the moli-mandala data without copying source text.",
    "The last four visible gaps, in TA-C06-dative-subject, TA-C37-ivar, TA-C37-ivar-en-nanbar, and TA-C39-evvalavu, gained one origin sentence in an existing section. Redundant prose paid for the additions; only TA-C39-evvalavu rose from 294 to 298 seconds.",
  ],
} as const satisfies EtymologyGapPinOwner;
