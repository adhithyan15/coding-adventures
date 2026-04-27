/**
 * SessionComplete.tsx — Session summary screen.
 *
 * Shown after the last card in a session is rated. Displays:
 *   - Cards reviewed count
 *   - Correct percentage (hard + good + easy / total)
 *   - New cards learned this session
 *
 * Two action buttons:
 *   "Study Again" → starts a new session for the same deck
 *   "Home" → navigates back to the deck list
 */

import { useStore } from "@coding-adventures/store";
import { store } from "../state.js";
import { startSessionAction } from "../actions.js";
import { buildSessionQueue } from "../queue.js";
import { getSessionCorrectPct, getSessionNewCount } from "../reducer.js";

interface SessionCompleteProps {
  onNavigate: (path: string) => void;
}

export function SessionComplete({ onNavigate }: SessionCompleteProps) {
  const state = useStore(store);

  // Find the most recently completed session
  const completedSession = [...state.sessions]
    .reverse()
    .find((s) => s.status === "completed");

  if (!completedSession) {
    return (
      <div>
        <p>No session data.</p>
        <button type="button" className="btn--secondary" onClick={() => onNavigate("/")}>
          Home
        </button>
      </div>
    );
  }

  const correctPct = getSessionCorrectPct(state.sessions, completedSession.id);
  const newLearned = getSessionNewCount(
    state.cardProgress,
    state.reviews,
    completedSession.id,
  );

  function handleStudyAgain() {
    const now = Date.now();
    const queue = buildSessionQueue(
      state.cards,
      state.cardProgress,
      completedSession!.deckId,
      now,
    );
    if (queue.length === 0) {
      onNavigate("/");
      return;
    }

    const sessionId = crypto.randomUUID
      ? crypto.randomUUID()
      : `${Date.now()}-${Math.random().toString(36).slice(2)}`;

    store.dispatch(startSessionAction(completedSession!.deckId, sessionId, queue));
    onNavigate("/session");
  }

  return (
    <div className="session-complete">
      <h1 className="session-complete__title">Session Complete!</h1>

      <div className="session-complete__grid">
        <div className="stat-card">
          <p className="stat-card__value">{completedSession.cardsReviewed}</p>
          <p className="stat-card__label">Cards reviewed</p>
        </div>
        <div className="stat-card">
          <p className="stat-card__value">{correctPct}%</p>
          <p className="stat-card__label">Correct</p>
        </div>
        <div className="stat-card">
          <p className="stat-card__value">{newLearned}</p>
          <p className="stat-card__label">New learned</p>
        </div>
      </div>

      <div className="session-complete__actions">
        <button type="button" className="btn--primary" onClick={handleStudyAgain}>
          Study Again
        </button>
        <button type="button" className="btn--secondary" onClick={() => onNavigate("/")}>
          Home
        </button>
      </div>
    </div>
  );
}
