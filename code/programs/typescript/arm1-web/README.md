# arm1-web

Interactive browser-based simulator for the ARM1 processor (1985), the first chip in
the ARM family. Step through pre-assembled programs and explore the internals at six
levels of abstraction.

## Background

The ARM1 was designed by Sophie Wilson and Steve Furber at Acorn Computers in Cambridge.
First silicon powered on 26 April 1985 — and worked correctly on the first attempt, a
remarkable feat for a new architecture. The chip contained 25,000 transistors and
implemented what would become one of the most successful CPU architectures in history.

Key ARM1 features:
- **32-bit RISC** — fixed-length 32-bit instructions, load/store architecture
- **3-stage pipeline** — Fetch → Decode → Execute, all in parallel
- **Conditional execution** — every instruction has a 4-bit condition code; no branch needed
- **Barrel shifter** — Operand2 can be shifted/rotated for free within a data processing instruction
- **R15 = PC + CPSR** — the program counter and status register are unified in one register
- **26-bit address space** — 64 MiB, with the top and bottom bits of R15 reserved for flags

## Running

```bash
npm install
npm run dev
```

Open `http://localhost:5173` in your browser.

## Pre-loaded Programs

| Program | Description | Result |
|---------|-------------|--------|
| Fibonacci | Compute fib(10) iteratively | R0 = 55 |
| Sum 1..10 | Accumulate 10+9+…+1 using SUBS+BNE | R1 = 55 |
| Array Max | Find max in [5,2,8,1,9,3,7] using MOVGT | R1 = 9 |
| Barrel Shifter | Step through LSL/LSR/ASR/ROR | R0=256, R1=16, R2=16, R3=0x01000001 |

## Visualization Tabs

### Registers
Shows all 16 visible registers (R0–R15) with the previous value highlighted orange when it
changes. The R15 diagram breaks out the N/Z/C/V flags, I/F interrupt-disable bits, the
24-bit PC field, and the 2-bit processor mode.

### Decode
Renders the most recently executed instruction as a 32-bit bit field map with colour-coded
regions for condition code, opcode, Rn, Rd, and Operand2. A table below shows all 16
condition codes for reference.

### Pipeline
Shows the ARM1's three-stage pipeline: what is currently being Fetched, Decoded, and
Executed simultaneously. Explains the PC+8 effect (PC reads as fetch-address+8 during
execution) and the branch flush penalty.

### Barrel Shifter
Visualizes the barrel shift applied to Operand2 for data processing instructions. Shows
input and output as 32-bit grids with highlighted bits that moved, plus a 5-level MUX2
tree diagram showing how the barrel shifter achieves any shift in a single cycle.

### Memory
A 4 KiB hex dump with the PC word highlighted in blue, the SP word in green, and the most
recent memory read/write highlighted in orange/red. Navigation buttons jump to PC, SP, or
the last accessed address.

### Trace
Full execution history with one row per instruction, showing the cycle number, address,
type badge, mnemonic, flag deltas, and changed registers. Skipped instructions (condition
not met) are shown with a `↷` marker and reduced opacity.

## Architecture

```
src/
├── App.tsx                    # Tab shell + controls
├── hooks/
│   └── useARM1.ts             # CPU state management hook
├── simulator/
│   ├── programs.ts            # Pre-assembled ARM1 machine code
│   └── types.ts               # Extended trace and pipeline types
├── components/
│   ├── registers/             # RegisterView (R0–R15 + CPSR)
│   ├── decode/                # DecodeView (bit-field map)
│   ├── pipeline/              # PipelineView (3-stage)
│   ├── barrel-shifter/        # BarrelShifterView
│   ├── memory/                # MemoryView (hex dump)
│   └── trace/                 # TraceView (execution log)
└── styles/
    ├── app.css                # Layout, controls, tabs
    └── views.css              # All component styles
```

## Dependencies

- `@coding-adventures/arm1-simulator` — behavioral ARM1 CPU (local package)
- `react` / `react-dom` — UI framework
- `vite` — build tool and dev server
- `vitest` — test runner

## Testing

```bash
npm test
```

Tests verify that each demo program produces the expected final register state when
run on the ARM1 behavioral simulator.
