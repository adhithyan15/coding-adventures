/**
 * FlagIndicator — displays an ALU condition flag with visual state.
 *
 * ALU condition flags are single-bit signals that describe properties of
 * the result. The CPU uses these to make branching decisions:
 *
 *   - Zero (Z): "Is the result all zeros?" → enables "if x == 0"
 *   - Carry (C): "Did unsigned overflow occur?" → result doesn't fit
 *   - Negative (N): "Is the MSB set?" → two's complement sign bit
 *   - Overflow (V): "Did signed overflow occur?" → wrong sign on result
 *
 * Each flag shows:
 *   - A colored dot (green when set, gray when clear)
 *   - The flag name and abbreviation
 *   - A brief explanation
 */

export interface FlagIndicatorProps {
  /** Short name of the flag (e.g., "Z", "C", "N", "V"). */
  abbreviation: string;
  /** Full name of the flag (e.g., "Zero"). */
  name: string;
  /** Whether the flag is currently set. */
  active: boolean;
  /** Brief explanation of what this flag means. */
  description: string;
}

export function FlagIndicator({ abbreviation, name, active, description }: FlagIndicatorProps) {
  return (
    <div
      className={`flag-indicator ${active ? "flag-indicator--active" : ""}`}
      aria-label={`${name} flag: ${active ? "set" : "clear"}`}
    >
      <span className="flag-indicator__badge">
        <span
          className={`flag-indicator__dot ${active ? "flag-indicator__dot--set" : "flag-indicator__dot--clear"}`}
        />
        <span className="flag-indicator__abbr">{abbreviation}</span>
      </span>
      <span className="flag-indicator__name">{name}</span>
      <span className="flag-indicator__desc">{description}</span>
    </div>
  );
}
