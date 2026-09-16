## Unreleased — chapters 90 and 91: the same agreement rule taught from both sides

Hindi A1 exam coverage **197/282 (70%) → 199/282 (71%)**, closing
`HI-A1-LEX-09` (physical characteristics of a person) and `HI-A1-LEX-11`
(character and personality adjectives).

### Chapter 90 is the complement of chapter 88, on purpose

Chapter 88 taught four adjectives and **none of them agreed** — and every one of
them ended in a consonant. Chapter 90 teaches four that **all** agree, and every
one ends in **-आ**. Set side by side, the two chapters let a learner sort any
Hindi adjective by its last letter instead of by a list.

| chapter | the words | ending | agrees? |
|---|---|---|---|
| 88 | āsān, mushkil, sundar, badsūrat | consonant | no |
| 90 | lambā, nāṭā, patlā, moṭā | **-आ** | yes |

### The fourth word was not optional

The inventory note was precise: *"`bara` and `chhota` are taught for things. No
word for tall, short, thin or heavy."*

Three words would have covered tall, thin and heavy, and it is tempting to wire
the fourth onto **छोटा**, which the track has had since chapter 41. That would
have been wrong, and not for politeness reasons: **about a person, *choṭā* means
*younger*.** *Choṭā bhāī* is a younger brother whatever his height. Hindi spent
its small-word on age and left height without an owner.

So **नाटा** (*nāṭā*) is authored as a real gap rather than a synonym, and the
review lesson makes the distinction explicit rather than leaving it implied. The
lesson is also honest about register: *nāṭā* is blunt about a person, and the
neutral phrase is *qad meṅ choṭā*, "short in height" — recognise the first,
reach for the second.

### Chapter 91 answers the same source as the Spanish point it derives from

`HI-A1-LEX-11` carries `derivedFrom: ES:A1-NE02-01`, and its note read *"Only
`achha`. Nothing describes a person's disposition."* Six words close it:

| word | meaning | ending | agrees? |
|---|---|---|---|
| बुरा | bad | **-आ** | yes |
| शर्मीला | shy | **-आ** | yes |
| होशियार | clever | consonant | no |
| मिलनसार | sociable | consonant | no |
| गंभीर | serious | consonant | no |
| मेहनती | hard-working | **-ई** | no |

Three shapes in one chapter, and the review lesson shows the split is not
arbitrary: **inherited** words mostly kept the **-आ**, **borrowed** ones mostly
arrived ending in a consonant. **गंभीर** is the inherited exception that keeps
this a tendency rather than a law — which is why the lesson tells you to act on
the *ending*, never on the etymology.

Two of the words are worth their own note:

- **होशियार** is Persian *hoś* (wits) + *-yār* (holding) — one who **keeps hold
  of their senses**. Shouted, it means *look out!*, which is the same idea about
  alertness pointed at a different moment.
- **शर्मीला** turns on an ending the exam actually needs. **-ई** says *has this
  thing*; **-ईला** says *is made of it*. A person can feel shame once; a
  *śarmīlā* person has it as a standing trait. That is settled character rather
  than mood, which is the whole point of the category.

### Terminal-lesson debt, budgeted rather than discovered

Each chapter carries a *repaso* and a *síntesis* after its word lessons, so
every one of the ten new atoms gets at least two later retrievals.

Measured against a clean-main equivalent rather than assumed:

| | before | after |
|---|---|---|
| attained level | null (in progress at pre-A1) | **unchanged** |
| reinforcement shortfall | 44 | **44** |
| pre-A1 headwords | 191 | **191** |

The shortfall stayed **flat**, which is the evidence the budget was right.

### Two gates caught real problems before generation

**Script closure: zero new debt, checked four times.** Devanagari glyphs that
the corpus shows but never teaches were found in the drafts and removed from
body prose each time — **छ** (in *acchā* and *choṭā*), **औ** (in *aur*), **झ**
(in *patjhaṛ*) and **थ** (in *thoṛī*). All were replaced with romanization, the
convention the Tamil precedent set. **No exemption list was edited and no
ceiling was touched**; the glyphs remain pre-existing debt owned by the lessons
that first showed them.

**Two ceilings rose by one each and were fixed in the prose, not the pin.**
`banned-words` 1056 → 1057 caught *"simply having the thing"* in the
**शर्मीला** wrap-up, and `info-dump` 32 → 33 caught *"The rule is the one you
already have"* in **लंबा**. Both were reworded and both ceilings are back where
they were.

The table-width check ran **before** generation this time rather than after the
suite, following the lesson recorded during the Spanish chapters: not one of the
fourteen lessons carries a table the narrator would refuse.

### Pins

Moved to measured values, none loosened:

| pin | before → after |
|---|---|
| Hindi lesson count | 366 → 380 |
| `coverage.covered` | 197 → 199 |
| `coverage.unmapped` | 85 → 83 |
| report string | 197/282 → 199/282 |

### Two counts went up, and both stay up

`measurement-blind` **3 → 5** — the two new *síntesis* lessons. This is
**HL-C377** for the third time: `isExplicitRetrievalOnlyLesson` in `ramp.ts`
accepts `review`, `practice` and `practice-mix` but not `synthesis`. The type
was **not** relabelled to clear the count.

`reinforcementWindowMisses` **968 → 996**. This is not the level-gate shortfall,
which stayed flat at 44. It rises because the track got fourteen lessons longer,
so R-windows previously skipped for earlier atoms now fall inside the track and
get judged.

### Verification

`human-language-data` 145 files / 2083 tests; `language-ladder` `bash BUILD`
39 files / 442 tests; all twelve `check:*` gates; `check-book-compile.sh hindi`
under XeLaTeX.

### What this does not claim

Coverage means the teaching exists, not that a reader scores. No track has had a
single exam item graded against it.
