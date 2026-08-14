import{t as e}from"./rolldown-runtime-DK3Fl9T5.js";var t=e({default:()=>n}),n=`---
schema_version: 2
id: HI-W05-conjuncts
spine_node: SPINE-EXCHANGE-NAMES
sequence: 240
delivery: script
chapter: 2
type: writing
headword: "स्त"
gloss: a vowel-less consonant usually fuses with the next consonant into a conjunct
romanization: "sta"
prerequisites: [HI-W05-virama-namaste, HI-W04-ra-sa-mera-naam]
sounds: [devanagari-virama, devanagari-conjunct]
roots: [devanagari-conjunct-formation]
duration:
  max_seconds: 189
requires:
  knowledge: [HI-CONCEPT-W05-VIRAMA-NAMASTE-01, HI-CONCEPT-W04-RA-SA-MERA-NAAM-01, HI-CONCEPT-W04-RA-SA-MERA-NAAM-02]
introduces:
  knowledge: [HI-CONCEPT-W05-CONJUNCTS-01, HI-CONCEPT-W05-CONJUNCTS-02]
practises:
  knowledge: [HI-CONCEPT-W05-CONJUNCTS-01, HI-CONCEPT-W05-CONJUNCTS-02]
skills: [writing, reading]
modes: [interpretive, presentational]
strands: [meaning-input, meaning-output, language-focus]
register: neutral
variety: standard-hindi
reviews_of: [HI-W05-virama-namaste, HI-W04-ra-sa-mera-naam]
---

# स्त — two consonants squeezed together

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[] -->

[PAUSE 2s] A visible virama can stop a vowel. In ordinary writing, the stopped
consonant often goes one step further and fuses with the consonant after it.

## You'll want to know — The usual squeeze
<!-- hl-knowledge: introduces=[HI-CONCEPT-W05-CONJUNCTS-01]; assesses=[] -->

The fused shape is a **conjunct**:

> स् + त → **स्त** (*sta*)

Look at स. It loses its right-hand spine, shrinks, and leans into त. For
**spine-bearing** letters this is the usual move: **the first consonant gives up
its spine and hands the syllable to the second**.

## You'll want to know — Spineless and irregular shapes
<!-- hl-knowledge: introduces=[HI-CONCEPT-W05-CONJUNCTS-02]; assesses=[] -->

र has no spine to give up, so it behaves differently: as a hook above a later
letter (र् + क → **र्क**) or as a small stroke under an earlier one
(क + ्र → **क्र**). Other spineless letters, **द** among them, stack or take
special shapes. A few common conjuncts — **क्ष**, **त्र**, **ज्ञ** — are
irregular and must be learned whole.

This is why Devanagari seems to have so many shapes: it is not only 40-odd
letters, but also the ligatures they form when they collide. Do not memorise
every conjunct as a separate character. Learn to recognise the *squeeze*.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[HI-CONCEPT-W05-CONJUNCTS-01, HI-CONCEPT-W05-CONJUNCTS-02] -->

[PAUSE 1s]
- [YOU WRITE: स् — स with the vowel stopped]
- [YOU WRITE: स्त — watch स **lose its spine** and lean into त]
- [YOU TRACE: र्क and क्र — two special jobs for spineless र]

## Wrap-up Recall
<!-- hl-knowledge: introduces=[]; assesses=[HI-CONCEPT-W05-CONJUNCTS-01, HI-CONCEPT-W05-CONJUNCTS-02] -->

[PAUSE 3s] What usually happens instead of leaving a virama visible? (The first
consonant **fuses** with the next into a **conjunct**.) In **स्त**, what does स
give up? (**Its spine**.) Why does र need different forms? (**It has no spine to
give up.**) Next: use **स्त** inside the first greeting you learned.
`,r=e({default:()=>i}),i=`---
schema_version: 2
id: HI-W05-virama-namaste
spine_node: SPINE-EXCHANGE-NAMES
sequence: 230
delivery: script
chapter: 2
type: writing
headword: "्"
gloss: "the virama or halant — the mark that stops a consonant's inherent vowel"
romanization: "(vowel killer)"
prerequisites: [HI-W04-write-mera-naam, HI-W03-preposed-i, HI-W02-ka-ta-mouth-order]
sounds: [devanagari-virama, devanagari-conjunct]
roots: [sanskrit-virama, sanskrit-namas]
etymology_hook: "virāma विराम means 'a STOPPING, a cessation' (vi- + ram- 'to rest') — the same word that grades Hindi's punctuation, pūrṇ virām 'complete stop' = full stop, alp virām 'slight stop' = comma; and namaste = namas 'a bow' + te 'to you', so the first word of this book is literally 'a bowing to you', from nam- 'to bend'"
duration:
  max_seconds: 181
requires:
  knowledge: [HI-CONCEPT-W04-WRITE-MERA-NAAM-01, HI-CONCEPT-W03-PREPOSED-I-01, HI-CONCEPT-W02-KA-TA-MOUTH-ORDER-01, HI-CONCEPT-W02-KA-TA-MOUTH-ORDER-02, HI-CONCEPT-W02-KA-TA-MOUTH-ORDER-03]
introduces:
  knowledge: [HI-CONCEPT-W05-VIRAMA-NAMASTE-01]
practises:
  knowledge: [HI-CONCEPT-W05-VIRAMA-NAMASTE-01]
skills: [writing, reading]
modes: [interpretive, presentational]
strands: [meaning-input, meaning-output, language-focus]
register: neutral
variety: standard-hindi
reviews_of: [HI-W04-write-mera-naam, HI-W02-abugida-ka-ta, HI-C01-namaste]
---

# The vowel killer — virama and halant

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[] -->

[PAUSE 2s] Every consonant carries a free "a." Mātrās replace it. But what if
you need **no vowel at all** — just a bare consonant?

## You'll want to know — The virama ्
<!-- hl-knowledge: introduces=[HI-CONCEPT-W05-VIRAMA-NAMASTE-01]; assesses=[] -->

**्** is the **virama** — a small stroke **below** the consonant that removes the
inherent vowel:

> क = *ka* → क् = ***k***

The name says exactly what it does. **विराम** (*virāma*) is "a **stopping**, a
cessation" — *vi-* + *ram-*, "to rest, to come to a halt." The vowel is told to
stop.

And the word is still working elsewhere in Hindi punctuation: a full stop is
**पूर्ण विराम** ("*complete* stop"), a comma is **अल्प विराम** ("*slight* stop"),
a semicolon **अर्ध विराम** ("*half* stop"). One word, graded by how long you rest.

A naming note: *virāma* is the Sanskrit term, and the one Unicode uses. In
everyday Hindi the mark is usually called the **हलंत** (*halant*). Same mark,
two names — you'll meet both.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[HI-CONCEPT-W05-VIRAMA-NAMASTE-01] -->

[PAUSE 1s]
- [YOU WRITE: क् — "क with the stroke below" — the vowel is dead]
- [YOU SAY: "virāma — stopping; halant — the everyday Hindi name"]
- [YOU CONTRAST: क *ka* → क् *k*]

## Wrap-up Recall
<!-- hl-knowledge: introduces=[]; assesses=[HI-CONCEPT-W05-VIRAMA-NAMASTE-01] -->

[PAUSE 3s] What does the virama **्** do, and what does its name mean? (Kills the
**inherent vowel**; *virāma* = "a **stopping**" — and a full stop in Hindi is
**पूर्ण विराम**, a "complete stop".) What is the mark usually called in everyday
Hindi? (The **halant**.) Next: watch a stopped consonant squeeze into the one
that follows.
`,a=e({default:()=>o}),o=`---
schema_version: 2
id: HI-W05-write-namaste
spine_node: SPINE-EXCHANGE-NAMES
sequence: 250
delivery: script
chapter: 2
type: writing
headword: "नमस्ते"
gloss: assemble namaste, then uncover the bowing gesture inside the greeting
romanization: "namaste"
prerequisites: [HI-W05-conjuncts, HI-W03-matras-naam, HI-W01-na-ma]
sounds: [devanagari-conjunct, devanagari-matra, devanagari-top-bar]
roots: [sanskrit-namas, sanskrit-nam-bend]
duration:
  max_seconds: 180
requires:
  knowledge: [HI-CONCEPT-W05-CONJUNCTS-01, HI-CONCEPT-W05-CONJUNCTS-02, HI-CONCEPT-W03-MATRAS-NAAM-01, HI-CONCEPT-W03-MATRAS-NAAM-02, HI-CONCEPT-W03-MATRAS-NAAM-03, HI-CONCEPT-W01-NA-MA-01, HI-CONCEPT-W01-NA-MA-02, HI-CONCEPT-W01-NA-MA-03]
introduces:
  knowledge: [HI-CONCEPT-W05-WRITE-NAMASTE-01, HI-CONCEPT-W05-WRITE-NAMASTE-02]
practises:
  knowledge: [HI-CONCEPT-W05-WRITE-NAMASTE-01, HI-CONCEPT-W05-WRITE-NAMASTE-02]
skills: [writing, reading]
modes: [interpretive, presentational]
strands: [meaning-input, meaning-output, language-focus]
register: neutral
variety: standard-hindi
reviews_of: [HI-W05-conjuncts, HI-C01-namaste, HI-C01-namaskar]
---

# नमस्ते — assemble the greeting at last

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses=[] -->

[PAUSE 2s] You have learned every piece separately. Now the writing track pays
off in the first word of
this book.

## You'll want to know — Assemble नमस्ते
<!-- hl-knowledge: introduces=[HI-CONCEPT-W05-WRITE-NAMASTE-01]; assesses=[] -->

| piece | | |
|---|---|---|
| **न** | *na* | Lesson 1 |
| **म** | *ma* | Lesson 1 |
| **स्** | *s* — स with its vowel killed | last lesson |
| **ते** | *te* — त + the े mātrā | Lessons 2 and 3 |

> **न · म · स् · ते → नमस्ते**

Write it, then draw **one bar** across the whole word.

## You'll want to know — What you have been writing all along
<!-- hl-knowledge: introduces=[HI-CONCEPT-W05-WRITE-NAMASTE-02]; assesses=[] -->

**नमस्ते** joins **namas** + **te**:

- **namas** — "a bow, an obeisance," from *nam-*, "**to bend**"
- **te** — "to you"

So *namaste* is literally **"a bowing to you."** It describes the physical
gesture, which is why the word and folded hands go together. The greeting is
the bow said aloud.

*Namaskār* is the same *namas* with *kāra*, "making": "**making** a bow." One
root, two greetings.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[HI-CONCEPT-W05-WRITE-NAMASTE-01, HI-CONCEPT-W05-WRITE-NAMASTE-02] -->

[PAUSE 1s]
- [YOU WRITE: स्त — the conjunct at the center]
- [YOU WRITE: ते — त plus the stroke above]
- [YOU WRITE: **नमस्ते** — one bar across all of it]
- [YOU SAY: "*namas* = a **bow**, *te* = **to you**"]

## Wrap-up Recall
<!-- hl-knowledge: introduces=[]; assesses=[HI-CONCEPT-W05-WRITE-NAMASTE-01, HI-CONCEPT-W05-WRITE-NAMASTE-02] -->

[PAUSE 3s] Write **नमस्ते**. How many bars? (**One** — it is one word.) What
does it literally mean? (**A bowing to you**: *namas*, "bow," + *te*, "to
you.") And *namaskār*? (The same *namas* + *kāra*, "**making** a bow.")
`;export{r as n,t as r,a as t};