# Changelog

## [0.1.0] - 2026-03-23

### Added

- `createDecadeCounter(digit?)` — create a 10-tube ring counter initialized to a decimal digit (0-9)
- `pulseDecadeCounter(counter, pulses?)` — advance the ring by N pulses, with carry detection on 9→0 wraparound and step-by-step trace
- `TubeState`, `DecadeCounter`, `PulseResult` interfaces
- `createAccumulator(value?, digitCount?)` — create a multi-digit accumulator from chained ring counters
- `accumulatorValue(acc)` — read the decimal value stored in an accumulator
- `accumulatorAdd(acc, addend)` — add a value using ENIAC's pulse counting method with carry propagation
- `Accumulator`, `DigitTrace`, `AdditionTrace` interfaces with per-digit trace data
- 33 tests covering ring counter behavior, accumulator creation, addition with carry, overflow, and trace verification
- Knuth-style literate programming documentation throughout
