/**
 * StudySession.tsx — The core study screen.
 *
 * Shows one card at a time from the active session queue.
 *
 * Flow:
 *   1. Card front is shown, back is hidden
 *   2. User clicks "Show Answer" → SESSION_REVEAL → card flips
 *   3. User clicks a rating button → SESSION_RATE → SESSION_ADVANCE
 *   4. If queue is exhausted → SESSION_COMPLETE → navigate to /session/complete
 */

import { useStore } from "@coding-adventures/store";
import { FlashCard, RatingButtons, ProgressBar } from "@coding-adventures/ui-components";
import type { Rating } from "@coding-adventures/ui-components";
import { store } from "../state.js";
import { revealAction, rateAction, advanceAction, completeSessionAction } from "../actions.js";

interface StudySessionProps {
  onNavigate: (path: string) => void;
}

export function StudySession({ onNavigate }: StudySessionProps) {
  const state = useStore(store);
  const { activeSession } = state;

  // Guard: if no active session, redirect home
  if (!activeSession) {
    return (
      <div>
        <p>No active session.</p>
        <button type="button" className="btn--secondary" onClick={() => onNavigate("/")}>
          Back to Home
        </button>
      </div>
    );
  }

  const { queue, currentIndex, revealed, sessionId } = activeSession;
  const currentCard = queue[currentIndex];
  const total = queue.length;

  // Guard: queue exhausted (should not normally reach here — complete fires first)
  if (!currentCard) {
    return (
      <div>
        <p>Session complete.</p>
        <button type="button" className="btn--secondary" onClick={() => onNavigate("/")}>
          Home
        </button>
      </div>
    );
  }

  function handleShowAnswer() {
    store.dispatch(revealAction());
  }

  function handleRate(rating: Rating) {
    if (!activeSession) return;

    const reviewId = crypto.randomUUID
      ? crypto.randomUUID()
      : `${Date.now()}-${Math.random().toString(36).slice(2)}`;

    const now = Date.now();

    // Rate the card (updates CardProgress + logs Review + increments session counters)
    store.dispatch(rateAction(currentCard!.id, sessionId, reviewId, rating, now));

    // Advance to the next card
    const nextIndex = currentIndex + 1;
    store.dispatch(advanceAction());

    // If we've gone through all cards, complete the session
    if (nextIndex >= total) {
      store.dispatch(completeSessionAction(sessionId, now));
      onNavigate("/session/complete");
    }
  }

  const progressLabel = `${currentIndex} / ${total}`;

  return (
    <div className="study-session">
      <div className="study-session__progress">
        <ProgressBar value={currentIndex} max={total} label={progressLabel} />
      </div>

      <div className="study-session__card">
        <FlashCard
          front={currentCard.front}
          back={currentCard.back}
          revealed={revealed}
        />
      </div>

      <div className="study-session__actions">
        {!revealed ? (
          <button
            type="button"
            className="btn--primary"
            onClick={handleShowAnswer}
          >
            Show Answer
          </button>
        ) : (
          <RatingButtons onRate={handleRate} />
        )}
      </div>
    </div>
  );
}
