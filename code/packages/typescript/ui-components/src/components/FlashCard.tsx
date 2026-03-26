/**
 * FlashCard — an animated flip card for spaced repetition study.
 *
 * Shows a front face (the prompt) and a back face (the answer). When
 * `revealed` changes from false to true, the card performs a CSS 3D flip
 * animation to show the back face. This gives the user a satisfying tactile
 * moment between seeing the question and the answer.
 *
 * === How the 3D flip works ===
 *
 * The trick uses three nested elements:
 *
 *   1. `.flash-card` — the outer container. Sets `perspective: 1000px` which
 *      gives the child its sense of depth. The larger the perspective value,
 *      the subtler the effect. 1000px is a good default.
 *
 *   2. `.flash-card__inner` — the rotating layer. Uses `transform-style:
 *      preserve-3d` so its children are positioned in 3D space, not flattened.
 *      On reveal it rotates 180° around the Y axis.
 *
 *   3. `.flash-card__front` and `.flash-card__back` — both positioned
 *      `absolute; inset: 0` so they stack on top of each other. The back face
 *      has `transform: rotateY(180deg)` so it starts "facing away". Both use
 *      `backface-visibility: hidden` so you only ever see one face at a time.
 *
 * === Accessibility ===
 *
 * The card uses `role="region"` with an `aria-label` so screen readers
 * announce context. The `aria-live="polite"` region announces the revealed
 * content when the card flips, without interrupting speech.
 *
 * @example
 * ```tsx
 * <FlashCard
 *   front="What is the capital of California?"
 *   back="Sacramento"
 *   revealed={isRevealed}
 * />
 * ```
 */

export interface FlashCardProps {
  /** The question or prompt shown before reveal. */
  front: string;
  /** The answer shown after reveal. */
  back: string;
  /** When true, the card flips to show the back face. */
  revealed: boolean;
  /** Optional CSS class for the outer container. */
  className?: string;
}

export function FlashCard({
  front,
  back,
  revealed,
  className = "flash-card",
}: FlashCardProps) {
  return (
    <div
      className={className}
      role="region"
      aria-label={revealed ? "Answer" : "Question"}
    >
      <div
        className={`flash-card__inner${revealed ? " flash-card__inner--revealed" : ""}`}
      >
        {/* Front face — the question */}
        <div className="flash-card__front" aria-hidden={revealed}>
          <p className="flash-card__text">{front}</p>
        </div>

        {/* Back face — the answer */}
        <div className="flash-card__back" aria-hidden={!revealed}>
          <p className="flash-card__label">Answer</p>
          <p className="flash-card__text">{back}</p>
        </div>
      </div>
    </div>
  );
}
