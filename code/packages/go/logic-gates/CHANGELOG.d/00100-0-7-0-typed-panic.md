## [0.7.0] - 2026-11-24

### Changed

- **Typed rejection identity (LGE-R-go).** Bit validation now panics with
  `*InvalidBitError{Param, Value}` instead of a plain string. Callers (and
  Warrant bindings) identify rejection with `errors.As` / `IsInvalidBit`, never
  by message. The message text is unchanged. Slice parameters report
  `name[index]` as `Param`.
- Internal CMOS evaluation failures now panic with `*InternalError`, a different
  type, so an implementation bug can never be mapped to input rejection.
- Structural/arity panics (e.g. `ANDn` with one input, `Mux4` selector width)
  remain plain strings; they are not input rejection.

### Added

- `InvalidBitError`, `InternalError`, `IsInvalidBit`.
- Exhaustive tests: literal truth tables for all gates, NAND-built equals
  direct, every integer in [-1024, 1024] except 0/1 plus MinInt/MaxInt
  extremes in every argument position, no valid input panics, all results are
  bits. Documented that Go's `int` parameter type makes the type-confusion
  matrix a compile-time property.

### Fixed

- Test helper `assertPanics` ignored its expected value; it now asserts the
  panic is `*InvalidBitError` (and `assertInvalidBit` checks Param/Value/message).

### Security

- `IsInvalidBit` is now an exact type assertion (no error-chain walk); an `*InternalError` or foreign wrapper around an `*InvalidBitError` is never a rejection. Typed-nil is rejected and `(*InvalidBitError)(nil).Error()` is safe.
- Bounded widths (string shape panic before any allocation/shift): Decoder n <= 16 (`MaxSelectBits`), MuxN/Demux/Encoder/PriorityEncoder size <= 65536 (`MaxWidth`), Counter width <= 64 (`MaxCounterWidth`) with `len(state.Bits) == Width` required.
- Counter validates `state.Bits` on every path (hold/reset/increment); DFlipFlop/Register/ShiftRegister validate FlipFlopState fields up front with Params like `state.MasterQ`, `state[2].SlaveQBar`.
