/**
 * DFlipFlopDiagram — interactive D Flip-Flop visualization.
 *
 * === From latch to flip-flop ===
 *
 * A D latch is "transparent" — its output follows the input while enable
 * is high. This creates problems in synchronous circuits: data can ripple
 * through multiple latches during a single clock period.
 *
 * The D Flip-Flop solves this with a master-slave design:
 *   - Master latch: transparent when clock is LOW (captures data)
 *   - Slave latch: transparent when clock is HIGH (outputs data)
 *
 * The result: data is captured at the rising edge of the clock. During
 * the HIGH period, new data cannot pass through because the master holds.
 *
 * This edge-triggered behavior is what makes synchronous digital design
 * possible — every flip-flop samples its input at exactly the same instant.
 */

import { useState, useCallback } from "react";
import type { Bit } from "@coding-adventures/logic-gates";
import type { FlipFlopState } from "@coding-adventures/logic-gates";
import { dFlipFlop } from "@coding-adventures/logic-gates";
import { useTranslation } from "@coding-adventures/ui-components";
import { BitToggle } from "../shared/BitToggle.js";
import { WireLabel } from "../shared/WireLabel.js";

function wireColor(bit: Bit): string {
  return bit === 1 ? "#4caf50" : "#777";
}

export function DFlipFlopDiagram() {
  const { t } = useTranslation();
  const [data, setData] = useState<Bit>(0);
  const [q, setQ] = useState<Bit>(0);
  const [ffState, setFfState] = useState<FlipFlopState | undefined>(undefined);
  const [lastEdge, setLastEdge] = useState<string>("—");

  // Pulse the clock: low then high (rising edge capture)
  const pulseClock = useCallback(() => {
    // Phase 1: clock LOW — master absorbs data
    const [, , state1] = dFlipFlop(data, 0, ffState);
    // Phase 2: clock HIGH — slave outputs what master captured
    const [newQ, , state2] = dFlipFlop(data, 1, state1);

    setQ(newQ);
    setFfState(state2);
    setLastEdge(`D=${data} → Q=${newQ}`);
  }, [data, ffState]);

  return (
    <div className="sequential-card">
      <div className="sequential-card__header">
        <h3 className="sequential-card__title">{t("seq.dFlipFlop.title")}</h3>
        <span className="sequential-card__badge">Master-Slave</span>
      </div>

      <p className="sequential-card__description">{t("seq.dFlipFlop.description")}</p>

      <div className="sequential-card__diagram">
        <div className="sequential-card__inputs">
          <BitToggle value={data} onChange={setData} label="D" />
          <button
            className="clock-pulse-btn"
            onClick={pulseClock}
            type="button"
            aria-label={t("seq.dFlipFlop.pulseAriaLabel")}
          >
            <span className="clock-pulse-btn__icon">⏱</span>
            <span className="clock-pulse-btn__text">{t("seq.dFlipFlop.pulse")}</span>
          </button>
        </div>

        <svg viewBox="0 0 320 120" className="sequential-card__svg" role="img" aria-label={t("seq.dFlipFlop.ariaLabel")}>
          {/* Data input wire */}
          <line x1="0" y1="40" x2="50" y2="40" stroke={wireColor(data)} strokeWidth="2" />
          <text x="10" y="33" fill={wireColor(data)} fontSize="10" fontWeight="600">D={data}</text>

          {/* Master latch */}
          <rect x="50" y="20" width="70" height="55" rx="5" fill="rgba(156,39,176,0.06)" stroke="#ce93d8" strokeWidth="1.5" />
          <text x="85" y="42" textAnchor="middle" fill="#ce93d8" fontSize="10" fontWeight="700">MASTER</text>
          <text x="85" y="55" textAnchor="middle" fill="#ce93d8" fontSize="9">D Latch</text>
          <text x="85" y="68" textAnchor="middle" fill="#888" fontSize="8">CLK̄ enable</text>

          {/* Wire between master and slave */}
          <line x1="120" y1="47" x2="160" y2="47" stroke="#888" strokeWidth="1.5" />

          {/* Slave latch */}
          <rect x="160" y="20" width="70" height="55" rx="5" fill="rgba(79,195,247,0.06)" stroke="#4fc3f7" strokeWidth="1.5" />
          <text x="195" y="42" textAnchor="middle" fill="#4fc3f7" fontSize="10" fontWeight="700">SLAVE</text>
          <text x="195" y="55" textAnchor="middle" fill="#4fc3f7" fontSize="9">D Latch</text>
          <text x="195" y="68" textAnchor="middle" fill="#888" fontSize="8">CLK enable</text>

          {/* Q output wire */}
          <line x1="230" y1="47" x2="300" y2="47" stroke={wireColor(q)} strokeWidth="2" />
          <text x="270" y="40" fill={wireColor(q)} fontSize="10" fontWeight="600">Q={q}</text>

          {/* Clock label at bottom */}
          <text x="140" y="100" textAnchor="middle" fill="#ffb74d" fontSize="10" fontWeight="600">
            ↑ Rising edge captures D into Q
          </text>
        </svg>

        <div className="sequential-card__outputs">
          <WireLabel value={q} label="Q" />
        </div>
      </div>

      {/* Last edge capture display */}
      <div className="sequential-card__state">
        {t("seq.dFlipFlop.lastCapture")}: {lastEdge}
      </div>
    </div>
  );
}
