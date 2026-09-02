## HL-C194 — Twenty-one Tamil headwords are load-bearing for want of a romanization

> **DONE.** All twenty-one now declare a `romanization`, each transcribed from
> that lesson's own prose. `headwordsWithoutRomanization` 21 → 0 and closure
> violations 29 → 21, exactly as this entry predicted. What is left of Tamil's
> closure debt is a different problem and is written up separately, in
> `02350-HL-C194-TAMIL-CLOSURE-IS-NOW-ORDERING-NOT-ABSENCE`. This entry is kept
> because these directories are append-only history; read it for the method, not
> as open work. The one instruction it gave that had to be exercised —
> *do not invent a romanization for a headword whose body does not already say
> the word aloud* — applied to two lessons, and both were fixed in the page
> instead: `TA-W00-va-guided-copy` and `TA-C19-vayathu` now say their headword
> aloud, so nothing had to be left and reported.

`measureScriptClosure` draws its exposure line at one mechanical place: a
lesson's headword is exposure when the lesson declares a `romanization`, and
something the reader has to decode when it does not. Tamil declares none on
**21 lessons whose headword is written in Tamil script**, so the track reports
`headwordsWithoutRomanization: 21` and carries 29 closure violations, several of
them for glyphs that appear nowhere except the undeclared headword.

Every one is a lesson that already teaches its word by ear in the body. The
missing field is metadata, not pedagogy — which is why the remediation is worth
doing and why it is not a way of hiding from the measurement: writing down how
the word is said is a real gain for the learner and it converts the headword
from load-bearing to exposure at the same time.

The twenty-one, as of chapter 65:

```
TA-C01-aam                          TA-C15-thanneer-arisi
TA-C01-nandri-family-register       TA-C16-thamizh-maadangal
TA-C02-magizhcci                    TA-C17-nanpakal-nalliravu
TA-C02-ungal-peyar-enna             TA-C17-transparent-middle-synonyms
TA-C03-eppadi-irukkirirgal          TA-C19-age-register-grammar
TA-C06-dative-subject-family        TA-C19-vayathu
TA-C09-mannikkavum                  TA-C20-pathinondru-irupathu
TA-C10-vaara-kizhamai               TA-C21-naay-poonai
TA-C11-nirangal                     TA-C25-goodnight-alternatives
TA-C12-kudumbam                     TA-W00-va-guided-copy
TA-C14-kaalangal
```

Two things make this larger than twenty-one edits, and are the reason it was
left out of the chapter-65 tranche rather than bolted onto it:

- **Most of these headwords are lists, not words.** `TA-C16-thamizh-maadangal`
  carries all twelve Tamil month names; `TA-C12-kudumbam` carries six kinship
  terms. Each needs a romanization that matches the transliteration the body
  already uses, checked word by word, or the field contradicts the lesson.
- **It re-renders sixteen book chapters.** The romanization is printed, so every
  touched chapter's generated `.tex`, narration and hashes move with it. That is
  a mechanical diff, but it is a wide one, and it wants to be reviewable on its
  own.

Do it as a single sweep, and re-measure `headwordsWithoutRomanization` and the
closure violation count together — the second should fall on its own as the
first reaches zero. Do **not** invent a romanization for a headword whose body
does not already say the word aloud; leave that one and report it.
