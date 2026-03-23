/**
 * CmosPanel — expandable panel showing the CMOS transistor implementation of a gate.
 *
 * === Why show transistors? ===
 *
 * Logic gates are abstractions. Underneath, each gate is a specific arrangement
 * of CMOS transistors — tiny switches etched into silicon. This panel bridges
 * the abstraction gap by showing exactly which transistors form each gate and
 * which are ON or OFF for the current inputs.
 *
 * === CMOS fundamentals ===
 *
 * Every CMOS gate has two networks:
 *
 *   Pull-up network (PMOS transistors):
 *     Connected to Vdd (power supply). When active, pulls output HIGH.
 *     PMOS transistors turn ON when their gate input is LOW (0).
 *
 *   Pull-down network (NMOS transistors):
 *     Connected to GND (ground). When active, pulls output LOW.
 *     NMOS transistors turn ON when their gate input is HIGH (1).
 *
 * For any valid input, exactly one network is active — this is what makes
 * CMOS so power-efficient (near-zero static power consumption).
 *
 * === Gate types ===
 *
 *   NOT:  1P + 1N = 2T  — "the simplest CMOS circuit"
 *   NAND: 2P parallel + 2N series = 4T — "CMOS natural gate"
 *   AND:  NAND + NOT = 6T — "needs extra inverter"
 *   NOR:  2P series + 2N parallel = 4T — "CMOS natural gate"
 *   OR:   NOR + NOT = 6T — "needs extra inverter"
 *   XOR:  complex — 6-12T depending on implementation
 *
 * === Accessibility ===
 *
 *   - aria-expanded on the toggle button
 *   - Focus management for keyboard users
 *   - SVG diagrams have role="img" and aria-label
 */

import { useState } from "react";
import type { Bit } from "@coding-adventures/logic-gates";
import {
  CMOSInverter,
  CMOSNand,
  CMOSNor,
} from "@coding-adventures/transistors";
import { useTranslation } from "@coding-adventures/ui-components";
import type { GateType } from "./GateSymbol.js";

export interface CmosPanelProps {
  /** Which gate's CMOS implementation to show. */
  gateType: GateType;
  /** Current input A value. */
  inputA: Bit;
  /** Current input B value (ignored for NOT). */
  inputB?: Bit;
}

/** Transistor counts for each gate type. */
const TRANSISTOR_COUNTS: Record<GateType, number> = {
  not: 2,
  nand: 4,
  and: 6,
  nor: 4,
  or: 6,
  xor: 6,
};

/** Whether a gate is a "natural" CMOS gate (no extra inverter needed). */
const NATURAL_GATES: Set<GateType> = new Set(["not", "nand", "nor"]);

/**
 * SVG diagram for the NOT gate CMOS implementation.
 *
 * Circuit layout:
 *         Vdd
 *          |
 *        [P] ← PMOS
 *          |
 *   In ----+---- Out
 *          |
 *        [N] ← NMOS
 *          |
 *         GND
 */
function NotDiagram({ inputA }: { inputA: Bit }) {
  const pmosOn = inputA === 0;
  const nmosOn = inputA === 1;

  return (
    <svg viewBox="0 0 160 180" className="cmos-diagram-mini" role="img" aria-label="CMOS NOT gate: 1 PMOS and 1 NMOS transistor">
      {/* Vdd rail */}
      <line x1="80" y1="10" x2="80" y2="35" stroke="#ccc" strokeWidth="1.5" />
      <text x="80" y="8" textAnchor="middle" fill="#aaa" fontSize="11">Vdd</text>

      {/* PMOS transistor */}
      <rect x="60" y="35" width="40" height="25" rx="3" fill={pmosOn ? "rgba(76,175,80,0.2)" : "rgba(100,100,100,0.1)"} stroke={pmosOn ? "#4caf50" : "#666"} strokeWidth="1.5" />
      <text x="80" y="51" textAnchor="middle" fill={pmosOn ? "#4caf50" : "#888"} fontSize="11" fontWeight="600">P</text>
      <text x="110" y="51" fill={pmosOn ? "#4caf50" : "#888"} fontSize="9">{pmosOn ? "ON" : "OFF"}</text>

      {/* Wire between PMOS and NMOS = output node */}
      <line x1="80" y1="60" x2="80" y2="95" stroke="#ccc" strokeWidth="1.5" />

      {/* Gate input wire */}
      <line x1="15" y1="77" x2="55" y2="77" stroke={inputA === 1 ? "#4caf50" : "#777"} strokeWidth="1.5" />
      <line x1="55" y1="47" x2="55" y2="107" stroke={inputA === 1 ? "#4caf50" : "#777"} strokeWidth="1.5" strokeDasharray="3,2" />
      <text x="10" y="72" fill="#aaa" fontSize="10">In</text>

      {/* Output wire */}
      <line x1="80" y1="77" x2="145" y2="77" stroke={inputA === 0 ? "#4caf50" : "#777"} strokeWidth="1.5" />
      <text x="148" y="81" fill="#aaa" fontSize="10">Out</text>

      {/* NMOS transistor */}
      <rect x="60" y="95" width="40" height="25" rx="3" fill={nmosOn ? "rgba(76,175,80,0.2)" : "rgba(100,100,100,0.1)"} stroke={nmosOn ? "#4caf50" : "#666"} strokeWidth="1.5" />
      <text x="80" y="111" textAnchor="middle" fill={nmosOn ? "#4caf50" : "#888"} fontSize="11" fontWeight="600">N</text>
      <text x="110" y="111" fill={nmosOn ? "#4caf50" : "#888"} fontSize="9">{nmosOn ? "ON" : "OFF"}</text>

      {/* GND rail */}
      <line x1="80" y1="120" x2="80" y2="145" stroke="#ccc" strokeWidth="1.5" />
      <text x="80" y="158" textAnchor="middle" fill="#aaa" fontSize="11">GND</text>
    </svg>
  );
}

/**
 * SVG diagram for the NAND gate CMOS implementation.
 *
 * Pull-up: 2 PMOS in PARALLEL (either can pull high)
 * Pull-down: 2 NMOS in SERIES (both must be on to pull low)
 *
 *           Vdd
 *          /   \
 *        [P1] [P2]   ← parallel
 *          \   /
 *           |
 *     ------+------ Out
 *           |
 *         [N1]       ← series
 *           |
 *         [N2]
 *           |
 *          GND
 */
function NandDiagram({ inputA, inputB }: { inputA: Bit; inputB: Bit }) {
  const p1On = inputA === 0;
  const p2On = inputB === 0;
  const n1On = inputA === 1;
  const n2On = inputB === 1;

  return (
    <svg viewBox="0 0 180 220" className="cmos-diagram-mini" role="img" aria-label="CMOS NAND gate: 2 PMOS parallel, 2 NMOS series">
      {/* Vdd rail */}
      <text x="90" y="10" textAnchor="middle" fill="#aaa" fontSize="11">Vdd</text>
      <line x1="60" y1="15" x2="60" y2="35" stroke="#ccc" strokeWidth="1.5" />
      <line x1="120" y1="15" x2="120" y2="35" stroke="#ccc" strokeWidth="1.5" />
      <line x1="60" y1="15" x2="120" y2="15" stroke="#ccc" strokeWidth="1.5" />

      {/* PMOS 1 (controlled by A) — parallel */}
      <rect x="40" y="35" width="40" height="22" rx="3" fill={p1On ? "rgba(76,175,80,0.2)" : "rgba(100,100,100,0.1)"} stroke={p1On ? "#4caf50" : "#666"} strokeWidth="1.5" />
      <text x="60" y="50" textAnchor="middle" fill={p1On ? "#4caf50" : "#888"} fontSize="10" fontWeight="600">P1</text>

      {/* PMOS 2 (controlled by B) — parallel */}
      <rect x="100" y="35" width="40" height="22" rx="3" fill={p2On ? "rgba(76,175,80,0.2)" : "rgba(100,100,100,0.1)"} stroke={p2On ? "#4caf50" : "#666"} strokeWidth="1.5" />
      <text x="120" y="50" textAnchor="middle" fill={p2On ? "#4caf50" : "#888"} fontSize="10" fontWeight="600">P2</text>

      {/* Join from parallel PMOS to output */}
      <line x1="60" y1="57" x2="60" y2="75" stroke="#ccc" strokeWidth="1.5" />
      <line x1="120" y1="57" x2="120" y2="75" stroke="#ccc" strokeWidth="1.5" />
      <line x1="60" y1="75" x2="120" y2="75" stroke="#ccc" strokeWidth="1.5" />
      <line x1="90" y1="75" x2="90" y2="90" stroke="#ccc" strokeWidth="1.5" />

      {/* Output wire */}
      <line x1="90" y1="82" x2="165" y2="82" stroke={!(n1On && n2On) ? "#4caf50" : "#777"} strokeWidth="1.5" />
      <text x="168" y="86" fill="#aaa" fontSize="10">Out</text>

      {/* Input labels */}
      <text x="5" y="50" fill="#aaa" fontSize="10">A</text>
      <line x1="15" y1="47" x2="35" y2="47" stroke={inputA === 1 ? "#4caf50" : "#777"} strokeWidth="1.5" />
      <text x="5" y="140" fill="#aaa" fontSize="10">B</text>
      <line x1="15" y1="137" x2="70" y2="137" stroke={inputB === 1 ? "#4caf50" : "#777"} strokeWidth="1.5" />

      {/* NMOS 1 (controlled by A) — series */}
      <rect x="70" y="95" width="40" height="22" rx="3" fill={n1On ? "rgba(76,175,80,0.2)" : "rgba(100,100,100,0.1)"} stroke={n1On ? "#4caf50" : "#666"} strokeWidth="1.5" />
      <text x="90" y="110" textAnchor="middle" fill={n1On ? "#4caf50" : "#888"} fontSize="10" fontWeight="600">N1</text>

      {/* Wire between N1 and N2 */}
      <line x1="90" y1="117" x2="90" y2="130" stroke="#ccc" strokeWidth="1.5" />

      {/* NMOS 2 (controlled by B) — series */}
      <rect x="70" y="130" width="40" height="22" rx="3" fill={n2On ? "rgba(76,175,80,0.2)" : "rgba(100,100,100,0.1)"} stroke={n2On ? "#4caf50" : "#666"} strokeWidth="1.5" />
      <text x="90" y="145" textAnchor="middle" fill={n2On ? "#4caf50" : "#888"} fontSize="10" fontWeight="600">N2</text>

      {/* GND rail */}
      <line x1="90" y1="152" x2="90" y2="175" stroke="#ccc" strokeWidth="1.5" />
      <text x="90" y="188" textAnchor="middle" fill="#aaa" fontSize="11">GND</text>

      {/* Parallel label */}
      <text x="90" y="30" textAnchor="middle" fill="#666" fontSize="8">parallel</text>
      {/* Series label */}
      <text x="125" y="125" fill="#666" fontSize="8">series</text>
    </svg>
  );
}

/**
 * SVG diagram for the NOR gate CMOS implementation.
 *
 * Pull-up: 2 PMOS in SERIES (both must be on)
 * Pull-down: 2 NMOS in PARALLEL (either pulls low)
 */
function NorDiagram({ inputA, inputB }: { inputA: Bit; inputB: Bit }) {
  const p1On = inputA === 0;
  const p2On = inputB === 0;
  const n1On = inputA === 1;
  const n2On = inputB === 1;

  return (
    <svg viewBox="0 0 180 220" className="cmos-diagram-mini" role="img" aria-label="CMOS NOR gate: 2 PMOS series, 2 NMOS parallel">
      {/* Vdd rail */}
      <text x="90" y="10" textAnchor="middle" fill="#aaa" fontSize="11">Vdd</text>
      <line x1="90" y1="15" x2="90" y2="30" stroke="#ccc" strokeWidth="1.5" />

      {/* PMOS 1 (controlled by A) — series */}
      <rect x="70" y="30" width="40" height="22" rx="3" fill={p1On ? "rgba(76,175,80,0.2)" : "rgba(100,100,100,0.1)"} stroke={p1On ? "#4caf50" : "#666"} strokeWidth="1.5" />
      <text x="90" y="45" textAnchor="middle" fill={p1On ? "#4caf50" : "#888"} fontSize="10" fontWeight="600">P1</text>

      {/* Wire between P1 and P2 */}
      <line x1="90" y1="52" x2="90" y2="65" stroke="#ccc" strokeWidth="1.5" />

      {/* PMOS 2 (controlled by B) — series */}
      <rect x="70" y="65" width="40" height="22" rx="3" fill={p2On ? "rgba(76,175,80,0.2)" : "rgba(100,100,100,0.1)"} stroke={p2On ? "#4caf50" : "#666"} strokeWidth="1.5" />
      <text x="90" y="80" textAnchor="middle" fill={p2On ? "#4caf50" : "#888"} fontSize="10" fontWeight="600">P2</text>

      {/* Output node */}
      <line x1="90" y1="87" x2="90" y2="105" stroke="#ccc" strokeWidth="1.5" />

      {/* Output wire */}
      <line x1="90" y1="97" x2="165" y2="97" stroke={(p1On && p2On) ? "#4caf50" : "#777"} strokeWidth="1.5" />
      <text x="168" y="101" fill="#aaa" fontSize="10">Out</text>

      {/* Input labels */}
      <text x="5" y="45" fill="#aaa" fontSize="10">A</text>
      <line x1="15" y1="42" x2="65" y2="42" stroke={inputA === 1 ? "#4caf50" : "#777"} strokeWidth="1.5" />
      <text x="5" y="80" fill="#aaa" fontSize="10">B</text>
      <line x1="15" y1="77" x2="65" y2="77" stroke={inputB === 1 ? "#4caf50" : "#777"} strokeWidth="1.5" />

      {/* NMOS 1 (controlled by A) — parallel */}
      <rect x="40" y="115" width="40" height="22" rx="3" fill={n1On ? "rgba(76,175,80,0.2)" : "rgba(100,100,100,0.1)"} stroke={n1On ? "#4caf50" : "#666"} strokeWidth="1.5" />
      <text x="60" y="130" textAnchor="middle" fill={n1On ? "#4caf50" : "#888"} fontSize="10" fontWeight="600">N1</text>

      {/* NMOS 2 (controlled by B) — parallel */}
      <rect x="100" y="115" width="40" height="22" rx="3" fill={n2On ? "rgba(76,175,80,0.2)" : "rgba(100,100,100,0.1)"} stroke={n2On ? "#4caf50" : "#666"} strokeWidth="1.5" />
      <text x="120" y="130" textAnchor="middle" fill={n2On ? "#4caf50" : "#888"} fontSize="10" fontWeight="600">N2</text>

      {/* Join to output */}
      <line x1="60" y1="105" x2="120" y2="105" stroke="#ccc" strokeWidth="1.5" />
      <line x1="60" y1="105" x2="60" y2="115" stroke="#ccc" strokeWidth="1.5" />
      <line x1="120" y1="105" x2="120" y2="115" stroke="#ccc" strokeWidth="1.5" />

      {/* GND rail */}
      <line x1="60" y1="137" x2="60" y2="160" stroke="#ccc" strokeWidth="1.5" />
      <line x1="120" y1="137" x2="120" y2="160" stroke="#ccc" strokeWidth="1.5" />
      <line x1="60" y1="160" x2="120" y2="160" stroke="#ccc" strokeWidth="1.5" />
      <text x="90" y="175" textAnchor="middle" fill="#aaa" fontSize="11">GND</text>

      {/* Series label */}
      <text x="125" y="60" fill="#666" fontSize="8">series</text>
      {/* Parallel label */}
      <text x="90" y="148" textAnchor="middle" fill="#666" fontSize="8">parallel</text>
    </svg>
  );
}

/**
 * Simple text-based diagram for AND (NAND+NOT), OR (NOR+NOT), and XOR.
 * These compound gates just show a note about their construction.
 */
function CompoundDiagram({ gateType, inputA, inputB }: { gateType: GateType; inputA: Bit; inputB: Bit }) {
  const { t } = useTranslation();
  const inverter = new CMOSInverter();

  if (gateType === "and") {
    const nand = new CMOSNand();
    const nandResult = nand.evaluateDigital(inputA, inputB ?? 0);
    const andResult = inverter.evaluateDigital(nandResult);
    return (
      <div>
        <svg viewBox="0 0 220 100" className="cmos-diagram-mini" role="img" aria-label="AND gate: NAND followed by NOT inverter">
          {/* NAND stage */}
          <rect x="10" y="25" width="70" height="40" rx="4" fill="rgba(100,100,100,0.1)" stroke="#666" strokeWidth="1.5" />
          <text x="45" y="50" textAnchor="middle" fill="#aaa" fontSize="12" fontWeight="600">NAND</text>
          <text x="45" y="65" textAnchor="middle" fill="#666" fontSize="9">4T</text>

          {/* Wire between stages */}
          <line x1="80" y1="45" x2="110" y2="45" stroke={nandResult === 1 ? "#4caf50" : "#777"} strokeWidth="2" />
          <text x="95" y="40" textAnchor="middle" fill={nandResult === 1 ? "#4caf50" : "#777"} fontSize="9">{nandResult}</text>

          {/* NOT stage */}
          <rect x="110" y="30" width="50" height="30" rx="4" fill="rgba(100,100,100,0.1)" stroke="#666" strokeWidth="1.5" />
          <text x="135" y="50" textAnchor="middle" fill="#aaa" fontSize="12" fontWeight="600">NOT</text>
          <text x="135" y="62" textAnchor="middle" fill="#666" fontSize="9">2T</text>

          {/* Output */}
          <line x1="160" y1="45" x2="210" y2="45" stroke={andResult === 1 ? "#4caf50" : "#777"} strokeWidth="2" />
          <text x="195" y="40" fill={andResult === 1 ? "#4caf50" : "#777"} fontSize="9">{andResult}</text>

          {/* Inputs */}
          <text x="5" y="20" fill="#aaa" fontSize="9">A={inputA}, B={inputB ?? 0}</text>
        </svg>
        <p className="cmos-panel__note">{t("cmos.needsInverter")}</p>
      </div>
    );
  }

  if (gateType === "or") {
    const nor = new CMOSNor();
    const norResult = nor.evaluateDigital(inputA, inputB ?? 0);
    const orResult = inverter.evaluateDigital(norResult);
    return (
      <div>
        <svg viewBox="0 0 220 100" className="cmos-diagram-mini" role="img" aria-label="OR gate: NOR followed by NOT inverter">
          {/* NOR stage */}
          <rect x="10" y="25" width="70" height="40" rx="4" fill="rgba(100,100,100,0.1)" stroke="#666" strokeWidth="1.5" />
          <text x="45" y="50" textAnchor="middle" fill="#aaa" fontSize="12" fontWeight="600">NOR</text>
          <text x="45" y="65" textAnchor="middle" fill="#666" fontSize="9">4T</text>

          {/* Wire between stages */}
          <line x1="80" y1="45" x2="110" y2="45" stroke={norResult === 1 ? "#4caf50" : "#777"} strokeWidth="2" />
          <text x="95" y="40" textAnchor="middle" fill={norResult === 1 ? "#4caf50" : "#777"} fontSize="9">{norResult}</text>

          {/* NOT stage */}
          <rect x="110" y="30" width="50" height="30" rx="4" fill="rgba(100,100,100,0.1)" stroke="#666" strokeWidth="1.5" />
          <text x="135" y="50" textAnchor="middle" fill="#aaa" fontSize="12" fontWeight="600">NOT</text>
          <text x="135" y="62" textAnchor="middle" fill="#666" fontSize="9">2T</text>

          {/* Output */}
          <line x1="160" y1="45" x2="210" y2="45" stroke={orResult === 1 ? "#4caf50" : "#777"} strokeWidth="2" />
          <text x="195" y="40" fill={orResult === 1 ? "#4caf50" : "#777"} fontSize="9">{orResult}</text>

          {/* Inputs */}
          <text x="5" y="20" fill="#aaa" fontSize="9">A={inputA}, B={inputB ?? 0}</text>
        </svg>
        <p className="cmos-panel__note">{t("cmos.needsInverter")}</p>
      </div>
    );
  }

  // XOR: just show a note about complexity
  return (
    <div>
      <svg viewBox="0 0 220 70" className="cmos-diagram-mini" role="img" aria-label="XOR gate: complex multi-transistor implementation">
        <rect x="10" y="10" width="200" height="45" rx="4" fill="rgba(100,100,100,0.1)" stroke="#666" strokeWidth="1.5" />
        <text x="110" y="35" textAnchor="middle" fill="#aaa" fontSize="11">XOR: transmission-gate or 4-NAND design</text>
        <text x="110" y="50" textAnchor="middle" fill="#666" fontSize="9">6T (optimized) to 16T (4-NAND construction)</text>
      </svg>
    </div>
  );
}

export function CmosPanel({ gateType, inputA, inputB = 0 }: CmosPanelProps) {
  const [expanded, setExpanded] = useState(false);
  const { t } = useTranslation();
  const count = TRANSISTOR_COUNTS[gateType];
  const isNatural = NATURAL_GATES.has(gateType);

  return (
    <div className="cmos-panel">
      <button
        className="cmos-panel__toggle"
        onClick={() => setExpanded(!expanded)}
        aria-expanded={expanded}
        type="button"
      >
        <span className={`cmos-panel__arrow ${expanded ? "cmos-panel__arrow--expanded" : ""}`}>
          {"\u25B6"}
        </span>
        <span>{expanded ? t("cmos.hideTransistors") : t("cmos.showTransistors")}</span>
        <span className="cmos-panel__badge">
          {count} {t("cmos.transistorCount")}
        </span>
      </button>

      {expanded && (
        <div className="cmos-panel__content">
          {gateType === "not" && <NotDiagram inputA={inputA} />}
          {gateType === "nand" && <NandDiagram inputA={inputA} inputB={inputB} />}
          {gateType === "nor" && <NorDiagram inputA={inputA} inputB={inputB} />}
          {(gateType === "and" || gateType === "or" || gateType === "xor") && (
            <CompoundDiagram gateType={gateType} inputA={inputA} inputB={inputB} />
          )}
          {isNatural && (
            <p className="cmos-panel__note cmos-panel__note--natural">
              {t("cmos.naturalGate")}
            </p>
          )}
        </div>
      )}
    </div>
  );
}
