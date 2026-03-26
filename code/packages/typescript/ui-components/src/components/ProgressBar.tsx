/**
 * ProgressBar — a generic visual progress indicator.
 *
 * Renders a horizontal bar that fills from left to right as `value`
 * approaches `max`. The bar turns green when `value === max` to signal
 * completion.
 *
 * === Design ===
 *
 * The bar uses two divs: an outer container (the track) and an inner div
 * (the fill). The fill width is a percentage calculated as `value / max`.
 * CSS transition on `width` animates the fill smoothly as progress updates.
 *
 * An optional `label` prop renders descriptive text below the bar.
 * Accessible attributes (`role="progressbar"`, `aria-valuenow`, etc.) are
 * set on the track element so screen readers can announce progress.
 *
 * === Usage ===
 *
 * ```tsx
 * // During a study session: 3 of 20 cards reviewed
 * <ProgressBar value={3} max={20} label="3 / 20 cards" />
 *
 * // Complete
 * <ProgressBar value={20} max={20} label="Done!" />
 * ```
 */

export interface ProgressBarProps {
  /** Current progress value. Clamped to [0, max]. */
  value: number;
  /** Maximum value. Must be > 0. */
  max: number;
  /** Optional label rendered below the bar. */
  label?: string;
  /** Optional CSS class for the outer container. */
  className?: string;
}

export function ProgressBar({
  value,
  max,
  label,
  className = "progress-bar",
}: ProgressBarProps) {
  const safeMax = max > 0 ? max : 1;
  const pct = Math.min(100, Math.max(0, (value / safeMax) * 100));
  const isComplete = max > 0 && value >= max;

  return (
    <div className={className}>
      <div
        className="progress-bar__track"
        role="progressbar"
        aria-valuenow={value}
        aria-valuemin={0}
        aria-valuemax={max}
        aria-label={label ?? `${value} of ${max}`}
      >
        <div
          className={`progress-bar__fill${isComplete ? " progress-bar__fill--complete" : ""}`}
          style={{ width: `${pct}%` }}
        />
      </div>
      {label !== undefined && (
        <p className="progress-bar__label">{label}</p>
      )}
    </div>
  );
}
