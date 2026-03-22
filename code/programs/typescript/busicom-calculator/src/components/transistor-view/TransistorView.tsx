/**
 * Transistor Level view — Layer 5.
 *
 * Shows how logic gates are built from CMOS transistors.
 *
 * Every digital gate is ultimately made of two types of transistors:
 *   - PMOS (P-channel): conducts when gate is LOW (pull-up network)
 *   - NMOS (N-channel): conducts when gate is HIGH (pull-down network)
 *
 * === CMOS NAND Gate (the universal gate) ===
 *
 *        Vdd
 *         │
 *     ┌───┤ PMOS (A)  ←── conducts when A=0
 *     │   │
 *     │   ├───┤ PMOS (B)  ←── conducts when B=0
 *     │       │
 *     ├───────┤──── Output
 *     │       │
 *     │   ┌───┤ NMOS (A)  ←── conducts when A=1
 *     │   │   │
 *     │   └───┤ NMOS (B)  ←── conducts when B=1
 *             │
 *            GND
 *
 * PMOS transistors are in PARALLEL (either can pull up).
 * NMOS transistors are in SERIES (both must pull down).
 * This implements Y = NOT(A AND B) = NAND.
 *
 * All other gates are built from NAND:
 *   NOT = NAND with both inputs tied together
 *   AND = NAND followed by NOT
 *   OR  = NAND(NOT(A), NOT(B))  (De Morgan's law)
 *   XOR = combination of NANDs
 */

import { useTranslation } from "../../i18n/index.js";
import type { DetailedTrace } from "../../cpu/types.js";
import type { Bit } from "@coding-adventures/logic-gates";

interface TransistorViewProps {
  trace?: DetailedTrace | undefined;
  traceHistory?: readonly DetailedTrace[];
}

/** SVG diagram of a CMOS NAND gate */
function CmosNandDiagram({ inputA, inputB }: { inputA: Bit; inputB: Bit }) {
  const output: Bit = (inputA === 1 && inputB === 1) ? 0 : 1;
  const pmosAConducts = inputA === 0;
  const pmosBConducts = inputB === 0;
  const nmosAConducts = inputA === 1;
  const nmosBConducts = inputB === 1;

  const activeColor = "var(--wire-high)";
  const inactiveColor = "#444";

  return (
    <div className="transistor-diagram">
      <h3>CMOS NAND Gate</h3>
      <div style={{ display: "flex", alignItems: "center", gap: 24, justifyContent: "center", flexWrap: "wrap" }}>
        <svg width="200" height="280" viewBox="0 0 200 280" aria-label="CMOS NAND gate transistor diagram">
          {/* Vdd rail */}
          <line x1="60" y1="10" x2="140" y2="10" stroke={activeColor} strokeWidth="2" />
          <text x="100" y="8" fill={activeColor} fontSize="10" textAnchor="middle">Vdd (5V)</text>

          {/* PMOS A — left branch */}
          <line x1="80" y1="10" x2="80" y2="50" stroke={pmosAConducts ? activeColor : inactiveColor} strokeWidth="2" />
          <rect x="60" y="50" width="40" height="30" rx="4" fill="none"
            stroke={pmosAConducts ? activeColor : inactiveColor} strokeWidth="2" />
          <text x="80" y="70" fill="#ccd" fontSize="9" textAnchor="middle">PMOS</text>
          {/* Gate terminal */}
          <line x1="40" y1="65" x2="60" y2="65" stroke={inputA ? activeColor : inactiveColor} strokeWidth="2" />
          <text x="30" y="60" fill="#8899aa" fontSize="8" textAnchor="middle">A={inputA}</text>
          <circle cx="58" cy="65" r="3" fill="none" stroke={pmosAConducts ? activeColor : inactiveColor} strokeWidth="1.5" />
          <line x1="80" y1="80" x2="80" y2="110" stroke={pmosAConducts ? activeColor : inactiveColor} strokeWidth="2" />

          {/* PMOS B — right branch */}
          <line x1="120" y1="10" x2="120" y2="50" stroke={pmosBConducts ? activeColor : inactiveColor} strokeWidth="2" />
          <rect x="100" y="50" width="40" height="30" rx="4" fill="none"
            stroke={pmosBConducts ? activeColor : inactiveColor} strokeWidth="2" />
          <text x="120" y="70" fill="#ccd" fontSize="9" textAnchor="middle">PMOS</text>
          <line x1="150" y1="65" x2="140" y2="65" stroke={inputB ? activeColor : inactiveColor} strokeWidth="2" />
          <text x="165" y="60" fill="#8899aa" fontSize="8" textAnchor="middle">B={inputB}</text>
          <circle cx="142" cy="65" r="3" fill="none" stroke={pmosBConducts ? activeColor : inactiveColor} strokeWidth="1.5" />
          <line x1="120" y1="80" x2="120" y2="110" stroke={pmosBConducts ? activeColor : inactiveColor} strokeWidth="2" />

          {/* Connection node — output */}
          <line x1="80" y1="110" x2="120" y2="110" stroke={output ? activeColor : inactiveColor} strokeWidth="2" />
          <circle cx="100" cy="110" r="4" fill={output ? activeColor : inactiveColor} />
          <line x1="100" y1="110" x2="180" y2="110" stroke={output ? activeColor : inactiveColor} strokeWidth="2" />
          <text x="185" y="114" fill={output ? activeColor : "#555"} fontSize="10">Out={output}</text>

          {/* NMOS A — series, top */}
          <line x1="100" y1="110" x2="100" y2="140" stroke={nmosAConducts ? activeColor : inactiveColor} strokeWidth="2" />
          <rect x="80" y="140" width="40" height="30" rx="4" fill="none"
            stroke={nmosAConducts ? activeColor : inactiveColor} strokeWidth="2" />
          <text x="100" y="160" fill="#ccd" fontSize="9" textAnchor="middle">NMOS</text>
          <line x1="60" y1="155" x2="80" y2="155" stroke={inputA ? activeColor : inactiveColor} strokeWidth="2" />
          <text x="45" y="150" fill="#8899aa" fontSize="8" textAnchor="middle">A={inputA}</text>

          {/* NMOS B — series, bottom */}
          <line x1="100" y1="170" x2="100" y2="200" stroke={(nmosAConducts && nmosBConducts) ? activeColor : inactiveColor} strokeWidth="2" />
          <rect x="80" y="200" width="40" height="30" rx="4" fill="none"
            stroke={nmosBConducts ? activeColor : inactiveColor} strokeWidth="2" />
          <text x="100" y="220" fill="#ccd" fontSize="9" textAnchor="middle">NMOS</text>
          <line x1="60" y1="215" x2="80" y2="215" stroke={inputB ? activeColor : inactiveColor} strokeWidth="2" />
          <text x="45" y="210" fill="#8899aa" fontSize="8" textAnchor="middle">B={inputB}</text>

          {/* GND rail */}
          <line x1="100" y1="230" x2="100" y2="260" stroke={inactiveColor} strokeWidth="2" />
          <line x1="60" y1="260" x2="140" y2="260" stroke={inactiveColor} strokeWidth="2" />
          <text x="100" y="275" fill="#556" fontSize="10" textAnchor="middle">GND (0V)</text>
        </svg>

        <div style={{ maxWidth: 200 }}>
          <div className="transistor-pair" style={{ flexDirection: "column", gap: 8 }}>
            <div className="transistor-card" style={{ minWidth: "auto", padding: 10 }}>
              <h4>Pull-up (PMOS)</h4>
              <p style={{ fontSize: "0.7rem", color: "#8899aa" }}>
                In parallel. Either PMOS conducting pulls output HIGH.
              </p>
              <div style={{ marginTop: 6 }}>
                <span className={`transistor-status ${pmosAConducts ? "transistor-status--conducting" : "transistor-status--cutoff"}`}>
                  PMOS-A: {pmosAConducts ? "ON" : "off"}
                </span>{" "}
                <span className={`transistor-status ${pmosBConducts ? "transistor-status--conducting" : "transistor-status--cutoff"}`}>
                  PMOS-B: {pmosBConducts ? "ON" : "off"}
                </span>
              </div>
            </div>
            <div className="transistor-card" style={{ minWidth: "auto", padding: 10 }}>
              <h4>Pull-down (NMOS)</h4>
              <p style={{ fontSize: "0.7rem", color: "#8899aa" }}>
                In series. Both NMOS must conduct to pull output LOW.
              </p>
              <div style={{ marginTop: 6 }}>
                <span className={`transistor-status ${nmosAConducts ? "transistor-status--conducting" : "transistor-status--cutoff"}`}>
                  NMOS-A: {nmosAConducts ? "ON" : "off"}
                </span>{" "}
                <span className={`transistor-status ${nmosBConducts ? "transistor-status--conducting" : "transistor-status--cutoff"}`}>
                  NMOS-B: {nmosBConducts ? "ON" : "off"}
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

/** SVG diagram of a CMOS Inverter (NOT gate) */
function CmosInverterDiagram({ input }: { input: Bit }) {
  const output: Bit = input === 1 ? 0 : 1;
  const pmosConducts = input === 0;
  const nmosConducts = input === 1;
  const activeColor = "var(--wire-high)";
  const inactiveColor = "#444";

  return (
    <div className="transistor-diagram">
      <h3>CMOS Inverter (NOT Gate)</h3>
      <div style={{ display: "flex", alignItems: "center", gap: 24, justifyContent: "center", flexWrap: "wrap" }}>
        <svg width="160" height="220" viewBox="0 0 160 220" aria-label="CMOS inverter transistor diagram">
          {/* Vdd */}
          <line x1="50" y1="10" x2="110" y2="10" stroke={activeColor} strokeWidth="2" />
          <text x="80" y="8" fill={activeColor} fontSize="10" textAnchor="middle">Vdd</text>

          {/* PMOS */}
          <line x1="80" y1="10" x2="80" y2="40" stroke={pmosConducts ? activeColor : inactiveColor} strokeWidth="2" />
          <rect x="60" y="40" width="40" height="30" rx="4" fill="none"
            stroke={pmosConducts ? activeColor : inactiveColor} strokeWidth="2" />
          <text x="80" y="60" fill="#ccd" fontSize="9" textAnchor="middle">PMOS</text>
          <line x1="40" y1="55" x2="60" y2="55" stroke={input ? activeColor : inactiveColor} strokeWidth="2" />
          <circle cx="58" cy="55" r="3" fill="none" stroke={pmosConducts ? activeColor : inactiveColor} strokeWidth="1.5" />
          <text x="25" y="50" fill="#8899aa" fontSize="8" textAnchor="middle">In={input}</text>

          {/* Output node */}
          <line x1="80" y1="70" x2="80" y2="100" stroke={output ? activeColor : inactiveColor} strokeWidth="2" />
          <circle cx="80" cy="100" r="4" fill={output ? activeColor : inactiveColor} />
          <line x1="80" y1="100" x2="140" y2="100" stroke={output ? activeColor : inactiveColor} strokeWidth="2" />
          <text x="145" y="104" fill={output ? activeColor : "#555"} fontSize="10">Out={output}</text>

          {/* NMOS */}
          <line x1="80" y1="100" x2="80" y2="130" stroke={nmosConducts ? activeColor : inactiveColor} strokeWidth="2" />
          <rect x="60" y="130" width="40" height="30" rx="4" fill="none"
            stroke={nmosConducts ? activeColor : inactiveColor} strokeWidth="2" />
          <text x="80" y="150" fill="#ccd" fontSize="9" textAnchor="middle">NMOS</text>
          <line x1="40" y1="145" x2="60" y2="145" stroke={input ? activeColor : inactiveColor} strokeWidth="2" />
          <text x="25" y="140" fill="#8899aa" fontSize="8" textAnchor="middle">In={input}</text>

          {/* GND */}
          <line x1="80" y1="160" x2="80" y2="190" stroke={inactiveColor} strokeWidth="2" />
          <line x1="50" y1="190" x2="110" y2="190" stroke={inactiveColor} strokeWidth="2" />
          <text x="80" y="205" fill="#556" fontSize="10" textAnchor="middle">GND</text>
        </svg>

        <div style={{ maxWidth: 200 }}>
          <p style={{ fontSize: "0.8rem", color: "#8899aa", lineHeight: 1.6 }}>
            The simplest CMOS gate. When input is <strong style={{ color: input ? activeColor : "#555" }}>{input}</strong>:
          </p>
          <ul style={{ fontSize: "0.75rem", color: "#8899aa", paddingLeft: 16, marginTop: 8 }}>
            <li>PMOS: {pmosConducts ? <span style={{ color: activeColor }}>conducting (pulls to Vdd)</span> : "cut off"}</li>
            <li>NMOS: {nmosConducts ? <span style={{ color: activeColor }}>conducting (pulls to GND)</span> : "cut off"}</li>
            <li>Output: <strong style={{ color: output ? activeColor : "#555" }}>{output}</strong></li>
          </ul>
        </div>
      </div>
    </div>
  );
}

/** Find the most recent trace with ALU detail. */
function findLastAluTrace(history: readonly DetailedTrace[]): DetailedTrace | undefined {
  for (let i = history.length - 1; i >= 0; i--) {
    if (history[i]!.aluDetail) return history[i];
  }
  return undefined;
}

export function TransistorView({ trace, traceHistory }: TransistorViewProps) {
  const { t } = useTranslation();

  // Use ALU data if available for realistic inputs
  const aluTrace = trace?.aluDetail ? trace : findLastAluTrace(traceHistory ?? []);
  const hasAlu = !!aluTrace?.aluDetail && aluTrace.aluDetail.adders.length > 0;
  const sampleA: Bit = hasAlu ? aluTrace!.aluDetail!.adders[0]!.a : 1;
  const sampleB: Bit = hasAlu ? aluTrace!.aluDetail!.adders[0]!.b : 0;

  return (
    <section className="transistor-view" aria-label={t("transistor.title")}>
      <h2>{t("transistor.title")}</h2>
      <p>{t("transistor.description")}</p>

      {hasAlu && (
        <p style={{ fontSize: "0.8rem", color: "var(--accent)", marginBottom: 12 }}>
          Showing transistor state for ALU bit 0: A={sampleA}, B={sampleB}
        </p>
      )}

      <CmosNandDiagram inputA={sampleA} inputB={sampleB} />
      <CmosInverterDiagram input={sampleA} />

      <div style={{ marginTop: 16, padding: 16, background: "var(--panel-bg)", borderRadius: 8, border: "1px solid var(--panel-border)" }}>
        <h3 style={{ marginBottom: 8 }}>Building Gates from NAND</h3>
        <p style={{ fontSize: "0.8rem", color: "#8899aa", lineHeight: 1.6 }}>
          NAND is the <strong style={{ color: "#ccd" }}>universal gate</strong> — any digital circuit
          can be built entirely from NAND gates:
        </p>
        <ul style={{ fontSize: "0.8rem", color: "#8899aa", paddingLeft: 16, marginTop: 8, lineHeight: 1.8 }}>
          <li><strong style={{ color: "#ccd" }}>NOT</strong> = NAND(A, A)</li>
          <li><strong style={{ color: "#ccd" }}>AND</strong> = NOT(NAND(A, B))</li>
          <li><strong style={{ color: "#ccd" }}>OR</strong> = NAND(NOT(A), NOT(B))</li>
          <li><strong style={{ color: "#ccd" }}>XOR</strong> = NAND(NAND(A, NAND(A,B)), NAND(B, NAND(A,B)))</li>
        </ul>
      </div>
    </section>
  );
}
