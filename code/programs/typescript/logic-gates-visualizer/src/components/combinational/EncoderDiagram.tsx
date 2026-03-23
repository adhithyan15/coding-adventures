/**
 * EncoderDiagram — interactive 4-to-2 Priority Encoder visualization.
 *
 * === What is a priority encoder? ===
 *
 * A priority encoder takes multiple input lines and outputs the binary index
 * of the highest-priority active input. It also produces a "valid" flag
 * indicating whether any input is active at all.
 *
 * Unlike a regular encoder (which requires exactly one active input), the
 * priority encoder handles the real-world case where multiple inputs can
 * be active simultaneously.
 *
 * === Where priority encoders are used ===
 *
 * - Interrupt controllers: when multiple hardware interrupts fire at the
 *   same time, the priority encoder picks the most important one
 * - Bus arbitration: when multiple devices request the bus, the priority
 *   encoder selects which one gets access
 * - Task schedulers: selecting the highest-priority ready task
 *
 * === This component ===
 *
 * Shows a 4-input priority encoder with:
 * - 4 toggle buttons for inputs (I0–I3)
 * - The highest-priority active input is highlighted
 * - 2-bit binary output + valid flag
 * - All values update reactively as inputs change
 */

import { useState } from "react";
import type { Bit } from "@coding-adventures/logic-gates";
import { priorityEncoder } from "@coding-adventures/logic-gates";
import { useTranslation } from "@coding-adventures/ui-components";
import { BitToggle } from "../shared/BitToggle.js";

function wireColor(bit: Bit): string {
  return bit === 1 ? "#4caf50" : "#777";
}

export function EncoderDiagram() {
  const { t } = useTranslation();
  const [inputs, setInputs] = useState<Bit[]>([0, 0, 0, 0]);

  const [binaryOut, valid] = priorityEncoder(inputs as [Bit, Bit, Bit, Bit]);

  // Find which input "won" (highest active index)
  let winner = -1;
  for (let i = inputs.length - 1; i >= 0; i--) {
    if (inputs[i] === 1) {
      winner = i;
      break;
    }
  }

  const toggleInput = (index: number, newValue: Bit) => {
    setInputs((prev) => {
      const next = [...prev];
      next[index] = newValue;
      return next;
    });
  };

  return (
    <div className="combinational-card">
      <div className="combinational-card__header">
        <h3 className="combinational-card__title">{t("comb.encoder.title")}</h3>
      </div>

      <p className="combinational-card__description">{t("comb.encoder.description")}</p>

      <div className="combinational-card__diagram">
        <div className="combinational-card__inputs combinational-card__inputs--row">
          {inputs.map((val, i) => (
            <div key={i} className={`encoder-input ${winner === i ? "encoder-input--winner" : ""}`}>
              <BitToggle
                value={val}
                onChange={(v) => toggleInput(i, v)}
                label={`I${i}`}
              />
              {winner === i && <span className="encoder-input__crown">★</span>}
            </div>
          ))}
        </div>

        <svg viewBox="0 0 300 150" className="combinational-card__svg" role="img" aria-label={t("comb.encoder.ariaLabel")}>
          {/* Input wires */}
          {inputs.map((val, i) => {
            const y = 25 + i * 32;
            const isWinner = winner === i;
            return (
              <g key={i}>
                <line x1="0" y1={y} x2="80" y2={y} stroke={wireColor(val)} strokeWidth={isWinner ? 2.5 : 1.5} />
                <text x="10" y={y - 5} fill={wireColor(val)} fontSize="9" fontWeight={isWinner ? "700" : "400"}>
                  I{i}={val}
                </text>
                {isWinner && (
                  <circle cx="75" cy={y} r="4" fill="#4caf50" opacity="0.7" />
                )}
              </g>
            );
          })}

          {/* Priority Encoder box */}
          <rect x="80" y="5" width="90" height="135" rx="6" fill="rgba(255,152,0,0.08)" stroke="#ffb74d" strokeWidth="1.5" />
          <text x="125" y="65" textAnchor="middle" fill="#ffb74d" fontSize="11" fontWeight="700">PRIORITY</text>
          <text x="125" y="80" textAnchor="middle" fill="#ffb74d" fontSize="11" fontWeight="700">ENCODER</text>
          <text x="125" y="95" textAnchor="middle" fill="#ffb74d" fontSize="10">4→2</text>

          {/* Output wires */}
          <line x1="170" y1="45" x2="260" y2="45" stroke={wireColor(binaryOut[0])} strokeWidth="2" />
          <text x="270" y="49" fill={wireColor(binaryOut[0])} fontSize="10" fontWeight="600">A0={binaryOut[0]}</text>

          <line x1="170" y1="75" x2="260" y2="75" stroke={wireColor(binaryOut[1])} strokeWidth="2" />
          <text x="270" y="79" fill={wireColor(binaryOut[1])} fontSize="10" fontWeight="600">A1={binaryOut[1]}</text>

          <line x1="170" y1="115" x2="260" y2="115" stroke={wireColor(valid)} strokeWidth="2" />
          <text x="270" y="119" fill={wireColor(valid)} fontSize="10" fontWeight="600">V={valid}</text>
        </svg>

        <div className="combinational-card__output-list">
          <span className={`encoder-result ${valid === 1 ? "encoder-result--valid" : "encoder-result--invalid"}`}>
            {valid === 1
              ? `${t("comb.encoder.winner")}: I${winner} → ${binaryOut[1]}${binaryOut[0]}`
              : t("comb.encoder.noInput")}
          </span>
        </div>
      </div>
    </div>
  );
}
