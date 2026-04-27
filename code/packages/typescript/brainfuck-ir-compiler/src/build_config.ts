/**
 * BuildConfig — controls what the Brainfuck IR compiler emits.
 *
 * =============================================================================
 * Design: composable flags, not a fixed enum
 * =============================================================================
 *
 * Build modes are **composable flags**, not a fixed enum. A BuildConfig
 * object controls every aspect of compilation independently:
 *
 *   - insertBoundsChecks: emit tape pointer range checks (debug builds)
 *   - insertDebugLocs:    emit source location markers (useful for debugging)
 *   - maskByteArithmetic: AND 0xFF after every cell mutation (correctness)
 *   - tapeSize:           configurable tape length (default 30,000 cells)
 *
 * =============================================================================
 * Presets
 * =============================================================================
 *
 *   debugConfig():   bounds checks ON, debug locs ON, masking ON
 *   releaseConfig(): bounds checks OFF, debug locs OFF, masking ON
 *
 * New modes can be added without modifying existing code — just construct
 * a BuildConfig with the desired flags. This is the "open-closed principle"
 * applied to compiler configuration: open for extension, closed for modification.
 *
 * =============================================================================
 * Why separate bounds checks from masking?
 * =============================================================================
 *
 * Bounds checking (insertBoundsChecks) and byte masking (maskByteArithmetic)
 * are independent safety mechanisms:
 *
 *   - Bounds checks: prevent the tape *pointer* from going out of the tape's
 *     memory range. A bounds-checking failure means the program would access
 *     unallocated memory — a hard crash.
 *
 *   - Byte masking: ensures tape *cell values* stay in the 0-255 range per
 *     the Brainfuck spec. Backends that use actual byte-width stores can skip
 *     this (because a byte store naturally masks to 0-255).
 *
 * Releasing with maskByteArithmetic=true but insertBoundsChecks=false is
 * the typical release build: correct Brainfuck semantics, no runtime overhead
 * for pointer range validation.
 */

export interface BuildConfig {
  /**
   * Insert tape pointer range checks before every pointer move (< and >).
   * If the pointer goes out of bounds, the program jumps to __trap_oob.
   *
   * Cost: ~2 instructions per pointer move (CMP_GT/CMP_LT + BRANCH_NZ).
   * Benefit: catches out-of-bounds tape access at runtime instead of crashing
   *   with a segfault or corrupting memory.
   */
  readonly insertBoundsChecks: boolean;

  /**
   * Emit COMMENT instructions with source locations.
   * These are stripped by the packager in release builds but help
   * when reading IR output during development.
   *
   * Currently unused in the core compiler (reserved for future use).
   */
  readonly insertDebugLocs: boolean;

  /**
   * Emit AND_IMM v, v, 255 after every cell mutation (INC, DEC).
   * This ensures cells stay in the 0-255 range per the Brainfuck specification.
   *
   * Backends that guarantee byte-width stores can skip this via an optimiser
   * pass (mask_elision). The masking is correct by default so release builds
   * keep it on unless an optimiser removes it.
   */
  readonly maskByteArithmetic: boolean;

  /**
   * Number of cells in the Brainfuck tape.
   * The default is 30,000, which is the canonical size from the original
   * Brainfuck specification by Urban Mueller (1993).
   *
   * Some programs require a larger tape. The max is implementation-defined
   * but must be > 0.
   */
  readonly tapeSize: number;
}

/**
 * debugConfig returns a BuildConfig suitable for debug builds.
 * All safety checks are enabled.
 *
 * Use this when developing or testing Brainfuck programs — it provides the
 * best diagnostic information when something goes wrong.
 *
 * @returns BuildConfig with all checks enabled, 30,000-cell tape.
 */
export function debugConfig(): BuildConfig {
  return {
    insertBoundsChecks: true,
    insertDebugLocs: true,
    maskByteArithmetic: true,
    tapeSize: 30000,
  };
}

/**
 * releaseConfig returns a BuildConfig suitable for release builds.
 * Bounds checks are disabled for maximum performance.
 * Byte masking is retained for correct Brainfuck semantics.
 *
 * Use this for final compilation when the program is known to be correct
 * and performance matters.
 *
 * @returns BuildConfig with bounds checks off, masking on, 30,000-cell tape.
 */
export function releaseConfig(): BuildConfig {
  return {
    insertBoundsChecks: false,
    insertDebugLocs: false,
    maskByteArithmetic: true,
    tapeSize: 30000,
  };
}
