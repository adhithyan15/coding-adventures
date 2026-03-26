/**
 * Individual calculator key button.
 *
 * Each key is a semantic <button> element with proper ARIA labeling.
 * Keyboard-accessible: Enter and Space activate the key.
 * Visual feedback on press via CSS :active state.
 */

import { useCallback } from "react";
import type { KeyName } from "../../hooks/useCalculator.js";

interface KeyProps {
  /** Key identifier used by the calculator hook. */
  name: KeyName;
  /** Display label (from i18n). */
  label: string;
  /** CSS class for styling variant (digit, operator, function). */
  className: string;
  /** Callback when key is pressed. */
  onPress: (name: KeyName) => void;
}

export function Key({ name, label, className, onPress }: KeyProps) {
  const handleClick = useCallback(() => {
    onPress(name);
  }, [name, onPress]);

  return (
    <button
      type="button"
      className={`key ${className}`}
      onClick={handleClick}
      aria-label={label}
    >
      {label}
    </button>
  );
}
