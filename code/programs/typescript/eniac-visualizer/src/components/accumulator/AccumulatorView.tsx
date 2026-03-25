/**
 * AccumulatorView — Tab 3: multi-digit decimal addition with carry.
 *
 * Shows 4 decade ring counters side-by-side representing a 4-digit number.
 * The user enters a number to add, and the visualization shows pulses
 * flowing through each decade with carry propagation.
 */

import { useState } from "react";
import {
  createAccumulator,
  accumulatorAdd,
  accumulatorValue,
} from "@coding-adventures/eniac";
import type { Accumulator, AdditionTrace } from "@coding-adventures/eniac";
import { useTranslation } from "@coding-adventures/ui-components";
import { TubeIndicator } from "../shared/TubeIndicator.js";

export function AccumulatorView() {
  const { t } = useTranslation();
  const [acc, setAcc] = useState<Accumulator>(createAccumulator(42, 4));
  const [addendStr, setAddendStr] = useState("75");
  const [lastTrace, setLastTrace] = useState<AdditionTrace | null>(null);

  const handleAdd = () => {
    const addend = parseInt(addendStr, 10);
    if (isNaN(addend) || addend < 0) return;
    const trace = accumulatorAdd(acc, addend);
    setAcc(trace.accumulator);
    setLastTrace(trace);
  };

  const handleReset = () => {
    setAcc(createAccumulator(0, 4));
    setLastTrace(null);
  };

  const currentValue = accumulatorValue(acc);

  return (
    <div className="acc-tab">
      <p className="acc-tab__intro">{t("acc.intro")}</p>

      <section className="eniac-card" aria-label={t("acc.ariaLabel")}>
        <h3 className="eniac-card__title">{t("acc.title")}</h3>

        {/* Current value display */}
        <div className="acc-value" aria-live="polite">
          <span className="acc-value__label">{t("acc.currentValue")}</span>
          <span className="acc-value__number">
            {currentValue.toString().padStart(4, "0")}
          </span>
        </div>

        {/* 4 decades side-by-side */}
        <div className="acc-decades">
          {[...acc.decades].reverse().map((decade, revIdx) => {
            const idx = acc.decades.length - 1 - revIdx;
            const posLabel = ["ones", "tens", "hundreds", "thousands"][idx] ?? `10^${idx}`;
            const carried = lastTrace?.carries[idx] ?? false;
            return (
              <div key={idx} className="acc-decade">
                <span className="acc-decade__pos">{posLabel}</span>
                <div className="acc-decade__tubes">
                  {decade.tubes.map((tube) => (
                    <TubeIndicator
                      key={tube.position}
                      label={tube.position}
                      conducting={tube.conducting}
                      highlight={carried && tube.conducting}
                    />
                  ))}
                </div>
                <span className="acc-decade__digit">{decade.currentDigit}</span>
                {carried && <span className="acc-decade__carry">{t("ring.carry")}</span>}
              </div>
            );
          })}
        </div>

        {/* Add controls */}
        <div className="acc-controls">
          <label className="acc-input-label" htmlFor="addend-input">
            {t("acc.addend")}
          </label>
          <input
            id="addend-input"
            type="number"
            min="0"
            max="9999"
            value={addendStr}
            onChange={(e) => setAddendStr(e.target.value)}
            className="acc-input"
          />
          <button className="eniac-btn" onClick={handleAdd} type="button">
            {t("acc.add")}
          </button>
          <button className="eniac-btn" onClick={handleReset} type="button">
            {t("acc.reset")}
          </button>
        </div>

        {/* Overflow warning */}
        {lastTrace?.overflow && (
          <p className="acc-overflow">{t("acc.overflow")}</p>
        )}

        {/* Per-digit trace table */}
        {lastTrace && (
          <table className="eniac-table">
            <caption>{t("acc.trace")}</caption>
            <thead>
              <tr>
                <th scope="col">{t("acc.digit")}</th>
                <th scope="col">{t("acc.before")}</th>
                <th scope="col">{t("acc.pulses")}</th>
                <th scope="col">{t("acc.after")}</th>
                <th scope="col">{t("acc.carryOut")}</th>
                <th scope="col">{t("acc.steps")}</th>
              </tr>
            </thead>
            <tbody>
              {lastTrace.digitTraces.map((dt) => (
                <tr key={dt.position} className={dt.carryOut ? "eniac-table__row--carry" : ""}>
                  <td>{dt.position}</td>
                  <td>{dt.digitBefore}</td>
                  <td>{dt.pulsesReceived}</td>
                  <td>{dt.digitAfter}</td>
                  <td>{dt.carryOut ? "Yes" : "No"}</td>
                  <td className="acc-steps-cell">
                    {dt.pulseResult.stepsTraced.length > 0
                      ? dt.pulseResult.stepsTraced.join("→")
                      : "—"}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}

        <div className="eniac-callout">
          <p>{t("acc.insight")}</p>
        </div>
      </section>
    </div>
  );
}
