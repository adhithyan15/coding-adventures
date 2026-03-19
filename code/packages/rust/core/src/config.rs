//! CoreConfig -- complete configuration for a processor core.
//!
//! # The Core: a Motherboard for Micro-Architecture
//!
//! A processor core is not a single piece of hardware. It is a composition of
//! many sub-components, each independently designed and tested:
//!
//!   - Pipeline (D04): moves instructions through stages (IF, ID, EX, MEM, WB)
//!   - Branch Predictor (D02): guesses which way branches will go
//!   - Hazard Detection (D03): detects data, control, and structural hazards
//!   - Cache Hierarchy (D01): L1I, L1D, optional L2 for fast memory access
//!   - Register File: fast storage for operands and results
//!   - Clock: drives everything in lockstep
//!
//! Every parameter that a real CPU architect would tune is exposed in
//! [`CoreConfig`]. Change the branch predictor and you get different accuracy.
//! Double the L1 cache and you get fewer misses. Deepen the pipeline and
//! you get higher clock speeds but worse misprediction penalties.

use branch_predictor::{
    AlwaysNotTakenPredictor, AlwaysTakenPredictor, BackwardTakenForwardNotTaken, BranchPredictor,
    OneBitPredictor, TwoBitPredictor, TwoBitState,
};
use cache::CacheConfig;
use cpu_pipeline::PipelineConfig;

// =========================================================================
// RegisterFileConfig -- configuration for the register file
// =========================================================================

/// Configuration for the general-purpose register file.
///
/// Real-world register file sizes:
///
/// ```text
///   MIPS:     32 registers, 32-bit  (R0 hardwired to zero)
///   ARMv8:    31 registers, 64-bit  (X0-X30, no zero register)
///   RISC-V:   32 registers, 32/64-bit (x0 hardwired to zero)
///   x86-64:   16 registers, 64-bit  (RAX, RBX, ..., R15)
/// ```
///
/// The zero_register convention (RISC-V, MIPS) simplifies instruction encoding:
/// any instruction can discard its result by writing to R0, and any instruction
/// can use "zero" as an operand without a special immediate encoding.
#[derive(Debug, Clone)]
pub struct RegisterFileConfig {
    /// Number of general-purpose registers.
    /// Typical values: 16 (ARM Thumb, x86), 32 (MIPS, RISC-V, ARMv8).
    pub count: usize,

    /// Bit width of each register: 32 or 64.
    pub width: usize,

    /// Whether register 0 is hardwired to zero.
    /// RISC-V and MIPS: true. ARM and x86: false.
    /// When true, writes to R0 are silently ignored and reads always return 0.
    pub zero_register: bool,
}

impl Default for RegisterFileConfig {
    /// Returns a sensible default: 16 registers, 32-bit, R0 hardwired to zero
    /// (RISC-V convention).
    fn default() -> Self {
        Self {
            count: 16,
            width: 32,
            zero_register: true,
        }
    }
}

// =========================================================================
// FPUnitConfig -- configuration for the floating-point unit
// =========================================================================

/// Configuration for the optional floating-point unit.
///
/// Not all cores have an FP unit. Microcontrollers (ARM Cortex-M0) and
/// efficiency cores often omit it to save area and power. When `fp_unit`
/// is None in CoreConfig, the core has no floating-point support.
#[derive(Debug, Clone)]
pub struct FPUnitConfig {
    /// Supported FP formats: "fp16", "fp32", "fp64".
    pub formats: Vec<String>,

    /// How many cycles an FP operation takes.
    /// Typical: 3-5 for add/multiply, 10-20 for divide.
    pub pipeline_depth: usize,
}

// =========================================================================
// CoreConfig -- complete configuration for a processor core
// =========================================================================

/// Holds every tunable parameter for a processor core.
///
/// This is the "spec sheet" for the core. A CPU architect decides these
/// values based on the target workload, power budget, and die area.
///
/// Changing any parameter affects measurable performance:
///
/// ```text
///   Deeper pipeline         -> higher clock speed, worse misprediction penalty
///   Better branch predictor -> fewer pipeline flushes
///   Larger L1 cache         -> fewer cache misses
///   More registers          -> fewer spills to memory
///   Forwarding enabled      -> fewer stall cycles
/// ```
#[derive(Debug, Clone)]
pub struct CoreConfig {
    /// Human-readable identifier for this configuration.
    /// Examples: "Simple", "CortexA78Like", "AppleM4Like".
    pub name: String,

    // --- Pipeline ---

    /// Pipeline stage configuration. Defaults to classic 5-stage.
    pub pipeline: PipelineConfig,

    // --- Branch Prediction ---

    /// Predictor algorithm:
    ///   "static_always_taken"     -- always predicts taken
    ///   "static_always_not_taken" -- always predicts not taken
    ///   "static_btfnt"            -- backward-taken, forward-not-taken
    ///   "one_bit"                 -- 1-bit dynamic predictor
    ///   "two_bit"                 -- 2-bit saturating counter (default)
    pub branch_predictor_type: String,

    /// Number of entries in the prediction table.
    /// Only used for dynamic predictors (one_bit, two_bit). Typical: 256-4096.
    pub branch_predictor_size: usize,

    /// Number of entries in the Branch Target Buffer.
    /// The BTB caches WHERE branches go (target addresses).
    pub btb_size: usize,

    // --- Hazard Handling ---

    /// Enables the hazard detection unit.
    /// When false, the pipeline assumes no hazards (for testing).
    pub hazard_detection: bool,

    /// Enables data forwarding (bypassing) paths.
    /// When true, the EX and MEM stages can forward results to earlier stages,
    /// avoiding stalls for many RAW hazards.
    pub forwarding: bool,

    // --- Register File ---

    /// Configuration for the general-purpose register file.
    /// If None, defaults to 16 registers, 32-bit, zero register enabled.
    pub register_file: Option<RegisterFileConfig>,

    // --- Floating Point ---

    /// Configuration for the floating-point unit. None = no FP support.
    pub fp_unit: Option<FPUnitConfig>,

    // --- Cache Hierarchy ---

    /// L1 instruction cache configuration.
    /// If None, a default 4KB direct-mapped cache is used.
    pub l1i_cache: Option<CacheConfig>,

    /// L1 data cache configuration.
    /// If None, a default 4KB direct-mapped cache is used.
    pub l1d_cache: Option<CacheConfig>,

    /// Unified L2 cache configuration. None = no L2.
    pub l2_cache: Option<CacheConfig>,

    // --- Memory ---

    /// Size of main memory in bytes. Default: 65536 (64KB).
    pub memory_size: usize,

    /// Access latency for main memory in cycles.
    /// Real DRAM: 50-100+ cycles. Default: 100.
    pub memory_latency: usize,
}

impl Default for CoreConfig {
    /// Returns a minimal, sensible configuration for testing.
    ///
    /// This is the "teaching core" -- a 5-stage pipeline with static prediction,
    /// small caches, and 16 registers. Equivalent to a 1980s RISC microprocessor.
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            pipeline: PipelineConfig::classic_5_stage(),
            branch_predictor_type: "static_always_not_taken".to_string(),
            branch_predictor_size: 256,
            btb_size: 64,
            hazard_detection: true,
            forwarding: true,
            register_file: None,
            fp_unit: None,
            l1i_cache: None,
            l1d_cache: None,
            l2_cache: None,
            memory_size: 65536,
            memory_latency: 100,
        }
    }
}

// =========================================================================
// Preset Configurations -- famous real-world cores approximated
// =========================================================================

/// Returns a minimal teaching core configuration.
///
/// Inspired by the MIPS R2000 (1985):
///   - 5-stage pipeline (IF, ID, EX, MEM, WB)
///   - Static predictor (always not taken)
///   - 4KB direct-mapped L1I and L1D caches
///   - No L2 cache
///   - 16 registers, 32-bit
///   - No floating point
///
/// Expected IPC: ~0.7-0.9 on simple programs.
pub fn simple_config() -> CoreConfig {
    let l1i = CacheConfig::new("L1I", 4096, 64, 1, 1);
    let l1d = CacheConfig::new("L1D", 4096, 64, 1, 1);
    let reg_cfg = RegisterFileConfig {
        count: 16,
        width: 32,
        zero_register: true,
    };

    CoreConfig {
        name: "Simple".to_string(),
        pipeline: PipelineConfig::classic_5_stage(),
        branch_predictor_type: "static_always_not_taken".to_string(),
        branch_predictor_size: 256,
        btb_size: 64,
        hazard_detection: true,
        forwarding: true,
        register_file: Some(reg_cfg),
        fp_unit: None,
        l1i_cache: Some(l1i),
        l1d_cache: Some(l1d),
        l2_cache: None,
        memory_size: 65536,
        memory_latency: 100,
    }
}

/// Approximates the ARM Cortex-A78 performance core.
///
/// The Cortex-A78 (2020) is used in Snapdragon 888 and Dimensity 9000:
///   - 13-stage pipeline (deep for high frequency)
///   - 2-bit predictor with 4096 entries (simplified vs real TAGE)
///   - 64KB 4-way L1I and L1D
///   - 256KB 8-way L2
///   - 31 registers, 64-bit (ARMv8)
///   - FP32 and FP64 support
///
/// Expected IPC: ~0.85-0.95 (our model is in-order; real A78 is out-of-order).
pub fn cortex_a78_like_config() -> CoreConfig {
    let l1i = CacheConfig::new("L1I", 65536, 64, 4, 1);
    let l1d = CacheConfig::new("L1D", 65536, 64, 4, 1);
    let l2 = CacheConfig::new("L2", 262144, 64, 8, 12);
    let reg_cfg = RegisterFileConfig {
        count: 31,
        width: 64,
        zero_register: false,
    };
    let fp_cfg = FPUnitConfig {
        formats: vec!["fp32".to_string(), "fp64".to_string()],
        pipeline_depth: 4,
    };

    CoreConfig {
        name: "CortexA78Like".to_string(),
        pipeline: PipelineConfig::deep_13_stage(),
        branch_predictor_type: "two_bit".to_string(),
        branch_predictor_size: 4096,
        btb_size: 1024,
        hazard_detection: true,
        forwarding: true,
        register_file: Some(reg_cfg),
        fp_unit: Some(fp_cfg),
        l1i_cache: Some(l1i),
        l1d_cache: Some(l1d),
        l2_cache: Some(l2),
        memory_size: 1048576,
        memory_latency: 100,
    }
}

// =========================================================================
// MultiCoreConfig -- configuration for a multi-core processor
// =========================================================================

/// Configuration for a multi-core CPU.
///
/// In a multi-core system, each core has its own L1 and L2 caches but
/// shares an L3 cache and main memory. The memory controller serializes
/// requests from multiple cores.
///
/// Real-world multi-core counts:
///
/// ```text
///   Raspberry Pi 4:     4 cores (Cortex-A72)
///   Apple M4:           4P + 6E = 10 cores
///   AMD Ryzen 9 7950X:  16 cores
///   Server chips:       64-128 cores
/// ```
#[derive(Debug, Clone)]
pub struct MultiCoreConfig {
    /// Number of processor cores.
    pub num_cores: usize,

    /// Configuration shared by all cores.
    /// (Heterogeneous multi-core is a future extension.)
    pub core_config: CoreConfig,

    /// Shared L3 cache configuration. None = no L3.
    pub l3_cache: Option<CacheConfig>,

    /// Total shared memory in bytes.
    pub memory_size: usize,

    /// DRAM access latency in cycles.
    pub memory_latency: usize,
}

impl Default for MultiCoreConfig {
    /// Returns a 2-core configuration for testing.
    fn default() -> Self {
        Self {
            num_cores: 2,
            core_config: simple_config(),
            l3_cache: None,
            memory_size: 1048576,
            memory_latency: 100,
        }
    }
}

// =========================================================================
// Helper: create branch predictor from config
// =========================================================================

/// Builds a `Box<dyn BranchPredictor>` from the config strings.
///
/// This factory function decouples the config (which uses strings) from the
/// concrete predictor types. The Core calls this once during construction.
pub fn create_branch_predictor(typ: &str, size: usize) -> Box<dyn BranchPredictor> {
    match typ {
        "static_always_taken" => Box::new(AlwaysTakenPredictor::new()),
        "static_always_not_taken" => Box::new(AlwaysNotTakenPredictor::new()),
        "static_btfnt" => Box::new(BackwardTakenForwardNotTaken::new()),
        "one_bit" => Box::new(OneBitPredictor::new(size)),
        "two_bit" => Box::new(TwoBitPredictor::new(size, TwoBitState::WeaklyNotTaken)),
        _ => Box::new(AlwaysNotTakenPredictor::new()),
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = CoreConfig::default();
        assert_eq!(cfg.name, "Default");
        assert_eq!(cfg.memory_size, 65536);
        assert!(cfg.hazard_detection);
        assert!(cfg.forwarding);
    }

    #[test]
    fn test_simple_config() {
        let cfg = simple_config();
        assert_eq!(cfg.name, "Simple");
        assert!(cfg.l1i_cache.is_some());
        assert!(cfg.l1d_cache.is_some());
        assert!(cfg.l2_cache.is_none());
        assert!(cfg.fp_unit.is_none());
        let reg = cfg.register_file.unwrap();
        assert_eq!(reg.count, 16);
        assert_eq!(reg.width, 32);
        assert!(reg.zero_register);
    }

    #[test]
    fn test_cortex_a78_config() {
        let cfg = cortex_a78_like_config();
        assert_eq!(cfg.name, "CortexA78Like");
        assert!(cfg.l2_cache.is_some());
        assert!(cfg.fp_unit.is_some());
        let reg = cfg.register_file.unwrap();
        assert_eq!(reg.count, 31);
        assert_eq!(reg.width, 64);
        assert!(!reg.zero_register);
    }

    #[test]
    fn test_default_register_file_config() {
        let cfg = RegisterFileConfig::default();
        assert_eq!(cfg.count, 16);
        assert_eq!(cfg.width, 32);
        assert!(cfg.zero_register);
    }

    #[test]
    fn test_multi_core_config_default() {
        let cfg = MultiCoreConfig::default();
        assert_eq!(cfg.num_cores, 2);
        assert!(cfg.l3_cache.is_none());
        assert_eq!(cfg.memory_size, 1048576);
    }

    #[test]
    fn test_create_branch_predictor_types() {
        let _p1 = create_branch_predictor("static_always_taken", 256);
        let _p2 = create_branch_predictor("static_always_not_taken", 256);
        let _p3 = create_branch_predictor("static_btfnt", 256);
        let _p4 = create_branch_predictor("one_bit", 256);
        let _p5 = create_branch_predictor("two_bit", 256);
        let _p6 = create_branch_predictor("unknown", 256);
        // All should create without panicking.
    }

    #[test]
    fn test_fp_unit_config() {
        let fp = FPUnitConfig {
            formats: vec!["fp32".to_string(), "fp64".to_string()],
            pipeline_depth: 4,
        };
        assert_eq!(fp.formats.len(), 2);
        assert_eq!(fp.pipeline_depth, 4);
    }
}
