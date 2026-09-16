## HL-C376 — the 21% was a keyword artefact; the real signal is a note that asserts absence and is wrong

HL-C375 reported that 451 of 2,189 uncovered points (21%) had a candidate atom
already in the corpus, and said explicitly that a candidate was a hypothesis to
be read rather than a match to trust. It was read. **Most of it is noise, and
the number should not be quoted again.**

Twenty-eight candidates were reviewed by hand:

- **Hindi A1 — 19 candidates, 0 genuine.** Every one is a keyword collision.
  `HI-GRAMMAR-C41-ADJ-SYSTEM` matched "the education **system**";
  `HI-CONCEPT-C07-NAHIN-*` matched "negatively"; `HI-LEX-C74-SIGNS-*` matched
  "on **signs** and forms" but teaches sign *words* (open, closed, entrance)
  while the point wants abbreviations and symbols (the rupee sign, `Dr.`, `No.`).
  Hindi's notes are accurate and were already saying so.
- **Spanish A1 — 9 candidates, 2 genuine.** `ES-GRAMMAR-DOUBLE-OBJECT-ORDER`
  matched "give an **order**"; `ES-HISTORY-RAE-INVERTED-MARKS` matched
  "examinations and **marks**"; `ES-GRAMMAR-COMMAND-*` matched "the **comma**".

So the keyword scan is the wrong instrument. The right one is narrower and
exact: **a point whose `note` asserts the exponent is absent, where the corpus
introduces it anyway.** Only 25 points corpus-wide carry such a note, 23 of them
Spanish, and of those 23 **nine are contradicted by an atom that is genuinely
introduced**:

| point | the note said | the corpus has |
|---|---|---|
| A1-F3-03 | "never introduces the verb" [preferir] | `ES-LEX-PREFERIR` (ES-C396) |
| A1-NE06-05 | "never introduces examen" | `ES-LEX-EXAMEN` (ES-C410) |
| A1-NE18-05 | "foto, which the corpus never introduces" | `ES-LEX-FOTO` (ES-C405) |
| A1-NE08-02 | "None is introduced" | `ES-LEX-PELICULA`, `ES-LEX-CONCIERTO` |
| A1-NE18-01 | "introduces none of them" | four of the eight exponents |
| A1-NE18-06 | "None is introduced" | `ES-LEX-PELICULA` |
| A1-NE16-02 | "introduces none of them" | `ES-LEX-ORDENADOR`, `ES-LEX-INTERNET` |
| A1-NE09-06 | "introduces none of them" | `ES-LEX-INTERNET`, `ES-LEX-CORREO-ELECTRONICO` |
| A1-NE12-02 | "introduces none of them" | `ES-LEX-CAMISA` |

Three of the nine close the point outright, because the source's whole named A1
exponent is present: A1-F3-03, A1-NE06-05, and A1-NE18-05 (taken in the previous
tranche). The other six are corrected in place and stay uncovered — pelicula
without cine or teatro does not buy "cinema and theatre", and Internet without
arroba or punto does not buy dictating an e-mail address.

**Revised estimate.** Not 21%. Corpus-wide there are 25 absence-claiming notes;
if Spanish's 9-of-23 error rate holds elsewhere the total free-wiring yield is
on the order of **ten points, not four hundred**. That is still worth taking —
it cost no authoring at all — but it does not change the shape of the work.
Closing A1 across the twelve Indian tracks needs roughly 1,090 points of real
teaching, and no audit makes that smaller.

**Why the notes were wrong matters more than that they were wrong.** Each was
true when written. Chapters 393-413 then taught internet, pelicula, concierto,
musica, foto, preferir and examen without anybody revisiting the inventory,
because nothing links a new lesson back to a point whose `probe` is `null`. The
audit trail decays silently and in the flattering direction for the author of
the lesson and the unflattering direction for the number. A gate that flagged
"note asserts X is absent, but an atom named for X is introduced" would have
caught all nine the day they went stale, and is the obvious next piece of
tooling.
