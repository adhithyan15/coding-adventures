---
category: Repo policy / workflow reminders
---

# A placeholder that looks invented is not a placeholder, and the rule applies to institutions as well as domains

Spanish chapter 429 ends on a synthesis lesson that reads a Spanish report card.
To make it look like a real document it carried a letterhead:

```
IES Miguel Hernández
Boletín de calificaciones
```

The name was chosen the way a novelist chooses one — a famous poet, the sort of
person Spanish schools are named after, plainly not anybody's data. The security
review found that **IES Miguel Hernández is the exact name of several real,
currently operating public secondary schools** — in Alicante, in Móstoles, in
Alhama de Murcia, among others. The lesson was printing a fabricated official
document under a real school's letterhead.

Nothing personal was exposed: no student name, no identifier, and marks that are
obviously invented, which is why the finding was LOW. The defect is the
reasoning, not the blast radius.

**It is the `ejemplo.es` mistake again, one tranche later.** That one is already
recorded in `a-placeholder-domain-in-teaching-material-must-be-a-reserved-one-not.md`:
a domain meaning "example" in the target language is not a reserved domain.
Here the same inference ran on a different kind of name — *this looks made up,
therefore it is safe* — and was wrong for the same reason. Looking invented is a
property of how a name reads; being a placeholder is a property of a convention
that guarantees nobody holds it.

Domains have RFCs for this. **Institutions do not**, so the standard has to be
that the placeholder cannot resolve to a real body: `IES Ejemplo` rather than any
plausible-sounding name, and the lesson says out loud that the name is a
stand-in. That last part is worth doing rather than skipping, because the
teaching improves — the reader is told what varies on a real document and what
does not.

Two things to carry forward.

**The check is "could this name belong to somebody", not "did I invent it."**
When learner-facing material needs a named school, company, hospital, street,
newspaper or clinic, search the name before shipping it. A name invented in good
faith from the local naming conventions is *more* likely to collide with a real
one, not less, because that is what the conventions are for.

**The first fix reintroduced the bug.** The replacement prose originally read
*"a real letterhead names the centre — IES Vicente Aleixandre, IES Al-Ándalus,
whatever the school is called."* Both of those are real schools too. Naming
examples of the class you are trying to avoid naming is the same error at one
remove; the wording that shipped describes the pattern instead ("usually a poet,
a scientist or a place") and names nothing.
