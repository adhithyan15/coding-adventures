/**
 * MultiplicationView — shows how multiplication is shift-and-add.
 *
 * === The Algorithm ===
 *
 * For each bit of the multiplier (B):
 *   - If bit i = 1: add the multiplicand (A) shifted left by i positions
 *   - If bit i = 0: skip (add nothing)
 *
 * This is binary long multiplication — identical to the pencil-and-paper
 * method you learned in school, but simpler because each digit is only 0 or 1.
 *
 * === Interactive ===
 *
 * The user sets two 4-bit numbers, and the component shows:
 * - A long multiplication grid (like on paper)
 * - Step-by-step partial products with running total
 * - Auto-step mode to watch the algorithm proceed
 *
 * Uses `shiftAndAddMultiplier()` from the arithmetic package for traced steps.
 */

import { useState } from "react";
import { shiftAndAddMultiplier } from "@coding-adventures/arithmetic";
import type { Bit } from "@coding-adventures/logic-gates";
import { useTranslation } from "@coding-adventures/ui-components";
import { BitGroup } from "../shared/BitGroup.js";

/** Convert LSB-first bit array to decimal. */
function bitsToDecimal(bits: Bit[]): number {
  return bits.reduce<number>((acc, bit, i) => acc + (bit << i), 0);
}

export function MultiplicationView() {
  const { t } = useTranslation();
  const [aBits, setABits] = useState<Bit[]>([1, 0, 1, 0]); // 5
  const [bBits, setBBits] = useState<Bit[]>([1, 1, 0, 0]); // 3

  const result = shiftAndAddMultiplier(aBits, bBits);

  const decA = bitsToDecimal(aBits);
  const decB = bitsToDecimal(bBits);
  const decProduct = bitsToDecimal(result.product);

  // For the long multiplication grid, display MSB first
  const aMsb = [...aBits].reverse();
  const bMsb = [...bBits].reverse();
  const productMsb = [...result.product].reverse();

  return (
    <section className="addition-card" aria-label={t("addition.mul.ariaLabel")}>
      <h3 className="addition-card__title">{t("addition.mul.title")}</h3>
      <p className="addition-card__description">{t("addition.mul.description")}</p>

      {/* Operand inputs */}
      <div className="addition-card__inputs">
        <BitGroup bits={aBits} onChange={setABits} label="A" />
        <span className="mul__operator">×</span>
        <BitGroup bits={bBits} onChange={setBBits} label="B" />
      </div>

      {/* Long multiplication grid */}
      <div className="mul__grid" aria-label={t("addition.mul.gridLabel")}>
        {/* Multiplicand (A) */}
        <div className="mul__row mul__row--operand">
          <span className="mul__row-label" />
          {aMsb.map((b, i) => (
            <span key={i} className={`mul__cell ${b ? "mul__cell--high" : "mul__cell--low"}`}>
              {b}
            </span>
          ))}
          <span className="mul__row-note">({decA})</span>
        </div>

        {/* Multiplier (B) with × prefix */}
        <div className="mul__row mul__row--operand">
          <span className="mul__row-label">×</span>
          {bMsb.map((b, i) => (
            <span key={i} className={`mul__cell ${b ? "mul__cell--high" : "mul__cell--low"}`}>
              {b}
            </span>
          ))}
          <span className="mul__row-note">({decB})</span>
        </div>

        <div className="mul__divider" />

        {/* Partial products — one per multiplier bit */}
        {result.steps.map((step, i) => {
          const ppMsb = [...step.partialProduct].reverse();
          const isActive = step.multiplierBit === 1;
          return (
            <div
              key={i}
              className={`mul__row mul__row--partial ${isActive ? "mul__row--active" : "mul__row--skip"}`}
            >
              <span className="mul__row-label" />
              {ppMsb.map((b, j) => (
                <span
                  key={j}
                  className={`mul__cell ${b ? "mul__cell--high" : "mul__cell--low"}`}
                >
                  {b}
                </span>
              ))}
              <span className="mul__row-note">
                {isActive
                  ? `bit ${i} = 1: ${t("addition.mul.add")} ${decA} << ${i}`
                  : `bit ${i} = 0: ${t("addition.mul.skip")}`}
              </span>
            </div>
          );
        })}

        <div className="mul__divider" />

        {/* Final product */}
        <div className="mul__row mul__row--result">
          <span className="mul__row-label">=</span>
          {productMsb.map((b, i) => (
            <span key={i} className={`mul__cell ${b ? "mul__cell--high" : "mul__cell--low"}`}>
              {b}
            </span>
          ))}
          <span className="mul__row-note">({decProduct})</span>
        </div>
      </div>

      {/* Equation */}
      <p className="mul__equation" aria-live="polite">
        {decA} × {decB} = {decProduct}
      </p>

      {/* Step trace table */}
      <table className="truth-table">
        <caption>{t("addition.mul.stepTrace")}</caption>
        <thead>
          <tr>
            <th scope="col">{t("addition.mul.step")}</th>
            <th scope="col">{t("addition.mul.bit")}</th>
            <th scope="col">{t("addition.mul.action")}</th>
            <th scope="col">{t("addition.mul.runningTotal")}</th>
          </tr>
        </thead>
        <tbody>
          {result.steps.map((step, i) => (
            <tr
              key={i}
              className={step.multiplierBit === 1 ? "truth-table__row--active" : ""}
            >
              <td>{i}</td>
              <td>{step.multiplierBit}</td>
              <td>
                {step.multiplierBit === 1
                  ? `+${decA}<<${i} (=${bitsToDecimal(step.partialProduct)})`
                  : t("addition.mul.skip")}
              </td>
              <td>{bitsToDecimal(step.runningTotal)}</td>
            </tr>
          ))}
        </tbody>
      </table>

      {/* Key insight callout */}
      <div className="addition-card__callout">
        <p>{t("addition.mul.insight")}</p>
      </div>
    </section>
  );
}
