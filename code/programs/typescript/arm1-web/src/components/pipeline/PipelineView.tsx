/**
 * ==========================================================================
 * PipelineView — 3-Stage Fetch/Decode/Execute Visualization
 * ==========================================================================
 *
 * The ARM1 uses a classic 3-stage pipeline:
 *
 *   Stage 1: Fetch   — reads the instruction word from memory at PC
 *   Stage 2: Decode  — extracts instruction fields (condition, opcode, regs…)
 *   Stage 3: Execute — performs the ALU/memory operation
 *
 * All three stages run simultaneously. While Stage 3 executes instruction N,
 * Stage 2 decodes N+1, and Stage 1 fetches N+2 from memory.
 *
 * # The PC=PC+8 Effect
 *
 * Because Fetch has already advanced PC by two words (8 bytes) by the time
 * Execute runs, any instruction that reads R15 sees PC+8, not PC.
 * Branch instructions compensate for this automatically.
 *
 *   Cycle  Fetch    Decode   Execute
 *     1    instr A   —        —
 *     2    instr B   instr A  —
 *     3    instr C   instr B  instr A  ← when A executes, PC = A+8
 *     4    instr D   instr C  instr B
 *
 * # Pipeline Hazards
 *
 * When Execute writes R15 (branch taken), the pipeline must be flushed:
 * the two instructions that were in Fetch and Decode are discarded.
 * This is why ARM branches cost 3 cycles instead of 1.
 */

import type { PipelineState } from "../../simulator/types.js";

interface StageBoxProps {
  stage: "fetch" | "decode" | "execute";
  label: string;
  pc: number;
  raw: number;
  mnemonic: string;
  valid: boolean;
  description: string;
}

function StageBox({ stage, label, pc, raw, mnemonic, valid, description }: StageBoxProps) {
  const hex = raw.toString(16).toUpperCase().padStart(8, "0");
  const pcHex = pc.toString(16).toUpperCase().padStart(8, "0");

  return (
    <div className={`pipeline-stage pipeline-${stage} ${valid ? "stage-valid" : "stage-stalled"}`}
         aria-label={`${label} stage`}>
      <div className="stage-header">
        <span className="stage-number">
          {stage === "fetch" ? "1" : stage === "decode" ? "2" : "3"}
        </span>
        <span className="stage-label">{label}</span>
      </div>
      <div className="stage-content">
        {valid ? (
          <>
            <div className="stage-pc">
              <span className="stage-field-label">Address</span>
              <span className="stage-field-value mono">0x{pcHex}</span>
            </div>
            {stage !== "fetch" && (
              <div className="stage-raw">
                <span className="stage-field-label">Encoding</span>
                <span className="stage-field-value mono">0x{hex}</span>
              </div>
            )}
            {stage === "execute" && (
              <div className="stage-mnemonic">
                <span className="stage-field-label">Instruction</span>
                <span className="stage-field-value mnemonic">{mnemonic}</span>
              </div>
            )}
            {stage === "decode" && (
              <div className="stage-mnemonic">
                <span className="stage-field-label">Decoded</span>
                <span className="stage-field-value mnemonic dim">{mnemonic}</span>
              </div>
            )}
          </>
        ) : (
          <span className="stage-stalled-label">pipeline filling…</span>
        )}
      </div>
      <div className="stage-description">{description}</div>
    </div>
  );
}

function PipelineArrow() {
  return (
    <div className="pipeline-arrow" aria-hidden="true">
      <div className="arrow-line" />
      <div className="arrow-head">▶</div>
    </div>
  );
}

interface PipelineViewProps {
  pipeline: PipelineState;
  totalCycles: number;
}

export function PipelineView({ pipeline, totalCycles }: PipelineViewProps) {
  return (
    <div className="pipeline-view">
      <header className="pipeline-header">
        <h2 className="panel-title">3-Stage Pipeline</h2>
        <p className="panel-subtitle">
          Three instructions move through the pipeline simultaneously. At cycle {totalCycles},
          Execute is running the instruction below while Decode and Fetch prepare the next two.
        </p>
      </header>

      <div className="pipeline-stages">
        <StageBox
          stage="fetch"
          label="Fetch"
          pc={pipeline.fetch.pc}
          raw={pipeline.fetch.raw}
          mnemonic={pipeline.fetch.mnemonic}
          valid={pipeline.fetch.valid}
          description="Reading instruction word from memory at PC."
        />
        <PipelineArrow />
        <StageBox
          stage="decode"
          label="Decode"
          pc={pipeline.decode.pc}
          raw={pipeline.decode.raw}
          mnemonic={pipeline.decode.mnemonic}
          valid={pipeline.decode.valid}
          description="Extracting condition code, opcode, register numbers, and Operand2 encoding."
        />
        <PipelineArrow />
        <StageBox
          stage="execute"
          label="Execute"
          pc={pipeline.execute.pc}
          raw={pipeline.execute.raw}
          mnemonic={pipeline.execute.mnemonic}
          valid={pipeline.execute.valid}
          description="Running the ALU operation or memory access. PC reads as PC+8 here."
        />
      </div>

      <section className="pipeline-explainer">
        <h3 className="explainer-title">Why PC = PC+8 During Execute?</h3>
        <div className="pipeline-timing-table">
          <div className="timing-row timing-header">
            <span>Cycle</span>
            <span>Fetch</span>
            <span>Decode</span>
            <span>Execute</span>
          </div>
          {[
            { cycle: "N",   fetch: "Instr A", decode: "—",       exec: "—" },
            { cycle: "N+1", fetch: "Instr B", decode: "Instr A", exec: "—" },
            { cycle: "N+2", fetch: "Instr C", decode: "Instr B", exec: "Instr A ← PC=A+8" },
            { cycle: "N+3", fetch: "Instr D", decode: "Instr C", exec: "Instr B ← PC=B+8" },
          ].map(row => (
            <div key={row.cycle} className={`timing-row ${row.exec.includes("A") ? "timing-highlight" : ""}`}>
              <span>{row.cycle}</span>
              <span>{row.fetch}</span>
              <span>{row.decode}</span>
              <span>{row.exec}</span>
            </div>
          ))}
        </div>
        <p className="pipeline-note">
          When Instr A runs in Execute (cycle N+2), the Fetch stage has already read
          address A+8 into its buffer — so PC holds A+8. Branch instructions offset
          by −8 to compensate. This is architectural, not a bug.
        </p>
      </section>

      <section className="pipeline-hazards">
        <h3 className="explainer-title">Branch Penalty</h3>
        <p className="pipeline-note">
          When a branch is taken, the two instructions in Fetch and Decode must be
          discarded (pipeline flush). The processor wastes 2 cycles refilling the
          pipeline from the branch target. This is why tight loops on the ARM1 cost
          slightly more than the instruction count suggests.
        </p>
        <p className="pipeline-note">
          ARM's solution (added later) was the Thumb instruction set and branch
          prediction. On the ARM1, code is arranged to avoid branches where possible
          — which is why the ARM ISA supports conditional execution on <em>every</em>
          instruction: MOVGT is cheaper than BLE + MOV.
        </p>
      </section>
    </div>
  );
}
