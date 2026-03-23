/**
 * CPU State view — Layer 2.
 *
 * Live dashboard showing the Intel 4004's internal state during execution:
 * program counter, accumulator, carry flag, registers, hardware stack,
 * RAM contents, and scrollable instruction trace log.
 */

import { useTranslation } from "../../i18n/index.js";
import type { CalculatorState } from "../../hooks/useCalculator.js";

interface CpuViewProps {
  calculator: CalculatorState;
}

export function CpuView({ calculator }: CpuViewProps) {
  const { t } = useTranslation();

  return (
    <section className="cpu-view" aria-label={t("cpu.title")}>
      <h2>{t("cpu.title")}</h2>

      {/* State fields */}
      <div className="cpu-state-grid">
        <div className="cpu-field">
          <span className="cpu-label">{t("cpu.pc")}</span>
          <span className="cpu-value">
            0x{calculator.pc.toString(16).padStart(3, "0")}
          </span>
        </div>
        <div className="cpu-field">
          <span className="cpu-label">{t("cpu.accumulator")}</span>
          <span className="cpu-value">
            {calculator.accumulator.toString(16).toUpperCase()}
            <span style={{ opacity: 0.5, fontSize: "0.8em", marginLeft: 6 }}>
              {calculator.accumulator.toString(2).padStart(4, "0")}
            </span>
          </span>
        </div>
        <div className="cpu-field">
          <span className="cpu-label">{t("cpu.carry")}</span>
          <span className={`cpu-value ${calculator.carry ? "cpu-value--carry-set" : ""}`}>
            {calculator.carry ? "1 (SET)" : "0 (CLEAR)"}
          </span>
        </div>
      </div>

      {/* Registers */}
      <div className="cpu-registers">
        <h3>{t("cpu.registers")}</h3>
        <div className="register-grid">
          {calculator.registers.map((val, i) => (
            <div
              key={i}
              className={`register-cell ${val !== 0 ? "register-cell--changed" : ""}`}
            >
              <span className="register-name">R{i}</span>
              <span className="register-value">
                {val.toString(16).toUpperCase()}
              </span>
            </div>
          ))}
        </div>
      </div>

      {/* RAM view — show first 4 registers of bank 0 */}
      <div className="cpu-ram">
        <h3>{t("cpu.ram")}</h3>
        {[0, 1, 2, 3].map((reg) => (
          <div key={reg} className="ram-register">
            <div className="ram-register-label">
              Register {reg}{reg === 0 ? " (Display)" : reg === 1 ? " (Input)" : reg === 2 ? " (Operand)" : " (Flags)"}
            </div>
            <div className="ram-cells">
              {(calculator.ramData[0]?.[reg] ?? []).slice(0, 16).map((val, i) => (
                <div
                  key={i}
                  className={`ram-cell ${val !== 0 ? "ram-cell--nonzero" : ""}`}
                >
                  {val.toString(16).toUpperCase()}
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>

      {/* Instruction trace */}
      <div className="cpu-trace">
        <h3>{t("cpu.trace")}</h3>
        {calculator.traceHistory.length === 0 ? (
          <p style={{ color: "#556", fontSize: "0.8rem" }}>No instructions executed yet.</p>
        ) : (
          calculator.traceHistory.slice(-20).map((trace, i) => (
            <div
              key={i}
              className={`trace-entry ${i === calculator.traceHistory.slice(-20).length - 1 ? "trace-entry--current" : ""}`}
            >
              <span className="trace-addr">
                0x{trace.address.toString(16).padStart(3, "0")}
              </span>
              <span className="trace-hex">
                {trace.raw.toString(16).padStart(2, "0").toUpperCase()}
              </span>
              <span className="trace-mnemonic">{trace.mnemonic}</span>
            </div>
          ))
        )}
      </div>

      {/* Controls */}
      <div className="cpu-controls">
        <button type="button" className="btn btn--primary" onClick={() => calculator.stepOne()}>
          {t("cpu.step")}
        </button>
        <button type="button" className="btn" onClick={() => calculator.reset()}>
          {t("cpu.reset")}
        </button>
      </div>
    </section>
  );
}
