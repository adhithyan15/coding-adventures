/**
 * RingCounterView — Tab 2: decade ring counter visualization.
 *
 * Shows 10 vacuum tubes arranged in a ring representing one decimal digit.
 * The user can pulse the counter, set the digit directly, or auto-pulse
 * to watch it count.
 */

import { useState, useRef, useEffect, useCallback } from "react";
import {
  createDecadeCounter,
  pulseDecadeCounter,
} from "@coding-adventures/eniac";
import type { DecadeCounter } from "@coding-adventures/eniac";
import { useTranslation } from "@coding-adventures/ui-components";
import { TubeIndicator } from "../shared/TubeIndicator.js";

export function RingCounterView() {
  const { t } = useTranslation();
  const [counter, setCounter] = useState<DecadeCounter>(createDecadeCounter(0));
  const [lastCarry, setLastCarry] = useState(false);
  const [autoPulsing, setAutoPulsing] = useState(false);
  const autoRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const handlePulse = useCallback(() => {
    setCounter((prev) => {
      const result = pulseDecadeCounter(prev, 1);
      setLastCarry(result.carry);
      return result.counter;
    });
  }, []);

  const handleSetDigit = (digit: number) => {
    setCounter(createDecadeCounter(digit));
    setLastCarry(false);
  };

  const toggleAuto = () => {
    setAutoPulsing((prev) => !prev);
  };

  useEffect(() => {
    if (autoPulsing) {
      autoRef.current = setInterval(handlePulse, 400);
      return () => {
        if (autoRef.current) clearInterval(autoRef.current);
      };
    }
  }, [autoPulsing, handlePulse]);

  return (
    <div className="ring-tab">
      <p className="ring-tab__intro">{t("ring.intro")}</p>

      <section className="eniac-card" aria-label={t("ring.ariaLabel")}>
        <h3 className="eniac-card__title">{t("ring.title")}</h3>

        {/* Ring of 10 tubes */}
        <div className="ring-display">
          <div className="ring-tubes">
            {counter.tubes.map((tube) => (
              <TubeIndicator
                key={tube.position}
                label={tube.position}
                conducting={tube.conducting}
              />
            ))}
          </div>

          <div className="ring-digit" aria-live="polite">
            <span className="ring-digit__label">{t("ring.digit")}</span>
            <span className="ring-digit__value">{counter.currentDigit}</span>
          </div>

          {/* Carry indicator */}
          <div className={`ring-carry ${lastCarry ? "ring-carry--active" : ""}`} aria-live="polite">
            {lastCarry ? t("ring.carry") : t("ring.noCarry")}
          </div>
        </div>

        {/* Controls */}
        <div className="ring-controls">
          <button className="eniac-btn" onClick={handlePulse} type="button">
            {t("ring.pulse")}
          </button>
          <button
            className={`eniac-btn ${autoPulsing ? "eniac-btn--active" : ""}`}
            onClick={toggleAuto}
            type="button"
          >
            {autoPulsing ? t("ring.stop") : t("ring.autoPulse")}
          </button>
          <select
            className="ring-select"
            value={counter.currentDigit}
            onChange={(e) => handleSetDigit(Number(e.target.value))}
            aria-label={t("ring.setDigit")}
          >
            {Array.from({ length: 10 }, (_, i) => (
              <option key={i} value={i}>{i}</option>
            ))}
          </select>
        </div>

        <p className="ring-tube-count">{t("ring.tubeCount")}</p>

        <div className="eniac-callout">
          <p>{t("ring.insight")}</p>
        </div>
      </section>
    </div>
  );
}
