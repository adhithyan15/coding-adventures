/**
 * ==========================================================================
 * DecodeView — Instruction Bit-Field Breakdown
 * ==========================================================================
 *
 * Visualises the 32-bit instruction word currently at PC, breaking it into
 * colour-coded fields according to the ARM1 instruction encoding.
 *
 * # ARM1 Instruction Encoding
 *
 * Every instruction is 32 bits, and every instruction has a 4-bit condition
 * code in bits 31:28. The remaining bits depend on the instruction class:
 *
 * Data Processing (bits 27:26 = 00):
 *   31:28  27:26  25    24:21    20   19:16  15:12  11:0
 *   cond   00     I     opcode   S    Rn     Rd     operand2
 *
 * Load/Store (bits 27:26 = 01):
 *   31:28  27:26  25    24   23  22   21   20   19:16  15:12  11:0
 *   cond   01     I     P    U   B    W    L    Rn     Rd     offset
 *
 * Branch (bits 27:25 = 101):
 *   31:28  27:25  24   23:0
 *   cond   101    L    offset (signed, in units of 4 bytes)
 *
 * SWI (bits 27:24 = 1111):
 *   31:28  27:24  23:0
 *   cond   1111   comment
 */

import { decode, opString, shiftString,
         INST_DATA_PROCESSING, INST_LOAD_STORE, INST_BLOCK_TRANSFER,
         INST_BRANCH, INST_SWI, INST_COPROCESSOR, INST_UNDEFINED,
         SHIFT_ROR, HALT_SWI } from "@coding-adventures/arm1-simulator";
import type { DecodedInstruction } from "@coding-adventures/arm1-simulator";
import type { SimulatorState } from "../../simulator/types.js";

// Condition code names for all 16 codes.
const COND_NAMES = [
  "EQ", "NE", "CS", "CC", "MI", "PL", "VS", "VC",
  "HI", "LS", "GE", "LT", "GT", "LE", "AL", "NV",
];
const COND_DESCRIPTIONS = [
  "Equal (Z=1)",
  "Not equal (Z=0)",
  "Carry set / ≥ unsigned (C=1)",
  "Carry clear / < unsigned (C=0)",
  "Minus / negative (N=1)",
  "Plus / positive or zero (N=0)",
  "Overflow set (V=1)",
  "Overflow clear (V=0)",
  "Unsigned higher (C=1 & Z=0)",
  "Unsigned lower or same (C=0 | Z=1)",
  "Signed ≥ (N=V)",
  "Signed < (N≠V)",
  "Signed > (Z=0 & N=V)",
  "Signed ≤ (Z=1 | N≠V)",
  "Always (unconditional)",
  "Never (reserved)",
];

// Bit range in the instruction word: [high, low, color class, label, value].
interface Field {
  high: number;
  low: number;
  cls: string;
  label: string;
  value: string;
  tooltip: string;
}

function extractBits(instr: number, high: number, low: number): number {
  return (instr >>> low) & ((1 << (high - low + 1)) - 1);
}

function buildFields(raw: number, d: DecodedInstruction): Field[] {
  const cond = extractBits(raw, 31, 28);
  const condName = COND_NAMES[cond] ?? "??";
  const condDesc = COND_DESCRIPTIONS[cond] ?? "";
  const condField: Field = {
    high: 31, low: 28, cls: "field-cond",
    label: "COND", value: condName,
    tooltip: `Condition code: ${condName} — ${condDesc}`,
  };

  switch (d.type) {
    case INST_DATA_PROCESSING: {
      const fields: Field[] = [condField];
      fields.push({ high: 27, low: 26, cls: "field-class", label: "00", value: "00", tooltip: "Instruction class: Data Processing" });
      fields.push({ high: 25, low: 25, cls: "field-flag", label: "I", value: d.immediate ? "1" : "0", tooltip: d.immediate ? "I=1: Operand2 is a rotated immediate" : "I=0: Operand2 is a register (+ optional shift)" });
      fields.push({ high: 24, low: 21, cls: "field-opcode", label: "OPCODE", value: opString(d.opcode), tooltip: `ALU opcode ${d.opcode}: ${opString(d.opcode)}` });
      fields.push({ high: 20, low: 20, cls: "field-flag", label: "S", value: d.s ? "1" : "0", tooltip: d.s ? "S=1: Update condition flags (N,Z,C,V) after this instruction" : "S=0: Do not update condition flags" });
      fields.push({ high: 19, low: 16, cls: "field-rn", label: "Rn", value: `R${d.rn}`, tooltip: `Rn = R${d.rn}: first operand (input to ALU)` });
      fields.push({ high: 15, low: 12, cls: "field-rd", label: "Rd", value: `R${d.rd}`, tooltip: `Rd = R${d.rd}: destination register` });
      if (d.immediate) {
        fields.push({ high: 11, low: 8, cls: "field-rotate", label: "ROT", value: `${d.rotate}`, tooltip: `Rotate field: actual rotation = ${d.rotate} × 2 = ${d.rotate * 2} bits right` });
        fields.push({ high: 7, low: 0, cls: "field-imm", label: "IMM8", value: `0x${d.imm8.toString(16).toUpperCase()}`, tooltip: `8-bit immediate value = ${d.imm8}, rotated right by ${d.rotate * 2}` });
      } else {
        if (d.shiftByReg) {
          fields.push({ high: 11, low: 8, cls: "field-rs", label: "Rs", value: `R${d.rs}`, tooltip: `Rs = R${d.rs}: shift amount register` });
          fields.push({ high: 7, low: 7, cls: "field-zero", label: "0", value: "0", tooltip: "Must be 0 for register-shifted register" });
          fields.push({ high: 6, low: 5, cls: "field-shift", label: "SHFT", value: shiftString(d.shiftType), tooltip: `Shift type: ${shiftString(d.shiftType)}` });
          fields.push({ high: 4, low: 4, cls: "field-flag", label: "1", value: "1", tooltip: "Bit 4=1: shift amount comes from register Rs" });
          fields.push({ high: 3, low: 0, cls: "field-rm", label: "Rm", value: `R${d.rm}`, tooltip: `Rm = R${d.rm}: register to shift` });
        } else {
          const shiftLabel = d.shiftType === SHIFT_ROR && d.shiftImm === 0 ? "RRX" : `${shiftString(d.shiftType)} #${d.shiftImm}`;
          fields.push({ high: 11, low: 7, cls: "field-shimm", label: "#", value: `${d.shiftImm}`, tooltip: `Shift amount = ${d.shiftImm}` });
          fields.push({ high: 6, low: 5, cls: "field-shift", label: "SHFT", value: shiftLabel, tooltip: `Shift type: ${shiftLabel}` });
          fields.push({ high: 4, low: 4, cls: "field-zero", label: "0", value: "0", tooltip: "Bit 4=0: shift amount is an immediate" });
          fields.push({ high: 3, low: 0, cls: "field-rm", label: "Rm", value: `R${d.rm}`, tooltip: `Rm = R${d.rm}: register to shift` });
        }
      }
      return fields;
    }

    case INST_LOAD_STORE: {
      const offsetDesc = d.immediate
        ? `register offset: R${d.rm}${d.shiftImm ? ` ${shiftString(d.shiftType)} #${d.shiftImm}` : ""}`
        : `immediate offset: ${d.offset12}`;
      return [
        condField,
        { high: 27, low: 26, cls: "field-class", label: "01", value: "01", tooltip: "Instruction class: Load/Store" },
        { high: 25, low: 25, cls: "field-flag", label: "I", value: d.immediate ? "1" : "0", tooltip: d.immediate ? "I=1: register offset" : "I=0: immediate offset" },
        { high: 24, low: 24, cls: "field-flag", label: "P", value: d.preIndex ? "1" : "0", tooltip: d.preIndex ? "P=1: pre-index (add offset before transfer)" : "P=0: post-index (add offset after transfer)" },
        { high: 23, low: 23, cls: "field-flag", label: "U", value: d.up ? "1" : "0", tooltip: d.up ? "U=1: add offset (upward)" : "U=0: subtract offset (downward)" },
        { high: 22, low: 22, cls: "field-flag", label: "B", value: d.byte ? "1" : "0", tooltip: d.byte ? "B=1: byte transfer" : "B=0: word transfer (32-bit)" },
        { high: 21, low: 21, cls: "field-flag", label: "W", value: d.writeBack ? "1" : "0", tooltip: d.writeBack ? "W=1: write back final address to Rn" : "W=0: no write-back" },
        { high: 20, low: 20, cls: "field-flag", label: "L", value: d.load ? "1" : "0", tooltip: d.load ? "L=1: Load (memory → register)" : "L=0: Store (register → memory)" },
        { high: 19, low: 16, cls: "field-rn", label: "Rn", value: `R${d.rn}`, tooltip: `Rn = R${d.rn}: base address register` },
        { high: 15, low: 12, cls: "field-rd", label: "Rd", value: `R${d.rd}`, tooltip: `Rd = R${d.rd}: ${d.load ? "destination" : "source"} register` },
        { high: 11, low: 0, cls: "field-imm", label: "OFFSET", value: offsetDesc, tooltip: `Offset: ${offsetDesc}` },
      ];
    }

    case INST_BLOCK_TRANSFER: {
      const modeLabel = !d.preIndex && d.up ? "IA" : d.preIndex && d.up ? "IB" : !d.preIndex && !d.up ? "DA" : "DB";
      const regCount = Array.from({ length: 16 }, (_, i) => (d.registerList >>> i) & 1).filter(Boolean).length;
      return [
        condField,
        { high: 27, low: 26, cls: "field-class", label: "10", value: "10", tooltip: "Instruction class: Block Transfer" },
        { high: 25, low: 25, cls: "field-zero", label: "0", value: "0", tooltip: "Bit 25=0: Block Transfer (vs Branch)" },
        { high: 24, low: 24, cls: "field-flag", label: "P", value: d.preIndex ? "1" : "0", tooltip: d.preIndex ? "P=1: pre-increment/decrement" : "P=0: post-increment/decrement" },
        { high: 23, low: 23, cls: "field-flag", label: "U", value: d.up ? "1" : "0", tooltip: d.up ? "U=1: increment" : "U=0: decrement" },
        { high: 22, low: 22, cls: "field-flag", label: "S", value: d.forceUser ? "1" : "0", tooltip: d.forceUser ? "S=1: force user-mode registers" : "S=0: use current-mode registers" },
        { high: 21, low: 21, cls: "field-flag", label: "W", value: d.writeBack ? "1" : "0", tooltip: d.writeBack ? "W=1: write back final base to Rn" : "W=0: no write-back" },
        { high: 20, low: 20, cls: "field-flag", label: "L", value: d.load ? "1" : "0", tooltip: d.load ? "L=1: LDM (load multiple)" : "L=0: STM (store multiple)" },
        { high: 19, low: 16, cls: "field-rn", label: "Rn", value: `R${d.rn}`, tooltip: `Rn = R${d.rn}: base address (${modeLabel} mode)` },
        { high: 15, low: 0, cls: "field-imm", label: "RLIST", value: `${regCount} regs`, tooltip: `Register list: ${regCount} registers (bits 15:0)` },
      ];
    }

    case INST_BRANCH: {
      const offset = d.branchOffset;
      return [
        condField,
        { high: 27, low: 25, cls: "field-class", label: "101", value: "101", tooltip: "Instruction class: Branch" },
        { high: 24, low: 24, cls: "field-flag", label: "L", value: d.link ? "1" : "0", tooltip: d.link ? "L=1: Branch with Link — saves PC+4 in R14 (LR)" : "L=0: Branch (no link)" },
        { high: 23, low: 0, cls: "field-imm", label: "OFFSET", value: `${offset >= 0 ? "+" : ""}${offset}`, tooltip: `Signed word offset ${offset} → byte offset ${offset * 4} from PC+8` },
      ];
    }

    case INST_SWI: {
      const isHalt = d.swiComment === HALT_SWI;
      return [
        condField,
        { high: 27, low: 24, cls: "field-class", label: "1111", value: "1111", tooltip: "Instruction class: SWI (Software Interrupt)" },
        { high: 23, low: 0, cls: "field-imm", label: "COMMENT", value: `0x${d.swiComment.toString(16).toUpperCase()}`, tooltip: isHalt ? "SWI 0x123456 = simulator halt signal" : `SWI comment field (OS call number)` },
      ];
    }

    default:
      return [condField];
  }
}

// Renders a row of coloured bit-cells for bits [high:low] of the instruction.
function BitRange({ raw, high, low, cls }: { raw: number; high: number; low: number; cls: string }) {
  const count = high - low + 1;
  return (
    <>
      {Array.from({ length: count }, (_, i) => {
        const bitPos = high - i;
        const bitVal = (raw >>> bitPos) & 1;
        return (
          <span key={bitPos} className={`instr-bit ${cls} ${bitVal ? "bit-1" : "bit-0"}`}>
            {bitVal}
          </span>
        );
      })}
    </>
  );
}

interface DecodeViewProps {
  state: SimulatorState;
}

export function DecodeView({ state }: DecodeViewProps) {
  const pc = state.pc;
  // Use the most recent trace's instruction, or decode what's at PC if not yet stepped.
  const lastTrace = state.traces.at(-1);
  const raw = lastTrace ? lastTrace.raw : 0;
  const decoded = lastTrace ? lastTrace.decoded : decode(0xE3A00000); // MOV R0, #0 as placeholder
  const mnemonic = lastTrace ? lastTrace.mnemonic : "(not yet executed)";
  const dispPc = lastTrace ? lastTrace.address : pc;

  const fields = buildFields(raw, decoded);

  const instTypeNames: Record<number, string> = {
    [INST_DATA_PROCESSING]: "Data Processing",
    [INST_LOAD_STORE]: "Single Data Transfer",
    [INST_BLOCK_TRANSFER]: "Block Data Transfer",
    [INST_BRANCH]: "Branch",
    [INST_SWI]: "Software Interrupt",
    [INST_COPROCESSOR]: "Coprocessor",
    [INST_UNDEFINED]: "Undefined",
  };

  const instClass = instTypeNames[decoded.type] ?? "Unknown";
  const rawHex = raw.toString(16).toUpperCase().padStart(8, "0");

  return (
    <div className="decode-view">
      <section className="decode-header">
        <div className="decode-addr">
          <span className="decode-addr-label">Address</span>
          <span className="decode-addr-value">
            0x{dispPc.toString(16).toUpperCase().padStart(8, "0")}
          </span>
        </div>
        <div className="decode-raw">
          <span className="decode-raw-label">Raw (hex)</span>
          <span className="decode-raw-value">0x{rawHex}</span>
        </div>
        <div className="decode-mnemonic">
          <span className="decode-mnemonic-label">Assembly</span>
          <span className="decode-mnemonic-value">{mnemonic}</span>
        </div>
        <div className="decode-class">
          <span className="decode-class-label">Class</span>
          <span className="decode-class-value">{instClass}</span>
        </div>
      </section>

      <section className="decode-bits" aria-label="Instruction bit fields">
        <h3 className="decode-section-title">Bit-Field Breakdown</h3>
        <div className="bit-positions">
          {Array.from({ length: 32 }, (_, i) => (
            <span key={i} className="bit-pos">{31 - i}</span>
          ))}
        </div>
        <div className="instr-bits-row">
          {fields.map(f => (
            <BitRange key={`${f.high}-${f.low}`} raw={raw} high={f.high} low={f.low} cls={f.cls} />
          ))}
        </div>
        <div className="field-labels">
          {fields.map(f => (
            <div
              key={`${f.high}-${f.low}`}
              className={`field-label ${f.cls}`}
              style={{ gridColumn: `span ${f.high - f.low + 1}` }}
              title={f.tooltip}
            >
              <span className="field-label-name">{f.label}</span>
              <span className="field-label-value">{f.value}</span>
            </div>
          ))}
        </div>
      </section>

      <section className="decode-fields" aria-label="Decoded fields detail">
        <h3 className="decode-section-title">Field Meanings</h3>
        <div className="field-detail-grid">
          {fields.map(f => (
            <div key={`${f.high}-${f.low}`} className={`field-detail ${f.cls}`}>
              <span className="field-detail-range">
                {f.high === f.low ? `bit ${f.high}` : `bits ${f.high}:${f.low}`}
              </span>
              <span className="field-detail-name">{f.label}</span>
              <span className="field-detail-value">{f.value}</span>
              <span className="field-detail-tooltip">{f.tooltip}</span>
            </div>
          ))}
        </div>
      </section>

      <section className="condition-table" aria-label="Condition code table">
        <h3 className="decode-section-title">All 16 Condition Codes</h3>
        <p className="panel-subtitle">
          Every ARM1 instruction has a 4-bit condition code (bits 31:28).
          The instruction executes only when the condition is satisfied by
          the current N/Z/C/V flags. Condition 0xE (AL) always executes.
        </p>
        <div className="cond-table-grid">
          {COND_NAMES.map((name, i) => (
            <div
              key={i}
              className={`cond-cell ${decoded.cond === i ? "cond-active" : ""}`}
              title={COND_DESCRIPTIONS[i]}
            >
              <span className="cond-code">0x{i.toString(16).toUpperCase()}</span>
              <span className="cond-name">{name}</span>
              <span className="cond-desc">{COND_DESCRIPTIONS[i]}</span>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}
