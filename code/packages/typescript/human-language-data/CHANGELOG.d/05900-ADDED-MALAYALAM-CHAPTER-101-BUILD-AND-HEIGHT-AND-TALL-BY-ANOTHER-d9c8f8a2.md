### Added — Malayalam chapter 101, build and height, and *tall* by another route

- `ML-A1-LEX-06` closes. Malayalam A1 coverage 207/243 -> **208/243 (86%)**, 35
  points unmapped. **The percentage moves, 85 → 86** (85.6 rounds up) —
  recomputed, not carried.

#### Tall is reached by a different route, and that is the chapter's point

Malayalam's ordinary word for *tall* is **ഉയരമുള്ള** — which needs the relative
participle **ഉള്ള**, and `ഉള്ള` is taught **nowhere**: zero occurrences in the
whole corpus.

Rather than smuggle it in, the chapter teaches **ഉയരം** as a **noun** and
reaches *tall* through a frame the learner already owns:

> **എനിക്ക് ഉയരമുണ്ട്.** — *enikku uyaramuṇṭŭ* — to-me height there-is.

`ML-C32-undu` (650) states **ഉണ്ട്** as *"there is; [someone] has"* outright, and
`ML-C06-dative-subject` (330) owns **എനിക്ക്**. Every piece is taught. Same
refusal as **ഒഴിവുസമയം** in ch98 and **ധരിക്കുക** in ch100.

#### No new grammar; the teaching is that adjacent words are not synonyms

| adjective | before the noun | ends |
|---|---|---|
| **കുറിയ** | **കുറിയ കുട്ടി** | **-ിയ** |
| **മെലിഞ്ഞ** | **മെലിഞ്ഞ കുട്ടി** | **-ഞ്ഞ** |
| **തടിച്ച** | **തടിച്ച കുട്ടി** | **-ച്ച** |

Three shapes, all closing on **-a**, all sitting in `ML-C42`'s frame unchanged.
**ചെറിയ** and **കുറിയ** differ only in their **first syllable** — **ചെ**
against **കു**, which is *two* characters, not one — and do **not** mean the
same thing: *small* against *short*. *Tall* is the mirror of neither.

#### Review corrected four claims, and three were in this chapter's generalising prose

- **The anusvara sentence said "always".** `ML-C101-uyaram` had *"the **ം** does
  what it always does when something follows it"*. `ML-C96-eluppam` says the
  opposite in as many words — *"what a word ending in **ം** does **depends on
  what attaches to it**"* — and `ML-C97-alla-sundaram` warns *"do not carry any
  of it across to the **ം**"*. **It is refuted by this chapter's own recall**,
  whose romanization field is `uyaravuṁ`: **ഉയരം** + **ഉം** → **ഉയരവും**, with a
  **വ**. Now scoped to **ആണ്** and **ഉണ്ട്**, with the counter-example named.
- ***"ചെറിയ is for objects, കുറിയ for people"* was false of both words, and
  contradicted a merged lesson.** `ML-C42-small` (1020) tells the learner to put
  **ചെറിയ** *"in front of a word you already know"* — and by 1020 they own
  **കുട്ടി** (810). The book had already invited **ചെറിയ കുട്ടി**, which this
  chapter was about to declare impossible.
- ***"The others end -ിയ"* was false for two of the five.** **നല്ല** ends
  **-ല്ല** and **പഴയ** ends in a plain **-യ** — neither has a **ി** at all.
  This one is doubly instructive: the draft fix that *added* **നല്ല** to the
  list is what made the next sentence false. A correction pass is as dangerous
  as a draft.
- ***"a shape you have not met"* / *"a third shape"*.** **-ഞ്ഞ** and **-ച്ച**
  have been in the book since **പച്ച** and **മഞ്ഞ** at chapter 22, sequence
  **520**. The lesson now cites them instead of claiming novelty.

Also softened: *"I am tall"* overstated a bare dative-existential, which reads
nearer *I have height*; **പൊക്കം** is the commoner noun for a person's stature.
The chapter now claims what it can **build**, not that it has built the idiom.
And `ML-C101-uyaram` declared `ML-LEX-AANU-01` in `requires`, `practises` and a
block's `assesses` while containing **zero** occurrences of **ആണ്** — removed.

#### Two inventions caught before the first commit — by a sweep, not by reading

- **ആൾ** ("person") was written into an example sentence. It is taught
  **nowhere**; the only occurrence in the corpus was the file I had just
  written. Replaced with **കുട്ടി**, owned since sequence 810.
- **സഹോദരൻ** was about to be quoted in Malayalam **script** from
  `ML-C32-undu` — which gives that example only in **romanization**. Quoting it
  would have introduced an unowned word through the back door. The borrowed
  example was dropped instead of imported.

**Both were found by a whole-token owner sweep, not by re-reading the prose**,
which is the point: every new file is now swept for Malayalam tokens with no
headword owner, leaving only joined forms built from owned pieces
(**ഉയരമുണ്ട്**, **കുട്ടിയാണ്**) and metalinguistic ending fragments
(**ച്ച**, **ഞ്ഞ**, **-ിയ**). Reading had already passed over **ആൾ** twice.

Also caught in draft: the adjective list claimed *"every adjective the book has
given you"* while omitting **നല്ല**. Named in full now.
