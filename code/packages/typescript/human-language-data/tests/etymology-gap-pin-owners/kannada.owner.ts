import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "kannada",
  withoutHook: 0,
  withoutVisible: 0,
  rationale: [
    "The 1,330 / 1,281 baseline fell to 0 / 0. All 1,345 gap lessons from KA-C01 through KA-C307 now state the word's origin in a taken-apart block or origin sentence with a matching hook.",
    "Native words follow DEDR (Burrow and Emeneau 1984) with every question mark, Cf. Skt., and Indo-Aryan note retained; Sanskrit tatsamas and tadbhavas follow CDIAL; English and Perso-Arabic loans name the donor. About 63 lessons say the origin is uncertain after searches by form and gloss and hedge the usual account where one exists.",
  ],
} as const satisfies EtymologyGapPinOwner;
