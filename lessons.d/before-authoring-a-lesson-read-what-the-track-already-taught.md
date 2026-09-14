# Before authoring a lesson, read what the track already taught

I wrote a four-lesson B1 chapter for Spanish. Two of the four re-taught material the
book already owned, and I did not notice because I designed the chapter from the
spine node downward instead of from the corpus upward.

- The **imperfect** is taught in full at chapter 16, three irregulars and all, with
  the same Latin-endings etymology I "introduced" at chapter 38.
- **`luego`** is taught at chapter 5 inside *hasta luego*, with the same `locus`
  etymology, as the atoms `ES-LEX-LUEGO` and `ES-ETYMON-LOCUS` — which my new lesson
  re-introduced under new names, so `atomsTaught` counted the word twice.

Neither is visible from the spine. Both are one grep away.

**Rule:** before writing a lesson, grep the track for the headword AND for the
concept, and read any lesson that already covers either. Then write the lesson for
what is genuinely left — which is usually narrower, and better, than what you planned.
Chapter 38's imperfect lesson went from "here is a tense" (wrong, and 22 chapters
late) to "here is which of the two you already have to reach for" — the actual B1
skill, and a smaller, truer step.

**Corollary — a "you already own every word" claim is checkable, so check it.** The
payoff story used five words the course teaches nowhere, printed directly under the
sentence "every word is one you already own". The `forwardReferences` metric cannot
catch this: its blind spot is vocabulary the corpus never teaches at all. Diff the
story's words against the track's headword list by hand.
