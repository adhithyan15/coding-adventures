# Changelog — Engram

## 0.1.0 — 2026-03-25

Initial release.

### Added

- **SM-2 spaced repetition algorithm** (`src/sm2.ts`) — pure functions for creating and
  updating `CardProgress` records based on four ratings: again, hard, good, easy. Interval
  and ease-factor clamped to specification limits (min EF: 1.3, max: 4.0).

- **Session queue assembly** (`src/queue.ts`) — blends due cards (have progress, overdue
  first) with new cards (never seen, capped at 7/session) into a session queue of up to 20
  cards. Pure functions: `buildSessionQueue`, `isDeckCaughtUp`, `getDeckStats`.

- **US State Capitals seed deck** (`src/seed.ts`) — 50 cards covering all US state capitals.
  Front: "What is the capital of {State}?" / Back: "{Capital}". Seeded on first launch when
  no decks exist in IndexedDB.

- **Flux state management** (`src/actions.ts`, `src/reducer.ts`, `src/state.ts`) — eight
  actions: DECK_CREATE, CARD_CREATE, SESSION_START, SESSION_REVEAL, SESSION_RATE,
  SESSION_ADVANCE, SESSION_COMPLETE, STATE_LOAD. Pure reducer with exported helper
  functions `getSessionCorrectPct` and `getSessionNewCount`.

- **IndexedDB persistence** (`src/persistence.ts`) — fire-and-forget middleware persisting
  decks, cards, card progress, sessions, and reviews. `activeSession` is ephemeral and never
  persisted.

- **Hash-based router** (`src/App.tsx`) — four routes: `/` (DeckList), `/session`
  (StudySession), `/session/complete` (SessionComplete), `/deck/:id/stats` (DeckStats).
  Works identically in Electron (file://) and the browser.

- **DeckList screen** — home screen showing all decks with due-card count, "Study" and
  "View Stats" actions per deck.

- **StudySession screen** — progress bar, flashcard flip animation, "Show Answer" button,
  four SM-2 rating buttons. Automatically completes session when queue is exhausted.

- **SessionComplete screen** — cards reviewed, correct percentage, new cards learned.
  "Study Again" rebuilds the queue for the same deck.

- **DeckStats screen** — total, new, learning, mastered, due-today counts; average ease
  factor; all-time review count.

- **Electron desktop app** — main process with `nodeIntegration: false` and
  `contextIsolation: true`. Dev mode loads Vite dev server; prod mode loads
  `dist/index.html` directly.

- **Shared UI components** (added to `@coding-adventures/ui-components`):
  - `FlashCard` — 3D CSS flip animation, front/back faces, full ARIA support.
  - `RatingButtons` — four color-coded SM-2 rating buttons (again=red, hard=orange,
    good=green, easy=blue).
  - `ProgressBar` — generic progress bar promoted from checklist-app.

- **Full test suite** — 95%+ coverage on SM-2 and queue logic, reducer tests for all
  8 action types, seed tests for all 50 capitals.
