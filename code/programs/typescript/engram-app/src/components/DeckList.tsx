/**
 * DeckList.tsx — Home screen. Lists all decks with study stats.
 *
 * For each deck the user sees:
 *   - Name and description
 *   - Card counts: total, due today, new (unseen)
 *   - "Study" button (disabled when caught up — no due or new cards)
 *   - "Stats" link → #/deck/:id/stats
 *
 * Clicking "Study" assembles a session queue via buildSessionQueue(),
 * creates a session ID, then dispatches SESSION_START before navigating
 * to the study screen.
 */

import { useStore } from "@coding-adventures/store";
import { store } from "../state.js";
import { startSessionAction } from "../actions.js";
import { buildSessionQueue, isDeckCaughtUp, getDeckStats } from "../queue.js";
import type { Deck } from "../types.js";

interface DeckListProps {
  onNavigate: (path: string) => void;
}

export function DeckList({ onNavigate }: DeckListProps) {
  const state = useStore(store);
  const now = Date.now();

  function handleStudy(deck: Deck) {
    const queue = buildSessionQueue(state.cards, state.cardProgress, deck.id, now);
    if (queue.length === 0) return;

    const sessionId = crypto.randomUUID
      ? crypto.randomUUID()
      : `${Date.now()}-${Math.random().toString(36).slice(2)}`;

    store.dispatch(startSessionAction(deck.id, sessionId, queue));
    onNavigate("/session");
  }

  if (state.decks.length === 0) {
    return (
      <div className="deck-list__empty">
        <p>No decks yet.</p>
      </div>
    );
  }

  return (
    <div className="deck-list">
      {state.decks.map((deck) => {
        const stats = getDeckStats(state.cards, state.cardProgress, deck.id, now);
        const caughtUp = isDeckCaughtUp(state.cards, state.cardProgress, deck.id, now);

        return (
          <div key={deck.id} className="deck-card">
            <p className="deck-card__name">{deck.name}</p>
            <p className="deck-card__description">{deck.description}</p>
            <div className="deck-card__meta">
              <span>{stats.total} cards</span>
              {stats.dueCount > 0 && (
                <span className="deck-card__meta-due">{stats.dueCount} due</span>
              )}
              {stats.newCount > 0 && (
                <span>{stats.newCount} new</span>
              )}
              {caughtUp && <span>All caught up!</span>}
            </div>
            <div className="deck-card__actions">
              <button
                type="button"
                className="btn--primary"
                onClick={() => handleStudy(deck)}
                disabled={caughtUp}
              >
                Study
              </button>
              <button
                type="button"
                className="btn--secondary"
                onClick={() => onNavigate(`/deck/${deck.id}/stats`)}
              >
                Stats
              </button>
            </div>
          </div>
        );
      })}
    </div>
  );
}
