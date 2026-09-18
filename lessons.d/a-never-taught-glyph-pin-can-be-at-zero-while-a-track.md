---
category: Repo policy / workflow reminders
---

# A never-taught-glyph pin can be at zero while a track still has script debt, so a genuinely new character is the one that trips it

Malayalam chapter 103 needed **പാഠം** and therefore U+0D20. Two sources said
different things about whether that was safe, and a third settled it.

**The inventory note (`ML-A1-SCR-12`)** says nine characters are used in
headwords and never taught. **`script-closure.test.ts`** pins the same track at
`neverTaughtGlyphs === 0`. Both are live, both pass, and they do not agree
because they are not measuring the same thing: a glyph that appears **inside
another letter lesson's example words** counts as taught by the metric, which is
the understatement already recorded in this directory. U+0D36 is in that
category. U+0D20 was in no headword and no example anywhere, so it was the track's
one genuinely never-taught glyph, and it tripped the pin the moment a word needed
it.

**Three consequences.**

**A note about script debt cannot tell you whether adding a word is safe.**
Derive it: is the character in any headword, and is it in any script lesson —
including as an example? A character that is in neither is the expensive case,
and it is invisible to `npm run validate` and to all twelve local gates. The full
suite is what fails.

**Closure is measured in reading order, so the fix has a position.** Teaching the
letter at the end of the corpus does not retire a violation earlier in it. Here
the lesson went between the word that introduced the chapter and the word that
needed the character, at a sequence in that gap.

**Teach the letter; do not feed the metric.** Adding the character to an existing
letter lesson's example words moves `neverTaughtGlyphs` to zero and teaches
nobody anything — it manufactures exactly the understatement that makes this
metric disagree with the inventory in the first place. Dropping the word is the
other wrong answer when the word is the one the exam point asks for.

**The cheap check, before drafting any word with an unusual character:** grep
every headword for the character, then grep the script lessons for it. Both
empty means a letter lesson comes with the word, and that is a chapter-shaping
fact worth knowing before the prose is written rather than after the suite runs.
