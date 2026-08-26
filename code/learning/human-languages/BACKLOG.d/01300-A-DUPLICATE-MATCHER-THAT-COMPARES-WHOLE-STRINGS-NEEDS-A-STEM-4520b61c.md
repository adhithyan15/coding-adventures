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

There is a second, sharper miss worth recording separately. **A word can already
be taught under a headword the vocabulary gate does not count at all.** `dar` is
introduced as the atom `ES-LEX-DAR` by `ES-C65-di`, whose `type` is `grammar` --
so `vocabularyOf`, which is restricted to `word`/`phrase`, does not see it.
Adding `dar` as a new headword would have INCREMENTED the A1 count while
re-teaching a lexeme the course already owns. Checking the headword list is not
enough; the atom ledger has to be checked too.
