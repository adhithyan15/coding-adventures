## Fixed — review pulses get back the kana and kanji an encoder turned into `?`

Twenty-five review pulses lost their Japanese when chapters 1-6 were authored
in #12472: every character the authoring tool's encoding could not hold came
out as one ASCII `?`, so the book and the narration printed "Say ??? and tap
all three morae", "Write ? from memory before tracing ?." and "recall how
?hard to exist? became thanks". JA-C01-practice was the twenty-sixth and was
fixed in the drive-debt change below. No revision ever held the real text:
the squashed commit and the pre-squash PR commit both carry the `?`. So each
span is inferred, from three independent constraints that agree in every case:

1. **One `?` per lost character.** JA-C01-practice proved it: its six runs
   (2, 3, 5, 5, 3, 4) are exactly はい, いいえ, こんにちは, ありがとう, 日本語
   and コーヒー. A curly quotation mark is non-ASCII too, so `?hard to exist?`
   is “hard to exist” with its quotes lost.
2. **The block's own `hl-knowledge: assesses=[…]` list**, which names the
   atom each pulse recalls, and the lesson that introduces that atom.
3. **The sentence around it** ("three morae", "its two known signs", "its
   five signs", "the sign that closes …", "its base sign").

| Lesson | Block assesses | Restored |
|---|---|---|
| JA-C01-iie | LEX-HAI, SCRIPT-I-01 | Say **はい**, then write **い** once |
| JA-W01-ha | SCRIPT-HIRAGANA-MORA (from JA-W01-i) | Say **い** and tap its one mora |
| JA-W01-ko | LEX-IIE, MORA-LENGTH | Say **いいえ** and tap all three morae |
| JA-W01-n | SCRIPT-HAI-READ-01 | write **はい** from its two known signs |
| JA-W01-ni | SCRIPT-N-01, SCRIPT-E-01 | write **ん** once and then **え** once |
| JA-W01-wa | LEX-IIE, MORA-LENGTH | Say **いいえ** again, keeping the two opening morae distinct |
| JA-W01-konnichiwa-read | SCRIPT-WA-01 | Write **わ** once, then set it beside **は** |
| JA-C01-konnichiwa | SCRIPT-N-01 | Write **ん** from memory and give it one full mora |
| JA-W03-a | LEX-KONNICHIWA, SCRIPT-NI-01 | write **に** from memory |
| JA-W03-ri | SCRIPT-CHI-01 | Write **ち** … before tracing **り** |
| JA-W03-ka | SCRIPT-WA-01, PARTICLE-WA-SPELLING | Write **わ**, then … the sign that closes **こんにちは** |
| JA-W03-dakuten | SCRIPT-KONNICHIWA-READ-01 | Write **こんにちは** from its five signs |
| JA-C01-arigatou | SCRIPT-KA-01 | Write **か** once …, then add the two dakuten strokes |
| JA-W03-sa | ETYMON-ARIGATASHI, LEX-ARIGATOU, SCRIPT-DAKUTEN | Say **ありがとう**, add the dakuten to **か**, and recall how “hard to exist” became thanks |
| JA-W03-ma | SCRIPT-TO-01 | Write **と** … before tracing **ま** |
| JA-W03-su | SCRIPT-U-01 | Write **う** … before tracing **す** |
| JA-C03-practice | ETYMON-ARIGATASHI, … | the “hard to exist” memory hook |
| JA-W05-nichi-kanji | SCRIPT-GOZAIMASU-READ-01, SCRIPT-SA-01 | Read **ございます** once and write its base sign **さ** |
| JA-W05-hon-kanji | SCRIPT-MA-01 | Write **ま** once from memory |
| JA-W05-gen-component | SCRIPT-SU-01 | Write **す** once from memory |
| JA-W05-mouth-component | SCRIPT-GOZAIMASU-READ-01 | Read **ございます** from memory |
| JA-C01-nihongo | SCRIPT-KANJI-GO-01 | Build **語** once from **言**, **五**, and **口** |
| JA-W06-ko-katakana | BRIDGE-SINO-JAPANESE, KANJI-READINGS, KANJI-SPEECH-COMPONENT-01 | Trace **言** once |
| JA-W06-long-mark | SCRIPT-KANJI-FIVE-COMPONENT-01 | Write **五** once from memory |
| JA-W06-hi-katakana | SCRIPT-KANJI-MOUTH-COMPONENT-01 | Write **口** once from memory |

Judgement calls:

- **The second sign of a "before tracing ?" pulse is the lesson's own new
  sign** (り, ま, す): the block assesses only the recalled sign, and the
  sentence says the second one is about to be traced, which only the
  headword is.
- **JA-W03-sa's "add the dakuten to ?" is か, not さ.** Both are one
  character and both take a dakuten. The pulse assesses JA-LEX-ARIGATOU and
  JA-SCRIPT-DAKUTEN, not the lesson's own さ (which the Guided Practice above
  it has already voiced to ざ), and the word it has just said, ありがとう,
  carries its dakuten on か. So か.
- **JA-W05-nichi-kanji's "its base sign" is さ**, the unvoiced base of the ざ
  in ございます, as SCRIPT-SA-01 in the same block says.
- **JA-C01-nihongo's three components are 言, 五 and 口**, in the order
  chapter 5 teaches them (JA-W05-gen-component, -five-component,
  -mouth-component, sequences 290-310).
- **Formatting follows the track, not the lost bytes.** The restored kana and
  kanji are bold, as in JA-C01-practice's fix and everywhere else these
  lessons name a Japanese form. An ASCII `**` would have survived the
  encoder, so the original probably had none. The lost quotation marks become
  curly “ ”, matching JA-C01-arigatou's `etymology_hook`.
- Regenerated: book chapters 1-6, narration ch01-ch06 (`.json` and `.txt`),
  their generated book and narration hashes, and the 25 lessons'
  `core/lesson-modality` owners (source hash only).
- `tests/lost-script.test.ts` in human-language-data now fails on this shape
  of damage in any lesson or generated book.

