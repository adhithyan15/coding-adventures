/**
 * DeckStats.tsx — Per-deck learning statistics screen.
 *
 * Shows a breakdown of how well the user knows the deck:
 *   - Total cards
 *   - New (never seen)
 *   - Learning (interval ≤ 21 days, actively rotating)
 *   - Mastered (interval > 21 days, rarely needs review)
 *   - Due today
 *   - Average ease factor
 *   - All-time reviews from this deck
 */

import { useStore } from "@coding-adventures/store";
import { store } from "../state.js";
import { getDeckStats } from "../queue.js";

interface DeckStatsProps {
  deckId: string;
  onNavigate: (path: string) => void;
}

export function DeckStats({ deckId, onNavigate }: DeckStatsProps) {
  const state = useStore(store);
  const deck = state.decks.find((d) => d.id === deckId);

  if (!deck) {
    return (
      <div>
        <p>Deck not found.</p>
        <button type="button" className="btn--secondary" onClick={() => onNavigate("/")}>
          Home
        </button>
      </div>
    );
  }

  const stats = getDeckStats(state.cards, state.cardProgress, deckId);

  // Count all-time reviews for cards in this deck
  const deckCardIds = new Set(
    state.cards.filter((c) => c.deckId === deckId).map((c) => c.id),
  );
  const allTimeReviews = state.reviews.filter((r) =>
    deckCardIds.has(r.cardId),
  ).length;

  return (
    <div className="deck-stats">
      <h1 className="deck-stats__title">{deck.name} — Stats</h1>

      <div className="deck-stats__grid">
        <div className="stat-card">
          <p className="stat-card__value">{stats.total}</p>
          <p className="stat-card__label">Total cards</p>
        </div>
        <div className="stat-card">
          <p className="stat-card__value">{stats.newCount}</p>
          <p className="stat-card__label">New</p>
        </div>
        <div className="stat-card">
          <p className="stat-card__value">{stats.learningCount}</p>
          <p className="stat-card__label">Learning</p>
        </div>
        <div className="stat-card">
          <p className="stat-card__value">{stats.masteredCount}</p>
          <p className="stat-card__label">Mastered</p>
        </div>
        <div className="stat-card">
          <p className="stat-card__value">{stats.dueCount}</p>
          <p className="stat-card__label">Due today</p>
        </div>
        <div className="stat-card">
          <p className="stat-card__value">
            {stats.averageEaseFactor > 0
              ? stats.averageEaseFactor.toFixed(2)
              : "—"}
          </p>
          <p className="stat-card__label">Avg. ease</p>
        </div>
        <div className="stat-card">
          <p className="stat-card__value">{allTimeReviews}</p>
          <p className="stat-card__label">All-time reviews</p>
        </div>
      </div>

      <div className="deck-stats__back">
        <button
          type="button"
          className="btn--secondary"
          onClick={() => onNavigate("/")}
        >
          Back
        </button>
      </div>
    </div>
  );
}
