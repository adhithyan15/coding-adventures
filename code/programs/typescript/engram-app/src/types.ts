/**
 * types.ts — All TypeScript interfaces for the Flashcard app.
 *
 * The data model has three layers:
 *
 *   1. Content layer  — Deck and Card. Authored once, studied many times.
 *      Analogous to a textbook: fixed after publication.
 *
 *   2. Progress layer — CardProgress. One record per card, updated on
 *      every review. Tracks the SM-2 spaced repetition state.
 *      Analogous to margin notes: personal and mutable.
 *
 *   3. Session layer  — Session, Review, ActiveSessionState. Records the
 *      history of each study sitting and drives the live session UI.
 *      Analogous to a study log: a timestamped record of what you did.
 */

// ── Content layer ───────────────────────────────────────────────────────────

/**
 * Deck — a named collection of related cards.
 *
 * Examples: "US State Capitals", "Spanish Verbs", "Periodic Table".
 * A deck is the unit of study — you start a session per deck, not per card.
 */
export interface Deck {
  id: string;
  name: string;
  description: string;
  createdAt: number; // Unix timestamp (ms)
}

/**
 * Card — the atomic unit of study.
 *
 * A card has two faces: front (the prompt) and back (the answer).
 * The user sees the front first, attempts recall, then reveals the back
 * to verify their answer before rating.
 *
 * Example:
 *   front: "What is the capital of California?"
 *   back:  "Sacramento"
 */
export interface Card {
  id: string;
  deckId: string;
  front: string; // Prompt shown before reveal
  back: string; // Answer shown after reveal
  createdAt: number;
}

// ── Progress layer ──────────────────────────────────────────────────────────

/**
 * CardProgress — the SM-2 spaced repetition state for one card.
 *
 * One record per card. Created when the card is reviewed for the first time.
 * Updated on every subsequent review via the SM-2 algorithm.
 *
 * Cards with no CardProgress record are "new" — never studied before.
 * Cards where nextDueAt <= Date.now() are "due" — scheduled for review.
 * Cards where nextDueAt > Date.now() are "scheduled" — waiting their turn.
 *
 * SM-2 parameters:
 *
 *   interval    — Days until next review. Starts at 1. Grows on correct
 *                 recall, resets to 1 on "again".
 *
 *   easeFactor  — Multiplier controlling interval growth. Starts at 2.5.
 *                 Range: [1.3, 4.0]. Increases on "easy", decreases on
 *                 "hard" or "again".
 */
export interface CardProgress {
  cardId: string; // Primary key — one record per card
  interval: number; // Days until next review (≥ 1)
  easeFactor: number; // SM-2 multiplier, range [1.3, 4.0], starts at 2.5
  nextDueAt: number; // Unix timestamp (ms) — due when <= Date.now()
  timesSeen: number; // Total review count across all sessions
  timesCorrect: number; // Reviews rated hard / good / easy
  timesIncorrect: number; // Reviews rated again
  lastSeenAt: number; // Timestamp of most recent review
}

// ── Session layer ───────────────────────────────────────────────────────────

/**
 * SessionStatus — lifecycle of a study session.
 *
 *   active    — currently in progress (user is reviewing cards)
 *   completed — all cards reviewed, session ended normally
 */
export type SessionStatus = "active" | "completed";

/**
 * Session — one study sitting for a specific deck.
 *
 * Created when the user clicks "Study". Completed when the card queue
 * is exhausted. Records aggregate stats for the session history.
 */
export interface Session {
  id: string;
  deckId: string;
  status: SessionStatus;
  startedAt: number;
  endedAt: number | null; // null while active
  cardsReviewed: number;
  cardsCorrect: number; // Ratings: hard + good + easy (not again)
}

/**
 * Rating — the four SM-2 recall quality ratings.
 *
 * Maps to SM-2 scores:
 *   again → 0  complete blank or wrong
 *   hard  → 1  correct but required significant effort
 *   good  → 2  correct with normal effort
 *   easy  → 3  immediate, effortless recall
 */
export type Rating = "again" | "hard" | "good" | "easy";

/**
 * Review — a single rating event within a session.
 *
 * One Review is created each time the user rates a card.
 * Together, reviews form a complete audit trail of study history.
 */
export interface Review {
  id: string;
  sessionId: string;
  cardId: string;
  rating: Rating;
  reviewedAt: number;
}

// ── App state ───────────────────────────────────────────────────────────────

/**
 * ActiveSessionState — ephemeral in-memory state for a live study session.
 *
 * This is NOT persisted to IndexedDB. If the app is closed mid-session,
 * the session is lost (but CardProgress records already written remain).
 * The session queue is rebuilt fresh on the next visit.
 */
export interface ActiveSessionState {
  sessionId: string;
  deckId: string;
  queue: Card[]; // Ordered list of cards for this session
  currentIndex: number; // Index of the card currently displayed
  revealed: boolean; // Whether the back face is currently visible
}

/**
 * AppState — the complete application state managed by the Flux store.
 *
 * Persisted arrays (decks, cards, cardProgress, sessions, reviews) are
 * loaded from IndexedDB on startup and written back via persistence middleware.
 *
 * activeSession is ephemeral — built from the queue at session start.
 */
export interface AppState {
  decks: Deck[];
  cards: Card[];
  cardProgress: CardProgress[];
  sessions: Session[];
  reviews: Review[];
  activeSession: ActiveSessionState | null;
}
