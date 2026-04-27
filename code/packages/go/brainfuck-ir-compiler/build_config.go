// Package brainfuckircompiler compiles Brainfuck ASTs into the general-purpose
// intermediate representation (IR) defined by the compiler-ir package.
//
// This is the Brainfuck-specific frontend of the AOT compiler pipeline. It
// knows Brainfuck semantics (tape, cells, pointer, loops, I/O) and translates
// them into target-independent IR instructions. It does NOT know about RISC-V,
// ARM, ELF, or any specific machine target.
//
// The compiler produces two outputs:
//   1. An IrProgram containing the compiled IR instructions
//   2. SourceToAst and AstToIr source map segments for debugging
//
// See spec BF03 for the full pipeline architecture.
package brainfuckircompiler

// ──────────────────────────────────────────────────────────────────────────────
// BuildConfig — controls what the compiler emits
//
// Build modes are **composable flags**, not a fixed enum. A BuildConfig
// object controls every aspect of compilation:
//
//   - InsertBoundsChecks: emit tape pointer range checks (debug builds)
//   - InsertDebugLocs: emit source location markers (useful for debugging)
//   - MaskByteArithmetic: AND 0xFF after every cell mutation (correctness)
//   - TapeSize: configurable tape length (default 30,000 cells)
//
// ──────────────────────────────────────────────────────────────────────────────
// Presets
//
//   DebugConfig:   bounds checks ON, debug locs ON, masking ON
//   ReleaseConfig: bounds checks OFF, debug locs OFF, masking ON
//
// New modes can be added without modifying existing code — just construct
// a BuildConfig with the desired flags.
// ──────────────────────────────────────────────────────────────────────────────

type BuildConfig struct {
	// InsertBoundsChecks adds tape pointer range checks before every
	// pointer move (< and >). If the pointer goes out of bounds, the
	// program traps to __trap_oob. This catches bugs in Brainfuck
	// programs but costs ~2 instructions per pointer move.
	InsertBoundsChecks bool

	// InsertDebugLocs emits COMMENT instructions with source locations.
	// These are stripped by the packager in release builds but help
	// when reading IR output during development.
	InsertDebugLocs bool

	// MaskByteArithmetic emits AND_IMM v, v, 255 after every cell
	// mutation (INC, DEC). This ensures cells stay in the 0-255 range
	// per the Brainfuck specification. Backends that guarantee byte-width
	// stores can skip this via an optimiser pass (mask_elision).
	MaskByteArithmetic bool

	// TapeSize is the number of cells in the Brainfuck tape.
	// The default is 30,000, which is the canonical size from the
	// original Brainfuck specification.
	TapeSize int
}

// DebugConfig returns a BuildConfig suitable for debug builds.
// All safety checks are enabled.
func DebugConfig() BuildConfig {
	return BuildConfig{
		InsertBoundsChecks: true,
		InsertDebugLocs:    true,
		MaskByteArithmetic: true,
		TapeSize:           30000,
	}
}

// ReleaseConfig returns a BuildConfig suitable for release builds.
// Safety checks are disabled for maximum performance.
func ReleaseConfig() BuildConfig {
	return BuildConfig{
		InsertBoundsChecks: false,
		InsertDebugLocs:    false,
		MaskByteArithmetic: true,
		TapeSize:           30000,
	}
}
