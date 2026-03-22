/**
 * Busicom 141-PF Calculator ROM Program
 *
 * === Historical Context ===
 *
 * The Busicom 141-PF was the first commercial product powered by the Intel 4004
 * microprocessor (1971). Busicom contracted Intel to build a custom chip set for
 * their desktop printing calculator. Ted Hoff proposed replacing the planned
 * 12-chip design with a single general-purpose 4-bit processor — the 4004.
 *
 * The 4004's instruction set was designed specifically for BCD (Binary-Coded
 * Decimal) arithmetic. The `DAA` instruction (Decimal Adjust Accumulator) exists
 * because of this calculator. The `KBP` instruction was designed for scanning
 * the calculator's keypad.
 *
 * === What This ROM Does ===
 *
 * This is a simplified but authentic 4004 program that implements a 4-function
 * BCD calculator. It uses the same instruction patterns the original Busicom ROM
 * used: FIM/SRC for RAM addressing, ADD+DAA for BCD addition, ISZ for digit
 * loops, RDR for keyboard input, and WMP for display output.
 *
 * === Memory Layout ===
 *
 * The 4004's RAM is organized as:
 *   4 banks × 4 registers × 16 characters (each character is 4 bits)
 *
 * We use Bank 0:
 *   Register 0: Display buffer     (13 BCD digits, characters 0-12)
 *   Register 1: Input accumulator   (13 BCD digits, characters 0-12)
 *   Register 2: Second operand      (13 BCD digits, characters 0-12)
 *   Register 3, Char 0: Digit count (how many digits entered so far)
 *   Register 3, Char 1: Sign        (0 = positive, 1 = negative)
 *   Register 3, Char 2: Operation   (0=none, 1=add, 2=sub, 3=mul, 4=div)
 *   Register 3, Char 3: State       (0=entering first, 1=entering second)
 *
 * === Key Encoding ===
 *
 * The ROM port (read via RDR) encodes which key is pressed:
 *   0x0: No key (idle)
 *   0x1-0x9: Digits 1-9
 *   0xA: Digit 0
 *   0xB: Decimal point (not implemented in v1)
 *   0xC: Add (+)
 *   0xD: Subtract (-)
 *   0xE: Multiply (×)
 *   0xF: Equals (=)
 */

// ============================================================================
// Assembler — a tiny label-resolving assembler for the 4004
// ============================================================================
//
// Writing machine code by hand with hardcoded addresses is error-prone.
// This assembler lets us write instructions with symbolic labels:
//
//   asm.label("loop");
//   asm.LDM(5);
//   asm.JUN("loop");  // resolved to the actual address of "loop"
//
// All forward references are patched in a second pass.

class Asm4004 {
  /** The ROM image being built. */
  private rom = new Uint8Array(4096);
  /** Current write position. */
  private pos = 0;
  /** Map of label names to addresses. */
  private labels = new Map<string, number>();
  /** Forward references: [romOffset, labelName, type] */
  private fixups: Array<[number, string, "jun" | "jcn" | "jms" | "isz"]> = [];

  // --- Positioning ---

  /** Set the write position (for placing routines at fixed addresses). */
  org(address: number): void {
    this.pos = address;
  }

  /** Define a label at the current position. */
  label(name: string): void {
    this.labels.set(name, this.pos);
  }

  /** Get the current write position. */
  get here(): number {
    return this.pos;
  }

  // --- Emit raw bytes ---

  private emit1(byte: number): void {
    this.rom[this.pos++] = byte & 0xff;
  }

  private emit2(byte1: number, byte2: number): void {
    this.rom[this.pos++] = byte1 & 0xff;
    this.rom[this.pos++] = byte2 & 0xff;
  }

  // --- 1-byte instructions ---

  NOP(): void { this.emit1(0x00); }
  HLT(): void { this.emit1(0x01); }
  CLB(): void { this.emit1(0xf0); }
  CLC(): void { this.emit1(0xf1); }
  IAC(): void { this.emit1(0xf2); }
  CMC(): void { this.emit1(0xf3); }
  CMA(): void { this.emit1(0xf4); }
  RAL(): void { this.emit1(0xf5); }
  RAR(): void { this.emit1(0xf6); }
  TCC(): void { this.emit1(0xf7); }
  DAC(): void { this.emit1(0xf8); }
  TCS(): void { this.emit1(0xf9); }
  STC(): void { this.emit1(0xfa); }
  DAA(): void { this.emit1(0xfb); }
  KBP(): void { this.emit1(0xfc); }
  RDM(): void { this.emit1(0xe9); }
  WRM(): void { this.emit1(0xe0); }
  RDR(): void { this.emit1(0xea); }
  WRR(): void { this.emit1(0xe2); }
  WMP(): void { this.emit1(0xe1); }

  /** LD Rn — load register into accumulator. */
  LD(r: number): void { this.emit1(0xa0 | (r & 0xf)); }
  /** XCH Rn — exchange accumulator with register. */
  XCH(r: number): void { this.emit1(0xb0 | (r & 0xf)); }
  /** ADD Rn — add register to accumulator. */
  ADD(r: number): void { this.emit1(0x80 | (r & 0xf)); }
  /** SUB Rn — subtract register from accumulator. */
  SUB(r: number): void { this.emit1(0x90 | (r & 0xf)); }
  /** INC Rn — increment register. */
  INC(r: number): void { this.emit1(0x60 | (r & 0xf)); }
  /** LDM n — load immediate nibble. */
  LDM(n: number): void { this.emit1(0xd0 | (n & 0xf)); }
  /** BBL n — return from subroutine, load n. */
  BBL(n: number): void { this.emit1(0xc0 | (n & 0xf)); }
  /** SRC Pn — set RAM address from register pair. */
  SRC(pair: number): void { this.emit1(0x21 | ((pair & 0x7) << 1)); }

  // --- 2-byte instructions ---

  /** FIM Pn, data — load register pair with immediate. */
  FIM(pair: number, data: number): void {
    this.emit2(0x20 | ((pair & 0x7) << 1), data & 0xff);
  }

  /** JUN — unconditional jump (label or address). */
  JUN(target: string | number): void {
    if (typeof target === "number") {
      this.emit2(0x40 | ((target >> 8) & 0xf), target & 0xff);
    } else {
      const addr = this.labels.get(target);
      if (addr !== undefined) {
        this.emit2(0x40 | ((addr >> 8) & 0xf), addr & 0xff);
      } else {
        // Forward reference — emit placeholder, record fixup
        this.fixups.push([this.pos, target, "jun"]);
        this.emit2(0x40, 0x00);
      }
    }
  }

  /** JCN cond, target — conditional jump. */
  JCN(cond: number, target: string | number): void {
    if (typeof target === "number") {
      this.emit2(0x10 | (cond & 0xf), target & 0xff);
    } else {
      const addr = this.labels.get(target);
      if (addr !== undefined) {
        this.emit2(0x10 | (cond & 0xf), addr & 0xff);
      } else {
        this.fixups.push([this.pos, target, "jcn"]);
        this.emit2(0x10 | (cond & 0xf), 0x00);
      }
    }
  }

  /** JMS — call subroutine. */
  JMS(target: string | number): void {
    if (typeof target === "number") {
      this.emit2(0x50 | ((target >> 8) & 0xf), target & 0xff);
    } else {
      const addr = this.labels.get(target);
      if (addr !== undefined) {
        this.emit2(0x50 | ((addr >> 8) & 0xf), addr & 0xff);
      } else {
        this.fixups.push([this.pos, target, "jms"]);
        this.emit2(0x50, 0x00);
      }
    }
  }

  /** ISZ Rn, target — increment register, jump if not zero. */
  ISZ(r: number, target: string | number): void {
    if (typeof target === "number") {
      this.emit2(0x70 | (r & 0xf), target & 0xff);
    } else {
      const addr = this.labels.get(target);
      if (addr !== undefined) {
        this.emit2(0x70 | (r & 0xf), addr & 0xff);
      } else {
        this.fixups.push([this.pos, target, "isz"]);
        this.emit2(0x70 | (r & 0xf), 0x00);
      }
    }
  }

  // --- Finalize ---

  /** Resolve all forward references and return the ROM image. */
  build(): Uint8Array {
    for (const [offset, labelName, type] of this.fixups) {
      const addr = this.labels.get(labelName);
      if (addr === undefined) {
        throw new Error(`Undefined label: ${labelName}`);
      }
      switch (type) {
        case "jun":
        case "jms":
          // High nibble goes in first byte's low nibble
          this.rom[offset] = (this.rom[offset]! & 0xf0) | ((addr >> 8) & 0xf);
          this.rom[offset + 1] = addr & 0xff;
          break;
        case "jcn":
        case "isz":
          // Only low byte (8-bit address within page)
          this.rom[offset + 1] = addr & 0xff;
          break;
      }
    }
    return this.rom;
  }
}

// ============================================================================
// ROM Address Map
// ============================================================================

export const ROM_ADDRESSES = {
  /** 0x000: Main entry — init and jump to scan loop. */
  MAIN: 0x000,
  /** 0x010: Keyboard scan loop — reads key, dispatches. ~50 bytes. */
  KEY_SCAN: 0x010,
  /** 0x060: Digit entry — write digit to input buffer. */
  DIGIT_ENTRY: 0x060,
  /** 0x080: Operator pressed — save operand and op code. */
  OP_PRESSED: 0x080,
  /** 0x0B0: Equals — dispatch to arithmetic, show result. */
  EQUALS: 0x0b0,
  /** 0x0F0: 13-digit BCD addition. */
  ADD_BCD: 0x0f0,
  /** 0x130: BCD subtraction. */
  SUB_BCD: 0x130,
  /** 0x160: Multiplication via repeated addition. */
  MUL_BCD: 0x160,
  /** 0x1A0: Division via repeated subtraction. */
  DIV_BCD: 0x1a0,
  /** 0x1E0: Copy result to display buffer. */
  DISPLAY: 0x1e0,
  /** 0x220: Zero all RAM. */
  CLEAR: 0x220,
} as const;

// ============================================================================
// ROM Program
// ============================================================================

/**
 * Build the Busicom calculator ROM.
 *
 * Returns a Uint8Array containing the complete ROM program. Uses the
 * label-resolving assembler above so all jump targets are computed
 * correctly — no manual address math.
 */
export function buildBusicomROM(): Uint8Array {
  const a = new Asm4004();

  // ========================================================================
  // MAIN (0x000): Initialize and enter scan loop
  // ========================================================================

  a.org(ROM_ADDRESSES.MAIN);
  a.JMS("clear");               // Clear all RAM
  a.JUN("key_scan");            // Enter scan loop

  // ========================================================================
  // KEY_SCAN (0x020): Read keyboard, dispatch
  // ========================================================================
  //
  // Read ROM port via RDR. If 0 (no key), loop. Otherwise:
  //   - 0x1-0x9: digit 1-9 → DIGIT_ENTRY
  //   - 0xA: digit 0 → DIGIT_ENTRY
  //   - 0xC: add → OP_PRESSED with code 1
  //   - 0xD: subtract → OP_PRESSED with code 2
  //   - 0xE: multiply → OP_PRESSED with code 3
  //   - 0xF: equals → EQUALS
  //
  // Register usage: R0 = key code, R1 = scratch

  a.org(ROM_ADDRESSES.KEY_SCAN);
  a.label("key_scan");
  a.RDR();                      // acc = ROM port (key code)
  a.XCH(0);                     // R0 = key code (save it)
  a.LD(0);                      // acc = key code
  a.JCN(0xC, "key_pressed");    // if acc != 0, key was pressed
  a.JUN("key_scan");            // no key → loop

  a.label("key_pressed");
  // Check for digit keys (1-9 and 0xA)
  // If key <= 0xA, it's a digit
  a.LD(0);                      // acc = key code
  a.LDM(0xB);
  a.XCH(1);                     // R1 = 0xB
  a.LD(0);                      // acc = key
  a.CLC();                      // Clear carry — SUB uses carry as borrow input
  a.SUB(1);                     // acc = key - 0xB (carry set if key >= 0xB)
  a.JCN(0xA, "is_digit");      // jump if carry clear (key < 0xB → digit)

  // Not a digit. Check specific operator keys.
  a.LD(0);                      // acc = key
  a.LDM(0xC);
  a.XCH(1);                     // R1 = 0xC
  a.LD(0);
  a.CLC();                      // Clear carry before comparison
  a.SUB(1);                     // acc = key - 0xC
  a.JCN(0xC, "not_add");       // jump if acc != 0
  // It's add (0xC)
  a.LDM(1);                    // op code 1 = add
  a.JUN("op_pressed");

  a.label("not_add");
  a.LD(0);                      // acc = key
  a.LDM(0xD);
  a.XCH(1);
  a.LD(0);
  a.CLC();                      // Clear carry before comparison
  a.SUB(1);
  a.JCN(0xC, "not_sub");
  // It's subtract (0xD)
  a.LDM(2);
  a.JUN("op_pressed");

  a.label("not_sub");
  // Check for multiply (0xE)
  a.LD(0);
  a.LDM(0xE);
  a.XCH(1);
  a.LD(0);
  a.CLC();                      // Clear carry before comparison
  a.SUB(1);
  a.JCN(0xC, "not_mul");
  a.LDM(3);
  a.JUN("op_pressed");

  a.label("not_mul");
  // Must be equals (0xF) or unknown — treat as equals
  a.JUN("equals");

  // ========================================================================
  // is_digit: jump to digit entry
  // ========================================================================
  a.label("is_digit");
  a.JUN("digit_entry");

  // ========================================================================
  // DIGIT_ENTRY (0x040): Write digit to input buffer
  // ========================================================================
  //
  // The digit value is in R0. For key codes 1-9, digit = code.
  // For key code 0xA, digit = 0.
  //
  // We write the digit to Register 1, Character 0 (LSB).
  // For multi-digit support, we'd shift existing digits left first,
  // but for v1 we handle single digits per entry.

  a.org(ROM_ADDRESSES.DIGIT_ENTRY);
  a.label("digit_entry");

  // Convert key code 0xA to digit 0
  a.LD(0);                      // acc = key code
  a.LDM(0xA);
  a.XCH(1);                     // R1 = 0xA
  a.LD(0);
  a.CLC();                      // Clear carry before comparison
  a.SUB(1);                     // acc = key - 0xA
  a.JCN(0xC, "digit_not_zero"); // jump if key != 0xA (acc != 0)
  // Key is 0xA → digit is 0
  a.LDM(0);
  a.XCH(0);                     // R0 = 0
  a.label("digit_not_zero");

  // Write digit to RAM Register 1, Character 0
  a.FIM(1, 0x10);               // P1 = (R2=0x1, R3=0x0) → register 1, char 0
  a.SRC(1);                     // Set RAM address
  a.LD(0);                      // acc = digit
  a.WRM();                      // RAM[1][0] = digit

  // Also update the display immediately
  a.JMS("display");
  a.JUN("key_scan");

  // ========================================================================
  // OP_PRESSED (0x060): Save operand and operation code
  // ========================================================================
  //
  // When an operator is pressed:
  //   1. Copy Register 1 (input) → Register 2 (saved operand)
  //   2. Store operation code in Register 3, Char 2
  //   3. Clear Register 1 for second number entry
  //
  // Operation code arrives in accumulator (1=add, 2=sub, 3=mul).

  a.org(ROM_ADDRESSES.OP_PRESSED);
  a.label("op_pressed");
  a.XCH(6);                     // R6 = operation code

  // Copy Register 1 → Register 2 (just char 0 for v1 simplicity)
  a.FIM(1, 0x10);               // P1 = register 1, char 0
  a.SRC(1);
  a.RDM();                      // acc = Register1[0]
  a.XCH(7);                     // R7 = digit

  a.FIM(1, 0x20);               // P1 = register 2, char 0
  a.SRC(1);
  a.LD(7);                      // acc = digit
  a.WRM();                      // Register2[0] = digit

  // Store operation code
  a.FIM(1, 0x32);               // P1 = register 3, char 2
  a.SRC(1);
  a.LD(6);
  a.WRM();                      // Register3[2] = op code

  // Clear Register 1
  a.FIM(1, 0x10);
  a.SRC(1);
  a.CLB();
  a.WRM();                      // Register1[0] = 0

  a.JUN("key_scan");

  // ========================================================================
  // EQUALS (0x080): Execute pending operation
  // ========================================================================
  //
  // Read operation code, dispatch to arithmetic routine, display result.

  a.org(ROM_ADDRESSES.EQUALS);
  a.label("equals");

  // Read operation code from RAM[3][2]
  a.FIM(1, 0x32);
  a.SRC(1);
  a.RDM();                      // acc = op code
  a.XCH(6);                     // R6 = op code

  // Dispatch: op=1 → ADD
  a.LDM(1);
  a.XCH(1);
  a.LD(6);
  a.CLC();                      // Clear carry before comparison
  a.SUB(1);                     // acc = op - 1
  a.JCN(0xC, "try_sub");       // jump if acc != 0
  a.JMS("add_bcd");
  a.JUN("show_result");

  a.label("try_sub");
  a.LDM(2);
  a.XCH(1);
  a.LD(6);
  a.CLC();                      // Clear carry before comparison
  a.SUB(1);
  a.JCN(0xC, "try_mul");
  a.JMS("sub_bcd");
  a.JUN("show_result");

  a.label("try_mul");
  a.LDM(3);
  a.XCH(1);
  a.LD(6);
  a.CLC();                      // Clear carry before comparison
  a.SUB(1);
  a.JCN(0xC, "show_result");   // unknown op → just display
  a.JMS("mul_bcd");

  a.label("show_result");
  a.JMS("display");
  a.JUN("key_scan");

  // ========================================================================
  // ADD_BCD (0x0A0): Single-digit BCD addition
  // ========================================================================
  //
  // Add Register 2[0] to Register 1[0] using ADD + DAA.
  // Result goes back to Register 1[0] (and overflow to Register 1[1]).

  a.org(ROM_ADDRESSES.ADD_BCD);
  a.label("add_bcd");

  // Read digit from Register 2[0]
  a.FIM(1, 0x20);
  a.SRC(1);
  a.RDM();                      // acc = Register2[0]
  a.XCH(7);                     // R7 = operand B

  // Read digit from Register 1[0]
  a.FIM(1, 0x10);
  a.SRC(1);
  a.RDM();                      // acc = Register1[0]

  // BCD add
  a.CLB();                      // Clear carry first
  // Re-read Register 1[0] since CLB cleared acc
  a.FIM(1, 0x10);
  a.SRC(1);
  a.RDM();
  a.ADD(7);                     // acc = A + B (binary)
  a.DAA();                      // BCD correction

  // Save result back to Register 1[0]
  a.XCH(8);                     // R8 = BCD result (low digit)
  // Convert carry flag to 0/1 in acc using LDM(0) + RAL
  // RAL rotates carry into bit 0: if carry=1 → acc=1, if carry=0 → acc=0
  a.LDM(0);                     // acc = 0 (carry unchanged)
  a.RAL();                      // acc = [carry, 0, 0, 0] = carry value
  a.XCH(9);                     // R9 = carry digit (0 or 1)

  // Write low digit
  a.FIM(1, 0x10);
  a.SRC(1);
  a.LD(8);
  a.WRM();                      // Register1[0] = low digit

  // Write carry to Register 1[1]
  a.FIM(1, 0x11);               // register 1, char 1
  a.SRC(1);
  a.LD(9);
  a.WRM();                      // Register1[1] = carry digit

  a.BBL(0);

  // ========================================================================
  // SUB_BCD (0x0C0): Single-digit subtraction
  // ========================================================================
  //
  // Subtract Register 2[0] from Register 1[0].
  // Uses SUB instruction (complement-add).

  a.org(ROM_ADDRESSES.SUB_BCD);
  a.label("sub_bcd");

  // Read operand from Register 2[0]
  a.FIM(1, 0x20);
  a.SRC(1);
  a.RDM();
  a.XCH(7);                     // R7 = subtrahend

  // Read from Register 1[0]
  a.FIM(1, 0x10);
  a.SRC(1);
  a.RDM();                      // acc = minuend

  // Clear carry (no borrow initially)
  a.CLC();                      // carry = 0 → SUB does true subtraction

  // Re-read and subtract
  a.FIM(1, 0x10);
  a.SRC(1);
  a.RDM();
  a.SUB(7);                     // acc = A - B

  // Write result
  a.WRM();                      // Register1[0] = result

  a.BBL(0);

  // ========================================================================
  // MUL_BCD (0x0E0): Single-digit multiplication via repeated addition
  // ========================================================================

  a.org(ROM_ADDRESSES.MUL_BCD);
  a.label("mul_bcd");

  // Read multiplier from Register 2[0]
  a.FIM(1, 0x20);
  a.SRC(1);
  a.RDM();
  a.XCH(8);                     // R8 = multiplier

  // Read multiplicand from Register 1[0]
  a.FIM(1, 0x10);
  a.SRC(1);
  a.RDM();
  a.XCH(9);                     // R9 = multiplicand

  // Clear result
  a.FIM(1, 0x10);
  a.SRC(1);
  a.CLB();
  a.WRM();                      // Register1[0] = 0

  // If multiplier is 0, done
  a.LD(8);
  a.JCN(0xC, "mul_loop");      // jump if multiplier != 0
  a.BBL(0);

  a.label("mul_loop");
  // Add multiplicand to result
  a.FIM(1, 0x10);
  a.SRC(1);
  a.RDM();                      // acc = current result
  a.ADD(9);                     // acc += multiplicand
  a.DAA();                      // BCD adjust
  a.WRM();                      // Store back

  // Handle carry to char 1
  a.XCH(10);                    // save low digit
  a.TCS();                      // acc = carry
  a.XCH(11);                    // R11 = carry
  a.FIM(1, 0x10);
  a.SRC(1);
  a.LD(10);
  a.WRM();                      // Rewrite low digit properly

  a.FIM(1, 0x11);               // char 1
  a.SRC(1);
  a.RDM();
  a.ADD(11);                    // Add carry
  a.WRM();

  // Decrement multiplier
  a.LD(8);
  a.DAC();                      // R8 - 1
  a.XCH(8);
  a.LD(8);
  a.JCN(0xC, "mul_loop");      // Loop if R8 != 0

  a.BBL(0);

  // ========================================================================
  // DIV_BCD (0x100): Single-digit division via repeated subtraction
  // ========================================================================

  a.org(ROM_ADDRESSES.DIV_BCD);
  a.label("div_bcd");

  // Read divisor from Register 2[0]
  a.FIM(1, 0x20);
  a.SRC(1);
  a.RDM();
  a.XCH(8);                     // R8 = divisor

  // Read dividend from Register 1[0]
  a.FIM(1, 0x10);
  a.SRC(1);
  a.RDM();
  a.XCH(9);                     // R9 = dividend

  a.CLB();
  a.XCH(10);                    // R10 = 0 (quotient)

  a.label("div_loop");
  a.LD(9);                      // acc = dividend
  a.CLC();                      // Clear carry for true subtraction
  a.SUB(8);                     // acc = dividend - divisor
  a.JCN(0xA, "div_done");      // jump if carry clear (borrow → negative)

  // No borrow → save new dividend, increment quotient
  a.XCH(9);                     // R9 = new dividend
  a.INC(10);                    // quotient++
  a.JUN("div_loop");

  a.label("div_done");
  // Write quotient to Register 1[0]
  a.FIM(1, 0x10);
  a.SRC(1);
  a.LD(10);
  a.WRM();

  a.BBL(0);

  // ========================================================================
  // DISPLAY (0x120): Copy result to display buffer + WMP output
  // ========================================================================
  //
  // Copy Register 1 → Register 0 (display buffer) for chars 0-3.
  // Output each digit via WMP.

  a.org(ROM_ADDRESSES.DISPLAY);
  a.label("display");

  // Copy chars 0-3 from Register 1 to Register 0
  a.CLB();
  a.XCH(4);                     // R4 = 0 (char index)

  a.label("display_loop");
  // Read from Register 1, char R4
  a.LDM(1);
  a.XCH(2);                     // R2 = 1 (register 1)
  a.LD(4);
  a.XCH(3);                     // R3 = char index
  a.SRC(1);
  a.RDM();                      // acc = Register1[charN]
  a.XCH(5);                     // R5 = digit

  // Write to Register 0, char R4
  a.LDM(0);
  a.XCH(2);                     // R2 = 0 (register 0)
  a.LD(4);
  a.XCH(3);                     // R3 = char index
  a.SRC(1);
  a.LD(5);
  a.WRM();                      // Register0[charN] = digit

  // Output via WMP
  a.LD(5);
  a.WMP();

  a.INC(4);                     // Next char
  a.LD(4);
  a.LDM(4);                    // Compare with 4
  a.XCH(1);
  a.LD(4);
  a.CLC();                      // Clear carry before comparison
  a.SUB(1);
  a.JCN(0xC, "display_loop");  // Loop if R4 != 4 (acc!=0 means R4!=4)

  a.BBL(0);

  // ========================================================================
  // CLEAR (0x140): Zero all RAM
  // ========================================================================

  a.org(ROM_ADDRESSES.CLEAR);
  a.label("clear");

  a.CLB();
  a.XCH(4);                     // R4 = 0 (char index)

  a.label("clear_loop");
  // Clear Register 0, char R4
  a.LDM(0);
  a.XCH(2);
  a.LD(4);
  a.XCH(3);
  a.SRC(1);
  a.CLB();
  a.WRM();

  // Clear Register 1, char R4
  a.LDM(1);
  a.XCH(2);
  a.LD(4);
  a.XCH(3);
  a.SRC(1);
  a.CLB();
  a.WRM();

  // Clear Register 2, char R4
  a.LDM(2);
  a.XCH(2);
  a.LD(4);
  a.XCH(3);
  a.SRC(1);
  a.CLB();
  a.WRM();

  // Clear Register 3, char R4
  a.LDM(3);
  a.XCH(2);
  a.LD(4);
  a.XCH(3);
  a.SRC(1);
  a.CLB();
  a.WRM();

  a.INC(4);
  a.LD(4);
  a.LDM(0);                    // Check if R4 wrapped to 0 (did 16 iterations)
  a.XCH(1);
  a.LD(4);
  a.CLC();                      // Clear carry before comparison
  a.SUB(1);
  a.JCN(0xC, "clear_loop");    // Loop if R4 != 0

  a.BBL(0);

  return a.build();
}

/**
 * Get the ROM as a pre-built Uint8Array.
 * Built once and cached.
 */
let cachedROM: Uint8Array | null = null;

export function getBusicomROM(): Uint8Array {
  if (!cachedROM) {
    cachedROM = buildBusicomROM();
  }
  return cachedROM;
}
