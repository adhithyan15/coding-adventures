/**
 * ==========================================================================
 * TraceView — Execution History Log
 * ==========================================================================
 *
 * Shows the last N instructions executed with register changes highlighted
 * in orange. The most recent instruction appears at the bottom.
 *
 * Each row shows:
 *   - Cycle number
 *   - PC address
 *   - Condition code + whether it was met
 *   - Disassembled mnemonic
 *   - Flag changes (N/Z/C/V)
 *   - Changed registers (before → after)
 *   - Memory accesses (reads/writes)
 */

import type { ExtendedTrace } from "../../simulator/types.js";
import { INST_DATA_PROCESSING, INST_BRANCH, INST_LOAD_STORE,
         INST_BLOCK_TRANSFER, INST_SWI } from "@coding-adventures/arm1-simulator";

const REG_NAMES = [
  "R0","R1","R2","R3","R4","R5","R6","R7",
  "R8","R9","R10","R11","R12","SP","LR","PC",
];

function hex8(n: number): string {
  return (n >>> 0).toString(16).toUpperCase().padStart(8, "0");
}

function hex4(n: number): string {
  return (n >>> 0).toString(16).toUpperCase().padStart(4, "0");
}

/** Determine the category badge colour for this instruction type. */
function instClass(trace: ExtendedTrace): string {
  switch (trace.decoded.type) {
    case INST_DATA_PROCESSING: return "class-dp";
    case INST_LOAD_STORE:      return "class-ls";
    case INST_BLOCK_TRANSFER:  return "class-bt";
    case INST_BRANCH:          return "class-br";
    case INST_SWI:             return "class-swi";
    default:                   return "class-undef";
  }
}

function instClassLabel(trace: ExtendedTrace): string {
  switch (trace.decoded.type) {
    case INST_DATA_PROCESSING: return "DP";
    case INST_LOAD_STORE:      return "LS";
    case INST_BLOCK_TRANSFER:  return "BT";
    case INST_BRANCH:          return "BR";
    case INST_SWI:             return "SWI";
    default:                   return "??";
  }
}

interface FlagDeltaProps {
  before: { N: boolean; Z: boolean; C: boolean; V: boolean };
  after: { N: boolean; Z: boolean; C: boolean; V: boolean };
}

function FlagDelta({ before, after }: FlagDeltaProps) {
  const flags = ["N", "Z", "C", "V"] as const;
  return (
    <div className="trace-flags">
      {flags.map(f => {
        const b = before[f];
        const a = after[f];
        const changed = b !== a;
        return (
          <span
            key={f}
            className={`trace-flag trace-flag-${f.toLowerCase()} ${a ? "flag-set" : "flag-clear"} ${changed ? "flag-changed" : ""}`}
            title={changed ? `${f}: ${b ? "1" : "0"} → ${a ? "1" : "0"}` : `${f} = ${a ? "1" : "0"}`}
          >
            {f}={a ? "1" : "0"}
          </span>
        );
      })}
    </div>
  );
}

interface RegDeltaProps {
  before: number[];
  after: number[];
}

function RegDelta({ before, after }: RegDeltaProps) {
  const changed: Array<{ idx: number; from: number; to: number }> = [];
  for (let i = 0; i < 16; i++) {
    if ((before[i] ?? 0) !== (after[i] ?? 0)) {
      changed.push({ idx: i, from: before[i] ?? 0, to: after[i] ?? 0 });
    }
  }
  if (changed.length === 0) return null;
  return (
    <div className="trace-regs">
      {changed.map(({ idx, from, to }) => (
        <span key={idx} className="trace-reg-change">
          <span className="trace-reg-name">{REG_NAMES[idx]}</span>
          <span className="trace-reg-from">0x{hex8(from)}</span>
          <span className="trace-arrow">→</span>
          <span className="trace-reg-to">0x{hex8(to)}</span>
        </span>
      ))}
    </div>
  );
}

interface TraceRowProps {
  trace: ExtendedTrace;
  isLatest: boolean;
}

function TraceRow({ trace, isLatest }: TraceRowProps) {
  const skipped = !trace.conditionMet;

  return (
    <div
      className={[
        "trace-row",
        isLatest ? "trace-latest" : "",
        skipped ? "trace-skipped" : "",
        instClass(trace),
      ].filter(Boolean).join(" ")}
      role="row"
    >
      <span className="trace-cycle" title="Cycle number">#{trace.cycle}</span>

      <span className="trace-addr" title="Instruction address">
        0x{hex4(trace.address)}
      </span>

      <span className={`trace-class-badge ${instClass(trace)}`}>
        {instClassLabel(trace)}
      </span>

      <span className="trace-mnemonic" title={skipped ? "Condition not met — instruction skipped" : "Executed"}>
        {skipped && <span className="skipped-marker" title="Condition not met">↷</span>}
        {trace.mnemonic}
      </span>

      <FlagDelta before={trace.flagsBefore} after={trace.flagsAfter} />

      {!skipped && (
        <RegDelta before={trace.regsBefore} after={trace.regsAfter} />
      )}

      {!skipped && trace.memoryReads.length > 0 && (
        <div className="trace-mem trace-mem-read">
          {trace.memoryReads.map((r, i) => (
            <span key={i} className="trace-mem-item">
              LD[0x{hex8(r.address)}]=0x{hex8(r.value)}
            </span>
          ))}
        </div>
      )}

      {!skipped && trace.memoryWrites.length > 0 && (
        <div className="trace-mem trace-mem-write">
          {trace.memoryWrites.map((r, i) => (
            <span key={i} className="trace-mem-item">
              ST[0x{hex8(r.address)}]←0x{hex8(r.value)}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

interface TraceViewProps {
  traces: ExtendedTrace[];
  totalCycles: number;
}

export function TraceView({ traces, totalCycles }: TraceViewProps) {
  if (traces.length === 0) {
    return (
      <div className="trace-view">
        <div className="no-data-message">
          Press <strong>Step</strong> or <strong>Run</strong> to start executing instructions.
          The execution history will appear here.
        </div>
      </div>
    );
  }

  return (
    <div className="trace-view">
      <header className="trace-header">
        <h2 className="panel-title">Execution Trace</h2>
        <p className="panel-subtitle">
          {totalCycles} instruction{totalCycles !== 1 ? "s" : ""} executed.
          Showing last {traces.length}.
          <span className="legend-inline">
            <span className="class-dp legend-badge">DP</span> Data Processing •
            <span className="class-ls legend-badge">LS</span> Load/Store •
            <span className="class-br legend-badge">BR</span> Branch •
            <span className="trace-skipped-sample">↷</span> Condition not met
          </span>
        </p>
      </header>

      <div className="trace-log" role="log" aria-live="polite" aria-label="Execution trace">
        <div className="trace-row trace-row-header" role="row">
          <span className="trace-cycle">#</span>
          <span className="trace-addr">Addr</span>
          <span className="trace-class-badge">Type</span>
          <span className="trace-mnemonic">Instruction</span>
          <span>Flags</span>
          <span>Register Changes</span>
        </div>
        {traces.map((trace, i) => (
          <TraceRow
            key={trace.cycle}
            trace={trace}
            isLatest={i === traces.length - 1}
          />
        ))}
      </div>
    </div>
  );
}
