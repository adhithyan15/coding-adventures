## HL-C384 — the letter track starts at chapter six, so eight glyphs can never be taught before their first word, and exactly two still can

`HL-C383` left one question open: the ten glyphs credited as taught but never
drawn have their first uses in chapters 1–13, "where the track has least room",
and it guessed that the squeeze was why they were never written. **That guess
was right and the cause is sharper than "least room". It is structural.**

### How the Hindi script track is actually built

| chapters | run | shape |
|---|---|---|
| 1–2 | `HI-W*`, sequences 60–266 | **whole words**: shirorekhā, नमस्ते, मेरा नाम, the danda, dictation |
| 6–16 | `HI-S*` | **two letter lessons per chapter**, at sequences `N05` and `N06` |
| 59 | `HI-W12-*`, sequences 2100–2200 | **eleven lessons catching up**: भ, य, ब, ड and four mātrās |

**There are three runs, not two, and the third one already does the thing this
entry recommends.** Chapter 59 goes back and draws characters the reader had
been using for fifty chapters — **ड** among them, which is why it is not on the
undrawn list below. That is the "teach it late and say so" pattern, already in
the corpus, at scale.

So the recommendation is not a new departure. It is **finishing a job the
corpus already started once and stopped part-way through**.

The opening teaches writing by **copying words the reader can already say**. It
draws the shirorekhā, the mātrās, the virāma and two conjuncts, and it gets a
learner writing *my name is ___* by the end of chapter 2. It never walks the
alphabet, because walking the alphabet is what chapters 6–16 are for.

**So every letter inside नमस्ते, ख़ुशी, ठीक, स्वागत, फिर and कृपया arrived on
the page before any lesson existed that could draw it.** That is not an
oversight; it is what teaching through whole words costs, and the cost was
invisible because `measureScriptClosure` credited those letters to whichever
lesson happened to print them.

### Eight cannot be fixed without rebuilding the opening

| glyph | first use | chapter |
|---|---|---|
| ः visarga | `HI-C01-namaste` | 1, seq 10 |
| ृ vocalic-r sign | `HI-C01-namaskar` | 1, seq 20 |
| ट retroflex ṭa | `HI-W02-ka-ta-mouth-order` | 1, seq 90 |
| ़ **nuqta** | `HI-C02-khushi` | 2, seq 180 |
| ठ retroflex ṭha | `HI-C03-thik` | 3, seq 310 |
| ग ga | `HI-C03-aapka-swagat-hai` | 3, seq 320 |
| फ pha | `HI-C04-phir` | 4, seq 340 |
| इ i | `HI-C05-karna` | 5, seq 420 |

**The letter track does not start until chapter 6.** There is no slot before any
of these.

Two ways forward, and the first is the one to take:

1. **Teach them late and say so.** Chapter 103 set the precedent with **ज** —
   first use chapter 2, taught chapter 30 — and the lesson says plainly that the
   reader has been seeing the letter without being shown it. Late is enormously
   better than never, and the admission is good teaching rather than an apology.
2. **Restructure chapters 1–5.** A much larger job, and it would trade away the
   thing the opening is good at: a learner writing a real sentence in two
   chapters.

### Exactly two can still be taught on time — and this is the last chance

| glyph | first use | free slot | stroke data |
|---|---|---|---|
| घ gha | `HI-C16-mahine`, seq 630 | **ch14, seq 612** | sourced, 2 pen lifts |
| ढ retroflex ḍha | `HI-C16-mahine`, seq 630 | **ch15, seq 622** | sourced, **1 pen lift** |

Chapters 14, 15 and 16 each carry only **one** script lesson where the pattern
allows two, so the second slot is free in all three. Two of those slots fall
before sequence 630.

**After these, no remaining glyph can be taught before its first word.** Every
other undrawn character is in the table above.

Watch the headroom: chapters 14, 15 and 16 are each **3 atoms with a payoff
assessing 2** — a ratio of 0.67 — and one letter lesson takes them to exactly
**0.50**, the floor. It fits, with nothing to spare. Do not put two in one.

### One thing these two will not fix

`HI-C16-mahine` is in closure debt for **four** glyphs: घ, ढ, भ and ौ. Teaching
घ and ढ leaves **ौ**, which the au-pair change places at chapter 33 — correct
for the three lessons it was written to serve (`HI-C34-padhna`,
`HI-C40-where`, `HI-C76-tenth`) and still too late for chapter 16.

Moving the au-mātrā into the third free slot would not help either: chapter 16's
own slot is sequence 632, which is **after** 630. Clearing `HI-C16-mahine`
completely would need the mātrā in chapter 14 or 15 as well — and those are the
two slots घ and ढ want. **One of the three lessons cannot be served**, and
chapter 16 is the one to leave, because the mātrā's placement already clears
three violations where घ and ढ clear one between them.


### Progress

**Three of the eight** are now drawn late and say so: **ग** (chapter 25), **फ**
(26) and the **nuqta**, in two lessons at chapters 60–61.

The nuqta could not go earlier than 60 for a reason this entry did not
anticipate. A lesson teaching a mark has to show it on carriers the reader can
draw, and retroflex-**ड** is drawn only in the chapter-59 run — so the mark's
placement was bounded by its *carriers*, not by its own first use at chapter 2.
Any future mark lesson has the same constraint, and it is worth checking before
picking a chapter.

**Five remain:** the visarga, the vocalic-ṛ sign, **ट**, **ठ** and **इ**, all
still bounded by the structural fact this entry records.


### Correction: the vocalic-ṛ sign was never one of the eight

This entry listed **ृ** among the glyphs that can never arrive on time, citing
`HI-C01-namaskar` at chapter 1 as its first use. **That is a Sanskrit root
quoted in an etymology note** — the entry counted √कृ, संस्कृत and कीदृश as
first uses, and none of them is a Hindi word the reader is asked to read.

The sign's only headword in the corpus is **कृपया**, at chapter 8. Its carrier
**क** is drawn at chapter 7, so the sign goes at chapter 7 too — **before the
word that needs it**. `HI-S144-vowel-sign-vocalic-r` does exactly that, and it
is the first glyph in this campaign to arrive on time.

**Seven, not eight**, and the same measurement error as `HI-C16-mahine`: a first
use counted from raw text rather than from what a lesson teaches. Check the
*kind* of appearance before recording a first use.
