package computeunit

// AMDComputeUnit -- AMD Compute Unit (GCN/RDNA) simulator.
//
// # How AMD CUs Differ from NVIDIA SMs
//
// While NVIDIA and AMD GPUs look similar from the outside, their internal
// organization is quite different:
//
//	NVIDIA SM:                          AMD CU (GCN):
//	---------                           --------------
//	4 warp schedulers                   4 SIMD units (16-wide each)
//	Each issues 1 warp (32 threads)     Each runs 1 wavefront (64 lanes)
//	Total: 128 threads/cycle            Total: 64 lanes x 4 = 256 lanes/cycle
//
//	Register file: unified              Register file: per-SIMD VGPR
//	Shared memory: explicit             LDS: explicit (similar to shared mem)
//	Warp scheduling: hardware           Wavefront scheduling: hardware
//	Scalar unit: per-thread             Scalar unit: SHARED by wavefront
//
// # The Scalar Unit -- AMD's Key Innovation
//
// The scalar unit executes operations that are the SAME across all lanes:
//   - Address computation (base_addr + offset)
//   - Loop counters (i++)
//   - Branch conditions (if i < N)
//   - Constants (pi, epsilon, etc.)
//
// Instead of doing this 64 times (once per lane), AMD does it ONCE in the
// scalar unit and broadcasts the result. This saves power and register space.
//
// # Architecture Diagram
//
//	AMDComputeUnit (GCN-style)
//	+---------------------------------------------------------------+
//	|                                                               |
//	|  Wavefront Scheduler                                          |
//	|  +----------------------------------------------------------+ |
//	|  | wf0: READY  wf1: STALLED  wf2: READY  wf3: READY ...    | |
//	|  +----------------------------------------------------------+ |
//	|                                                               |
//	|  +------------------+ +------------------+                    |
//	|  | SIMD Unit 0      | | SIMD Unit 1      |                    |
//	|  | 16-wide ALU      | | 16-wide ALU      |                    |
//	|  | VGPR: 256        | | VGPR: 256        |                    |
//	|  +------------------+ +------------------+                    |
//	|  +------------------+ +------------------+                    |
//	|  | SIMD Unit 2      | | SIMD Unit 3      |                    |
//	|  +------------------+ +------------------+                    |
//	|                                                               |
//	|  +------------------+                                         |
//	|  | Scalar Unit      |  <- executes once for all lanes         |
//	|  | SGPR: 104        |  (address computation, flow control)    |
//	|  +------------------+                                         |
//	|                                                               |
//	|  Shared Resources:                                            |
//	|  +-----------------------------------------------------------+|
//	|  | LDS (Local Data Share): 64 KB                              ||
//	|  | L1 Vector Cache: 16 KB                                     ||
//	|  | L1 Scalar Cache: 16 KB                                     ||
//	|  +-----------------------------------------------------------+|
//	+---------------------------------------------------------------+

import (
	"fmt"

	fp "github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	pee "github.com/adhithyan15/coding-adventures/code/packages/go/parallel-execution-engine"
)

// =========================================================================
// AMDCUConfig -- configuration for an AMD-style Compute Unit
// =========================================================================

// AMDCUConfig holds configuration for an AMD-style Compute Unit.
//
// Real-world CU configurations:
//
//	Parameter            | GCN (Vega)   | RDNA2 (RX 6000) | RDNA3
//	---------------------+--------------+------------------+------
//	SIMD units           | 4            | 2 (per CU)       | 2
//	Wave width           | 64           | 32 (native)      | 32
//	Max wavefronts       | 40           | 32               | 32
//	VGPRs per SIMD       | 256          | 256              | 256
//	SGPRs                | 104          | 104              | 104
//	LDS size             | 64 KB        | 128 KB           | 128 KB
//	L1 vector cache      | 16 KB        | 128 KB           | 128 KB
type AMDCUConfig struct {
	NumSIMDUnits    int
	WaveWidth       int
	MaxWavefronts   int
	MaxWorkGroups   int
	Policy          SchedulingPolicy
	VGPRPerSIMD     int
	SGPRCount       int
	LDSSize         int
	L1VectorCache   int
	L1ScalarCache   int
	L1InstrCache    int
	FloatFmt        fp.FloatFormat
	ISA             gpucore.InstructionSet
	MemLatencyCycles int
}

// DefaultAMDCUConfig returns an AMDCUConfig with sensible defaults.
func DefaultAMDCUConfig() AMDCUConfig {
	return AMDCUConfig{
		NumSIMDUnits:    4,
		WaveWidth:       64,
		MaxWavefronts:   40,
		MaxWorkGroups:   16,
		Policy:          ScheduleLRR,
		VGPRPerSIMD:     256,
		SGPRCount:       104,
		LDSSize:         65536,
		L1VectorCache:   16384,
		L1ScalarCache:   16384,
		L1InstrCache:    32768,
		FloatFmt:        fp.FP32,
		ISA:             gpucore.GenericISA{},
		MemLatencyCycles: 200,
	}
}

// =========================================================================
// WavefrontSlot -- tracks one wavefront's state
// =========================================================================

// WavefrontSlot tracks one wavefront in the AMD CU's scheduler.
//
// Similar to WarpSlot in the NVIDIA SM, but for AMD wavefronts.
// Each slot tracks the wavefront's state and which SIMD unit
// it's assigned to.
type WavefrontSlot struct {
	WaveID       int
	WorkID       int
	State        WarpState
	SIMDUnit     int
	Engine       *pee.WavefrontEngine
	StallCounter int
	Age          int
	VGPRsUsed   int
}

// =========================================================================
// AMDComputeUnit -- the main CU simulator
// =========================================================================

// AMDComputeUnit is an AMD Compute Unit (GCN/RDNA) simulator.
//
// Manages wavefronts across SIMD units, with scalar unit support,
// LDS (Local Data Share), and wavefront scheduling.
//
// === Key Differences from StreamingMultiprocessor ===
//
//  1. SIMD units instead of warp schedulers: Each SIMD unit is a
//     16-wide vector ALU.
//  2. Scalar unit: Operations common to all lanes execute once.
//  3. LDS instead of shared memory: Functionally similar, but AMD's
//     LDS has different banking.
//  4. LRR scheduling: AMD typically uses Loose Round Robin instead
//     of NVIDIA's GTO.
type AMDComputeUnit struct {
	config        AMDCUConfig
	clk           *clock.Clock
	cycle         int
	lds           *SharedMemory
	ldsUsed       int
	wavefronts    []*WavefrontSlot
	nextWaveID    int
	vgprAllocated []int
	rrIndex       int
}

// NewAMDComputeUnit creates a new AMD CU simulator.
func NewAMDComputeUnit(config AMDCUConfig, clk *clock.Clock) *AMDComputeUnit {
	return &AMDComputeUnit{
		config:        config,
		clk:           clk,
		lds:           NewSharedMemory(config.LDSSize),
		vgprAllocated: make([]int, config.NumSIMDUnits),
	}
}

// --- ComputeUnit interface ---

// Name returns the compute unit name.
func (cu *AMDComputeUnit) Name() string { return "CU" }

// Arch returns AMD CU architecture.
func (cu *AMDComputeUnit) Arch() Architecture { return ArchAMDCU }

// Idle returns true if no active wavefronts remain.
func (cu *AMDComputeUnit) Idle() bool {
	if len(cu.wavefronts) == 0 {
		return true
	}
	for _, w := range cu.wavefronts {
		if w.State != WarpStateCompleted {
			return false
		}
	}
	return true
}

// Occupancy returns the current occupancy: active wavefronts / max wavefronts.
func (cu *AMDComputeUnit) Occupancy() float64 {
	if cu.config.MaxWavefronts == 0 {
		return 0.0
	}
	active := 0
	for _, w := range cu.wavefronts {
		if w.State != WarpStateCompleted {
			active++
		}
	}
	return float64(active) / float64(cu.config.MaxWavefronts)
}

// Config returns the CU configuration.
func (cu *AMDComputeUnit) Config() AMDCUConfig { return cu.config }

// LDS returns the Local Data Share instance.
func (cu *AMDComputeUnit) LDS() *SharedMemory { return cu.lds }

// WavefrontSlots returns all wavefront slots (for inspection).
func (cu *AMDComputeUnit) WavefrontSlots() []*WavefrontSlot { return cu.wavefronts }

// --- Dispatch ---

// Dispatch dispatches a work group to this CU.
//
// Decomposes the work group into wavefronts and assigns them to
// SIMD units round-robin.
func (cu *AMDComputeUnit) Dispatch(work WorkItem) error {
	numWaves := (work.ThreadCount + cu.config.WaveWidth - 1) / cu.config.WaveWidth

	currentActive := 0
	for _, w := range cu.wavefronts {
		if w.State != WarpStateCompleted {
			currentActive++
		}
	}

	if currentActive+numWaves > cu.config.MaxWavefronts {
		return &ResourceError{
			Message: fmt.Sprintf("Not enough wavefront slots: need %d, available %d",
				numWaves, cu.config.MaxWavefronts-currentActive),
		}
	}

	smemNeeded := work.SharedMemBytes
	if cu.ldsUsed+smemNeeded > cu.config.LDSSize {
		return &ResourceError{
			Message: fmt.Sprintf("Not enough LDS: need %d, available %d",
				smemNeeded, cu.config.LDSSize-cu.ldsUsed),
		}
	}

	cu.ldsUsed += smemNeeded

	for waveIdx := 0; waveIdx < numWaves; waveIdx++ {
		waveID := cu.nextWaveID
		cu.nextWaveID++

		threadStart := waveIdx * cu.config.WaveWidth
		threadEnd := threadStart + cu.config.WaveWidth
		if threadEnd > work.ThreadCount {
			threadEnd = work.ThreadCount
		}
		actualLanes := threadEnd - threadStart

		// Assign to a SIMD unit round-robin
		simdUnit := waveIdx % cu.config.NumSIMDUnits

		numVGPRs := cu.config.VGPRPerSIMD
		if numVGPRs > 256 {
			numVGPRs = 256
		}

		// Create WavefrontEngine
		engine := pee.NewWavefrontEngine(
			pee.WavefrontConfig{
				WaveWidth:   actualLanes,
				NumVGPRs:    numVGPRs,
				NumSGPRs:    cu.config.SGPRCount,
				LDSSize:     cu.config.LDSSize,
				FloatFormat: cu.config.FloatFmt,
				ISA:         cu.config.ISA,
			},
			cu.clk,
		)

		if work.Program != nil {
			engine.LoadProgram(work.Program)
		}

		// Set per-lane data
		for laneOffset := 0; laneOffset < actualLanes; laneOffset++ {
			globalTID := threadStart + laneOffset
			if regs, ok := work.PerThreadData[globalTID]; ok {
				for reg, val := range regs {
					_ = engine.SetLaneRegister(laneOffset, reg, val)
				}
			}
		}

		slot := &WavefrontSlot{
			WaveID:    waveID,
			WorkID:    work.WorkID,
			State:     WarpStateReady,
			SIMDUnit:  simdUnit,
			Engine:    engine,
			VGPRsUsed: numVGPRs,
		}
		cu.wavefronts = append(cu.wavefronts, slot)
	}

	return nil
}

// --- Execution ---

// Step advances one cycle: schedule wavefronts, execute on SIMD units.
//
// The AMD CU scheduler uses LRR (Loose Round Robin) by default:
// rotate through wavefronts, skip any that are stalled.
func (cu *AMDComputeUnit) Step(edge clock.ClockEdge) ComputeUnitTrace {
	cu.cycle++

	// Tick stall counters
	for _, slot := range cu.wavefronts {
		if slot.StallCounter > 0 {
			slot.StallCounter--
			if slot.StallCounter == 0 && slot.State == WarpStateStalledMemory {
				slot.State = WarpStateReady
			}
		}
		if slot.State != WarpStateCompleted && slot.State != WarpStateRunning {
			slot.Age++
		}
	}

	// Schedule: pick up to numSIMDUnits wavefronts (one per SIMD unit)
	engineTraces := make(map[int]pee.EngineTrace)
	var schedulerActions []string

	for simdID := 0; simdID < cu.config.NumSIMDUnits; simdID++ {
		var ready []*WavefrontSlot
		for _, w := range cu.wavefronts {
			if w.State == WarpStateReady && w.SIMDUnit == simdID {
				ready = append(ready, w)
			}
		}
		if len(ready) == 0 {
			continue
		}

		// LRR: pick oldest ready wavefront
		picked := ready[0]
		for _, w := range ready[1:] {
			if w.Age > picked.Age {
				picked = w
			}
		}

		picked.State = WarpStateRunning
		trace := picked.Engine.Step(edge)
		engineTraces[picked.WaveID] = trace

		schedulerActions = append(schedulerActions,
			fmt.Sprintf("SIMD%d: issued wave %d", simdID, picked.WaveID))
		picked.Age = 0

		// Update state after execution
		if picked.Engine.IsHalted() {
			picked.State = WarpStateCompleted
		} else if isMemoryInstruction(trace) {
			picked.State = WarpStateStalledMemory
			picked.StallCounter = cu.config.MemLatencyCycles
		} else {
			picked.State = WarpStateReady
		}
	}

	if len(schedulerActions) == 0 {
		schedulerActions = append(schedulerActions, "all wavefronts stalled or completed")
	}

	activeWaves := 0
	for _, w := range cu.wavefronts {
		if w.State != WarpStateCompleted {
			activeWaves++
		}
	}

	totalVGPRs := cu.config.VGPRPerSIMD * cu.config.NumSIMDUnits
	allocatedVGPRs := 0
	for _, v := range cu.vgprAllocated {
		allocatedVGPRs += v
	}

	occupancy := 0.0
	if cu.config.MaxWavefronts > 0 {
		occupancy = float64(activeWaves) / float64(cu.config.MaxWavefronts)
	}

	return ComputeUnitTrace{
		Cycle:             cu.cycle,
		UnitName:          cu.Name(),
		Arch:              cu.Arch(),
		SchedulerAction:   joinStrings(schedulerActions, "; "),
		ActiveWarps:       activeWaves,
		TotalWarps:        cu.config.MaxWavefronts,
		EngineTraces:      engineTraces,
		SharedMemoryUsed:  cu.ldsUsed,
		SharedMemoryTotal: cu.config.LDSSize,
		RegisterFileUsed:  allocatedVGPRs,
		RegisterFileTotal: totalVGPRs,
		Occupancy:         occupancy,
	}
}

// Run runs until all work completes or maxCycles is reached.
func (cu *AMDComputeUnit) Run(maxCycles int) []ComputeUnitTrace {
	var traces []ComputeUnitTrace
	for cycleNum := 1; cycleNum <= maxCycles; cycleNum++ {
		edge := clock.ClockEdge{
			Cycle:    cycleNum,
			Value:    1,
			IsRising: true,
		}
		trace := cu.Step(edge)
		traces = append(traces, trace)
		if cu.Idle() {
			break
		}
	}
	return traces
}

// Reset resets all state.
func (cu *AMDComputeUnit) Reset() {
	cu.wavefronts = nil
	cu.lds.Reset()
	cu.ldsUsed = 0
	cu.vgprAllocated = make([]int, cu.config.NumSIMDUnits)
	cu.nextWaveID = 0
	cu.rrIndex = 0
	cu.cycle = 0
}

// String returns a human-readable representation.
func (cu *AMDComputeUnit) String() string {
	active := 0
	for _, w := range cu.wavefronts {
		if w.State != WarpStateCompleted {
			active++
		}
	}
	return fmt.Sprintf("AMDComputeUnit(waves=%d/%d, occupancy=%.1f%%)",
		active, cu.config.MaxWavefronts, cu.Occupancy()*100)
}
