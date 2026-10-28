---
category: Testing & coverage
---

# A pinyin headword is matched as a forward reference against earlier italics and equal pinyin headwords count once

Chinese teaches a word whose characters the reader has not written with a PINYIN headword
(the letter-anchoring `unwritten: 0` ceiling forbids showing an unwritten character). Two traps
came with it that a hanzi headword never meets:

- `continuity.ts` matches every headword of two or more code points against earlier text set
  in italics, and Chinese's forward-reference pin is 0. *yǔ*, *shù*, *mǎ*, *hào*, *shān* and *bái*
  were already printed in italics in earlier lessons, so a pinyin headword spelled that way reads
  as a forward reference and fails the pin.
- The gate counts DISTINCT headwords, trimmed and lowercased, so two words that are both headed
  *tā* count once.

The builder now refuses both, and the seed was re-cut (号 -> 日期, 树 -> 星星, 尝 -> 点菜; 雨 马
山 白 written so their words take hanzi headwords; 她 before 他).

Do differently: when a track's headwords are a romanization, run the forward-reference and
distinct-headword checks against the whole corpus before choosing words, not after drafting.
