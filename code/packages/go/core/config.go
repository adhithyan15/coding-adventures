// Package core integrates all D-series micro-architectural components into a
// complete processor core.
//
// # The Core: a Motherboard for Micro-Architecture
//
// A processor core is not a single piece of hardware. It is a composition of
// many sub-components, each independently designed and tested:
//
//   - Pipeline (D04): moves instructions through stages (IF, ID, EX, MEM, WB)
//   - Branch Predictor (D02): guesses which way branches will go
//   - Hazard Detection (D03): detects data, control, and structural hazards
//   - Cache Hierarchy (D01): L1I, L1D, optional L2 for fast memory access
//   - Register File: fast storage for operands and results
//   - Clock: drives everything in lockstep
//
// The Core itself defines no new micro-architectural behavior. It wires the
// parts together, like a motherboard connects CPU, RAM, and peripherals.
// The same Core can run ARM, RISC-V, or any custom ISA -- the ISA decoder
// is injected from outside.
//
// # Configuration
//
// Every parameter that a real CPU architect would tune is exposed in
// CoreConfig. Change the branch predictor and you get different accuracy.
// Double the L1 cache and you get fewer misses. Deepen the pipeline and
// you get higher clock speeds but worse misprediction penalties.
//
// # Multi-Core
//
// MultiCoreCPU connects multiple cores to a shared L3 cache, memory
// controller, and interrupt controller -- modeling a modern multi-core chip.
package core

import (
	branchpredictor "github.com/adhithyan15/coding-adventures/code/packages/go/branch-predictor"
	"github.com/adhithyan15/coding-adventures/code/packages/go/cache"
	cpupipeline "github.com/adhithyan15/coding-adventures/code/packages/go/cpu-pipeline"
)

// =========================================================================
// RegisterFileConfig -- configuration for the register file
// =========================================================================

// RegisterFileConfig holds the parameters for the general-purpose register file.
//
// Real-world register file sizes:
//
//	MIPS:     32 registers, 32-bit  (R0 hardwired to zero)
//	ARMv8:    31 registers, 64-bit  (X0-X30, no zero register)
//	RISC-V:   32 registers, 32/64-bit (x0 hardwired to zero)
//	x86-64:   16 registers, 64-bit  (RAX, RBX, ..., R15)
//
// The zero_register convention (RISC-V, MIPS) simplifies instruction encoding:
// any instruction can discard its result by writing to R0, and any instruction
// can use "zero" as an operand without a special immediate encoding.
type RegisterFileConfig struct {
	// Count is the number of general-purpose registers.
	// Typical values: 16 (ARM Thumb, x86), 32 (MIPS, RISC-V, ARMv8).
	Count int

	// Width is the bit width of each register: 32 or 64.
	Width int

	// ZeroRegister controls whether register 0 is hardwired to zero.
	// RISC-V and MIPS: true. ARM and x86: false.
	// When true, writes to R0 are silently ignored and reads always return 0.
	ZeroRegister bool
}

// DefaultRegisterFileConfig returns a sensible default: 16 registers, 32-bit,
// with R0 hardwired to zero (RISC-V convention).
func DefaultRegisterFileConfig() RegisterFileConfig {
	result, _ := StartNew[RegisterFileConfig]("core.DefaultRegisterFileConfig", RegisterFileConfig{},
		func(op *Operation[RegisterFileConfig], rf *ResultFactory[RegisterFileConfig]) *OperationResult[RegisterFileConfig] {
			return rf.Generate(true, false, RegisterFileConfig{
				Count:        16,
				Width:        32,
				ZeroRegister: true,
			})
		}).GetResult()
	return result
}

// =========================================================================
// FPUnitConfig -- configuration for the floating-point unit
// =========================================================================

// FPUnitConfig configures the optional floating-point unit.
//
// Not all cores have an FP unit. Microcontrollers (ARM Cortex-M0) and
// efficiency cores often omit it to save area and power. When FPUnitConfig
// is nil in CoreConfig, the core has no floating-point support.
type FPUnitConfig struct {
	// Formats lists supported FP formats: "fp16", "fp32", "fp64".
	Formats []string

	// PipelineDepth is how many cycles an FP operation takes.
	// Typical: 3-5 for add/multiply, 10-20 for divide.
	PipelineDepth int
}

// =========================================================================
// CoreConfig -- complete configuration for a processor core
// =========================================================================

// CoreConfig holds every tunable parameter for a processor core.
//
// This is the "spec sheet" for the core. A CPU architect decides these
// values based on the target workload, power budget, and die area.
//
// Changing any parameter affects measurable performance:
//
//	Deeper pipeline         -> higher clock speed, worse misprediction penalty
//	Better branch predictor -> fewer pipeline flushes
//	Larger L1 cache         -> fewer cache misses
//	More registers          -> fewer spills to memory
//	Forwarding enabled      -> fewer stall cycles
type CoreConfig struct {
	// Name is a human-readable identifier for this configuration.
	// Examples: "Simple", "CortexA78Like", "AppleM4Like".
	Name string

	// --- Pipeline ---

	// Pipeline defines the pipeline stages. Defaults to classic 5-stage.
	Pipeline cpupipeline.PipelineConfig

	// --- Branch Prediction ---

	// BranchPredictorType selects the predictor algorithm:
	//   "static_always_taken"     -- always predicts taken
	//   "static_always_not_taken" -- always predicts not taken
	//   "static_btfnt"            -- backward-taken, forward-not-taken
	//   "one_bit"                 -- 1-bit dynamic predictor
	//   "two_bit"                 -- 2-bit saturating counter (default)
	BranchPredictorType string

	// BranchPredictorSize is the number of entries in the prediction table.
	// Only used for dynamic predictors (one_bit, two_bit). Typical: 256-4096.
	BranchPredictorSize int

	// BTBSize is the number of entries in the Branch Target Buffer.
	// The BTB caches WHERE branches go (target addresses).
	BTBSize int

	// --- Hazard Handling ---

	// HazardDetection enables the hazard detection unit.
	// When false, the pipeline assumes no hazards (for testing).
	HazardDetection bool

	// Forwarding enables data forwarding (bypassing) paths.
	// When true, the EX and MEM stages can forward results to earlier stages,
	// avoiding stalls for many RAW hazards.
	Forwarding bool

	// --- Register File ---

	// RegisterFile configures the general-purpose register file.
	// If nil, defaults to 16 registers, 32-bit, zero register enabled.
	RegisterFile *RegisterFileConfig

	// --- Floating Point ---

	// FPUnit configures the floating-point unit. nil = no FP support.
	FPUnit *FPUnitConfig

	// --- Cache Hierarchy ---

	// L1ICacheConfig configures the L1 instruction cache.
	// If nil, a default 4KB direct-mapped cache is used.
	L1ICache *cache.CacheConfig

	// L1DCacheConfig configures the L1 data cache.
	// If nil, a default 4KB direct-mapped cache is used.
	L1DCache *cache.CacheConfig

	// L2Cache configures the unified L2 cache. nil = no L2.
	L2Cache *cache.CacheConfig

	// --- Memory ---

	// MemorySize is the size of main memory in bytes. Default: 65536 (64KB).
	MemorySize int

	// MemoryLatency is the access latency for main memory in cycles.
	// Real DRAM: 50-100+ cycles. Default: 100.
	MemoryLatency int
}

// DefaultCoreConfig returns a minimal, sensible configuration for testing.
//
// This is the "teaching core" -- a 5-stage pipeline with static prediction,
// small caches, and 16 registers. Equivalent to a 1980s RISC microprocessor.
func DefaultCoreConfig() CoreConfig {
	result, _ := StartNew[CoreConfig]("core.DefaultCoreConfig", CoreConfig{},
		func(op *Operation[CoreConfig], rf *ResultFactory[CoreConfig]) *OperationResult[CoreConfig] {
			return rf.Generate(true, false, CoreConfig{
				Name:                "Default",
				Pipeline:            cpupipeline.Classic5Stage(),
				BranchPredictorType: "static_always_not_taken",
				BranchPredictorSize: 256,
				BTBSize:             64,
				HazardDetection:     true,
				Forwarding:          true,
				RegisterFile:        nil,
				FPUnit:              nil,
				L1ICache:            nil,
				L1DCache:            nil,
				L2Cache:             nil,
				MemorySize:          65536,
				MemoryLatency:       100,
			})
		}).GetResult()
	return result
}

// =========================================================================
// Preset Configurations -- famous real-world cores approximated
// =========================================================================

// SimpleConfig returns a minimal teaching core.
//
// Inspired by the MIPS R2000 (1985):
//   - 5-stage pipeline (IF, ID, EX, MEM, WB)
//   - Static predictor (always not taken)
//   - 4KB direct-mapped L1I and L1D caches
//   - No L2 cache
//   - 16 registers, 32-bit
//   - No floating point
//
// Expected IPC: ~0.7-0.9 on simple programs.
func SimpleConfig() CoreConfig {
	result, _ := StartNew[CoreConfig]("core.SimpleConfig", CoreConfig{},
		func(op *Operation[CoreConfig], rf *ResultFactory[CoreConfig]) *OperationResult[CoreConfig] {
			l1i := cache.CacheConfig{
				Name: "L1I", TotalSize: 4096, LineSize: 64,
				Associativity: 1, AccessLatency: 1, WritePolicy: "write-back",
			}
			l1d := cache.CacheConfig{
				Name: "L1D", TotalSize: 4096, LineSize: 64,
				Associativity: 1, AccessLatency: 1, WritePolicy: "write-back",
			}
			regCfg := RegisterFileConfig{Count: 16, Width: 32, ZeroRegister: true}
			return rf.Generate(true, false, CoreConfig{
				Name:                "Simple",
				Pipeline:            cpupipeline.Classic5Stage(),
				BranchPredictorType: "static_always_not_taken",
				BranchPredictorSize: 256,
				BTBSize:             64,
				HazardDetection:     true,
				Forwarding:          true,
				RegisterFile:        &regCfg,
				FPUnit:              nil,
				L1ICache:            &l1i,
				L1DCache:            &l1d,
				L2Cache:             nil,
				MemorySize:          65536,
				MemoryLatency:       100,
			})
		}).GetResult()
	return result
}

// CortexA78LikeConfig approximates the ARM Cortex-A78 performance core.
//
// The Cortex-A78 (2020) is used in Snapdragon 888 and Dimensity 9000:
//   - 13-stage pipeline (deep for high frequency)
//   - 2-bit predictor with 4096 entries (simplified vs real TAGE)
//   - 64KB 4-way L1I and L1D
//   - 256KB 8-way L2
//   - 31 registers, 64-bit (ARMv8)
//   - FP32 and FP64 support
//
// Expected IPC: ~0.85-0.95 (our model is in-order; real A78 is out-of-order).
func CortexA78LikeConfig() CoreConfig {
	result, _ := StartNew[CoreConfig]("core.CortexA78LikeConfig", CoreConfig{},
		func(op *Operation[CoreConfig], rf *ResultFactory[CoreConfig]) *OperationResult[CoreConfig] {
			l1i := cache.CacheConfig{
				Name: "L1I", TotalSize: 65536, LineSize: 64,
				Associativity: 4, AccessLatency: 1, WritePolicy: "write-back",
			}
			l1d := cache.CacheConfig{
				Name: "L1D", TotalSize: 65536, LineSize: 64,
				Associativity: 4, AccessLatency: 1, WritePolicy: "write-back",
			}
			l2 := cache.CacheConfig{
				Name: "L2", TotalSize: 262144, LineSize: 64,
				Associativity: 8, AccessLatency: 12, WritePolicy: "write-back",
			}
			regCfg := RegisterFileConfig{Count: 31, Width: 64, ZeroRegister: false}
			fpCfg := FPUnitConfig{Formats: []string{"fp32", "fp64"}, PipelineDepth: 4}
			return rf.Generate(true, false, CoreConfig{
				Name:                "CortexA78Like",
				Pipeline:            cpupipeline.Deep13Stage(),
				BranchPredictorType: "two_bit",
				BranchPredictorSize: 4096,
				BTBSize:             1024,
				HazardDetection:     true,
				Forwarding:          true,
				RegisterFile:        &regCfg,
				FPUnit:              &fpCfg,
				L1ICache:            &l1i,
				L1DCache:            &l1d,
				L2Cache:             &l2,
				MemorySize:          1048576,
				MemoryLatency:       100,
			})
		}).GetResult()
	return result
}

// =========================================================================
// MultiCoreConfig -- configuration for a multi-core processor
// =========================================================================

// MultiCoreConfig holds the configuration for a multi-core CPU.
//
// In a multi-core system, each core has its own L1 and L2 caches but
// shares an L3 cache and main memory. The memory controller serializes
// requests from multiple cores.
//
// Real-world multi-core counts:
//
//	Raspberry Pi 4:     4 cores (Cortex-A72)
//	Apple M4:           4P + 6E = 10 cores
//	AMD Ryzen 9 7950X:  16 cores
//	Server chips:       64-128 cores
type MultiCoreConfig struct {
	// NumCores is the number of processor cores.
	NumCores int

	// CoreConfig is the configuration shared by all cores.
	// (Heterogeneous multi-core is a future extension.)
	CoreConfig CoreConfig

	// L3Cache configures the shared L3 cache. nil = no L3.
	L3Cache *cache.CacheConfig

	// MemorySize is the total shared memory in bytes.
	MemorySize int

	// MemoryLatency is the DRAM access latency in cycles.
	MemoryLatency int
}

// DefaultMultiCoreConfig returns a 2-core configuration for testing.
func DefaultMultiCoreConfig() MultiCoreConfig {
	result, _ := StartNew[MultiCoreConfig]("core.DefaultMultiCoreConfig", MultiCoreConfig{},
		func(op *Operation[MultiCoreConfig], rf *ResultFactory[MultiCoreConfig]) *OperationResult[MultiCoreConfig] {
			return rf.Generate(true, false, MultiCoreConfig{
				NumCores:      2,
				CoreConfig:    SimpleConfig(),
				L3Cache:       nil,
				MemorySize:    1048576,
				MemoryLatency: 100,
			})
		}).GetResult()
	return result
}

// =========================================================================
// Helper: create branch predictor from config
// =========================================================================

// createBranchPredictor builds a BranchPredictor from the config strings.
//
// This factory function decouples the config (which uses strings) from the
// concrete predictor types. The Core calls this once during construction.
func createBranchPredictor(typ string, size int) branchpredictor.BranchPredictor {
	switch typ {
	case "static_always_taken":
		return branchpredictor.NewAlwaysTakenPredictor()
	case "static_always_not_taken":
		return branchpredictor.NewAlwaysNotTakenPredictor()
	case "static_btfnt":
		return branchpredictor.NewBTFNTPredictor()
	case "one_bit":
		return branchpredictor.NewOneBitPredictor(size)
	case "two_bit":
		return branchpredictor.NewTwoBitPredictor(size, branchpredictor.WeaklyNotTaken)
	default:
		return branchpredictor.NewAlwaysNotTakenPredictor()
	}
}
