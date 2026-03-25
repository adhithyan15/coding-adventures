/**
 * ComparisonView — Tab 4: ENIAC decimal vs modern binary, side-by-side.
 *
 * Shows the same addition computed two different ways:
 * - Left: ENIAC's decimal pulse counting through ring counters
 * - Right: Modern binary ripple-carry adder
 */

import { useState } from "react";
import {
  createAccumulator,
  accumulatorAdd,
  accumulatorValue,
} from "@coding-adventures/eniac";
import { rippleCarryAdderTraced } from "@coding-adventures/arithmetic";
import type { Bit } from "@coding-adventures/logic-gates";
import { useTranslation } from "@coding-adventures/ui-components";

/** Convert integer to LSB-first bit array. */
function intToBits(n: number, width: number): Bit[] {
  return Array.from({ length: width }, (_, i) => ((n >> i) & 1) as Bit);
}

/** Convert LSB-first bit array to integer. */
function bitsToInt(bits: Bit[]): number {
  return bits.reduce<number>((acc, bit, i) => acc + (bit << i), 0);
}

export function ComparisonView() {
  const { t } = useTranslation();
  const [aVal, setAVal] = useState(42);
  const [bVal, setBVal] = useState(75);

  // ENIAC decimal path
  const acc = createAccumulator(aVal, 4);
  const eniacTrace = accumulatorAdd(acc, bVal);
  const eniacResult = accumulatorValue(eniacTrace.accumulator);
  const eniacTubes = 4 * 10; // 4 digits × 10 tubes each

  // Binary path
  const bitWidth = 14; // 14 bits can hold 0-9999
  const aBits = intToBits(aVal, bitWidth);
  const bBits = intToBits(bVal, bitWidth);
  const binaryResult = rippleCarryAdderTraced(aBits, bBits);
  const binaryValue = bitsToInt(binaryResult.sum);
  const binaryFlipFlops = bitWidth; // 1 flip-flop per bit

  return (
    <div className="comp-tab">
      <p className="comp-tab__intro">{t("comp.intro")}</p>

      <section className="eniac-card" aria-label={t("comp.ariaLabel")}>
        <h3 className="eniac-card__title">{t("comp.title")}</h3>

        {/* Operand inputs */}
        <div className="comp-inputs">
          <label className="comp-input-group">
            <span>{t("comp.operandA")}</span>
            <input
              type="number"
              min="0"
              max="4999"
              value={aVal}
              onChange={(e) => setAVal(Math.min(4999, Math.max(0, parseInt(e.target.value) || 0)))}
              className="comp-input"
            />
          </label>
          <span className="comp-plus">+</span>
          <label className="comp-input-group">
            <span>{t("comp.operandB")}</span>
            <input
              type="number"
              min="0"
              max="4999"
              value={bVal}
              onChange={(e) => setBVal(Math.min(4999, Math.max(0, parseInt(e.target.value) || 0)))}
              className="comp-input"
            />
          </label>
        </div>

        {/* Side-by-side panels */}
        <div className="comp-panels">
          {/* ENIAC decimal */}
          <div className="comp-panel comp-panel--eniac">
            <h4 className="comp-panel__title">{t("comp.eniac")}</h4>
            <div className="comp-panel__result">
              <span className="comp-panel__label">{t("comp.result")}</span>
              <span className="comp-panel__value">{eniacResult.toString().padStart(4, "0")}</span>
            </div>
            <div className="comp-panel__detail">
              <span>{t("comp.representation")}: {t("comp.tubesPerDigit")}</span>
              <span>{t("comp.method")}: {t("comp.pulseCounting")}</span>
              <span>{t("comp.tubesNeeded")}: {eniacTubes}</span>
            </div>
            {/* Per-digit carry summary */}
            <div className="comp-carry-chain">
              {eniacTrace.digitTraces.map((dt) => (
                <span
                  key={dt.position}
                  className={`comp-carry-digit ${dt.carryOut ? "comp-carry-digit--carry" : ""}`}
                >
                  {dt.digitBefore}+{dt.pulsesReceived}={dt.digitAfter}
                  {dt.carryOut ? " C" : ""}
                </span>
              ))}
            </div>
          </div>

          {/* Modern binary */}
          <div className="comp-panel comp-panel--binary">
            <h4 className="comp-panel__title">{t("comp.binary")}</h4>
            <div className="comp-panel__result">
              <span className="comp-panel__label">{t("comp.result")}</span>
              <span className="comp-panel__value">{binaryValue}</span>
            </div>
            <div className="comp-panel__detail">
              <span>{t("comp.representation")}: {t("comp.flipFlopsPerBit")}</span>
              <span>{t("comp.method")}: {t("comp.gateLogic")}</span>
              <span>{t("comp.tubesNeeded")}: {binaryFlipFlops}</span>
            </div>
            {/* Binary bits display */}
            <div className="comp-binary-bits">
              {[...binaryResult.sum].reverse().map((bit, i) => (
                <span
                  key={i}
                  className={`comp-bit ${bit ? "comp-bit--high" : "comp-bit--low"}`}
                >
                  {bit}
                </span>
              ))}
            </div>
          </div>
        </div>

        {/* Comparison table */}
        <table className="eniac-table">
          <thead>
            <tr>
              <th scope="col" />
              <th scope="col">{t("comp.eniac")}</th>
              <th scope="col">{t("comp.binary")}</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td>{t("comp.representation")}</td>
              <td>{t("comp.tubesPerDigit")}</td>
              <td>{t("comp.flipFlopsPerBit")}</td>
            </tr>
            <tr>
              <td>{t("comp.method")}</td>
              <td>{t("comp.pulseCounting")}</td>
              <td>{t("comp.gateLogic")}</td>
            </tr>
            <tr>
              <td>{t("comp.tubesNeeded")}</td>
              <td>{eniacTubes}</td>
              <td>{binaryFlipFlops}</td>
            </tr>
          </tbody>
        </table>

        <div className="eniac-callout">
          <p>{t("comp.insight")}</p>
        </div>
      </section>
    </div>
  );
}
