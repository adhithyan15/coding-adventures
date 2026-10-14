import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "telugu",
  withoutHook: 0,
  withoutVisible: 0,
  rationale: [
    "The 1,376 / 1,318 baseline fell to 0 / 0. Every Telugu word lesson now carries an etymology hook and states its origin, almost always in a taken-apart block.",
    "Native words follow DEDR with listed cognates and all Sanskrit, uncertainty, and Indo-Aryan-appendix notes preserved; Sanskrit loans follow Turner's CDIAL with its Dravidian notes; both were read from moli-mandala data without copying source text. English, Persian, Arabic, and Portuguese loans name their donor. Seventy-nine lessons state that the origin remains uncertain after searches by form and gloss.",
  ],
} as const satisfies EtymologyGapPinOwner;
