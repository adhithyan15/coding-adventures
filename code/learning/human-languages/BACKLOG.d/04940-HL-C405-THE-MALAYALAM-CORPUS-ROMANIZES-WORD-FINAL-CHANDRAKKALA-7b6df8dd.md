## HL-C405 — the Malayalam corpus romanizes word-final chandrakkala two ways, and both are in force

**Status: CLOSED (2026-09-19).** Found while writing chapter 102, which needed
to put two of them on the same page and chose not to.

**The two spellings.** A word ending in a consonant plus chandrakkala is
romanized with a plain **u** in the early chapters and with **ŭ** in the later
ones:

| lesson | sequence | headword | romanization |
|---|---|---|---|
| `ML-C06-dative-ikku` | 320 | **-ിക്ക്** | *-ikku* |
| `ML-C93-counted` | 3310 | **രണ്ട് പുസ്തകം** | *raṇṭŭ pustakaṁ* |
| `ML-C95-akathu-purathu` | 3390 | **അകത്ത്** | *akattŭ* |

`ML-C06`'s own practice prints **ജോലിക്ക്** as *jōlikku*. Chapter 102 builds
**മണിക്ക്** on exactly that model and writes it *maṇikkŭ*, following the
convention every recent lesson uses.

**Why this is not a typo to fix in passing.** The early spelling is not one
lesson's slip — it is the shape chapter 6 taught the ending in, and the reader
has carried it since sequence 320. Changing it changes what a learner was told.
Leaving it means the same ending is spelled two ways in one book. Neither is
free, and the choice belongs to a pass that enumerates every site rather than to
whichever chapter happens to notice.

**What chapter 102 did instead.** It quotes **ജോലിക്ക്** in **script only**, so
the two spellings never appear side by side on one page, and writes *maṇikkŭ*
for the new form. That is a dodge, deliberately, and it is recorded here so the
dodge does not read as a decision.

**The scope is wider than the final vowel, and the first draft of this shard got
that wrong too.** It scoped the census to *u* against *ŭ*. Review found two
divergences it would not have seen, both in the two-word phrase chapter 102 is
built on:

| word | one spelling | the other |
|---|---|---|
| **മണി** | *maṇi* — `ML-C18-mani` **headword** | *mani* — `ML-C18-mani.md:73`, **its own body** |
| **രണ്ട്** | *raṇṭŭ* — `ML-C93-counted` | *randu* — `ML-W07-number-words-1-5.md:49` |

The differences are in **ṇ/n** and **ṭ/d**, invisible to a census keyed on the
final vowel — and the first is *inside one lesson*, between its headword and its
own prose. Scoping the census to the chandrakkala would have reproduced, one
level up, the exact failure this shard was filed to avoid.

**What closing this needs.** An enumeration of every Malayalam romanization in
the corpus — headwords **and** bodies — grouped by the Malayalam token they
render, listing every distinct spelling of each. Not a grep for one spelling;
a grouping that makes disagreement visible without knowing in advance which
letters disagree. Then one convention, applied everywhere, with the early
lessons' text updated rather than left as an exception. Expect a large
mechanical diff and a small number of decisions.

**Note on the neighbouring divergence.** The same early chapters also use **zh**
where later ones use **ḻ** and **th** where later ones use **t** — chapter 10's
*thiṅkaḷ* and *āzhcha* against chapter 33's *eḻutuka* and chapter 95's
*tuṟakkuka*. Chapter 102 copies chapter 10's spelling for **തിങ്കളാഴ്ച**, on the
ground that a learner meets the word in that table first. Whether that is one
problem with this one or three separate ones is part of what the census has to
answer; do not assume they resolve together.

### Closure

They were one problem: the older lessons mixed an English-friendly spelling
with the later book's letter-by-letter spelling. A census grouped every
Malayalam-script headword declaration and every adjacent native/italic pair in
the lesson bodies by the native token, without searching for any anticipated
Latin spelling. It exposed the named collisions and their neighbours together:
**പേര്** as *pēru/pēr*, **മണി** as *mani/maṇi*, **ചായ** as
*chaaya/chāya/cāya*, and **തിങ്കളാഴ്ച** as
*thiṅkaḷāzhcha/tiṅkaḷāḻca*, as well as the suffix collision that opened this
shard.

The corpus now follows the later letter-by-letter convention throughout those
recurring families:

- word-final chandrakkala carries **ŭ**: *-ikkŭ*, *enikkŭ*, *pattŭ*;
- anusvara carries **ṁ**: *namaskāraṁ*, *santōṣaṁ*, *malayāḷaṁ*;
- retroflex and palatal letters keep their marks: *maṇi*, *raṇṭŭ*, *neñcŭ*;
- **ച / ഴ / ത** are **c / ḻ / t** unless the script really contains an
  aspirated consonant: *cāya*, *āḻca*, *tala*, but *katha* remains *katha*.

The dative lesson no longer teaches the exception: **-ിക്ക്** is *-ikkŭ*,
**-ിന്** is *-inŭ*, **ജോലിക്ക്** is *jōlikkŭ*, and chapter 102's
*maṇikkŭ* now visibly follows the same rule. Cross-language comparisons retain
their own spelling — the Telugu cousin of **പേര്** remains *pēru*, for example — so
normalising Malayalam did not silently rewrite its neighbours.

The corpus test pins fourteen representatives across the opening, number,
calendar, body, drink, question and nominalisation chapters, including all
three concrete collisions that filed this shard. Regenerated book and narration
artifacts carry the same spellings, so the reader no longer meets one form in a
lesson and another in its audio or chapter output.
