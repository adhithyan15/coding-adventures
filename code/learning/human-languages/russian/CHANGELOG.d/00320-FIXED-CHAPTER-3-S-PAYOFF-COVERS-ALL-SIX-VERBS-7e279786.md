## Fixed — chapter 3's payoff covers all six verbs

Chapter 3's payoff, RU-C03-idti, assessed only идти's own three atoms and the
two present-tense patterns: 5 of the 19 atoms the chapter introduces (0.26),
below the 0.5 `chapter-payoff-not-representative` floor. Chapter 3 is all
spoken lessons with no writing runway, and RU-C03-idti is its last lesson by
sequence (190), so it keeps the payoff and no new lesson was needed. It now
assesses all 19 (1.00):

- A new "Guided Practice — the chapter, said back" section, placed before the
  wrap-up, has five spoken recall prompts drawn only from what the earlier
  lessons taught. **Я студент**, with быть silent and a cousin of *be*.
  **Я живу**, and the old meaning of *quick* (alive). **Я не знаю**, where
  **не** is all of *don't* and знать is *know*. **Я говорю**, where **г** is
  gamma and a hard *g*, and *govern* is not related. **Я вижу**, where **д**
  turns to **ж** in the *I* form only, and the English kin is *wit*.
- RU-C03-govorit-false-friend (sequence 175) joins `prerequisites`, so the
  *govern* false-friend atom is reachable. Before this change the chain ran
  govorit → videt → idti and skipped it. `reviews_of` now lists all six earlier
  chapter-3 lessons.
- The 14 added atoms are listed in `requires.knowledge` and
  `practises.knowledge`.
- To keep the lesson under the 300 s cap, the existing prose is tightened, but
  none of what it teaches is dropped. The warm-up is shorter. The endings line
  is folded into one sentence. The going-now / going-often lens is condensed.
  The *h₁ei-* cousin list drops *ambition* and *perish*, along with their
  glosses and *initial*. The suppletion paragraphs are tightened, but they
  still say шёл shares no root with идти, still name **suppletion**, and still
  give go / went from *wend*. The computed duration goes from 281 s to 289 s,
  and `duration.max_seconds` rises from 240 to 290.
- `chapters.d/0003.json` lists all 19 atoms (plus the carried-in RU-LEX-YA),
  and its summary and note are rewritten. The old note asked for a dedicated
  payoff lesson.

