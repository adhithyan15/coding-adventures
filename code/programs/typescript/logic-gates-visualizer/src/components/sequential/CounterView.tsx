/**
 * CounterView — interactive 4-bit binary counter visualization.
 *
 * === What is a counter? ===
 *
 * A counter is a register that increments its own value on each clock
 * pulse. It combines storage (register = N flip-flops in parallel) with
 * arithmetic (incrementer = chain of half-adders with carry_in=1).
 *
 * A 4-bit counter counts: 0000 → 0001 → 0010 → ... → 1111 → 0000 (wraps)
 *
 * === Where counters are used ===
 *
 * - Program counters (PC): track which instruction to fetch next
 * - Pipeline stage counters: track where each instruction is in the pipeline
 * - Timer/clock dividers: divide a fast clock into slower ones
 * - Loop iteration counting in hardware state machines
 *
 * === This component ===
 *
 * Shows a 4-bit counter with:
 * - Manual clock pulse button (step one count)
 * - Auto-step mode (continuously count at adjustable speed)
 * - Reset button (back to 0000)
 * - Visual display of all 4 bits with decimal equivalent
 */

import { useState, useCallback, useRef, useEffect } from "react";
import type { Bit } from "@coding-adventures/logic-gates";
import type { CounterState } from "@coding-adventures/logic-gates";
import { counter } from "@coding-adventures/logic-gates";
import { useTranslation } from "@coding-adventures/ui-components";

const WIDTH = 4;

/** Convert bit array (LSB first) to decimal number. */
function bitsToDecimal(bits: Bit[]): number {
  let val = 0;
  for (let i = bits.length - 1; i >= 0; i--) {
    val = (val << 1) | bits[i];
  }
  return val;
}

export function CounterView() {
  const { t } = useTranslation();
  const [bits, setBits] = useState<Bit[]>(Array(WIDTH).fill(0) as Bit[]);
  const [cState, setCState] = useState<CounterState | undefined>(undefined);
  const [autoStep, setAutoStep] = useState(false);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Step the counter once: clock LOW then HIGH
  const step = useCallback(() => {
    // Clock LOW: master latches absorb
    const [, state1] = counter(0, 0, cState, WIDTH);
    // Clock HIGH: slave latches output, value updates
    const [newBits, state2] = counter(1, 0, state1, WIDTH);
    setBits(newBits);
    setCState(state2);
  }, [cState]);

  // Reset the counter to 0000
  const reset = useCallback(() => {
    const [, state1] = counter(0, 1, cState, WIDTH);
    const [newBits, state2] = counter(1, 1, state1, WIDTH);
    setBits(newBits);
    setCState(state2);
    setAutoStep(false);
  }, [cState]);

  // Auto-step: count automatically
  useEffect(() => {
    if (autoStep) {
      intervalRef.current = setInterval(() => {
        step();
      }, 500);
    } else if (intervalRef.current) {
      clearInterval(intervalRef.current);
      intervalRef.current = null;
    }

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
  }, [autoStep, step]);

  const decimal = bitsToDecimal(bits);
  // Display bits MSB first (reversed from internal LSB-first storage)
  const displayBits = [...bits].reverse();

  return (
    <div className="sequential-card">
      <div className="sequential-card__header">
        <h3 className="sequential-card__title">{t("seq.counter.title")}</h3>
        <span className="sequential-card__badge">{WIDTH}-bit</span>
      </div>

      <p className="sequential-card__description">{t("seq.counter.description")}</p>

      <div className="counter-display">
        {/* Bit cells (MSB → LSB) */}
        <div className="counter-display__bits" role="group" aria-label={t("seq.counter.bitsAriaLabel")}>
          {displayBits.map((bit, i) => (
            <div key={i} className={`counter-bit ${bit === 1 ? "counter-bit--high" : "counter-bit--low"}`}>
              <span className="counter-bit__label">B{WIDTH - 1 - i}</span>
              <span className="counter-bit__value">{bit}</span>
            </div>
          ))}
        </div>

        {/* Decimal value */}
        <div className="counter-display__decimal" aria-live="polite">
          <span className="counter-display__decimal-label">=</span>
          <span className="counter-display__decimal-value">{decimal}</span>
          <span className="counter-display__decimal-max">/ {(1 << WIDTH) - 1}</span>
        </div>
      </div>

      {/* Controls */}
      <div className="counter-controls">
        <button
          className="counter-controls__btn counter-controls__btn--step"
          onClick={step}
          type="button"
          aria-label={t("seq.counter.stepAriaLabel")}
        >
          {t("seq.counter.step")}
        </button>

        <button
          className={`counter-controls__btn counter-controls__btn--auto ${autoStep ? "counter-controls__btn--active" : ""}`}
          onClick={() => setAutoStep(!autoStep)}
          type="button"
          aria-pressed={autoStep}
          aria-label={t("seq.counter.autoAriaLabel")}
        >
          {autoStep ? t("seq.counter.stop") : t("seq.counter.auto")}
        </button>

        <button
          className="counter-controls__btn counter-controls__btn--reset"
          onClick={reset}
          type="button"
          aria-label={t("seq.counter.resetAriaLabel")}
        >
          {t("seq.counter.reset")}
        </button>
      </div>
    </div>
  );
}
