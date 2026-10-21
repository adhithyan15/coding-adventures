---
category: Repo policy / workflow reminders
---

# A core's last grammar lesson needs later words that revisit it, not only its letters

The Telugu B1 plan guarded the five new letters against the level gate's
"every atom at or below the level is revisited at least twice" rule: each
letter was revisited inside the core and again in chapters 340-344. The core
alone measured 0 thin atoms. But once the 1,120-word vocabulary was generated,
the gate still held Telugu at A2 with one thin atom:
TE-GRAMMAR-C325-DEPENDS-01 ("… మీద ఆధారపడి ఉంటుంది", it depends on), the core's
last grammar lesson. Its only later practice was the chapter review; the
writing task did not use it, and no vocabulary word did. The core-alone
measurement did not flag it; it surfaced only once the tranches existed, so a
0-thin result on the core alone is not enough.

The fix: two notes in chapters 326 and 327 use … మీద ఆధారపడి ఉంటుంది and list
the atom in `extra_atoms` with its lesson in `extra_prereq`.

What to do differently:
- Treat every atom introduced in a core's last two chapters (grammar and
  skills, not just letters) as needing planned revisits in the first
  vocabulary tranche.
- Measure `revisits < 2` over the WHOLE track after generating the tranches
  (`revisits.mjs <dist> 0`), not only on the core.
