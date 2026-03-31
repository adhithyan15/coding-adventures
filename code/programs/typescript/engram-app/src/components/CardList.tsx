/**
 * CardList.tsx — View all cards in a deck with edit/delete/add controls.
 *
 * Route: #/deck/:id/cards
 *
 * Displays each card's front and back text. Provides buttons to:
 *   - Add a new card → navigates to #/deck/:id/cards/new
 *   - Edit a card → navigates to #/deck/:id/cards/:cardId/edit
 *   - Delete a card → confirms then dispatches CARD_DELETE
 */

import { useStore } from "@coding-adventures/store";
import { store } from "../state.js";
import { deleteCardAction } from "../actions.js";

interface CardListProps {
  deckId: string;
  onNavigate: (path: string) => void;
}

export function CardList({ deckId, onNavigate }: CardListProps) {
  const state = useStore(store);
  const deck = state.decks.find((d) => d.id === deckId);
  const cards = state.cards.filter((c) => c.deckId === deckId);

  if (!deck) {
    return (
      <div className="form-container">
        <p>Deck not found.</p>
        <button type="button" className="btn--secondary" onClick={() => onNavigate("/")}>
          Back to Home
        </button>
      </div>
    );
  }

  function handleDelete(cardId: string) {
    if (!window.confirm("Delete this card? Its study progress will also be removed.")) {
      return;
    }
    store.dispatch(deleteCardAction(cardId));
  }

  return (
    <div className="card-list">
      <div className="card-list__header">
        <h2 className="card-list__title">{deck.name} — Cards</h2>
        <button
          type="button"
          className="btn--primary"
          onClick={() => onNavigate(`/deck/${deckId}/cards/new`)}
        >
          Add Card
        </button>
      </div>

      {cards.length === 0 ? (
        <div className="card-list__empty">
          <p>No cards yet. Add your first card to start studying.</p>
        </div>
      ) : (
        <div className="card-list__items">
          {cards.map((card) => (
            <div key={card.id} className="card-item">
              <div className="card-item__content">
                <p className="card-item__front">{card.front}</p>
                <p className="card-item__back">{card.back}</p>
              </div>
              <div className="card-item__actions">
                <button
                  type="button"
                  className="btn--secondary"
                  onClick={() => onNavigate(`/deck/${deckId}/cards/${card.id}/edit`)}
                >
                  Edit
                </button>
                <button
                  type="button"
                  className="btn--danger"
                  onClick={() => handleDelete(card.id)}
                >
                  Delete
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      <button
        type="button"
        className="btn--secondary card-list__back"
        onClick={() => onNavigate("/")}
      >
        Back
      </button>
    </div>
  );
}
