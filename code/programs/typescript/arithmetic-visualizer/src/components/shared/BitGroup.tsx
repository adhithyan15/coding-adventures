/**
 * BitGroup — a row of BitToggles representing a multi-bit number.
 *
 * Displays N toggle buttons (one per bit) arranged MSB-first (left-to-right),
 * plus a decimal value display below. This makes it easy to input binary
 * numbers while seeing their decimal equivalent.
 *
 * === Bit ordering ===
 *
 * Internally, all bit arrays in the arithmetic package use LSB-first ordering
 * (index 0 = least significant bit). But visually, humans read numbers
 * left-to-right with the most significant bit first. BitGroup handles
 * this translation: it displays MSB on the left but stores LSB at index 0.
 *
 * === Accessibility ===
 *
 * Each bit toggle has an aria-label like "Bit 3 of A: 0, click to toggle",
 * and the decimal display has an aria-live region for screen readers.
 */

import { type Bit } from "@coding-adventures/logic-gates";
import { BitToggle } from "./BitToggle.js";

export interface BitGroupProps {
  /** The bit array (LSB first, as used by the arithmetic package). */
  bits: Bit[];
  /** Called when any bit is toggled. Receives the updated full array. */
  onChange: (newBits: Bit[]) => void;
  /** Label for this group (e.g., "A", "B"). */
  label: string;
  /** Whether to show the decimal value (default: true). */
  showDecimal?: boolean;
}

/** Convert LSB-first bit array to decimal. */
function bitsToDecimal(bits: Bit[]): number {
  return bits.reduce<number>((acc, bit, i) => acc + (bit << i), 0);
}

export function BitGroup({ bits, onChange, label, showDecimal = true }: BitGroupProps) {
  const handleBitChange = (index: number, newValue: Bit) => {
    const updated = [...bits];
    updated[index] = newValue;
    onChange(updated);
  };

  // Display MSB first (reverse the array for visual ordering).
  const displayBits = [...bits].reverse();
  const decimal = bitsToDecimal(bits);

  return (
    <div className="bit-group">
      <span className="bit-group__label">{label}</span>
      <div className="bit-group__bits">
        {displayBits.map((bit, displayIndex) => {
          // Map display index (MSB first) back to storage index (LSB first).
          const storageIndex = bits.length - 1 - displayIndex;
          return (
            <BitToggle
              key={storageIndex}
              value={bit}
              onChange={(v) => handleBitChange(storageIndex, v)}
              label={`${label}${storageIndex}`}
            />
          );
        })}
      </div>
      {showDecimal && (
        <span className="bit-group__decimal" aria-live="polite">
          = {decimal}
        </span>
      )}
    </div>
  );
}
