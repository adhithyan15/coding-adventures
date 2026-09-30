---
category: Repo policy / workflow reminders
---

# A new headword that earlier lessons already use, or that spells an English word, creates forward references

**What went wrong.** The first German A2 vocabulary tranche added 185
headwords. `continuity.test.ts` caps German's track-wide forward references at
45 ("may fall, never grow"), and the count jumped from 29 to 185.

Forward references are counted by matching tokens, so two kinds of headword
set them off:

- **Words the course already uses in passing** (*dann*, *schon*,
  *bekommen*, *gehören*, *die Frage*). Once a late chapter teaches them as
  headwords, every earlier use becomes a reference to a word taught later.
- **Words that spell English words** (*das Kind*, *der Cousin*, *der Pass*,
  *fast*, *das Problem*, *die Information*, *modern*, *die Bank*). The English
  prose of earlier lessons matches them.

The candidate checker's `IN-TEXT` flag catches the first kind only when the
whole headword, article included, appears in an earlier body. It misses bare
forms and English homographs.

**Fix.** Measure `measureContinuity(...).forwardReferences` for the new
lessons, exclude the 37 headwords that caused any, and regenerate. The tranche
went from 185 words to 150, still with fifty verbs.

**Next time.** For a Latin-script track, check forward references before
running the full suite. Better still, drop any candidate whose bare form (no
article) appears in an earlier lesson body or is an English word.
