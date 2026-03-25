/**
 * actions.ts — Action type constants and creator functions.
 *
 * Actions are plain objects that describe "what happened". They are the
 * only way to trigger a state change. The reducer receives each action and
 * computes the next state.
 *
 * === Pattern ===
 *
 * Each action has:
 *   1. A string constant (e.g. DECK_CREATE) — prevents typos when switching
 *      on action.type.
 *   2. A creator function (e.g. createDeckAction) — a factory that builds
 *      the action object. Centralises the shape of each action.
 *
 * Components import creator functions, not the raw constants. The constants
 * are exported for use in the reducer and persistence middleware switch
 * statements.
 */

// ── Action type constants ───────────────────────────────────────────────────

export const DECK_CREATE = "DECK_CREATE";
export const CARD_CREATE = "CARD_CREATE";
export const SESSION_START = "SESSION_START";
export const SESSION_REVEAL = "SESSION_REVEAL";
export const SESSION_RATE = "SESSION_RATE";
export const SESSION_ADVANCE = "SESSION_ADVANCE";
export const SESSION_COMPLETE = "SESSION_COMPLETE";
export const STATE_LOAD = "STATE_LOAD";

// ── Action types ────────────────────────────────────────────────────────────

import type { Action } from "@coding-adventures/store";
import type { Card, CardProgress, Deck, Rating, Review, Session } from "./types.js";

export interface DeckCreateAction extends Action {
  type: typeof DECK_CREATE;
  name: string;
  description: string;
}

export interface CardCreateAction extends Action {
  type: typeof CARD_CREATE;
  deckId: string;
  front: string;
  back: string;
}

export interface SessionStartAction extends Action {
  type: typeof SESSION_START;
  deckId: string;
  sessionId: string;
  queue: Card[];
}

export interface SessionRevealAction extends Action {
  type: typeof SESSION_REVEAL;
}

export interface SessionRateAction extends Action {
  type: typeof SESSION_RATE;
  cardId: string;
  sessionId: string;
  reviewId: string;
  rating: Rating;
  now: number;
}

export interface SessionAdvanceAction extends Action {
  type: typeof SESSION_ADVANCE;
}

export interface SessionCompleteAction extends Action {
  type: typeof SESSION_COMPLETE;
  sessionId: string;
  now: number;
}

export interface StateLoadAction extends Action {
  type: typeof STATE_LOAD;
  decks: Deck[];
  cards: Card[];
  cardProgress: CardProgress[];
  sessions: Session[];
  reviews: Review[];
}

export type AppAction =
  | DeckCreateAction
  | CardCreateAction
  | SessionStartAction
  | SessionRevealAction
  | SessionRateAction
  | SessionAdvanceAction
  | SessionCompleteAction
  | StateLoadAction;

// ── Action creators ─────────────────────────────────────────────────────────

/**
 * createDeckAction — create a new deck.
 */
export function createDeckAction(
  name: string,
  description: string,
): DeckCreateAction {
  return { type: DECK_CREATE, name, description };
}

/**
 * createCardAction — add a card to a deck.
 */
export function createCardAction(
  deckId: string,
  front: string,
  back: string,
): CardCreateAction {
  return { type: CARD_CREATE, deckId, front, back };
}

/**
 * startSessionAction — begin a study session.
 *
 * The queue is pre-built by buildSessionQueue() before dispatch.
 * A session ID is also pre-generated so the reducer doesn't need to.
 */
export function startSessionAction(
  deckId: string,
  sessionId: string,
  queue: Card[],
): SessionStartAction {
  return { type: SESSION_START, deckId, sessionId, queue };
}

/**
 * revealAction — flip the current card to show its back face.
 */
export function revealAction(): SessionRevealAction {
  return { type: SESSION_REVEAL };
}

/**
 * rateAction — rate the current card and update its CardProgress via SM-2.
 *
 * Also creates a Review record. The reviewId and now timestamp are passed
 * in so the reducer stays pure (no Date.now() calls inside).
 */
export function rateAction(
  cardId: string,
  sessionId: string,
  reviewId: string,
  rating: Rating,
  now: number = Date.now(),
): SessionRateAction {
  return { type: SESSION_RATE, cardId, sessionId, reviewId, rating, now };
}

/**
 * advanceAction — move to the next card in the session queue.
 */
export function advanceAction(): SessionAdvanceAction {
  return { type: SESSION_ADVANCE };
}

/**
 * completeSessionAction — mark the session as completed.
 */
export function completeSessionAction(
  sessionId: string,
  now: number = Date.now(),
): SessionCompleteAction {
  return { type: SESSION_COMPLETE, sessionId, now };
}

/**
 * stateLoadAction — bulk-load persisted data from IndexedDB on startup.
 */
export function stateLoadAction(
  decks: Deck[],
  cards: Card[],
  cardProgress: CardProgress[],
  sessions: Session[],
  reviews: Review[],
): StateLoadAction {
  return { type: STATE_LOAD, decks, cards, cardProgress, sessions, reviews };
}
