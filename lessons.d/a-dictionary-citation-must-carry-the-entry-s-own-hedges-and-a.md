---
category: Testing & coverage
---

# A dictionary citation must carry the entry's own hedges, and a sampled review misses them

Filling Malayalam etymologies from DEDR and CDIAL, the author's script
confirmed that every entry number existed and every named cognate appeared
in the article. A full review (all 1384 lessons, not a sample) still found
22 lessons that misreported their source. The numbers were right, but the
hedges were lost: DEDR 50 says *accan* is "probably < IA", and the lesson
called it native; a "?" in the article was moved onto the wrong word; nine
"Cf. Skt." notes were dropped; seven words were called "not recorded" when
DEDR or CDIAL holds them. Earlier Tamil and French reviews read samples of
200-250 and reported near-zero number errors, which measured the wrong
thing.

Do: script the number and cognate checks, and also check that every
hedge in the entry (`?`, "probably", "Cf.", "< IA") survives into the
lesson. Before writing "not recorded", search both dictionaries by form
and by gloss. Have the reviewer read every entry when the lessons are
generated in bulk. Do not: treat "the number exists and names this word"
as "the citation is right".
