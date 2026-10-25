## Unreleased — Urdu's six recognition-only letters are pinned in the glyph evidence

- **Tests:** `tests/glyph-evidence/080-urdu.evidence.ts` gains a case for the
  six rows the Urdu B1 core (chapters 264-275) adds to `urdu-nastaliq.d`:
  ث ڈ ذ ص ع غ. Each is pinned with its sound and role, an empty
  `strokeOrder`, no `strokeOrderSource` and no `penLifts`, and a note that
  no Urdu-scoped stroke-order source has been verified yet; the inventory now
  holds 37 letters. The rows are recognition-only, so no ductus path or
  filmstrip changes, and a Persian or Arabic row's order is never borrowed.
