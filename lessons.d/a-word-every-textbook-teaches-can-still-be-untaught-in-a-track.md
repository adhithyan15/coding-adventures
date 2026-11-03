---
category: Testing & coverage
---

# A word every textbook teaches can still be untaught in a track, so segment each example against the taught table

The Chinese B1 core brief printed examples like 我喜欢很多中国菜, 这么多书 and 你应该多喝水.
Every character was written, so the printability check passed. But 很多 'a lot' and 多
before a verb ('more') are not taught anywhere in the Chinese track: 多 entered in 多少,
多大 and 多云, and *hěn duō* appears in no lesson. Likewise 可是 'but' is a later
tranche's word, and 上 as a bare locative ('on the mountain') had to be taught in place
(山上 as 山 with the short form of *shàngbian*).

Printable is not taught. Segment every hanzi example against the taught-word table
(every claims file's headwords, plus the core's own earlier words and the function tokens
its grammar teaches) and read the leftovers one by one: a single leftover character is
often a word nobody taught. The B1 core's build script now prints that report
(`build.py --words`), and the leftovers were replaced or taught in place.
