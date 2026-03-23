/**
 * BitToggle — a clickable button that toggles between 0 and 1.
 *
 * In digital logic, every wire carries exactly one of two values: 0 or 1.
 * This component lets the user interactively set a wire's value by clicking
 * or pressing Enter/Space. The button visually reflects the current state:
 *
 *   - HIGH (1): green border, green glow, green text
 *   - LOW  (0): dim gray border and text
 *
 * === Accessibility ===
 *
 * - Keyboard: Enter or Space toggles the value
 * - Screen readers: aria-label describes the input name, current value,
 *   and available action
 * - Focus ring visible on keyboard navigation
 */

import type { Bit } from "@coding-adventures/logic-gates";

export interface BitToggleProps {
  /** The current binary value (0 or 1). */
  value: Bit;
  /** Called when the user toggles the value. Receives the NEW value. */
  onChange: (newValue: Bit) => void;
  /** Display label for this input (e.g., "A", "B"). */
  label: string;
}

export function BitToggle({ value, onChange, label }: BitToggleProps) {
  const handleClick = () => {
    onChange(value === 0 ? 1 : 0);
  };

  const ariaLabel = `Input ${label}: ${value}, click to toggle`;

  return (
    <div style={{ display: "flex", flexDirection: "column", alignItems: "center" }}>
      <button
        className={`bit-toggle ${value === 1 ? "bit-toggle--high" : "bit-toggle--low"}`}
        onClick={handleClick}
        aria-label={ariaLabel}
        type="button"
      >
        {value}
      </button>
      <span className="bit-toggle__label">{label}</span>
    </div>
  );
}
