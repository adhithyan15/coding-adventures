---
category: Repo policy / workflow reminders
---

# Write the rule down, then run it: the sibling check failed on the chapter after I wrote it

Chapter 466's review produced a rule and I recorded it in the tranche plan:

> Citing a rule owner is not enough — read what sits **beside** it. Citing
> `ES-C421-actor` for the `-tor` rule missed `ES-C421-actriz` next to it,
> which holds the counterexample.

Chapter 467's pre-check then cited `ES-C345-objeto` as the owner of the
*iacere* family, and did not open `ES-C345-repaso-persona-objeto` sitting
beside it. That file says:

> "the **object** of a verb is exactly what the word says — **the thing the
> action is thrown at**."

The new lesson had built its whole contrast on *objeto* meaning "thrown
before you" and *objetivo* meaning "the thing you throw at" — a distinction
the sibling had already assigned the other way, and which is false anyway
(*obiectivus* is an adjective formed *on* *obiectum*; `-ivo` names a habit,
it does not reverse a direction). The lesson's own closing line disproved
it: an **objective lens** is named for facing the *object*.

The plan file even listed the sibling check under "Sibling rule, new from
466", naming `ES-C345-*` explicitly as files to read. It was written and not
executed.

So the failure is not that the rule was missing. It is that a rule living in
a plan document is a note, not a step. The greps in this workflow get run
because they are commands with output; the sibling read got skipped because
it was prose.

What follows:

- The sibling check has to produce **output**, like the hit-count grep does.
  `ls ES-C<owner-chapter>-*.md` and then actually opening each one, with the
  list pasted into the plan, not a sentence promising to.
- A rule recorded in the same session that discovered it has not yet been
  tested. Treat its first application as the risky one, not the safe one.
- Two independent signals disagreeing is the cheapest check available. Here
  the lesson's own last paragraph contradicted its main claim, and nothing
  in the writing process compared them.
