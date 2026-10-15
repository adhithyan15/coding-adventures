import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "japanese",
  withoutHook: 0,
  withoutVisible: 0,
  rationale: [
    "The 601 / 688 baseline fell to 0 / 0. Every Japanese word lesson now has an etymology hook and states its origin: 689 gained taken-apart blocks, the 2 empty taken-apart sections gained one origin sentence each, and 4 lessons whose text already named an origin gained or corrected a hook. 91 older hooks were replaced, most because they described a sign, a sound or a grammar point rather than where the word comes from.",
    "Native words are named as inherited, with their kanji as later kun'yomi, ateji or meaning spellings per JMdict and KANJIDIC2; Sino-Japanese words name each character's meaning (Unihan, KANJIDIC2) and its Middle Chinese reading in Baxter's transcription, read from the Qieyun / Guangyun categories; the three loanwords name the donor JMdict gives (Dutch or English for koohii and kan, Portuguese for tenpura). 519 lessons are native, 136 Sino-Japanese, 28 mix the two layers, 3 are loans, and 9 origins are stated as not settled, among them uma, ume and kaban; about 30 more carry a hedged usual account. The popular story that arigatou comes from Portuguese obrigado is refuted in place. No max_seconds value was raised; one long lesson was tightened instead.",
  ],
} as const satisfies EtymologyGapPinOwner;
