## A duplicate matcher that compares whole strings needs a stem rule, not a longer delimiter list

`vocabularyOf` compares full headword strings, so a near-duplicate does not fail
the A1 gate -- it **inflates** it, which is the dangerous direction. Successive
tranches have answered this by lengthening the list of things to split on:
articles, compound delimiters, `+ subjunctive` annotations, ellipses. Tranche 4
found that the delimiter list has stopped being where the misses are.

Of the candidates dropped this time, **the mechanical matcher caught none of
them**. Every real catch came from one of two places:

1. **A shared visible initial stem, across unrelated roots.** `la camisa` beside
   the taught `caminar`/`el camino`; `el abrigo` beside `abrir`; `el hombre`
   beside `el hombro`, which it differs from only in the final vowel; `el
   cuerpo` beside `la cuerda`. None of these share an etymology. All of them
   share the letters a learner actually keys on, which is the confusion the
   course exists to prevent -- the same reasoning that dropped `las vacaciones`
   for `la vaca` in tranche 3.

2. **The root ledger.** `dejar` looks nothing like the taught `lejos` and is the
   same `laxus`. `cantar` is the `incantare` already spent on `encantado`.
   `la rueda` is the `rota` whose diminutive `rotella` is the taught `la
   rodilla`. `el rey` is a third helping of a `regere` already spent on
   `regular` and `la derecha`.

The working rule tranche 4 settled on, offered here because it is the part worth
reusing: **a taught word sharing an INITIAL stem is a drop; a taught word
appearing as a non-initial substring across no morpheme boundary is a false
alarm.** That line keeps `el dedo` (inside `alrededor`, which is `retro`),
`el corazon` (ending in the taught `razon`, which is `ratio`) and `la lampara`
(containing the taught `para`) while still dropping all four stem cases above.

### The second door: a word the gate already owns but does not count

Everything above is a near-duplicate the string comparison cannot see. This is
different in kind, and it is the more dangerous of the two, because **no string
rule of any sophistication could have caught it.**

`dar` is not a headword anywhere in the Spanish track. It passes every test in
the section above: no article to strip, no compound to split, no shared stem, no
spent root. By every available check it is a free word.

It is not. `ES-C65-di` introduces the atom **`ES-LEX-DAR`** -- the course
teaches `dar`, and has for three hundred chapters. What hides it is that
`ES-C65-di` has `type: grammar`, and `vocabularyOf` is restricted to
`word`/`phrase` precisely so that drill titles and grammar labels cannot inflate
a vocabulary count. That restriction is correct and should stay. Its side effect
is that a lexeme introduced by a grammar lesson is **owned but uncounted**.

Adding `dar` as a new `word` headword would therefore have **raised the A1
number by one while re-teaching a word the learner already has**. That is the
same failure direction as a near-duplicate -- the gate going up without the
learner gaining anything -- arriving through an entirely different door. A
tranche that diffs only against the headword list walks into it every time, and
never sees why.

**The check that is missing:** a candidate has to be tested against the *atom*
ledger, across ALL lesson types, and not only against the headword list.
`grep -l "ES-LEX-<WORD>" lessons/*.md` finds it by hand today. Doing it in
`validate.ts` -- flagging a new `word` lesson whose headword matches a lexical
atom that some other lesson already introduces -- would close it permanently for
every track, and costs one pass over the corpus.
