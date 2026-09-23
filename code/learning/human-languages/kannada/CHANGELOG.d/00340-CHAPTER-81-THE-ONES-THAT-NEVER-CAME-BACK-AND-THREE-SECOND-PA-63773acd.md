## Chapter 81 — the ones that never came back, and three second passes in place

Six lessons. None of them teaches anything.

| metric | before → after |
|---|---|
| pre-A1 atoms revisited fewer than twice | 41 → **0** |
| ladder blockers | 3 → **2** (`atom-budget 1`, `vocabulary 121`) |
| lessons | 348 → 354 |
| atoms introduced | **0** |

`atom-budget 1` is **pre-existing**, measured at the base commit rather than
assumed.

### Forty-one atoms, forty-four retrievals

The level gate asks for two revisits per atom, and Kannada had **only three
atoms with none at all** — the other thirty-eight had one and needed one more.
Sanskrit's twenty-eight atoms cost forty-five retrievals; Kannada's forty-one
cost forty-four. Counting atoms would have made this track look like the most
expensive of the three; counting retrievals made it the cheapest.

### Twenty-three of the forty-one were script

That is what shaped the tranche. Kannada teaches its characters **one per
chapter**, scattered from chapter 2 to chapter 42, and a character taught that
way is met once and never gathered. Every one of those twenty-three lives on the
same path segment, so a single new chapter can reach all of them.

**Chapter 81** does, in three lessons that sort them the way chapters 77-80
already sort characters — by what kind of thing each one is:

- **signs that hang** — six vowel signs and two marks, none able to stand alone
- **vowels that stand** — the five full-size letters, each against the sign that
  writes the same sound when a consonant is there to carry it
- **ten consonants** — sorted by where in the mouth they are made, which is the
  idea the whole Kannada grid is built on

The middle lesson is the one that earns its place: **ಇ** against **ಿ**, **ಓ**
against **ೋ** — same vowel, two shapes, and the shape tells you where in the word
you are. Nothing before this had put the pair side by side.

**ಞ**, the palatal nasal, had not been printed since the chapter that taught it.

### Three more in place, where the material lives

- **chapter 2** — the whole name exchange: two words for *you*, a question with
  no verb in it, and the two ways to agree. The lesson's claim is that
  *nimma hesaru ēnu?* has **no verb at all**, and that *nimma* rather than
  *ninna* was a register decision made before the sentence started.
- **chapter 4** — leaving. **ಹೋಗಿ ಬರುತ್ತೇನೆ** is *I will go and come back*, said
  by the person leaving whether or not they intend to return; announcing a
  departure as final is the thing being avoided. And there is no **ನಾನು** in it,
  because the verb ending already carries the person.
- **chapter 20** — the dative. **ನನಗೆ ಕನ್ನಡ ಗೊತ್ತು** is *to-me Kannada known*:
  no subject, no verb *to know*. Age works the same way — *your age how-much?* —
  so one suffix carries knowing and years alike, where English needs two
  different verbs and Kannada needs neither.

### What it cost the graph

Curriculum digest `e4e3c864`/7533 → `8fe09909`/7539 on a 50-line structural
diff: the six lessons joining their lists, and one new segment plus extension
for chapter 81.
