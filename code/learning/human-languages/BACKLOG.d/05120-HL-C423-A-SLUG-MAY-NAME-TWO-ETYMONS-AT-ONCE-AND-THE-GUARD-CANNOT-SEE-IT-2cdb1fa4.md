## HL-C423-2cdb1fa4 — A slug may name two etymons at once, and the guard cannot see it

**Status: OPEN.** Found while backfilling `ES-C283-amable`'s empty `roots:`,
immediately after the HL-C419 shape normalisation landed.

`check:root-slug-splits` groups slugs by `(lemma, tag)`. Two slugs whose lemmas
differ are, to that guard, two different etymons — which is correct, and is why
HL-C419's entry records *prefix families* (`permittere-latin` against
`mittere-latin`) as out of scope. This is the same blind spot from the other
direction: **one slug whose lemma is itself two etymons.**

`PT-C25-amigo` carried `roots: [latin-amicus-amare]`. The slug is unique in the
corpus, so it joined **nothing** — Portuguese *amigo* had no cousins at all,
while its Italian sibling `IT-C23-amico-amica` writes the same fact as
`[amicus-latin, amare-latin]` and joined normally. The Portuguese lesson's own
hook says exactly what the Italian one says: *"amigo/amiga ← Latin
amicus/amica, built on amare"*. One track spelled a two-etymon fact as two
slugs and the other as one, and no gate could tell.

### The census, and why it is not a batch fix

Eighteen live slugs have a multi-part lemma where **every part is itself a live
lemma under the same tag**. They are not one defect. At least three kinds are
mixed together, and separating them needs each lesson read:

| kind | examples | verdict |
|---|---|---|
| a genuine compound etymon | `ecce-hoc` (*ça*), `ecce-iste` (*ce*), `de-mane` (*demain*), `medius-dies` (*mediodía*), `in-contra` (*encontrar*), `decem-et` (*dieciséis*) | **correct as one slug** — the compound IS the etymon |
| a derivation written as one slug | `amicus-amare`, `familia-famulus`, `comparare-parare` | **two slugs**, as the Italian sibling already writes |
| several etymons in a range or pair | `unus-duo-tres`, `sex-septem-octo`, `ekadasha-vimshati`, `magis-minus`, `ire-vadere`, `me-te`, `ad-maneana`, `per-ad`, `pro-per` | needs a decision — one lesson, several roots |

Only `amicus-amare` was fixed here, because its sibling lesson in another track
states the identical fact in the split form, so the corpus itself supplies the
answer. Nothing else has that control, and guessing would assert an etymology
rather than reveal one — the failure `cousins.ts` exists to prevent.

### Why a guard is hard here, stated honestly

HL-C419's guard works because a *shape* split is decidable from the slug alone.
This is not. `ecce-hoc` and `amicus-amare` are the same string shape and
opposite facts: one is a compound that existed as a unit in Latin, the other is
a word and the verb it was built from. A checker keyed on the slug **cannot**
separate them, exactly as the prefix-family case cannot be separated.

What could be checked cheaply: flag a compound-lemma slug that is **unique in
the corpus** while each of its parts is a live lemma elsewhere. That is the
`amicus-amare` signature — a slug that joins nothing while its pieces join
plenty — and it is a strictly weaker claim than "this is two etymons". It would
have caught this one. It would not catch a wrong compound that two lessons
share.

### Counting unit

**The unit is a SLUG**, not an etymon and not a lesson. Eighteen slugs; the
lessons carrying them number more, because `magis-minus` alone is on three.
