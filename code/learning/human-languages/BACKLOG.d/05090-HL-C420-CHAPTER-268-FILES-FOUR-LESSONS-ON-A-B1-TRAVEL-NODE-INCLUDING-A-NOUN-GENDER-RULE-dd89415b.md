## HL-C420-dd89415b — Chapter 268 files four lessons on a B1 travel node, including a noun-gender rule

**Status: OPEN.** Found while checking why `problema` is still counted missing
by the A2 book-bounded audit after chapter 469, by reading the lesson rather
than trusting the earlier entry.

**This corrects HL-C418.** That entry groups `explicar`, `creer` and `problema`
together as three lexemes riding on `SPINE-GIVE-REASONS` via `ES-PATH-037`.
That is right for `explicar` and `creer`. It is **wrong for `problema`**:

```
ES-C268-problema   spine_node: SPINE-HANDLE-TRAVEL   sequence: 2650
```

Different node, different cause, and a much easier question to answer.

### The node's stage is correct; the lessons on it are not

`core/spine.d/0250-SPINE-HANDLE-TRAVEL.json` reads:

> "I can deal with most situations that come up while travelling."

That is the CEFR **B1** travel descriptor almost word for word — *"Can deal
with most situations likely to arise whilst travelling in an area where the
language is spoken."* The node is staged **B1** and it is staged correctly.
Nothing here argues for moving it.

What is wrong is which lessons were filed on it. Only **four lessons in the
entire 23-track corpus** name this node, and all four are Spanish chapter 268:

| lesson | seq | what it actually teaches |
|---|---|---|
| ES-C268-donde-esta | 2620 | *¿dónde está?* |
| ES-C268-billete | 2630 | *billete*, a ticket |
| ES-C268-problema | 2650 | *problema*, and `-ma` nouns being masculine |

`ES-C268-repaso-dos-finales-dos-generos` (2655) is the fourth, reviewing the
gender rule.

### The chapter contradicts itself, ten sequence steps apart

Chapter 268's six lessons alternate between two nodes two CEFR levels apart:

| lesson | seq | node | stage |
|---|---|---|---|
| ES-C268-habitacion | 2640 | SPINE-NAME-EVERYDAY-THINGS | A1 |
| ES-C268-problema | 2650 | SPINE-HANDLE-TRAVEL | B1 |

`ES-C268-problema`'s own *"You'll want to know first"* block links to
`habitación` as *"the gender rule it followed."* So two lessons, ten sequence
steps apart, teaching the same grammatical topic, are separated by two levels
of the ladder. One of the two placements has to be wrong, and it is not the
one on the everyday-things node.

`ES-C268-problema` introduces `ES-GRAMMAR-MASCULINE-MA-NOUNS` and says in its
own text that the rule is *"worth more than the word itself"*: Greek `-ma`
nouns are masculine in Spanish — *el problema*, *el idioma*, *el clima*.
Whether a learner can deal with most situations arising while travelling is a
real B1 question. Whether *el problema* is masculine is not that question at
all.

`¿dónde está?` is the same shape of error and more obvious still:
`SPINE-ASK-LOCATION` exists, is staged **A1**, and is already a declared
prerequisite of `SPINE-HANDLE-TRAVEL`.

### Why this matters beyond one word

The ladder is what makes the book-bounded audits mean anything. A lesson on
the wrong node is not a mislabelled file — it removes real teaching from the
level that teaches it and hands it to a level that does not. A reader who has
finished the A2 book has, on the current wiring, genuinely not met *problema*,
although it is taught at sequence 2650 of roughly 12,900.

### Proposed fix, not yet taken

Re-file the four lessons on nodes that match what they teach:

| lesson | proposed node |
|---|---|
| ES-C268-donde-esta | SPINE-ASK-LOCATION (A1) |
| ES-C268-billete | SPINE-NAME-EVERYDAY-THINGS (A1) |
| ES-C268-problema | SPINE-NAME-EVERYDAY-THINGS (A1) |

with the gender repaso following `problema`, matching `habitación` and
`ES-C268-repaso-llegar-y-quedarse`, which are already there.

`SPINE-HANDLE-TRAVEL` would then hold no lessons, which is honest: the corpus
does not yet teach the B1 travel capability, and pretending otherwise by
parking A1 vocabulary there helps nobody.

**Deliberately not done as part of a vocabulary chapter.** This re-stages
already-merged content and changes what the A1 and A2 books contain, so it
wants its own branch and its own review. Recorded here so the reasoning is not
re-derived a third time.

**Do not** teach `problema` again. HL-C418's warning holds: a second lesson
for a word the corpus already has is duplication the reader meets twice, and
it would move the gap number for a reason that is not progress.
