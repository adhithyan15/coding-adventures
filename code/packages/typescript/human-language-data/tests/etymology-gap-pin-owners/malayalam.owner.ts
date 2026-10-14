import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "malayalam",
  withoutHook: 0,
  withoutVisible: 0,
  rationale: [
    "The 1,367 / 1,275 baseline fell to 0 / 0. Every Malayalam word lesson now carries an etymology hook and states its origin, almost always in a taken-apart block.",
    "Native words follow DEDR (Burrow and Emeneau 1984), naming only listed cognates and retaining its Sanskrit and uncertainty caveats; Sanskrit loans follow Turner's CDIAL where available; English, Portuguese, Arabic, and Persian loans name the donor. Forty-seven lessons say the origin is uncertain or only usually traced.",
  ],
} as const satisfies EtymologyGapPinOwner;
