/**
 * RippleCarryDiagram — interactive 4-bit ripple-carry adder.
 *
 * Shows 4 full adders chained together, with carry rippling from LSB to MSB.
 * Uses `rippleCarryAdderTraced()` to capture per-bit snapshots, displaying
 * each adder's inputs, sum, and carry for full transparency.
 *
 * === The key insight ===
 *
 * This is the SAME circuit that will power subtraction (Tab 2) and
 * multiplication (also Tab 2). The adder is the CPU's workhorse — when
 * we subtract, we just flip the inputs; when we multiply, we add
 * shifted partial products through this same chain.
 *
 * === Layout ===
 *
 *     [BitGroup A]  (4-bit input, MSB-first display)
 *   + [BitGroup B]  (4-bit input, MSB-first display)
 *   = [Result]      (4-bit sum + decimal)
 *
 *     [4 chained full adder boxes with carry arrows]
 *     [Per-bit snapshot table]
 */

import { useState } from "react";
import { rippleCarryAdderTraced } from "@coding-adventures/arithmetic";
import type { Bit } from "@coding-adventures/logic-gates";
import { useTranslation } from "@coding-adventures/ui-components";
import { BitGroup } from "../shared/BitGroup.js";
import { WireLabel } from "../shared/WireLabel.js";

/** Convert LSB-first bit array to decimal. */
function bitsToDecimal(bits: Bit[]): number {
  return bits.reduce<number>((acc, bit, i) => acc + (bit << i), 0);
}

export function RippleCarryDiagram() {
  const { t } = useTranslation();
  const [aBits, setABits] = useState<Bit[]>([1, 0, 1, 0]); // 5
  const [bBits, setBBits] = useState<Bit[]>([1, 1, 0, 0]); // 3

  const result = rippleCarryAdderTraced(aBits, bBits);
  const decA = bitsToDecimal(aBits);
  const decB = bitsToDecimal(bBits);
  const decSum = bitsToDecimal(result.sum);

  return (
    <section className="adder-card adder-card--wide" aria-label={t("adders.ripple.ariaLabel")}>
      <h3 className="adder-card__title">{t("adders.ripple.title")}</h3>
      <p className="adder-card__description">{t("adders.ripple.description")}</p>

      {/* Operand inputs */}
      <div className="ripple__operands">
        <BitGroup bits={aBits} onChange={setABits} label="A" />
        <span className="ripple__operator">+</span>
        <BitGroup bits={bBits} onChange={setBBits} label="B" />
        <span className="ripple__operator">=</span>
        <div className="ripple__result" aria-live="polite">
          <div className="ripple__result-bits">
            {[...result.sum].reverse().map((bit, i) => (
              <span
                key={i}
                className={`ripple__result-bit ${bit ? "ripple__result-bit--high" : "ripple__result-bit--low"}`}
              >
                {bit}
              </span>
            ))}
          </div>
          <span className="ripple__result-decimal">= {decSum}</span>
        </div>
      </div>

      {/* Equation display */}
      <p className="ripple__equation" aria-live="polite">
        {decA} + {decB} = {decSum}
        {result.carryOut === 1 ? ` (+ carry)` : ""}
      </p>

      {/* Overflow indicator */}
      <p className={`ripple__overflow ${result.carryOut === 1 ? "ripple__overflow--active" : ""}`}>
        <WireLabel value={result.carryOut} label="Carry Out" />
        {" "}
        {result.carryOut === 1 ? t("adders.ripple.overflow") : t("adders.ripple.noOverflow")}
      </p>

      {/* Chain of 4 full adders (SVG) */}
      <svg
        className="ripple__chain-svg"
        viewBox="0 0 520 140"
        aria-hidden="true"
      >
        {result.adders.map((snap, i) => {
          const x = 30 + i * 120;
          const y = 20;
          return (
            <g key={i}>
              {/* Full adder box */}
              <rect
                x={x} y={y} width="90" height="90" rx="6"
                className="gate-box"
              />
              <text x={x + 45} y={y + 20} textAnchor="middle" className="gate-label">
                FA{i}
              </text>

              {/* Input labels inside box */}
              <text x={x + 10} y={y + 40} className="wire-text--dim" fontSize="9">
                A={snap.a}
              </text>
              <text x={x + 10} y={y + 52} className="wire-text--dim" fontSize="9">
                B={snap.b}
              </text>
              <text x={x + 10} y={y + 64} className="wire-text--dim" fontSize="9">
                Cin={snap.cIn}
              </text>

              {/* Sum output (below) */}
              <line x1={x + 45} y1={y + 90} x2={x + 45} y2={y + 120} className={snap.sum ? "wire--high" : "wire--low"} strokeWidth="2" />
              <text x={x + 45} y={y + 135} textAnchor="middle" className={snap.sum ? "wire-text--high" : "wire-text--low"} fontSize="11">
                S{i}={snap.sum}
              </text>

              {/* Carry output (right, into next adder) */}
              {i < 3 && (
                <line
                  x1={x + 90} y1={y + 45}
                  x2={x + 120} y2={y + 45}
                  className={snap.cOut ? "wire--high" : "wire--low"}
                  strokeWidth="2"
                  markerEnd="url(#arrowhead)"
                />
              )}

              {/* Carry-out label */}
              <text x={x + 70} y={y + 85} className={snap.cOut ? "wire-text--high" : "wire-text--low"} fontSize="8">
                C={snap.cOut}
              </text>
            </g>
          );
        })}

        {/* Arrow marker definition */}
        <defs>
          <marker id="arrowhead" markerWidth="8" markerHeight="6" refX="8" refY="3" orient="auto">
            <polygon points="0 0, 8 3, 0 6" fill="var(--tab-text)" />
          </marker>
        </defs>

        {/* Initial carry-in (0) */}
        <text x="15" y="68" className="wire-text--dim" fontSize="9">Cin=0</text>
        <line x1="20" y1="65" x2="30" y2="65" className="wire--low" strokeWidth="2" />

        {/* Final carry-out */}
        <line
          x1={30 + 3 * 120 + 90} y1={20 + 45}
          x2={30 + 3 * 120 + 110} y2={20 + 45}
          className={result.carryOut ? "wire--high" : "wire--low"}
          strokeWidth="2"
        />
        <text
          x={30 + 3 * 120 + 115} y={20 + 50}
          className={result.carryOut ? "wire-text--high" : "wire-text--low"}
          fontSize="10"
        >
          Cout={result.carryOut}
        </text>
      </svg>

      {/* Per-bit snapshot table */}
      <table className="truth-table ripple__snapshot-table">
        <caption>Per-Adder Snapshots</caption>
        <thead>
          <tr>
            <th scope="col">Bit</th>
            <th scope="col">A</th>
            <th scope="col">B</th>
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
    </section>
  );
}
