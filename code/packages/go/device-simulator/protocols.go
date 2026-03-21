// Package devicesimulator implements Layer 6 of the accelerator computing stack --
// complete device simulators that model entire accelerator chips with all their
// compute units, global memory, caches, and work distributors.
//
// # What is a Device Simulator?
//
// A device simulator models a **complete accelerator** -- not just one compute
// unit, but the entire chip with all its compute units, global memory, caches,
// and the work distributor that ties them together.
//
// Think of it as the difference between simulating one factory floor (Layer 7)
// versus simulating the entire factory complex:
//
//	Layer 7 (Compute Unit):    One SM / CU / MXU -- a single factory floor
//	Layer 6 (Device):          The whole factory -- all floors + warehouse +
//	                           shipping dock + floor manager's office
//
// The device layer adds four new concepts:
//
//  1. **Global Memory (VRAM)** -- the large device-wide memory (the warehouse).
//     All compute units share it. High bandwidth but high latency (~400 cycles).
//
//  2. **L2 Cache** -- sits between compute units and global memory. Reduces the
//     average latency for frequently-accessed data.
//
//  3. **Work Distributor** -- takes kernel launches (work orders) and assigns
//     thread blocks to compute units that have available resources.
//
//  4. **Host Interface** -- the connection to the CPU. Data must be copied from
//     CPU memory to device memory before the GPU can use it (except on Apple's
//     unified memory, where it's zero-copy).
//
// # Memory Hierarchy at the Device Level
//
//	                +--------------+
//	    CPU RAM --> | Host Interface| --> PCIe / NVLink / unified
//	                +------+-------+
//	                       |
//	                +------+-------+
//	                | Global Memory |  24-80 GB, ~400 cycle latency
//	                |  (HBM/GDDR)  |  1-3 TB/s bandwidth
//	                +------+-------+
//	                       |
//	                +------+-------+
//	                |   L2 Cache   |  4-96 MB, ~200 cycle latency
//	                |  (shared)    |
//	                +--+---+---+---+
//	                   |   |   |
//	                 CU 0 CU 1 ... CU N   (each with local shared memory)
//
// # Protocol-Based Design
//
// Like the compute-unit package, we use Go interfaces. The same AcceleratorDevice
// interface works for NVIDIA GPUs, AMD GPUs, Google TPUs, Intel GPUs, and Apple ANEs.
package devicesimulator

import (
	"fmt"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	computeunit "github.com/adhithyan15/coding-adventures/code/packages/go/compute-unit"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// =========================================================================
// MemoryTransaction -- a single wide memory access after coalescing
// =========================================================================

// MemoryTransaction is a single wide memory transaction after coalescing.
//
// When 32 threads in a warp each request 4 bytes, those 128 bytes of
// requests might coalesce into a single 128-byte transaction (best case)
// or 32 separate transactions (worst case -- scattered access).
//
// Coalescing visual:
//
//	Best case (1 transaction):
//	    Thread  0  1  2  3  4  ...  31
//	    Addr   [0][4][8][12][16]...[124]
//	           +----------------------+
//	             One 128B transaction
//
//	Worst case (32 transactions):
//	    Thread  0     1      2      3
//	    Addr   [0]  [512]  [1024]  [1536]  ...
//	            |      |      |      |
//	         Trans 1 Trans 2 Trans 3 Trans 4
type MemoryTransaction struct {
	// Address is the aligned start address of the transaction.
	Address int
	// Size is the transaction size in bytes (32, 64, or 128).
	Size int
	// ThreadMask is a bitmask of which threads are served by this transaction.
	// Bit i is set if thread i's request falls in this range.
	ThreadMask uint64
}

// =========================================================================
// GlobalMemoryStats -- tracks memory access patterns and efficiency
// =========================================================================

// GlobalMemoryStats tracks memory access patterns and efficiency.
//
// Memory access patterns are the #1 performance bottleneck on GPUs.
// A kernel that achieves perfect coalescing uses 32x less bandwidth than
// one with fully scattered access. These stats tell you whether your
// memory accesses are efficient.
//
// Key metric: **coalescing_efficiency**
//
//	= total_requests / total_transactions
//	Ideal = 1.0 (every request coalesces into existing transactions)
//	Worst = 32.0 for 32-wide warps (nothing coalesces)
type GlobalMemoryStats struct {
	TotalReads           int
	TotalWrites          int
	TotalTransactions    int
	TotalRequests        int
	BytesTransferred     int
	CoalescingEfficiency float64
	PartitionConflicts   int
	HostToDeviceBytes    int
	DeviceToHostBytes    int
	HostTransferCycles   int
}

// UpdateEfficiency recalculates coalescing efficiency from current counts.
func (s *GlobalMemoryStats) UpdateEfficiency() {
	if s.TotalTransactions > 0 {
		s.CoalescingEfficiency = float64(s.TotalRequests) / float64(s.TotalTransactions)
	}
}

// =========================================================================
// KernelDescriptor -- what gets launched on the device
// =========================================================================

// KernelDescriptor describes a kernel launch (GPU) or operation (TPU/NPU).
//
// GPU-style devices (NVIDIA, AMD, Intel) receive a **program** with grid
// and block dimensions -- "run this code on this many threads."
//
// Dataflow-style devices (TPU, NPU) receive an **operation** with input
// and weight data -- "multiply these matrices" or "apply this activation."
//
// The same KernelDescriptor handles both by having fields for each style.
// GPU devices use the Program/GridDim/BlockDim fields. Dataflow devices use the
// Operation/InputData/WeightData fields.
//
// GPU Example:
//
//	kernel := KernelDescriptor{
//	    Name:      "saxpy",
//	    KernelID:  0,
//	    Program:   []gpucore.Instruction{limm(0, alpha), load(1, ...), ...},
//	    GridDim:   [3]int{256, 1, 1},     // 256 blocks
//	    BlockDim:  [3]int{256, 1, 1},     // 256 threads per block
//	}
//	// Total: 256 * 256 = 65,536 threads
//
// Dataflow Example:
//
//	kernel := KernelDescriptor{
//	    Name:       "matmul",
//	    Operation:  "matmul",
//	    InputData:  A,     // MxK matrix
//	    WeightData: B,     // KxN matrix
//	}
type KernelDescriptor struct {
	// Common fields
	Name     string
	KernelID int

	// GPU-style fields
	Program             []gpucore.Instruction
	GridDim             [3]int // (gx, gy, gz)
	BlockDim            [3]int // (bx, by, bz)
	SharedMemBytes      int
	RegistersPerThread  int

	// Dataflow-style fields (TPU/NPU)
	Operation     string
	InputData     [][]float64
	WeightData    [][]float64
	OutputAddress int
}

// DefaultKernelDescriptor creates a KernelDescriptor with sensible defaults.
func DefaultKernelDescriptor() KernelDescriptor {
	return KernelDescriptor{
		Name:               "unnamed",
		GridDim:            [3]int{1, 1, 1},
		BlockDim:           [3]int{32, 1, 1},
		RegistersPerThread: 32,
	}
}

// TotalThreads returns the total number of threads across all blocks.
func (k KernelDescriptor) TotalThreads() int {
	gx, gy, gz := k.GridDim[0], k.GridDim[1], k.GridDim[2]
	bx, by, bz := k.BlockDim[0], k.BlockDim[1], k.BlockDim[2]
	return gx * gy * gz * bx * by * bz
}

// TotalBlocks returns the total number of thread blocks in the grid.
func (k KernelDescriptor) TotalBlocks() int {
	return k.GridDim[0] * k.GridDim[1] * k.GridDim[2]
}

// ThreadsPerBlock returns the number of threads in each block.
func (k KernelDescriptor) ThreadsPerBlock() int {
	return k.BlockDim[0] * k.BlockDim[1] * k.BlockDim[2]
}

// =========================================================================
// DeviceConfig -- full device specification
// =========================================================================

// DeviceConfig is the complete device specification.
//
// Every accelerator is characterized by:
//   - How many compute units it has
//   - How much and how fast its memory is
//   - How it connects to the CPU
//   - How it distributes work
//
// By changing these parameters, the same device simulator code can model
// anything from a laptop GPU to a datacenter TPU.
//
// Memory hierarchy parameters:
//
//	Host RAM --[host_bandwidth]--> Global Memory (VRAM)
//	                                    |
//	                            [global_memory_bandwidth]
//	                                    |
//	                               L2 Cache
//	                                    |
//	                            Compute Units (shared memory)
//	                                    |
//	                               Registers
type DeviceConfig struct {
	// Identity
	Name         string
	Architecture string

	// Compute
	NumComputeUnits int
	CUConfig        interface{} // vendor-specific CU config

	// Memory hierarchy
	L2CacheSize          int
	L2CacheLatency       int
	L2CacheAssociativity int
	L2CacheLineSize      int

	GlobalMemorySize      int
	GlobalMemoryBandwidth float64
	GlobalMemoryLatency   int
	MemoryChannels        int

	// Host interface
	HostBandwidth float64
	HostLatency   int
	UnifiedMemory bool

	// Scheduling
	MaxConcurrentKernels   int
	WorkDistributionPolicy string
}

// DefaultDeviceConfig returns a DeviceConfig with sensible defaults.
func DefaultDeviceConfig() DeviceConfig {
	return DeviceConfig{
		Name:                   "Generic Accelerator",
		Architecture:           "generic",
		NumComputeUnits:        4,
		L2CacheSize:            4 * 1024 * 1024,
		L2CacheLatency:         200,
		L2CacheAssociativity:   16,
		L2CacheLineSize:        128,
		GlobalMemorySize:       16 * 1024 * 1024,
		GlobalMemoryBandwidth:  1000.0,
		GlobalMemoryLatency:    400,
		MemoryChannels:         8,
		HostBandwidth:          64.0,
		HostLatency:            1000,
		UnifiedMemory:          false,
		MaxConcurrentKernels:   1,
		WorkDistributionPolicy: "round_robin",
	}
}

// =========================================================================
// Vendor-specific configs
// =========================================================================

// ShaderEngineConfig is AMD Shader Engine -- mid-level grouping of CUs.
//
// AMD organizes CUs into Shader Engines, each sharing a geometry
// processor and rasterizer. For compute workloads, the main effect
// is that the Command Processor assigns work at the SE level first.
type ShaderEngineConfig struct {
	CUsPerEngine int
	SharedL1Size int
}

// DefaultShaderEngineConfig returns defaults for AMD Shader Engine config.
func DefaultShaderEngineConfig() ShaderEngineConfig {
	return ShaderEngineConfig{
		CUsPerEngine: 16,
		SharedL1Size: 32 * 1024,
	}
}

// AmdGPUConfig is the AMD-specific config with Shader Engine hierarchy.
type AmdGPUConfig struct {
	DeviceConfig
	NumShaderEngines   int
	SEConfig           ShaderEngineConfig
	InfinityCacheSize  int
	InfinityCacheLatency int
	NumACEs            int
}

// XeSliceConfig is Intel Xe-Slice -- mid-level grouping of Xe-Cores.
type XeSliceConfig struct {
	XeCoresPerSlice  int
	L1CachePerSlice  int
}

// DefaultXeSliceConfig returns defaults for Intel Xe-Slice config.
func DefaultXeSliceConfig() XeSliceConfig {
	return XeSliceConfig{
		XeCoresPerSlice: 4,
		L1CachePerSlice: 192 * 1024,
	}
}

// IntelGPUConfig is Intel-specific config with Xe-Slice hierarchy.
type IntelGPUConfig struct {
	DeviceConfig
	NumXeSlices int
	SliceConfig XeSliceConfig
}

// ICILink is one ICI link to another TPU chip.
//
// TPU pods use Inter-Chip Interconnect (ICI) to connect multiple
// TPU chips in a 4D torus topology. Each link provides high-bandwidth,
// low-latency communication for collective operations (all-reduce, etc.)
type ICILink struct {
	TargetChipID int
	Bandwidth    float64
	Latency      int
}

// TPUConfig is TPU-specific config with Vector/Scalar units and ICI.
type TPUConfig struct {
	DeviceConfig
	VectorUnitWidth int
	ScalarRegisters int
	TransposeUnit   bool
	ICILinks        []ICILink
}

// ANEConfig is Apple ANE-specific config with DMA and SRAM.
//
// The ANE is unique: it shares unified memory with CPU and GPU,
// eliminating the PCIe transfer bottleneck entirely. The 'copy'
// operation just remaps page tables -- zero cycles, zero bytes moved.
type ANEConfig struct {
	DeviceConfig
	SharedSRAMSize int
	SRAMBandwidth  float64
	SRAMLatency    int
	DMAChannels    int
	DMABandwidth   float64
}

// =========================================================================
// Default configs -- model real hardware
// =========================================================================

// DefaultNvidiaConfig returns an H100-like configuration (scaled down for simulation).
func DefaultNvidiaConfig() DeviceConfig {
	return DeviceConfig{
		Name:                   "NVIDIA H100",
		Architecture:           "nvidia_sm",
		NumComputeUnits:        132,
		L2CacheSize:            50 * 1024 * 1024,
		L2CacheLatency:         200,
		L2CacheAssociativity:   32,
		L2CacheLineSize:        128,
		GlobalMemorySize:       80 * 1024 * 1024,
		GlobalMemoryBandwidth:  3350.0,
		GlobalMemoryLatency:    400,
		MemoryChannels:         8,
		HostBandwidth:          64.0,
		HostLatency:            1000,
		UnifiedMemory:          false,
		MaxConcurrentKernels:   128,
		WorkDistributionPolicy: "round_robin",
	}
}

// DefaultAmdConfig returns an RX 7900 XTX-like configuration.
func DefaultAmdConfig() AmdGPUConfig {
	return AmdGPUConfig{
		DeviceConfig: DeviceConfig{
			Name:                   "AMD RX 7900 XTX",
			Architecture:           "amd_cu",
			NumComputeUnits:        96,
			L2CacheSize:            6 * 1024 * 1024,
			L2CacheLatency:         150,
			L2CacheAssociativity:   16,
			L2CacheLineSize:        128,
			GlobalMemorySize:       24 * 1024 * 1024,
			GlobalMemoryBandwidth:  960.0,
			GlobalMemoryLatency:    350,
			MemoryChannels:         6,
			HostBandwidth:          32.0,
			HostLatency:            1000,
			UnifiedMemory:          false,
			MaxConcurrentKernels:   8,
			WorkDistributionPolicy: "round_robin",
		},
		NumShaderEngines:     6,
		SEConfig:             DefaultShaderEngineConfig(),
		InfinityCacheSize:    96 * 1024 * 1024,
		InfinityCacheLatency: 50,
		NumACEs:              4,
	}
}

// DefaultTPUConfig returns a TPU v4-like configuration.
func DefaultTPUConfig() TPUConfig {
	return TPUConfig{
		DeviceConfig: DeviceConfig{
			Name:                   "Google TPU v4",
			Architecture:           "google_mxu",
			NumComputeUnits:        1,
			L2CacheSize:            0,
			L2CacheLatency:         0,
			L2CacheAssociativity:   0,
			L2CacheLineSize:        128,
			GlobalMemorySize:       32 * 1024 * 1024,
			GlobalMemoryBandwidth:  1200.0,
			GlobalMemoryLatency:    300,
			MemoryChannels:         4,
			HostBandwidth:          500.0,
			HostLatency:            500,
			UnifiedMemory:          false,
			MaxConcurrentKernels:   1,
			WorkDistributionPolicy: "sequential",
		},
		VectorUnitWidth: 128,
		ScalarRegisters: 32,
		TransposeUnit:   true,
	}
}

// DefaultIntelConfig returns an Arc A770-like configuration.
func DefaultIntelConfig() IntelGPUConfig {
	return IntelGPUConfig{
		DeviceConfig: DeviceConfig{
			Name:                   "Intel Arc A770",
			Architecture:           "intel_xe_core",
			NumComputeUnits:        32,
			L2CacheSize:            16 * 1024 * 1024,
			L2CacheLatency:         180,
			L2CacheAssociativity:   16,
			L2CacheLineSize:        128,
			GlobalMemorySize:       16 * 1024 * 1024,
			GlobalMemoryBandwidth:  512.0,
			GlobalMemoryLatency:    350,
			MemoryChannels:         4,
			HostBandwidth:          32.0,
			HostLatency:            1000,
			UnifiedMemory:          false,
			MaxConcurrentKernels:   16,
			WorkDistributionPolicy: "round_robin",
		},
		NumXeSlices: 8,
		SliceConfig: DefaultXeSliceConfig(),
	}
}

// DefaultAppleConfig returns an M3 Max ANE-like configuration.
func DefaultAppleConfig() ANEConfig {
	return ANEConfig{
		DeviceConfig: DeviceConfig{
			Name:                   "Apple M3 Max ANE",
			Architecture:           "apple_ane_core",
			NumComputeUnits:        16,
			L2CacheSize:            0,
			L2CacheLatency:         0,
			L2CacheAssociativity:   0,
			L2CacheLineSize:        128,
			GlobalMemorySize:       128 * 1024 * 1024,
			GlobalMemoryBandwidth:  200.0,
			GlobalMemoryLatency:    100,
			MemoryChannels:         8,
			HostBandwidth:          200.0,
			HostLatency:            0,
			UnifiedMemory:          true,
			MaxConcurrentKernels:   1,
			WorkDistributionPolicy: "scheduled",
		},
		SharedSRAMSize: 32 * 1024 * 1024,
		SRAMBandwidth:  1000.0,
		SRAMLatency:    5,
		DMAChannels:    4,
		DMABandwidth:   100.0,
	}
}

// =========================================================================
// DeviceTrace -- cycle-by-cycle visibility into the whole device
// =========================================================================

// DeviceTrace records one cycle of device-wide activity.
//
// At the compute unit level (Layer 7), traces show what one SM/CU is doing.
// At the device level, we need to see all compute units simultaneously, plus
// the memory system and work distributor.
//
// Key questions a DeviceTrace answers:
//   - How many compute units are busy vs idle?
//   - Is the memory system a bottleneck (high bandwidth utilization)?
//   - Is the work distributor keeping up (many pending blocks)?
//   - What's the overall device occupancy?
type DeviceTrace struct {
	Cycle      int
	DeviceName string

	// Work distribution
	DistributorActions []string
	PendingBlocks      int
	ActiveBlocks       int

	// Per-CU traces (can be empty for idle CUs)
	CUTraces []computeunit.ComputeUnitTrace

	// Memory system
	L2Hits              int
	L2Misses            int
	MemoryTransactions  int
	MemoryBandwidthUsed float64

	// Aggregate metrics
	TotalActiveWarps int
	DeviceOccupancy  float64
	FlopsThisCycle   int
}

// Format returns a human-readable summary of this cycle.
//
// Example output:
//
//	[Cycle 10] NVIDIA H100 -- 45.2% occupancy
//	  Distributor: Block 42 -> SM 7, Block 43 -> SM 12
//	  Pending: 890 blocks, Active: 1056 blocks
//	  L2: 342 hits, 12 misses (96.6% hit rate)
//	  Memory: 8 transactions, 45.2% bandwidth
//	  Active warps: 4234
func (t DeviceTrace) Format() string {
	lines := []string{
		fmt.Sprintf("[Cycle %d] %s -- %.1f%% occupancy",
			t.Cycle, t.DeviceName, t.DeviceOccupancy*100),
	}

	if len(t.DistributorActions) > 0 {
		actionsStr := strings.Join(t.DistributorActions, ", ")
		lines = append(lines, fmt.Sprintf("  Distributor: %s", actionsStr))
	}

	lines = append(lines, fmt.Sprintf("  Pending: %d blocks, Active: %d blocks",
		t.PendingBlocks, t.ActiveBlocks))

	totalL2 := t.L2Hits + t.L2Misses
	if totalL2 > 0 {
		hitRate := float64(t.L2Hits) / float64(totalL2) * 100
		lines = append(lines, fmt.Sprintf("  L2: %d hits, %d misses (%.1f%% hit rate)",
			t.L2Hits, t.L2Misses, hitRate))
	}

	lines = append(lines, fmt.Sprintf("  Memory: %d transactions, %.1f%% bandwidth",
		t.MemoryTransactions, t.MemoryBandwidthUsed*100))

	lines = append(lines, fmt.Sprintf("  Active warps: %d", t.TotalActiveWarps))

	return strings.Join(lines, "\n")
}

// =========================================================================
// DeviceStats -- aggregate metrics across the entire simulation
// =========================================================================

// DeviceStats holds device-wide aggregate statistics.
//
// These stats answer the key performance questions:
//
//  1. **Compute utilization**: Are the compute units busy or sitting idle?
//  2. **Memory bandwidth utilization**: Is the memory system saturated?
//  3. **Load imbalance**: Are some CUs doing more work than others?
//  4. **L2 effectiveness**: Is the cache helping?
type DeviceStats struct {
	// Time
	TotalCycles  int
	ActiveCycles int
	IdleCycles   int

	// Compute
	TotalFlops         int
	AchievedTFLOPS     float64
	PeakTFLOPS         float64
	ComputeUtilization float64

	// Memory
	GlobalMemoryStats          GlobalMemoryStats
	L2HitRate                  float64
	MemoryBandwidthUtilization float64

	// Work distribution
	TotalKernelsLaunched  int
	TotalBlocksDispatched int
	AvgBlocksPerCU        float64
	LoadImbalance         float64

	// Per-CU breakdown
	PerCUActiveCycles []int
	PerCUOccupancy    []float64
}

// =========================================================================
// AcceleratorDevice -- the unified device interface
// =========================================================================

// AcceleratorDevice is the interface for any accelerator device: GPU, TPU, NPU.
//
// This is the top-level interface for Layer 6. The ISA Simulator (Layer 5)
// and Runtime Simulator (Layer 4) will interact with devices through
// this interface.
//
// Despite radical differences between a GPU (thread-parallel, thousands of
// cores) and a TPU (dataflow, one large matrix unit), they share a common
// lifecycle:
//
//  1. Allocate device memory
//  2. Copy data from host to device
//  3. Launch computation
//  4. Wait for completion
//  5. Copy results back to host
type AcceleratorDevice interface {
	// Name returns the device name ("NVIDIA H100", "Apple M3 Max ANE", etc.).
	Name() string

	// Config returns the full device configuration.
	Config() DeviceConfig

	// Malloc allocates device memory. Returns device pointer (address).
	Malloc(size int) (int, error)

	// Free releases device memory allocation.
	Free(address int)

	// MemcpyHostToDevice copies from host to device. Returns cycles consumed.
	MemcpyHostToDevice(dst int, data []byte) (int, error)

	// MemcpyDeviceToHost copies from device to host. Returns (data, cycles).
	MemcpyDeviceToHost(src int, size int) ([]byte, int, error)

	// LaunchKernel submits a kernel for execution.
	LaunchKernel(kernel KernelDescriptor)

	// Step advances the entire device by one clock cycle.
	Step(edge clock.ClockEdge) DeviceTrace

	// Run runs until all kernels complete or maxCycles reached.
	Run(maxCycles int) []DeviceTrace

	// Idle returns true when all CUs are idle and no pending work remains.
	Idle() bool

	// Reset resets all state -- CUs, memory, caches, work queues.
	Reset()

	// Stats returns aggregate statistics across all compute units and memory.
	Stats() DeviceStats

	// ComputeUnits returns direct access to individual compute units.
	ComputeUnits() []computeunit.ComputeUnit

	// GlobalMemory returns access to device memory.
	GlobalMem() *SimpleGlobalMemory
}
