/**
 * SrLatchDiagram — interactive SR Latch visualization.
 *
 * === The birth of memory ===
 *
 * The SR (Set-Reset) Latch is where digital memory begins. Built from just
 * two cross-coupled NOR gates, it creates a feedback loop that can "latch"
 * into one of two stable states — and stay there even after the input that
 * caused it is removed.
 *
 * This is a profound leap: combinational circuits (Tabs 1-3) have no memory.
 * Their outputs depend only on current inputs. The SR latch introduces STATE —
 * the circuit remembers what happened in the past.
 *
 * === How feedback creates memory ===
 *
 *     R ---[NOR]---+--- Q
 *            ^     |
 *            |     v
 *            +---[NOR]--- Q̄
 *     S -----^
 *
 * Each NOR gate's output feeds into the other's input. This cross-coupling
 * creates two stable states:
 *   Set:   Q=1, Q̄=0  (the latch remembers a 1)
 *   Reset: Q=0, Q̄=1  (the latch remembers a 0)
 *
 * S=1,R=1 is the "forbidden" state — both outputs go to 0, violating
 * the Q/Q̄ complementary invariant.
 */

import { useState } from "react";
import type { Bit } from "@coding-adventures/logic-gates";
import { srLatch } from "@coding-adventures/logic-gates";
import { useTranslation } from "@coding-adventures/ui-components";
import { BitToggle } from "../shared/BitToggle.js";
import { WireLabel } from "../shared/WireLabel.js";

function wireColor(bit: Bit): string {
  return bit === 1 ? "#4caf50" : "#777";
}

export function SrLatchDiagram() {
  const { t } = useTranslation();
  const [s, setS] = useState<Bit>(0);
  const [r, setR] = useState<Bit>(0);
  const [q, setQ] = useState<Bit>(0);
  const [qBar, setQBar] = useState<Bit>(1);

  // Compute new outputs from current inputs and previous state
  const [newQ, newQBar] = srLatch(s, r, q, qBar);

  // Update stored state when inputs change
  const handleS = (val: Bit) => {
    setS(val);
    const [nq, nqb] = srLatch(val, r, q, qBar);
    setQ(nq);
    setQBar(nqb);
  };

  const handleR = (val: Bit) => {
    setR(val);
    const [nq, nqb] = srLatch(s, val, q, qBar);
    setQ(nq);
    setQBar(nqb);
  };

  const isForbidden = s === 1 && r === 1;

  return (
    <div className="sequential-card">
      <div className="sequential-card__header">
        <h3 className="sequential-card__title">{t("seq.srLatch.title")}</h3>
        <span className="sequential-card__badge">2 NOR gates</span>
      </div>

      <p className="sequential-card__description">{t("seq.srLatch.description")}</p>

      <div className="sequential-card__diagram">
        <div className="sequential-card__inputs">
          <BitToggle value={s} onChange={handleS} label="S" />
          <BitToggle value={r} onChange={handleR} label="R" />
        </div>

        <svg viewBox="0 0 300 140" className="sequential-card__svg" role="img" aria-label={t("seq.srLatch.ariaLabel")}>
          {/* R input wire to top NOR */}
          <line x1="0" y1="35" x2="60" y2="35" stroke={wireColor(r)} strokeWidth="2" />
          <text x="10" y="28" fill={wireColor(r)} fontSize="10" fontWeight="600">R={r}</text>

          {/* Top NOR gate */}
          <rect x="60" y="20" width="55" height="35" rx="5" fill="rgba(79,195,247,0.08)" stroke="#4fc3f7" strokeWidth="1.5" />
          <text x="88" y="42" textAnchor="middle" fill="#4fc3f7" fontSize="11" fontWeight="700">NOR</text>

          {/* Q output wire */}
          <line x1="115" y1="37" x2="270" y2="37" stroke={wireColor(newQ)} strokeWidth="2" />
          <text x="250" y="30" fill={wireColor(newQ)} fontSize="10" fontWeight="600">Q={newQ}</text>

          {/* S input wire to bottom NOR */}
          <line x1="0" y1="105" x2="60" y2="105" stroke={wireColor(s)} strokeWidth="2" />
          <text x="10" y="98" fill={wireColor(s)} fontSize="10" fontWeight="600">S={s}</text>

          {/* Bottom NOR gate */}
          <rect x="60" y="90" width="55" height="35" rx="5" fill="rgba(79,195,247,0.08)" stroke="#4fc3f7" strokeWidth="1.5" />
          <text x="88" y="112" textAnchor="middle" fill="#4fc3f7" fontSize="11" fontWeight="700">NOR</text>

          {/* Q̄ output wire */}
          <line x1="115" y1="107" x2="270" y2="107" stroke={wireColor(newQBar)} strokeWidth="2" />
          <text x="250" y="100" fill={wireColor(newQBar)} fontSize="10" fontWeight="600">Q̄={newQBar}</text>

          {/* Feedback: Q → bottom NOR input */}
          <line x1="140" y1="37" x2="140" y2="70" stroke={wireColor(newQ)} strokeWidth="1.5" strokeDasharray="4,3" />
          <line x1="140" y1="70" x2="45" y2="70" stroke={wireColor(newQ)} strokeWidth="1.5" strokeDasharray="4,3" />
          <line x1="45" y1="70" x2="45" y2="97" stroke={wireColor(newQ)} strokeWidth="1.5" strokeDasharray="4,3" />
          <line x1="45" y1="97" x2="60" y2="97" stroke={wireColor(newQ)} strokeWidth="1.5" strokeDasharray="4,3" />

          {/* Feedback: Q̄ → top NOR input */}
          <line x1="140" y1="107" x2="140" y2="70" stroke={wireColor(newQBar)} strokeWidth="1.5" strokeDasharray="4,3" opacity="0.5" />
          <line x1="155" y1="107" x2="155" y2="60" stroke={wireColor(newQBar)} strokeWidth="1.5" strokeDasharray="4,3" />
          <line x1="155" y1="60" x2="50" y2="60" stroke={wireColor(newQBar)} strokeWidth="1.5" strokeDasharray="4,3" />
          <line x1="50" y1="60" x2="50" y2="47" stroke={wireColor(newQBar)} strokeWidth="1.5" strokeDasharray="4,3" />
          <line x1="50" y1="47" x2="60" y2="47" stroke={wireColor(newQBar)} strokeWidth="1.5" strokeDasharray="4,3" />

          {/* Feedback label */}
          <text x="180" y="76" textAnchor="middle" fill="#888" fontSize="8" fontStyle="italic">feedback</text>
        </svg>

        <div className="sequential-card__outputs">
          <WireLabel value={newQ} label="Q" />
          <WireLabel value={newQBar} label="Q̄" />
        </div>
      </div>

      {/* State indicator */}
      <div className={`sequential-card__state ${isForbidden ? "sequential-card__state--forbidden" : ""}`}>
        {isForbidden
          ? t("seq.srLatch.forbidden")
          : s === 1
            ? t("seq.srLatch.setting")
            : r === 1
              ? t("seq.srLatch.resetting")
              : t("seq.srLatch.holding")}
      </div>

      {/* Truth table */}
      <table className="truth-table">
        <caption>{t("truthTable.title")}</caption>
        <thead>
          <tr>
            <th scope="col">S</th>
            <th scope="col">R</th>
            <th scope="col">Q</th>
            <th scope="col">Action</th>
          </tr>
        </thead>
        <tbody>
          {[
            { s: 0, r: 0, q: "Q", action: "Hold" },
            { s: 1, r: 0, q: "1", action: "Set" },
            { s: 0, r: 1, q: "0", action: "Reset" },
            { s: 1, r: 1, q: "?", action: "Forbidden" },
          ].map((row, idx) => {
            const isActive = s === row.s && r === row.r;
            return (
              <tr key={idx} className={isActive ? "truth-table__row--active" : ""} aria-current={isActive ? "true" : undefined}>
                <td>{row.s}</td>
                <td>{row.r}</td>
                <td>{row.q}</td>
                <td>{row.action}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
