/**
 * Calculator component — Layer 1 (user-facing).
 *
 * A visual replica of the Busicom 141-PF printing calculator (1971).
 * Users click buttons to perform arithmetic. The display shows results
 * computed by the actual Intel 4004 gate-level simulator running in
 * the browser.
 *
 * Keyboard shortcuts: type digits (0-9), operators (+, -, *, /),
 * Enter or = for equals, Escape or c for clear.
 */

import { useEffect } from "react";
import { useTranslation } from "../../i18n/index.js";
import { Display } from "./Display.js";
import { Keypad } from "./Keypad.js";
import type { CalculatorState, KeyName } from "../../hooks/useCalculator.js";

/** Map physical keyboard keys to calculator key names. */
const KEYBOARD_MAP: Record<string, KeyName> = {
  "0": "0", "1": "1", "2": "2", "3": "3", "4": "4",
  "5": "5", "6": "6", "7": "7", "8": "8", "9": "9",
  "+": "+", "-": "-", "*": "×", "/": "÷",
  "=": "=", "Enter": "=",
  "Escape": "C", "c": "C", "C": "C",
};

interface CalculatorProps {
  calculator: CalculatorState;
}

export function Calculator({ calculator }: CalculatorProps) {
  const { t } = useTranslation();

  // Global keyboard handler for calculator shortcuts
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      const keyName = KEYBOARD_MAP[e.key];
      if (keyName) {
        e.preventDefault();
        calculator.pressKey(keyName);
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [calculator]);

  return (
    <section className="calculator" aria-label={t("layer.calculator")}>
      <div className="calculator-body">
        <Display digits={calculator.displayDigits} />
        <Keypad onKeyPress={calculator.pressKey} />
      </div>
    </section>
  );
}
