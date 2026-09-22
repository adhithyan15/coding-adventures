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

**The rule.** A citation is a claim that you have read the lesson. Reading the
lesson means reading its chapter. So: collect every lesson id you cite, link
to, or name in prose, including the ones reached through `prerequisites:`; for
each, take the chapter number from its `chapter:` field, never off the id; run
`ls ES-C<ch>-*.md`; and open every file in that listing, pasting the list into
the plan file. That last step is the part that costs time and the part that
pays. The cost scales with how many chapters you cite, which is a reason to
cite fewer and more deliberately, not a reason to skip the check.

**Why a headword grep cannot replace it.** A grep answers "has this word been
used before". It cannot answer "has this *rule* been taught before", because a
rule is owned by a lesson whose headword may share nothing with yours. The
`-dora` gender rule is owned by a lesson about a water meter. Rules travel by
chapter, not by vocabulary.

The chapter after this one confirmed it twice more before a line was written.
`ES-C330-parada` turned out to own the whole `para-` family — *parasol*,
*parachute*, and the Romance-versus-Greek `para-` distinction — and to derive
`para-` from *parare*, "to make ready against". The planned gloss for
`paraguas` was *para* ("stops") plus *aguas*, which would have contradicted a
merged lesson outright. The same listing turned up `ES-C383-dejar` beside
`buscar`, which improved the chapter rather than only correcting it.
