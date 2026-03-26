# ENIAC

**Decimal arithmetic models for the first general-purpose electronic computer (1945).**

## What is ENIAC?

ENIAC (Electronic Numerical Integrator and Computer) was the first general-purpose, programmable electronic computer. Built at the University of Pennsylvania by J. Presper Eckert and John Mauchly, it used **17,468 vacuum tubes** and filled an entire room.

Unlike modern computers that use binary, ENIAC used **decimal arithmetic**. Each digit (0-9) was stored in a **decade ring counter** — a ring of 10 vacuum tubes where exactly one is "on" at a time.

## The Decade Ring Counter

```
    [0]---[1]---[2]---[3]---[4]---[5]---[6]---[7]---[8]---[9]
     |                                                       |
     +-------------------------------------------------------+
```

To count: a pulse advances the "on" tube one position. When it wraps from 9→0, it generates a carry pulse to the next decade.

## Usage

```typescript
import {
  createDecadeCounter,
  pulseDecadeCounter,
  createAccumulator,
  accumulatorAdd,
  accumulatorValue,
} from "@coding-adventures/eniac";

// Decade ring counter: 10 tubes representing one digit
let counter = createDecadeCounter(7);  // digit = 7
let result = pulseDecadeCounter(counter, 5);
// 7→8→9→0→1→2, carry generated at 9→0
// result.counter.currentDigit === 2
// result.carry === true
// result.stepsTraced === [8, 9, 0, 1, 2]

// Multi-digit accumulator: 42 + 75 = 117
let acc = createAccumulator(42, 4);    // 4-digit accumulator set to 42
let trace = accumulatorAdd(acc, 75);
// accumulatorValue(trace.accumulator) === 117
// trace.carries === [false, true, false, false]
// Tens digit carried: 4+7=11 → digit=1, carry to hundreds
```

## Where it fits

```
[ENIAC (1945)] → Transistors → Logic Gates → Arithmetic → CPU
```

## Installation

```bash
npm install @coding-adventures/eniac
```
