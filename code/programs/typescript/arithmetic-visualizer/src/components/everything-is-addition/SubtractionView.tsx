/**
 * SubtractionView — shows how subtraction is just addition in disguise.
 *
 * === The Two's Complement Trick ===
 *
 * A - B = A + NOT(B) + 1
 *
 * This component walks through the transformation step by step:
 *
 *   Step 1: Start with A - B (the problem)
 *   Step 2: Negate B → NOT(B) + 1 = two's complement of B = -B
 *   Step 3: Add A + (-B) through the SAME ripple-carry adder
 *
 * The key insight: the ALU doesn't need separate subtraction hardware.
 * It just needs NOT gates on the B input and a carry-in of 1. The same
 * adder circuit handles both ADD and SUB.
 *
 * === Interactive ===
 *
 * The user sets two 4-bit numbers (A and B), and the component shows:
 * - The original subtraction problem
 * - B being negated (bit flip + add 1)
 * - The final addition with per-bit trace
 */

import { useState } from "react";
import {
  rippleCarryAdderTraced,
  twosComplementNegate,
} from "@coding-adventures/arithmetic";
import { NOT, type Bit } from "@coding-adventures/logic-gates";
import { useTranslation } from "@coding-adventures/ui-components";
import { BitGroup } from "../shared/BitGroup.js";

/** Convert LSB-first bit array to decimal (unsigned). */
function bitsToDecimal(bits: Bit[]): number {
  return bits.reduce<number>((acc, bit, i) => acc + (bit << i), 0);
}

/** Convert LSB-first bit array to signed decimal (4-bit two's complement). */
function bitsToSigned(bits: Bit[]): number {
  const unsigned = bitsToDecimal(bits);
  const n = bits.length;
  // If MSB is set, this is a negative number in two's complement
  if (bits[n - 1] === 1) {
    return unsigned - (1 << n);
  }
  return unsigned;
}

export function SubtractionView() {
  const { t } = useTranslation();
  const [aBits, setABits] = useState<Bit[]>([1, 1, 1, 0]); // 7
  const [bBits, setBBits] = useState<Bit[]>([1, 1, 0, 0]); // 3

  // Step 2: Negate B
  const notB: Bit[] = bBits.map((bit) => NOT(bit));
  const [negB] = twosComplementNegate(bBits);

  // Step 3: Add A + (-B) using the same adder
  const result = rippleCarryAdderTraced(aBits, negB);

  const decA = bitsToDecimal(aBits);
  const decB = bitsToDecimal(bBits);
  const decResult = bitsToSigned(result.sum);

  return (
    <section className="addition-card" aria-label={t("addition.sub.ariaLabel")}>
      <h3 className="addition-card__title">{t("addition.sub.title")}</h3>
      <p className="addition-card__description">{t("addition.sub.description")}</p>

      {/* Operand inputs */}
      <div className="addition-card__inputs">
        <BitGroup bits={aBits} onChange={setABits} label="A" />
        <BitGroup bits={bBits} onChange={setBBits} label="B" />
      </div>

      {/* Step 1: The problem */}
      <div className="sub-step">
        <h4 className="sub-step__title">{t("addition.sub.step1")}</h4>
        <p className="sub-step__equation">
          {decA} − {decB} = ?
        </p>
      </div>

      {/* Step 2: Negate B */}
      <div className="sub-step">
        <h4 className="sub-step__title">{t("addition.sub.step2")}</h4>
        <div className="sub-step__transform">
          <div className="sub-step__row">
            <span className="sub-step__label">B =</span>
            <span className="sub-step__bits">
              {[...bBits].reverse().map((b, i) => (
                <span key={i} className={b ? "bit--high" : "bit--low"}>{b}</span>
              ))}
            </span>
          </div>
          <div className="sub-step__row">
            <span className="sub-step__label">NOT(B) =</span>
            <span className="sub-step__bits">
              {[...notB].reverse().map((b, i) => (
                <span key={i} className={b ? "bit--high" : "bit--low"}>{b}</span>
              ))}
            </span>
          </div>
          <div className="sub-step__row">
            <span className="sub-step__label">NOT(B) + 1 =</span>
            <span className="sub-step__bits">
              {[...negB].reverse().map((b, i) => (
                <span key={i} className={b ? "bit--high" : "bit--low"}>{b}</span>
              ))}
            </span>
            <span className="sub-step__note">= −{decB} {t("addition.sub.twosComp")}</span>
          </div>
        </div>
      </div>

      {/* Step 3: Add */}
      <div className="sub-step">
        <h4 className="sub-step__title">{t("addition.sub.step3")}</h4>
        <p className="sub-step__equation" aria-live="polite">
          {decA} + (−{decB}) = {decResult}
        </p>

        {/* Per-adder trace table */}
        <table className="truth-table">
          <caption>{t("addition.sub.adderTrace")}</caption>
          <thead>
            <tr>
              <th scope="col">Bit</th>
              <th scope="col">A</th>
              <th scope="col">−B</th>
              <th scope="col">Cin</th>
              <th scope="col">Sum</th>
              <th scope="col">Cout</th>
            </tr>
          </thead>
          <tbody>
            {result.adders.map((snap, i) => (
              <tr key={i}>
                <td>{i}</td>
                <td>{snap.a}</td>
                <td>{snap.b}</td>
                <td>{snap.cIn}</td>
                <td>{snap.sum}</td>
                <td>{snap.cOut}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Key insight callout */}
      <div className="addition-card__callout">
        <p>{t("addition.sub.insight")}</p>
      </div>
    </section>
  );
}
