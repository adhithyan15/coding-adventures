---
category: Repo policy / workflow reminders
---

# The sibling check has to run on every lesson you cite, not only on your own chapter

The rule as it stood was: before writing chapter N, run `ls ES-C<N>-*.md` for
the chapters you are about to lean on, open each file, and paste the list so
the check produces output. That rule held for three chapters and then let a
duplication straight through.

Chapter 469 taught `la impresora`. The headword greps found
`ES-C432-lavadora`, which already owns `-dora` for machines, complete with the
`secadora` / `calculadora` table the draft had rebuilt from scratch. Opening
it was enough to catch that one.

What it was not enough to catch was **`ES-C432-contador`**, one file along in
the same chapter. Contador already owns the very claim the draft was staging
as its Grammar Lens:

> Note the gender: this one is masculine where the washing machine was
> feminine, and nothing about the ending decides that.

No headword in chapter 469 is anywhere near the word *contador*. No grep of
*imprimir*, *impresora*, *conectar* or *llamada* would ever surface it. It was
found only because `ls ES-C432-*.md` was run **on a chapter that was cited**,
rather than on the chapter being written.

The same pass turned up a second one in the same shape: the `conectar` draft
explained `el ordenador`'s masculine gender as a dropped Spanish noun, which
contradicts `ES-C393-ordenador` — the direct prerequisite — where the
computing sense is a loan from French *ordinateur*.

## The rule

A citation is a claim that you have read the lesson. Reading the lesson means
reading its **chapter**. So:

1. Collect every lesson id you cite, link to, or name in prose — including the
   ones you reach through `prerequisites:`.
2. For each, take the chapter number **from its `chapter:` field**, and run
   `ls ES-C<ch>-*.md`.
3. Open every file in that listing and paste the list into the plan file.

Step 3 is the part that costs time and the part that pays. The cost scales
with how many chapters you cite, which is a reason to cite fewer and more
deliberately, not a reason to skip the check.

## Why a headword grep cannot replace it

A grep answers "has this word been used before". It cannot answer "has this
*rule* been taught before", because a rule is owned by a lesson whose headword
may share nothing with yours. `-dora` gender is owned by a lesson about a
water meter. Rules travel by chapter, not by vocabulary.
