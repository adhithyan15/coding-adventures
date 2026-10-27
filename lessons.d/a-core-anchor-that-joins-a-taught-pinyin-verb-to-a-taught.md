---
category: Testing & coverage
---

# A core anchor that joins a taught pinyin verb to a taught noun is a derivation, so anchor the character on the grammar lesson that prints it

The Chinese B1 core plan anchored 坐 on a new word lesson, 坐车 *zuòchē* 'to travel by
car', with the example 我们坐车去. But *zuò* "to sit; to travel by" was already taught
by ear in chapter 44 (ZH-C44-zuo), and its own example is *wǒmen zuò chē qù*: 坐车 was
the taught verb plus the taught noun 车, the kind of headword the A2 reviewers replaced
(见到, 读书, 老人, 写字) and the B1 builder's derivation screen refuses. The plan also
believed the bare slug `middle-chinese-dzwax` belonged to 做; it is 坐's own, from the
same chapter-44 lesson.

The fix: 坐 is anchored by the grammar lesson whose pattern first prints it, 坐火车去
(TRANSPORT-TICKET), directly before the writing lesson, and the chapter keeps its
headword count with a word that is not a derivation (*tuìpiào*, 'to return a ticket').

Before accepting any anchor, look up every character of it in the taught table by its
PINYIN headwords too (a word taught by ear has no hanzi headword to collide with), read
the earlier lesson that teaches it, and compare the examples. If the anchor is that word
plus another taught word, anchor on the grammar lesson instead, or choose a word with its
own sense.
