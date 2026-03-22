/**
 * Calculator keypad — the button grid.
 *
 * Modeled after the Busicom 141-PF layout. The original had rows of
 * buttons with colored function keys (orange for operators, dark for
 * digits). We replicate this with CSS classes for styling.
 *
 * All buttons are keyboard-accessible with visible focus indicators.
 */

import { useCallback } from "react";
import { useTranslation } from "../../i18n/index.js";
import { Key } from "./Key.js";
import type { KeyName } from "../../hooks/useCalculator.js";

interface KeypadProps {
  onKeyPress: (key: KeyName) => void;
}

/**
 * Keypad layout — rows of keys matching the Busicom 141-PF arrangement.
 *
 * Each key has:
 *   - name: KeyName used by the calculator hook
 *   - labelKey: i18n key for the display label
 *   - className: CSS class for styling (operator, digit, function)
 */
const KEYPAD_LAYOUT: Array<Array<{ name: KeyName; labelKey: string; className: string }>> = [
  // Row 1: Clear and operators
  [
    { name: "C", labelKey: "calculator.key.clear", className: "key--function" },
    { name: "÷", labelKey: "calculator.key.div", className: "key--operator" },
    { name: "×", labelKey: "calculator.key.mul", className: "key--operator" },
    { name: "-", labelKey: "calculator.key.sub", className: "key--operator" },
  ],
  // Row 2: 7, 8, 9, +
  [
    { name: "7", labelKey: "calculator.key.7", className: "key--digit" },
    { name: "8", labelKey: "calculator.key.8", className: "key--digit" },
    { name: "9", labelKey: "calculator.key.9", className: "key--digit" },
    { name: "+", labelKey: "calculator.key.add", className: "key--operator" },
  ],
  // Row 3: 4, 5, 6
  [
    { name: "4", labelKey: "calculator.key.4", className: "key--digit" },
    { name: "5", labelKey: "calculator.key.5", className: "key--digit" },
    { name: "6", labelKey: "calculator.key.6", className: "key--digit" },
  ],
  // Row 4: 1, 2, 3, =
  [
    { name: "1", labelKey: "calculator.key.1", className: "key--digit" },
    { name: "2", labelKey: "calculator.key.2", className: "key--digit" },
    { name: "3", labelKey: "calculator.key.3", className: "key--digit" },
    { name: "=", labelKey: "calculator.key.equals", className: "key--equals" },
  ],
  // Row 5: 0
  [
    { name: "0", labelKey: "calculator.key.0", className: "key--digit key--wide" },
  ],
];

export function Keypad({ onKeyPress }: KeypadProps) {
  const { t } = useTranslation();

  const handleKey = useCallback(
    (name: KeyName) => {
      onKeyPress(name);
    },
    [onKeyPress],
  );

  return (
    <div className="keypad" role="group" aria-label={t("layer.calculator")}>
      {KEYPAD_LAYOUT.map((row, rowIndex) => (
        <div key={rowIndex} className="keypad-row">
          {row.map((key) => (
            <Key
              key={key.name}
              name={key.name}
              label={t(key.labelKey)}
              className={key.className}
              onPress={handleKey}
            />
          ))}
        </div>
      ))}
    </div>
  );
}
