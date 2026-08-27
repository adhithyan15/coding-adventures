## 0.14.0 — start over: a "Reset progress" control (HL03 polish)

- **You can now clear your progress.** The app persists a lot — the review
  quiz's SRS state and answer log, the teaching cursor, and the lesson schedule —
  but had no way to wipe it (handing the tab to someone else, or re-walking from
  the top). A quiet **"Reset progress"** control sits at the foot of the Learn
  view; it's a **two-click confirm** (first click arms *"Clear all progress…?"*
  with Yes / Cancel, second executes) so a stray tap can't erase everything.
- New pure `src/reset.ts`: `OWNED_STORAGE_KEYS` sources the three keys from the
  modules that own them (so the list can't drift from what's actually written),
  and `clearProgress(storage)` removes exactly those — **only keys this app
  owns**, guarded per-key so one locked key can't turn "start over" into a crash.
  Executing also resets the in-memory session to concept 0 with an empty review.
- 6 new tests (225 total) with a control that bites: a `clearProgress` that
  missed any owned key leaves it behind and fails; unowned keys are left
  untouched; a throwing `removeItem` still clears the rest. Verified in a real
  browser — both the link and the armed *"Yes, reset / Cancel"* state render.

