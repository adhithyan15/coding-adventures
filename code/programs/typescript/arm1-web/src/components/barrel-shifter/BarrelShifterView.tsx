/**
 * ==========================================================================
 * BarrelShifterView — Shift Operation Visualization
 * ==========================================================================
 *
 * The ARM1's barrel shifter sits between the register file and the ALU.
 * Before the ALU sees Operand2, the barrel shifter can apply any of these
 * operations in a single clock cycle (hence "for free"):
 *
 *   LSL #n  — Logical Shift Left: multiply by 2^n, fills zeros on right
 *   LSR #n  — Logical Shift Right: unsigned divide by 2^n, fills zeros on left
 *   ASR #n  — Arithmetic Shift Right: signed divide, fills sign-bit on left
 *   ROR #n  — Rotate Right: bits wrap from right back into left end
 *   RRX     — Rotate Right Extended: rotate 1 bit through the C flag (33-bit rotate)
 *
 * # Why a "barrel" shifter?
 *
 * A naive shifter would use n separate single-bit shift stages in sequence.
 * A barrel shifter uses a tree of multiplexers (MUX2 gates) to perform any
 * shift amount in one step. The ARM1's barrel shifter has 5 levels of MUX2
 * gates (one per bit of the 5-bit shift amount), giving O(log n) instead
 * of O(n) gate depth.
 *
 * Level 0:  shift by 0 or 1
 * Level 1:  shift by 0 or 2   (combined with level 0: 0,1,2,3)
 * Level 2:  shift by 0 or 4   (0–7)
 * Level 3:  shift by 0 or 8   (0–15)
 * Level 4:  shift by 0 or 16  (0–31)
 *
 * See the gate-level package (arm1-gatelevel) for the full MUX2 tree.
 */

import type { ShiftDetail, SimulatorState } from "../../simulator/types.js";

// Format a 32-bit unsigned number as a binary string, grouped by nibble.
function toBinaryString(n: number): string {
  const s = (n >>> 0).toString(2).padStart(32, "0");
  return `${s.slice(0, 4)} ${s.slice(4, 8)} ${s.slice(8, 12)} ${s.slice(12, 16)} ` +
         `${s.slice(16, 20)} ${s.slice(20, 24)} ${s.slice(24, 28)} ${s.slice(28, 32)}`;
}

// Renders one row of 32 coloured bit cells.
interface BitRowProps {
  value: number;
  label: string;
  highlightBits?: Set<number>; // which bit positions (0=LSB) to highlight
}

function BitRow({ value, label, highlightBits }: BitRowProps) {
  return (
    <div className="bit-row">
      <span className="bit-row-label">{label}</span>
      <div className="bit-cells">
        {Array.from({ length: 32 }, (_, i) => {
          const pos = 31 - i;  // bit 31 first
          const bitVal = (value >>> pos) & 1;
          const highlighted = highlightBits?.has(pos);
          return (
            <span
              key={i}
              className={`bit-cell ${bitVal ? "bit-1" : "bit-0"} ${highlighted ? "bit-highlight" : ""}`}
              title={`bit ${pos} = ${bitVal}`}
            >
              {bitVal}
            </span>
          );
        })}
      </div>
      <span className="bit-row-hex">0x{(value >>> 0).toString(16).toUpperCase().padStart(8, "0")}</span>
    </div>
  );
}

// Compute which bits "moved" so we can highlight them.
function computeMovedBits(shift: ShiftDetail): { inputMoved: Set<number>; outputMoved: Set<number> } {
  const inputMoved = new Set<number>();
  const outputMoved = new Set<number>();

  if (shift.isNop || shift.shiftType === "none") return { inputMoved, outputMoved };

  const { input, output, shiftType, amount } = shift;
  const n = amount;

  // Find bits that are 1 in input and ended up at a different position in output.
  for (let bit = 0; bit < 32; bit++) {
    if ((input >>> bit) & 1) {
      let destBit: number | null = null;
      switch (shiftType) {
        case "LSL": destBit = bit + n < 32 ? bit + n : null; break;
        case "LSR": destBit = bit - n >= 0 ? bit - n : null; break;
        case "ASR": destBit = bit - n >= 0 ? bit - n : null; break;
        case "ROR": destBit = ((bit - n) % 32 + 32) % 32; break;
        case "RRX": destBit = bit > 0 ? bit - 1 : null; break;
      }
      if (destBit !== null && destBit !== bit) {
        inputMoved.add(bit);
        outputMoved.add(destBit);
      }
    }
  }

  // Also highlight output bits that came from carry-in (for RRX).
  if (shiftType === "RRX" && ((output >>> 31) & 1)) {
    outputMoved.add(31);
  }

  return { inputMoved, outputMoved };
}

interface ShiftDiagramProps {
  shift: ShiftDetail;
}

function ShiftDiagram({ shift }: ShiftDiagramProps) {
  const { inputMoved, outputMoved } = computeMovedBits(shift);

  // Human-readable description of the operation.
  const opDescriptions: Record<string, string> = {
    LSL: `Logical Shift Left #${shift.amount} — bits move toward MSB, zeros fill LSB end. Equivalent to unsigned multiplication by 2^${shift.amount}.`,
    LSR: `Logical Shift Right #${shift.amount} — bits move toward LSB, zeros fill MSB end. Equivalent to unsigned division by 2^${shift.amount}.`,
    ASR: `Arithmetic Shift Right #${shift.amount} — bits move toward LSB, the sign bit (MSB) is copied into the vacated positions. Equivalent to signed division by 2^${shift.amount}.`,
    ROR: `Rotate Right #${shift.amount} — bits wrap from the right end back into the left end. No bits are lost.`,
    RRX: `Rotate Right Extended — rotate right by 1 bit through the 33-bit register+carry chain. Bit 0 becomes the carry output; the carry input becomes bit 31.`,
    none: "No shift — Operand2 passes through unchanged (LSL #0).",
  };

  return (
    <div className="shift-diagram">
      <div className="shift-operation-badge">
        <span className="shift-type">{shift.isNop ? "LSL #0 (pass-through)" : `${shift.shiftType} #${shift.amount}`}</span>
        <span className="shift-description">
          {opDescriptions[shift.isNop ? "none" : shift.shiftType] ?? ""}
        </span>
      </div>

      <div className="bit-diagram">
        <BitRow
          value={shift.input}
          label="Input (Rm)"
          highlightBits={inputMoved}
        />

        <div className="shift-arrow-row">
          <span className="shift-arrow-label">
            {shift.shiftType} {shift.shiftType !== "RRX" ? `#${shift.amount}` : ""}
          </span>
          <div className="shift-arrows">
            {shift.shiftType === "LSL" && <span className="shift-dir shift-left">◀◀◀ shift left {shift.amount}</span>}
            {shift.shiftType === "LSR" && <span className="shift-dir shift-right">shift right {shift.amount} ▶▶▶</span>}
            {shift.shiftType === "ASR" && <span className="shift-dir shift-right">arith right {shift.amount} ▶▶▶</span>}
            {shift.shiftType === "ROR" && <span className="shift-dir shift-rotate">↷ rotate right {shift.amount}</span>}
            {shift.shiftType === "RRX" && <span className="shift-dir shift-rotate">↷ rotate through C</span>}
            {shift.isNop && <span className="shift-dir shift-nop">— pass through —</span>}
          </div>
        </div>

        <BitRow
          value={shift.output}
          label="Output"
          highlightBits={outputMoved}
        />
      </div>

      <div className="carry-out">
        <span className="carry-label">Carry-Out</span>
        <span className={`carry-value ${shift.carryOut ? "carry-1" : "carry-0"}`}>
          {shift.carryOut ? "1" : "0"}
        </span>
        <span className="carry-desc">
          {shift.shiftType === "LSL"
            ? `Bit ${32 - shift.amount} of input`
            : shift.shiftType === "LSR" || shift.shiftType === "ASR"
            ? `Bit ${shift.amount - 1} of input`
            : shift.shiftType === "ROR"
            ? `Bit ${shift.amount - 1} of input`
            : shift.shiftType === "RRX"
            ? "Bit 0 of input"
            : "Unchanged from C flag"}
        </span>
      </div>

      <div className="binary-view">
        <div className="binary-row">
          <span className="binary-label">Input</span>
          <span className="binary-value mono">{toBinaryString(shift.input)}</span>
        </div>
        <div className="binary-row">
          <span className="binary-label">Output</span>
          <span className="binary-value mono">{toBinaryString(shift.output)}</span>
        </div>
      </div>
    </div>
  );
}

// MUX2 tree diagram — shows the 5 levels of the barrel shifter.
function MuxTreeDiagram({ shiftAmount }: { shiftAmount: number }) {
  const bits = [
    (shiftAmount >> 0) & 1,
    (shiftAmount >> 1) & 1,
    (shiftAmount >> 2) & 1,
    (shiftAmount >> 3) & 1,
    (shiftAmount >> 4) & 1,
  ];

  return (
    <div className="mux-tree">
      <h4 className="mux-title">5-Level MUX2 Tree</h4>
      <p className="mux-description">
        Each level selects whether to apply that power-of-2 shift.
        The 5-bit shift amount controls all 5 MUX2 selectors simultaneously.
      </p>
      <div className="mux-levels">
        {[0, 1, 2, 3, 4].map(level => (
          <div key={level} className={`mux-level ${bits[level] ? "mux-active" : "mux-bypass"}`}>
            <span className="mux-level-label">Level {level}</span>
            <div className="mux-box">MUX2</div>
            <span className="mux-sel">
              sel={bits[level]} → {bits[level] ? `shift by ${1 << level}` : "pass through"}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

interface BarrelShifterViewProps {
  state: SimulatorState;
}

export function BarrelShifterView({ state }: BarrelShifterViewProps) {
  const lastTrace = state.traces.at(-1);
  const shift = lastTrace?.shift;

  if (!lastTrace) {
    return (
      <div className="barrel-view">
        <div className="no-data-message">
          Step through an instruction to see the barrel shifter in action.
          Try the <strong>Barrel Shifter</strong> program for a focused demo.
        </div>
      </div>
    );
  }

  if (!shift) {
    return (
      <div className="barrel-view">
        <div className="barrel-header">
          <h2 className="panel-title">Barrel Shifter</h2>
          <div className="last-instr">Last instruction: <code>{lastTrace.mnemonic}</code></div>
        </div>
        <div className="no-shift-message">
          The last instruction (<code>{lastTrace.mnemonic}</code>) did not use the barrel
          shifter — it was either a Load/Store, Branch, Block Transfer, SWI, or a data
          processing instruction with an immediate (constant) Operand2 and no rotation.
        </div>
        <section className="barrel-explainer">
          <h3 className="explainer-title">About the Barrel Shifter</h3>
          <p>
            The barrel shifter sits between the register file and the ALU. For data
            processing instructions with a register Operand2, the shifter transforms
            the register value before the ALU sees it — all within a single clock cycle.
          </p>
          <p>
            Instructions like <code>ADD R0, R1, R2 LSL #3</code> compute R1 + (R2 × 8)
            without needing a separate multiply instruction. This "shift for free" feature
            is one of the reasons ARM code is so compact.
          </p>
        </section>
      </div>
    );
  }

  return (
    <div className="barrel-view">
      <header className="barrel-header">
        <h2 className="panel-title">Barrel Shifter</h2>
        <p className="panel-subtitle">
          Last instruction: <code>{lastTrace.mnemonic}</code> at
          0x{lastTrace.address.toString(16).toUpperCase().padStart(8, "0")}
        </p>
      </header>

      <ShiftDiagram shift={shift} />

      {shift.shiftType !== "none" && !shift.isNop && (
        <MuxTreeDiagram shiftAmount={shift.amount} />
      )}

      <section className="shift-types-reference">
        <h3 className="explainer-title">Shift Type Reference</h3>
        <div className="shift-type-cards">
          {[
            { name: "LSL", desc: "Logical Shift Left", example: "0x00000001 LSL #3 = 0x00000008", note: "Equivalent to × 2^n (unsigned)" },
            { name: "LSR", desc: "Logical Shift Right", example: "0x00000080 LSR #3 = 0x00000010", note: "Equivalent to ÷ 2^n (unsigned)" },
            { name: "ASR", desc: "Arithmetic Shift Right", example: "0xFF000000 ASR #4 = 0xFFF00000", note: "Sign extends — divides signed integers" },
            { name: "ROR", desc: "Rotate Right", example: "0x00000001 ROR #1 = 0x80000000", note: "Wraps bits around — no information lost" },
            { name: "RRX", desc: "Rotate Right Extended", example: "0x00000001 RRX (C=1) = 0x80000000", note: "33-bit rotate through the carry flag" },
          ].map(({ name, desc, example, note }) => (
            <div key={name} className={`shift-card ${shift.shiftType === name ? "shift-card-active" : ""}`}>
              <span className="shift-card-name">{name}</span>
              <span className="shift-card-desc">{desc}</span>
              <code className="shift-card-example">{example}</code>
              <span className="shift-card-note">{note}</span>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}
