/**
 * ALU Detail view — Layer 3.
 *
 * Shows the ripple carry adder chain when the current instruction
 * involves the ALU. Each of the 4 full adders displays its inputs (A, B,
 * Cin) and outputs (Sum, Cout) with carry propagation highlighted between
 * adders. Bits are color-coded: green for 1, dim for 0.
 *
 * The full adder is the fundamental building block of all arithmetic:
 *
 *     A ──┐         ┌── Sum
 *     B ──┤ Full    │
 *         │ Adder ──┤
 *   Cin ──┘         └── Cout
 *
 * Four of these chain together into a 4-bit ripple carry adder,
 * where each adder's Cout feeds the next adder's Cin.
 */

import { useTranslation } from "../../i18n/index.js";
import type { DetailedTrace } from "../../cpu/types.js";
import type { Bit } from "@coding-adventures/logic-gates";

interface AluViewProps {
  trace: DetailedTrace | undefined;
  traceHistory: readonly DetailedTrace[];
}

function BitCircle({ value }: { value: Bit }) {
  return (
    <span className={`adder-bit ${value ? "adder-bit--high" : "adder-bit--low"}`}>
      {value}
    </span>
  );
}

/** Find the most recent trace with ALU detail in the history. */
function findLastAluTrace(history: readonly DetailedTrace[]): DetailedTrace | undefined {
  for (let i = history.length - 1; i >= 0; i--) {
    if (history[i]!.aluDetail) return history[i];
  }
  return undefined;
}

export function AluView({ trace, traceHistory }: AluViewProps) {
  const { t } = useTranslation();

  // Use the last trace if it has ALU detail, otherwise find most recent one
  const aluTrace = trace?.aluDetail ? trace : findLastAluTrace(traceHistory);

  if (!aluTrace?.aluDetail) {
    return (
      <section className="alu-view" aria-label={t("alu.title")}>
        <h2>{t("alu.title")}</h2>
        <p>{t("alu.noAluOp")}</p>
      </section>
    );
  }

  const { aluDetail } = aluTrace;

  return (
    <section className="alu-view" aria-label={t("alu.title")}>
      <h2>{t("alu.title")}</h2>
      <p>{t("alu.description")}</p>

      <div className="alu-operation">
        {t(`alu.op.${aluDetail.operation}`)}
      </div>

      {/* Inputs */}
      <div className="alu-inputs">
        <div>
          <span>{t("alu.inputA")}</span>
          <code>
            {[...aluDetail.inputA].reverse().map((b, i) => (
              <BitCircle key={i} value={b} />
            ))}
          </code>
        </div>
        <div>
          <span>{t("alu.inputB")}</span>
          <code>
            {[...aluDetail.inputB].reverse().map((b, i) => (
              <BitCircle key={i} value={b} />
            ))}
          </code>
        </div>
        <div>
          <span>{t("alu.carryIn")}</span>
          <code><BitCircle value={aluDetail.carryIn} /></code>
        </div>
      </div>

      {/* Ripple Carry Adder Chain — MSB on left, LSB on right */}
      <h3 style={{ marginBottom: 12 }}>Ripple Carry Adder Chain</h3>
      <div className="alu-adder-chain">
        {[...aluDetail.adders].reverse().map((adder, i) => {
          const bitIndex = 3 - i;
          return (
            <div key={bitIndex} className="full-adder-card">
              <h3>Bit {bitIndex}</h3>
              <div className="adder-io">
                <div className="adder-input">
                  <span className="adder-input-label">A</span>
                  <BitCircle value={adder.a} />
                </div>
                <div className="adder-input">
                  <span className="adder-input-label">B</span>
                  <BitCircle value={adder.b} />
                </div>
                <div className="adder-input">
                  <span className="adder-input-label">Cin</span>
                  <BitCircle value={adder.cIn} />
                </div>
                <hr className="adder-divider" />
                <div className="adder-output">
                  <span className="adder-input-label">Sum</span>
                  <BitCircle value={adder.sum} />
                </div>
                <div className="adder-output">
                  <span className="adder-input-label">Cout</span>
                  <BitCircle value={adder.cOut} />
                </div>
              </div>
              {/* Carry wire to next adder */}
              {i < 3 && (
                <span className={`adder-carry-wire ${adder.cOut ? "adder-carry-wire--active" : ""}`}>
                  →
                </span>
              )}
            </div>
          );
        })}
      </div>

      {/* Result */}
      <div className="alu-result">
        <span>{t("alu.result")}:</span>
        <code>
          {[...aluDetail.result].reverse().map((b, i) => (
            <BitCircle key={i} value={b} />
          ))}
        </code>
        <span>= {parseInt([...aluDetail.result].reverse().map(b => b.toString()).join(""), 2)}</span>
        <span style={{ marginLeft: 12 }}>
          {t("alu.carryOut")}: <BitCircle value={aluDetail.carryOut} />
        </span>
      </div>
    </section>
  );
}
