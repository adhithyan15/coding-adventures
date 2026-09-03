## HL-C316 — Russian A1: no caveat, so the case column had to be measured into existence

`core/exam-inventory-russian-a1.json` enumerates **228** A1 points and the corpus
covers **73** of them, 32%. That is the lowest of the eleven inventories written
so far except Marathi's 29%, and the number is not the interesting part.

### A track with no caveat gives you nothing to answer, so measure

Every earlier inventory in this series had an external steer in
`core/exam-levels.json` naming the column the Spanish proxy could not supply.
Japanese is told JLPT scores no production. Chinese is told its CEFR
correspondence is unpublished. Tamil is told it is diglossic. Sanskrit is told a
traditional syllabus is not parallel to CEFR. **Russian's entry carries none** —
exam TORFL, basis published, A1 maps to TEU, and it stops.

The temptation is to import the shape of whichever track you wrote last. What
worked instead was to walk the proxy's columns and watch where they broke:

    Spanish column        what it resolved to in Russian
    -----------------     -----------------------------------------
    5 noun points         gender (transfers), plural (absent), CASE
    8 pronoun points      three persons of six, and CASE
    6 verb-phrase points  direct object, indirect object — both CASE
    14 verb points        one -l past collapsing three Spanish tenses,
                          and ASPECT, which has no Spanish column at all

So `Padezh` and `Vid` became categories rather than footnotes, and ten of the
file's fourteen `russianSpecific` grammar points live in them. **The lesson is
that an absent caveat is not a licence to transfer more; it is an instruction to
measure more**, because nobody has done the reading for you.

### Padezh reads 3 of 10, and the missing plural is the expensive one

The track teaches exactly two case contrasts and teaches both of them well:
`menya`/`vas` against `ya`/`vy` in chapter 2, with a drill lesson whose only job
is to make the pair stick, and the genitive after `do` in chapter 11, which turns
four frozen farewells into one grammatical fact. That second lesson is the best
piece of grammar teaching in the track.

Everything else is absent: accusative of nouns, animacy, dative, prepositional,
instrumental — and **the plural, which is not late but simply not there**. Every
noun in 88 lessons is singular. That single absence blocks the numeral column
(Russian numerals govern case and number), the counted-noun column, and the
plural half of every case.

### The joining column is 0 of 13, and this time the word is on the page

Sixth track running, and the first that is neither Indo-Aryan nor Dravidian:

    i           (and)         introduced by NO lesson; PRINTED as a
                              conjunction in 2 lesson bodies
    ili         (or)          0 occurrences in 88 files
    no / a      (but)         0
    chto        (that)        0
    potomu chto (because)     0
    kogda       (when)        0
    kotoryy     (which)       0
    chtoby      (in order to) 0

Checked in Cyrillic and in romanization. Gujarati's `ane` was worse in one sense
— zero occurrences of any kind — and Russian's is worse in another: `Ya chitayu i
ponimayu` is set as a model sentence in chapter 4 and `Moloko, syr, sok i sup,
pozhaluysta` in chapter 13, so the commonest word in the language is doing
grammatical work in front of the reader who is never told it is a word. **A
one-atom lesson closes it, and `chto` after `dumat` and `znat` — both already
taught — is the single highest-value word in the Russian queue.**

### Half a repair kit is a distinct failure from none

Gujarati had no way to say "I don't understand" and no word for sorry. Russian is
the other shape, and it is worth naming separately:

    ya ne ponimayu   TAUGHT in full, chapter 4, stress marked
    ya ne znayu      TAUGHT in full, chapter 3
    izvinite         0 occurrences in 88 files
    prostite         0
    povtorite        0
    medlenno         0

So the learner can report that the conversation has failed and cannot apologise,
cannot ask for a repeat, and cannot ask for slower speech. **Reporting a
breakdown without a way out of it is arguably worse than silence**, because the
speaker has committed to a repair sequence they cannot finish. Every one of the
four missing words is an imperative or an adverb; none needs a case, a plural or
a conjunction, so this is the cheapest column in the file to close.

### 29 of 33 letters, and two of the four gaps are on the page

Counted letter by letter against `data/scripts/cyrillic.json`, which lists all 33
with a cited stroke order and is marked complete:

    taught with their own writing lesson    29
    shown in lesson bodies, taught by none   2   (yo in 5 files, hard sign in 4)
    absent from the corpus entirely          2   (shcha, e-oborotnoe)

That second row is the Kannada shape — a letter the reader is asked to decode
that no lesson closes — and here it is exactly two letters wide rather than a
whole alphabet. But the column still reads 5 of 10, and the three uncovered
points that are not about letters are the ones worth acting on: **the letter
names are never given** (so no candidate can spell aloud, which is a TORFL A1
listening task), **the spelling rule is never stated** (which decides endings all
over the noun and verb systems), and **cursive is never taught**. Russian
handwriting is a second alphabet rather than a style — `propisnoy` te, em and sha
are unreadable to somebody who has only met the printed shapes — and the ductus
file already carries a cited school hand for all 33 letters, so the evidence to
teach it is checked in and unused.

### What to author next, off this inventory

In value order, and each item is named by the point it closes:

1. `chto` as a complementiser — closes `RU-A1-SUB-02`, and gives `dumat` and
   `znat`, already taught, something to take.
2. `i`, `no`, `a`, `ili` — closes four of the five `Sochinenie` points.
3. `izvinite`, `povtorite`, `medlenno`, `eshche raz` — closes the repair column
   and needs no new grammar.
4. The nominative plural — unblocks the numerals, the counted noun and half the
   case column.
5. `eto` — closes `RU-A1-DEM-03` and turns the zero copula the track already
   teaches into the commonest sentence on an A1 Russian paper.
6. One adjective pair (`khoroshiy`/`plokhoy`) plus `khorosho`/`plokho` — closes
   `RU-A1-ADV-04`, `RU-A1-F2-04` and most of the evaluation column at once.
