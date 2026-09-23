## HL-C420-dd89415b — Chapter 268 files four lessons on a B1 travel node, including a noun-gender rule

**Status: OPEN — attempted and reverted, see below.** Found while checking why `problema` is still counted missing
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

---

### ATTEMPTED AND REVERTED (measured, 2026-09-23)

**The proposed fix above is not four file edits. It is a change to what the
shared 23-language spine means by "handle travel", and it needs its own
argument.** Implemented exactly as proposed, verified, and reverted.

What the change actually achieves, measured rather than predicted:

```
A2  objectiveFailed  48 -> 47      mock 1 listening 13 -> 14
A1  objectiveFailed   0 ->  0      lessonCount 1022 -> 1026   <- the control
pre-A1 objectiveFailed 0 ->  0
```

One item, from `problema` alone, on mock 2 item 39 — exactly the magnitude
HL-C418's correction predicted. The A1 gate staying at 0 is the control proving
the four lessons can sit in the A1 book without breaking it.

**What the entry above missed.** `SPINE-HANDLE-TRAVEL` declares three concepts,
and they are precisely the `concept_tag`s of the three lessons being moved:

| concept | lesson |
|---|---|
| `DIRECTION-ASK` | `ES-C268-donde-esta` |
| `TRANSPORT-TICKET` | `ES-C268-billete` |
| `PROBLEM-REPORT` | `ES-C268-problema` |

The concepts are what bind the lessons to the node. Moving the lessons without
the concepts raises four errors — `curriculum-relocation-ledger-drift`, plus
`unclassified-curriculum-extension-lesson` for each of the three:

```
spanish: SPINE-HANDLE-TRAVEL relocation ledger does not match lesson metadata
spanish: ES-C268-donde-esta is local support but belongs to no extension node
spanish: ES-C268-billete   is local support but belongs to no extension node
spanish: ES-C268-problema  is local support but belongs to no extension node
```

`curriculum.ts:509` is the rule: a content lesson is shared content only when
`conceptOwner.get(concept) === placement.node`. Off its concept's node, it is
**local support**, and local support must hang off an extension. Which is the
right model — but it means there are only two honest routes, and both are
larger than a re-file:

1. **Move the concepts too.** Cheap in blast radius — measured, only Spanish
   has a lesson tagged with any of the three, and no other node declares them —
   but it leaves `SPINE-HANDLE-TRAVEL` with **zero concepts**, which reads as
   *vacuously satisfied*. That is dishonest in the opposite direction from the
   one this entry complains about: it would say the corpus handles B1 travel
   when nothing teaches it. It also asserts, for all 23 tracks, that asking
   directions belongs to `SPINE-ASK-LOCATION` (defensible) and that buying a
   ticket belongs to `SPINE-NAME-EVERYDAY-THINGS` (much weaker).
2. **Teach the validator that `relocates` blesses an off-node lesson.** The
   ledger's `relocates` map exists to record exactly "this node declares the
   concept, this track teaches it over there" — and `curriculum.ts:509` does
   not consult it. That looks like a real gap in the model rather than a rule
   working as intended, and closing it is a validation-semantics change, not a
   content one.

**Why neither was taken.** One exam item does not buy a change to the shared
spine's concept ownership or to validation semantics. And HL-C418 already
reached the same verdict on the same kind of question, about `explicar`: *"a
curriculum judgement about the ladder, not a mapping tidy-up, and it should be
argued on whether an A2 candidate is expected to give reasons at all, never on
the two exam items it would buy."* The identical sentence applies here, with
*handle travel* in place of *give reasons*.

**What stands from the entry above.** The diagnosis is still right, and the
strongest piece of it is untouched: `ES-C268-problema` and `ES-C268-habitacion`
teach the same gender rule ten sequence steps apart, separated by two levels,
with `problema`'s own text linking back to `habitación`. A reader who finishes
the A2 book has genuinely not met `problema`. That is a real defect; it is just
not a cheap one.

**Method note.** The entry proposed a fix without checking what bound the
lessons to the node it wanted them moved off. The fix was written from the
lesson table — which is what `spine_node` a lesson names — and not from the
spine node, which is what `concepts` it owns. One `cat` of
`core/spine.d/0250-SPINE-HANDLE-TRAVEL.json` was the difference, and it is the
second time in this pair of entries that a recommendation came from a summary
rather than from the file it was about.
