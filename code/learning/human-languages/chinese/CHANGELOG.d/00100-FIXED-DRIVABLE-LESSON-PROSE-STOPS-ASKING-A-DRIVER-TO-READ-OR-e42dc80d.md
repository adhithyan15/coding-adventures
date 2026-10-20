## Fixed — drivable lesson prose stops asking a driver to read or handle cards

Narration reads bare prose aloud as written, so a prose instruction to read
printed script, handle cards or cover the page reached a driver unhedged (issue
#12070, tenth pass): "Hear, picture the part, say, and read **おなか**." was
narrated word for word. Each such step now moves into a cue the narration
defers (`[YOU READ: …]`, `[YOU COVER: …]`, `[YOU CHECK: …]`, `[YOU FIND: …]`,
`[YOU WRITE: …]`: "once you have stopped driving — …"), in the authored order,
or is said for the ear and voice where the step was not about the page. Prose
that followed a new cue in the same paragraph now has a paragraph of its own.
The new prose check in human-language-data demands zero such spans in drivable
lessons. Every edited lesson stays `drivable: true` (only its
`core/lesson-modality` source hash changes).

- **Count:** 64 drivable lessons; 69 `[YOU READ: …]`, 3 `[YOU COVER: …]`, 1
  `[YOU CHECK: …]` and 2 `[YOU WRITE: …]` cues, and 13 ear-and-voice rewrites.
- Character reading: "Read the character without pinyin.", "Read it once
  without pinyin.", "Read 同, then 学.", "Read the pair without pinyin.", "Read
  the two known shapes from left to right." and the like become READ cues in
  ZH-C12-erzi, ZH-C12-jia, ZH-C12-nu, ZH-C12-ren, ZH-C13-da, ZH-C13-xue,
  ZH-C13-xuesheng, ZH-C14-xiao, ZH-C15-tongxue, ZH-C15-zhong,
  ZH-C15-zhongxuesheng, ZH-C16-hanyu, ZH-C16-hanzi, ZH-C17-zhongguo,
  ZH-C17-zhongwen, ZH-C18-kanjian, ZH-C19-nihaoma, ZH-C22-zhongguoren. A spoken
  step about the character just read ("then give its meaning", "then hold its
  first tone level") stays inside the cue. "Read this before looking below:" →
  `[YOU READ: this first, before the explanation]`.
- "Hide the pinyin. Read 书, …" (ZH-C18-shu, ZH-C19-ma) → `[YOU COVER: the
  pinyin] [YOU READ: 书, …]`; ZH-C12-ren "Read **人**. Cover it and wait five
  seconds." → READ, then COVER, then the existing WRITE cue.
- Four-skill payoffs ZH-C12..C16-practice: "produce all six from meaning cards"
  → "produce all six from their English meanings" (meanings can be heard); "3.
  **Reading:** read the six character cards without pinyin" → `3. **Reading:**
  [YOU READ: the six character cards without pinyin]`. ZH-C19-practice,
  ZH-C20-practice and ZH-C21-practice likewise; ZH-C22..C28-practice's
  "**Read.** 我是中国人 — 你是中国人吗" becomes the READ cue itself.
- Spaced reviews: ZH-R12 "Then take six meaning cards, say each word, and read
  the six character cards." → "Then, from six English meanings, say each word.
  [YOU READ: the six character cards]"; ZH-R13 the same; "Say and read each" →
  "Say each. [YOU READ: each]" (R14, R15-2, R16-2); "Say each answer and read
  its unpointed card." → "Say each answer. [YOU READ: its unpointed card]"
  (R15-1, R16-1, R17, R18, R19); "Shuffle four meaning cards: …" → "Take four
  meanings in a shuffled order: …"; "Shuffle five unpointed cards: …. Read each
  cold" → one READ cue (R17/R18-reading-five); "Then reverse the cards and say
  all five Mandarin forms" → "Then reverse the direction …"; "then put that
  card away" → "then set that word aside"; R15-school-map-4's "Build two
  three-item rows without a model … Read both rows aloud. Add **同学** and **上学**
  from meaning cards." → WRITE, READ, WRITE cues (the rows are written: the
  lesson is a dictation-transcription stage).
- Ear rewrites where the step is about the sentence, not the script: ZH-C27-sui
  "Read that again and look for the verb" → "Say that again and listen for the
  verb"; ZH-C03-yi "Read those two facts together" → "Take those two facts
  together"; ZH-C23-de-mingci "**Read the Mandarin order against the
  English.**" → "**Set …**"; ZH-C28-xihuan-shenme "**Now look at where 什么
  is.**" → "**Now notice where 什么 is.**"; ZH-C17-zhongwen's meaning card → "one
  Chapter 15 school word by its English meaning".
- Left alone: "Keep the character covered" / "Keep the page covered" (a state,
  not a step), "Look at what is NOT there" (the idiom), the glosses *read a
  book*, and ZH-C27-practice (not drivable).
- **Review follow-up** (same change, second commit). The first pass's rewrites lost some of what a listener needs and broke some of what the book prints; this track's share of the fixes:
  - A spoken premise, gloss or answer that the first pass had moved inside a
    deferred cue is said in prose again, and the cue keeps only the look
    (ZH-C19-nihaoma): "They are 你, 好, and the new 吗 on the end.".
  - Cues that opened with *it*, *them*, *this* or *these* name their object, so
    the book no longer prints "*Read it:* them again" (ZH-C06, ZH-C12-erzi,
    ZH-C12-jia, ZH-C12-ren, ZH-C13-xue, ZH-C16-hanyu, ZH-C17-zhongguo,
    ZH-C17-zhongwen, ZH-C22-zhongguoren, ZH-C29-diyi-ke, ZH-C29-words): "[YOU
    READ: **儿子** once without pinyin]", "[YOU READ: **汉语** first, before the
    explanation]", "[YOU COVER: **人**, then wait five seconds]" (was "Cover: it,
    and wait five seconds").
  - The spaced reviews ZH-R14..R19 read "[YOU READ: each Mandarin form]" (R14,
    R15) or "[YOU READ: each Mandarin form without pinyin]" (R16-1, R17, R18,
    R19) where the first pass left "[YOU READ: each]" and "[YOU READ: its
    unpointed card]", whose *its* had no referent for a listener.
    ZH-R15-school-map-3 "Take four meanings in a shuffled order: …" → "Hear four
    meanings in mixed order: … Say each. [YOU READ: each Mandarin form]". ZH-R19
    "[YOU READ: its unpointed card] Decide which …" gives the decision its own
    paragraph.
  - ZH-R18-book-reading-r1 "Check whether 看 is present before deciding noun or
    activity." and ZH-R18-looking-three-r1 "Check the neighbour of 看 …" become
    `[YOU CHECK: …]` cues: both check the characters the learner just wrote.
  - Four-skill items drop the "**Reading:**" label in front of a READ cue, which
    the book printed as "Reading: *Read it:* …" (ZH-C12..C16-practice,
    ZH-C19-practice).
  - A new cue followed by prose or by another cue in the same paragraph now ends
    its paragraph, so the book no longer runs "*Cover:* the page The next
    question …" together (ZH-C12-nu, ZH-C12-ren, ZH-C18-shu, ZH-C19-ma,
    ZH-R12..R19, 17 lessons).
- **Second review follow-up:** ZH-R18-book-reading-r1 and ZH-R18-looking-three-r1 no longer defer the check on 看 until after the writing step, where "before deciding" came too late. *kàn* is audible, so the prose now says "Listen for *kàn* before deciding …" before the answers, and the CHECK cue is gone.

