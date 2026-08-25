### Removed — three chapter 2 lessons stop teaching script

With the glyphs housed, `TA-C02-en`, `TA-C02-enna` and `TA-C02-peyar` drop their
`## The letters in this word` sections, and `ch02-introductions.tex` drops the three
matching `sounds` boxes. Verified against the **generated** manifest rather than the
source: all three now record `reasons: ["no-visual-dependency"]` and read as `voice`, so
this is script teaching genuinely leaving the lesson, not a heading renamed out from
under the classifier. Tamil chapter 2 becomes startable by ear for the first time
(`unstartableChapters` 129 → 128, its drivable prefix 0 → 2).

**`TA-C02-nii-niingal` keeps its section**, because ங, ள and the ீ sign are still taught
nowhere else. A strict check — does a strand block make the glyph its own subject, in a
heading, its own table row, or an "X is Y" sentence — was needed to see this: all three
*appear* in strand lessons, but only inside examples (ஸ்ரீ, ங்க, புள்ளி). Mere
appearance is not teaching, and the looser check would have licensed deleting the only
explanation those letters have. The same test says ப was never taught either, which is
why `TA-W09` teaches it rather than assuming it.

Six chapter 3 lessons still teach script inline, and chapters 3-5's book `sounds` boxes
still show ப, எ and ய before the strand reaches them. This closes chapter 2 only.

Measured: `atomsTaught` 2502 → 2507, `voice` 1076 → 1079, `sight` 535 → 532, `pen`
56 → 58, `unstartableChapters` 142 → 141, `drivablePrefixTotal` 876 → 878, and
`fullyDrivableChapters` 323 → 321 as chapters 21 and 23 each take a writing lesson.

`atomsNeverRevisited` **rises**, 472 → 474, and it is worth saying why rather than
burying it. Three of the five new atoms are `TA-W09`'s, and nothing follows `TA-W09`, so
they are orphans by construction. Against that, `TA-W09` re-uses ர when it spells
**பெயர்**, and declaring `CA-ONE-LETTER-01` pulls that atom out of the orphan set for
the first time. Three in, one out.

`missedByWindow.R2` 1716 → 1718 is the same shape and the more interesting one: all five
new atoms miss R2 too, offset by **three** pre-existing atoms that `TA-W09` pulls back
into it. `TA-W09` sits 12 lessons after `TA-W06` and 8 after `TA-W07`, both inside R2's
5-15 window, so practising `INDEPENDENT-VOWEL-I-01`, `LA-AI-SIGN-02` and
`CA-ONE-LETTER-01` there reinforces them at a distance R1 can never reach. A strand
spread thin misses the near window and starts hitting the far one.


