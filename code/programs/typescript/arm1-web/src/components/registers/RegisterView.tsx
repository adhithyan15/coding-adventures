/**
 * ==========================================================================
 * RegisterView — CPU Registers and CPSR Panel
 * ==========================================================================
 *
 * Displays the ARM1's 16 visible registers (R0–R15) alongside a detailed
 * breakdown of R15, which serves as both the Program Counter and the
 * Current Program Status Register (CPSR).
 *
 * # R15 Layout (ARM1-specific)
 *
 *   Bit 31: N — Negative (result's MSB)
 *   Bit 30: Z — Zero (result was 0)
 *   Bit 29: C — Carry (unsigned overflow or shifter carry-out)
 *   Bit 28: V — Overflow (signed overflow)
 *   Bit 27: I — IRQ disable (1 = IRQs masked)
 *   Bit 26: F — FIQ disable (1 = FIQs masked)
 *   Bits 25:2: PC (24-bit word address, bits 1:0 always 0)
 *   Bits  1:0: M (processor mode: 00=USR, 01=FIQ, 10=IRQ, 11=SVC)
 *
 * This is quite different from ARMv4+ where the CPSR is a separate register.
 * On the ARM1, every branch-with-link (BL) saves the return address AND
 * all the flag/mode bits into R14, enabling trivial interrupt return.
 */

import type { SimulatorState } from "../../simulator/types.js";
import type { Flags } from "@coding-adventures/arm1-simulator";
import { modeString } from "@coding-adventures/arm1-simulator";

/** Register alias names for R0–R15. */
const REG_ALIASES: Record<number, string> = {
  11: "FP",   // Frame Pointer (convention)
  12: "IP",   // Intra-Procedure-call scratch (convention)
  13: "SP",   // Stack Pointer
  14: "LR",   // Link Register
  15: "PC",   // Program Counter + Status
};

interface RegisterRowProps {
  index: number;
  value: number;
  prevValue: number | undefined;
  isCurrent: boolean;
}

function RegisterRow({ index, value, prevValue, isCurrent }: RegisterRowProps) {
  const alias = REG_ALIASES[index];
  const changed = prevValue !== undefined && prevValue !== value;
  const hex = value.toString(16).toUpperCase().padStart(8, "0");
  const decimal = value <= 0x7FFFFFFF ? value : value - 0x100000000; // signed interpretation

  return (
    <div className={`reg-row ${changed ? "reg-changed" : ""} ${isCurrent ? "reg-current" : ""}`}>
      <span className="reg-name">
        R{index}
        {alias && <span className="reg-alias">{alias}</span>}
      </span>
      <span className="reg-hex">0x{hex}</span>
      <span className="reg-dec">{decimal}</span>
    </div>
  );
}

interface FlagBitProps {
  name: string;
  value: boolean;
  description: string;
}

function FlagBit({ name, value, description }: FlagBitProps) {
  return (
    <div className={`flag-bit flag-${name.toLowerCase()} ${value ? "flag-set" : "flag-clear"}`}
         title={description}>
      <span className="flag-name">{name}</span>
      <span className="flag-value">{value ? "1" : "0"}</span>
    </div>
  );
}

interface ModeIndicatorProps {
  mode: number;
  irqDisabled: boolean;
  fiqDisabled: boolean;
}

function ModeIndicator({ mode, irqDisabled, fiqDisabled }: ModeIndicatorProps) {
  return (
    <div className="mode-indicator">
      <div className="mode-badge mode-badge--active">
        {modeString(mode)} mode
      </div>
      <div className={`interrupt-bit ${irqDisabled ? "irq-disabled" : "irq-enabled"}`}
           title="IRQ disable bit (I=1 means IRQs are masked)">
        I={irqDisabled ? "1" : "0"}
      </div>
      <div className={`interrupt-bit ${fiqDisabled ? "fiq-disabled" : "fiq-enabled"}`}
           title="FIQ disable bit (F=1 means FIQs are masked)">
        F={fiqDisabled ? "1" : "0"}
      </div>
    </div>
  );
}

// R15 bit-field diagram showing the dual nature of PC+CPSR
function R15Diagram({ r15, flags }: { r15: number; flags: Flags }) {
  const bits = Array.from({ length: 32 }, (_, i) => (r15 >>> (31 - i)) & 1);

  // Colour the bit groups
  const bitClass = (i: number): string => {
    if (i <= 3)  return "r15-flag";      // bits 31:28 = N/Z/C/V
    if (i === 4) return "r15-irq";       // bit 27 = I
    if (i === 5) return "r15-fiq";       // bit 26 = F
    if (i <= 29) return "r15-pc";        // bits 25:2 = PC
    return "r15-mode";                   // bits 1:0 = M
  };

  const flagNames = ["N", "Z", "C", "V"];

  return (
    <div className="r15-diagram">
      <div className="r15-bits">
        {bits.map((b, i) => (
          <span key={i} className={`r15-bit ${bitClass(i)} ${b ? "bit-1" : "bit-0"}`}>
            {i <= 3 && <span className="r15-bit-label">{flagNames[i]}</span>}
            {i === 4 && <span className="r15-bit-label">I</span>}
            {i === 5 && <span className="r15-bit-label">F</span>}
            {b}
          </span>
        ))}
      </div>
      <div className="r15-legend">
        <span className="legend-flag">N Z C V</span>
        <span className="legend-i">I</span>
        <span className="legend-f">F</span>
        <span className="legend-pc">←──────── PC (bits 25:2) ────────→</span>
        <span className="legend-mode">M</span>
      </div>
      <div className="flags-row">
        <FlagBit name="N" value={flags.N} description="Negative — result bit 31 was 1 (two's complement negative)" />
        <FlagBit name="Z" value={flags.Z} description="Zero — result was exactly 0x00000000" />
        <FlagBit name="C" value={flags.C} description="Carry — unsigned overflow or barrel shifter carry-out" />
        <FlagBit name="V" value={flags.V} description="oVerflow — signed two's complement overflow" />
      </div>
    </div>
  );
}

interface RegisterViewProps {
  state: SimulatorState;
}

export function RegisterView({ state }: RegisterViewProps) {
  // Find the most recent trace to show register changes.
  const lastTrace = state.traces.at(-1);
  const prevRegs = lastTrace?.regsBefore;

  // PC is two instructions ahead of the currently-executing instruction
  // during execution (3-stage pipeline). R15 in the register file holds
  // the actual PC (next instruction to fetch).
  const currentPc = state.pc;

  return (
    <div className="register-view">
      <section className="reg-panel" aria-label="Register File">
        <h2 className="panel-title">Register File</h2>
        <p className="panel-subtitle">
          16 visible registers — orange = changed by last instruction
        </p>
        <div className="reg-grid">
          {state.registers.map((val, i) => (
            <RegisterRow
              key={i}
              index={i}
              value={val}
              prevValue={prevRegs?.[i]}
              isCurrent={i === 15}
            />
          ))}
        </div>
      </section>

      <aside className="cpsr-panel" aria-label="CPSR / R15 Status">
        <h2 className="panel-title">R15 — PC + Status</h2>
        <p className="panel-subtitle">
          On the ARM1, R15 is both the program counter and the status register.
          This unusual design means every instruction automatically saves and
          restores processor state via the register file.
        </p>

        <R15Diagram r15={state.r15} flags={state.flags} />

        <ModeIndicator
          mode={state.mode}
          irqDisabled={state.irqDisabled}
          fiqDisabled={state.fiqDisabled}
        />

        <div className="pc-info">
          <div className="pc-row">
            <span className="pc-label">PC (next fetch)</span>
            <span className="pc-value">0x{currentPc.toString(16).toUpperCase().padStart(8, "0")}</span>
          </div>
          <div className="pc-row">
            <span className="pc-label">PC during exec</span>
            <span className="pc-value">
              0x{((currentPc + 4) & 0x03FFFFFC).toString(16).toUpperCase().padStart(8, "0")}
              <span className="pc-note"> (PC+8 = fetch+2)</span>
            </span>
          </div>
          <div className="pc-row">
            <span className="pc-label">Cycles executed</span>
            <span className="pc-value">{state.totalCycles}</span>
          </div>
        </div>

        {state.halted && (
          <div className="halt-banner" role="status">
            HALTED — SWI #0x123456
          </div>
        )}
      </aside>
    </div>
  );
}
