---
category: Repo policy / workflow reminders
---

# Lane's lexicon_xml files a root under its own Buckwalter key: initial hamza A, medial A^ or A', a doubled root as two letters

Checking Arabic roots by script against laneslexicon/lexicon_xml (raw files
`_A0.xml`, `b0.xml`, ... `_Y1.xml`), a plain transliteration of the root missed
dozens of real articles. The `div2 n="..."` key is Buckwalter with its own
habits:

- an initial hamza is `A` (`Anf`, `Akl`), a medial or final one `A^` or `A'`
  (`sA^l`, `mrA'`), and at least one key carries a stray vowel (`mlA^a`);
- a doubled root is keyed by two letters (`sn` for *s-n-n*, `Hm` for *ḥ-m-m*),
  and some nouns sit under a two-letter key (`dm`, `yd`);
- a final weak letter may be `w`, `y` or `Y`, and some keys join variants
  (`*rw and *rY`, `qhl qhw`, `Quasi ...`);
- reduplicated roots (*z-l-z-l*, *s-l-s-l*) are filed under the doubled root
  (`zl`, `sl`).

The later letters (k to y) come from the posthumous supplement and are thin:
*n-w-m* and *h-t-f* have no article at all, and *q-h-w* says only "See
Supplement".

Do: normalise keys (split on ` and `, ` or `, spaces; map `Y` to `y`; try the
two-letter key for doubled and weak-final roots) before calling a root
missing, and say "Lane has no article on this root" rather than citing Lane
for a root it does not cover.
