/**
 * FullAdderDiagram — interactive full adder visualization.
 *
 * Shows how two half adders and an OR gate combine to add two bits plus a
 * carry-in. The user toggles A, B, and CarryIn, and sees intermediate values
 * flow through the circuit.
 *
 * === Circuit ===
 *
 *     A ──┐
 *         ├──→ [HA1] ──→ partialSum ──┐
 *     B ──┘                           ├──→ [HA2] ──→ Sum
 *                              Cin ───┘
 *
 *         HA1.carry ──┐
 *                     ├──→ [OR] ──→ CarryOut
 *         HA2.carry ──┘
 *
 * === Educational purpose ===
 *
 * The full adder is the building block for multi-bit addition. Understanding
 * how it chains half adders and propagates carries is essential for grasping
 * the ripple-carry adder.
 */

import { useState } from "react";
import { halfAdder, fullAdder } from "@coding-adventures/arithmetic";
import { OR, type Bit } from "@coding-adventures/logic-gates";
import { useTranslation } from "@coding-adventures/ui-components";
import { BitToggle } from "../shared/BitToggle.js";
import { WireLabel } from "../shared/WireLabel.js";
import { TruthTable } from "../shared/TruthTable.js";

/** All 8 rows of the full adder truth table (precomputed). */
const TRUTH_TABLE_ROWS = [
  { inputs: [0, 0, 0] as Bit[], outputs: [0, 0] as Bit[] },
  { inputs: [0, 0, 1] as Bit[], outputs: [1, 0] as Bit[] },
  { inputs: [0, 1, 0] as Bit[], outputs: [1, 0] as Bit[] },
  { inputs: [0, 1, 1] as Bit[], outputs: [0, 1] as Bit[] },
  { inputs: [1, 0, 0] as Bit[], outputs: [1, 0] as Bit[] },
  { inputs: [1, 0, 1] as Bit[], outputs: [0, 1] as Bit[] },
  { inputs: [1, 1, 0] as Bit[], outputs: [0, 1] as Bit[] },
  { inputs: [1, 1, 1] as Bit[], outputs: [1, 1] as Bit[] },
];

function findActiveRow(a: Bit, b: Bit, cin: Bit): number {
  return a * 4 + b * 2 + cin;
}

export function FullAdderDiagram() {
  const { t } = useTranslation();
  const [a, setA] = useState<Bit>(0);
  const [b, setB] = useState<Bit>(0);
  const [cin, setCin] = useState<Bit>(0);

  // Compute full adder using the actual function
  const [sum, cout] = fullAdder(a, b, cin);

  // Also compute intermediate values for the visualization
  const [partialSum, partialCarry] = halfAdder(a, b);
  const [, carry2] = halfAdder(partialSum, cin);
  const carryOut = OR(partialCarry, carry2);

  // Suppress lint: carryOut should match cout (it does, by construction)
  void carryOut;

  return (
    <section className="adder-card" aria-label={t("adders.full.ariaLabel")}>
      <h3 className="adder-card__title">{t("adders.full.title")}</h3>
      <p className="adder-card__description">{t("adders.full.description")}</p>

      <div className="adder-card__circuit">
        <div className="adder-card__inputs">
          <BitToggle value={a} onChange={setA} label="A" />
          <BitToggle value={b} onChange={setB} label="B" />
          <BitToggle value={cin} onChange={setCin} label="Cin" />
        </div>

        {/* SVG circuit diagram showing two half adders + OR */}
        <svg
          className="adder-card__svg"
          viewBox="0 0 400 160"
          aria-hidden="true"
        >
          {/* Input wires */}
          <line x1="10" y1="30" x2="60" y2="30" className={a ? "wire--high" : "wire--low"} strokeWidth="2" />
          <line x1="10" y1="70" x2="60" y2="70" className={b ? "wire--high" : "wire--low"} strokeWidth="2" />
          <line x1="10" y1="130" x2="170" y2="130" className={cin ? "wire--high" : "wire--low"} strokeWidth="2" />

          {/* HA1 box */}
          <rect x="60" y="15" width="60" height="70" rx="4" className="gate-box" />
          <text x="90" y="45" textAnchor="middle" className="gate-label">HA1</text>

          {/* HA1 outputs */}
          <line x1="120" y1="35" x2="170" y2="35" className={partialSum ? "wire--high" : "wire--low"} strokeWidth="2" />
          <text x="145" y="28" className="wire-text--dim" fontSize="9">pSum={partialSum}</text>

          <line x1="120" y1="65" x2="170" y2="65" className={partialCarry ? "wire--high" : "wire--low"} strokeWidth="2" />

          {/* HA1 carry goes down to OR */}
          <line x1="170" y1="65" x2="170" y2="120" className={partialCarry ? "wire--high" : "wire--low"} strokeWidth="2" />
          <line x1="170" y1="120" x2="260" y2="120" className={partialCarry ? "wire--high" : "wire--low"} strokeWidth="2" />

          {/* HA2 box */}
          <rect x="170" y="20" width="60" height="70" rx="4" className="gate-box" />
          <text x="200" y="50" textAnchor="middle" className="gate-label">HA2</text>

          {/* Cin wire into HA2 */}
          <line x1="170" y1="130" x2="170" y2="70" className={cin ? "wire--high" : "wire--low"} strokeWidth="2" />

          {/* HA2 sum output → final Sum */}
          <line x1="230" y1="35" x2="350" y2="35" className={sum ? "wire--high" : "wire--low"} strokeWidth="2" />
          <text x="355" y="40" className={sum ? "wire-text--high" : "wire-text--low"}>Sum={sum}</text>

          {/* HA2 carry → OR */}
          <line x1="230" y1="65" x2="260" y2="65" className={carry2 ? "wire--high" : "wire--low"} strokeWidth="2" />

          {/* OR gate connects HA2 carry down */}
          <line x1="260" y1="65" x2="260" y2="100" className={carry2 ? "wire--high" : "wire--low"} strokeWidth="2" />

          {/* OR box */}
          <rect x="240" y="100" width="50" height="40" rx="4" className="gate-box" />
          <text x="265" y="125" textAnchor="middle" className="gate-label">OR</text>

          {/* OR output → CarryOut */}
          <line x1="290" y1="120" x2="350" y2="120" className={cout ? "wire--high" : "wire--low"} strokeWidth="2" />
          <text x="355" y="125" className={cout ? "wire-text--high" : "wire-text--low"}>Cout={cout}</text>

          {/* Input labels */}
          <text x="5" y="25" className="wire-text--dim">A</text>
          <text x="5" y="65" className="wire-text--dim">B</text>
          <text x="5" y="125" className="wire-text--dim">Cin</text>
        </svg>

        <div className="adder-card__outputs">
          <WireLabel value={sum} label="Sum" />
          <WireLabel value={cout} label="Carry Out" />
        </div>
      </div>

      <div className="adder-card__intermediates">
        <WireLabel value={partialSum} label="HA1 Sum" />
        <WireLabel value={partialCarry} label="HA1 Carry" />
        <WireLabel value={carry2} label="HA2 Carry" />
      </div>

      <TruthTable
        inputHeaders={["A", "B", "Cin"]}
        outputHeaders={["Sum", "Cout"]}
        rows={TRUTH_TABLE_ROWS}
        activeRow={findActiveRow(a, b, cin)}
      />
    </section>
  );
}
