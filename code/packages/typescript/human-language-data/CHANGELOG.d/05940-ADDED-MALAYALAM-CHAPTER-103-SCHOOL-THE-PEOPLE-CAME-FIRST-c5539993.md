### Added — Malayalam chapter 103, school, where the people arrived 2,400 points before the place

- `ML-A1-LEX-33` closes. Malayalam A1 coverage 209/243 -> **210/243 (86%)**, 33
  points unmapped. **The percentage holds** — 210/243 is 86.4 — recomputed, not
  carried.

#### The note was partly stale, for the second tranche running, in the same shape

It read: *"None of paLLikkoodam, paadam or pareeksha is taught."*

True of those three words. What it leaves out is that the **domain was not
empty**: `ML-C48-teacher` (1290) owns **അധ്യാപകൻ** and `ML-C48-student` (1300)
owns **വിദ്യാർത്ഥി**. The learner has had the **people** of a school since
sequence 1290 with nothing around them — and that is the gap the chapter is
built on: the place, the thing taught in it, the test of the thing, and the
number at the end.

Two tranches running, the note was accurate about the words it named and wrong
about the scope it implied. `ML-A1-TIME-10` named the wrong *case*; this one
named three absent words and let them stand for a whole domain.

#### Four words, three origins

| | | |
|---|---|---|
| **പള്ളിക്കൂടം** | *paḷḷikkūṭaṁ* | a compound of two buildings |
| **പാഠം**, **പരീക്ഷ** | *pāṭhaṁ*, *parīkṣa* | Sanskrit |
| **മാർക്ക്** | *mārkkŭ* | English |

**പള്ളിക്കൂടം is *paḷḷi* plus *kūṭaṁ*** — a place of worship plus a hall. The
etymology is given in **romanization only**: neither part is a taught headword,
and naming them in script would introduce two unowned tokens, which is the trap
`lessons.d` records as *"the parts a lesson names to explain a word are forward
references too"*. The historical claim is hedged to what is checkable about
**modern** Malayalam, with the *why* offered as the traditional account rather
than asserted.

#### The two Sanskrit words deliberately disagree

**പാഠം** ends in the anusvara and gives way to **-ത്ത-** under a case ending —
**പാഠത്തിൽ** — exactly as `HL-C400` narrowed that rule to say. **പരീക്ഷ** ends in
**അ** and does no such thing. Same chapter, same subject, same source language,
different endings, and the chapter says outright that **the ending decides, not
the meaning**. That is a revisit of `ML-GRAMMAR-C83-OBLIQUE-AM-01` on a word that
earns it rather than a restatement of it.

#### A glyph claim checked against the data and the font, not against a note

**This is the finding worth keeping.** `പരീക്ഷ` needs the **ക്ഷ** conjunct, and
`ML-A1-SCR-12`'s note lists *"sha (14)"* among nine characters the corpus never
teaches. Read literally, that refuses the word.

It is the wrong letter. Malayalam has two:

| letter | codepoint | taught? |
|---|---|---|
| **ശ** *śa* | U+0D36 | **no** |
| **ഷ** *ṣa* | U+0D37 | **yes** — `ML-S121-letter-ssa`, sequence 471 |

**ക്ഷ** is **ക** + virama + **ഷ** — the *taught* one. The word costs no script
debt at all. Filed as `HL-C407`, along with the finding that SCR-12's **14** is
one of **three** numbers: **12** distinct tokens in headword position carry
**ശ**, across **15** headword fields, and the two differ because **ശനി** and
**വൃശ്ചികം** sit inside multi-word headwords.

Separately, **പാഠം needs U+0D20**, a character **no headword in this corpus has
ever used**. The `NotoSansMalayalam` cmap was read directly before the word was
committed to — present — which is what `lessons.d` means by *"a character no book
has rendered before is invisible to every local gate"*.

#### Romanizations copied from named witnesses, with one derived and said so

- **kṣ** from six unanimous headwords (`ML-C09`, `ML-C49`, `ML-C53`, `ML-C63`,
  `ML-C65`, `ML-C92`), with **അപേക്ഷ** → *apēkṣa* fixing the word-final **-kṣa**
- the chillu **ർ** before a consonant → **r** from seven (*śarkkara*,
  *cērkkuka*, *karṣakan*, *bharttāvŭ*, *karkkaṭakaṁ*, *tīrccayāyuṁ*,
  *vidyārtthi*)
- **ṭh** for U+0D20 is the one **derived** rather than copied, because that
  character has no witness anywhere: **ട** is *ṭ*, and every aspirate in this
  corpus carries an *h* (**ഭ** → *bh*, **ധ** → *dh*), so **ഠ** → *ṭh*

#### The full suite caught a second gate, and the fix was to teach the letter

`script-closure.test.ts` pins Malayalam at **`neverTaughtGlyphs === 0`**, and
**പാഠം** broke it. That pin is a stronger claim than `ML-A1-SCR-12`'s note
suggests: **ശ** counts as taught by this metric because it appears inside another
letter lesson's example words, which is the understatement `lessons.d` already
records. **ഠ** appears nowhere at all — not in a headword, not as an example —
so it was the one genuinely never-taught glyph in the track.

**The fix is `ML-S147-letter-ttha`, not a different word for *lesson*.** Closure
is measured in **reading order**, so the letter is taught at sequence **3725**,
between the school and the lesson that needs it. **ഠ** is **ട** with a breath
after it, which makes it a natural pairing lesson rather than an errand.

**The draft claimed more than that and was wrong.** It called ഠ *"the only letter
in the book that arrives with no history"*, which **ten** merged script lessons
contradict in identical words — `ML-S07`, `ML-S110`, `ML-S111`, `ML-S116`,
`ML-S125`, `ML-S127`, `ML-S131`, `ML-S141`, `ML-S143` and `ML-S144` each say of
their own character *"it has not been on a page yet: it arrives with the word on
the next one."* Arriving with the word that needs it is this corpus's **normal**
way of introducing a letter, not an exception. The lesson now uses that same
sentence, which is true and is the house voice for exactly this situation. What
is distinctive about ഠ is a fact about the **metric**, not about the reader, and
it belongs in this changelog rather than in a lesson.

Teaching it was the cheaper honest option. Padding an existing letter lesson's
examples with **ഠ** would have moved the same number without teaching anybody
anything, which is exactly the failure mode `lessons.d` names when it says the
metric "understates real script debt by a factor of three."

#### A check that was circular, and caught by the gate

The first draft tagged `sounds: [malayalam-retroflex-lla]` and
`[malayalam-ksha]`, chosen from a grep of "sound ids already in use" — **which
included the two files being validated**. Both tags are unregistered; the
registry has `malayalam-geminate-lla` and `malayalam-conjunct-kssa`, the latter
already used by `ML-C09-kshamikkanam` for this very conjunct. A corpus sweep run
over a set that contains the new work answers a different question than the one
being asked.
