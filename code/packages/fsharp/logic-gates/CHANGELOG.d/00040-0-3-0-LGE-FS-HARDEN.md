## 0.3.0

### Changed (LGE-FS-HARDEN)

- BREAKING: `InvalidBitException` is now `[<Sealed>]`, and a null parameter name raises `ArgumentNullException("parameter")` instead of being propagated.
- BREAKING: the exception message now reads `Bit values must be 0 or 1, got N.` and is rendered in the invariant culture (the `Message` override avoids the BCL's current-culture `Actual value was N`). N-ary shape messages are culture-free too.
- N-ary bit errors use `InvalidBitException` with `inputs[i]` names.
- `TreatWarningsAsErrors`, F# 9 nullness (`<Nullable>enable`), FS0025 (incomplete matches) and `--checked+` are on for the library. No caller input can reach an `OverflowException`.
- New `LogicGates.fsi` pins the public API (every gate `int -> int -> int`, N-ary `int list -> int`).
- BUILD / BUILD_windows run tests in Debug and Release; the 95% line-coverage gate runs on Release.
- Version bumped 0.2.0 -> 0.3.0 (the 0.2.1 N-ary caps shard is folded in).

### Added

- `LogicGatesHardeningTests`: custom-NegativeSign culture, null names, sealed type, extremes with no overflow, monomorphic public surface.
- Not in scope (tracked under LGE-DOTNET-SDK): lock files and the target-framework move.
