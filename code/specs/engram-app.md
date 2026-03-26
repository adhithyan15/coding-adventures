# Engram — Spaced Repetition Flashcard Desktop App

## Overview

Anki is the gold standard for spaced repetition study tools. It schedules card
reviews at the mathematically optimal moment: just before you would forget the
answer. This minimises review time while maximising long-term retention.

This app is a faithful Anki clone built as an Electron desktop application.
The goal is to understand the SM-2 spaced repetition algorithm, Flux state
management, IndexedDB persistence, and the anatomy of a learning tool — and
to lay the foundation for a future Duolingo-style language learning app.

The first release ships with a single static deck: all 50 US state capitals.

---

## Core Concepts

### Deck

A **Deck** is a named collection of cards (e.g., "US State Capitals",
"Spanish Vocabulary"). Decks are authored once and studied repeatedly.

### Card

A **Card** is the atomic unit of study. It has a **front** (the question or
prompt) and a **back** (the answer). Cards belong to exactly one deck.

Example:
- Front: "What is the capital of California?"
- Back: "Sacramento"

### CardProgress

A **CardProgress** record tracks the spaced repetition state for one card per
user. It stores the SM-2 parameters: interval, ease factor, and next due date.

Cards with no `CardProgress` record are **new** — never seen before.
Cards with a `CardProgress.nextDueAt <= now` are **due** for review.
Cards with a future `nextDueAt` are **scheduled** — recently reviewed.

### Session

A **Session** is one study sitting. It has a fixed queue of cards assembled
at session start. The session records how many cards were reviewed and how
many were rated correctly.

Sessions are ephemeral in memory while active. They are persisted when
completed.

### Review

A **Review** is a single rating event within a session. The user sees a card,
reveals the answer, and rates their recall with one of four ratings:
`again`, `hard`, `good`, or `easy`. Each review updates the card's
`CardProgress` via the SM-2 algorithm.

---

## SM-2 Algorithm

SM-2 is the scheduling algorithm originally published by Piotr Wozniak and
used by Anki. It models human memory as an exponentially decaying function:
we forget things faster after long gaps without review.

The algorithm maintains two parameters per card per user:

**interval** — The number of days until the card is reviewed again. Starts at
1. Grows multiplicatively on each correct recall.

**easeFactor** — A multiplier controlling how fast the interval grows.
Starts at 2.5. Increases when you find the card easy; decreases when you
find it hard. Clamped to the range [1.3, 4.0].

### Rating → Score Mapping

| Rating | Score | Meaning |
|--------|-------|---------|
| again  | 0     | Complete blank / wrong |
| hard   | 1     | Got it right but struggled |
| good   | 2     | Normal correct recall |
| easy   | 3     | Trivially easy, knew it instantly |

### Interval Update Rules

```
score = 0 (again):
  interval    ← 1
  easeFactor  ← max(1.3, easeFactor - 0.20)

score = 1 (hard):
  interval    ← max(1, round(interval × 1.2))
  easeFactor  ← max(1.3, easeFactor - 0.15)

score = 2 (good):
  interval    ← max(1, round(interval × easeFactor))

score = 3 (easy):
  interval    ← max(1, round(interval × easeFactor × 1.3))
  easeFactor  ← min(4.0, easeFactor + 0.15)
```

The next due date is always: `now + interval × 24 × 60 × 60 × 1000` ms.

### Initial State for New Cards

When a card is reviewed for the first time, a new `CardProgress` record
is created with `interval = 1` and `easeFactor = 2.5`, then the rating
is applied on top.

---

## Session Queue Assembly

A session queue is assembled at session start from two pools:

**Due cards** — Cards with a `CardProgress` record where `nextDueAt <= now`.
Sorted by `nextDueAt` ascending (most overdue first). Capped at
`SESSION_SIZE - MAX_NEW_PER_SESSION` slots.

**New cards** — Cards with no `CardProgress` record at all (never reviewed).
Limited to `MAX_NEW_PER_SESSION` cards per session.

Constants:
- `SESSION_SIZE = 20` — Maximum total cards per session
- `MAX_NEW_PER_SESSION = 7` — Maximum new cards introduced per session

The due cards fill the remaining slots up to `SESSION_SIZE`. New cards
fill up to `MAX_NEW_PER_SESSION` slots. The queue may be shorter than
`SESSION_SIZE` if fewer cards are available.

If the review queue is empty and no new cards remain, the deck is fully
caught up and no session is started.

---

## Data Model

### Deck

```typescript
interface Deck {
  id: string;         // UUID
  name: string;       // "US State Capitals"
  description: string;
  createdAt: number;  // Unix timestamp (ms)
}
```

### Card

```typescript
interface Card {
  id: string;
  deckId: string;
  front: string;      // Question / prompt shown first
  back: string;       // Answer revealed on user request
  createdAt: number;
}
```

### CardProgress

```typescript
interface CardProgress {
  cardId: string;         // Primary key — one record per card
  interval: number;       // Days until next review (≥ 1)
  easeFactor: number;     // SM-2 multiplier [1.3, 4.0], starts at 2.5
  nextDueAt: number;      // Unix timestamp (ms) — card is due when <= now
  timesSeen: number;      // Total review count
  timesCorrect: number;   // Reviews rated hard / good / easy
  timesIncorrect: number; // Reviews rated again
  lastSeenAt: number;     // Timestamp of most recent review
}
```

### Session

```typescript
type SessionStatus = "active" | "completed";

interface Session {
  id: string;
  deckId: string;
  status: SessionStatus;
  startedAt: number;
  endedAt: number | null;
  cardsReviewed: number;
  cardsCorrect: number;   // Reviews rated hard / good / easy
}
```

### Review

```typescript
type Rating = "again" | "hard" | "good" | "easy";

interface Review {
  id: string;
  sessionId: string;
  cardId: string;
  rating: Rating;
  reviewedAt: number;
}
```

### AppState

```typescript
interface ActiveSessionState {
  sessionId: string;
  deckId: string;
  queue: Card[];           // Ordered cards for this session
  currentIndex: number;    // Index of the card currently shown
  revealed: boolean;       // Whether the back face is visible
}

interface AppState {
  decks: Deck[];
  cards: Card[];
  cardProgress: CardProgress[];
  sessions: Session[];
  reviews: Review[];
  activeSession: ActiveSessionState | null; // Ephemeral — not persisted
}
```

---

## Actions

```
DECK_CREATE       { name, description }
CARD_CREATE       { deckId, front, back }
SESSION_START     { deckId, queue: Card[] }
SESSION_REVEAL    {}  — flip card to show back face
SESSION_RATE      { cardId, sessionId, rating }
SESSION_ADVANCE   {}  — move to next card in queue
SESSION_COMPLETE  { sessionId }
STATE_LOAD        { decks, cards, cardProgress, sessions, reviews }
```

`SESSION_RATE` performs two mutations atomically:
1. Upserts the `CardProgress` record via SM-2 algorithm
2. Appends a `Review` record

---

## UI Screens

### Home (`#/`)

Lists all decks. For each deck:
- Name and description
- Count: total cards, cards due today, new cards
- "Study" button → builds queue → `SESSION_START` → navigates to `#/session`
- "Stats" link → `#/deck/:id/stats`

If the queue is empty (no due cards, no new cards): show "All caught up!"
message instead of the Study button.

### Study Session (`#/session`)

Shown while a session is active.

- Progress bar: `currentIndex / queue.length` cards complete
- FlashCard component showing the current card
  - Front face always visible
  - Back face hidden until user reveals it
- "Show Answer" button (visible when `!revealed`)
- Rating buttons: **Again | Hard | Good | Easy** (visible when `revealed`)

On rating:
1. Dispatch `SESSION_RATE` → updates `CardProgress` + logs `Review`
2. Dispatch `SESSION_ADVANCE` → increments `currentIndex`, resets `revealed`
3. If `currentIndex >= queue.length`: dispatch `SESSION_COMPLETE`, navigate to `#/session/complete`

### Session Complete (`#/session/complete`)

- "Session Complete!" heading
- Cards reviewed count
- Correct percentage (hard + good + easy / total)
- New cards learned this session
- "Study Again" button → start new session for same deck
- "Home" button → `#/`

### Deck Stats (`#/deck/:id/stats`)

Per-deck learning statistics:
- Total cards in deck
- New (never seen): cards with no `CardProgress`
- Learning (interval ≤ 21 days): cards in active rotation
- Mastered (interval > 21 days): cards seen infrequently
- Average ease factor
- Total reviews all-time
- "Back" button → `#/`

---

## Shared UI Components (additions to `@coding-adventures/ui-components`)

### FlashCard

A card that animates a CSS flip when `revealed` changes from `false` to `true`.

```typescript
interface FlashCardProps {
  front: string;
  back: string;
  revealed: boolean;
  className?: string;
}
```

Implementation:
- Outer container: `perspective: 1000px`
- Inner container: `transform-style: preserve-3d`, `transition: transform 0.5s`
- When `revealed`: `transform: rotateY(180deg)`
- Front face: visible at `rotateY(0deg)`
- Back face: visible at `rotateY(180deg)` via `backface-visibility: hidden`
- Both faces use `position: absolute; inset: 0`
- Front uses `var(--panel-bg)` background
- Back uses a slightly lighter background tint for visual distinction

### RatingButtons

Four buttons in a horizontal row for SM-2 rating.

```typescript
interface RatingButtonsProps {
  onRate: (rating: "again" | "hard" | "good" | "easy") => void;
  disabled?: boolean;
  className?: string;
}
```

Color scheme:
- Again: `var(--danger-color)` (#f87171 red)
- Hard: `#fb923c` (orange)
- Good: `var(--check-color)` (#4ade80 green)
- Easy: `#60a5fa` (blue)

Each button: outlined style (transparent background, colored border + text),
fills on hover.

### ProgressBar (promoted from checklist-app)

A generic visual progress bar suitable for any completion ratio.

```typescript
interface ProgressBarProps {
  value: number;  // Current progress (0 to max)
  max: number;    // Maximum value
  label?: string; // Optional label rendered below the bar
  className?: string;
}
```

The `checklist-app` currently has a domain-specific `ProgressBar` component.
The shared version generalises the props and removes the checklist-specific
i18n copy. The checklist-app will be updated to use the shared version.

---

## Persistence

IndexedDB stores (opened in `src/main.tsx`):

```
decks          keyPath: "id"
cards          keyPath: "id"     index: { name: "deckId", keyPath: "deckId" }
card_progress  keyPath: "cardId"
sessions       keyPath: "id"     index: { name: "deckId", keyPath: "deckId" }
reviews        keyPath: "id"
```

`activeSession` is ephemeral — never persisted to IndexedDB. If the app is
closed mid-session, the session is lost (the `CardProgress` records already
written remain correct).

Persistence middleware follows the fire-and-forget pattern from checklist-app:
`next()` runs the reducer first, then the middleware writes affected records
without awaiting.

---

## Seed Data

On first launch (no decks in IndexedDB), the app seeds one deck:
**"US State Capitals"** — all 50 US states with their capital cities.

Card format:
- Front: "What is the capital of {State}?"
- Back: "{Capital}"

The seed covers all 50 states in alphabetical order.

---

## Tech Stack

Mirrors `checklist-app` exactly:

| Concern | Technology |
|---------|-----------|
| UI | React 19 |
| Bundler | Vite 6 |
| Language | TypeScript 5.7 |
| Tests | Vitest 3 + jsdom |
| Desktop | Electron 35 |
| Packaging | electron-builder 26 |
| State | `@coding-adventures/store` |
| Persistence | `@coding-adventures/indexeddb` |
| Shared UI | `@coding-adventures/ui-components` |

---

## Release

Tagged releases follow the `flashcard-v*` pattern:

```bash
git tag flashcard-v0.1.0
git push origin flashcard-v0.1.0
```

GitHub Actions builds macOS (dmg + zip), Linux (AppImage), and Windows
(nsis + portable) in parallel, then creates a single GitHub Release with all
installers attached.

`appId`: `com.codingadventures.flashcard`
`productName`: `Flashcard`

---

## Testing Requirements

| Module | Target |
|--------|--------|
| `sm2.ts` | 100% — all four rating paths, easeFactor min/max clamping, interval floor |
| `queue.ts` | 100% — due only, new only, mixed, empty deck, overload |
| `reducer.ts` | 95%+ — every action type |
| `seed.ts` | 100% — 50 cards, unique IDs, correct capitals |
| Components | 80%+ |
