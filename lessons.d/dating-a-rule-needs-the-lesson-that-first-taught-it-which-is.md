---
category: Repo policy / workflow reminders
---

# Dating a rule needs the lesson that first taught it, which is almost never in prerequisites

Three tranches running, every authoring error in the Spanish corpus was
found by reading the files in a new lesson's own `prerequisites:` list, so
that became the bounded rule: read the three to five files the lesson
names, not the thirteen hundred it doesn't.

Tranche 4d broke the rule's back. Its worst errors were in files the
prerequisite list does not reach:

- `ES-C464-recordar` dated the `o → ue` stem break to *la rueda*, chapter
  449. It is taught at chapter **11**, in a lesson whose title is the rule,
  and applied to this very root at 284. Neither file is a prerequisite.
- `ES-C464-dentista` re-taught the `-ista` rule as new, with the same three
  examples. Chapter **9** teaches it and uses *dentista* as its worked
  example. Not a prerequisite.
- `ES-C463-importar` filed *puerta* and *pasaporte* under *portare*.
  `ES-C279-puerta` separates *porta* from *portare* explicitly. Not a
  prerequisite.

The pattern is consistent and it is about the *kind* of sentence. A claim
about **this word** is checkable from the prerequisite closure, because
that is what the closure is for. A claim about **a rule** — where it was
introduced, how many times you have seen it, whether it has exceptions, who
else it applies to — belongs to whatever lesson owns the rule, and rule
lessons sit early, far outside any later lesson's closure.

So the check is now two-part. Prerequisites for word claims, as before. And
for every sentence of the form "the same X you have been tracking since Y",
"this is the Nth Z", "you now have all of them", or "which is how most W
are built", grep the corpus for where X, Z or W is actually taught before
writing the date or the count. `grep -l` on the rule's name across
`lessons/` costs one command and is the only thing that catches these.
