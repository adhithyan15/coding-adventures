## Seven chapters on the function words the book was already using: 42/74 → 67/74

The last pass closed `Lexique de base` at 10/10 and said the finding out loud:
**every remaining open point was grammar or a function word**, and no quantity of
headwords would move any of them. This is the tranche that follows from it, and
the reason its ratio is so high is uncomfortable —

> **Eleven of the words these chapters teach were already in the book's own
> prose, with no lesson owning any of them.**

*et* was "the glue word" inside a practice dialogue. *et toi ?* was a phrase in
two more. *de* was inside *du* and never met alone. *que* was inside *est-ce que*
and *parce que* and never met at all. *très*, *beaucoup*, *un peu*, *ou*, *mais*,
*quoi* and *finir* were all being used and none was being taught.

### Measured with `measureExamCoverage`, before and after, on the merged tree

| | before | after |
|---|---|---|
| A1 exam coverage | 42/74 (57%) | **67/74 (91%)** |
| lessons | 232 | 274 |
| atoms taught | 314 | 353 |
| atoms never revisited | 37 | **11** |
| reinforcement window misses | 723 | **675** |
| forward references | 76 | 116 |
| lessons over the computed 300s ceiling | 0 | 0 |
| rule statements / paradigm tables / full grids | 32 / 91 / 20 | 32 / 91 / 20 |
| cross-chapter prose references | 0 | 0 |
| LaTeX warnings (all six kinds) | 0 | 0 |

### Eight columns close

| column | before | after |
|---|---|---|
| Le verbe | 12/17 | **17/17** |
| Les pronoms | 2/5 | **5/5** |
| La phrase | 0/4 | **4/4** |
| Le nom | 1/4 | **4/4** |
| Prononciation et orthographe | 4/5 | **5/5** |
| La negation | 2/3 | **3/3** |
| L'interrogation | 5/5 | 5/5 |
| Lexique de base | 10/10 | 10/10 |
| Les determinants | 2/8 | 6/8 |
| Les prepositions | 0/4 | 1/4 |

### One fact, planted and spent three chapters later

Chapter 39 says the **u** in **au** used to be an **l**. Chapter 42 spends it:
*chevaux* is not an irregular plural at all, it is *chevals* with that same *l*
softened, and the *x* is a spelling convention for a *u-s* that got there
honestly. One twelfth-century sound change, two chapters apart, and the second
one costs nothing because the first paid for it.

The same shape runs through the tranche. **ecce**, "behold," is inside *ce*,
*ici*, *voici* and *voilà* — four words, one root, four different moments of
freezing. The unstressed-versus-standing accident that split *que* from *quoi*
is the same accident that split *je* from *moi* and *tu* from *toi*, and the same
one that did **not** split *elle*.

### Zero reinforcement debt created, and forty-eight paid

Every one of the 39 new atoms is revisited, and **not one misses a window it was
long enough to have**: R1 0, R2 0, R3 0, R4 0.

The 42 added lessons also **newly expose** windows the old tail was too short to
have — an atom taught in chapter 37 had four lessons after it, so R2, R3 and R4
were uncounted for it. Those are paid as the tranche goes:

| | before | after | this tranche's own |
|---|---|---|---|
| atoms never revisited | 37 | 11 | 0 |
| R1 misses | 103 | 103 | 0 |
| R2 misses | 188 | 188 | 0 |
| R3 misses | 263 | 228 | 0 |
| R4 misses | 169 | 156 | 0 |

**Twenty-six atoms stopped being orphans.** Ten of them were etymologies —
*désolé*, *jaune*, *oui*, *plaire*, *pleuvoir*, *vert*, *sel*, *sucre*, *œuf*,
*beurre* — taught once and never touched again, and they are paid by two distant
bands in chapter 44 that had a reason to name them anyway.

### The one number that rose, and the decomposition

`forwardReferences` goes 76 → 116, and **all forty are pre-existing debt made
legible**. Every one is a lesson using a word that this tranche now teaches:

| word | earlier uses | now owned by |
|---|---|---|
| *de* | 8 | `FR-C35-de` |
| *très* | 6 | `FR-C40-tres` |
| *que* | 6 | `FR-C39-que` |
| *ou* | 5 | `FR-C36-ou-conj` |
| *et* | 4 | `FR-C36-et` |
| *beaucoup*, *un peu*, *finir*, *mais*, *quoi*, *parce que* | 10 | chapters 36–40 |

A word used and never taught scored **zero** before this tranche. The same word
used before it is taught scores **forty**. The second number is the honest one,
and the fix is ordering rather than authoring — which is exactly why the
possessives (`A1-D-06`) are **not** in this tranche: *mon* and *ma* have been in
the book since chapter 2, and teaching them at chapter 44 would have made forty
earlier lessons point forward at it.

### A splitter that read a French pronoun as a Spanish conjunction

`continuity.ts` split a headword on ` y ` to catch Spanish alternatives like
*negar y preguntar*. French **il y a** contains that exact string, so the
headword parsed as *il* and *a* — registering the lesson as teaching **il**,
after which **44 earlier lessons** saying *il est* or *il fait* reported as
forward references to a chapter-38 lesson about existence.

That is a mis-measurement rather than debt, and it is now fixed at the source:
the ` y ` split is scoped to Spanish, grounded in a census of every headword in
the corpus — **exactly two contain ` y `, and both are Spanish.** The article
allowlist immediately below it was written for the same reason after `así que`
registered `que`, and the note there says the lesson had to be renamed to work
around the measurement. Renaming a lesson to satisfy a splitter is what that
allowlist exists to stop.

### What is left, and why

Seven points. Three of them are `Les prepositions` — place, countries and time —
and they are the **worst ratio in the inventory**: fourteen words for three
points, against the thirty-five words that bought twenty-five points here. The
others are the possessives (blocked on ordering, above), the tens above twenty,
the pre-posed adjective set (`FR-C13-vin-rouge` names the class and gives *bon*
as its only member — "a few others you will meet"), and the time adverbs.

French's 74 points restate the DELF A1 structure rather than the Spanish/PCIC
proxy the twenty derived tracks use, so a point here is worth proportionally much
more and the percentage is not comparable with theirs.

