## Zero characters used but untaught

`TE-A1-L-08` — *the independent vowel letters, for a vowel that starts a word*
— closes. Telugu A1 coverage **217/326 → 218/326**; the script column
**14/23 → 15/23**.

**The number that matters is not the coverage one.** Counted over every Telugu
lesson body, the characters this track prints without teaching now stand at
**zero**. Three passes ago that number was fifteen.

| metric | before → after |
|---|---|
| exam-point coverage | 217/326 → **218/326 (67%)** |
| characters used but untaught | **5 → 0** |
| uses of an untaught character | **489 → 0** |
| atoms taught | 474 → 479 |
| measurable lessons | 365 → 371 |
| writing-practice lessons | 97 → 103 |
| `forwardReferences` | unchanged |
| `scriptClosureViolations` | unchanged (8) |
| `durationViolations` | unchanged (0) |
| `atomsNeverRevisited` | unchanged |
| `payoffSurprises` | unchanged (0) |
| reinforcement-window misses | 879 → 898 |

### What was wrong

Four of Telugu's independent vowel letters had a lesson — *i*, *u*, *e* and
vocalic *r*. Five more were on the track's own pages with nothing teaching
them:

> అ 303 · ఆ 104 · ఏ 57 · ఐ 13 · ఒ 12

**అ was the most-used untaught character left in the track.** It opens అవును,
the word for *yes*, in chapter one — and it is the vowel every consonant
lesson had already been describing without ever showing the reader its letter.

### What was added

| lesson | seq | ch | character | anchored in |
|---|---|---|---|---|
| `TE-S162-letter-a` | 31 | 1 | అ | అవును |
| `TE-S163-letter-ee` | 112 | 2 | ఏ | ఏమిటి |
| `TE-S164-letter-ai` | 331 | 7 | ఐ | ఐదు |
| `TE-S165-letter-o` | 342 | 7 | ఒ | ఒకటి |
| `TE-S166-letter-aa` | 343 | 7 | ఆ | ఆరు |
| `TE-S167-script-recall-vowels` | 376 | 10 | — | cold retrieval of all five |

**The tranche is built around one question, not five shapes**: *is a consonant
carrying this vowel?* If yes, a sign; if no, a letter. Every lesson here asks
it again.

- **అ** is the odd one out and goes first, at sequence 31, before any vowel
  sign has been taught. It is the vowel whose sign is the absence of a sign —
  which is why క is *ka* and not *k* — so it can be taught with nothing else in
  place, and the first actual sign (◌ా) follows four slots later.
- **ఏ** is the ordinary case, and it is placed where the reader has had ◌ే
  since chapter one. Sign known, letter new, pattern stated.
- **ఒ** sits one slot after ◌ొ, the short-*o* sign, so the pair lands on
  facing pages inside the same list of numbers: తొమ్మిది carries the sign,
  ఒకటి carries the letter.
- **ఆ** closes the *a* square — short and long, carried and standing alone,
  with the one empty cell called out as the surprise it is.
- **ఐ** is the diphthong, and the point of its lesson is that Telugu spends one
  character on a vowel English writes with a digraph, a silent letter or
  nothing at all, depending on the word.

### Why the point closed, and what it does not claim

Nine of Telugu's fourteen independent vowel letters now have a lesson. The five
without one — ఈ, ఊ, ఌ, ఓ, ఔ — have **zero occurrences across all 365 lesson
bodies**, on the same terms as the five consonants left open by `TE-A1-L-10`
one pass earlier. The closure is against the point's label — *for a vowel that
starts a word* — and not against a full vowel chart. `HL-C391` carries the
chart as a separate future entry.

### One of those absences is not a script problem

**ఈ is the everyday Telugu word for "this" in front of a noun**, and it appears
nowhere in the corpus. `TE-C41` teaches the pronouns ఇది and అది — *this one*,
*that one* — and never the adjective pair ఈ / ఆ. A reader finishing this track
can point at something and cannot say "this book".

Teaching the letter would not have fixed that; the word has to land first, and
then the letter has an anchor like every other script lesson here. `BACKLOG.d`
`HL-C392` carries it as **lexical** work, which is where it belongs. It is the
same shape as the gaps Malayalam and Tamil turned up in their pronoun columns:
a rule taught correctly, with half of what it predicts never handed over.

