/**
 * DecoderDiagram — interactive 2-to-4 Decoder visualization.
 *
 * === What is a decoder? ===
 *
 * A decoder converts an N-bit binary input into a "one-hot" output:
 * exactly one of 2^N output lines goes HIGH, and all others stay LOW.
 * The binary input selects which output line is activated.
 *
 * For a 2-to-4 decoder:
 *   Input 00 → Output Y0 = 1, Y1=Y2=Y3 = 0
 *   Input 01 → Output Y1 = 1, Y0=Y2=Y3 = 0
 *   Input 10 → Output Y2 = 1, Y0=Y1=Y3 = 0
 *   Input 11 → Output Y3 = 1, Y0=Y1=Y2 = 0
 *
 * === Where decoders are used ===
 *
 * - Memory address decoding: select which memory chip/row to access
 * - Instruction decoding: the CPU's instruction decoder activates the
 *   right execution unit based on the opcode bits
 * - Display drivers: 7-segment display decoders convert binary to segment patterns
 */

import { useState } from "react";
import type { Bit } from "@coding-adventures/logic-gates";
import { decoder } from "@coding-adventures/logic-gates";
import { useTranslation } from "@coding-adventures/ui-components";
import { BitToggle } from "../shared/BitToggle.js";

function wireColor(bit: Bit): string {
  return bit === 1 ? "#4caf50" : "#777";
}

export function DecoderDiagram() {
  const { t } = useTranslation();
  const [a0, setA0] = useState<Bit>(0);
  const [a1, setA1] = useState<Bit>(0);

  // decoder() takes inputs LSB first
  const outputs = decoder([a0, a1]);

  return (
    <div className="combinational-card">
      <div className="combinational-card__header">
        <h3 className="combinational-card__title">{t("comb.decoder.title")}</h3>
      </div>

      <p className="combinational-card__description">{t("comb.decoder.description")}</p>

      <div className="combinational-card__diagram">
        <div className="combinational-card__inputs">
          <BitToggle value={a0} onChange={setA0} label="A0" />
          <BitToggle value={a1} onChange={setA1} label="A1" />
        </div>

        <svg viewBox="0 0 300 180" className="combinational-card__svg" role="img" aria-label={t("comb.decoder.ariaLabel")}>
          {/* Input wires */}
          <line x1="0" y1="60" x2="70" y2="60" stroke={wireColor(a0)} strokeWidth="2" />
          <text x="35" y="54" textAnchor="middle" fill={wireColor(a0)} fontSize="10" fontWeight="600">A0={a0}</text>
          <line x1="0" y1="120" x2="70" y2="120" stroke={wireColor(a1)} strokeWidth="2" />
          <text x="35" y="114" textAnchor="middle" fill={wireColor(a1)} fontSize="10" fontWeight="600">A1={a1}</text>

          {/* Decoder box */}
          <rect x="70" y="20" width="80" height="140" rx="6" fill="rgba(156,39,176,0.08)" stroke="#ce93d8" strokeWidth="1.5" />
          <text x="110" y="85" textAnchor="middle" fill="#ce93d8" fontSize="12" fontWeight="700">DEC</text>
          <text x="110" y="100" textAnchor="middle" fill="#ce93d8" fontSize="10">2→4</text>

          {/* Output wires */}
          {outputs.map((val, i) => {
            const y = 40 + i * 32;
            return (
              <g key={i}>
                <line x1="150" y1={y} x2="250" y2={y} stroke={wireColor(val)} strokeWidth="2" />
                <text x="260" y={y + 4} fill={wireColor(val)} fontSize="10" fontWeight="600">
                  Y{i}={val}
                </text>
                {/* Highlight active output */}
                {val === 1 && (
                  <circle cx="155" cy={y} r="4" fill="#4caf50" opacity="0.6" />
                )}
              </g>
            );
          })}
        </svg>

        <div className="combinational-card__output-list">
          {outputs.map((val, i) => (
            <span key={i} className={`decoder-output ${val === 1 ? "decoder-output--active" : ""}`}>
              Y{i}: {val}
            </span>
          ))}
        </div>
      </div>

      {/* Truth table */}
      <table className="truth-table">
        <caption>{t("truthTable.title")}</caption>
        <thead>
          <tr>
            <th scope="col">A1</th>
            <th scope="col">A0</th>
            <th scope="col">Y0</th>
            <th scope="col">Y1</th>
            <th scope="col">Y2</th>
            <th scope="col">Y3</th>
          </tr>
        </thead>
        <tbody>
          {[
            [0, 0, 1, 0, 0, 0],
            [0, 1, 0, 1, 0, 0],
            [1, 0, 0, 0, 1, 0],
            [1, 1, 0, 0, 0, 1],
          ].map((row, idx) => {
            const isActive = a1 === row[0] && a0 === row[1];
            return (
              <tr key={idx} className={isActive ? "truth-table__row--active" : ""} aria-current={isActive ? "true" : undefined}>
                {row.map((val, col) => (
                  <td key={col}>{val}</td>
                ))}
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
