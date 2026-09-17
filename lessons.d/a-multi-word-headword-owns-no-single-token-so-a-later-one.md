---
category: Repo policy / workflow reminders
---

# A multi-word headword owns no single token, so a later one-word lesson inherits every earlier use of it

`continuity.ts` builds its forward-reference matchers from lesson headwords, and
its comment is explicit that **a multi-word headword is kept whole**: "buenos
días" matches as one string, not as "buenos" and "días". Only `word` and
`phrase` lessons create a matcher at all.

The consequence is not obvious. `ML-C12-kudumbam` teaches six Malayalam family
words under a single six-word headword, so **no lesson owned the bare token
ചേച്ചി**, and a script lesson using it sat as invisible debt for 255 lessons.
Writing a later `word` lesson whose headword was exactly ചേച്ചി made that lesson
the token's first owner, and every earlier use became a forward reference at
once. The pin went 12 → 13 on a chapter that introduced one genuinely new word.

Two things follow.

**Before giving a word its own lesson, grep for it as a token and check whether
an earlier bundle headword already contains it.** A word "taught" inside a
multi-word headword is, to this gate, not taught. The grep that matters is for
uses of the token in lessons that come *before* the new one.

**Then ask whether the lesson really teaches the word.** Where the word is
already known and what is new is a use or a rule, the lesson's `type` is
`grammar`, not `word` — which is both the honest classification and the one that
does not claim ownership. Do not reclassify a genuine vocabulary lesson to dodge
the gate; if the lesson really does introduce the word, the fix is to move it to
where the word is first used.
