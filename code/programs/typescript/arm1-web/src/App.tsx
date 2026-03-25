/**
 * ==========================================================================
 * ARM1 Processor Simulator — Root Application
 * ==========================================================================
 *
 * This application lets you step through ARM1 assembly code and observe
 * the processor internals at multiple levels of abstraction:
 *
 *   Registers    — 16-register file + CPSR/R15 breakdown + flags
 *   Decode       — 32-bit instruction word broken into colour-coded fields
 *   Pipeline     — 3-stage Fetch → Decode → Execute visualization
 *   Barrel       — Barrel shifter input/output with bit-level animation
 *   Memory       — Byte-addressable hex dump with PC/SP/access highlighting
 *   Trace        — Instruction history with register deltas and flag changes
 *
 * # Controls
 *
 *   Step         — Execute one instruction
 *   Run 10       — Execute 10 instructions at once
 *   Run to End   — Execute until SWI #0x123456 (halt) or 100 000 steps
 *   Reset        — Reload the current program from the start
 *   Program ▼    — Switch to a different pre-loaded demo
 */

import { useState, useCallback, useRef } from "react";
import { useARM1 } from "./hooks/useARM1.js";
import { RegisterView } from "./components/registers/RegisterView.js";
import { DecodeView } from "./components/decode/DecodeView.js";
import { PipelineView } from "./components/pipeline/PipelineView.js";
import { BarrelShifterView } from "./components/barrel-shifter/BarrelShifterView.js";
import { MemoryView } from "./components/memory/MemoryView.js";
import { TraceView } from "./components/trace/TraceView.js";

type Tab = "registers" | "decode" | "pipeline" | "barrel" | "memory" | "trace";

const TABS: Array<{ id: Tab; label: string; shortLabel: string }> = [
  { id: "registers", label: "Registers",      shortLabel: "Regs" },
  { id: "decode",    label: "Decode",         shortLabel: "Dec"  },
  { id: "pipeline",  label: "Pipeline",       shortLabel: "Pipe" },
  { id: "barrel",    label: "Barrel Shifter", shortLabel: "BSft" },
  { id: "memory",    label: "Memory",         shortLabel: "Mem"  },
  { id: "trace",     label: "Trace",          shortLabel: "Log"  },
];

export function App() {
  const [activeTab, setActiveTab] = useState<Tab>("registers");
  const tabListRef = useRef<HTMLElement>(null);
  const {
    state,
    step,
    runN,
    runToEnd,
    reset,
    loadProgram,
    programIndex,
    programs,
    readMemory,
  } = useARM1();

  /** WAI-ARIA tab keyboard navigation. */
  const handleTabKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      const idx = TABS.findIndex(t => t.id === activeTab);
      let next = idx;
      if (e.key === "ArrowRight" || e.key === "ArrowDown") next = (idx + 1) % TABS.length;
      else if (e.key === "ArrowLeft" || e.key === "ArrowUp") next = (idx - 1 + TABS.length) % TABS.length;
      else if (e.key === "Home") next = 0;
      else if (e.key === "End") next = TABS.length - 1;
      else return;
      e.preventDefault();
      setActiveTab(TABS[next]!.id);
      const buttons = tabListRef.current?.querySelectorAll<HTMLButtonElement>("[role=tab]");
      buttons?.[next]?.focus();
    },
    [activeTab],
  );

  const prog = programs[programIndex]!;

  return (
    <div className="app">
      {/* ================================================================ */}
      {/* Header                                                            */}
      {/* ================================================================ */}
      <header className="app-header">
        <div className="header-title-group">
          <h1 className="app-title">ARM1 Processor Simulator</h1>
          <p className="app-subtitle">
            The first ARM chip — designed by Sophie Wilson &amp; Steve Furber,
            Acorn Cambridge, 1985. 25,000 transistors. First silicon worked first time.
          </p>
        </div>

        {/* ---- Program selector + controls ---- */}
        <div className="controls-bar">
          <div className="program-selector">
            <label htmlFor="program-select" className="control-label">Program</label>
            <select
              id="program-select"
              className="program-select"
              value={programIndex}
              onChange={e => loadProgram(Number(e.target.value))}
            >
              {programs.map((p, i) => (
                <option key={i} value={i}>{p.name}</option>
              ))}
            </select>
            <div className="program-expected">→ {prog.expectedResult}</div>
          </div>

          <div className="sim-controls">
            <button
              className="btn btn-step"
              onClick={step}
              disabled={state.halted}
              title="Execute one instruction (keyboard: Space)"
            >
              Step
            </button>
            <button
              className="btn btn-run"
              onClick={() => runN(10)}
              disabled={state.halted}
              title="Execute 10 instructions"
            >
              Run ×10
            </button>
            <button
              className="btn btn-run-all"
              onClick={runToEnd}
              disabled={state.halted}
              title="Run until halt (SWI #0x123456)"
            >
              Run to End
            </button>
            <button
              className="btn btn-reset"
              onClick={reset}
              title="Reset CPU and reload program"
            >
              Reset
            </button>
          </div>

          <div className={`status-badge ${state.halted ? "status-halted" : state.totalCycles === 0 ? "status-ready" : "status-running"}`}>
            {state.halted ? "HALTED" : state.totalCycles === 0 ? "READY" : "RUNNING"}
            {state.totalCycles > 0 && (
              <span className="status-cycles"> · {state.totalCycles} cycles</span>
            )}
          </div>
        </div>

        {/* ---- Tab bar ---- */}
        <nav
          className="tab-bar"
          role="tablist"
          aria-label="Simulator views"
          ref={tabListRef}
          onKeyDown={handleTabKeyDown}
        >
          {TABS.map(tab => (
            <button
              key={tab.id}
              role="tab"
              aria-selected={activeTab === tab.id}
              aria-controls={`panel-${tab.id}`}
              tabIndex={activeTab === tab.id ? 0 : -1}
              className={`tab-btn ${activeTab === tab.id ? "tab-active" : ""}`}
              onClick={() => setActiveTab(tab.id)}
            >
              <span className="tab-long">{tab.label}</span>
              <span className="tab-short">{tab.shortLabel}</span>
            </button>
          ))}
        </nav>
      </header>

      {/* ================================================================ */}
      {/* Main panel                                                        */}
      {/* ================================================================ */}
      <main
        className="tab-panel"
        role="tabpanel"
        id={`panel-${activeTab}`}
        aria-label={TABS.find(t => t.id === activeTab)?.label}
      >
        {activeTab === "registers" && <RegisterView state={state} />}
        {activeTab === "decode"    && <DecodeView state={state} />}
        {activeTab === "pipeline"  && <PipelineView pipeline={state.pipeline} totalCycles={state.totalCycles} />}
        {activeTab === "barrel"    && <BarrelShifterView state={state} />}
        {activeTab === "memory"    && <MemoryView state={state} readMemory={readMemory} />}
        {activeTab === "trace"     && <TraceView traces={state.traces} totalCycles={state.totalCycles} />}
      </main>

      {/* ================================================================ */}
      {/* Listing sidebar (collapsible)                                     */}
      {/* ================================================================ */}
      <aside className="listing-panel" aria-label="Program listing">
        <h3 className="listing-title">Assembly Listing</h3>
        <div className="listing-code">
          {prog.listing.map((line, i) => {
            // Parse the address from each listing line (format: "0xADDR  INSTR  ; comment")
            const addrMatch = line.match(/^(0x[0-9a-fA-F]+)/);
            const lineAddr = addrMatch ? parseInt(addrMatch[1], 16) : -1;
            const isActive = lineAddr === state.traces.at(-1)?.address;
            return (
              <div key={i} className={`listing-line ${isActive ? "listing-active" : ""}`}>
                <pre>{line}</pre>
              </div>
            );
          })}
        </div>
        <div className="listing-description">{prog.description}</div>
      </aside>

      {/* ================================================================ */}
      {/* Footer                                                            */}
      {/* ================================================================ */}
      <footer className="app-footer">
        <p>
          ARM1 (ARMv1) — Acorn RISC Machine, 1985 · 25,000 transistors ·
          3-stage pipeline · 26-bit address space · every instruction conditional ·
          barrel shifter on Operand2 ·{" "}
          <a
            href="https://github.com/simonwhatley/arm1-simulator"
            target="_blank"
            rel="noopener noreferrer"
          >
            historical reference
          </a>
        </p>
      </footer>
    </div>
  );
}
