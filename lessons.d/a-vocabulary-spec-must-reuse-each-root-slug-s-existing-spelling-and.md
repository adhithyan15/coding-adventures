---
category: Repo policy / workflow reminders
---

# A vocabulary spec must reuse each root slug's existing spelling and screen headwords for hand verbs

**What went wrong.** The French B1 vocabulary (chapters 268-531) passed the
forward-reference census and `cli validate`, but the full suite failed twice:

1. **Root-slug splits.** Three roots were written `camera-latin`,
   `colaphus-latin` and `respondere-latin`. The corpus already spells those
   etyma `latin-camera`, `latin-colaphus` and `latin-respondere`, so
   `root-slug-splits.test.ts` reported three new splits. Its debt pin may only
   fall. The brief said "convention `<lemma>-latin`", but that convention is not
   universal: older tracks used `latin-<lemma>` for some etyma.
2. **A hand verb in a heading.** The headword *le point de vue* puts "point" in
   the narrated heading "You'll want to know: le point de vue". The plain scan
   in `drivable-instructions.test.ts` treats "point" as a hand verb whatever
   language it is in.

**Fix.** Rename the three slugs to the existing spelling. Add the heading to
`SPOKEN_HAND_VERB_ALLOWLIST` under the "word used as a noun" reason, since the
headword cannot be reworded.

**Next time.** Before generating a tranche:
- grep each `roots:` slug, and its swapped form `latin-x` / `x-latin`, across
  every track, and use whichever already exists;
- grep the headwords for the hand verbs in `PLAIN_HAND_VERB` (point, note down,
  record, trace, copy, circle, tick, tap, clap, select and the rest), and
  allowlist or replace each hit.
