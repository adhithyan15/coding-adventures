/**
 * RatingButtons — SM-2 spaced repetition rating controls.
 *
 * Renders four buttons used to rate how well a flashcard was recalled.
 * The ratings map directly to SM-2 score values:
 *
 *   again (0) — complete blank or wrong answer
 *   hard  (1) — correct but required significant effort
 *   good  (2) — correct recall with normal effort
 *   easy  (3) — immediate, effortless recall
 *
 * === Color coding ===
 *
 * The buttons use traffic-light + blue convention familiar from Anki:
 *
 *   Again → red    — a stop signal: go back to start
 *   Hard  → orange — caution: right answer, wrong confidence
 *   Good  → green  — success: normal learning progression
 *   Easy  → blue   — acceleration: interval grows faster
 *
 * Each button uses an outlined style by default (colored border + text,
 * transparent background). On hover the button fills with its color.
 * This avoids a wall of solid color while still making the semantics clear.
 *
 * === Usage ===
 *
 * These buttons should only appear after the card has been revealed.
 * The parent component is responsible for gating their visibility.
 *
 * @example
 * ```tsx
 * {revealed && (
 *   <RatingButtons onRate={(rating) => handleRate(rating)} />
 * )}
 * ```
 */

export type Rating = "again" | "hard" | "good" | "easy";

export interface RatingButtonsProps {
  /** Called when the user clicks a rating button. */
  onRate: (rating: Rating) => void;
  /** When true, all buttons are disabled (e.g., during async transition). */
  disabled?: boolean;
  /** Optional CSS class for the container. */
  className?: string;
}

/** Metadata for each rating button. */
const RATINGS: Array<{
  rating: Rating;
  label: string;
  modifier: string;
  title: string;
}> = [
  {
    rating: "again",
    label: "Again",
    modifier: "again",
    title: "I didn't know this. Show me again soon.",
  },
  {
    rating: "hard",
    label: "Hard",
    modifier: "hard",
    title: "I got it right but it was difficult.",
  },
  {
    rating: "good",
    label: "Good",
    modifier: "good",
    title: "I recalled this correctly with normal effort.",
  },
  {
    rating: "easy",
    label: "Easy",
    modifier: "easy",
    title: "I knew this instantly. Increase the interval.",
  },
];

export function RatingButtons({
  onRate,
  disabled = false,
  className = "rating-buttons",
}: RatingButtonsProps) {
  return (
    <div className={className} role="group" aria-label="Rate your recall">
      {RATINGS.map(({ rating, label, modifier, title }) => (
        <button
          key={rating}
          type="button"
          className={`rating-buttons__btn rating-buttons__btn--${modifier}`}
          onClick={() => onRate(rating)}
          disabled={disabled}
          title={title}
        >
          {label}
        </button>
      ))}
    </div>
  );
}
