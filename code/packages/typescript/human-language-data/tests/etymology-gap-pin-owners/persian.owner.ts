import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "persian",
  withoutHook: 0,
  withoutVisible: 0,
  rationale: [
    "The 1,300 / 1,311 baseline fell to 0 / 0. Every Persian word lesson now has an etymology hook and states its origin: the 1,300 vocabulary-wave lessons gained a taken-apart block and a hook, 12 hand-written lessons gained a taken-apart block, the two empty taken-apart blocks (chetor, raftan) now name a source, and 14 hooks that described grammar or spelling rather than origin were replaced.",
    "Arabic loans name the Arabic word and its root in Lane's Lexicon (every root checked by script; the meaning checked where Lane's entry carries it); inherited words give their Middle Persian form, cited to EtymDB 2.1 where its rows support it and hedged as usually traced otherwise; French, English, Russian, Turkic, Mongolian, Greek, Aramaic and Indian loans name the donor. About 80 origins are explicitly not settled or only partly settled, after an independent review of all 1,320 changed lessons fixed 54 (4.1%) and gave 33 earlier \"not settled\" origins a hedged, sourced account. Older claims that no source supports were corrected: cheshm is not from the root of English spy, and setâre, chahâr and yek no longer cite unattested Old or Middle Persian forms. No max_seconds value was raised.",
  ],
} as const satisfies EtymologyGapPinOwner;
