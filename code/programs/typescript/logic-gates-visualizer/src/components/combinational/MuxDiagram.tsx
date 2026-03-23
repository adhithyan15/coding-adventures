/**
 * MuxDiagram — interactive 2:1 Multiplexer visualization.
 *
 * === What is a MUX? ===
 *
 * A multiplexer (MUX) is a data selector — it picks one of several inputs
 * and routes it to a single output. Think of it as a railroad switch:
 * multiple tracks converge into one, and the switch selects which track
 * is connected.
 *
 * The 2:1 MUX is the simplest: two data inputs (D0, D1), one select line (S),
 * one output.
 *
 *   When S = 0: output = D0
 *   When S = 1: output = D1
 *
 * === Gate implementation ===
 *
 * Built from AND, OR, and NOT gates:
 *   output = OR(AND(D0, NOT(S)), AND(D1, S))
 *
 * When S=0, NOT(S)=1, so AND(D0, 1)=D0 passes through.
 * When S=1, AND(D1, 1)=D1 passes through.
 *
 * === Why MUXes matter ===
 *
 * MUXes are everywhere in digital design:
 * - CPU register files use MUXes to select which register to read
 * - ALUs use MUXes to select operands
 * - FPGAs are literally arrays of look-up tables, which are just big MUXes
 * - Data forwarding paths in pipelined CPUs are MUX trees
 */

import { useState } from "react";
import type { Bit } from "@coding-adventures/logic-gates";
import { mux2 } from "@coding-adventures/logic-gates";
import { useTranslation } from "@coding-adventures/ui-components";
import { BitToggle } from "../shared/BitToggle.js";
import { WireLabel } from "../shared/WireLabel.js";

function wireColor(bit: Bit): string {
  return bit === 1 ? "#4caf50" : "#777";
}

export function MuxDiagram() {
  const { t } = useTranslation();
  const [d0, setD0] = useState<Bit>(0);
  const [d1, setD1] = useState<Bit>(1);
  const [sel, setSel] = useState<Bit>(0);

  const output = mux2(d0, d1, sel);

  return (
    <div className="combinational-card">
      <div className="combinational-card__header">
        <h3 className="combinational-card__title">{t("comb.mux.title")}</h3>
      </div>

      <p className="combinational-card__description">{t("comb.mux.description")}</p>

      <div className="combinational-card__diagram">
        <div className="combinational-card__inputs">
          <BitToggle value={d0} onChange={setD0} label="D0" />
          <BitToggle value={d1} onChange={setD1} label="D1" />
          <BitToggle value={sel} onChange={setSel} label="S" />
        </div>

        <svg viewBox="0 0 280 160" className="combinational-card__svg" role="img" aria-label={t("comb.mux.ariaLabel")}>
          {/* D0 input wire */}
          <line x1="0" y1="40" x2="80" y2="40" stroke={wireColor(d0)} strokeWidth="2" />
          <text x="60" y="34" fill={wireColor(d0)} fontSize="10" fontWeight="600">{d0}</text>

          {/* D1 input wire */}
          <line x1="0" y1="100" x2="80" y2="100" stroke={wireColor(d1)} strokeWidth="2" />
          <text x="60" y="94" fill={wireColor(d1)} fontSize="10" fontWeight="600">{d1}</text>

          {/* MUX trapezoid body */}
          <polygon
            points="80,20 80,140 160,120 160,40"
            fill="rgba(79,195,247,0.08)"
            stroke="#4fc3f7"
            strokeWidth="1.5"
          />
          <text x="115" y="75" textAnchor="middle" fill="#4fc3f7" fontSize="14" fontWeight="700">MUX</text>
          <text x="115" y="90" textAnchor="middle" fill="#4fc3f7" fontSize="10">2:1</text>

          {/* D0 label inside MUX */}
          <text x="88" y="45" fill="#aaa" fontSize="9">D0</text>
          {/* D1 label inside MUX */}
          <text x="88" y="105" fill="#aaa" fontSize="9">D1</text>

          {/* Select line (bottom) */}
          <line x1="0" y1="145" x2="115" y2="145" stroke={wireColor(sel)} strokeWidth="2" />
          <line x1="115" y1="145" x2="115" y2="140" stroke={wireColor(sel)} strokeWidth="2" />
          <text x="60" y="140" fill={wireColor(sel)} fontSize="10" fontWeight="600">S={sel}</text>

          {/* Highlight which input is selected */}
          {sel === 0 && (
            <line x1="80" y1="40" x2="160" y2="80" stroke={wireColor(d0)} strokeWidth="2" strokeDasharray="4,3" opacity="0.6" />
          )}
          {sel === 1 && (
            <line x1="80" y1="100" x2="160" y2="80" stroke={wireColor(d1)} strokeWidth="2" strokeDasharray="4,3" opacity="0.6" />
          )}

          {/* Output wire */}
          <line x1="160" y1="80" x2="260" y2="80" stroke={wireColor(output)} strokeWidth="2" />
          <text x="210" y="74" fill={wireColor(output)} fontSize="10" fontWeight="600">{output}</text>
        </svg>

        <div className="combinational-card__output">
          <WireLabel value={output} label="Out" />
        </div>
      </div>

      {/* Truth table */}
      <table className="truth-table">
        <caption>{t("truthTable.title")}</caption>
        <thead>
          <tr>
            <th scope="col">S</th>
            <th scope="col">Out</th>
          </tr>
        </thead>
        <tbody>
          <tr className={sel === 0 ? "truth-table__row--active" : ""} aria-current={sel === 0 ? "true" : undefined}>
            <td>0</td>
            <td>D0</td>
          </tr>
          <tr className={sel === 1 ? "truth-table__row--active" : ""} aria-current={sel === 1 ? "true" : undefined}>
            <td>1</td>
            <td>D1</td>
          </tr>
        </tbody>
      </table>
    </div>
  );
}
