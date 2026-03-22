/**
 * Calculator component — Layer 1 (user-facing).
 *
 * A visual replica of the Busicom 141-PF printing calculator (1971).
 * Users click buttons to perform arithmetic. The display shows results
 * computed by the actual Intel 4004 gate-level simulator running in
 * the browser.
 */

import { useTranslation } from "../../i18n/index.js";
import { Display } from "./Display.js";
import { Keypad } from "./Keypad.js";
import type { CalculatorState } from "../../hooks/useCalculator.js";

interface CalculatorProps {
  calculator: CalculatorState;
}

export function Calculator({ calculator }: CalculatorProps) {
  const { t } = useTranslation();

  return (
    <section className="calculator" aria-label={t("layer.calculator")}>
      <div className="calculator-body">
        <Display digits={calculator.displayDigits} />
        <Keypad onKeyPress={calculator.pressKey} />
      </div>
    </section>
  );
}
