# swift/logic_gates

Boolean logic gates and combinational/sequential circuits implemented in Swift,
built on top of [`swift/transistors`](../transistors) — the CMOS simulation
layer below them in the computing stack.

## What this package is

Every gate in this library delegates to a CMOS transistor simulation rather
than using Swift's `&&` or `||` operators. This means you can trace the signal
all the way from individual p-type/n-type transistors up through logic gates,
up through latches and flip-flops, up through registers and counters — every
layer is built from the layer beneath it.

```
Transistors   →   Logic Gates   →   Sequential / Combinational circuits
(this repo)       (this package)    (this package)
```

## Installation

Add to your `Package.swift` as a local dependency:

```swift
.package(path: "../logic_gates")
```

Then add `"LogicGates"` to your target's dependencies.

## Primitive gates

All gates accept only 0 or 1. Any other value throws `LogicGateError.invalidBit`.

```swift
import LogicGates

try notGate(0)          // → 1  (CMOS inverter)
try andGate(1, 1)       // → 1  (CMOS AND)
try orGate(0, 1)        // → 1  (CMOS OR)
try xorGate(1, 1)       // → 0  (CMOS XOR)
try nandGate(1, 1)      // → 0  (CMOS NAND)
try norGate(0, 0)       // → 1  (CMOS NOR)
try xnorGate(0, 1)      // → 0  (CMOS XNOR)
```

### NAND functional completeness

NAND is a universal gate — you can build any Boolean function from NAND alone.
This package includes implementations to prove it:

```swift
try nandNot(0)          // → 1  (NOT via NAND)
try nandAnd(1, 1)       // → 1  (AND via NAND)
try nandOr(0, 1)        // → 1  (OR via NAND)
try nandXor(1, 1)       // → 0  (XOR via NAND)
```

### Multi-input gates

```swift
try andN([1, 1, 1, 0])  // → 0  (any 0 → 0)
try orN([0, 0, 0, 1])   // → 1  (any 1 → 1)
```

## Sequential circuits

### SR Latch

The fundamental memory element. Built from two cross-coupled NOR gates.
Pass the previous Q and Q̄ values to model hold behaviour between cycles.

```swift
let set  = try srLatch(set: 1, reset: 0)   // → q: 1, qBar: 0
let hold = try srLatch(set: 0, reset: 0,
                       q: set.q, qBar: set.qBar)  // → q: 1 (held)
```

Truth table:

| S | R | Q  | Q̄  | Comment         |
|---|---|----|-----|-----------------|
| 0 | 0 | Q  | Q̄  | Hold (no change) |
| 1 | 0 | 1  | 0   | Set              |
| 0 | 1 | 0  | 1   | Reset            |
| 1 | 1 | 0  | 0   | Invalid (avoid)  |

### D Latch

A level-sensitive latch. When `enable=1` the output follows the data input;
when `enable=0` the output is frozen.

```swift
let transparent = try dLatch(data: 1, enable: 1)  // q=1
let opaque      = try dLatch(data: 0, enable: 0,
                             q: 1, qBar: 0)         // q=1 (held)
```

### D Flip-Flop (master-slave)

Edge-triggered storage. Q captures D on the rising clock edge (CLK 0 → 1).
The master samples when CLK=0; the slave propagates when CLK=1.

```swift
let low  = try dFlipFlop(data: 1, clock: 0)
let high = try dFlipFlop(data: 1, clock: 1,
                         q: low.q, qBar: low.qBar,
                         masterQ: low.masterQ,
                         masterQBar: 1 - low.masterQ)
// high.q == 1
```

### Register

N parallel flip-flops. All bits load simultaneously on the rising clock edge.

```swift
let (q, states) = try register(data: [1, 0, 1, 0], clock: 1)
// q == [1, 0, 1, 0]
```

### Shift Register (4-bit default)

Serial-in / parallel-out. Each rising clock edge shifts bits left, discarding
bit N-1 and inserting `serialIn` at position 0.

```swift
var q = [0, 0, 0, 0]
(q, _, _) = try shiftRegister(serialIn: 1, clock: 1, q: q)  // [1,0,0,0]
(q, _, _) = try shiftRegister(serialIn: 1, clock: 1, q: q)  // [1,1,0,0]
```

### Counter (4-bit default)

Binary up-counter with overflow flag and synchronous reset.

```swift
var q = [0, 0, 0, 0]
var overflow = 0
(q, overflow, _) = try counter(clock: 1, q: q)   // q=[0,0,0,1]
// After 15 more increments:
(q, overflow, _) = try counter(clock: 1, q: [1,1,1,1])  // q=[0,0,0,0], overflow=1

// Synchronous reset
(q, _, _) = try counter(clock: 1, reset: 1, q: [1,0,1,0])  // q=[0,0,0,0]
```

## Combinational circuits

### Multiplexer (MUX)

Routes one of N inputs to the output based on select bits.

```swift
try mux2(d0: 0, d1: 1, sel: 1)          // → 1
try mux4(d0: 1, d1: 0, d2: 0, d3: 0,
         sel: [0, 0])                     // → 1
try muxN(inputs: [0, 0, 1, 0], sel: [0, 1])  // → 1 (input 2)
```

### Demultiplexer (DEMUX)

Routes one data input to one of 2^N outputs.

```swift
let out = try demux(data: 1, sel: [1, 0])
// out == [0, 1, 0, 0]  (data routed to output 1)
```

### Decoder

Activates exactly one of 2^N outputs based on an N-bit binary input (one-hot output).

```swift
try decoder(inputs: [1, 0])  // → [0, 1, 0, 0]  (input 1 → output 1 active)
try decoder(inputs: [1, 1])  // → [0, 0, 0, 1]  (input 3 → output 3 active)
```

`inputs[0]` is the LSB; `inputs[N-1]` is the MSB.

### Encoder

The inverse of a decoder. Requires exactly one active input; throws otherwise.

```swift
try encoder(inputs: [0, 1, 0, 0])  // → [1, 0]  (input 1 → binary 1)
try encoder(inputs: [0, 0, 0, 1])  // → [1, 1]  (input 3 → binary 3)
```

### Priority Encoder

Like an encoder but handles multiple active inputs — the highest-index active
input wins. Returns `(output: [Int], valid: Int)` where `valid=0` means no
inputs were active.

```swift
let (out, valid) = try priorityEncoder(inputs: [1, 0, 1, 0])
// valid=1, out=[0,1] → index 2 wins
```

### Tri-State Buffer

Returns `Int?`. `nil` means high-impedance (high-Z) — the output is
electrically disconnected from the bus.

```swift
try triState(data: 1, enable: 1)  // → Optional(1)
try triState(data: 1, enable: 0)  // → nil  (high-Z)
```

## Error handling

```swift
public enum LogicGateError: Error {
    case invalidBit(name: String, got: Int)
    case insufficientInputs(minimum: Int, got: Int)
    case invalidSelectLength(expected: Int, got: Int)
    case invalidEncoderInput(String)
}
```

All functions are marked `throws`. Catch errors to get readable descriptions:

```swift
do {
    _ = try notGate(5)
} catch let e as LogicGateError {
    print(e.description)
    // "invalid bit 'a': expected 0 or 1, got 5"
}
```

## Where this fits in the stack

```
spec 10: logic-gates   ← this package
spec 09: transistors   ← swift/transistors (CMOS simulation)
spec 08: logic-gates   ← concept level
```

The next layer above this package is `swift/arithmetic` (adders, ALU, etc.),
which will build adders and comparators from the gates provided here.
