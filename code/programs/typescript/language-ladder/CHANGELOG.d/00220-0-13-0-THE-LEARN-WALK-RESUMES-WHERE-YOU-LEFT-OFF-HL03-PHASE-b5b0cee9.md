## 0.13.0 — the Learn walk resumes where you left off (HL03 phase 7b)

- **The teaching cursor now persists.** Review progress and the lesson schedule
  already survived reloads; the Learn cursor didn't — walk to concept 40, close
  the tab, and you were dumped back at "thanks". New `src/cursorstore.ts` saves
  the concept index to `localStorage` on every Prev/Next and restores it at
  startup, so the app resumes exactly where you were. The restored index is
  **clamped to the current spine** (the curriculum grows and shrinks), and a
  corrupt / wrong-version / out-of-range blob falls back to concept 0 rather than
  throwing or pointing off the end.
- 11 new tests (219 total) with controls that bite: strip the version gate and a
  stale blob resurfaces; drop the `getItem` guard and a throwing storage breaks
  startup; a saved index past a now-shorter spine clamps to the last concept.
  Verified in a real browser — seeding the cursor and reloading opens on
  "Concept 5 · Greeting · Hello", not concept 1.
- **Slice 7b (grammar introduction) was reframed:** the curriculum's grammar
  signal is a single concept tag (`GRAMMAR-THE`, articles) with no dedicated
  explanation field — too thin to ground an honest "new grammar" note the way
  scripts have `signature` data. Rather than fabricate one, this slice does the
  more valuable, fully-grounded resume-cursor work. Grammar introduction can
  return if the curriculum grows richer grammar metadata.

