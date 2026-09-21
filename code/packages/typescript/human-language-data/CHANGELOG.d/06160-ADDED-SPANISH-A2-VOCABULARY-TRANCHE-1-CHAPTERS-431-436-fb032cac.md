### Added — Spanish A2 vocabulary, tranche 1 (chapters 431–436)

Thirty headwords, **chosen by the exam rather than by theme**. The A2 mock audit
named 191 lexemes the two DELE A2 papers need and the corpus did not teach; the
46 appearing in two or more items are the priority, and this tranche covers 30
of them.

```
A2 objectiveFailed        93 -> 88
A2 missingObjectiveLexemes 191 -> 161
```

**The lexeme count fell by exactly 30** — the number of headwords taught. A word
already taught under another name would have made the drop smaller, so the
arithmetic is the proof that all thirty were genuinely missing.

`objectiveFailed` moved only 5 over the same thirty words, and that is expected
rather than disappointing: an item passes only when **every** lexeme in its
`requires` row is taught, so one missing word holds a whole item red. The lexeme
count is the leading indicator; the item count moves in steps as rows complete.

#### The chapters teach a morphology spine, not a wordlist

| ch | ending introduced | the word that does **not** come apart |
|---|---|---|
| 431 El Pedido | **-dor** (proveedor) | — |
| 432 La Casa | **-ería** (estantería, tubería) | **sofá** — Arabic, whole |
| 433 El Edificio | **-ero** (portera) | **garaje** — French, respelled |
| 434 El Taller | reuses **-dor** | **batería** — wears the ending, built in French |
| 435 Los Papeles | **-ción**, **-encia** | **currículum** — kept its Latin **-um** |
| 436 Gente | — | **grupo** — via Italian, whole |

**batería** is the pivot. After three chapters of endings that pay off, the
learner meets a word that *looks* built and is not: the derivation happened in
French and the pieces never entered Spanish. Every chapter carries one such
word, so the habit taught is *look for the join*, never *there is always a join*.

435's **jubilado** answers 431's **el pedido** — the same participle-hardening
move, once into a thing and once into a person. 436 closes a lending cycle the
learner can now say end to end: **prestar** hands it over, **recoger** fetches
it, **devolver** brings it back, and the borrower's *pedir prestado* contains the
same *pedir* that sits under *el pedido*.

#### compañero chains to chapter 26 rather than re-teaching it

`ES-C26-pan` has taught the etymology since the bread lesson, under atom
`ES-ETYMON-COMPANION-03`: *com-* plus *pānis*, **one you share bread with**. The
learner was given the story and never the word. Its lesson says so and reuses
that atom. A census found **45 of the 191** already appear in lesson bodies
without being headwords — exposure, not teaching, the same distinction
`script-closure` draws for glyphs — and each should chain the same way.

#### Four things the gates caught that nothing else would have

1. **These lessons are A1, not A2.** A level is derived from the **spine node**
   the path segment names — `levels.ts` says so in capitals, precisely so "a
   track cannot claim A1 by editing frontmatter". `SPINE-READ-SIGNS-AND-NOTICES`
   is `stage: A1`, so the extension's own `stage` field does not move a lesson's
   level. The tranche still counts toward the A2 audit, because
   `lessonsUpToLevel("A2")` includes A1 — but the **A1** audit grew by 42 lessons
   and was regenerated. Reaching A2 properly needs an A2 text-strand spine node,
   which is a spine change and not this one.
2. **31 curriculum-graph errors.** `practises.knowledge` must be exactly what the
   body blocks assess, and every required atom's introducing lesson must sit in
   the prerequisite closure.
3. **Six standalone-book offenders**, all the phrase *"the course"* in the
   `currículum` lesson. Meant in the Latin sense; in a standalone PDF it reads as
   *this* course. Rewritten to *the running of a life*.
4. **A level regression.** The level gate dropped Spanish from **attained A1 to
   pre-A1** on the `reinforcement` criterion: *two atoms at or below A1 are
   revisited fewer than twice*. **sofá** was revisited only by its repaso and
   **grupo** only by its payoff. Both were given a genuine second outing rather
   than padded metadata, and Spanish is back to **attained: A1**.

Point 4 generalises and is worth stating plainly: **adding vocabulary can lower
a track's attained level.** A chapter whose payoff does not reach every word it
taught leaves that word under-reinforced. The 5-words-plus-repaso-plus-payoff
template does not guarantee two revisits; it has to be checked.

#### Verification

`npm run validate` 21/21, all thirteen gates, the full suite at 172 files /
**2204 passed** / 1 skipped, `check-book-compile.sh --strict spanish` with the
Spanish warning counters unchanged from before the tranche, and the membership
digest moved by **exactly 42** lessons across the six new path segments —
attribution checked, not assumed.

Also filed: `HL-C416`, recording that the A1 and pre-A1 mock papers use five
live-ccTLD domains (`@correo.es` ×2, `@hotelmar.es`, `@hotelsol.es`,
`@sanmartin.es`) which can resolve and so do not meet the placeholder rule.
Pre-existing and untouched here.
