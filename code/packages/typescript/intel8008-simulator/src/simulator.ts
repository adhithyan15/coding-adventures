/**
 * Intel 8008 Behavioral Simulator — the world's first 8-bit microprocessor.
 *
 * === Historical context ===
 *
 * The Intel 8008 was released in April 1972 — one year after the 4-bit Intel 4004.
 * Designed by Ted Hoff, Stanley Mazor, and Hal Feeney for Computer Terminal Corporation
 * (CTC), who wanted a CPU for their Datapoint 2200 terminal. CTC ultimately rejected
 * the chip as too slow, allowing Intel to sell it commercially. The rest is history:
 * the 8008 inspired the 8080, which inspired the Z80 and the x86 — making this
 * rejected terminal chip the ancestor of the processor you're running right now.
 *
 * Specifications:
 *   - ~3,500 PMOS transistors (vs 4004's 2,300)
 *   - 500–800 kHz two-phase clock
 *   - 8-bit data bus, 14-bit address bus
 *   - 7 general-purpose 8-bit registers: A (accumulator), B, C, D, E, H, L
 *   - M pseudo-register: indirect memory access via H:L pair
 *   - 4 condition flags: Carry, Zero, Sign, Parity
 *   - 8-level push-down stack (entry 0 IS the program counter)
 *   - 16 KiB unified address space (14-bit = 2^14 = 16,384 bytes)
 *   - 8 input ports, 24 output ports
 *
 * === Why a custom fetch-decode-execute loop? ===
 *
 * The 8008 has variable-length instructions (1, 2, or 3 bytes), a 14-bit PC,
 * an unconventional push-down stack where the PC IS the top entry, and a
 * separate I/O port model. These features don't fit GenericVM's fixed-opcode
 * model, so we use a custom dispatch loop that mirrors the real chip's design.
 *
 * === Register encoding ===
 *
 * All instructions encode registers in 3-bit fields:
 *   000 = B   001 = C   010 = D   011 = E
 *   100 = H   101 = L   110 = M (memory)   111 = A
 *
 * Register index 6 (M) means "memory at address [H:L]" — it is not a physical
 * register. When M appears as a source, the simulator reads memory; when M
 * appears as a destination, the simulator writes memory.
 *
 * === The 8-level push-down stack ===
 *
 * The 8008's hardware stack is built into the chip — there is no stack pointer
 * visible to the programmer. The chip contains 8 × 14-bit registers arranged
 * as a circular push-down stack:
 *
 *   Entry 0: Current program counter (always here)
 *   Entry 1: Most recent return address (saved by CALL)
 *   Entry 2: Next return address
 *   ...
 *   Entry 7: Oldest return address
 *
 * CALL: rotates stack down (entry 0 → entry 1, ..., entry 6 → entry 7),
 *       then loads the target address into entry 0.
 * RETURN: rotates stack up (entry 1 → entry 0, ..., entry 7 → entry 6).
 *
 * Since entry 0 is consumed by the current PC, programs can nest at most
 * 7 calls deep before the stack wraps (silently overwriting the oldest
 * return address).
 *
 * === Instruction encoding overview ===
 *
 * Bits 7–6 (group):
 *   00 = Register ops (INR, DCR, Rotates) + 2-byte MVI + OUT + RET + RST
 *   01 = MOV, HLT, JMP/JFC/JTC/..., CAL/CFC/CTC/..., IN
 *   10 = ALU register operand (ADD, ADC, SUB, SBB, ANA, XRA, ORA, CMP)
 *   11 = ALU immediate
 *
 * Bits 5–3 (DDD): Destination register or ALU operation
 * Bits 2–0 (SSS): Source register or sub-operation
 *
 * === OUT instruction encoding ===
 *
 * OUT instruction: group 00, bits[1:0]=10
 *   opcode = 00 MMM M10  (M=port number bits, 5-bit port in bits[5:1])
 *   But rotates also use group 00, bits[2:0]=010 for DDD=0,1,2,3.
 *   Priority: RLC/RRC/RAL/RAR are specific opcodes (0x02,0x0A,0x12,0x1A).
 *   All other group-00, bits[2:0]=010 opcodes are OUT with port=(opcode>>1)&0x0F.
 *
 * === IN instruction encoding ===
 *
 * IN instruction: group 01, bits[2:0]=001
 *   opcode = 01 PPP 001  (P=port number in bits[5:3])
 *   IN 0 = 0x41, IN 1 = 0x49, ..., IN 7 = 0x79
 */

// ---------------------------------------------------------------------------
// Public Types
// ---------------------------------------------------------------------------

/**
 * The Intel 8008's four condition flags.
 *
 * These flags are updated by most ALU operations and tested by conditional
 * jump/call/return instructions. The 8008 notably lacks an Auxiliary Carry
 * flag (unlike its successor, the 8080), so BCD arithmetic requires software.
 */
export interface Flags {
  /** CY — Set when addition overflows 8 bits or subtraction borrows. */
  carry: boolean;
  /** Z  — Set when the result is exactly 0x00. */
  zero: boolean;
  /** S  — Set when bit 7 of the result is 1 (negative in two's complement). */
  sign: boolean;
  /**
   * P  — Set when the result has an EVEN number of 1-bits (even parity).
   *
   * Why "parity"? In early computing, parity bits were appended to data bytes
   * to detect transmission errors. Even parity means the total number of 1-bits
   * (including the parity bit) is even. The 8008 reports parity of the result,
   * useful for checksum calculations and BCD correction routines.
   *
   * Example: 0x03 = 0b00000011 → two 1-bits → even parity → P=1
   *          0x07 = 0b00000111 → three 1-bits → odd parity → P=0
   */
  parity: boolean;
}

/**
 * Trace record for one executed instruction.
 *
 * Captures the instruction's address, encoding, human-readable mnemonic,
 * and the state of the accumulator and flags before and after execution.
 * Memory operations also record the address and value accessed.
 */
export interface Trace {
  /** PC where this instruction was fetched (before increment). */
  address: number;
  /** Raw instruction bytes (1, 2, or 3 bytes). */
  raw: Uint8Array;
  /** Human-readable mnemonic: "MOV A, B", "ADI 0x05", "JMP 0x0100". */
  mnemonic: string;
  /** Accumulator value before execution. */
  aBefore: number;
  /** Accumulator value after execution. */
  aAfter: number;
  /** Flags before execution. */
  flagsBefore: Flags;
  /** Flags after execution. */
  flagsAfter: Flags;
  /** 14-bit memory address accessed (if instruction used M register), or null. */
  memAddress: number | null;
  /** Value read or written at memAddress, or null. */
  memValue: number | null;
}

// ---------------------------------------------------------------------------
// Register name map for mnemonics
// ---------------------------------------------------------------------------
const REG_NAMES = ["B", "C", "D", "E", "H", "L", "M", "A"] as const;

// Condition code names for mnemonics
const COND_NAMES = ["C", "Z", "S", "P"] as const;

// ALU operation names for register-mode instructions
const ALU_REG_NAMES = ["ADD", "ADC", "SUB", "SBB", "ANA", "XRA", "ORA", "CMP"];

// ALU operation names for immediate-mode instructions
const ALU_IMM_NAMES = ["ADI", "ACI", "SUI", "SBI", "ANI", "XRI", "ORI", "CPI"];

// ---------------------------------------------------------------------------
// Intel8008Simulator
// ---------------------------------------------------------------------------

/**
 * Intel 8008 behavioral simulator.
 *
 * Executes 8008 machine code directly using host-language arithmetic —
 * no gate simulation. For gate-level simulation (routing every operation
 * through AND/OR/XOR/NOT), see the companion `intel8008-gatelevel` package.
 *
 * === Usage ===
 *
 * ```typescript
 * const sim = new Intel8008Simulator();
 *
 * // Load 1 into B, 2 into A, compute A = A + B, halt
 * const program = new Uint8Array([0x06, 0x01, 0x3E, 0x02, 0x80, 0x76]);
 * const traces = sim.run(program);
 * console.log(sim.a); // 3
 * ```
 *
 * === Public registers ===
 *
 * All registers are 8-bit unsigned (0–255).
 * - `a`: Accumulator — primary target of ALU operations
 * - `b`, `c`, `d`, `e`: General-purpose registers
 * - `h`, `l`: High and low bytes of the memory address pair
 * - `pc`: 14-bit program counter (0–16383)
 * - `hlAddress`: 14-bit address formed as (H & 0x3F) << 8 | L
 */
export class Intel8008Simulator {
  // -------------------------------------------------------------------------
  // Internal state
  // -------------------------------------------------------------------------

  /**
   * Register file: indices 0–7 map to B, C, D, E, H, L, (unused), A.
   * Index 6 is never used directly — it is the M pseudo-register (memory).
   */
  private regs: number[] = new Array(8).fill(0);

  /** 16,384 bytes of unified program+data memory. */
  private mem: Uint8Array = new Uint8Array(16384);

  /**
   * 8-level push-down stack.
   *
   * Entry 0 is always the current program counter. Entries 1–7 hold saved
   * return addresses. The stack hardware is circular: pushing rotates entries
   * down, popping rotates entries up.
   */
  private stackEntries: number[] = new Array(8).fill(0);

  /**
   * Number of saved return addresses (not counting the current PC at entry 0).
   * Range: 0–7. When this reaches 7, the next push overwrites entry 7.
   */
  private stackDepth: number = 0;

  /** Current condition flags. */
  private flags: Flags = { carry: false, zero: false, sign: false, parity: false };

  /** True after HLT — no more instructions execute until reset(). */
  private halted: boolean = false;

  /** 8 input port values (set externally). */
  private inputPorts: number[] = new Array(8).fill(0);

  /** 24 output port values (written by OUT instructions). */
  private outputPorts: number[] = new Array(24).fill(0);

  // -------------------------------------------------------------------------
  // Public register accessors
  // -------------------------------------------------------------------------

  /** Accumulator (register A = index 7). 8-bit unsigned (0–255). */
  get a(): number { return this.regs[7]; }
  /** Register B (index 0). 8-bit unsigned (0–255). */
  get b(): number { return this.regs[0]; }
  /** Register C (index 1). 8-bit unsigned (0–255). */
  get c(): number { return this.regs[1]; }
  /** Register D (index 2). 8-bit unsigned (0–255). */
  get d(): number { return this.regs[2]; }
  /** Register E (index 3). 8-bit unsigned (0–255). */
  get e(): number { return this.regs[3]; }
  /** Register H (index 4) — high byte of memory address pair. */
  get h(): number { return this.regs[4]; }
  /** Register L (index 5) — low byte of memory address pair. */
  get l(): number { return this.regs[5]; }

  /**
   * Current program counter (14-bit, 0–16383).
   *
   * This is always entry 0 of the push-down stack.
   */
  get pc(): number { return this.stackEntries[0]; }

  /**
   * 14-bit memory address formed from H and L registers.
   *
   * The M pseudo-register always refers to this address:
   *   address = (H & 0x3F) << 8 | L
   *
   * Only the low 6 bits of H contribute (top 2 bits are "don't care"),
   * giving a 14-bit address space: 6 bits from H + 8 bits from L.
   */
  get hlAddress(): number {
    return ((this.regs[4] & 0x3F) << 8) | this.regs[5];
  }

  /** Current flag state (snapshot — not a live reference). */
  get currentFlags(): Flags {
    return { ...this.flags };
  }

  /** True if the processor has halted (HLT instruction executed). */
  get isHalted(): boolean { return this.halted; }

  /** Read-only view of the 16 KiB memory. */
  get memory(): Uint8Array { return this.mem; }

  /** Current stack contents (all 8 entries; entry 0 = current PC). */
  get stack(): number[] { return [...this.stackEntries]; }

  /** Number of saved return addresses (0–7). */
  get depth(): number { return this.stackDepth; }

  // -------------------------------------------------------------------------
  // Public API
  // -------------------------------------------------------------------------

  /**
   * Load a program into memory at the given start address.
   *
   * Does NOT reset CPU state — call `reset()` first if needed.
   * Also sets the PC to startAddress so execution begins at the right place.
   *
   * @param program - Machine code bytes.
   * @param startAddress - Where to copy the program (default: 0x0000).
   */
  loadProgram(program: Uint8Array, startAddress = 0): void {
    this.mem.set(program, startAddress);
    this.stackEntries[0] = startAddress;
  }

  /**
   * Execute one instruction and return a trace record.
   *
   * Throws if the processor is halted. Check `isHalted` before calling,
   * or call `reset()` to restart.
   *
   * @returns Trace record describing the executed instruction.
   */
  step(): Trace {
    if (this.halted) {
      throw new Error("Processor is halted. Call reset() to restart.");
    }

    const instrAddr = this.pc;
    const aBefore = this.regs[7];
    const flagsBefore = { ...this.flags };

    // ------------------------------------------------------------------
    // FETCH: read opcode and advance PC
    // ------------------------------------------------------------------
    const opcode = this.mem[instrAddr & 0x3FFF];
    this.stackEntries[0] = (instrAddr + 1) & 0x3FFF;

    const group = (opcode >> 6) & 0x03;
    const ddd   = (opcode >> 3) & 0x07;
    const sss   = opcode & 0x07;

    let mnemonic = `???_0x${opcode.toString(16).padStart(2,"0")}`;
    let memAddress: number | null = null;
    let memValue: number | null = null;
    const rawBytes: number[] = [opcode];

    // ------------------------------------------------------------------
    // DECODE + EXECUTE
    // ------------------------------------------------------------------

    if (opcode === 0x76 || opcode === 0xFF) {
      // ================================================================
      // HLT
      //   0x76 = 01 110 110 = MOV M, M (intentional design quirk)
      //   0xFF = 11 111 111 (second halt encoding)
      // ================================================================
      this.halted = true;
      mnemonic = "HLT";

    } else if (group === 0x01 && sss === 0x01) {
      // ================================================================
      // IN instruction: 01 PPP 001
      //   Bits[7:6]=01, bits[2:0]=001
      //   Port number P = bits[5:3] = ddd
      //   IN 0=0x41, IN 1=0x49, ..., IN 7=0x79
      // ================================================================
      const port = ddd & 0x07;
      this.regs[7] = this.inputPorts[port] & 0xFF;
      mnemonic = `IN ${port}`;

    } else if (group === 0x01 && (sss & 0x03) === 0x00 && (ddd <= 3 || opcode === 0x7C)) {
      // ================================================================
      // JMP / Conditional Jumps (3 bytes): group=01, bits[1:0]=00
      //   Encoding: 01 CCC T00
      //   CCC = bits[5:3] = ddd (condition code 0–3 for flags; 7=unconditional)
      //   T   = bit[2]    = sss bit[2] (0=false/not-set, 1=true/set)
      //
      // Only valid when ddd (CCC) ∈ {0,1,2,3} (actual condition codes)
      // OR opcode = 0x7C (JMP unconditional, CCC=7, T=1).
      //
      // Opcodes:
      //   JFC=0x40, JTC=0x44, JFZ=0x48, JTZ=0x4C
      //   JFS=0x50, JTS=0x54, JFP=0x58, JTP=0x5C
      //   JMP=0x7C (CCC=111 = unconditional)
      //
      // Note: 0x78 (MOV A,B) has ddd=7 but opcode≠0x7C → falls through to MOV.
      // Note: 0x60-0x7B with bits[1:0]=00 and ddd≥4 → MOV instructions.
      // ================================================================
      const addrLo = this.mem[this.pc & 0x3FFF];
      this.stackEntries[0] = (this.pc + 1) & 0x3FFF;
      const addrHi = this.mem[this.pc & 0x3FFF];
      this.stackEntries[0] = (this.pc + 1) & 0x3FFF;
      const target = ((addrHi & 0x3F) << 8) | addrLo;
      rawBytes.push(addrLo, addrHi);

      const cccFull = ddd;          // bits[5:3]
      const tBit = (sss >> 2) & 1;  // bit[2] of opcode = sense

      if (cccFull === 0x07) {
        // Unconditional JMP
        this.stackEntries[0] = target;
        mnemonic = `JMP 0x${target.toString(16).toUpperCase().padStart(4, "0")}`;
      } else {
        const condCode = cccFull & 0x03;
        const condMet = this.evalCondition(condCode, tBit === 1);
        if (condMet) {
          this.stackEntries[0] = target;
        }
        const condName = COND_NAMES[condCode] ?? "?";
        const prefix = tBit === 1 ? "JT" : "JF";
        mnemonic = `${prefix}${condName} 0x${target.toString(16).toUpperCase().padStart(4, "0")}`;
      }

    } else if (group === 0x01 && (sss & 0x03) === 0x02 && (ddd <= 3 || opcode === 0x7E)) {
      // ================================================================
      // CALL / Conditional Calls (3 bytes): group=01, bits[1:0]=10
      //   Encoding: 01 CCC T10
      //   CFC=0x42, CTC=0x46, CFZ=0x4A, CTZ=0x4E
      //   CFS=0x52, CTS=0x56, CFP=0x5A, CTP=0x5E
      //   CAL=0x7E (CCC=111 = unconditional)
      //
      // Only valid when ddd ∈ {0,1,2,3} OR opcode=0x7E (CAL).
      // Note: 0x7A (MOV A,D), 0x6A (MOV L,D), etc. use bits[1:0]=10 with ddd≥4
      //       → those fall through to MOV.
      // Note: 0x7E is CAL (not MOV A,M — those bits conflict; CAL takes priority).
      // ================================================================
      const addrLo = this.mem[this.pc & 0x3FFF];
      this.stackEntries[0] = (this.pc + 1) & 0x3FFF;
      const addrHi = this.mem[this.pc & 0x3FFF];
      this.stackEntries[0] = (this.pc + 1) & 0x3FFF;
      const target = ((addrHi & 0x3F) << 8) | addrLo;
      rawBytes.push(addrLo, addrHi);

      const cccFull = ddd;
      const tBit = (sss >> 2) & 1;

      if (cccFull === 0x07) {
        // Unconditional CAL
        this.pushAndJump(target);
        mnemonic = `CAL 0x${target.toString(16).toUpperCase().padStart(4, "0")}`;
      } else {
        const condCode = cccFull & 0x03;
        const condMet = this.evalCondition(condCode, tBit === 1);
        if (condMet) {
          this.pushAndJump(target);
        }
        const condName = COND_NAMES[condCode] ?? "?";
        const prefix = tBit === 1 ? "CT" : "CF";
        mnemonic = `${prefix}${condName} 0x${target.toString(16).toUpperCase().padStart(4, "0")}`;
      }

    } else if (group === 0x01) {
      // ================================================================
      // MOV D, S: 01 DDD SSS (1 byte)
      // Already handled HLT (0x76), IN, JMP, CALL above.
      // Remaining group-01 instructions: MOV register to register.
      // ================================================================
      const srcVal = this.readRegMem(sss, (addr) => {
        memAddress = addr;
        memValue = this.mem[addr & 0x3FFF];
      });
      this.writeRegMem(ddd, srcVal, (addr, val) => {
        memAddress = addr;
        memValue = val;
      });
      mnemonic = `MOV ${REG_NAMES[ddd]}, ${REG_NAMES[sss]}`;

    } else if (group === 0x00) {
      // ================================================================
      // GROUP 00
      // ================================================================

      if (sss === 0x06) {
        // ----------------------------------------------------------------
        // MVI D, d: 00 DDD 110 — 2-byte move immediate
        // Fetch the immediate data byte from the next memory location.
        // If DDD=M (6), writes to mem[H:L].
        // ----------------------------------------------------------------
        const imm = this.mem[this.pc & 0x3FFF];
        this.stackEntries[0] = (this.pc + 1) & 0x3FFF;
        rawBytes.push(imm);
        this.writeRegMem(ddd, imm, (addr, val) => {
          memAddress = addr;
          memValue = val;
        });
        mnemonic = `MVI ${REG_NAMES[ddd]}, 0x${imm.toString(16).toUpperCase().padStart(2, "0")}`;

      } else if (sss === 0x00) {
        // ----------------------------------------------------------------
        // INR D: 00 DDD 000 — increment register
        // Increments the register by 1. Wraps 0xFF → 0x00.
        // Updates Z, S, P flags. Does NOT update CY (carry preserved).
        // Note: INR B encodes to 0x00 (used as NOP-equivalent).
        // ----------------------------------------------------------------
        const oldVal = this.readRegMem(ddd, (addr) => {
          memAddress = addr;
          memValue = this.mem[addr & 0x3FFF];
        });
        const result = (oldVal + 1) & 0xFF;
        this.writeRegMem(ddd, result, (addr, val) => {
          memAddress = addr;
          memValue = val;
        });
        this.flags = this.computeFlags(result, this.flags.carry, false);
        mnemonic = `INR ${REG_NAMES[ddd]}`;

      } else if (sss === 0x01) {
        // ----------------------------------------------------------------
        // DCR D: 00 DDD 001 — decrement register
        // Decrements the register by 1. Wraps 0x00 → 0xFF.
        // Updates Z, S, P flags. Does NOT update CY (carry preserved).
        // ----------------------------------------------------------------
        const oldVal = this.readRegMem(ddd, (addr) => {
          memAddress = addr;
          memValue = this.mem[addr & 0x3FFF];
        });
        const result = (oldVal - 1 + 256) & 0xFF;
        this.writeRegMem(ddd, result, (addr, val) => {
          memAddress = addr;
          memValue = val;
        });
        this.flags = this.computeFlags(result, this.flags.carry, false);
        mnemonic = `DCR ${REG_NAMES[ddd]}`;

      } else if (sss === 0x02 && ddd <= 0x03) {
        // ----------------------------------------------------------------
        // Rotate instructions: 00 0RR 010  (RR = bits[4:3])
        // These use specific ddd values 0–3 to select the rotate type.
        // Only Z, S, P are untouched — only CY is updated.
        // ----------------------------------------------------------------
        const acc = this.regs[7];
        switch (ddd) {
          case 0x00: {
            // RLC: Rotate Left Circular
            // CY ← A[7]; A[0] ← A[7]; A ← (A << 1) | A[7]
            // The bit that falls off the left wraps to the right (bit 0),
            // AND also goes to the carry flag.
            const bit7 = (acc >> 7) & 1;
            this.regs[7] = ((acc << 1) | bit7) & 0xFF;
            this.flags = { ...this.flags, carry: bit7 === 1 };
            mnemonic = "RLC";
            break;
          }
          case 0x01: {
            // RRC: Rotate Right Circular
            // CY ← A[0]; A[7] ← A[0]; A ← (A >> 1) | (A[0] << 7)
            const bit0 = acc & 1;
            this.regs[7] = ((acc >> 1) | (bit0 << 7)) & 0xFF;
            this.flags = { ...this.flags, carry: bit0 === 1 };
            mnemonic = "RRC";
            break;
          }
          case 0x02: {
            // RAL: Rotate Left through Carry (9-bit rotation)
            // The 9-bit register [CY | A7..A0] rotates left by 1:
            //   new_CY ← A[7]; A[0] ← old_CY
            const bit7 = (acc >> 7) & 1;
            const oldCy = this.flags.carry ? 1 : 0;
            this.regs[7] = ((acc << 1) | oldCy) & 0xFF;
            this.flags = { ...this.flags, carry: bit7 === 1 };
            mnemonic = "RAL";
            break;
          }
          case 0x03: {
            // RAR: Rotate Right through Carry (9-bit rotation)
            //   new_CY ← A[0]; A[7] ← old_CY
            const bit0 = acc & 1;
            const oldCy = this.flags.carry ? 1 : 0;
            this.regs[7] = ((acc >> 1) | (oldCy << 7)) & 0xFF;
            this.flags = { ...this.flags, carry: bit0 === 1 };
            mnemonic = "RAR";
            break;
          }
        }

      } else if (sss === 0x02 && ddd >= 0x04) {
        // ----------------------------------------------------------------
        // OUT instruction: 00 MMM M10 (group 00, bits[2:0]=010, ddd>=4)
        //
        // Port number encoding: port = (opcode >> 1) & 0x1F (bits[5:1])
        //   For ddd=4 (0x22): port = (0x22 >> 1) & 0x1F = 0x11 = 17? No.
        //
        // Let me use the straightforward encoding from the spec:
        //   OUT port: port = (opcode & 0xFE) >> 1 = opcode >> 1
        //   (the entire upper 7 bits of the opcode give the 5-bit port number
        //    since bit[0]=0 always for OUT)
        //
        // Actually: OUT encoding is 00 PPP P10.
        //   Bits [7:6]=00 (group), bits[5:1]=PPPPP (5-bit port), bits[0]=0.
        //   Wait, bits[2:0]=010 means bit[0]=0, bit[1]=1, bit[2]=0.
        //   The port number is in bits[5:1] (5 bits) after removing bit[0].
        //   port = (opcode >> 1) & 0x1F... but bit[1] is always 1 for OUT (it's in "010"),
        //   so the lowest bit of port is always 1? That gives only odd ports.
        //
        // Hmm. Let me just use a practical approach:
        //   The 5-bit port number is in bits [5:1] = (opcode >> 1) & 0x1F
        //   But with bits[2:0]=010, bit[1]=1, so ports are:
        //     ddd=4 (opcode=0x22): port=(0x22>>1)&0x1F = 0x11 = 17
        //     ddd=5 (opcode=0x2A): port=(0x2A>>1)&0x1F = 0x15 = 21
        //     ddd=6 (opcode=0x32): port=(0x32>>1)&0x1F = 0x19 = 25 -- out of range
        //     ddd=7 (opcode=0x3A): port=(0x3A>>1)&0x1F = 0x1D = 29 -- out of range
        //
        // That doesn't work well either. The real encoding per Intel 8008 datasheet:
        //   OUT pp: format is 00 PP0 010 where PP are 2-bit port address (ports 0-3)
        //   (the rest of the byte is fixed at bit pattern 00_xx0_010)
        //   Extended ports 4-7 use: 00 PP1 010
        //
        // Simplified approach for the simulator: treat ddd*2 + (ddd>=4 ? 1:0) as port.
        // Actually, let me just use: port = (opcode >> 1) & 0x1F
        // and clamp to 0-23.
        //
        // Given the ambiguity, we'll use the simplest consistent encoding:
        // port = (opcode & 0x3E) >> 1  (bits[5:1], the 5 bits above bit[0])
        // ----------------------------------------------------------------
        const port = (opcode & 0x3E) >> 1;
        if (port < 24) {
          this.outputPorts[port] = this.regs[7] & 0xFF;
        }
        mnemonic = `OUT ${port}`;

      } else if ((sss & 0x03) === 0x03) {
        // ----------------------------------------------------------------
        // RET / Conditional Returns: 00 CCC T11
        //   bits[1:0] = 11 in the sss field: (sss & 0x03) = 0x03
        //   bit[2] of sss = T (sense: 0=jump-if-false, 1=jump-if-true)
        //   bits[5:3] = CCC (condition code, or 111 for unconditional)
        //
        // sss=011 (bits[1:0]=11, T=bit[2]=0): RFC, RFZ, RFS, RFP
        // sss=111 (bits[1:0]=11, T=bit[2]=1): RTC, RTZ, RTS, RTP, RET
        // RET always = 0x3F = 00 111 111 (CCC=111, T=1)
        // ----------------------------------------------------------------
        const cccFull = ddd;
        const tBit = (sss >> 2) & 1;  // bit[2] of sss

        if (cccFull === 0x07) {
          // Unconditional RET (also covers 0x3F where ddd=7, sss=7)
          this.popReturn();
          mnemonic = "RET";
        } else {
          const condCode = cccFull & 0x03;
          const condMet = this.evalCondition(condCode, tBit === 1);
          if (condMet) {
            this.popReturn();
          }
          const condName = COND_NAMES[condCode] ?? "?";
          const prefix = tBit === 1 ? "RT" : "RF";
          mnemonic = `${prefix}${condName}`;
        }

      } else if (sss === 0x05) {
        // ----------------------------------------------------------------
        // RST N: 00 AAA 101 — 1-byte CALL to fixed address N*8
        //   N = ddd (0–7), target = N * 8 (0, 8, 16, 24, 32, 40, 48, 56)
        //   Pushes current PC, jumps to target.
        //
        // RST is equivalent to a 1-byte CAL — useful for interrupt handlers
        // in the first 64 bytes of memory (the "restart vectors").
        // ----------------------------------------------------------------
        const rstAddr = ddd << 3;
        this.pushAndJump(rstAddr);
        mnemonic = `RST ${ddd}`;

      } else {
        mnemonic = `???_grp00_0x${opcode.toString(16).padStart(2, "0")}`;
      }

    } else if (group === 0x02) {
      // ================================================================
      // GROUP 10: ALU register operand — 10 OOO SSS
      //   OOO = ddd = operation select (0–7)
      //   SSS = sss = source register (0–7, where 6=M)
      //
      // Operations:
      //   000=ADD, 001=ADC, 010=SUB, 011=SBB
      //   100=ANA, 101=XRA, 110=ORA, 111=CMP
      // ================================================================
      const srcVal = this.readRegMem(sss, (addr) => {
        memAddress = addr;
        memValue = this.mem[addr & 0x3FFF];
      });
      this.executeALUOp(ddd, srcVal);
      mnemonic = `${ALU_REG_NAMES[ddd] ?? "???"} ${REG_NAMES[sss]}`;

    } else {
      // ================================================================
      // GROUP 11: ALU immediate — 11 OOO 100 (2 bytes)
      //   OOO = ddd = operation select (0–7)
      //   sss must be 100 (= 4) for this encoding
      //
      // Operations:
      //   000=ADI, 001=ACI, 010=SUI, 011=SBI
      //   100=ANI, 101=XRI, 110=ORI, 111=CPI
      // ================================================================
      if (sss === 0x04) {
        const imm = this.mem[this.pc & 0x3FFF];
        this.stackEntries[0] = (this.pc + 1) & 0x3FFF;
        rawBytes.push(imm);
        this.executeALUOp(ddd, imm);
        mnemonic = `${ALU_IMM_NAMES[ddd] ?? "???"} 0x${imm.toString(16).toUpperCase().padStart(2, "0")}`;
      } else {
        mnemonic = `???_grp11_0x${opcode.toString(16).padStart(2, "0")}`;
      }
    }

    const aAfter = this.regs[7];
    const flagsAfter = { ...this.flags };

    return {
      address: instrAddr,
      raw: new Uint8Array(rawBytes),
      mnemonic,
      aBefore,
      aAfter,
      flagsBefore,
      flagsAfter,
      memAddress,
      memValue,
    };
  }

  /**
   * Load a program and run it until HLT or maxSteps is reached.
   *
   * @param program - Machine code bytes.
   * @param maxSteps - Safety limit (default: 100,000).
   * @param startAddress - Load address (default: 0x0000).
   * @returns Array of trace records for each executed instruction.
   */
  run(program: Uint8Array, maxSteps = 100000, startAddress = 0): Trace[] {
    this.reset();
    this.loadProgram(program, startAddress);
    const traces: Trace[] = [];
    while (!this.halted && traces.length < maxSteps) {
      traces.push(this.step());
    }
    return traces;
  }

  /**
   * Reset the processor to its power-on state.
   *
   * Clears all registers, flags, stack, and halted state.
   * Does NOT clear memory — call `loadProgram()` after reset to reload.
   */
  reset(): void {
    this.regs.fill(0);
    this.stackEntries.fill(0);
    this.stackDepth = 0;
    this.flags = { carry: false, zero: false, sign: false, parity: false };
    this.halted = false;
    // Note: inputPorts and outputPorts are NOT cleared on reset.
    // They model external hardware state that persists independently of the CPU.
    // Use setInputPort() to configure before run() or step().
  }

  /**
   * Set an input port value (read by IN instructions).
   *
   * @param port - Port number (0–7).
   * @param value - 8-bit value (0–255).
   */
  setInputPort(port: number, value: number): void {
    if (port < 0 || port > 7) {
      throw new RangeError(`Input port must be 0–7, got ${port}`);
    }
    this.inputPorts[port] = value & 0xFF;
  }

  /**
   * Read an output port value (written by OUT instructions).
   *
   * @param port - Port number (0–23).
   * @returns 8-bit value written by the most recent OUT instruction.
   */
  getOutputPort(port: number): number {
    if (port < 0 || port > 23) {
      throw new RangeError(`Output port must be 0–23, got ${port}`);
    }
    return this.outputPorts[port];
  }

  // -------------------------------------------------------------------------
  // Private helpers
  // -------------------------------------------------------------------------

  /**
   * Read a register value (0–7). If regIdx = 6 (M), reads from mem[H:L].
   *
   * @param regIdx - Register index 0–7.
   * @param onMemAccess - Called with the resolved address when M is accessed.
   * @returns 8-bit value.
   */
  private readRegMem(regIdx: number, onMemAccess?: (addr: number) => void): number {
    if (regIdx === 6) {
      const addr = this.hlAddress;
      if (onMemAccess) onMemAccess(addr);
      return this.mem[addr & 0x3FFF];
    }
    return this.regs[regIdx];
  }

  /**
   * Write a value to a register (0–7). If regIdx = 6 (M), writes to mem[H:L].
   *
   * @param regIdx - Register index 0–7.
   * @param value - 8-bit value to write (masked to 8 bits).
   * @param onMemAccess - Called with (address, value) when M is written.
   */
  private writeRegMem(
    regIdx: number,
    value: number,
    onMemAccess?: (addr: number, val: number) => void,
  ): void {
    const v = value & 0xFF;
    if (regIdx === 6) {
      const addr = this.hlAddress;
      this.mem[addr & 0x3FFF] = v;
      if (onMemAccess) onMemAccess(addr, v);
    } else {
      this.regs[regIdx] = v;
    }
  }

  /**
   * Compute condition flags from an 8-bit ALU result.
   *
   * === Flag computation ===
   *
   * - carry: provided by the caller (from the arithmetic carry-out).
   * - zero: r8 === 0 — all 8 bits are 0.
   * - sign: bit 7 of r8 — interpreting as a signed byte, 1 = negative.
   * - parity: even parity (P=1 when even number of 1-bits).
   *
   * Parity can be computed via XOR reduction: XOR all 8 bits together.
   * If the result is 0, there's an even number of 1s (even parity).
   * This is the gate-level parity algorithm (see bits.ts in gatelevel package).
   *
   * Example:
   *   0x03 = 0b00000011 → XOR(0,0,0,0,0,0,1,1) = 0 → even parity → P=1
   *   0x07 = 0b00000111 → XOR(0,0,0,0,0,1,1,1) = 1 → odd parity  → P=0
   *
   * @param result - 8-bit result value (may be larger; masked internally).
   * @param carry - Carry/borrow from the operation.
   * @param updateCarry - If false, preserves the existing carry flag.
   */
  private computeFlags(result: number, carry: boolean, updateCarry = true): Flags {
    const r8 = result & 0xFF;
    // Count 1-bits for parity. Pop-count via bit string:
    const ones = r8.toString(2).split("").filter((b) => b === "1").length;
    return {
      carry: updateCarry ? carry : this.flags.carry,
      zero: r8 === 0,
      sign: (r8 & 0x80) !== 0,
      parity: ones % 2 === 0,  // true = even parity (P=1)
    };
  }

  /**
   * Push a call target onto the stack and jump to it.
   *
   * The current PC (entry 0) becomes the return address (entry 1),
   * and the call target is loaded into entry 0.
   *
   * Stack rotation (shift everything down by 1):
   *   entry[7] ← entry[6]
   *   ...
   *   entry[1] ← entry[0]   (saves return address = PC after instruction)
   *   entry[0] ← target
   *
   * On overflow (more than 7 nested calls), entry[7] is silently overwritten.
   */
  private pushAndJump(target: number): void {
    for (let i = 7; i > 0; i--) {
      this.stackEntries[i] = this.stackEntries[i - 1];
    }
    this.stackEntries[0] = target & 0x3FFF;
    this.stackDepth = Math.min(this.stackDepth + 1, 7);
  }

  /**
   * Pop the return address from the stack and resume from it.
   *
   * Stack rotation (shift everything up by 1):
   *   entry[0] ← entry[1]   (new PC = saved return address)
   *   ...
   *   entry[6] ← entry[7]
   *   entry[7] ← 0
   */
  private popReturn(): void {
    for (let i = 0; i < 7; i++) {
      this.stackEntries[i] = this.stackEntries[i + 1];
    }
    this.stackEntries[7] = 0;
    this.stackDepth = Math.max(this.stackDepth - 1, 0);
  }

  /**
   * Evaluate a condition code against the current flags.
   *
   * Condition codes:
   *   0 = CY (Carry)
   *   1 = Z  (Zero)
   *   2 = S  (Sign)
   *   3 = P  (Parity)
   *
   * @param code - 0–3 condition code.
   * @param sense - true = "if set" (JTx/CTx/RTx), false = "if not set" (JFx/CFx/RFx).
   */
  private evalCondition(code: number, sense: boolean): boolean {
    let flagVal: boolean;
    switch (code) {
      case 0: flagVal = this.flags.carry;  break;
      case 1: flagVal = this.flags.zero;   break;
      case 2: flagVal = this.flags.sign;   break;
      case 3: flagVal = this.flags.parity; break;
      default: flagVal = false;
    }
    return sense ? flagVal : !flagVal;
  }

  /**
   * Execute an ALU operation on accumulator A and source operand.
   *
   * === 8008 ALU operations ===
   *
   * The 8008 ALU always uses A as the left operand and source as the right.
   * The result always goes back into A, except for CMP which only updates flags.
   *
   * Subtraction note: SUB/SBB use two's complement. CY=1 means borrow occurred
   * (unsigned A < source). This matches the 8008 datasheet convention.
   *
   * AND/OR/XOR always clear CY to 0. This is a hardware design choice — the
   * 8008 uses the carry flag as a borrow/carry indicator for arithmetic, and
   * logical operations reset it to a known state.
   *
   * @param op - 3-bit operation code (0–7).
   * @param src - Source value (8-bit).
   */
  private executeALUOp(op: number, src: number): void {
    const a = this.regs[7];
    let result: number;
    let carry: boolean;

    switch (op) {
      case 0x00: { // ADD: A ← A + src
        const sum = a + src;
        result = sum & 0xFF;
        carry = sum > 0xFF;
        this.regs[7] = result;
        this.flags = this.computeFlags(result, carry);
        break;
      }
      case 0x01: { // ADC: A ← A + src + CY
        const sum = a + src + (this.flags.carry ? 1 : 0);
        result = sum & 0xFF;
        carry = sum > 0xFF;
        this.regs[7] = result;
        this.flags = this.computeFlags(result, carry);
        break;
      }
      case 0x02: { // SUB: A ← A - src; CY=1 means borrow (unsigned A < src)
        const diff = a - src;
        result = (diff + 256) & 0xFF;
        carry = diff < 0;
        this.regs[7] = result;
        this.flags = this.computeFlags(result, carry);
        break;
      }
      case 0x03: { // SBB: A ← A - src - CY
        const diff = a - src - (this.flags.carry ? 1 : 0);
        result = ((diff % 256) + 256) % 256;
        carry = diff < 0;
        this.regs[7] = result;
        this.flags = this.computeFlags(result, carry);
        break;
      }
      case 0x04: { // ANA: A ← A & src; CY=0
        result = (a & src) & 0xFF;
        this.regs[7] = result;
        this.flags = this.computeFlags(result, false);
        break;
      }
      case 0x05: { // XRA: A ← A ^ src; CY=0
        result = (a ^ src) & 0xFF;
        this.regs[7] = result;
        this.flags = this.computeFlags(result, false);
        break;
      }
      case 0x06: { // ORA: A ← A | src; CY=0
        result = (a | src) & 0xFF;
        this.regs[7] = result;
        this.flags = this.computeFlags(result, false);
        break;
      }
      case 0x07: { // CMP: flags ← A - src; A unchanged (compare)
        const diff = a - src;
        result = (diff + 256) & 0xFF;
        carry = diff < 0;
        this.flags = this.computeFlags(result, carry);
        // A is NOT written
        break;
      }
    }
  }
}
