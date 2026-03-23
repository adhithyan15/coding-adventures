/**
 * WireLabel — a small inline indicator showing a wire's current value.
 *
 * In digital circuits, every wire carries either 0 (LOW) or 1 (HIGH).
 * This component provides a compact visual representation:
 *
 *   - A colored dot: green for HIGH, gray for LOW
 *   - The numeric value: "1" or "0"
 */

import type { Bit } from "@coding-adventures/logic-gates";

export interface WireLabelProps {
  /** The current binary value on this wire. */
  value: Bit;
  /** Optional text label (e.g., "Sum", "Carry"). */
  label?: string;
}

export function WireLabel({ value, label }: WireLabelProps) {
  const ariaLabel = label
    ? `${label}: ${value}`
    : `Output: ${value}`;

  return (
    <span className="wire-label" aria-label={ariaLabel}>
      <span
        className={`wire-label__dot ${value === 1 ? "wire-label__dot--high" : "wire-label__dot--low"}`}
      />
      <span className={value === 1 ? "wire--high" : "wire--low"}>
        {label ? `${label}: ${value}` : String(value)}
      </span>
    </span>
  );
}
