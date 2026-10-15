import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "chinese",
  withoutHook: 0,
  withoutVisible: 0,
  rationale: [
    "The 0 / 62 baseline fell to 0 / 0, and the 43 empty taken-apart blocks to 0. Every Chinese word lesson now states where its word comes from: 30 gained a taken-apart block, 32 listening lessons gained an origin sentence in pinyin, and the 43 blocks that took only the characters apart gained one origin sentence each. All 105 hooks were replaced, because they described character shapes, strokes or tones rather than the word's origin; the shapes stay in the bodies, labelled as the history of the writing.",
    "Inherited words give their Middle Chinese reading in Baxter's transcription, read from the Guangyun categories (tshet-uinh, every reading checked by script) with Unihan glosses; compounds name each morpheme. 41 lessons teach a single inherited word, 58 a compound or pattern of inherited words, 5 a particle or question word whose origin is stated as not settled (shenme, ma, de, ne), and 1 the Hindu-Arabic digits. Four lessons carry a hedged Tibetan comparison (wu, liu, jiu, ri); no Old Chinese or Proto-Sino-Tibetan form is given. No max_seconds value was raised; zaijian was tightened instead.",
  ],
} as const satisfies EtymologyGapPinOwner;
