/**
 * CardForm.tsx — Create or edit a card.
 *
 * Routes:
 *   #/deck/:id/cards/new               → create mode
 *   #/deck/:id/cards/:cardId/edit      → edit mode
 *
 * On submit, dispatches CARD_CREATE or CARD_UPDATE and navigates back
 * to the card list for this deck.
 */

import { useState } from "react";
import { useStore } from "@coding-adventures/store";
import { store } from "../state.js";
import { createCardAction, updateCardAction } from "../actions.js";

interface CardFormProps {
  deckId: string;
  cardId?: string; // undefined = create mode, present = edit mode
  onNavigate: (path: string) => void;
}

export function CardForm({ deckId, cardId, onNavigate }: CardFormProps) {
  const state = useStore(store);
  const deck = state.decks.find((d) => d.id === deckId);
  const existing = cardId ? state.cards.find((c) => c.id === cardId) : undefined;

  const [front, setFront] = useState(existing?.front ?? "");
  const [back, setBack] = useState(existing?.back ?? "");

  const isEdit = !!existing;
  const title = isEdit ? "Edit Card" : "New Card";
  const submitLabel = isEdit ? "Save" : "Create";
  const backPath = `/deck/${deckId}/cards`;

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

  if (cardId && !existing) {
    return (
      <div className="form-container">
        <p>Card not found.</p>
        <button type="button" className="btn--secondary" onClick={() => onNavigate(backPath)}>
          Back to Cards
        </button>
      </div>
    );
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const trimmedFront = front.trim();
    const trimmedBack = back.trim();
    if (!trimmedFront || !trimmedBack) return;

    if (isEdit && cardId) {
      store.dispatch(updateCardAction(cardId, trimmedFront, trimmedBack));
    } else {
      store.dispatch(createCardAction(deckId, trimmedFront, trimmedBack));
    }
    onNavigate(backPath);
  }

  return (
    <div className="form-container">
      <h2 className="form-container__title">{title}</h2>
      <p className="form-container__subtitle">{deck.name}</p>
      <form onSubmit={handleSubmit} className="form">
        <label className="form__label">
          Front (question/prompt)
          <textarea
            className="form__textarea"
            value={front}
            onChange={(e) => setFront(e.target.value)}
            placeholder="e.g. What is the capital of California?"
            rows={3}
            autoFocus
          />
        </label>
        <label className="form__label">
          Back (answer)
          <textarea
            className="form__textarea"
            value={back}
            onChange={(e) => setBack(e.target.value)}
            placeholder="e.g. Sacramento"
            rows={3}
          />
        </label>
        <div className="form__actions">
          <button
            type="submit"
            className="btn--primary"
            disabled={!front.trim() || !back.trim()}
          >
            {submitLabel}
          </button>
          <button type="button" className="btn--secondary" onClick={() => onNavigate(backPath)}>
            Cancel
          </button>
        </div>
      </form>
    </div>
  );
}
