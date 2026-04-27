# Engram

A spaced repetition flashcard desktop app built with Electron and React.

Engram implements the **SM-2 algorithm** — the same algorithm that powers Anki —
to schedule when you see each card again. Cards you know well get longer intervals
(days, then weeks, then months). Cards you struggle with come back tomorrow.

The result: you study at the exact moment you're about to forget, making each
review session as efficient as possible.

## The SM-2 Algorithm

Every card has two numbers:

- **Interval** — days until you see it again. Starts at 1.
- **Ease factor** — a multiplier (default 2.5) that grows the interval over time.

After each review you pick one of four ratings:

| Rating | Meaning | Effect |
|--------|---------|--------|
| Again | Completely forgot | Reset interval to 1 day; ease factor drops |
| Hard | Recalled with difficulty | Interval grows slowly; ease factor drops slightly |
| Good | Recalled correctly | Interval multiplied by ease factor (normal growth) |
| Easy | Recalled instantly | Interval multiplied by ease factor × 1.3 bonus; EF rises |

A card with interval=1 rated "good" (EF=2.5) becomes interval=2, then 5, then 12,
then 31 — roughly doubling every review. Rate it "again" and it resets to 1.

## Architecture

```
src/
├── types.ts          All TypeScript interfaces
├── sm2.ts            SM-2 algorithm — pure functions, no side effects
├── queue.ts          Session queue builder — blends due + new cards
├── seed.ts           US State Capitals — 50 flashcards for first launch
├── actions.ts        Action constants + creator functions
├── reducer.ts        Pure state transitions
├── state.ts          Store instantiation
├── persistence.ts    IndexedDB middleware (fire-and-forget)
├── main.tsx          Async init: IndexedDB → STATE_LOAD or seedDeck → React mount
├── App.tsx           Hash router (4 routes)
└── components/
    ├── DeckList.tsx      Home screen
    ├── StudySession.tsx  Review screen
    ├── SessionComplete.tsx  Summary after session
    └── DeckStats.tsx     Per-deck learning statistics
```

State flows one direction: `action → reducer → store → React`. IndexedDB writes
happen as a side effect in `persistence.ts` — never in the reducer.

`activeSession` (which card you're on, whether you've revealed it) is ephemeral:
it lives in the store but is never written to IndexedDB. If the app closes
mid-session, CardProgress records written so far are preserved; the session itself
is simply lost.

## Session Queue

Each session blends two categories:

- **Due cards** — have a `CardProgress` record and `nextDueAt <= now`.
  Sorted most-overdue first (worst first).
- **New cards** — never seen (no `CardProgress` record yet).
  Capped at 7 per session to avoid overwhelming new learners.

Total session size: 20 cards (configurable in `src/queue.ts`).

## Screens

| Route | Screen |
|---|---|
| `#/` | Deck list — study or view stats per deck |
| `#/session` | Study session — flip card, rate, advance |
| `#/session/complete` | Session summary — reviewed, correct %, new learned |
| `#/deck/:id/stats` | Deck stats — new / learning / mastered / due |

## Running

```bash
# Install dependencies (from repo root)
cd code/packages/typescript/indexeddb && npm install
cd ../store && npm install
cd ../ui-components && npm install
cd ../../programs/typescript/engram-app && npm install

# Run in browser (Vite dev server)
npm run dev

# Run tests
npm run test

# Build for Electron (production)
npm run build
npx tsc -p electron/tsconfig.json
npx electron .
```

## Testing

```bash
npm run test           # run tests once
npm run test:coverage  # with coverage report
```

Target: 95%+ on `sm2.ts` and `queue.ts`, 80%+ overall.

## Releasing

Tag the commit and push:

```bash
git tag engram-v0.1.0
git push origin engram-v0.1.0
```

GitHub Actions builds macOS (.dmg + .zip), Windows (.exe + portable), and
Linux (.AppImage) in parallel and attaches all installers to a GitHub Release.

## Spec

See [`/code/specs/engram-app.md`](/code/specs/engram-app.md) for the full specification
including type definitions, SM-2 algorithm details, session queue assembly, UI/UX flows,
and persistence schema.
