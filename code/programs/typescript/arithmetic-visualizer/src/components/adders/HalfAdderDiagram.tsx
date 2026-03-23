/**
 * HalfAdderDiagram — interactive half adder visualization.
 *
 * Shows the simplest arithmetic circuit: two gates (XOR for sum, AND for carry)
 * wired to two input bits. The user toggles A and B and sees the sum and carry
 * change in real time.
 *
 * === Circuit ===
 *
 *     A ──┬──→ [XOR] ──→ Sum
 *         │
 *     B ──┼──→ [AND] ──→ Carry
 *         │
 *
 * Sum = XOR(A, B): 1 when inputs differ
 * Carry = AND(A, B): 1 only when both are 1
 *
 * === Educational purpose ===
 *
 * This is addition in its most primitive form. Understanding why XOR gives the
 * sum bit and AND gives the carry is the key insight: XOR is "addition without
 * carrying" and AND detects "when carrying is needed."
 */

import { useState } from "react";
import { halfAdder } from "@coding-adventures/arithmetic";
import { useTranslation } from "@coding-adventures/ui-components";
import type { Bit } from "@coding-adventures/logic-gates";
import { BitToggle } from "../shared/BitToggle.js";
import { WireLabel } from "../shared/WireLabel.js";
import { TruthTable } from "../shared/TruthTable.js";

/** All 4 rows of the half adder truth table (precomputed). */
const TRUTH_TABLE_ROWS = [
  { inputs: [0, 0] as Bit[], outputs: [0, 0] as Bit[] },
  { inputs: [0, 1] as Bit[], outputs: [1, 0] as Bit[] },
  { inputs: [1, 0] as Bit[], outputs: [1, 0] as Bit[] },
  { inputs: [1, 1] as Bit[], outputs: [0, 1] as Bit[] },
];

/** Find the active truth table row given current inputs. */
function findActiveRow(a: Bit, b: Bit): number {
  return a * 2 + b;
}

export function HalfAdderDiagram() {
  const { t } = useTranslation();
  const [a, setA] = useState<Bit>(0);
  const [b, setB] = useState<Bit>(0);

  const [sum, carry] = halfAdder(a, b);

  return (
    <section className="adder-card" aria-label={t("adders.half.ariaLabel")}>
      <h3 className="adder-card__title">{t("adders.half.title")}</h3>
      <p className="adder-card__description">{t("adders.half.description")}</p>

      <div className="adder-card__circuit">
        <div className="adder-card__inputs">
          <BitToggle value={a} onChange={setA} label="A" />
          <BitToggle value={b} onChange={setB} label="B" />
        </div>

        {/* SVG circuit diagram */}
        <svg
          className="adder-card__svg"
          viewBox="0 0 280 120"
          aria-hidden="true"
        >
          {/* Input wires */}
          <line x1="10" y1="30" x2="80" y2="30" className={a ? "wire--high" : "wire--low"} strokeWidth="2" />
          <line x1="10" y1="90" x2="80" y2="90" className={b ? "wire--high" : "wire--low"} strokeWidth="2" />

          {/* XOR gate box */}
          <rect x="80" y="10" width="60" height="40" rx="4" className="gate-box" />
          <text x="110" y="35" textAnchor="middle" className="gate-label">XOR</text>

          {/* AND gate box */}
          <rect x="80" y="70" width="60" height="40" rx="4" className="gate-box" />
          <text x="110" y="95" textAnchor="middle" className="gate-label">AND</text>

          {/* Branch wires from inputs to both gates */}
          <line x1="40" y1="30" x2="40" y2="90" className={`${a && b ? "wire--high" : "wire--low"}`} strokeWidth="1" strokeDasharray="3,2" />

          {/* Output wires */}
          <line x1="140" y1="30" x2="210" y2="30" className={sum ? "wire--high" : "wire--low"} strokeWidth="2" />
          <line x1="140" y1="90" x2="210" y2="90" className={carry ? "wire--high" : "wire--low"} strokeWidth="2" />

          {/* Output labels */}
          <text x="225" y="35" className={sum ? "wire-text--high" : "wire-text--low"}>Sum={sum}</text>
          <text x="225" y="95" className={carry ? "wire-text--high" : "wire-text--low"}>Carry={carry}</text>

          {/* Input labels */}
          <text x="5" y="25" className="wire-text--dim">A</text>
          <text x="5" y="85" className="wire-text--dim">B</text>
        </svg>

        <div className="adder-card__outputs">
          <WireLabel value={sum} label="Sum" />
          <WireLabel value={carry} label="Carry" />
        </div>
      </div>

      <TruthTable
        inputHeaders={["A", "B"]}
        outputHeaders={["Sum", "Carry"]}
        rows={TRUTH_TABLE_ROWS}
        activeRow={findActiveRow(a, b)}
      />
    </section>
  );
}
