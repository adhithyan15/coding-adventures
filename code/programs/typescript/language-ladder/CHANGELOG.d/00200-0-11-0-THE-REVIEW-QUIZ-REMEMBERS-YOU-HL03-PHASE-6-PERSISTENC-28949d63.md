## 0.11.0 — the review quiz remembers you (HL03 phase 6, persistence)

- **The Learn-mode review quiz now persists between visits.** New
  `src/reviewstore.ts` saves the review `Progress` (per-cell Leitner state + the
  answer log) and the SRS session clock to `localStorage` after every answer,
  and restores them at startup — so promotions, demotions, and logged confusions
  survive a reload. Mirrors `progress.ts` (which does this for the lesson
  schedule): the engine stays pure, all (de)serialization is pure and
  unit-tested, and the untrusted blob is validated field-by-field — a corrupt,
  wrong-version, or wrong-shaped payload restores as **empty rather than
  throwing** (a study app that won't start over one bad key is worse than lost
  progress). States are stored as `[cellKey, QuizState]` entry pairs, sidestepping
  the `__proto__`/key-escaping hazards of an object map.
- 9 new tests (194 total), including controls that bite: strip the version gate
  and a stale blob surfaces; drop the `getItem` guard and a throwing storage
  breaks startup. Verified in a real browser — seeding a saved review and
  reloading restores "1 answered" (fresh is "0").
- **Slice 6d (retire the standalone artifacts) was a no-op:** the HL03 spec's
  "script field-guide, spot-the-script quiz, letter-reading trainer" were only
  ever ephemeral exploratory Artifacts, never committed to the repo (no matching
  files or history), and their capabilities already live in Browse / Practice /
  Concepts. Nothing to remove — so this slice does the persistence work instead.

