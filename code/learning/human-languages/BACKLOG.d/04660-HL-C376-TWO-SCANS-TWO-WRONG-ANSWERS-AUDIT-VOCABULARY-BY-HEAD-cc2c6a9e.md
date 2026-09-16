## HL-C376 — two scans, two wrong answers: audit vocabulary by headword, never by atom id

HL-C375 claimed 451 of 2,189 uncovered points (21%) had a candidate atom already
in the corpus. **That number is withdrawn.** So is the number that replaced it.
Both scans were wrong, in opposite directions, and the second was worse because
it looked careful.

### Scan one: keyword match on atom ids — overcounted wildly

Twenty-eight candidates were read by hand. Hindi: **19 candidates, 0 genuine.**
`HI-GRAMMAR-C41-ADJ-SYSTEM` matched "the education **system**";
`HI-LEX-C74-SIGNS-*` matched "on **signs** and forms" but teaches sign *words*
(open, closed, entrance) where the point wants ₹ and `Dr.`. Spanish: 9
candidates, 2 genuine — `ES-GRAMMAR-DOUBLE-OBJECT-ORDER` matched "give an
**order**", `ES-GRAMMAR-COMMAND-*` matched "the **comma**".

### Scan two: `ES-LEX-<WORD>` id lookup — undercounted, and was quoted anyway

The replacement looked for an atom literally named `ES-LEX-CINE`. The corpus
does not name atoms that way. Roughly half its vocabulary uses **chain ids**:
`ES-LEX-C354-WHERE-28` is *el cine*, `ES-LEX-C360-REF-19` is *el pantalón*,
`ES-LEX-C342-CLOCK-03` is *el punto*. The inventory already probes 102 such
atoms, so they are first-class and wireable. The scan could not see any of them,
reported "nine of 23 notes contradicted", and that figure was written into a
backlog entry, a commit message and a PR comment before anyone checked it.

Corrected by matching lesson `headword:` instead, the six notes this tranche
rewrote were understating taught vocabulary almost everywhere:

| point | claimed taught | actually taught |
|---|---|---|
| A1-NE12-02 clothing | camisa | pantalon, falda, camisa, zapato (4 of 7) |
| A1-NE18-01 arts | 4 of 8 | cine, musica, pelicula, concierto, foto, museo (6 of 8) |
| A1-NE16-02 computing | ordenador, Internet | + correo electronico (3 of 4) |
| A1-NE09-06 e-mail | Internet, correo | + punto (3 of 7) |
| A1-NE18-06 cinema | pelicula | + cine |
| A1-NE08-02 shows | pelicula, concierto | + cine, museo |

**No total is offered to replace 21% or "about ten".** Two scans have now
produced two confidently-wrong figures from the same repository; a third
estimate from a third heuristic has not earned trust. What is durable is the
method: **resolve an exponent through the lesson's `headword:`, which is the
field that actually carries vocabulary, and never through the spelling of an
atom id.**

### The rule that follows

A point's `note` is gating data, not commentary. Every absence claim in one is a
measurement that can rot, and they rot silently and in the flattering direction
for whoever last added a lesson. Chapters 340-415 introduced cine, museo,
pantalon, falda, camisa, zapato, punto, ordenador, internet, correo electronico,
concierto, pelicula, foto, musica, preferir and examen while notes went on
asserting their absence.

The gate worth building is exact and cheap: **no `note` may assert a word is
absent while a lesson carries that word as its `headword:`**. It would have
caught every error above, in both scans, on the day each went stale.

### Still open

A1-NE18-01 needs only teatro and exposicion. A1-NE16-02 needs only pagina web.
A1-NE02-01 (character adjectives) has alegre and simpatico taught and was never
examined. These are candidates for the next tranche and are recorded here as
candidates, not as coverage.
