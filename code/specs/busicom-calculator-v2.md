# Busicom 141-PF Calculator — V2 Enhancements

## Overview

The Busicom 141-PF Calculator web app (PR #126) demonstrates the full
computing stack — from key press to transistor switching — using the
Intel 4004 gate-level simulator and every layer of the TypeScript
package ecosystem. V1 proved the architecture: five drill-down tabs
(Calculator, CPU, ALU, Gate, Transistor) with an execution flow
pipeline on the Calculator tab.

V2 addresses three shortcomings discovered during V1 development:

1. **ALU detail is reconstructed, not observed.** The `TracingCPU`
   wrapper replays `fullAdder()` calls after the CPU finishes an
   instruction. This works but means the visualization shows
   *recomputed* state, not the *actual* intermediate signals. If the
   `intel4004-gatelevel` package emitted ALU traces natively, every
   consumer would get them for free — no replay hackery needed.

2. **No clock/timing visualization.** The 4004 is a clocked machine —
   every instruction follows a fetch→decode→execute rhythm driven by
   the two-phase clock. But V1 doesn't show *when* things happen,
   only *what* happens. A timing diagram would complete the picture.

3. **Polish gaps.** Keyboard navigation, screen reader support,
   reduced motion, and the "C" key bug were deferred from V1.

```
┌─────────────────────────────────────────────────────┐
│                 V2 Scope                            │
│                                                     │
│  ┌─────────────────────────────────────────────┐    │
│  │ Package Enhancement: intel4004-gatelevel     │    │
│  │   • Native ALU trace emission in step()      │    │
│  │   • Decoder control signal snapshot          │    │
│  │   • Remove need for replay in TracingCPU     │    │
│  └─────────────────────────────────────────────┘    │
│                                                     │
│  ┌─────────────────────────────────────────────┐    │
│  │ New Layer: Clock / Timing Diagram            │    │
│  │   • Two-phase clock visualization            │    │
│  │   • Fetch / Decode / Execute phase mapping   │    │
│  │   • Integration with clock package           │    │
│  └─────────────────────────────────────────────┘    │
│                                                     │
│  ┌─────────────────────────────────────────────┐    │
│  │ App Polish & Accessibility                   │    │
│  │   • Keyboard nav, ARIA, reduced motion       │    │
│  │   • Fix "C" key bug                          │    │
│  │   • Mobile responsiveness audit              │    │
│  └─────────────────────────────────────────────┘    │
│                                                     │
└─────────────────────────────────────────────────────┘
```

---

## Part 1: Native ALU Trace Emission

### Problem

In V1, the `TracingCPU` wrapper in the Busicom app reconstructs ALU
detail by calling `fullAdder()` from the `arithmetic` package after
the CPU has already executed the instruction:

```
V1 flow:
  cpu.step()  →  GateTrace { accBefore, accAfter, carry }
                     ↓
  TracingCPU detects ADD/SUB from decoded instruction
                     ↓
  Replays fullAdder(a[0], b[0], cIn) ... fullAdder(a[3], b[3], c[2])
                     ↓
  Constructs ALUDetail { adders: [...], result, carryOut }
```

This has three problems:
1. **Duplication** — the same arithmetic runs twice (once in the CPU,
   once in the replay).
2. **Fragility** — if the CPU's ALU implementation changes, the replay
   must change too. They can silently diverge.
3. **Incompleteness** — other consumers of `intel4004-gatelevel` don't
   get ALU detail at all. Only our specific wrapper does.

### Solution

Add an optional `aluTrace` field to the `GateTrace` returned by
`step()`. The CPU's internal ALU already computes per-adder state —
we just need to capture and surface it.

### Changes to `intel4004-gatelevel`

#### New types (`types.ts` or inline in `cpu.ts`)

```typescript
/**
 * State of a single full adder during an ALU operation.
 *
 *       a ──┐
 *            ├─ XOR ─── XOR ─── sum
 *       b ──┘    │       │
 *                AND     AND
 *                 │       │
 *       cIn ─────┘       └── OR ── cOut
 */
interface FullAdderSnapshot {
  a: Bit;
  b: Bit;
  cIn: Bit;
  sum: Bit;
  cOut: Bit;
}

/**
 * Complete trace of a 4-bit ALU operation.
 *
 * Captures every intermediate value so visualizations can show
 * exactly how the result was computed — from input bits through
 * the carry chain to the final output.
 */
interface ALUTrace {
  /** Which ALU operation was performed. */
  operation: "add" | "sub" | "inc" | "dec" | "complement" | "and" | "or";

  /** 4-bit input A (LSB first). */
  inputA: Bit[];

  /** 4-bit input B (LSB first). For SUB, this is the complemented value. */
  inputB: Bit[];

  /** Carry/borrow input to the adder chain. */
  carryIn: Bit;

  /**
   * Per-bit full adder snapshots, from bit 0 (LSB) to bit 3 (MSB).
   *
   * adders[0].cOut feeds into adders[1].cIn — this is the ripple
   * carry chain that gives the "ripple carry adder" its name.
   */
  adders: FullAdderSnapshot[];

  /** 4-bit result (LSB first). */
  result: Bit[];

  /** Final carry out from the MSB adder. */
  carryOut: Bit;
}
```

#### Extended `GateTrace`

```typescript
interface GateTrace {
  // ... existing fields unchanged ...
  address: number;
  raw: number;
  raw2: number | null;
  mnemonic: string;
  accumulatorBefore: number;
  accumulatorAfter: number;
  carryBefore: boolean;
  carryAfter: boolean;

  // ── V2 additions ──

  /**
   * Full instruction decode — the control signals that tell the CPU
   * what to do with this instruction.
   *
   * Always present. Previously only available via a separate
   * decode() call; now captured during execution.
   */
  decoded: DecodedInstruction;

  /**
   * ALU operation detail, if the instruction used the ALU.
   *
   * Present for: ADD, SUB, INC, DAA, and accumulator instructions
   * that route through the adder (CMA, etc.).
   *
   * Absent (undefined) for: JUN, JMS, FIM, SRC, I/O, etc.
   */
  aluTrace?: ALUTrace;

  /**
   * Memory access performed by this instruction, if any.
   *
   * Present for: LDM, LD, XCH, RDM, WRM, WMP, RDR, etc.
   * Absent for: NOP, JUN, arithmetic-only instructions.
   */
  memoryAccess?: {
    type: "reg_read" | "reg_write" | "ram_read" | "ram_write" | "rom_read" | "port_read" | "port_write";
    address: number;
    value: number;
  };
}
```

#### Implementation in `GateALU`

The `GateALU` class (`alu.ts`) currently calls `this._alu.add(a, b, carryIn)`
which returns `[result, carry]`. To capture per-adder state, we have
two options:

**Option A: Instrument the arithmetic package's `rippleCarryAdder`.**
Add an optional `trace` output parameter or return extended results.
This is cleaner but touches two packages.

**Option B: Replay in `GateALU` itself.** After calling `this._alu.add()`,
immediately replay the 4 `fullAdder()` calls to capture intermediates.
This is what `TracingCPU` does today but moved into the package.

**Recommended: Option A.** The `arithmetic` package's `rippleCarryAdder`
already calls `fullAdder` four times in sequence. Adding a return
value for per-adder state is a small, backwards-compatible change:

```typescript
// In @coding-adventures/arithmetic

interface RippleCarryResult {
  sum: Bit[];
  carryOut: Bit;
  /** Per-bit adder snapshots for visualization. */
  adders: Array<{ a: Bit; b: Bit; cIn: Bit; sum: Bit; cOut: Bit }>;
}

// Existing signature (unchanged, backwards compatible):
function rippleCarryAdder(a: Bit[], b: Bit[], cIn: Bit): [Bit[], Bit];

// New overload or companion function:
function rippleCarryAdderTraced(a: Bit[], b: Bit[], cIn: Bit): RippleCarryResult;
```

Then `GateALU.add()` calls `rippleCarryAdderTraced()` instead and
stores the result in a `_lastAluTrace` field that `cpu.step()` reads
when building the `GateTrace`.

#### Implementation in `Intel4004GateLevel.step()`

```typescript
step(): GateTrace {
  const address = this.pc;
  const accBefore = this.accumulator;
  const carryBefore = this.carry;

  // Fetch + decode
  const raw = this._rom[this._pc.value];
  const decoded = decode(raw, ...);

  // Execute (may trigger ALU, memory access, etc.)
  this._execute(decoded);

  // Build trace
  return {
    address,
    raw,
    raw2: decoded.isTwoByte ? this._rom[address + 1] : null,
    mnemonic: this._mnemonic(decoded),
    accumulatorBefore: accBefore,
    accumulatorAfter: this.accumulator,
    carryBefore,
    carryAfter: this.carry,

    // V2: always include decode
    decoded,

    // V2: ALU trace if the instruction used the ALU
    aluTrace: this._alu.lastTrace,    // undefined if ALU wasn't used

    // V2: memory access if applicable
    memoryAccess: this._lastMemAccess, // set during _execute()
  };
}
```

The key insight: `_alu.lastTrace` is set during `_execute()` and
cleared at the start of each `step()`. This is a "last result" pattern
— the ALU remembers what it just did, and the CPU reads it once.

### Changes to `TracingCPU` (Busicom app)

With native traces, the `TracingCPU` wrapper becomes dramatically
simpler. The entire `replayAdderChain()` function and its associated
logic can be deleted:

```typescript
// V1 (complex):
step(): DetailedTrace {
  const trace = this._cpu.step();
  const decoded = decode(trace.raw, trace.raw2 ?? undefined);
  const aluDetail = this._replayAdderChain(decoded, ...);  // 50+ lines
  return { ...trace, decoded, aluDetail, snapshot: this._snapshot() };
}

// V2 (simple):
step(): DetailedTrace {
  const trace = this._cpu.step();
  // decoded and aluTrace already on trace — just add snapshot
  return { ...trace, snapshot: this._snapshot() };
}
```

The `DetailedTrace` type in the app can be simplified to just extend
`GateTrace` with `snapshot` — the rest comes from the package.

### Backwards Compatibility

- `GateTrace` gains new optional fields (`decoded`, `aluTrace`,
  `memoryAccess`). Existing consumers that destructure only the old
  fields continue to work unchanged.
- The `rippleCarryAdder` function signature doesn't change. We add
  `rippleCarryAdderTraced` as a new export.
- Existing tests don't need modification — they test the same values.
  New tests verify the trace fields.

### Test Plan

| Test | What it verifies |
|------|-----------------|
| `step()` returns `decoded` for every instruction | Decode is always present |
| `step()` returns `aluTrace` for ADD, SUB, INC, DAA | ALU trace captured |
| `step()` returns `undefined` aluTrace for JUN, FIM, NOP | Non-ALU instructions |
| `aluTrace.adders[0].cOut === aluTrace.adders[1].cIn` | Carry chain integrity |
| `rippleCarryAdderTraced` matches `rippleCarryAdder` results | Traced version agrees |
| SUB trace shows complemented B | Complement-add visible |
| Memory access captured for LD, XCH, WRM, RDM | memoryAccess field |

---

## Part 2: Clock & Timing Diagram (Layer 2.5)

### Concept

The Intel 4004 uses a **two-phase non-overlapping clock**:

```
        ┌───┐   ┌───┐   ┌───┐   ┌───┐
  Φ1  ──┘   └───┘   └───┘   └───┘   └──

           ┌───┐   ┌───┐   ┌───┐   ┌───
  Φ2  ─────┘   └───┘   └───┘   └───┘

        ├─── 1 machine cycle ───┤
        A1 A2 A3 M1 M2 X1 X2 X3
```

Each machine cycle has **8 clock phases** organized into 3 sub-cycles:

| Sub-cycle | Phases | What happens |
|-----------|--------|-------------|
| **A** (Address) | A1, A2, A3 | PC sent to ROM address bus |
| **M** (Memory) | M1, M2 | ROM data returned on bus |
| **X** (Execute) | X1, X2, X3 | Instruction decoded and executed |

This is the heartbeat of the CPU. Every register write, every ALU
operation, every RAM access happens at a specific phase within this
cycle. Understanding timing is the bridge between "what does an
instruction do?" and "how does the hardware make it happen?"

### New Tab: "Timing"

Add a sixth tab between CPU and ALU:

```
  Calculator │ CPU │ Timing │ ALU │ Gates │ Transistors
```

The Timing tab shows:

#### 1. Clock Waveform Diagram

An SVG timing diagram showing Φ1 and Φ2 as square waves. The
current phase is highlighted. As the user steps through instructions,
the waveform scrolls and the highlight moves.

```
  Φ1  ─┐ ┌─┐ ┌─┐ ┌─┐ ┌─┐ ┌─
       └─┘ └─┘ └─┘ └─┘ └─┘

  Φ2  ──┐ ┌─┐ ┌─┐ ┌─┐ ┌─┐ ┌
        └─┘ └─┘ └─┘ └─┘ └─┘

  Bus  ──[  ADDR  ][DATA][ EXEC ]──
            A1-A3   M1-M2  X1-X3
```

#### 2. Phase Activity Table

Shows what happens during each phase of the current instruction:

| Phase | Activity | Signal |
|-------|----------|--------|
| A1 | PC bits 0-3 sent to bus | `0x5` |
| A2 | PC bits 4-7 sent to bus | `0x0` |
| A3 | PC bits 8-11 sent to bus | `0x0` |
| M1 | ROM data bits 0-3 received | `0xA` |
| M2 | ROM data bits 4-7 received | `0x3` |
| X1 | Instruction decoded | `ADD R3` |
| X2 | ALU computes result | `acc=5+3=8` |
| X3 | Result written to accumulator | `acc←8` |

#### 3. Multi-Instruction Timeline

A horizontal timeline showing the last N instructions as rectangles,
each subdivided into A/M/X sub-cycles. Color-coded by instruction
type (ALU=green, jump=blue, I/O=orange, memory=purple).

```
  ──┤ FIM P0,0x40 ├┤ SRC P0 ├┤ RDM ├┤ ADD R3 ├┤ XCH R5 ├──
     A  M  X        A  M  X   A M X   A M X     A  M  X
```

### Integration with Clock Package

The `@coding-adventures/clock` package already provides `MultiPhaseClock`
with N non-overlapping phases. We use it to model the 4004's 8-phase
cycle:

```typescript
import { Clock, MultiPhaseClock } from "@coding-adventures/clock";

// 4004 runs at 740 kHz → 8 phases per instruction
const masterClock = new Clock(740_000);
const phases = new MultiPhaseClock(masterClock, 8);

// Map phase indices to 4004 sub-cycle names
const PHASE_NAMES = ["A1", "A2", "A3", "M1", "M2", "X1", "X2", "X3"];
```

The clock doesn't drive the CPU (we step manually), but it provides
the conceptual model. The timing diagram component reads the clock
state to determine which phase label to highlight.

### Phase-to-Activity Mapping

Each instruction type has a characteristic phase pattern. We define
these as data, not code:

```typescript
interface PhaseActivity {
  phase: string;        // "A1" | "A2" | ... | "X3"
  activity: string;     // Human-readable description
  signal?: string;      // Optional: actual value on the bus
  component?: string;   // Which hardware block is active
}

function getPhaseActivities(trace: GateTrace): PhaseActivity[] {
  // All instructions share A1-A3 (address) and M1-M2 (data fetch)
  const common = [
    { phase: "A1", activity: "PC[3:0] → bus", signal: hex(trace.address & 0xF), component: "PC" },
    { phase: "A2", activity: "PC[7:4] → bus", signal: hex((trace.address >> 4) & 0xF), component: "PC" },
    { phase: "A3", activity: "PC[11:8] → bus", signal: hex((trace.address >> 8) & 0xF), component: "PC" },
    { phase: "M1", activity: "ROM data[3:0]", signal: hex(trace.raw & 0xF), component: "ROM" },
    { phase: "M2", activity: "ROM data[7:4]", signal: hex((trace.raw >> 4) & 0xF), component: "ROM" },
  ];

  // X1-X3 depend on instruction type
  const execute = getExecutePhases(trace);
  return [...common, ...execute];
}
```

### Component: `TimingView`

```
src/components/timing-view/
  TimingView.tsx          — Main container with all 3 sub-views
  ClockWaveform.tsx       — SVG Φ1/Φ2 square wave diagram
  PhaseTable.tsx          — Table of phase activities
  InstructionTimeline.tsx — Multi-instruction horizontal timeline
```

### Execution Flow Integration

The existing execution flow pipeline on the Calculator tab gains a
new "Clock Phase" stage between Fetch and Decode:

```
  Key Press → I/O → Fetch → Clock Phase → Decode → Execute → ...
```

This stage shows: "Phase X2: ALU computing" or "Phase M1: Reading ROM
data" — connecting the what (instruction flow) to the when (clock
timing).

---

## Part 3: App Polish & Accessibility

### 3a. Fix "C" Key Bug

**Problem:** The "C" (Clear) key currently maps to `0xF` which the ROM
interprets as the equals operation, not clear.

**Fix:** In the ROM's key scan table, assign "C" a dedicated key code
(e.g., `0xE`) and add a handler in the `KEY_SCAN` routine:

```
; In KEY_SCAN after checking for digits and operators:
  LD R6
  CLC
  SUB R_CLEAR_CODE    ; Compare with clear key code
  JCN nz, NOT_CLEAR
  JMS CLEAR           ; Jump to clear routine
  JUN MAIN            ; Return to idle
NOT_CLEAR:
```

Also add "CE" (Clear Entry) support — clears only the current input
buffer without erasing the stored operand.

### 3b. Keyboard Navigation (WCAG 2.1 AA)

| Element | Keyboard behavior |
|---------|------------------|
| Calculator keys | Arrow keys navigate the grid, Enter/Space presses |
| Layer tabs | Left/Right arrows move between tabs, Enter activates |
| Step controls | Tab to reach, Enter/Space to activate |
| Register table | Tab to table, arrow keys within cells |
| Trace log | Tab to log, Up/Down to scroll entries |

**Focus management:**
- Switching tabs moves focus to the new panel's first interactive element
- After pressing a calculator key, focus returns to the key grid
- Visible focus rings on all interactive elements (no `outline: none`)

**Implementation:**
- `onKeyDown` handlers on the keypad grid for arrow key navigation
- `role="tablist"` / `role="tab"` / `role="tabpanel"` (already in V1)
- `aria-live="polite"` on the display for screen reader announcements
- `aria-live="polite"` on CPU state summary for register changes

### 3c. Screen Reader Support

| Component | ARIA treatment |
|-----------|---------------|
| Display | `aria-label="Calculator display"`, `aria-live="polite"` announces value changes |
| Each key | `aria-label="digit 5"` or `aria-label="add operator"` |
| Layer tabs | Already have `role="tab"`, add `aria-label` with layer description |
| Register table | `role="grid"` with `aria-label` on each cell |
| ALU diagram | `aria-label` describing the operation in words |
| SVG gates | Each SVG gets `role="img"` and descriptive `aria-label` |
| Transistor diagrams | `aria-label` describing conducting/cutoff state |
| Timing waveform | `aria-label` describing current phase and activity |

### 3d. Reduced Motion

```css
@media (prefers-reduced-motion: reduce) {
  /* Disable all animations */
  .flow-dot--current { animation: none; }
  .segment--glow { animation: none; }
  .carry-arrow { transition: none; }

  /* Replace transitions with instant changes */
  * { transition-duration: 0.01ms !important; }
}
```

### 3e. Mobile Responsiveness

The V1 layout works on desktop but needs adjustment for mobile:

- **Calculator tab**: Keys should fill the viewport width on phones.
  The execution flow stacks below the calculator with collapsible
  stages (tap to expand).
- **CPU tab**: Register table becomes a scrollable 4-column grid
  instead of 16 columns. Trace log uses horizontal scroll.
- **ALU tab**: Full adder cards stack vertically instead of
  horizontally. Carry arrows become vertical.
- **Gate/Transistor tabs**: SVG diagrams scale to viewport width
  with `viewBox` (already set) and `width="100%"`.
- **Timing tab**: Waveform scrolls horizontally. Phase table stacks
  on small screens.

Breakpoints:
- `≤ 480px`: Single-column layout, collapsed execution flow
- `481–768px`: Two-column where possible, expanded flow
- `≥ 769px`: Full desktop layout (current V1)

---

## Implementation Plan

### Phase 1: Package Enhancements (arithmetic + intel4004-gatelevel)

1. Add `rippleCarryAdderTraced()` to `@coding-adventures/arithmetic`
   - New function alongside existing `rippleCarryAdder`
   - Returns `RippleCarryResult` with per-adder snapshots
   - Tests: verify traced results match non-traced results
   - Update CHANGELOG, README

2. Update `GateALU` in `intel4004-gatelevel` to use traced adder
   - Store `_lastTrace: ALUTrace | undefined`
   - Clear at start of each operation, populate during add/sub/inc
   - Expose via `get lastTrace()` getter

3. Extend `GateTrace` with `decoded`, `aluTrace`, `memoryAccess`
   - Capture `decoded` from existing `decode()` call in `step()`
   - Read `aluTrace` from `GateALU.lastTrace`
   - Track memory access in `_execute()` methods
   - Update all existing tests to verify new fields
   - Update CHANGELOG, README

4. Simplify `TracingCPU` in Busicom app
   - Remove `replayAdderChain()` and all reconstruction logic
   - `DetailedTrace` becomes `GateTrace & { snapshot: CpuSnapshot }`
   - Verify all existing app tests still pass

### Phase 2: Timing Visualization

5. Create timing data model
   - `PhaseActivity` type and `getPhaseActivities()` function
   - Map instruction types to X1/X2/X3 activities
   - Tests for phase activity generation

6. Build timing components
   - `ClockWaveform.tsx` — SVG two-phase clock diagram
   - `PhaseTable.tsx` — current instruction phase breakdown
   - `InstructionTimeline.tsx` — multi-instruction horizontal view
   - `TimingView.tsx` — container component

7. Integrate timing tab
   - Add "Timing" to layer tabs in `App.tsx`
   - Add execution flow "Clock Phase" stage
   - Add CSS for timing components
   - Add i18n strings to `en.json`

### Phase 3: Bug Fix + Accessibility + Polish

8. Fix "C" key
   - Update ROM key code mapping
   - Add CLEAR handler in KEY_SCAN
   - Add CE (Clear Entry) support
   - Test: press C clears display, CE clears only current input

9. Keyboard navigation
   - Arrow key handlers on keypad grid
   - Focus management on tab switches
   - Visible focus indicators in CSS
   - Test: navigate entire app with keyboard only

10. Screen reader support
    - `aria-live` regions on display and CPU state
    - `aria-label` on all interactive elements
    - `role="img"` + descriptions on SVG diagrams
    - Test: VoiceOver/NVDA walkthrough

11. Reduced motion + mobile
    - `prefers-reduced-motion` media query
    - Responsive breakpoints at 480px and 768px
    - Test: resize browser, toggle reduced motion

---

## Files Changed

### Package: `@coding-adventures/arithmetic`
| File | Change |
|------|--------|
| `src/index.ts` | Add `rippleCarryAdderTraced` export |
| `src/adder.ts` (or equivalent) | Implement traced version |
| `tests/adder.test.ts` | Tests for traced adder |
| `CHANGELOG.md` | Document new function |
| `README.md` | Usage example |

### Package: `@coding-adventures/intel4004-gatelevel`
| File | Change |
|------|--------|
| `src/alu.ts` | Use traced adder, expose `lastTrace` |
| `src/cpu.ts` | Include `decoded`, `aluTrace`, `memoryAccess` in `GateTrace` |
| `src/index.ts` | Export new types (`ALUTrace`, `FullAdderSnapshot`) |
| `tests/cpu.test.ts` | Verify new GateTrace fields |
| `tests/alu.test.ts` | Verify ALU trace capture |
| `CHANGELOG.md` | Document trace enhancements |
| `README.md` | Updated usage examples |

### App: `busicom-calculator`
| File | Change |
|------|--------|
| `src/cpu/tracing-cpu.ts` | Remove replay logic, simplify to snapshot-only wrapper |
| `src/cpu/types.ts` | Simplify `DetailedTrace` — most fields now come from `GateTrace` |
| `src/rom/busicom-rom.ts` | Fix "C" key code, add CE handler |
| `src/App.tsx` | Add Timing tab |
| `src/components/timing-view/TimingView.tsx` | New: container |
| `src/components/timing-view/ClockWaveform.tsx` | New: SVG clock diagram |
| `src/components/timing-view/PhaseTable.tsx` | New: phase activity table |
| `src/components/timing-view/InstructionTimeline.tsx` | New: multi-instruction timeline |
| `src/components/execution-flow/ExecutionFlow.tsx` | Add Clock Phase stage |
| `src/components/calculator/Calculator.tsx` | Keyboard nav, ARIA |
| `src/components/calculator/Keypad.tsx` | Arrow key handlers |
| `src/components/calculator/Display.tsx` | `aria-live` region |
| `src/styles/calculator.css` | Focus indicators, responsive breakpoints |
| `src/styles/views.css` | Timing styles, reduced motion, mobile layout |
| `src/i18n/locales/en.json` | Timing tab strings, ARIA labels |
| `src/hooks/useCalculator.ts` | Adapt to new GateTrace shape |

---

## Verification

1. **Package tests**: `cd arithmetic && npx vitest run` — all pass,
   traced adder matches non-traced
2. **CPU tests**: `cd intel4004-gatelevel && npx vitest run` — all
   pass, new GateTrace fields present
3. **App tests**: `cd busicom-calculator && npx vitest run` — all
   pass, simplified tracing wrapper works
4. **Build**: `npx vite build` succeeds
5. **Manual audit**:
   - Click calculator buttons, verify display
   - Switch to Timing tab, step through instructions, verify waveform
   - Tab through all interactive elements with keyboard
   - Enable VoiceOver, navigate the app
   - Toggle `prefers-reduced-motion`, verify no animations
   - Resize to 375px wide (iPhone), verify layout
6. **Coverage**: >80% across all changed packages
