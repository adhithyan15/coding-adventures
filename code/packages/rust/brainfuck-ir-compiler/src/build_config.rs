//! # BuildConfig — compilation mode flags.
//!
//! Build modes are **composable flags**, not a fixed enum. A `BuildConfig`
//! controls every aspect of compilation:
//!
//! - `insert_bounds_checks` — emit tape pointer range checks (debug builds)
//! - `insert_debug_locs` — emit source location markers (debugging)
//! - `mask_byte_arithmetic` — AND 0xFF after every cell mutation (correctness)
//! - `tape_size` — configurable tape length (default 30,000 cells)
//!
//! ## Presets
//!
//! | Preset            | Bounds checks | Debug locs | Byte masking |
//! |-------------------|---------------|------------|--------------|
//! | `debug_config()`  | ON            | ON         | ON           |
//! | `release_config()`| OFF           | OFF        | ON           |
//!
//! New modes can be added without modifying existing code — just construct
//! a `BuildConfig` with the desired flags.
//!
//! ## Example
//!
//! ```
//! use brainfuck_ir_compiler::build_config::{BuildConfig, debug_config, release_config};
//!
//! let dbg = debug_config();
//! assert!(dbg.insert_bounds_checks);
//! assert!(dbg.mask_byte_arithmetic);
//! assert_eq!(dbg.tape_size, 30000);
//!
//! let rel = release_config();
//! assert!(!rel.insert_bounds_checks);
//! assert!(rel.mask_byte_arithmetic); // always on in release
//! ```

// ===========================================================================
// BuildConfig
// ===========================================================================

/// Controls what the Brainfuck IR compiler emits.
///
/// All fields are public so callers can override individual settings
/// from a preset. For example, to use release mode with a custom tape size:
///
/// ```
/// use brainfuck_ir_compiler::build_config::release_config;
/// let mut config = release_config();
/// config.tape_size = 1000;
/// assert_eq!(config.tape_size, 1000);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildConfig {
    /// Add tape pointer range checks before every pointer move (`<` and `>`).
    ///
    /// If the pointer goes out of bounds, the program traps to `__trap_oob`.
    /// This catches bugs in Brainfuck programs but costs ~2 instructions per
    /// pointer move.
    pub insert_bounds_checks: bool,

    /// Emit `COMMENT` instructions with source locations.
    ///
    /// These are stripped by the packager in release builds but help when
    /// reading IR output during development.
    pub insert_debug_locs: bool,

    /// Emit `AND_IMM v, v, 255` after every cell mutation (INC, DEC).
    ///
    /// This ensures cells stay in the 0–255 range per the Brainfuck
    /// specification. Backends that guarantee byte-width stores can skip
    /// this via an optimiser pass (`mask_elision`).
    pub mask_byte_arithmetic: bool,

    /// Number of cells in the Brainfuck tape.
    ///
    /// The default is 30,000, which is the canonical size from the original
    /// Brainfuck specification. Must be greater than 0.
    pub tape_size: usize,
}

// ===========================================================================
// Presets
// ===========================================================================

/// Return a `BuildConfig` suitable for debug builds.
///
/// All safety checks are enabled.
///
/// # Example
///
/// ```
/// use brainfuck_ir_compiler::build_config::debug_config;
/// let cfg = debug_config();
/// assert!(cfg.insert_bounds_checks);
/// assert!(cfg.insert_debug_locs);
/// assert!(cfg.mask_byte_arithmetic);
/// assert_eq!(cfg.tape_size, 30000);
/// ```
pub fn debug_config() -> BuildConfig {
    BuildConfig {
        insert_bounds_checks: true,
        insert_debug_locs: true,
        mask_byte_arithmetic: true,
        tape_size: 30000,
    }
}

/// Return a `BuildConfig` suitable for release builds.
///
/// Safety checks that cost runtime performance are disabled.
///
/// # Example
///
/// ```
/// use brainfuck_ir_compiler::build_config::release_config;
/// let cfg = release_config();
/// assert!(!cfg.insert_bounds_checks);
/// assert!(!cfg.insert_debug_locs);
/// assert!(cfg.mask_byte_arithmetic);
/// assert_eq!(cfg.tape_size, 30000);
/// ```
pub fn release_config() -> BuildConfig {
    BuildConfig {
        insert_bounds_checks: false,
        insert_debug_locs: false,
        mask_byte_arithmetic: true,
        tape_size: 30000,
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_config() {
        let cfg = debug_config();
        assert!(cfg.insert_bounds_checks, "debug should have bounds checks");
        assert!(cfg.insert_debug_locs, "debug should have debug locs");
        assert!(cfg.mask_byte_arithmetic, "debug should have byte masking");
        assert_eq!(cfg.tape_size, 30000);
    }

    #[test]
    fn test_release_config() {
        let cfg = release_config();
        assert!(!cfg.insert_bounds_checks, "release should NOT have bounds checks");
        assert!(!cfg.insert_debug_locs, "release should NOT have debug locs");
        assert!(cfg.mask_byte_arithmetic, "release should have byte masking");
        assert_eq!(cfg.tape_size, 30000);
    }

    #[test]
    fn test_custom_tape_size() {
        let mut cfg = release_config();
        cfg.tape_size = 1000;
        assert_eq!(cfg.tape_size, 1000);
    }

    #[test]
    fn test_clone() {
        let cfg = debug_config();
        let cloned = cfg.clone();
        assert_eq!(cfg, cloned);
    }

    #[test]
    fn test_debug_differs_from_release() {
        let dbg = debug_config();
        let rel = release_config();
        assert_ne!(dbg, rel);
        assert_ne!(dbg.insert_bounds_checks, rel.insert_bounds_checks);
    }
}
