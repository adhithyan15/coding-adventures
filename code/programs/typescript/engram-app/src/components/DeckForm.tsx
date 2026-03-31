/**
 * DeckForm.tsx — Create or edit a deck.
 *
 * Routes:
 *   #/deck/new          → create mode (empty form)
 *   #/deck/:id/edit     → edit mode (pre-filled with existing deck data)
 *
 * On submit, dispatches DECK_CREATE or DECK_UPDATE and navigates home.
 */

import { useState } from "react";
import { useStore } from "@coding-adventures/store";
import { store } from "../state.js";
import { createDeckAction, updateDeckAction } from "../actions.js";

interface DeckFormProps {
  deckId?: string; // undefined = create mode, present = edit mode
  onNavigate: (path: string) => void;
}

export function DeckForm({ deckId, onNavigate }: DeckFormProps) {
  const state = useStore(store);
  const existing = deckId ? state.decks.find((d) => d.id === deckId) : undefined;

  const [name, setName] = useState(existing?.name ?? "");
  const [description, setDescription] = useState(existing?.description ?? "");

  const isEdit = !!existing;
  const title = isEdit ? "Edit Deck" : "New Deck";
  const submitLabel = isEdit ? "Save" : "Create";

  // Deck not found in edit mode — show error
  if (deckId && !existing) {
    return (
      <div className="form-container">
        <p>Deck not found.</p>
        <button type="button" className="btn--secondary" onClick={() => onNavigate("/")}>
          Back to Home
        </button>
      </div>
    );
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const trimmedName = name.trim();
    if (!trimmedName) return;

    if (isEdit && deckId) {
      store.dispatch(updateDeckAction(deckId, trimmedName, description.trim()));
    } else {
      store.dispatch(createDeckAction(trimmedName, description.trim()));
    }
    onNavigate("/");
  }

  return (
    <div className="form-container">
      <h2 className="form-container__title">{title}</h2>
      <form onSubmit={handleSubmit} className="form">
        <label className="form__label">
          Name
          <input
            type="text"
            className="form__input"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. US State Capitals"
            autoFocus
          />
        </label>
        <label className="form__label">
          Description
          <textarea
            className="form__textarea"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="What is this deck about?"
            rows={3}
          />
        </label>
        <div className="form__actions">
          <button type="submit" className="btn--primary" disabled={!name.trim()}>
            {submitLabel}
          </button>
          <button type="button" className="btn--secondary" onClick={() => onNavigate("/")}>
            Cancel
          </button>
        </div>
      </form>
    </div>
  );
}
