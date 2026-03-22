/**
 * Execution Flow — the "click-to-gates" pipeline visualization.
 *
 * Shows how a single key press flows through the entire computing stack:
 *
 *   1. KEY PRESS     → User clicks "5" button
 *   2. I/O PORT      → ROM port set to 0x5
 *   3. FETCH         → CPU reads opcode from ROM at PC
 *   4. DECODE        → Instruction bits decoded into control signals
 *   5. EXECUTE       → ALU/registers/RAM updated
 *   6. ALU           → Full adders process the bits
 *   7. GATES         → AND/OR/XOR gates compute results
 *   8. TRANSISTORS   → CMOS NMOS/PMOS pairs switch
 *   9. DISPLAY       → Result appears on 7-segment LED
 *
 * Each stage is a card connected by a vertical "pipeline" line.
 * Active stages glow green; the current stage pulses red.
 */

import type { DetailedTrace } from "../../cpu/types.js";

interface ExecutionFlowProps {
  trace: DetailedTrace | undefined;
  traceHistory: readonly DetailedTrace[];
  displayDigits: number[];
  pc: number;
  accumulator: number;
}

interface FlowStage {
  label: string;
  getValue: () => string;
  getDetail?: () => string;
  isActive: boolean;
}

/** Find the most recent trace with ALU detail. */
function findLastAluTrace(history: readonly DetailedTrace[]): DetailedTrace | undefined {
  for (let i = history.length - 1; i >= 0; i--) {
    if (history[i]!.aluDetail) return history[i];
  }
  return undefined;
}

export function ExecutionFlow({ trace, traceHistory, displayDigits, pc, accumulator }: ExecutionFlowProps) {
  const hasTrace = !!trace;
  const aluTrace = trace?.aluDetail ? trace : findLastAluTrace(traceHistory);
  const hasAlu = !!aluTrace?.aluDetail;

  // Build the pipeline stages
  const stages: FlowStage[] = [
    {
      label: "Key Press",
      getValue: () => trace ? `Key pressed → ROM scans` : "Waiting for input...",
      getDetail: () => "User clicks a button on the calculator",
      isActive: hasTrace,
    },
    {
      label: "I/O Port",
      getValue: () => `ROM port value read by RDR instruction`,
      getDetail: () => "CPU executes RDR to read the key code from the ROM port",
      isActive: hasTrace,
    },
    {
      label: "Fetch",
      getValue: () => {
        if (!trace) return "—";
        return `PC=0x${trace.address.toString(16).padStart(3, "0")} → opcode 0x${trace.raw.toString(16).padStart(2, "0").toUpperCase()}`;
      },
      getDetail: () => `ROM[${pc.toString(16)}] → ${trace?.raw.toString(2).padStart(8, "0") ?? "—"}`,
      isActive: hasTrace,
    },
    {
      label: "Decode",
      getValue: () => {
        if (!trace) return "—";
        return `${trace.mnemonic}`;
      },
      getDetail: () => {
        if (!trace) return "";
        const d = trace.decoded;
        const flags: string[] = [];
        if (d.isAdd) flags.push("ADD");
        if (d.isSub) flags.push("SUB");
        if (d.isLdm) flags.push("LDM");
        if (d.isLd) flags.push("LD");
        if (d.isXch) flags.push("XCH");
        if (d.isJun) flags.push("JUN");
        if (d.isJcn) flags.push("JCN");
        if (d.isJms) flags.push("JMS");
        if (d.isInc) flags.push("INC");
        if (d.isFim) flags.push("FIM");
        if (d.isSrc) flags.push("SRC");
        if (d.isIo) flags.push("I/O");
        if (d.isAccum) flags.push("ACCUM");
        if (d.isBbl) flags.push("BBL");
        return flags.length ? `Decoder signals: ${flags.join(", ")}` : "Decoder active";
      },
      isActive: hasTrace,
    },
    {
      label: "Execute",
      getValue: () => {
        if (!trace) return "—";
        return `acc=${accumulator.toString(16).toUpperCase()} (${accumulator.toString(2).padStart(4, "0")})`;
      },
      getDetail: () => {
        if (!trace?.memoryAccess) return "";
        const m = trace.memoryAccess;
        return `${m.type}: addr=${m.address}, val=${m.value}`;
      },
      isActive: hasTrace,
    },
    {
      label: "ALU",
      getValue: () => {
        if (!hasAlu) return "Not used for this instruction";
        const alu = aluTrace!.aluDetail!;
        const aVal = parseInt([...alu.inputA].reverse().map(b => b.toString()).join(""), 2);
        const bVal = parseInt([...alu.inputB].reverse().map(b => b.toString()).join(""), 2);
        const rVal = parseInt([...alu.result].reverse().map(b => b.toString()).join(""), 2);
        return `${aVal} ${alu.operation === "sub" ? "-" : "+"} ${bVal} = ${rVal}`;
      },
      getDetail: () => {
        if (!hasAlu) return "";
        const alu = aluTrace!.aluDetail!;
        return `Carry chain: ${alu.adders.map(a => a.cOut).join(" → ")} → Cout=${alu.carryOut}`;
      },
      isActive: hasAlu,
    },
    {
      label: "Logic Gates",
      getValue: () => {
        if (!hasAlu) return "20+ gates in ALU idle";
        const totalGates = aluTrace!.aluDetail!.adders.length * 5;
        return `${totalGates} gates activated (${aluTrace!.aluDetail!.adders.length} full adders × 5 gates)`;
      },
      getDetail: () => hasAlu ? "2×XOR + 2×AND + 1×OR per full adder" : "",
      isActive: hasAlu,
    },
    {
      label: "Transistors",
      getValue: () => {
        if (!hasAlu) return "CMOS pairs idle";
        const totalTransistors = aluTrace!.aluDetail!.adders.length * 5 * 4;
        return `~${totalTransistors} transistors switched`;
      },
      getDetail: () => hasAlu ? "Each gate = 2 PMOS + 2 NMOS (CMOS pair)" : "",
      isActive: hasAlu,
    },
    {
      label: "Display Output",
      getValue: () => {
        // Show non-zero digits
        const reversed = [...displayDigits].reverse();
        const firstNonZero = reversed.findIndex(d => d !== 0);
        if (firstNonZero === -1) return "0";
        return reversed.slice(firstNonZero).join("");
      },
      getDetail: () => "BCD digits → 7-segment LED display",
      isActive: hasTrace,
    },
  ];

  return (
    <div className="execution-flow">
      <div className="flow-title">
        Signal Flow: Key Press → Transistors
      </div>
      <div className="flow-pipeline">
        {stages.map((stage, i) => {
          const isCurrent = stage.isActive && (i === stages.length - 1 || !stages[i + 1]?.isActive);
          return (
            <div key={i} className="flow-stage">
              <div className="flow-connector">
                {i > 0 && (
                  <div className={`flow-line ${stage.isActive ? "flow-line--active" : ""}`} />
                )}
                <div className={`flow-dot ${stage.isActive ? (isCurrent ? "flow-dot--current" : "flow-dot--active") : ""}`} />
                {i < stages.length - 1 && (
                  <div className={`flow-line ${stage.isActive ? "flow-line--active" : ""}`} />
                )}
              </div>
              <div className={`flow-content ${stage.isActive ? (isCurrent ? "flow-content--current" : "flow-content--active") : ""}`}>
                <div className="flow-stage-label">{stage.label}</div>
                <div className="flow-stage-value">
                  <code>{stage.getValue()}</code>
                </div>
                {stage.getDetail && stage.getDetail() && (
                  <div className="flow-stage-detail">{stage.getDetail()}</div>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
