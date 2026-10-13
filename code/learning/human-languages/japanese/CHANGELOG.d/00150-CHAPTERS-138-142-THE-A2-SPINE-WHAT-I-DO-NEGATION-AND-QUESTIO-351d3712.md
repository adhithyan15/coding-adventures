## Chapters 138-142: the A2 spine, what I do, negation and questions, the past, the future and practical texts

The level gate counted none of the five A2 spine nodes as realized for
Japanese. Chapters 131-137 wrote the kana that A2 words need; chapters
138-142 now spend them, one chapter per node, so the A2 gate no longer
reports a missing spine node. Vocabulary (694 of 1200 headwords) and verbs
(73 of 120) remain, so Japanese stays at A1.

Every headword is new to the corpus (no headword or homograph existed) and
written only in kana the book already teaches. Verbs are taught in the polite
**ます** form, the form a learner says, and carry `JA-VERB-*` tags for their
dictionary form.

| chapter | spine node | lessons |
|---|---|---|
| 138, My Day | SPINE-SAY-WHAT-I-DO | **おきます** (I get up), **はたらきます** (I work), **あるきます** (I walk), **かえります** (I go home), **ねます** (I go to bed); the notes find the stem of both verb groups, and the last lesson tabulates them |
| 139, Is It? No | SPINE-NEGATE-AND-ASK | **ですか** (is it ...?), **ちがいます** (no, that is not right), **じゃない** (is not), **なにも** (nothing), **だれも** (nobody), **どこにも** (nowhere); the notes add **ません** for the negative verb |
| 140, Last Week | SPINE-TALK-ABOUT-PAST | **せんしゅう** (last week), **せんげつ** (last month), **きょねん** (last year), **さっき** (a moment ago), **おわりました** (it is over); the notes add **ました** and **ませんでした** |
| 141, Next Week | SPINE-TALK-ABOUT-FUTURE | **らいしゅう** (next week), **らいげつ** (next month), **らいねん** (next year), **つもり** (a plan), **でしょう** (probably); the notes show that **ます** with a time word says the future |
| 142, Notices | SPINE-READ-PRACTICAL-TEXTS | **ちゅうい** (caution), **きんえん** (no smoking), **うけつけ** (reception), **えいぎょうちゅう** (open), **じゅんびちゅう** (not yet open), each with a notice to read and act on; then a first-pass review of chapters 138-140 and a second-pass review of 141-142 that reads a three-line notice on a clinic door |

Seven of the 26 headwords are verbs (the five of chapter 138, **ちがいます**
and **おわりました**). The last word lesson of chapters 138-141 is the
chapter payoff, and its practice uses all of that chapter's words; chapter
142's payoff is the second-pass review. Each new path segment carries an
extension at stage A2. The spine overlays in `curriculum.d/spine/` do not
change: they list canonical concept tags the track does not carry, and the
new lessons use track tags, as Telugu's and Malayalam's A2 spine chapters do.

**Reinforcement.** Twenty-eight more lessons make 46 older (atom, window)
slots measurable for the first time: R4 for 24 atoms from chapters 130-133,
R3 for 18 atoms from chapters 136-137, and R2 for four chapter 137 atoms.
Each new lesson's warm-up retrieves the atom due at its position at exactly
80, 20 and 5 lessons. Inside the five chapters, every new atom is retrieved
one lesson later and, where the track is long enough to judge it, five
lessons later, and the first eight are retrieved again twenty lessons later.
The R1, R2, R3 and R4 miss counts stay at 1, 459, 245 and 519, with no
per-atom change, and every new atom is revisited at least twice. Letter
anchoring and script closure do not move (zero violations, zero never-taught
glyphs).

