## German joins the cross-language core verbs (2026-08-07)

- Retagged **six** verb lessons from language-local ids to the canonical
  concepts owned by `SPINE-SAY-WHAT-I-DO`, so German's verbs finally join the
  cross-language corpus instead of being seven private ids no other track can
  see: `GE-C16-sein` → `VERB-BE`, `GE-C14-haben` → `VERB-HAVE`,
  `GE-C03-gehen` → `VERB-GO`, `GE-C05-machen` → `VERB-DO-MAKE`,
  `GE-C05-lernen` → `VERB-LEARN`, `GE-C05-wohnen` → `VERB-LIVE`
  (*wohnen* is taught as "to live / to dwell", which is exactly `VERB-LIVE`).
  German's core-verb coverage goes **0/40 → 6/40**, and `VERB-DO-MAKE` and
  `VERB-LEARN` leave the corpus-wide `universallyMissing` list (29 → 27) —
  German is the first track anywhere to realize either.
- `GE-C02-heissen` keeps its namespaced `DE-VERB-HEISSEN`. No core concept
  means "to be called": *heißen* is not a translation of anything on the
  shared list, and forcing it onto one would be a false join.
- **Rewired `curriculum.json` so the realization path matches the retag.** A
  canonical concept obliges its lesson to sit in the segment of the node that
  owns it, so the four verb tranches moved into their own
  `SPINE-SAY-WHAT-I-DO` segments — `GE-PATH-011` (*gehen*), `GE-PATH-014`
  (*wohnen*, *machen*, *lernen*), `GE-PATH-018` (*haben*), `GE-PATH-021`
  (*sein*) — and left `SPINE-CHECK-WELLBEING` and `SPINE-TIME-OF-DAY`, where
  they had been sitting as language-specific extension material.
- **Teaching order is untouched.** No chapter was reordered and no lesson
  renumbered: each moved lesson holds the exact position it already had, and
  the segments were split around it rather than resequenced. *gehen* still
  lands immediately before *wie geht es*, which needs it.
- Three orphan lessons entered the path because the retag required it.
  `GE-C05-lernen` and `GE-C16-sein` realize canonical concepts and so cannot
  be absent from it; *sein* in turn declares `GE-C15-praeteritum` as a
  prerequisite (its *war/waren* forms are Präteritum), which pulled
  `GE-C15-perfekt` and `GE-C15-praeteritum` in behind it. Those two are a
  German-local past, not `VERB-PAST`, so they are recorded as a
  `SPINE-TALK-ABOUT-PAST` segment (`GE-PATH-020`) whose omission ledger still
  names `VERB-PAST` as undelivered. The node is now realized without the
  debt being quietly written off.
- Path segments were renumbered `GE-PATH-001..026` and extensions renamed to
  match their host segment, keeping the ids monotonic in path order as every
  other track has them. `GE-EXT-011-LANGUAGE-SPECIFIC` was deleted outright:
  it existed only to classify *gehen* as local support, and *gehen* is now
  shared content.
- Derived levels move accordingly: German reaches **A2** for the first time,
  corpus `A2` 91 → 99, `A1` 307 → 304, `pre-A1` 657 → 656, unmapped 170 → 166.

