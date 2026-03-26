/**
 * Gate Level view — Layer 4.
 *
 * Shows individual logic gate activations for the current operation.
 * Includes IEEE standard SVG gate symbols with wire values color-coded.
 *
 * When the last instruction was an ALU operation, we show the full adder
 * internal gates: 2 XOR + 2 AND + 1 OR that make up each full adder.
 *
 *   Full adder internals:
 *
 *     A ──┬── XOR ──┬── XOR ── Sum
 *     B ──┘    │    │
 *              │   Cin
 *     A ──┬── AND   │
 *     B ──┘    │    │
 *              ├── OR ── Cout
 *    P  ──┬── AND
 *   Cin ──┘
 *
 *   Where P = A XOR B (partial sum from first XOR)
 */

import { useTranslation } from "../../i18n/index.js";
import type { DetailedTrace, FullAdderState } from "../../cpu/types.js";
import type { Bit } from "@coding-adventures/logic-gates";

interface GateViewProps {
  trace: DetailedTrace | undefined;
  traceHistory: readonly DetailedTrace[];
}

/** SVG for AND gate (IEEE symbol) */
function AndGateSvg({ active }: { active?: boolean }) {
  const color = active ? "var(--wire-high)" : "#555";
  return (
    <svg width="48" height="36" viewBox="0 0 48 36" aria-label="AND gate">
      <path
        d="M4 4 L24 4 C36 4 44 12 44 18 C44 24 36 32 24 32 L4 32 Z"
        fill="none" stroke={color} strokeWidth="2"
      />
      <line x1="0" y1="12" x2="4" y2="12" stroke={color} strokeWidth="2" />
      <line x1="0" y1="24" x2="4" y2="24" stroke={color} strokeWidth="2" />
      <line x1="44" y1="18" x2="48" y2="18" stroke={color} strokeWidth="2" />
    </svg>
  );
}

/** SVG for OR gate */
function OrGateSvg({ active }: { active?: boolean }) {
  const color = active ? "var(--wire-high)" : "#555";
  return (
    <svg width="48" height="36" viewBox="0 0 48 36" aria-label="OR gate">
      <path
        d="M4 4 Q14 18 4 32 Q24 32 44 18 Q24 4 4 4 Z"
        fill="none" stroke={color} strokeWidth="2"
      />
      <line x1="0" y1="12" x2="8" y2="12" stroke={color} strokeWidth="2" />
      <line x1="0" y1="24" x2="8" y2="24" stroke={color} strokeWidth="2" />
      <line x1="44" y1="18" x2="48" y2="18" stroke={color} strokeWidth="2" />
    </svg>
  );
}

/** SVG for XOR gate */
function XorGateSvg({ active }: { active?: boolean }) {
  const color = active ? "var(--wire-high)" : "#555";
  return (
    <svg width="48" height="36" viewBox="0 0 48 36" aria-label="XOR gate">
      <path
        d="M8 4 Q18 18 8 32 Q28 32 44 18 Q28 4 8 4 Z"
        fill="none" stroke={color} strokeWidth="2"
      />
      <path d="M4 4 Q14 18 4 32" fill="none" stroke={color} strokeWidth="2" />
      <line x1="0" y1="12" x2="8" y2="12" stroke={color} strokeWidth="2" />
      <line x1="0" y1="24" x2="8" y2="24" stroke={color} strokeWidth="2" />
      <line x1="44" y1="18" x2="48" y2="18" stroke={color} strokeWidth="2" />
    </svg>
  );
}

/** SVG for NOT gate (triangle + bubble) */
function NotGateSvg({ active }: { active?: boolean }) {
  const color = active ? "var(--wire-high)" : "#555";
  return (
    <svg width="48" height="36" viewBox="0 0 48 36" aria-label="NOT gate">
      <polygon
        points="4,4 36,18 4,32"
        fill="none" stroke={color} strokeWidth="2"
      />
      <circle cx="39" cy="18" r="3" fill="none" stroke={color} strokeWidth="2" />
      <line x1="0" y1="18" x2="4" y2="18" stroke={color} strokeWidth="2" />
      <line x1="42" y1="18" x2="48" y2="18" stroke={color} strokeWidth="2" />
    </svg>
  );
}

function WireLabel({ value }: { value: Bit }) {
  return (
    <span className={`wire-label ${value ? "wire-label--high" : "wire-label--low"}`}>
      {value}
    </span>
  );
}

/** Show the internal gates of a full adder for one bit position */
function FullAdderGates({ adder, bitIndex }: { adder: FullAdderState; bitIndex: number }) {
  // Compute intermediate values:
  // XOR1: A XOR B = partial sum (P)
  const p: Bit = (adder.a ^ adder.b) as Bit;
  // AND1: A AND B
  const and1: Bit = (adder.a & adder.b) as Bit;
  // XOR2: P XOR Cin = Sum
  // AND2: P AND Cin
  const and2: Bit = (p & adder.cIn) as Bit;
  // OR: AND1 OR AND2 = Cout

  return (
    <div className="fa-internals">
      <div style={{ fontSize: "0.7rem", color: "#8899aa", marginBottom: 4 }}>
        Full Adder — Bit {bitIndex} internal gates:
      </div>

      <div className="fa-gate-row">
        <span className="fa-gate-name">XOR1:</span>
        <div className="fa-gate-io">
          <WireLabel value={adder.a} /> XOR <WireLabel value={adder.b} /> = <WireLabel value={p} />
        </div>
        <XorGateSvg active={p === 1} />
      </div>

      <div className="fa-gate-row">
        <span className="fa-gate-name">XOR2:</span>
        <div className="fa-gate-io">
          <WireLabel value={p} /> XOR <WireLabel value={adder.cIn} /> = <WireLabel value={adder.sum} />
        </div>
        <XorGateSvg active={adder.sum === 1} />
      </div>

      <div className="fa-gate-row">
        <span className="fa-gate-name">AND1:</span>
        <div className="fa-gate-io">
          <WireLabel value={adder.a} /> AND <WireLabel value={adder.b} /> = <WireLabel value={and1} />
        </div>
        <AndGateSvg active={and1 === 1} />
      </div>

      <div className="fa-gate-row">
        <span className="fa-gate-name">AND2:</span>
        <div className="fa-gate-io">
          <WireLabel value={p} /> AND <WireLabel value={adder.cIn} /> = <WireLabel value={and2} />
        </div>
        <AndGateSvg active={and2 === 1} />
      </div>

      <div className="fa-gate-row">
        <span className="fa-gate-name">OR:</span>
        <div className="fa-gate-io">
          <WireLabel value={and1} /> OR <WireLabel value={and2} /> = <WireLabel value={adder.cOut} />
        </div>
        <OrGateSvg active={adder.cOut === 1} />
      </div>
    </div>
  );
}

/** Find the most recent trace with ALU detail. */
function findLastAluTrace(history: readonly DetailedTrace[]): DetailedTrace | undefined {
  for (let i = history.length - 1; i >= 0; i--) {
    if (history[i]!.aluTrace) return history[i];
  }
  return undefined;
}

export function GateView({ trace, traceHistory }: GateViewProps) {
  const { t } = useTranslation();

  const aluTrace = trace?.aluTrace ? trace : findLastAluTrace(traceHistory);
  const hasAlu = !!aluTrace?.aluTrace && aluTrace.aluTrace.adders.length > 0;

  return (
    <section className="gate-view" aria-label={t("gate.title")}>
      <h2>{t("gate.title")}</h2>
      <p>{t("gate.description")}</p>

      {trace && (
        <div className="gate-info">
          <p>Current instruction: <code>{trace.mnemonic}</code> at 0x{trace.address.toString(16).padStart(3, "0")}</p>

          {/* Instruction decoder */}
          <div className="gate-diagram" style={{ marginTop: 16 }}>
            <h3>{t("gate.decoder")}</h3>
            <p style={{ fontSize: "0.8rem", color: "#8899aa", marginBottom: 12 }}>
              {t("gate.decoder.description")}
            </p>
            <div style={{ fontFamily: "var(--mono)", fontSize: "0.8rem" }}>
              <div style={{ marginBottom: 8 }}>
                Opcode: <code style={{ color: "var(--wire-high)" }}>
                  {trace.raw.toString(2).padStart(8, "0")}
                </code>
                {" "}(0x{trace.raw.toString(16).padStart(2, "0").toUpperCase()})
              </div>
              <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
                {trace.raw.toString(2).padStart(8, "0").split("").map((bit, i) => (
                  <span
                    key={i}
                    className={`adder-bit ${bit === "1" ? "adder-bit--high" : "adder-bit--low"}`}
                    style={{ width: 28, height: 28, fontSize: "0.8rem" }}
                  >
                    {bit}
                  </span>
                ))}
              </div>
              <div style={{ fontSize: "0.65rem", color: "#556", marginTop: 4, display: "flex", gap: 4 }}>
                {"D7 D6 D5 D4 D3 D2 D1 D0".split(" ").map((label, i) => (
                  <span key={i} style={{ width: 28, textAlign: "center" }}>{label}</span>
                ))}
              </div>
            </div>
          </div>

          {/* Gate-level descriptions */}
          <div style={{ display: "flex", flexWrap: "wrap", gap: 8, marginTop: 12, marginBottom: 16 }}>
            <div className="gate-card">
              <div className="gate-card-icon"><AndGateSvg active /></div>
              <div className="gate-card-text">
                <div className="gate-card-title">{t("gate.and")}</div>
                <div className="gate-card-desc">{t("gate.and.description")}</div>
              </div>
            </div>
            <div className="gate-card">
              <div className="gate-card-icon"><OrGateSvg active /></div>
              <div className="gate-card-text">
                <div className="gate-card-title">{t("gate.or")}</div>
                <div className="gate-card-desc">{t("gate.or.description")}</div>
              </div>
            </div>
            <div className="gate-card">
              <div className="gate-card-icon"><XorGateSvg active /></div>
              <div className="gate-card-text">
                <div className="gate-card-title">{t("gate.xor")}</div>
                <div className="gate-card-desc">{t("gate.xor.description")}</div>
              </div>
            </div>
            <div className="gate-card">
              <div className="gate-card-icon"><NotGateSvg active /></div>
              <div className="gate-card-text">
                <div className="gate-card-title">{t("gate.not")}</div>
                <div className="gate-card-desc">{t("gate.not.description")}</div>
              </div>
            </div>
          </div>

          {/* Full adder gate breakdowns if ALU was active */}
          {hasAlu && (
            <div className="gate-diagram">
              <h3>ALU Gate Activations — {t(`alu.op.${aluTrace!.aluTrace!.operation}`)}</h3>
              <p style={{ fontSize: "0.8rem", color: "#8899aa", marginBottom: 12 }}>
                Each full adder is built from 5 logic gates. Here are the exact gate activations
                for the {aluTrace!.aluTrace!.operation} operation:
              </p>
              {aluTrace!.aluTrace!.adders.map((adder, i) => (
                <FullAdderGates key={i} adder={adder} bitIndex={i} />
              ))}
            </div>
          )}
        </div>
      )}

      {!trace && (
        <p style={{ color: "#556" }}>Execute an instruction to see gate-level details.</p>
      )}
    </section>
  );
}
