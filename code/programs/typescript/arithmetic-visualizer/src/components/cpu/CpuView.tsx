/**
 * CpuView — Tab 4: CPU Step-Through.
 *
 * Loads an Intel 4004 program, steps through it one instruction at a time,
 * and displays the full CPU state: accumulator, registers, carry, PC,
 * decoded instruction, and ALU trace.
 *
 * === Educational purpose ===
 *
 * This ties everything together: the adders from Tab 1, the two's complement
 * from Tab 2, and the ALU from Tab 3 are all visible inside a real CPU
 * executing real instructions. Each step shows exactly which gates fire.
 */

import { useState, useRef, useCallback, useEffect } from "react";
import { Intel4004GateLevel } from "@coding-adventures/intel4004-gatelevel";
import type { GateTrace } from "@coding-adventures/intel4004-gatelevel";
import { useTranslation } from "@coding-adventures/ui-components";
import { EXAMPLE_PROGRAMS } from "./programs.js";

export function CpuView() {
  const { t } = useTranslation();
  const cpuRef = useRef<Intel4004GateLevel | null>(null);
  const autoStepRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const [selectedProgram, setSelectedProgram] = useState(0);
  const [loaded, setLoaded] = useState(false);
  const [traces, setTraces] = useState<GateTrace[]>([]);
  const [autoStepping, setAutoStepping] = useState(false);

  // CPU snapshot state (updated after each step)
  const [accum, setAccum] = useState(0);
  const [carry, setCarry] = useState(false);
  const [regs, setRegs] = useState<number[]>(new Array(16).fill(0));
  const [pc, setPc] = useState(0);
  const [halted, setHalted] = useState(false);

  const updateCpuState = useCallback(() => {
    const cpu = cpuRef.current;
    if (!cpu) return;
    setAccum(cpu.accumulator);
    setCarry(cpu.carry);
    setRegs([...cpu.registers]);
    setPc(cpu.pc);
    setHalted(cpu.halted);
  }, []);

  const handleLoad = useCallback(() => {
    const program = EXAMPLE_PROGRAMS[selectedProgram];
    const cpu = new Intel4004GateLevel();
    cpu.loadProgram(new Uint8Array(program.bytes));
    cpuRef.current = cpu;
    setLoaded(true);
    setTraces([]);
    setAutoStepping(false);
    if (autoStepRef.current) clearInterval(autoStepRef.current);
    updateCpuState();
  }, [selectedProgram, updateCpuState]);

  const handleStep = useCallback(() => {
    const cpu = cpuRef.current;
    if (!cpu || cpu.halted) return;
    try {
      const trace = cpu.step();
      setTraces((prev) => [...prev, trace]);
      updateCpuState();
    } catch {
      setHalted(true);
      setAutoStepping(false);
      if (autoStepRef.current) clearInterval(autoStepRef.current);
    }
  }, [updateCpuState]);

  const handleReset = useCallback(() => {
    setAutoStepping(false);
    if (autoStepRef.current) clearInterval(autoStepRef.current);
    handleLoad();
  }, [handleLoad]);

  const toggleAutoStep = useCallback(() => {
    if (autoStepping) {
      setAutoStepping(false);
      if (autoStepRef.current) clearInterval(autoStepRef.current);
    } else {
      setAutoStepping(true);
    }
  }, [autoStepping]);

  // Auto-step interval
  useEffect(() => {
    if (autoStepping && !halted) {
      autoStepRef.current = setInterval(() => {
        handleStep();
      }, 500);
      return () => {
        if (autoStepRef.current) clearInterval(autoStepRef.current);
      };
    }
    if (halted && autoStepping) {
      setAutoStepping(false);
    }
  }, [autoStepping, halted, handleStep]);

  const program = EXAMPLE_PROGRAMS[selectedProgram];
  const lastTrace = traces.length > 0 ? traces[traces.length - 1] : null;

  return (
    <div className="cpu-view">
      <p className="cpu-view__intro">{t("cpu.intro")}</p>

      {/* Program loader */}
      <section className="cpu-panel" aria-label={t("cpu.loader")}>
        <h3 className="cpu-panel__title">{t("cpu.loader")}</h3>
        <div className="cpu-loader">
          <select
            className="cpu-loader__select"
            value={selectedProgram}
            onChange={(e) => setSelectedProgram(Number(e.target.value))}
            aria-label={t("cpu.selectProgram")}
          >
            {EXAMPLE_PROGRAMS.map((prog, i) => (
              <option key={i} value={i}>
                {t(prog.nameKey)}
              </option>
            ))}
          </select>
          <p className="cpu-loader__desc">{t(program.descKey)}</p>
          <div className="cpu-loader__hex">
            {program.bytes.map((b, i) => (
              <span
                key={i}
                className={`cpu-loader__byte ${loaded && pc === i ? "cpu-loader__byte--active" : ""}`}
              >
                {b.toString(16).toUpperCase().padStart(2, "0")}
              </span>
            ))}
          </div>
          <button className="cpu-btn" onClick={handleLoad} type="button">
            {t("cpu.load")}
          </button>
        </div>
      </section>

      {loaded && (
        <>
          {/* Step controls */}
          <section className="cpu-panel" aria-label={t("cpu.controls")}>
            <div className="cpu-controls">
              <button
                className="cpu-btn"
                onClick={handleStep}
                disabled={halted}
                type="button"
              >
                {t("cpu.step")}
              </button>
              <button
                className={`cpu-btn ${autoStepping ? "cpu-btn--active" : ""}`}
                onClick={toggleAutoStep}
                disabled={halted}
                type="button"
              >
                {autoStepping ? t("cpu.stop") : t("cpu.autoStep")}
              </button>
              <button className="cpu-btn" onClick={handleReset} type="button">
                {t("cpu.reset")}
              </button>
              {halted && (
                <span className="cpu-halted">{t("cpu.halted")}</span>
              )}
              <span className="cpu-step-count">
                {t("cpu.stepCount")}: {traces.length}
              </span>
            </div>
          </section>

          {/* CPU state dashboard */}
          <section className="cpu-panel" aria-label={t("cpu.state")}>
            <h3 className="cpu-panel__title">{t("cpu.state")}</h3>
            <div className="cpu-state">
              {/* Accumulator */}
              <div className="cpu-state__item">
                <span className="cpu-state__label">ACC</span>
                <span className="cpu-state__value">{accum}</span>
                <span className="cpu-state__hex">
                  (0x{accum.toString(16).toUpperCase()})
                </span>
              </div>
              {/* Carry */}
              <div className="cpu-state__item">
                <span className="cpu-state__label">Carry</span>
                <span className={`cpu-state__flag ${carry ? "cpu-state__flag--set" : ""}`}>
                  {carry ? "1" : "0"}
                </span>
              </div>
              {/* PC */}
              <div className="cpu-state__item">
                <span className="cpu-state__label">PC</span>
                <span className="cpu-state__value">
                  0x{pc.toString(16).toUpperCase().padStart(3, "0")}
                </span>
              </div>
            </div>

            {/* Register file */}
            <div className="cpu-registers">
              <span className="cpu-registers__title">{t("cpu.registers")}</span>
              <div className="cpu-registers__grid">
                {regs.map((val, i) => {
                  const changed = lastTrace && (
                    lastTrace.decoded.regIndex === i ||
                    (lastTrace.decoded.isXch && lastTrace.decoded.regIndex === i) ||
                    (lastTrace.decoded.isInc && lastTrace.decoded.regIndex === i)
                  );
                  return (
                    <div
                      key={i}
                      className={`cpu-reg ${changed ? "cpu-reg--changed" : ""}`}
                    >
                      <span className="cpu-reg__name">R{i}</span>
                      <span className="cpu-reg__val">{val}</span>
                    </div>
                  );
                })}
              </div>
            </div>
          </section>

          {/* Current instruction */}
          {lastTrace && (
            <section className="cpu-panel" aria-label={t("cpu.currentInstr")}>
              <h3 className="cpu-panel__title">{t("cpu.currentInstr")}</h3>
              <div className="cpu-instr">
                <div className="cpu-instr__row">
                  <span className="cpu-instr__label">{t("cpu.address")}</span>
                  <span className="cpu-instr__value">
                    0x{lastTrace.address.toString(16).toUpperCase().padStart(3, "0")}
                  </span>
                </div>
                <div className="cpu-instr__row">
                  <span className="cpu-instr__label">{t("cpu.opcode")}</span>
                  <span className="cpu-instr__value">
                    0x{lastTrace.raw.toString(16).toUpperCase().padStart(2, "0")}
                    {lastTrace.raw2 !== null
                      ? ` 0x${lastTrace.raw2.toString(16).toUpperCase().padStart(2, "0")}`
                      : ""}
                  </span>
                </div>
                <div className="cpu-instr__row">
                  <span className="cpu-instr__label">{t("cpu.mnemonic")}</span>
                  <span className="cpu-instr__mnemonic">{lastTrace.mnemonic}</span>
                </div>
                <div className="cpu-instr__row">
                  <span className="cpu-instr__label">ACC</span>
                  <span className="cpu-instr__value">
                    {lastTrace.accumulatorBefore} → {lastTrace.accumulatorAfter}
                  </span>
                </div>
                <div className="cpu-instr__row">
                  <span className="cpu-instr__label">Carry</span>
                  <span className="cpu-instr__value">
                    {lastTrace.carryBefore ? "1" : "0"} → {lastTrace.carryAfter ? "1" : "0"}
                  </span>
                </div>
              </div>

              {/* ALU trace (if present) */}
              {lastTrace.aluTrace && (
                <div className="cpu-alu-trace">
                  <h4 className="cpu-alu-trace__title">{t("cpu.aluTrace")}</h4>
                  <span className="cpu-alu-trace__op">{lastTrace.aluTrace.operation}</span>
                  {lastTrace.aluTrace.adders.length > 0 && (
                    <table className="truth-table">
                      <caption>{t("cpu.adderDetails")}</caption>
                      <thead>
                        <tr>
                          <th scope="col">Bit</th>
                          <th scope="col">A</th>
                          <th scope="col">B</th>
                          <th scope="col">Cin</th>
                          <th scope="col">Sum</th>
                          <th scope="col">Cout</th>
                        </tr>
                      </thead>
                      <tbody>
                        {lastTrace.aluTrace.adders.map((snap, i) => (
                          <tr key={i}>
                            <td>{i}</td>
                            <td>{snap.a}</td>
                            <td>{snap.b}</td>
                            <td>{snap.cIn}</td>
                            <td>{snap.sum}</td>
                            <td>{snap.cOut}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  )}
                </div>
              )}
            </section>
          )}

          {/* Trace history */}
          {traces.length > 0 && (
            <section className="cpu-panel" aria-label={t("cpu.traceHistory")}>
              <h3 className="cpu-panel__title">{t("cpu.traceHistory")}</h3>
              <div className="cpu-trace-list">
                {traces.map((trace, i) => (
                  <div
                    key={i}
                    className={`cpu-trace-row ${i === traces.length - 1 ? "cpu-trace-row--current" : ""}`}
                  >
                    <span className="cpu-trace-row__num">#{i + 1}</span>
                    <span className="cpu-trace-row__addr">
                      {trace.address.toString(16).toUpperCase().padStart(3, "0")}
                    </span>
                    <span className="cpu-trace-row__mnem">{trace.mnemonic}</span>
                    <span className="cpu-trace-row__acc">
                      ACC: {trace.accumulatorBefore}→{trace.accumulatorAfter}
                    </span>
                    {trace.aluTrace && (
                      <span className="cpu-trace-row__alu">[ALU: {trace.aluTrace.operation}]</span>
                    )}
                  </div>
                ))}
              </div>
            </section>
          )}
        </>
      )}
    </div>
  );
}
