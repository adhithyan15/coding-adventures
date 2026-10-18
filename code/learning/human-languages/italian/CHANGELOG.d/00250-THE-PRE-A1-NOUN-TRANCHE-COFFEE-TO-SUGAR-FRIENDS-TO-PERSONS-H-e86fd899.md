## The pre-A1 noun tranche: coffee to sugar, friends to persons, heart to throat (HL09)

Fifteen everyday nouns across four chapters (22–25), the track's first tranche
built entirely of nouns rather than verbs, targeting the level-gate's
`vocabulary` blocker at pre-A1. Wired to three pre-A1 spine nodes through new
`IT-PATH-026`..`029` segments and `IT-EXT-026`..`029` language-specific
extensions:

- **Chapter 22 — Coffee, Tea, Milk, and Sugar** (`IT-C22-caffe`, `IT-C22-te`,
  `IT-C22-latte`, `IT-C22-zucchero`; concepts `IT-FOOD-COFFEE`, `IT-FOOD-TEA`,
  `IT-FOOD-MILK`, `IT-FOOD-SUGAR`) on `SPINE-POLITE-REQUEST-REPAIR`, alongside
  Chapter 11's bread and wine. Three loanwords and one inherited word at the
  same table: *caffè* through Ottoman Turkish from Arabic *qahwa*, *tè*
  through Dutch from Hokkien Chinese *tê* (the sea route, against the *chai*
  languages' overland one), *zucchero* from Arabic *sukkar* by a different
  road than *caffè* — through Old French, the same route English *sugar*
  took — and *latte*, straight from Latin *lac*, never borrowed at all.
  *Zucchero* is also the first noun in the track that actually needs Chapter
  1's second article, *lo*, on a real word rather than an invented example.
  11 new atoms.
- **Chapter 23 — Friends, Family, and a Name** (`IT-C23-amico-amica`,
  `IT-C23-famiglia`, `IT-C23-nome`, `IT-C23-persona`; concepts
  `IT-PEOPLE-FRIEND`, `IT-FAMILY-GROUP`, `IT-WORD-NOME`, `IT-WORD-PERSONA`) on
  `SPINE-EXCHANGE-NAMES`. *L'amico*/*l'amica* ← *amicus*/*amica*, from
  *amāre* ("to love") — and English *enemy* is the same root with a "not" in
  front (*inimicus*). *Il nome* is the word Chapter 3's *mi chiamo* ("I call
  myself") always talked around without ever saying. *La persona* closes the
  chapter: grammatically feminine by fixed category, never by the sex of
  whoever it names — a second, sharper example of the gender-vs-referent gap
  Chapter 17's *la mano* first raised. 11 new atoms.
- **Chapter 24 — Heart, Eyes, Ears, and Mouth** (`IT-C24-cuore`,
  `IT-C24-occhio`, `IT-C24-orecchio`, `IT-C24-bocca`; concepts
  `IT-BODY-HEART`, `IT-BODY-EYE`, `IT-BODY-EAR`, `IT-BODY-MOUTH`) and
  **Chapter 25 — Nose, Stomach, and Throat** (`IT-C25-naso`, `IT-C25-stomaco`,
  `IT-C25-gola`; concepts `IT-BODY-NOSE`, `IT-BODY-STOMACH`, `IT-BODY-THROAT`)
  on `SPINE-CHECK-WELLBEING`, paying off the "next: the rest of the body"
  line Chapter 17's *la mano* lesson ended on. *Occhio* and *orecchio* are
  built by the same Italian sound-law, applied twice: an unstressed vowel
  drops from *oculus*/*auricula* and the leftover *-c(u)l-* hardens to
  *-cchi-*. *Bocca* is Vulgar Latin slang for "cheek" that displaced the real
  classical word for mouth, *os/oris*, across nearly all of Romance. *Stomaco*
  is the one body word that passed through Greek (*stómakhos*) before it ever
  reached Latin. 11 + 8 new atoms.

**Vocabulary, measured with `buildCurriculumGapReport` before and after:**

    headwords at or below pre-A1     30 -> 45
    vocabulary shortfall (of 300)   270 -> 255
    track vocabulary (any level)     67 -> 82
    reinforcement shortfall (pre-A1) 24 -> 13
    attained / inProgressAt        null / pre-A1  (unchanged)

Fifteen new word lessons moved the pre-A1 headword count by exactly fifteen —
`vocabularyOf()` counts distinct `headword:` strings, and even the paired
`l'amico, l'amica` lesson contributes one, the same as every other track's
prior noun tranche found. Closing pre-A1 on vocabulary alone still needs
~255 more lessons of this shape.

The reinforcement drop is real work, not incidental: eleven of the 24 atoms
the continuity ledger reported as revisited fewer than twice before this
tranche are now revisited at least twice, rescued by reaching back into
`practises.knowledge` from these new lessons — *così così* and *prego*'s
sounds from Chapter 2, *età*'s three atoms from Chapter 14, *parlo italiano*
from Chapter 5, and *mano*/*testa* from Chapter 17. The other thirteen (seven
`(practice)`-lesson atoms, the *passato remoto*'s three, and three farewell
sound-atoms from Chapter 4) were left alone deliberately: none of them had an
honest home in a noun tranche about drinks, people and the body, and forcing
one would have been exactly the mismatched-node move the prior wave warned
against.

**A structural finding, not a new one — confirmed, and sharpened.** The prior
wave's three tracks (Hindi, Arabic, Tamil) reported that the seven pre-A1
spine nodes are all social speech acts with no concept for naming a concrete
object in front of you, and dropped household words that had no honest home
rather than force them on. Italian's level-gate report carries a second,
different `spine-nodes` finding worth naming precisely: `SPINE-RESPOND-BASIC`
— the node for *yes*/*no*/*okay* — is not merely under-realized, it has **zero**
lessons (`"segments": []` in `curriculum.json`, all four of its concepts
correctly listed under `omits`). Italian has never taught *sì* or *no*. This
tranche does not touch it: it is a `vocabulary` gap on a `SAY-WHAT-I-DO`-shaped
node, not a `POLITE-REQUEST-REPAIR`/`EXCHANGE-NAMES`/`CHECK-WELLBEING` noun
gap, and belongs to whichever tranche authors Italian's basic responses.

Gates: zero forward references, zero atom-budget violations from these
fifteen lessons (Italian's three pre-existing violations — `IT-C13-rosso-blu`,
`IT-C14-eta`, `IT-C15-passato-remoto` — predate this branch), all four
chapters at or under `maxNewAtomsPerChapter: 12`, and every lesson computes
under 300 effective seconds. `narration/ch22.txt`–`ch25.txt` read correctly
aloud, including the two 3-column tables (the article-elision table in
`IT-C23-amico-amica` and the sound-law table in `IT-C24-orecchio`). The
25-chapter, 160-page book compiles under XeLaTeX with zero `Missing character`
errors.

