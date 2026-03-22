# Busicom 141-PF Calculator — Interactive Web Simulator

## Overview

An interactive web application that simulates the Busicom 141-PF printing calculator — the
first commercial product powered by the Intel 4004 microprocessor (1971). Users click
calculator buttons and can drill down through five visualization layers, from the calculator
display all the way down to CMOS transistors.

The entire computing stack runs in the browser — no server required. The app is a
Progressive Web App (PWA) that works offline after the first visit.

## Historical Context

The Busicom 141-PF was a desktop printing calculator manufactured by Nippon Calculating
Machine Corporation (Busicom) of Japan. In 1969, Busicom contracted Intel to design a
custom chip set. Ted Hoff proposed replacing their planned 12-chip design with a single
general-purpose 4-bit microprocessor — the Intel 4004.

The 4004's instruction set was designed specifically for BCD (Binary-Coded Decimal)
arithmetic, the foundation of calculator math. The `DAA` (Decimal Adjust Accumulator)
instruction exists because of this calculator. The `KBP` (Keyboard Process) instruction
was designed for scanning the calculator's keypad.

## Layer Position

```
[Transistors] → [Logic Gates] → [Arithmetic] → [Intel 4004] → [YOU ARE HERE]
    CMOS          AND/OR/NOT      Adders/ALU     Gate-level       Busicom
  transistors     flip-flops     ripple carry    CPU simulator     calculator
```

## Architecture

### Tech Stack

- **Framework**: React 19 + TypeScript
- **Bundler**: Vite 6
- **Testing**: Vitest
- **Deployment**: GitHub Pages (static) + PWA (offline)
- **Styling**: Hand-crafted CSS (retro 1970s aesthetic)

### Dependencies (all existing packages in this repo)

| Package | Layer | What it provides |
|---------|-------|-----------------|
| `@coding-adventures/intel4004-gatelevel` | CPU | 46-instruction 4004 with exposed internals |
| `@coding-adventures/logic-gates` | Gates | AND, OR, NOT, XOR, flip-flops, registers |
| `@coding-adventures/arithmetic` | Math | halfAdder, fullAdder, rippleCarryAdder, ALU |
| `@coding-adventures/clock` | Timing | Clock edges, dividers |
| `@coding-adventures/transistors` | Physics | CMOS NAND/NOR, MOSFET models |

## Five Visualization Layers

### Layer 1: Calculator

A visual replica of the Busicom 141-PF:

- **Display**: 13-digit 7-segment LED display (CSS-only, no images)
- **Keypad**: 0-9, decimal point, +, -, ×, ÷, =, C (clear), CE (clear entry)
- **Aesthetic**: Olive/brown plastic casing, orange function keys, 1970s industrial design

When the user clicks a button, the 4004 ROM program detects the key press via the
simulated I/O port and runs the appropriate routine (add, subtract, multiply, divide).
The result appears on the display.

### Layer 2: CPU State

A live dashboard showing the 4004's internal state during execution:

- **Program Counter** (12-bit): Current instruction address
- **Accumulator** (4-bit): Shown in hex, binary, and decimal
- **Carry Flag**: Set/clear indicator
- **Registers**: 16 × 4-bit registers (R0–R15) in a 4×4 grid
- **Hardware Stack**: 3 × 12-bit return addresses
- **RAM**: Bank/register/character view with current selection highlighted
- **Instruction Trace**: Scrollable log of executed instructions with before/after state

Step controls:
- **Step**: Execute one instruction
- **Run to Key Scan**: Execute until the program checks for keyboard input
- **Free Run**: Auto-step at adjustable speed (1–1000 instructions/second)
- **Reset**: Clear all state and reload ROM

### Layer 3: ALU Detail

Activated when the current instruction involves the ALU (ADD, SUB, INC, DAC, IAC, etc.):

- **Inputs**: A (accumulator) and B (register/RAM value) shown as 4-bit binary
- **Ripple Carry Chain**: 4 full adders shown left to right
  - Each full adder shows: input A bit, input B bit, carry in, sum out, carry out
  - Carry propagation highlighted with animation
- **Full Adder Expansion**: Click any full adder to see its 5 internal gates
  (2 XOR + 2 AND + 1 OR)
- **Subtraction**: Shows NOT gates complementing B before the adder chain

### Layer 4: Gate Level

Shows individual logic gate activations for the current operation:

- **SVG Gate Symbols**: IEEE standard shapes (AND, OR, NOT, XOR)
- **Wire Values**: Color-coded (dim = 0, bright = 1) AND labeled with 0/1 text
  for accessibility (not color alone)
- **Sub-views**:
  - **Decoder**: AND/NOT gate tree that identifies the instruction type
  - **ALU Gates**: The full adder chain at gate level
  - **Register MUX**: Multiplexer gates that select which register to read

### Layer 5: Transistor Level

The bottom of the stack — CMOS transistor implementations:

- **Gate Selection**: User picks a gate from Layer 4 to inspect
- **CMOS Diagram**: NMOS and PMOS transistor pairs with labeled terminals
  (gate, source, drain)
- **Pull-up/Pull-down**: Network topology for the selected gate type
- **Voltage Levels**: Vdd (5V), GND (0V), and intermediate node voltages
- **Current Flow**: Highlighted path showing which transistors are conducting
  for the current input values
- Uses actual `CMOSNand()` / `CMOSInverter()` from the transistors package

## ROM Program

### Memory Model

The Busicom calculator ROM is a simplified but authentic 4004 program that implements
4-function BCD arithmetic. It uses ~200-300 bytes of the 4096-byte ROM space.

```
RAM Layout (4 banks × 4 registers × 16 characters):
  Bank 0, Register 0: Display buffer (13 BCD digits, LSB first)
  Bank 0, Register 1: Input accumulator (13 BCD digits)
  Bank 0, Register 2: Second operand (13 BCD digits)
  Bank 0, Register 3: Status (digit count, sign, operation code, decimal position)
```

### ROM Address Map

```
0x000: MAIN          — Initialize, enter keyboard scan loop
0x020: KEY_SCAN      — Read keyboard via RDR, dispatch to handler
0x040: DIGIT_ENTRY   — Append digit to input buffer
0x060: OP_PRESSED    — Store operand, save operation code
0x080: EQUALS        — Execute pending operation, display result
0x0A0: ADD_BCD       — 13-digit BCD addition using ADD + DAA
0x0C0: SUB_BCD       — 13-digit BCD subtraction (complement-add)
0x0E0: MUL_BCD       — Multiplication via repeated addition
0x100: DIV_BCD       — Division via repeated subtraction
0x120: DISPLAY       — Copy result to display buffer, output via WMP
0x140: CLEAR         — Zero all RAM, reset state
0x160: NEGATE        — Two's complement of BCD number
```

### Key Encoding (via ROM port)

| RDR Value | Key |
|-----------|-----|
| 0x0 | No key (idle) |
| 0x1–0x9 | Digits 1–9 |
| 0xA | Digit 0 |
| 0xB | Decimal point |
| 0xC | Add (+) |
| 0xD | Subtract (-) |
| 0xE | Multiply (×) / Divide (÷) |
| 0xF | Equals (=) / Clear (C) |

The specific operation (multiply vs divide, equals vs clear) is determined by
a mode register in RAM status nibbles.

### Instruction Patterns

The ROM heavily uses these 4004 patterns:

1. **RAM access**: `FIM P0, addr` → `SRC P0` → `RDM` (read) or `WRM` (write)
2. **BCD digit add**: `ADD Rn` → `DAA` (decimal adjust)
3. **Digit loop**: `ISZ Rn, loop_start` (iterate over 13 digits)
4. **Subroutine**: `JMS addr` → ... → `BBL return_value`
5. **Input scan**: `RDR` → `JCN test_zero` (branch if key pressed)

## I/O Simulation

### Input (Keyboard → CPU)

The `TracingCPU` wrapper intercepts instruction execution:

1. User clicks a calculator key → `pendingKey` set in React state
2. CPU runs its keyboard scan loop: `RDR` reads `romPort`
3. Before `RDR` executes, the wrapper sets `romPort = pendingKey`
4. After `RDR` executes, `pendingKey` is cleared
5. CPU branches based on key value, runs the appropriate routine

### Output (CPU → Display)

1. ROM program writes display digits via `WMP` (RAM output port)
2. The wrapper captures RAM output port values after each step
3. React component reads the 13-digit display buffer from RAM
4. CSS 7-segment display renders each digit

### Execution Model

Clicking a button triggers a sequence:
1. Set `pendingKey` state
2. Start `requestAnimationFrame` loop
3. Each frame: execute N instructions (adjustable speed)
4. Stop when CPU hits `RDR` with no pending key (idle in scan loop)
5. Read display buffer from RAM, update display component

## Enhanced Tracing

The `TracingCPU` class wraps `Intel4004GateLevel` to produce `DetailedTrace` objects
that power the ALU and gate visualizations:

```typescript
interface DetailedTrace extends GateTrace {
  // Decoded instruction fields
  decoded: DecodedInstruction;

  // CPU state snapshot after execution
  snapshot: {
    accumulator: number;
    registers: number[];
    carry: boolean;
    pc: number;
    stackPointer: number;
    ramBank: number;
  };

  // ALU detail (when applicable)
  aluDetail?: {
    operation: 'add' | 'sub' | 'inc' | 'dec' | 'complement';
    inputA: Bit[];
    inputB: Bit[];
    carryIn: Bit;
    adders: Array<{
      a: Bit; b: Bit; cIn: Bit;
      sum: Bit; cOut: Bit;
    }>;
    result: Bit[];
    carryOut: Bit;
  };

  // Register/RAM access
  memoryAccess?: {
    type: 'reg_read' | 'reg_write' | 'ram_read' | 'ram_write';
    address: number;
    value: number;
  };
}
```

ALU details are reconstructed by replaying the gate chain (halfAdder → fullAdder)
independently, using the instruction's operands.

## Accessibility (WCAG 2.1 AA)

- All interactive elements keyboard-accessible with visible focus indicators
- Screen reader support: ARIA labels, live regions for display/CPU state updates
- Color contrast ≥ 4.5:1 (AA standard)
- Wire values use color AND text labels (not color alone)
- `prefers-reduced-motion` respected for all animations
- Semantic HTML: heading hierarchy, landmark regions, button elements
- Focus management on layer transitions

## Localization (i18n)

- All text (UI chrome + educational content) externalized to JSON locale files
- Default language: English (`en.json`)
- Adding a language: drop a new JSON file (e.g., `ja.json`) — no code changes
- Flat dot-notation keys: `"calculator.display.label"`, `"alu.fullAdder.title"`
- Fallback to English for missing keys
- Language picker shown when 2+ locale files exist
- CSS logical properties for future RTL support

## Deployment

- **GitHub Pages**: Static hosting, free forever
- **PWA**: Service worker caches all assets for offline use
- **Install**: Users can add to home screen on mobile/desktop
- **GitHub Actions**: Auto-deploy on push to main

## Test Strategy

1. **ROM correctness**: Unit tests for each arithmetic routine
   - 2 + 3 = 5, 9 + 1 = 10, 999 + 1 = 1000
   - 5 - 3 = 2, 3 - 5 = -2, 1000 - 1 = 999
   - 3 × 4 = 12, 99 × 99 = 9801
   - 12 ÷ 4 = 3, 10 ÷ 3 = 3 (integer)

2. **Tracing CPU**: Verify DetailedTrace contains correct ALU detail
   for ADD, SUB, INC instructions

3. **Components**: React Testing Library tests for key interactions,
   display rendering, layer switching

4. **Accessibility**: Automated checks for ARIA attributes, keyboard nav

5. **Coverage target**: >80%
