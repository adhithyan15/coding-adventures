---
category: Testing & coverage
---

# Grammar-lesson headwords anchor a written Chinese character as well as word headwords do

The Chinese B1 core writes 然, 所, 走 and 坐 directly after GRAMMAR lessons whose
headwords first print them (先…然后…最后, 因为…所以…, 去…怎么走, 坐火车去), not after a
word lesson. Earlier Chinese stages always anchored on a word. Measured after authoring,
`anchoring.mjs chinese` listed all sixteen new letter lessons as anchored (letter lessons
209 -> 225; cold 0, buildsToward 51, unreadInventory 0, unwritten 0): the letter-anchoring
measure counts any non-writing lesson whose headword holds the character.

So a character whose natural home is a function pattern (然 in 然后, 所 in 所以) can be
written right after the grammar lesson, and the word fallback (不然, 所在, 走开) is not
needed. Two conditions keep it honest: the writing lesson must come directly after the
grammar lesson, and the grammar headword's other characters must all be written already.
Re-run `anchoring.mjs` after authoring; if a grammar anchor ever comes out builds-toward,
use the word fallback rather than moving the writing lesson.
