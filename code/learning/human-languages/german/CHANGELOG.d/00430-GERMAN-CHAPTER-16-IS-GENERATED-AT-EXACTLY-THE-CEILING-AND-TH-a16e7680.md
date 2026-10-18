## German chapter 16 is generated at exactly the ceiling, and the .tex had named a word it never taught

`ch16-family.tex` is now generated as **chapter 16, *The Pair That Proves
Grimm's Law***. German's hand-written chapters: **4 -> 3**. **No renumber** —
one chapter in, one chapter out, and the thirteen lessons fit the thirteen free
sequence slots between chapter 15 and chapter 17.

### Sizing it four ways

| instrument | answer |
|---|---|
| `handwritten_parity.py` | gap of **7** (sounds ×2, grammarlens ×1, culture ×2, etymology ×1, morphologybox ×1) |
| `grep -l '^chapter: 16$'` | **2 lessons** |
| the `.tex` | **2 sections**, two three-column tables, 2 cousinweb, 2 culture, 1 grammarlens, 1 etymology, 1 morphologybox |
| taught-form census | *Vater*, *Mutter*, *Bruder*, *Schwester*, *Geschwister* — and ***Eltern***, which the `.tex` names a section after and never teaches |

**12 atoms against a ceiling of 12, so one chapter and no split.** The rule is
the ceiling, not the median.

It fits at twelve for two separate reasons, and both are worth stating because
neither is luck.

**Three things it argues are already owned.** `GE-GRAMMAR-SOUND-LAW-01` from
chapter 6 is Grimm's law itself; `GE-SOUND-V-AS-F-01` from chapter 7 is the *V*
of *Vater*, given on *vier*; `GE-GRAMMAR-NATIVE-VS-LOAN-01` from chapter 9 is
"the months were bought and the family was not." All three are **spent**. Without
them the chapter is fifteen and splits.

**The culture claim rides as a culture claim.** `GE-CULTURE-FAMILIE-GERMANISCH-01`
is declared only in `introduces_culture_claims`, exactly as
`GE-CULTURE-DREI-SCHICHTEN-01` is in chapter 12, so it does not consume a
thirteenth atom slot.

### *die Eltern* — a word the chapter was named after and never taught

The `.tex` labels its first section `lesson:eltern` and then teaches only *Vater*
and *Mutter*. German's own README already advertised the chapter as "Eltern,
Geschwister". So the word was claimed in two places, used as a section name, and
taught nowhere — the fourth chapter in a row to turn up one of these.

It is taught now, with what is underneath it: *Eltern* is a **comparative frozen
into a noun**, "the older ones," and English made the same word into *elders*
and narrowed it. Its sibling collective *Geschwister* is built the other way, by
gathering with **Ge-** — so the chapter closes on two plural-only nouns made from
German's own parts by two different machines.

### One law, four verdicts

The `.tex` shows Grimm's law on *Vater*/*pater* and *Mutter*/*mater* and then
lists *Bruder* and *Schwester* as further cousins. The lessons make the four a
single argument, because the interesting one is the third:

| pair | what moved |
|---|---|
| *Vater* / *pater* | **p → f** |
| *Mutter* / *mater* | **t → th**, then German's own later shift again |
| *Bruder* / *frater* | the swap runs the **other way** |
| *Schwester* / *soror* | **nothing** — the law does not touch an *s* |

*Bruder* looks like a contradiction of *Vater* and is not: the two began from
different ancient sounds, a plain *p* and a breathy *b*, and each moved once in
its own direction. That is what makes it a law you can run forwards on a word
you have never seen, rather than a preference for *f*.

### One finding no gate had shown me before

`GE-C10-schwester` closed with "Next: the third language in the room," meaning
Latin. `standalone-book.test.ts` refuses **`the <ordinal> language`** anywhere in
a shipped book, because it is an ordinal over a set the reader is not holding and
cannot check. The guard is deliberately blunt and the phrase was rewritten rather
than the guard weakened.

### Counters, re-measured against the merged tree

| counter | before | after |
|---|---|---|
| German hand-written chapters | 4 | **3** |
| `handwritten_parity.py` german | 29 | **22** blocks at risk |
| German lessons (schema-v2) | 249 | **262** |
| atoms taught | 421 | **433** |
| atom-measurement-blind lessons | 8 | **6** |
| chapters over the 12-atom budget | 5 | **5** |
| culture claims | 25 | **26** |
| atoms never revisited | 81 | **81** |
| forward references | 31 | **29** |

Forward references fall by two because the schema-v1 `GE-C10-eltern` named
*Bruder* and *Schwester* before either was taught. Both are taught in this
chapter now.

### Bundle bands

Chapter 16's lessons keep `GE-C10-` ids, which land in ladder band C9. The gate
is unchanged at **591 batches over 591 bands, zero splits, 191 kB largest** —
the width-3 narrowing in the previous PR had the room, as measured.

