## Joined the cross-language verb corpus

- Retagged six verb lessons from language-local `IT-VERB-*` ids to the canonical
  `VERB-*` concepts, so Italian's verbs now join the cross-language corpus
  instead of being unrelated to every other track's:
  `IT-VERB-BE` → `VERB-BE` (essere), `IT-VERB-HAVE` → `VERB-HAVE` (avere),
  `IT-VERB-ANDARE` → `VERB-GO`, `IT-VERB-PARLARE` → `VERB-SPEAK`,
  `IT-VERB-ABITARE` → `VERB-LIVE`, `IT-VERB-LAVORARE` → `VERB-WORK`.
- `IT-VERB-STARE` stays namespaced on purpose. *stare* is the other half of the
  essere/stare pair and `essere` already takes `VERB-BE`; a second lesson on the
  same canonical concept is a duplicate realisation, and no core concept covers
  the state-of-being verb on its own.
- Rewired `curriculum.json` for the realisation rule that a canonical concept
  owned by `SPINE-SAY-WHAT-I-DO` brings with it, using two different mechanisms
  because the six lessons are not one case:
  - **Chapter 5 moved.** `IT-PATH-011` is split in place into a
    `SPINE-SAY-WHAT-I-DO` segment (parlare, abitare, lavorare) and a
    `SPINE-POLITE-REQUEST-REPAIR` segment (parlo-italiano, practice), and the
    three verbs' authored `spine_node` pins are corrected to match. Chapter 5 is
    titled "The First Verbs" and teaches the regular `-are` present; its pin to
    `SPINE-POLITE-REQUEST-REPAIR` was a placeholder from before the A2 verb
    tranche existed, when a present-tense lesson had no node it could legally
    declare. Nothing moves relative to anything else — the split is where the
    chapter already changed subject.
  - **Chapters 14 and 16 stayed put, recorded as `relocates`.** avere is taught
    in "Having and Telling Your Age" to build *ho vent'anni*, and essere/andare
    in the location-and-past chapter; those pins are right, and forcing the three
    into a "what I do (present)" segment would have split two coherent chapter
    units and filed past-tense material under the present. `relocates` records
    where the concept is realised without falsifying the authored pin.
- `VERB-BE`, `VERB-HAVE`, `VERB-GO`, `VERB-SPEAK`, `VERB-LIVE` and `VERB-WORK`
  leave `SPINE-SAY-WHAT-I-DO`'s `omits`; every node's `segments` ledger is
  refreshed against the authored path. `IT-EXT-011-LANGUAGE-SPECIFIC` keeps only
  parlo-italiano — the three verbs are shared realisations now, not
  language-specific support. Path and extension ids are renumbered to stay
  ascending in path order, the convention every other track follows.
- `chapters.json` chapter 5 now declares both nodes it realises. Chapters 14 and
  16 needed no change, a direct consequence of the `relocates` choice.
- No chapter reordered, no lesson renumbered, no lesson prose touched — only
  frontmatter `concept_tag` and three `spine_node` pins.

