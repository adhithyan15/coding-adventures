## Added — chapter 30, how are you, and the wellbeing node

Chapter 30, **How Are You?**, realizes `SPINE-CHECK-WELLBEING`, the last of the
seven pre-A1 spine nodes the track had not reached. Eight lessons on a new path
segment (`ZH-PATH-029`) and extension (`ZH-EXT-066-WELLBEING`, pre-A1):

- **zěnmeyàng** (`QUESTION-HOW`): the everyday "how are you doing?", beside the
  textbook 你好吗. The chapter opener, taught by ear.
- **很** *hěn* (word) and **ZH-W30-hen** (writing): the plain link in front of an
  adjective (我很好, with no 是), and the one character the chapter writes. Nine
  strokes, but two pieces: its left piece 彳 is the person piece 亻 with one more
  left-falling stroke above it.
- **很好** *hěn hǎo* (`WORD-WELL`): the answer, and two third tones side by side
  (spoken *hén hǎo*, written *hěn hǎo*).
- **máng** "busy", **lèi** "tired" and **hái xíng** "not bad" (`WORD-SOSO`).
- **ZH-C30-practice**, the payoff: a morning exchange that asks how somebody is
  and answers it four ways.

The four words whose characters the track does not write yet (zěnmeyàng, máng,
lèi, hái xíng) have pinyin headwords and are taught by ear, so no lesson shows a
character that no lesson writes: the letter-anchoring ratchet keeps Chinese at
0 unwritten characters. Every hanzi the chapter prints is one the reader writes.

Curriculum: `curriculum.d/_meta.json` gains `conceptAliases` STATE-HOW-ARE-YOU ->
ZH-STATE-HOW-ARE-YOU (你好吗, chapter 19) and WORD-GOOD -> ZH-WORD-HAO (好, chapter
1), which those lessons genuinely teach, so the wellbeing node's `omits` is now
empty.

Revisits: the chapter practises the four atoms that had been revisited only once
(the counting and morning exchanges, 早上 and 早上好) and the five chapter 28-29
atoms that a new chapter would otherwise leave thin. THIN is 0.

Etymology: Middle Chinese from the Guangyun in Baxter's transcription. 很 is
*honX* "defiant" and 忙 is *mang* "afraid", each with its later sense hedged; the
characters for *hái* and *xíng* are *hwaen* "return" and *haeng* "walk, go"
(reusing the corpus slug `middle-chinese-haeng`); the Guangyun does not give
"tired" for *lèi*, so that history is not settled.

Font: `_fonts/subset-cjk.sh` now also gathers every ideograph the track prints
(lessons, chapter ledgers, book, mocks), and the subset gains 很 彳 艮 (98 -> 101
CJK code points).

### Fixed — 九十九 is jiǔshíjiǔ, not jiúshíjiǔ

Third-tone sandhi needs a third tone right after the first one, and 十 is second
tone, so the first 九 of 九十九 keeps its dip. ZH-C21-ershi taught the opposite
("the first nine rises"), ZH-C21-practice drilled it, and ZH-C21-digits,
ZH-C22-zhongguoren and ZH-C25-practice printed *jiúshíjiǔ*. All five now say
*jiǔshíjiǔ*, and ZH-C21-ershi's sound section now teaches 九十九 as the case where
the rule does not fire. Found by a reading census against CC-CEDICT.

### Fixed — 好不好 is not a verb

ZH-C19-haobuhao was tagged `ZH-QUESTION-VERB-NOT-VERB`, and the level gate counts
any tag with a `VERB-` part as a verb headword, so 好不好 was the track's only
pre-A1 "verb". It is now `ZH-QUESTION-A-NOT-A`; pre-A1 verbs go 1 -> 0, which is
the honest count.

