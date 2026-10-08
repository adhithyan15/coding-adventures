# Changelog — Japanese track

All notable changes to the Japanese curriculum track are recorded here.

## Fixed — review pulses get back the kana and kanji an encoder turned into `?`

Twenty-five review pulses lost their Japanese when chapters 1-6 were authored
in #12472: every character the authoring tool's encoding could not hold came
out as one ASCII `?`, so the book and the narration printed "Say ??? and tap
all three morae", "Write ? from memory before tracing ?." and "recall how
?hard to exist? became thanks". JA-C01-practice was the twenty-sixth and was
fixed in the drive-debt change below. No revision ever held the real text:
the squashed commit and the pre-squash PR commit both carry the `?`. So each
span is inferred, from three independent constraints that agree in every case:

1. **One `?` per lost character.** JA-C01-practice proved it: its six runs
   (2, 3, 5, 5, 3, 4) are exactly はい, いいえ, こんにちは, ありがとう, 日本語
   and コーヒー. A curly quotation mark is non-ASCII too, so `?hard to exist?`
   is “hard to exist” with its quotes lost.
2. **The block's own `hl-knowledge: assesses=[…]` list**, which names the
   atom each pulse recalls, and the lesson that introduces that atom.
3. **The sentence around it** ("three morae", "its two known signs", "its
   five signs", "the sign that closes …", "its base sign").

| Lesson | Block assesses | Restored |
|---|---|---|
| JA-C01-iie | LEX-HAI, SCRIPT-I-01 | Say **はい**, then write **い** once |
| JA-W01-ha | SCRIPT-HIRAGANA-MORA (from JA-W01-i) | Say **い** and tap its one mora |
| JA-W01-ko | LEX-IIE, MORA-LENGTH | Say **いいえ** and tap all three morae |
| JA-W01-n | SCRIPT-HAI-READ-01 | write **はい** from its two known signs |
| JA-W01-ni | SCRIPT-N-01, SCRIPT-E-01 | write **ん** once and then **え** once |
| JA-W01-wa | LEX-IIE, MORA-LENGTH | Say **いいえ** again, keeping the two opening morae distinct |
| JA-W01-konnichiwa-read | SCRIPT-WA-01 | Write **わ** once, then set it beside **は** |
| JA-C01-konnichiwa | SCRIPT-N-01 | Write **ん** from memory and give it one full mora |
| JA-W03-a | LEX-KONNICHIWA, SCRIPT-NI-01 | write **に** from memory |
| JA-W03-ri | SCRIPT-CHI-01 | Write **ち** … before tracing **り** |
| JA-W03-ka | SCRIPT-WA-01, PARTICLE-WA-SPELLING | Write **わ**, then … the sign that closes **こんにちは** |
| JA-W03-dakuten | SCRIPT-KONNICHIWA-READ-01 | Write **こんにちは** from its five signs |
| JA-C01-arigatou | SCRIPT-KA-01 | Write **か** once …, then add the two dakuten strokes |
| JA-W03-sa | ETYMON-ARIGATASHI, LEX-ARIGATOU, SCRIPT-DAKUTEN | Say **ありがとう**, add the dakuten to **か**, and recall how “hard to exist” became thanks |
| JA-W03-ma | SCRIPT-TO-01 | Write **と** … before tracing **ま** |
| JA-W03-su | SCRIPT-U-01 | Write **う** … before tracing **す** |
| JA-C03-practice | ETYMON-ARIGATASHI, … | the “hard to exist” memory hook |
| JA-W05-nichi-kanji | SCRIPT-GOZAIMASU-READ-01, SCRIPT-SA-01 | Read **ございます** once and write its base sign **さ** |
| JA-W05-hon-kanji | SCRIPT-MA-01 | Write **ま** once from memory |
| JA-W05-gen-component | SCRIPT-SU-01 | Write **す** once from memory |
| JA-W05-mouth-component | SCRIPT-GOZAIMASU-READ-01 | Read **ございます** from memory |
| JA-C01-nihongo | SCRIPT-KANJI-GO-01 | Build **語** once from **言**, **五**, and **口** |
| JA-W06-ko-katakana | BRIDGE-SINO-JAPANESE, KANJI-READINGS, KANJI-SPEECH-COMPONENT-01 | Trace **言** once |
| JA-W06-long-mark | SCRIPT-KANJI-FIVE-COMPONENT-01 | Write **五** once from memory |
| JA-W06-hi-katakana | SCRIPT-KANJI-MOUTH-COMPONENT-01 | Write **口** once from memory |

Judgement calls:

- **The second sign of a "before tracing ?" pulse is the lesson's own new
  sign** (り, ま, す): the block assesses only the recalled sign, and the
  sentence says the second one is about to be traced, which only the
  headword is.
- **JA-W03-sa's "add the dakuten to ?" is か, not さ.** Both are one
  character and both take a dakuten. The pulse assesses JA-LEX-ARIGATOU and
  JA-SCRIPT-DAKUTEN, not the lesson's own さ (which the Guided Practice above
  it has already voiced to ざ), and the word it has just said, ありがとう,
  carries its dakuten on か. So か.
- **JA-W05-nichi-kanji's "its base sign" is さ**, the unvoiced base of the ざ
  in ございます, as SCRIPT-SA-01 in the same block says.
- **JA-C01-nihongo's three components are 言, 五 and 口**, in the order
  chapter 5 teaches them (JA-W05-gen-component, -five-component,
  -mouth-component, sequences 290-310).
- **Formatting follows the track, not the lost bytes.** The restored kana and
  kanji are bold, as in JA-C01-practice's fix and everywhere else these
  lessons name a Japanese form. An ASCII `**` would have survived the
  encoder, so the original probably had none. The lost quotation marks become
  curly “ ”, matching JA-C01-arigatou's `etymology_hook`.
- Regenerated: book chapters 1-6, narration ch01-ch06 (`.json` and `.txt`),
  their generated book and narration hashes, and the 25 lessons'
  `core/lesson-modality` owners (source hash only).
- `tests/lost-script.test.ts` in human-language-data now fails on this shape
  of damage in any lesson or generated book.

## Fixed — drivable lessons stop telling a driver to write

The modality manifest marks 102 lessons in chapters 7-18 and 131-137, and
their review lessons, `drivable: true`, but each still asked for writing in
bare prose ("Write **い**, **ち**, and **と**.", "Copy **あね**, hide it, and
write it from the meaning.", "4. **Write:** hear the word, wait ten seconds,
and write all five signs."). Narration reads bare prose unhedged, so the
audio edition told a driver to write (issue #12070). Each writing task is now
a `[YOU WRITE: …]` cue, as the Chinese track did in #17014: the narration
defers it ("[once you have stopped driving — write: …]") and the book prints
it as "*Write it:* …". The cue does not create a writing block, so every
lesson stays drivable.

- **Lessons:** JA-C01-practice; JA-C08-hear-sayounara, -sayounara;
  JA-C09-ichido, -mou, -mou-ichido-onegaishimasu, -onegaishimasu,
  -sumimasen; JA-C10-itte-kudasai, -koko, -mou-sukoshi, -slower-please,
  -sukoshi, -wakarimashita, -yukkuri; JA-C11-ashi, -body-map, -hana, -kao,
  -kuchi, -me-eye, -mimi, -te-hand; JA-C12-atama, -body-map-two, -ha-tooth,
  -kami, -kata, -koshi, -onaka, -senaka; JA-C13-ane, -ani, -chichi, -haha,
  -imouto, -kodomo, -otouto, -otto, -tsuma; JA-C131-chotto, -isha, -ocha,
  -ocha-o-kudasai, -toshokan; JA-C132-kore, -kuruma, -soko, -sore, -soto;
  JA-C133-eki, -heya, -ike, -inu, -kesa, -kinou, -nuno; JA-C14-go, -ichi,
  -ni, -san, -yon; JA-C15-juu, -ku, -nana, -roku; JA-C16-futatsu, -hitotsu,
  -itsutsu, -mittsu; JA-C17-kokonotsu, -muttsu, -nanatsu, -too, -yattsu;
  JA-C18-hitori-futari, -hon, -nin; JA-R12-body-01, -doorway, -farewell,
  -foundation-01, -foundation-02, -mixed-scripts, -repair-01 to -04,
  -writing-01, -writing-02; JA-R13-family-a to -d, -tsu-contrast;
  JA-R131-tea-please, JA-R132-this-and-that, JA-R133-last-four,
  JA-R134-phone-and-train, JA-R135-hospital-and-bank,
  JA-R136-family-and-water, JA-R137-pencil-and-ticket.
- A spoken half stays prose ("Say *sumimasen, yoku wakarimasen*. [YOU WRITE:
  **め** and **ど**, once each]"). A sentence that followed the old
  instruction and is not about the writing ("Retrieve **くち** — mouth.",
  "Then say *a station* — **R2**, five lessons back.") is now its own
  paragraph. Where a warm-up interleaved writing and speaking ("Say **はい**,
  write **語**, recall its Chinese-derived bridge, and write **もうすこし**."),
  each writing step is its own cue in the original order.
- The chained "Hear, point, say, read, then write **X**." of chapter 12 keeps
  its four spoken steps on **X** and ends with the cue; "hide and write"
  becomes "with the word hidden" inside the cue, as in chapter 11's
  "3. Hide it and write both signs." steps.
- "Copy **X**, hide it, and write it from the meaning." (chapters 13 and 14)
  becomes `[YOU WRITE: one copy of **X**; then hide it and write it from the
  meaning]`, so the narrated cue still says the learner copies a model first.
- The four-skill blocks of JA-C01-practice, JA-C08-sayounara,
  JA-C09-mou-ichido-onegaishimasu and JA-C11-body-map labelled their writing
  step `**Write:**`. That label was itself the bare imperative, so the step is
  now just the cue, which the book prints as "*Write it:* …" and which keeps
  the Listen / Speak / Read / Write order. The dictation instructions ("hear
  the word, wait ten seconds") move inside the cue after a dash.
- The numbered steps of JA-R131 to JA-R137 stay one item each, as
  ZH-R21-close's did in #17014. A spoken half that is about the written words
  goes inside the cue ("2. [YOU WRITE: **きのう** and **けさ** from memory,
  and say which one is earlier]"), and so does circling a sign the learner
  has just written ("circle the two signs said *o*"). A spoken half that
  stands alone comes first ("3. Say *heya*. [YOU WRITE: **へや**]"), so a
  driver can still do it.
- Two circling tasks were really questions a listener can answer aloud, so
  they became spoken prompts instead of cues: JA-C09-ichido's "Circle the
  dakuten." now asks which mora carries the dakuten (the last, *do*), and
  JA-C10-mou-sukoshi's "Circle **う**, the second beat of *mō*." now asks the
  learner to name that sign.
- Steps the detector does not match but that still ask for a pen are cues
  too: JA-C10-slower-please's "3. **Read/write:** …" is now a **Read:** step
  and a cue; JA-R12-foundation-02's "Build the five and mouth components of
  **語**" and JA-R13-family-a's "Build distant **語** once" are inside cues;
  JA-R12-writing-01's "Add dakuten to a known base" and JA-R13-family-d's
  "Add distant **ー** after **コ**" open their cues.
- Warm-ups that wrote a sign and then said it ("Before the new word: write
  **お**, then **ち**, and say each one.", JA-C131-ocha; also JA-C132-soko and
  JA-C133-eki) now say the sounds first and end with the cue, so the
  speaking can happen while driving.
- JA-C17-nanatsu's Grammar Lens said "Write the two rows out and the pattern
  is exact:" before a table; it now reads "Set the two rows side by side".
- JA-C01-practice's review pulse had lost its kana when it was authored
  ("From memory, write ??, ???, ?????, ?????, ???, and ????."). The cue now
  names the six words that fit both the six lengths and the block's
  `assesses` list (hiragana, kanji bridge, katakana loan, dakuten):
  **はい**, **いいえ**, **こんにちは**, **ありがとう**, **日本語** and
  **コーヒー**. The same `?` damage in JA-C01-iie, JA-W01-ko, -n, -wa,
  JA-W03-sa, -ka, -dakuten, JA-W05-mouth-component and -nichi-kanji is not
  drivable debt and is left for its own change.
- The lessons leave `tests/drivable-writing-debt/` in human-language-data;
  this track has no debt left, so its ledger file is deleted.
- Regenerated: book chapters 7-18 and 131-137, narration (`.json` and
  `.txt`), their generated book and narration hashes, and each lesson's
  `core/lesson-modality` owner (source hash only; all still `drivable: true`).

## はい: the writing step matches its strip

JA-W01-hai-read's Script section told the reader "Now write them touching:",
but the lesson's stroke-order filmstrip draws は and then い, each in panels
of its own, and the rest of the lesson ("Two signs, side by side", "**ha**
then **i**") teaches them one after the other. The sentence now reads "Now
write them side by side, one sign and then the other:". Nothing else in the
lesson changes: no atom, stroke count or activity. Book chapter 1, narration
ch01 and the lesson's modality owner are regenerated.

## Stroke order for 言, 五 and 口, read in three existing words

The last three writing lessons without a stroke-order filmstrip,
JA-W05-gen-component, -five-component and -mouth-component, write **言**,
**五** and **口** on their own before chapter 5 assembles **語**. They had no
inventory row, and a cited stroke order needs a row to belong to. A row
counts against the track's ratchet of inventory letters read in no word
(nine, which may only fall) unless some word headword reads the glyph.

The track already teaches a word for each of the three, spelled in kana.
Those three word lessons now spell their headword with the kanji the reader
has been able to write since chapter 5, and show the kana reading beside it:

| lesson | headword was | headword is | shown as |
|---|---|---|---|
| JA-C14-go (chapter 14) | ご | 五 | **五** (ご) — *go* — five |
| JA-C11-kuchi (chapter 11) | くち | 口 | **口** (くち) — *kuchi* — mouth |
| JA-C28-iu (chapter 28) | いう | 言う | **言う** (いう) — *iu* — to say |

Each lesson says where the kanji comes from (the chapter-5 component), keeps
its kana practice, and accepts the kana spelling as well as the kanji in its
wrap-up answer. JA-C11-kuchi and JA-C28-iu now also practise the mouth and
speech component atoms they write. No lesson is added, moved or retagged,
and no knowledge atom is introduced, so the continuity, closure and
chapter budgets are unchanged.

Adding new word lessons instead was measured and set aside: くち and いう
already exist, so new lessons would have taught the same words twice; a word
in chapter 5 after the components would make 言 "builds-toward" and raise
that ratchet, and one before them would show 言 before it is taught; and
chapters 10 and 11 are at the twelve-atom chapter budget.

**The three rows** cite KanjiVG's directed paths for their own code points
(kanji/08a00.svg, 04e94.svg, 053e3.svg), the same order and directions as
語's own 言, 五 and 口:

| sign | strokes, in order |
|---|---|
| 言 | top mark; long bar; two short bars; the box (left side, top turning down the right side, base) |
| 五 | top bar; a stroke falling from it down and to the left to the base; the middle bar turning down; the long base bar |
| 口 | left side; top turning down the right side; base |

Only the order and direction come from KanjiVG; the pen paths follow the
bundled font's standalone glyphs. KanjiVG draws 言's top mark as a dot
falling to the right, where the print glyph has a short bar, so the path
runs along the bar; the record says so. All three lessons now print a
filmstrip, and so do all 82 Japanese writing lessons.

**JA-W05-five-component is corrected to match its own filmstrip.** It called
五's second stroke "a short stroke down, leaning left"; standalone, that
stroke runs from the top bar down to the bottom line, as KanjiVG's s2 does.
It now reads "a stroke from it down to the bottom line, leaning left", and
its feedback says "down stroke". No assessment answer changes there.

The letter-anchoring ratchet stays at nine (ア イ ト ハ ル ン 今 有 難). The
book, narration and modality outputs of chapters 5, 11, 14 and 28 are
regenerated.

## Stroke order for the katakana and kanji of the writing lessons

Five writing lessons taught a katakana or kanji sign with an inventory row
and printed no stroke-order filmstrip: **コ** and **ヒ**
(JA-W06-ko-katakana, JA-W06-hi-katakana) and **語**, **日** and **本**
(JA-W05-go-kanji, -nichi-kanji, -hon-kanji). Their rows said only
"authoritative". Each now cites KanjiVG's directed paths for its own code
point, and all five lessons now print a filmstrip.

| sign | KanjiVG | strokes, in order |
|---|---|---|
| コ | kanji/030b3.svg | the top bar turning down the right side; the base bar |
| ヒ | kanji/030d2.svg | the short bar, left to right, rising; the vertical turning right along the base |
| 日 | kanji/065e5.svg | the left side down; the top turning down the right side; the middle bar; the base |
| 語 | kanji/08a9e.svg | 言 (top mark, three bars, its box), then 五 (top bar, a short stroke falling to the left, the middle bar turning down, the long base bar), then 口 (left side, top turning down, base) |
| 本 | kanji/0672c.svg | the bar; the stem; the left sweep; the right sweep; the short lower bar |

Only the order and direction come from KanjiVG; the pen paths follow the
bundled font's own outline. KanjiVG draws the top mark of 語's 言 as a dot
falling to the right, where the print glyph has a short bar, so the path
runs along the bar from left to right; the record says so.

**言, 五 and 口 still print no filmstrip.** Chapter 5 also writes them on
their own (JA-W05-gen-component, -five-component, -mouth-component), but
they have no inventory row, and a stroke order needs a row to belong to. No
word lesson spells them alone, so adding rows would raise the track's count
of inventory letters read in no word, which may only fall. For now they are
drawn as the parts of 語, in JA-W05-go-kanji's filmstrip.

**Two lessons are corrected to match KanjiVG.** JA-W06-hi-katakana said
ヒ's first stroke runs "from right toward left"; it runs from left to right,
rising a little. JA-W05-five-component put 五's turn on the second stroke
("a short vertical and turn; a second horizontal"); the second stroke is a
short stroke falling to the left, and the turn ends the middle bar, as 語's
filmstrip shows. Its trace line and feedback, and the recall line "top,
turn, middle, long bottom" in JA-C14-go, now read "top, down, middle and
turn, (long) bottom". No assessment answer changes and no lesson is added,
moved or retagged. The generated LaTeX of chapters 5 and 6 gains the five
figures, and the book and narration of chapters 5, 6 and 14 are
regenerated.

## Stroke order for the voiced kana, ゛, ゜ and ー

Seventeen writing lessons taught a voiced kana and printed no stroke-order
filmstrip: **ど** and **だ** (JA-W09-do, JA-W11-da), the eight of chapters 134
and 135, and the seven of chapters 136 and 137. Three more taught the marks
themselves: the dakuten **゛** (JA-W03-dakuten), the handakuten **゜**
(JA-W18-handakuten) and the long-vowel mark **ー** (JA-W06-long-mark). The
voiced rows said only "authoritative", the mark records had no stroke order at
all, and ど and だ had no row, being covered only through their decomposition.
Every voiced kana row the inventory holds (が ぎ ぐ げ ご ざ ず ぜ ぞ だ で ど ば
び ぶ べ ぼ ぱ ぴ ぷ ぺ ぽ, 22 in all, with だ and ど added) and the three mark
records now cite KanjiVG's directed paths for their own code point. All
twenty lessons now print a filmstrip.

KanjiVG draws a voiced kana as one glyph: the base sign's own paths, the same
as in its file for the base sign, then the mark.

| part | KanjiVG | order and direction |
|---|---|---|
| dakuten ゛ | kanji/0309b.svg, and the last two paths of every voiced kana | two ticks, the left one first, each drawn down to the right |
| handakuten ゜ | kanji/0309c.svg, and the last path of ぱ ぴ ぷ ぺ ぽ | one ring, from its foot, clockwise: round the left, over the top, down the right |
| long-vowel mark ー | kanji/030fc.svg | one bar, left to right (horizontal writing) |

So a dakuten adds two strokes and two pen lifts to the base sign, and a
handakuten one of each: が has five strokes and four pen lifts, ぱ four
strokes and three.

Only the order and direction come from KanjiVG. The pen paths follow the
bundled font's own outline: each base sign's fitted path, moved to where the
voiced glyph's outline puts that sign, and re-fitted to its medial line; the
mark fitted on its own contours.

**Where KanjiVG and an older base row disagree, the voiced glyph follows
KanjiVG and says so.** Three base rows cite Sirgazil's animations, and on
three points KanjiVG's files for both the base and the voiced sign differ
from them:

- **ぜ**: KanjiVG draws せ's right stem (with its hook) second and the left
  stem third. The せ row has them the other way round.
- **ぶ, ぷ**: KanjiVG draws ふ's lower-left mark up to the right. The ふ row
  draws it down and to the left.
- **ぼ, ぽ**: KanjiVG's left vertical of ほ ends in a flick up to the right,
  as は's does. The ほ row says it hooks left.

The せ, ふ and ほ rows and their filmstrips are not changed here. JA-W136-ze
asks the reader to write せ "in the order you learned it", which is the せ
row's order; its filmstrip now shows KanjiVG's. Reconciling the three base
rows with KanjiVG is left as follow-up work.

No lesson text changes, no assessment answer changes, and no lesson is added,
moved or retagged. The generated LaTeX of chapters 3, 6, 9, 11, 18 and 134 to
137 gains the twenty figures.

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

## Chapters 136-137: the z row and the p row, ず, ぜ, ぞ, ぱ, ぴ, ぷ and ぺ

After chapter 135 the book wrote the whole *g* and *b* rows, but of the *z*
row only **ざ** and **じ**, and of the *p* row only **ぽ**. Script closure
counts the precomposed sign, so A2 words such as **かぞく** (a family),
**みず** (water) and **きっぷ** (a ticket) still could not be written.
Chapters 136 and 137 add 21 lessons and write seven more signs. The three
*z* signs are a sign the reader already writes plus the two-stroke dakuten,
like **ざ**. The four *p* signs are an *h*-row sign plus the small circle of
the handakuten, like **ぽ**, which the book has written since chapter 18 and
used on no other sign until now. Each writing lesson pairs the new sign with
its base sign. With them the book writes the whole *z* row (ざ じ ず ぜ ぞ)
and the whole *p* row (ぱ ぴ ぷ ぺ ぽ). Only the two rare *d* signs of the *t*
row are still unwritten.

**Word before sign, every time.** As in chapters 131-135, each new sign is
first seen inside a word taught one lesson earlier, with a romanization, and
only then written:

| lesson | what it does |
|---|---|
| **かぞく** (*kazoku*, a family) | shows ぞ between か and く |
| writing **ぞ** | そ plus the dakuten, as さ became ざ |
| **かぞえる** (*kazoeru*, to count) | spends ぞ; a verb (JA-VERB-KAZOERU) |
| **みず** (*mizu*, water) | shows ず after み |
| writing **ず** | す plus the dakuten |
| **すずしい** (*suzushii*, cool) | spends ず, with plain す beside it |
| **かぜ** (*kaze*, a wind; a cold) | shows ぜ; one word in kana, two in kanji |
| writing **ぜ** | せ plus the dakuten; the *z* row is complete |
| review (chapter 136) | reads the five words, writes each new sign beside its base sign, and かぞく, かぞえる and かぜ |
| **いっぱい** (*ippai*, full; a lot) | shows ぱ after the held beat of っ |
| writing **ぱ** | は plus the handakuten, as ほ became ぽ in いっぽん |
| **しんぱい** (*shinpai*, worry) | spends ぱ |
| **えんぴつ** (*enpitsu*, a pencil) | shows ぴ |
| writing **ぴ** | ひ plus the handakuten; ひ, び and ぴ side by side |
| **いっぴき** (*ippiki*, one small animal) | spends ぴ; the animal counter, beside いっぽん |
| **きっぷ** (*kippu*, a ticket) | shows ぷ |
| writing **ぷ** | ふ plus the handakuten, read *pu* |
| **てんぷら** (*tenpura*, tempura) | spends ぷ |
| **ぺらぺら** (*perapera*, fluent) | shows ぺ, twice |
| writing **ぺ** | へ plus the handakuten; the *p* row is complete |
| review (chapter 137) | reads the seven words, writes each new sign beside its base sign, and えんぴつ, きっぷ and ぺらぺら |

Every word is new to the corpus (no headword or homograph existed) and uses
only kana the book already writes, plus the one new sign. **かぞえる** is the
one verb. Two lessons separate each writing lesson from the next. Chapter
136 sits on the everyday-things spine node, like chapter 134; chapter 137,
with its ticket, pencil and tempura, sits on saying what I want.

All seven letter lessons are **anchored** (HL-C443; 32 -> 39). Cold,
builds-toward, unwritten and unread inventory stay at 1, 35, 0 and 9. Script
closure stays at zero violations and zero never-taught glyphs (taught glyphs
77 -> 84). Each of the seven gets an inventory row: the *z* signs in the form
ざ uses (base sign in full, then the two short strokes at the upper right),
the *p* signs in the form ぽ uses (base sign in full, then a small circle at
the upper right). None cites a stroke-order source of its own, so none has a
ductus or a filmstrip.

**Reinforcement.** Twenty-one more lessons make 43 older (atom, window)
slots measurable for the first time: R4 for 21 words from chapters 126-130,
R3 for 18 atoms from chapters 134-135, and R2 for four atoms from chapter
135. Each new lesson's warm-up retrieves the R4 atom due at its position at
exactly 80 lessons and, where one is due, the R3 atom at exactly 20. The
first four take the chapter 135 atoms at five. Inside the two chapters, every
new atom is retrieved one lesson later and, where the track is long enough to
judge it, five lessons later; **かぞく**, the first new word, is retrieved
again twenty lessons later in the chapter 137 review. The R1, R2, R3 and R4
miss counts stay at 1, 459, 245 and 519, with no per-atom change. Every new
atom except ぺ, written in the second-to-last lesson, is revisited at least
twice. Japanese stays at A1.

## Chapters 134-135: eight voiced kana, で, ば, べ, ぶ, び, ぐ, げ and ぎ

After chapter 133 the book wrote every basic hiragana, but only seven voiced
ones: **が**, **ご**, **ざ**, **じ**, **だ**, **ど** and **ぼ** (and **ぽ** with
the handakuten). Everyday A2 words need more: **でんわ** (a telephone),
**べんきょう** (study), **ぎんこう** (a bank) and **げんき** (well) could not be
written at all. Chapters 134 and 135 add 24 lessons and write eight more,
four per chapter. Each one is a sign the reader already writes plus the
two-stroke dakuten, and each writing lesson pairs it with that base sign. They
are the first new voiced signs since chapter 18. With them the book writes the
whole *g* row (が ぎ ぐ げ ご) and the whole *b* row (ば び ぶ べ ぼ).

**Word before sign, every time.** As in chapters 131-133, each new sign is
first seen inside a word taught one lesson earlier, with a romanization, and
only then written:

| lesson | what it does |
|---|---|
| **でんわ** (*denwa*, a telephone) | shows で after ん and わ |
| writing **で** | て plus the dakuten, as た became だ and と became ど |
| **でんしゃ** (*densha*, a train) | spends で at once |
| **かばん** (*kaban*, a bag) | shows ば between か and ん |
| writing **ば** | は plus the dakuten; on an *h* sign the mark makes a *b*, as on ほ for ぼ |
| **ばしょ** (*basho*, a place) | spends ば |
| **たべる** (*taberu*, to eat) | shows べ; a verb (JA-VERB-TABERU) |
| writing **べ** | へ plus the dakuten |
| **べんきょう** (*benkyō*, study) | spends べ |
| **しんぶん** (*shinbun*, a newspaper) | shows ぶ |
| writing **ぶ** | ふ plus the dakuten, read *bu* |
| review (chapter 134) | reads the seven words, writes each new sign beside its base sign, and でんしゃ, ばしょ and しんぶん |
| **びょういん** (*byōin*, a hospital) | shows び; keep the ょ small, or the word is *biyōin*, a hair salon |
| writing **び** | ひ plus the dakuten; the *b* row is complete |
| **どようび** (*doyōbi*, Saturday) | spends び |
| **いりぐち** (*iriguchi*, an entrance) | shows ぐ; *guchi* is *kuchi*, a mouth, voiced |
| writing **ぐ** | く plus the dakuten |
| **でぐち** (*deguchi*, an exit) | spends ぐ, and で from chapter 134 |
| **げつようび** (*getsuyōbi*, Monday) | shows げ; ends in the *yōbi* of どようび |
| writing **げ** | け plus the dakuten |
| **げんき** (*genki*, well, healthy) | spends げ |
| **ぎんこう** (*ginkō*, a bank) | shows ぎ |
| writing **ぎ** | き plus the dakuten; the *g* row is complete |
| review (chapter 135) | reads the seven words, writes each new sign beside its base sign, and どようび, げつようび and ぎんこう |

Every word is new to the corpus and uses only kana the book already writes,
plus the one new sign. Two lessons separate each writing lesson from the
next. Chapter 134 sits on the everyday-things spine node, like chapters 132
and 133; chapter 135, with its hospital, entrance, exit and bank, sits on
asking where things are.

All eight letter lessons are **anchored** (HL-C443). Cold, builds-toward,
unwritten and unread inventory stay at 1, 35, 0 and 9. Script closure stays
at zero violations and zero never-taught glyphs: closure counts the
precomposed sign, so each new sign appears only in its own headword until its
writing lesson. Each of the eight also gets an inventory row in the form
が, ご, ざ and ぼ already use (base sign in full, then the two short strokes
at the upper right). None of them cites a stroke-order source of its own, so
none has a ductus or a filmstrip.

**Reinforcement.** Twenty-four more lessons make 46 older (atom, window)
slots measurable for the first time: R4 for 24 words from chapters 121-126,
R3 for 18 atoms from chapters 132-133, and R2 for four atoms from chapter
133. Each new lesson's warm-up retrieves the R4 atom due at its position at
exactly 80 lessons and, where one is due, the R3 atom at exactly 20. The
first four take the chapter 133 atoms at five. Inside the two chapters, every
new atom is retrieved one lesson later and, where the track is long enough to
judge it, five lessons later; the first four chapter 134 atoms are retrieved
again twenty lessons later, at the end of chapter 135. The R1, R2, R3 and R4
miss counts stay at 1, 459, 245 and 519, with no per-atom change. Every new
atom except ぎ, written in the second-to-last lesson, is revisited at least
twice. Japanese stays at A1.

## Stroke order for に, は, ま, り and ん

The last five basic hiragana without a cited stroke order were **に**, **は**,
**ま**, **り** and **ん** (JA-W01-ni, JA-W01-ha, JA-W03-ma, JA-W03-ri and
JA-W01-n), taught since chapters 1 to 4. Their inventory rows said only
"authoritative", so they had no ductus and their writing lessons printed no
filmstrip. Each row now cites KanjiVG's directed paths for its own code point,
the way こ to と do. All five lessons now print a filmstrip.

| sign | KanjiVG file | paths | pen lifts |
|---|---|---|---|
| に | kanji/0306b.svg | the left vertical with its flick, the upper stroke, the lower curve | 2 |
| は | kanji/0306f.svg | the left vertical with its flick, the bar, the vertical with its loop | 2 |
| ま | kanji/0307e.svg | the upper bar, the lower bar, the vertical with its loop | 2 |
| り | kanji/0308a.svg | the left stroke with its flick, the long right stroke | 1 |
| ん | kanji/03093.svg | one stroke: the diagonal, back up it, the hump, the rising finish | 0 |

**Every one of the 46 basic hiragana, あ to ん with を, now has a cited stroke
order and a ductus**, and the script-ductus and inventory tests check all 46.

Only the order and direction come from KanjiVG. The pen paths follow the
bundled font's own outline. In the print glyph the flick of に's and は's left
vertical leaves the stroke a little above its foot, and ん's hump leaves the
diagonal partway up, so the path goes down to the end of that ink and climbs
back before turning. り's print glyph draws the pen's way from the left stroke
to the right one as a thin rising arch, so in the filmstrip the two strokes
touch at the top of that arch, though in KanjiVG they do not.

**Three lessons are corrected**, each only where its text contradicted both
KanjiVG and the filmstrip now printed beside it:

- JA-W01-ha said は's left vertical ends with a flick to the left, that the
  crossbar crosses that vertical, and that the loop starts at the crossbar's
  right and comes back up to close. The flick goes up to the right; the
  crossbar sits to the right of the vertical without touching it; and the last
  stroke starts above the crossbar, comes down through it, rounds to the left
  at the foot, comes back up across itself and runs out to the lower right.
  The three list items now say so.
- JA-W01-ni said に's left vertical has "the same small flick left at the
  foot" as は. It is the same flick, but it goes up to the right. That list
  item now says so.
- JA-W01-n began ん with "a small tick down-left". The stroke cuts all the way
  from the top down to the lower left, climbs back up the same line and turns
  over a small hump before rounding to the right and rising. That sentence now
  says so.

No assessment answer changes, and no lesson was added, moved or retagged. The
ま and り lessons' prose is unchanged. JA-W03-ri still says the two strokes of
り never touch, which KanjiVG's handwriting agrees with even though the print
glyph joins them.

## Stroke order for こ, さ, す, ち and と

Five more signs that chapters 2, 3 and 4 teach had no cited stroke order:
**こ**, **さ**, **す**, **ち** and **と** (JA-W01-ko, JA-W03-sa, JA-W03-su,
JA-W01-chi and JA-W03-to). Their inventory rows said only "authoritative", so
they had no ductus and their writing lessons printed no filmstrip. Each row now
cites KanjiVG's directed paths for its own code point, the way あ to か do. All
five lessons now print a filmstrip.

| sign | KanjiVG file | paths | pen lifts |
|---|---|---|---|
| こ | kanji/03053.svg | the upper stroke, the lower curve | 1 |
| さ | kanji/03055.svg | the bar, the slanting stroke with its hook, the curve at the foot | 2 |
| す | kanji/03059.svg | the bar, the vertical with its loop and tail | 1 |
| ち | kanji/03061.svg | the bar, the body in one movement | 1 |
| と | kanji/03068.svg | the short stroke, the long sweep | 1 |

Only the order and direction come from KanjiVG. The pen paths follow the
bundled font's own outline. In the print glyph, す's vertical and the right
side of its loop share one band of ink, so the path comes back down that band
before the tail. ち's descender meets the rising stroke in a sharp point, so
the path goes down to that tip and turns back up the same ink.

**Three lessons are corrected**, each only where its text contradicted both
KanjiVG and the filmstrip now printed beside it:

- JA-W03-sa called さ "two strokes" in its gloss and title, though its own
  body counts three. It also described the second stroke as a short curve
  below the bar, falling to the left, and the foot curve as opening left. The
  second stroke starts above the bar, slants down to the right through it and
  hooks back to the left; the foot curve comes down on the left and runs out
  to the right. The title, gloss, the two list items and the guided-practice
  cue now say so. The paragraph about printed fonts is unchanged.
- JA-W01-chi described ち's body as curving left and then round into a belly
  "to the right and up". It falls through the bar to the lower left, turns
  sharply up to the right, rounds the belly and finishes low on the left. That
  one list item now says so.
- JA-W03-to said と's two strokes never meet. In KanjiVG the short stroke ends
  on the long one, and the print glyph joins them there. The title, the
  opening hook, the line introducing the strokes, the second list item and
  the guided-practice cue now say the strokes touch, and the recall activity's
  answer moves from "no" to "yes", with its accepted answers and feedback to
  match.

No lesson was added, moved or retagged, and the other two lessons' prose is
unchanged.

Five basic hiragana rows (に, は, ま, り and ん) still have no cited
stroke-order source, so those signs have no ductus and no filmstrip yet.

## Stroke order for あ, い, う, え, お and か

Six signs that chapters 1, 3 and 10 teach had no cited stroke order: **あ**,
**い**, **う**, **え**, **お** and **か** (JA-W03-a, JA-W01-i, JA-W03-u,
JA-W01-e, JA-W10-o and JA-W03-ka). Their inventory rows said only
"authoritative", so they had no ductus and their writing lessons printed no
filmstrip. Each row now cites KanjiVG's directed paths for its own code
point, the way き, け, ぬ, へ and ら do. All six lessons now print a filmstrip.

| sign | KanjiVG file | paths | pen lifts |
|---|---|---|---|
| あ | kanji/03042.svg | the bar, the vertical, the looping stroke | 2 |
| い | kanji/03044.svg | the long left stroke with its flick, the short right stroke | 1 |
| う | kanji/03046.svg | the short top stroke, the wide curve | 1 |
| え | kanji/03048.svg | the short top stroke, the body in one movement | 1 |
| お | kanji/0304a.svg | the bar, the long vertical with its loop and bowl, the dot | 2 |
| か | kanji/0304b.svg | the bar with its turn and hook, the long falling stroke, the dot | 2 |

Only the order and direction come from KanjiVG. The pen paths follow the
bundled font's own outline. In the print glyph, え's small hump branches off
its diagonal, so the path goes down to the tip of the diagonal and climbs back
up the same ink before turning over the hump.

**One lesson is corrected.** JA-W03-ka described the first stroke of か as
going down and curving right at the foot, and the second as a short vertical
falling to the right. That is not how か is written, and the new filmstrip
shows otherwise. The first stroke is a bar that turns down the right side and
hooks back to the left. The second is a long stroke falling to the lower left
through the bar. The lesson now says so, and its two feedback lines name the
strokes the same way. Nothing else in the six lessons changes: no lesson was
added, moved or retagged.

Ten basic hiragana rows (こ, さ, す, ち, と, に, は, ま, り and ん) still have
no cited stroke-order source, so those signs have no ductus and no filmstrip
yet.

## Chapter 133: き, け, ぬ and へ, the last four basic hiragana

After chapter 132 the book wrote every sign of the basic hiragana table but
four: **き**, **け**, **ぬ** and **へ**. Its words had been chosen to avoid
them. Chapter 133 adds twelve lessons and writes all four. They were the last
basic hiragana without a writing lesson.

ら had the opposite gap: chapter 8 has a writing lesson for it, but the script
inventory had no row for ら, so it had no stroke-order source and no
filmstrip. Chapter 133 adds that row, cited to KanjiVG like the other four,
and a ductus. JA-W08-ra now prints a filmstrip; its prose is unchanged.
Every basic hiragana now has a writing lesson and an inventory row, but
sixteen rows (あ, い, う, え, お, か, こ, さ, す, ち, と, に, は, ま, り and
ん) still have no cited stroke-order source, so those signs have no ductus
and no filmstrip yet.

**Word before sign, every time.** Each new sign is first seen inside a word
taught one lesson earlier, and only then written:

| lesson | what it does |
|---|---|
| **えき** (*eki*, a station) | shows き after the え the reader writes |
| writing **き** | さ with a second bar |
| **きのう** (*kinō*, yesterday) | spends き at once |
| **いけ** (*ike*, a pond) | shows け after い |
| writing **け** | the first two strokes of は, then a sweep to the lower left |
| **けさ** (*kesa*, this morning) | spends け; sits beside きのう |
| **いぬ** (*inu*, a dog) | shows ぬ, which looks like め with a loop |
| writing **ぬ** | め with a small closed loop at the end |
| **ぬの** (*nuno*, cloth) | spends ぬ |
| **へや** (*heya*, a room) | shows へ, the sign the は, ひ, ふ, ほ row was missing |
| writing **へ** | one stroke over a low peak |
| review | reads the seven words, writes きのう, けさ and へや, and each new sign once |

All four letter lessons are **anchored** (HL-C443), and cold, builds-toward,
unwritten and unread inventory stay at 1, 35, 0 and 9. Two lessons separate
each writing lesson from the next. Script closure stays at zero violations,
because each word lesson declares a romanization and its new sign appears only
in that word.

**Where the stroke orders come from.** All four follow を, そ, れ and る.
KanjiVG's directed paths for U+304D, U+3051, U+306C and U+3078 give the order
and direction: き is four paths, け three, ぬ two and へ one. ら's record uses
U+3089's file, which has two paths. The pen paths
follow the bundled font's own outline. Where the print glyph has no separate
ink for a return, the path retraces the ink: け climbs back a short way from
the foot of its left stroke into the flick.

**Reinforcement.** Twelve more lessons make 23 older (atom, window) slots
measurable for the first time: R4 for ten words from chapters 119-121, R3 for
nine atoms from chapters 131-132, and R2 for four atoms from chapter 132. Each
new lesson's warm-up retrieves the R4 and R3 atoms due at its own position, at
exactly 80 and 20 lessons, in their original order, and the first four take
the chapter 132 atoms at five. Every new atom is also retrieved one lesson
later and, where the chapter is long enough to judge it, five lessons later.
The miss counts stay at R2 459, R3 245 and R4 519. Every new atom except へ,
written in the second-to-last lesson, is revisited at least twice inside the
chapter; へ is used again in the review. Japanese stays at A1.

## Chapter 132: そ, れ and る, each written after a word that holds it

The book's 295 A1 words were chosen to avoid three very common signs: **そ**,
**れ** and **る**. Chapter 131 used the same approach to add small ゃ, small ょ
and を. Chapter 132 adds nine lessons and writes these three. The pointing words
**そこ**, **これ** and **それ** sit beside the **ここ** the reader already knows.

**Word before sign, every time.** Each new sign is first seen inside a word
taught one lesson earlier, and only then written:

| lesson | what it does |
|---|---|
| **そこ** (*soko*, there) | shows そ beside the こ of ここ |
| writing **そ** | one stroke that zigzags twice before its curve |
| **そと** (*soto*, outside) | spends そ at once |
| **これ** (*kore*, this) | shows れ: a thing near you, as ここ is a place |
| writing **れ** | the first stroke and zigzag of わ and ね, with a flick at the end |
| **それ** (*sore*, that) | spends れ and そ together; the four pointing words line up |
| **くるま** (*kuruma*, a car) | shows る between two signs the reader writes |
| writing **る** | ろ with a small closed loop at the bottom |
| review | reads the five words, points with *kore* and *sore*, writes くるま |

The signs come in the order そ, れ, る so that それ can use the first two. All
three letter lessons are **anchored** (HL-C443), and cold, builds-toward,
unwritten and unread inventory stay at 1, 35, 0 and 9. Two lessons separate
each writing lesson from the next. Script closure stays at zero violations,
because each word lesson declares a romanization and its new sign appears only
in that headword.

**Where the stroke orders come from.** All three follow を. KanjiVG's
directed paths for U+305D, U+308C and U+308B give the order and direction:
そ and る are one path each, and れ is two. The pen paths follow the bundled
font's own outline. Where the print glyph has no separate ink for a return,
the path retraces the ink: そ doubles back along its middle bar, and れ climbs
back up its diagonal.

**Reinforcement.** Nine more lessons make 22 older (atom, window) slots
measurable for the first time: R4 for nine words from chapters 117-119, R3 for
nine qualities from chapters 129-130, and R2 for four atoms from chapter 131.
Each new lesson's warm-up retrieves one R4 word and one R3 word, at exactly 80
and 20 lessons, in their original order. Four warm-ups also take the chapter
131 atoms, each at five lessons. All 22 slots are served, so the miss counts
stay at R2 459, R3 245 and R4 519. Every new atom except the last sign is
revisited at least twice inside the chapter. る, written in the second-to-last
lesson, is used again in the review. Japanese stays at A1.

## Chapter 131: small ゃ, small ょ and を, each written after a word that holds it

The book wrote every hiragana sign its words used, but three common ones were
still missing: the small **ゃ** and **ょ** that fold a sign into one beat, and
**を**, the object particle. Chapter 131 adds nine lessons.

**Word before sign, every time.** Each new sign is first seen inside a word taught
one lesson earlier, and only then written:

| lesson | what it does |
|---|---|
| **おちゃ** (*ocha*, tea) | shows ゃ: two beats, *o–cha* |
| writing **ゃ** | や written small, in the same three strokes |
| **いしゃ** (*isha*, a doctor) | spends ゃ at once |
| **ちょっと** (*chotto*, a little) | shows ょ beside the っ the reader already writes |
| writing **ょ** | よ written small; with ゃ and ゅ the set is complete |
| **としょかん** (*toshokan*, a library) | spends ょ at once |
| **おちゃを ください** (*ocha o kudasai*, tea, please) | the object particle, said *o* |
| writing **を** | said like お, kept for the particle only, as は is kept for *wa* |
| review | reads the four words, writes the request from memory |

So all three letter lessons are **anchored** (HL-C443). None of them waits for
a later word, and the track's cold and builds-toward counts do not move. Two
lessons separate each writing lesson from the next. Script closure stays at
zero violations, because each word lesson declares a romanization and its new
sign sits only in that headword.

**Where the stroke orders come from.** Small ゃ and ょ use the rule small ゅ
set: the full-size sign's observed order and its citation, fitted to the small
glyph, and an explicit note that the size change is not separate evidence.
KanjiVG's files for U+3083 and U+3087 hold the same strokes in the same order.
を takes its order and direction from KanjiVG's three directed paths, and its
pen path follows the bundled font's own outline.

**Reinforcement.** Nine more lessons make 21 older (atom, window) slots
measurable for the first time: R4 for nine words from chapters 115-117, R3
for nine qualities from chapters 127-129, and R2 for three from chapter 130.
Each new lesson's warm-up retrieves one R4 word and one R3 word, at exactly 80
and 20 lessons, in their original order; three warm-ups also take the R2
words. All 21 are served, so the miss counts stay at R2 459, R3 245 and R4
519. Every new atom is revisited at least twice inside the chapter, and
Japanese stays at A1.

## Chapter payoffs say "I can", not "i can"

The payoff line under each chapter's goal lowercased the goal's first letter,
so 111 chapters printed "Complete the last lesson of chapter N: i can say …".
Every one now keeps the capital: "…: I can say …", and a new test
(`payoff-summary-case.test.ts`) fails if the lowercase pronoun comes back. Only
the payoff summary changed; no lesson, word or atom moved.

## Chapters 72-130: 295 hiragana words, and Japanese attains A1

Japanese had four A1 gaps:

- ten of the twelve A1 spine nodes had no lesson;
- it taught 330 headwords against 600;
- it taught 27 verbs against 40;
- four atoms were revisited fewer than twice.

**The words.** Fifty-nine chapters of five, closed by twelve review lessons:

- **time:** **ごご** (*gogo*), **まよなか** (*mayonaka*), **ことし**
  (*kotoshi*), and the first six days of the month, from **ついたち**
  (*tsuitachi*) to **むいか** (*muika*) (TIME-OF-DAY)
- **where:** **どこ** (*doko*), **うしろ** (*ushiro*), **よこ** (*yoko*),
  **かいだん** (*kaidan*), **こうさてん** (*kousaten*) and more (ASK-LOCATION)
- **this and that:** **この**, **あの**, **どの**, **こんな**, **あんな**,
  **どんな** and more (DEFINITE-REFERENCE)
- **actions:** thirty-five verbs, from **うつ** (*utsu*) to **まよう**
  (*mayou*) (NAME-EVERYDAY-ACTIONS)
- **want, like, have and why:** **ほしい** (*hoshii*), **ねがう** (*negau*),
  **このむ** (*konomu*), **しゅみ** (*shumi*), **とくい** (*tokui*),
  **にがて** (*nigate*), **どうして** (*doushite*), **りゆう** (*riyuu*)
  and more (SAY-WHAT-I-WANT, SAY-WHAT-I-LIKE, SAY-WHAT-I-HAVE-AND-CAN-DO,
  SAY-WHY)
- **things:** 105 of them, covering the house, the kitchen, nature, fish,
  animals, the body, and the shops of a town (NAME-EVERYDAY-THINGS)
- **qualities:** fifty-five describing words, from **かたい** (*katai*) to
  **たしか** (*tashika*) (DESCRIBE-QUALITIES)

**The spelling rule, tightened.** Every headword is hiragana. Each sign in it
must satisfy all three of these:

- it has a writing lesson;
- the script data file covers it;
- if it is a precomposed voiced kana, it is one the book teaches (が, ご, ざ,
  ぼ, ぽ, ど, だ).

ら has a writing lesson but no row in the script data, so no word uses it.
Neither do ず, ば, で or the other voiced kana the book has not taught. A
review lesson recalls a word by sound when its spelling would print such a
sign, and each review's typed activity asks for a word the reader can spell.

**The revisits.** Five warm-up lines retrieve the four thin atoms: connected
reading, the counter's sound change (**いっぽん**, **さんぼん**, **ろっぽん**),
reading greetings and reading words.

      japanese lessons     427  ->  734
      japanese headwords   330  ->  625
      level gate        pre-A1  ->  A1

## HL-C443: わ gets a word first

こんにちは says its "wa" with **は**, so the chapter-2 lesson for **わ** came
before any word holding it. It now follows **わに** (*wani*, a crocodile), a
short romanized word lesson. Its one activity is answered in rōmaji, because
わ is only written in the next lesson. The word is revisited by the わ lesson
and the こんにちは reading lesson, and again in the ありがとう warm-up, inside
its R2 window. No older atom loses a window.

め stays cold: its chapter is at its twelve-atom budget.

    japanese cold letter lessons   2  ->  1 (ceiling ratcheted)
    japanese lessons             426  ->  427

## Chapters 20-71: 260 hiragana words, and Japanese attains pre-A1

Japanese had two pre-A1 gaps: 256 headwords and 5 verbs.

**The vocabulary.** Fifty-two chapters of five words each (twenty-seven verbs),
in three runs of seventeen or eighteen. Each run closes with two reviews, in
chapters 37, 54 and 71. Every word is chained into the next two lessons and
carries one objective activity, a typed recall of the word.

**The spelling rule.** Every headword is hiragana, spelled only with glyphs the
track's script lessons have already taught and that the script data file
covers. That rule decides most of the list:

- Words with き, け, そ, ぬ, へ, る, れ, を or small ゃ/ょ wait.
- So does every voiced sign no script lesson has written yet: び, ば, ぶ, ず,
  ぜ, で, ぐ, ぱ, ぴ.
- ら is taught inside さようなら but has no script-data entry, so it waits too.

That is why the verbs are the ones whose dictionary form avoids る (**いく**,
**かう**, **のむ**, **よむ**, **かく**, **はなす**), and why **みず**, **ゆび**
and **からだ** are not here yet.

The review lessons recall each word by its romanization only. A review has no
romanized headword to make its kana exposure-only, so any kana it printed
would count as load-bearing script.

    japanese pre-A1 blockers   vocabulary 256 + verb-vocabulary 5  ->  none (attained)

## Chapter 19 — reading: words, greetings, and the cues you have only heard

Japanese's reading rung. Three lessons -- six body words, the six doorway
greetings, and the classroom cues -- and no new word or new sign in any of them.

- Every kana was checked against the signs the script ladder has actually
  taught, the same check Urdu's rung needed. Japanese passed it first time,
  which is what nineteen chapters of one-sign-at-a-time buys.
- The middle lesson notes something only hiragana does: **て** and **め** are
  whole words AND single signs, and nothing on the page says which is meant.
  Only knowing the word does.
- **こんにちは** ends in **は** read *wa*, a spelling kept from a sentence nobody
  finishes any more. The lesson says the learner does not need the history --
  only the knowledge that some words are spelled for a reason rather than by a
  rule they can apply.
- The last lesson is the repair kit. わかりました and わかりません differ in three
  signs at the end and nothing else, and a cue you can only recognise when
  somebody says it to you is a cue you can only use when they do.
- pre-A1 reading moves 0/1 -> 1/1, which is the track's only declared reading
  part.

Japanese also leaves the corpus's zero-findings list, and it was the last track
on it. Three reading skills introduced in the final chapter have nothing after
them to retrieve them: seven windows come due and none can be met. The
assertion that used to name japanese now pins the SHAPE of the remaining
finding as well as the empty list, so an empty list cannot pass by being
uninformative.

Filed while here: HL-C371. `reading-reach` counts whitespace-separated tokens,
and Japanese writes no spaces, so a fourteen-sign line counts as one. The rung
passes its part honestly, but the NUMBER beside it is not words.

## [Unreleased]

### Added — chapters 16, 17 and 18: the other count, and what the tsu was doing all along

`JA-A1-NUM-02` read "Japanese-specific and still absent." It is now covered, and
it is the FIRST japaneseSpecific point in this file to close at all — it has no
Spanish column behind it, because Spanish has no classifier system. Coverage
66/179 → **67/179**. Twenty-six lessons in three chapters, twenty-three atoms,
ten new glyphs.

**CHAPTER 14 LEFT A SENTENCE HANGING AND THIS IS WHERE IT LANDS.** `JA-C14-yon`
said of the native word standing inside the borrowed count: *that split is coming
back; it is the same seam the counters run along.* The seam is now shown twice
more and named as one thing:

- **よん / なな** — the native word inside the borrowed COUNT (chapter 14; chapter
  17 shows the other end of the same pair, **よっつ / ななつ**).
- **ひとり / ふたり** — the native word inside a borrowed COUNTER, at exactly one
  and two, with **さんにん** taking over from three.

No new atom was invented for the second: it is `JA-GRAMMAR-KUN-IN-THE-COUNT`,
introduced in chapter 14, doing the same job in a different place.

**THE READER ALREADY OWNED TWO COUNTERS AND NOBODY HAD SAID SO.** The **つ** of
the native series IS the general counter — which is why nine of its ten words
carry it and **とお** does not — and the **ど** of chapter 9's **いちど** is a
counter too. That lesson's own words were *ichi is one, and do counts an
occurrence*; it never used the term. Chapter 18 cashes both, and no
`JA-LEX-DO-COUNTER` atom was invented for the second. The test asserts none
exists.

**THE ORDER IS BY COST, AND IT DELIBERATELY DIFFERS FROM CHAPTER 15's.** Five of
the ten native words — **みっつ よっつ いつつ ななつ とお** — cost no sign at all,
and each of the five signs the others need is bought in the lesson **immediately
before** the word that needs it, so the count grows contiguously. Chapter 15 did
the opposite: it bought both its kana last and left the reader counting round an
audible hole at six for three lessons. That was right there, because the borrowed
count has an exception at six to make legible; it would be wrong here, because
the native series has none and a hole would buy nothing.

**THE SIGNS WERE PRICED FROM THE CORPUS, AND THE PRICE WAS FIVE, NOT FOUR.**
Deriving the taught set at point of use rather than guessing: **ひ**, **ふ**,
**む**, **や** — and **の**, which **ここのつ** needs and which is easy to miss.
The **ここ** at that word's front is the shape of the word for *here* the reader
has had since chapter 10; the lesson names that as an accident rather than
explaining it, because an accident that gets explained is worse than one that
gets named.

**EVERY STROKE ORDER IS OBSERVED, AND THE FRAME COUNTS ARE IN THE INVENTORY.**
Following the standard `ろ` set in chapter 15, each of the six new kana was taken
from Sirgazil's CC0 Commons animations, downloaded and read frame by frame, and
cross-checked against KanjiVG's path data for the same codepoint:

    ひ  33 frames, 3.3 s   1 stroke,  0 pen lifts   marker never leaves the origin
    ふ  31 frames, 3.1 s   4 strokes, 3 pen lifts   tick, body, lower-left, lower-right
    む  32 frames, 3.2 s   3 strokes, 2 pen lifts   the right-hand mark is separate
    や  28 frames, 2.8 s   3 strokes, 2 pen lifts   the SWEEP is first, not the descender
    の  32 frames, 3.2 s   1 stroke,  0 pen lifts   crosses its own line at the end
    ほ  37 frames, 3.7 s   4 strokes, 3 pen lifts   は plus one horizontal, drawn above

**や** is the one worth having looked at: the finished shape says the long
descender was drawn first, and the animation and KanjiVG agree that it is drawn
**last**. That is a claim no author should make from memory.

**THE COUNTERS ARE THE THREE THE CORPUS'S OWN NOUNS CAN EXERCISE.** Not the usual
textbook set: **つ** on the face words of chapters 11-12, **にん** on the nine
family words of chapter 13, and **ほん** on **あし** and **かみ**. Every example
counts something the reader can already name, which is why **まい** — flat things
— is *not* taught: the track has no flat noun, and a counter with nothing to
count is a word rather than a skill.

**ほん COST ONE SIGN AND ALMOST NONE.** The kanji it is written with is **本**,
which the reader has been writing since chapter 5 inside **日本語**, and the
chapter says so. What it had to buy was the kana **ほ** — which is **は** plus one
horizontal — and then the **handakuten ゜**, a mark the script inventory has named
since chapter one and no lesson had ever taught. **いっぽん** is what finally
needed it.

**THE NINE-FORM OF THE COUNTER IS LEFT OUT AND THE PAGE SAYS SO.** The
**いっぽん** table prints one, two, three, four, five, six, seven, eight and ten;
nine's everyday form needs **き**, a sign this book has not taught. Guessing it,
or writing it in romanization inside a table of kana, would both have been worse
than naming the gap.

**REINFORCEMENT, DECOMPOSED ATOM BY ATOM RATHER THAN BY TOTALS:**

    reinforcementWindowMisses          0 ->   0
    reinforcementMissesByWindow-R1     0 ->   0
    reinforcementMissesByWindow-R2     0 ->   0
    reinforcementMissesByWindow-R3     0 ->   0
    reinforcementMissesByWindow-R4     0 ->   0
    atomsTaught                      125 -> 148
    atomsNeverRevisited                0 ->   0
    scriptClosureViolations            0 ->   0
    neverTaughtGlyphs                  0 ->   0
    taughtGlyphs                      50 ->  60

**Zero to zero, in every window.** This is one of two tracks in the corpus with
no reinforcement miss anywhere, and twenty-six lessons did not put one in it. The
tranche's own twenty-three atoms create **zero** debt. Twenty-six lessons make
**47** (atom, window) slots newly judgeable on OLDER atoms — five R2, sixteen R3,
**twenty-six R4** — and all forty-seven are answered. The R4 payments are
arithmetic rather than search: the twenty-six atoms introduced at positions 51-76
have their fourth window open at exactly 131-156, so new lesson *n* services the
atom introduced at *n*−80, one per lesson. Where a better pairing existed it was
taken instead of the diagonal minimum — **いちど**'s R4 lands in the lesson that
first says the word *counter* out loud, and the four body words of chapters 11-12
land in the counting lessons where a reader would actually be counting them.

**THE ATOM-STEP BUDGET FORCED THE SHAPE, AND THAT IS THE RIGHT OUTCOME.** The
native count was authored as ONE chapter of fifteen atoms and
`maxNewAtomsPerChapter` is **12**, so the gentle-ramp atom-step finding went from
zero to one. The fix was not to raise the budget: the chapter was split at five,
exactly where chapters 14 and 15 split the borrowed count, giving 7 and 8 atoms
and restoring the finding to zero. This track has never carried a gentle-ramp
finding of any kind and still does not.

**TWO GATES CAUGHT REAL DEFECTS RATHER THAN NUMBERS.** The narration refusal
count rose by one: `JA-C16-yottsu`'s two-system table was five columns with the
number under no header — the exact shape the chapter policy calls unspeakable. It
is now three labelled columns (number / borrowed / native) and the narrator reads
it. The info-dump rule-statement count rose by one on a sentence in the final
payoff beginning *the rule for a learner is…*; it was rewritten rather than
absorbed, and the ceiling did not move.

**THE SCRIPT COST WAS MEASURED PER LESSON, AND THAT CAUGHT A CLASS OF LEAK.**
Because a script lesson teaches every target-script glyph in its body, a leak
teaches a glyph by accident — and the aggregate cannot see it. A per-lesson walk
found three kinds: the kanji **一 二 三 四 六 七 八 九 十** entering through
**Wiktionary link labels**, a URL not being prose but a link label being one;
**ぶ** and **ぷ** in a passing example; and the digit-one sign appearing in the
zero lesson before its own. All were removed, and closure and never-taught both
stayed at zero.

**NOT CLAIMED, and the inventory and the tests both say so rather than leaving it
to be inferred:** **まい**, **ひき**, **さつ**, **だい** and the rest of the
counter set; the nine-form of **ほん**; and any lexical atom for the **ど** of
**いちど**. `JA-A1-NUM-03`, the ordinals, changes from "blocked upstream" to
ordinary vocabulary work, because a Japanese ordinal is a counter with **だい-**
in front or **-め** behind and the counters now exist.

### Added — chapters 14 and 15: the cardinals one to ten, and the two signs they cost

`JA-A1-NUM-01` was ticked on ONE numeral, and that one was inside a phrase. The
track taught no other number and no counter. Fourteen lessons in two chapters
close it on all ten, and the coverage total does not move — which is the finding,
not an oversight: the tranche DEEPENS two existing ticks rather than adding one,
and a coverage column cannot see that. The named test pin exists for exactly that
reason.

**TWO OF THE TEN WERE ALREADY IN THE READER'S HANDS, AND THE CHAPTERS OPEN ON
THEM.** *ichi* has been said since chapter 9 inside **もういちど**, whose lesson
states in so many words that *ichi* is one. And **五** has been in the hand since
chapter 5, where the script lesson that taught its four strokes wrote on the page
that **the sign means five on its own** and then asked for nothing but its sound,
because it was there to cue the *go* inside **語**. That was an explicit deferral,
and chapter 14 is where it is paid. So the chapter opens **ichi, go** — one out of
the mouth, one out of the hand — and neither lesson spends a sign.

**THE ORDER IS BY COST, NOT BY NUMBER, IN BOTH CHAPTERS.** Chapter 14 runs
*ichi, go, ni, san, yon* and costs **zero** new signs. Chapter 15 runs *nana,
hachi, ku*, then buys **ろ** for *roku* and **ゅ** for *jū* — so the reader counts
round an audible hole at six for three lessons, and the chapter says on the page
why the hole is there. Six and ten are the only two numerals between one and ten
whose everyday reading needs a hiragana sign this book had not taught.

**THE GRAMMAR ATOM IS THE SEAM THE COUNTERS WILL RUN ALONG, AND IT IS TAUGHT WITH
ITS EDGES.** Wiktionary's readings box gives **し** as the on'yomi of four and
**よん** as the kun'yomi, and **しち**/**なな** the same way at seven. So at four
and seven the NATIVE word stands inside a borrowed count and is the one the reader
will hear. `JA-GRAMMAR-KUN-IN-THE-COUNT` is introduced at four and held at seven,
and then the chapter shows both edges rather than leaving the reader to overgeneralise:
**はち** has no second name at all, and **く** has one whose second reading the
same box calls on'yomi as well. Some numbers have two names; only some of those
get the second from the native side.

**TEN WORDS, NINETY-NINE NUMBERS.** `JA-GRAMMAR-JUU-COMPOUND` is what carries the
point past ten: a numeral before **じゅう** multiplies it, one after it is added,
and nothing is inserted between them.

**REINFORCEMENT: THE TRACK'S PERFECT RECORD IS INTACT, AND ONE DEFECT IS GONE.**

    reinforcementWindowMisses          0 ->   0
    reinforcementMissesByWindow-R1     0 ->   0
    reinforcementMissesByWindow-R2     0 ->   0
    reinforcementMissesByWindow-R3     0 ->   0
    reinforcementMissesByWindow-R4     0 ->   0
    atomsTaught                      111 -> 125
    atomsNeverRevisited                1 ->   0
    scriptClosureViolations            0 ->   0
    neverTaughtGlyphs                  0 ->   0

Fourteen more lessons make **29** (atom, window) slots newly judgeable on OLDER
atoms — one R1, two R2, ten R3, sixteen R4 — debt the length EXPOSES rather than
creates. Every one is answered, and answered on a **diagonal**: lesson *n* of the
tranche services the R4 of the atom introduced at old position 36+*n* and the
R3 of the atom at 98+*n*, so the fourteen warm-ups walk the whole of chapters 8-9
and the whole of chapter 13 in order. The tranche's own fourteen atoms create
**zero** debt. Zero regressions on previously judged slots. And
`JA-PERFORMANCE-FAMILY-NINE-01`, introduced by the track's last lesson and
therefore never revisited, is revisited now — which is why `atomsNeverRevisited`
falls to **zero**.

**THE SCRIPT COST WAS TWO SIGNS AND IT WAS MEASURED, NOT ASSUMED.** ろ and ゅ join
`data/scripts/japanese.d`, with owner declarations and evidence. **ろ's stroke
order was observed**: the cited Sirgazil animation was fetched, expanded to its 26
frames and read, and the start marker never leaves the upper-left origin, so the
run is single and `penLifts` is 0. **ゅ claims no independent handwriting
evidence**: it reuses ゆ's observed two-run movement and says so in its
`variation` field, exactly as U+3063 small tsu reuses つ's. Closure held at zero
only because the chapters do NOT print the nine kanji they cannot teach — three
Wiktionary link texts originally carried 四, 七 and 九 and were rewritten, which
the closure measurement caught and prose review would not have.

**A LEVEL GATE FLIPPED, AND IT IS WORTH THE SAME CAVEAT CHINESE AND MARWADI GOT.**
`JA-C14-yon` realizes `SPINE-COUNT-ONE-TO-FIVE`, so the track's `reach` moves from
pre-A1 to A1 and no track in the corpus is now below A1. Everything else the track
holds is still pre-A1 and `attained` has not moved. One node realized is not a
level reached.

**NOT CLAIMED, and the inventory notes now say so rather than leaving it to be
inferred.** The nine remaining kanji (五 is the only one of the ten the reader can
write as Japanese writes it). Zero, which needs **れ**. *kyū*, which needs **き**
and is given in romanization with the reason. The COUNTERS — `JA-A1-NUM-02` —
which is the next tranche and is what still stops the reader counting *things*
rather than counting aloud; the reader already owns one counter without its being
named as one, the **-ど** of chapter 9's **いちど**, whose lesson says *do* counts
an occurrence. And ordinals, `JA-A1-NUM-03`, which are blocked behind the
counters and not behind the cardinals any more.

### Changed — learner guide follows the authored runway (#12568)

- Replaced the obsolete single-chapter/eight-lesson description with the current
  twelve-chapter, 100-session pre-A1 sequence.
- Reported vocabulary and script coverage from the canonical lesson ledger and
  script-closure measurement: 28 word lessons, seven phrase lessons, and 47 of
  47 shown glyphs taught with no closure violations.
- Made the five-minute script-before-decoding and writing-stage policies
  explicit, and connected the opening book honestly to the full pre-A1-to-C2
  non-compensatory assessment contract.

### Added — one sound-first farewell (#12475)

- Added six <=5-minute sessions for **さようなら**: sound and social job first,
  then only the three new signs **よ**, **な**, and **ら**, a sign-by-sign
  assembly, and a four-skill payoff.
- Reused known **さ** and **う** instead of reteaching them, while preserving a
  trace-copy-recall writing ramp for the genuinely new shapes.
- Realized the shared `FAREWELL` function without pretending this one expression
  covers every relationship or departure context.
- Grounded the A1 headword and contextual warning in Japan Foundation Marugoto
  and Irodori materials.

### Changed — script closure before decoding (#12471)

- Reordered the 29 existing lessons so every hiragana writing step precedes the
  word that asks the learner to decode it; the old content-first order is gone.
- Split the starter into seven small chapters, keeping hiragana, kanji, and
  katakana arrivals separate and moving the four-skill doorway exchange to the
  end of the runway.
- Added nine <=5-minute writing lessons: `日`, `本`, three components and the
  assembled `語`, followed by `コ`, the long-vowel mark `ー`, and `ヒ`.
- Rewrote advanced etymology and keigo examples in romanization so fifteen rare
  or unrelated signs no longer become accidental pre-A1 decoding tests.
- Replaced the stale eight-session map with the complete 38-session authored
  order and its machine-checked review rule.

### Added — cumulative pre-A1 writing evidence (#12365)

- Turned four already gentle Chapter 2 lessons into an explicit cumulative
  writing ladder: trace one visible sign, copy one visible sign with cues, hide
  and recall one two-sign word, then transcribe one heard or romanized mora.
- Kept every action inside its original one-sign load and below five minutes;
  the evidence metadata now follows the learner action instead of merely tagging
  lessons that happen to contain handwriting.

### Added — pre-A1 four-skill task shapes (#12363)

- Made the project-defined rung below JLPT/JF A1 executable as four independently
  scored reading, listening, writing, and speaking sections.
- Kept writing productive: delayed kana recall, dictation/transcription, and one
  bounded independent response earn points; tracing and visible copying do not.
- Recorded exact project-owned timing and length bounds without inventing an
  external “JLPT N6” or implying that chapter coverage proves readiness.

### Added — pre-A1-to-C2 four-skill assessment contract (#12361)

- Replaced the old unofficial one-level-per-JLPT mapping with the official CEFR
  reference score bands introduced on JLPT score reports in December 2025.
- Added a seven-rung assessment target that preserves the official JLPT
  language-knowledge, reading, and listening pass conditions where they apply,
  then adds independently scored JF Standard/CEFR-aligned writing and speaking.
- Kept pre-A1 and C2 explicitly project-defined: JLPT's official CEFR reference
  range begins at A1 and ends at C1, and JLPT itself tests no production or
  interaction at any level.

### Added — Chapter 2, eight hiragana signs, one per lesson (HL-C211)

Ten lessons. **Eight teach one sign each; two introduce nothing at all** and
instead assemble a word out of signs the reader can already write:

    i -> ha -> [hai] -> e        both yes-or-no answers become readable
    ko -> n -> ni -> chi -> wa -> [konnichiwa]

`scriptLessons` 0 -> 10, `taughtGlyphs` 0 -> 8, `neverTaughtGlyphs` **43 -> 35**.
Corpus-wide `tracksTeachingNoScript` falls to **6**.

**The sign for *wa* is taught deliberately late**, after the greeting is already
known — so that the shape arrives with its warning attached. The daytime greeting
*sounds* like it ends in *wa* and is written with the sign read *ha*, because that
sign is doing a second job as the topic marker. Teaching the *wa* sign first would
have quietly created the commonest beginner spelling error in the language.

**Three signs deliver a payoff the same day they are learned.** Two signs make the
word for *yes* readable; a third adds the word for *no*. The assembly lessons exist
to mark that moment — the point where a reader stops recalling a shape and starts
sounding one out.

**The mora rule gets its clearest evidence here.** The sign with no vowel takes a
full beat, exactly as long as the four around it, which is why the greeting is five
beats rather than four. The romanization cannot show that; the signs can.

This is the first of five tranches for this track. 35 glyphs remain — 16 hiragana,
10 katakana, 9 kanji — and the katakana and kanji sets are separate writing systems
with their own ramps.


### Added — Chapter 1, "Three Writing Systems in One Doorway" (HL-C40)

- Registered `japanese` in `core/languages.json` (Japonic / `japanese` script,
  bridging Chinese for the Sino-Japanese layer and Portuguese and German for the
  loanword layer) and declared the track's script in `track.json`, so no edit to
  the built-in `LANGUAGE_SCRIPT` map was needed.
- Added eight schema-v2 lessons, sequences 10–80, each with typed blocks, a
  first-line `hl-knowledge` directive per block, one compiled `hl-activity`, and
  a declared duration under 300 effective seconds:
  - `JA-C01-hai` — **はい**, hiragana and the mora as the unit of timing.
  - `JA-C01-iie` — **いいえ**, mora length as a meaning contrast (いえ vs いいえ).
  - `JA-C01-konnichiwa` — **こんにちは**, the moraic **ん**, and the topic
    particle **は** read *wa*, with the 1946 spelling reform as the reason.
  - `JA-C01-nihongo` — **日本語**, kanji, and the multiple-readings problem.
  - `JA-C01-koohii` — **コーヒー**, katakana, the chōonpu, and the Arabic *qahwa*
    borrowing shared with English *coffee*.
  - `JA-C01-arigatou` — **ありがとう**, the dakuten, and 有り難し "hard to exist".
  - `JA-C01-gozaimasu` — **ありがとうございます**, politeness as verb morphology.
  - `JA-C01-practice` — the six-line doorway exchange payoff.
- Added `curriculum.json` with three path segments, a ledger entry for all eleven
  spine nodes, and seven Japanese-specific extension nodes (five `script`, one
  `register`, one `consolidation`).
- Added `chapters.json` with the chapter capability and a payoff assessing ten of
  the chapter's eighteen introduced atoms.
- Added `roadmap.md`, `session-map.md` (review ledger through S23),
  `pronunciation-reference.md`, and `README.md`.
- Added `data/scripts/japanese.json`: one inventory covering hiragana, katakana,
  the length bar, and the seven kanji the lessons use, with `role` distinguishing
  the systems and the dakuten/handakuten as marks.
- Vendored `_fonts/NotoSansJP-Subset.ttf` (SIL OFL 1.1) with `_fonts/subset-jp.sh`
  to regenerate it, following the existing `subset-cjk.sh` precedent.
- Added the generated LaTeX Chapter 1 and the book scaffolding, with a
  `japanese-main` script set mapping Katakana, Hiragana, and Han to one `\ja`
  command.

### Notes on method

- Seven of the eight lessons carry a `script` block and are therefore derived as
  `sight` under HL08. That is deliberate: a sign's shape cannot be read aloud, and
  marking these lessons drivable would promise a learner something no narration
  can deliver. The chapter's drivable prefix is 0 and its practice lesson is the
  only `voice` lesson.
- No cognate with English is claimed anywhere except **コーヒー**, where the
  shared Arabic source is real. `JA-C01-hai` states plainly that its own
  etymology is unsettled and that no English cousin exists.
