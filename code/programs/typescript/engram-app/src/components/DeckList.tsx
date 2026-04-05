/**
 * DeckList.tsx — Home screen. Lists all decks with study stats and CRUD controls.
 *
 * For each deck the user sees:
 *   - Name and description
 *   - Card counts: total, due today, new (unseen)
 *   - "Study" button (disabled when caught up — no due or new cards)
 *   - "Cards" button → #/deck/:id/cards (manage cards)
 *   - "Edit" button → #/deck/:id/edit
 *   - "Stats" button → #/deck/:id/stats
 *   - "Delete" button → confirms then dispatches DECK_DELETE
 *
 * A "New Deck" button at the top navigates to #/deck/new.
 */

import { useStore } from "@coding-adventures/store";
import { store } from "../state.js";
import { startSessionAction, deleteDeckAction } from "../actions.js";
import { buildSessionQueue, isDeckCaughtUp, getDeckStats } from "../queue.js";
import { generateSecureId } from "../secure-id.js";
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

    const sessionId = generateSecureId();

    store.dispatch(startSessionAction(deck.id, sessionId, queue));
    onNavigate("/session");
  }

  function handleDelete(deck: Deck) {
    const cardCount = state.cards.filter((c) => c.deckId === deck.id).length;
    const message = cardCount > 0
      ? `Delete "${deck.name}" and its ${cardCount} card${cardCount === 1 ? "" : "s"}? This also removes all study progress and session history for this deck.`
      : `Delete "${deck.name}"?`;
    if (!window.confirm(message)) return;
    store.dispatch(deleteDeckAction(deck.id));
  }

  return (
    <div className="deck-list">
      <div className="deck-list__header">
        <button
          type="button"
          className="btn--primary"
          onClick={() => onNavigate("/deck/new")}
        >
          New Deck
        </button>
      </div>

      {state.decks.length === 0 ? (
        <div className="deck-list__empty">
          <p>No decks yet. Create your first deck to start studying.</p>
        </div>
      ) : (
        state.decks.map((deck) => {
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
                  onClick={() => onNavigate(`/deck/${deck.id}/cards`)}
                >
                  Cards
                </button>
                <button
                  type="button"
                  className="btn--secondary"
                  onClick={() => onNavigate(`/deck/${deck.id}/edit`)}
                >
                  Edit
                </button>
                <button
                  type="button"
                  className="btn--secondary"
                  onClick={() => onNavigate(`/deck/${deck.id}/stats`)}
                >
                  Stats
                </button>
                <button
                  type="button"
                  className="btn--danger"
                  onClick={() => handleDelete(deck)}
                >
                  Delete
                </button>
              </div>
            </div>
          );
        })
      )}
    </div>
  );
}
