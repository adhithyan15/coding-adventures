/**
 * 7-segment LED display — CSS only, no images.
 *
 * === How 7-segment displays work ===
 *
 * Each digit is made of 7 segments labeled A through G:
 *
 *      ─── A ───
 *     │         │
 *     F         B
 *     │         │
 *      ─── G ───
 *     │         │
 *     E         C
 *     │         │
 *      ─── D ───
 *
 * Each BCD digit (0-9) lights a specific combination of segments:
 *
 *   0: A B C D E F    (all except G)
 *   1: B C             (right side only)
 *   2: A B D E G      (top, top-right, middle, bottom-left, bottom)
 *   3: A B C D G      (top, right, middle, bottom)
 *   4: B C F G        (right, left-top, middle)
 *   5: A C D F G      (top, bottom-right, bottom, left-top, middle)
 *   6: A C D E F G    (all except B)
 *   7: A B C          (top, right)
 *   8: A B C D E F G  (all segments)
 *   9: A B C D F G    (all except E)
 *
 * The Busicom 141-PF had green LED displays. We use CSS to draw
 * each segment as a positioned element with a glow effect.
 */

import { useMemo } from "react";
import { useTranslation } from "../../i18n/index.js";

/**
 * Which segments are active for each digit (0-9).
 * Array index = digit value, each entry = array of active segment letters.
 */
const SEGMENT_MAP: Record<number, string[]> = {
  0: ["a", "b", "c", "d", "e", "f"],
  1: ["b", "c"],
  2: ["a", "b", "d", "e", "g"],
  3: ["a", "b", "c", "d", "g"],
  4: ["b", "c", "f", "g"],
  5: ["a", "c", "d", "f", "g"],
  6: ["a", "c", "d", "e", "f", "g"],
  7: ["a", "b", "c"],
  8: ["a", "b", "c", "d", "e", "f", "g"],
  9: ["a", "b", "c", "d", "f", "g"],
};

/** All possible segments. */
const ALL_SEGMENTS = ["a", "b", "c", "d", "e", "f", "g"];

interface SevenSegmentDigitProps {
  value: number;
  position: number;
}

/**
 * A single 7-segment digit display.
 *
 * Each segment is a CSS element positioned absolutely within the digit box.
 * Active segments get the "segment--on" class for the glowing green effect.
 */
function SevenSegmentDigit({ value, position }: SevenSegmentDigitProps) {
  const activeSegments = SEGMENT_MAP[value] ?? [];

  return (
    <div
      className="seven-segment-digit"
      aria-hidden="true"
      data-position={position}
    >
      {ALL_SEGMENTS.map((seg) => (
        <div
          key={seg}
          className={`segment segment-${seg} ${activeSegments.includes(seg) ? "segment--on" : "segment--off"}`}
        />
      ))}
    </div>
  );
}

interface DisplayProps {
  /** 13 BCD digits, LSB first (index 0 = rightmost digit on display). */
  digits: number[];
}

/**
 * 13-digit 7-segment LED display panel.
 *
 * Renders digits from left (MSB, index 12) to right (LSB, index 0).
 * Digits that are 0 and to the left of the first nonzero digit are
 * rendered as blank (leading zero suppression).
 */
export function Display({ digits }: DisplayProps) {
  const { t } = useTranslation();

  // Reverse so display is MSB-left, LSB-right
  const displayDigits = useMemo(() => {
    const reversed = [...digits].reverse();
    // Find first nonzero digit for leading zero suppression
    const firstNonZero = reversed.findIndex((d) => d !== 0);
    return reversed.map((d, i) => ({
      value: d,
      blank: firstNonZero === -1 ? i < reversed.length - 1 : i < firstNonZero,
    }));
  }, [digits]);

  // Compute the displayed number as text for screen readers
  const displayText = useMemo(() => {
    const nonZeroStart = displayDigits.findIndex((d) => !d.blank);
    if (nonZeroStart === -1) return "0";
    return displayDigits
      .slice(nonZeroStart)
      .map((d) => d.value.toString())
      .join("");
  }, [displayDigits]);

  return (
    <div
      className="display"
      role="status"
      aria-live="polite"
      aria-label={`${t("calculator.display.label")}: ${displayText}`}
    >
      <div className="display-glass">
        {displayDigits.map((d, i) => (
          <SevenSegmentDigit
            key={i}
            value={d.blank ? -1 : d.value}
            position={i}
          />
        ))}
      </div>
      {/* Screen-reader-only text */}
      <span className="sr-only">{displayText}</span>
    </div>
  );
}
