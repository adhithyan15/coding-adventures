# Paired atoms drift apart: the reviews declared one twin and never the other (human-language-data)

Spanish's A1 `reinforcement` blocker stood at 83 atoms "revisited fewer than twice."
Twenty-three of them were being revisited on the page and were never declared.

Every object-pronoun lesson introduces a **pair** — `ES-C42-lo` introduces
`ES-LEX-LO-OBJECT` *and* `ES-GRAMMAR-DIRECT-OBJECT-LO`, and *la*, *te*, *nos*, *os*,
*le* do the same. Every downstream review then declares the `ES-GRAMMAR-*` twin and
none of them declares the `ES-LEX-*` twin, while printing the word itself in its own
recap table:

```
| it (masculine) | **lo** | *lo tengo* — I have it |
| you            | **te** | *te quiero* — I love you |
```

The word is on the page. The ledger said it had not been seen since the chapter that
taught it. Closing those cost **no new prose at all** — only
`practises.knowledge` plus the body block's `assesses`, which is what `curriculum.ts`
already requires them to agree on.

This is the `roots: []` lesson one field over, and the general form is worth stating:
**when a lesson introduces two atoms for one teaching moment, nothing keeps later
lessons citing both.** The one that names the *rule* gets cited, because reviews are
written about rules; the one that names the *word* gets dropped. Any `X-LEX-*` /
`X-GRAMMAR-*` pair in any track is a candidate. Diff the two twins' revisit counts.

**Reject about a third of what the screen proposes, and expect to.** A prose match is
a prompt to go read the line, not a verdict. *ocho* in "Tengo ocho años" does not
revisit ROMAN-MONTH-NAMES; *libro* in "Un libro" does not revisit ORDINAL-APOCOPE;
*grande* in "muy grande" does not revisit the exclamative `¡Qué grande!`. Each of
those would have "closed" an atom and taught nothing — **the detector matching rather
than the teaching**. A wiring pass that accepts every hit is indistinguishable from
one that edits the frontmatter at random and is worse than leaving the atom open,
because it retires the atom from the report.

**Wiring has a hard ceiling, and it is the prerequisite closure.** A practised atom
must already be in the carrier's transitive `prerequisites` closure
(`schema-v2-practice-before-introduction`), and Spanish's chains break at chapter
boundaries — `ES-C346-escuela` reaches back to `ES-C334-edad`, skipping 345 entirely.
So "some later lesson says the word" is not sufficient; check closure first, or the
validator rejects the edit.

**Corollary for reinforcement specifically:** the windows are only judged where the
track is long enough to contain them, so the last lessons are never measured. Measure
that tail explicitly rather than reasoning about it — Spanish's came to exactly two
atoms, both in the final chapter, and both become blockers the moment anything lands
after them.
