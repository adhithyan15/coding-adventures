package parallelexecutionengine

// WavefrontEngine -- SIMD parallel execution (AMD GCN/RDNA style).
//
// # What is a Wavefront?
//
// AMD calls their parallel execution unit a "wavefront." It's 64 lanes on GCN
// (Graphics Core Next) or 32 lanes on RDNA (Radeon DNA). A wavefront is
// fundamentally different from an NVIDIA warp:
//
//	NVIDIA Warp (SIMT):                AMD Wavefront (SIMD):
//	+--------------------------+       +--------------------------+
//	| 32 threads               |       | 32 lanes                 |
//	| Each has its own regs    |       | ONE vector register file  |
//	| Logically own PC         |       | ONE program counter       |
//	| HW manages divergence    |       | Explicit EXEC mask        |
//	+--------------------------+       +--------------------------+
//
// The critical architectural difference:
//
//	SIMT (NVIDIA): "32 independent threads that HAPPEN to run together"
//	SIMD (AMD):    "1 instruction that operates on a 32-wide vector"
//
// # AMD's Two Register Files
//
// AMD wavefronts have TWO types of registers:
//
//	Vector GPRs (VGPRs):              Scalar GPRs (SGPRs):
//	+------------------------+        +------------------------+
//	| v0: [l0][l1]...[l31]  |        | s0:  42.0              |
//	| v1: [l0][l1]...[l31]  |        | s1:  3.14              |
//	| ...                    |        | ...                    |
//	+------------------------+        +------------------------+
//	One value PER LANE                One value for ALL LANES
//
// # The EXEC Mask
//
// AMD uses a register called EXEC to control which lanes execute each
// instruction. Unlike NVIDIA's hardware-managed divergence, the EXEC mask
// is explicitly set by instructions.
//
// # Simplification for Our Simulator
//
// For educational clarity, we use GPUCore instances per lane internally
// (just like WarpEngine), but expose the AMD-style interface externally:
// vector registers, scalar registers, and explicit EXEC mask.

import (
	"fmt"

	fp "github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic"
	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// =========================================================================
// WavefrontConfig -- configuration for an AMD-style SIMD wavefront
// =========================================================================

// WavefrontConfig holds the configuration for an AMD-style SIMD wavefront engine.
//
// Real-world reference values:
//
//	Architecture | Wave Width | VGPRs | SGPRs | LDS
//	-------------+------------+-------+-------+---------
//	AMD GCN      | 64         | 256   | 104   | 64 KB
//	AMD RDNA     | 32         | 256   | 104   | 64 KB
//	Our default  | 32         | 256   | 104   | 64 KB
type WavefrontConfig struct {
	WaveWidth   int
	NumVGPRs    int
	NumSGPRs    int
	LDSSize     int
	FloatFormat fp.FloatFormat
	ISA         gpucore.InstructionSet
}

// DefaultWavefrontConfig returns a WavefrontConfig with sensible defaults.
func DefaultWavefrontConfig() WavefrontConfig {
	return WavefrontConfig{
		WaveWidth:   32,
		NumVGPRs:    256,
		NumSGPRs:    104,
		LDSSize:     65536,
		FloatFormat: fp.FP32,
		ISA:         gpucore.GenericISA{},
	}
}

// =========================================================================
// VectorRegisterFile -- one value per lane per register
// =========================================================================

// VectorRegisterFile is an AMD-style vector register file:
// NumVGPRs registers x WaveWidth lanes.
//
// Each "register" is actually a vector of WaveWidth values. When you
// write to v3[lane 5], you're writing to one slot in a 2D array:
//
//	+--------------------------------------------+
//	|         Lane 0   Lane 1   Lane 2  ...      |
//	| v0:    [ 1.0  ] [ 2.0  ] [ 3.0  ]  ...    |
//	| v1:    [ 0.5  ] [ 0.5  ] [ 0.5  ]  ...    |
//	| v2:    [ 0.0  ] [ 0.0  ] [ 0.0  ]  ...    |
//	| ...                                        |
//	+--------------------------------------------+
type VectorRegisterFile struct {
	NumVGPRs  int
	WaveWidth int
	Fmt       fp.FloatFormat
	data      [][]fp.FloatBits // data[vreg][lane]
}

// NewVectorRegisterFile creates a new vector register file initialized to zero.
func NewVectorRegisterFile(numVGPRs, waveWidth int, fmt fp.FloatFormat) *VectorRegisterFile {
	data := make([][]fp.FloatBits, numVGPRs)
	zero := fp.FloatToBits(0.0, fmt)
	for i := range data {
		row := make([]fp.FloatBits, waveWidth)
		for j := range row {
			row[j] = zero
		}
		data[i] = row
	}
	return &VectorRegisterFile{
		NumVGPRs:  numVGPRs,
		WaveWidth: waveWidth,
		Fmt:       fmt,
		data:      data,
	}
}

// Read reads one lane of a vector register as a Go float64.
func (v *VectorRegisterFile) Read(vreg, lane int) float64 {
	return fp.BitsToFloat(v.data[vreg][lane])
}

// Write writes a Go float64 to one lane of a vector register.
func (v *VectorRegisterFile) Write(vreg, lane int, value float64) {
	v.data[vreg][lane] = fp.FloatToBits(value, v.Fmt)
}

// ReadAllLanes reads all lanes of a vector register as float64 values.
func (v *VectorRegisterFile) ReadAllLanes(vreg int) []float64 {
	result := make([]float64, v.WaveWidth)
	for lane := 0; lane < v.WaveWidth; lane++ {
		result[lane] = fp.BitsToFloat(v.data[vreg][lane])
	}
	return result
}

// =========================================================================
// ScalarRegisterFile -- one value shared across all lanes
// =========================================================================

// ScalarRegisterFile is an AMD-style scalar register file: NumSGPRs
// single-value registers.
//
// Scalar registers hold values that are the SAME for all lanes:
// constants, loop counters, memory base addresses.
type ScalarRegisterFile struct {
	NumSGPRs int
	Fmt      fp.FloatFormat
	data     []fp.FloatBits
}

// NewScalarRegisterFile creates a new scalar register file initialized to zero.
func NewScalarRegisterFile(numSGPRs int, fmt fp.FloatFormat) *ScalarRegisterFile {
	data := make([]fp.FloatBits, numSGPRs)
	zero := fp.FloatToBits(0.0, fmt)
	for i := range data {
		data[i] = zero
	}
	return &ScalarRegisterFile{
		NumSGPRs: numSGPRs,
		Fmt:      fmt,
		data:     data,
	}
}

// Read reads a scalar register as a Go float64.
func (s *ScalarRegisterFile) Read(sreg int) float64 {
	return fp.BitsToFloat(s.data[sreg])
}

// Write writes a Go float64 to a scalar register.
func (s *ScalarRegisterFile) Write(sreg int, value float64) {
	s.data[sreg] = fp.FloatToBits(value, s.Fmt)
}

// =========================================================================
// WavefrontEngine -- the SIMD parallel execution engine
// =========================================================================

// WavefrontEngine is a SIMD wavefront execution engine (AMD GCN/RDNA style).
//
// One instruction stream, one wide vector ALU, explicit EXEC mask.
// Internally uses GPUCore per lane for instruction execution, but
// exposes the AMD-style vector/scalar register interface.
//
// # Key Differences from WarpEngine
//
//  1. ONE program counter (not per-thread PCs).
//  2. Vector registers are a 2D array (vreg x lane), not per-thread.
//  3. Scalar registers are shared across all lanes.
//  4. EXEC mask is explicitly controlled, not hardware-managed.
//  5. No divergence stack -- mask management is programmer/compiler's job.
type WavefrontEngine struct {
	config   WavefrontConfig
	clk      *clock.Clock
	cycle    int
	program  []gpucore.Instruction
	execMask []bool
	VRF      *VectorRegisterFile
	SRF      *ScalarRegisterFile
	lanes    []*gpucore.GPUCore
	halted   bool
}

// NewWavefrontEngine creates a new AMD-style SIMD wavefront engine.
func NewWavefrontEngine(config WavefrontConfig, clk *clock.Clock) *WavefrontEngine {
	lanes := make([]*gpucore.GPUCore, config.WaveWidth)
	memPerLane := config.LDSSize
	if config.WaveWidth > 0 {
		memPerLane = config.LDSSize / config.WaveWidth
	}
	for i := 0; i < config.WaveWidth; i++ {
		lanes[i] = gpucore.NewGPUCore(
			gpucore.WithISA(config.ISA),
			gpucore.WithFormat(config.FloatFormat),
			gpucore.WithNumRegisters(config.NumVGPRs),
			gpucore.WithMemorySize(memPerLane),
		)
	}

	execMask := make([]bool, config.WaveWidth)
	for i := range execMask {
		execMask[i] = true
	}

	return &WavefrontEngine{
		config:   config,
		clk:      clk,
		execMask: execMask,
		VRF:      NewVectorRegisterFile(config.NumVGPRs, config.WaveWidth, config.FloatFormat),
		SRF:      NewScalarRegisterFile(config.NumSGPRs, config.FloatFormat),
		lanes:    lanes,
	}
}

// --- Interface methods ---

// Name returns the engine name for traces.
func (w *WavefrontEngine) Name() string { return "WavefrontEngine" }

// Width returns the number of SIMD lanes.
func (w *WavefrontEngine) Width() int { return w.config.WaveWidth }

// ExecutionModel returns SIMD.
func (w *WavefrontEngine) ExecutionModel() ExecutionModel { return SIMD }

// IsHalted returns true if the wavefront has halted.
func (w *WavefrontEngine) IsHalted() bool { return w.halted }

// ExecMask returns a copy of the current EXEC mask.
func (w *WavefrontEngine) ExecMask() []bool {
	mask := make([]bool, len(w.execMask))
	copy(mask, w.execMask)
	return mask
}

// Config returns the configuration this engine was created with.
func (w *WavefrontEngine) Config() WavefrontConfig { return w.config }

// --- Program loading ---

// LoadProgram loads a program into the wavefront.
//
// The same program is loaded into all lane cores. Unlike SIMT where
// each thread can (logically) have a different PC, the wavefront has
// ONE shared PC for all lanes.
func (w *WavefrontEngine) LoadProgram(program []gpucore.Instruction) {
	w.program = make([]gpucore.Instruction, len(program))
	copy(w.program, program)
	for _, lane := range w.lanes {
		lane.LoadProgram(w.program)
	}
	for i := range w.execMask {
		w.execMask[i] = true
	}
	w.halted = false
	w.cycle = 0
}

// --- Register setup ---

// SetLaneRegister sets a per-lane vector register value.
//
// This writes to both the VRF (our AMD-style register file) and
// the internal GPUCore for that lane (for execution).
//
// Returns an error if the lane is out of range.
func (w *WavefrontEngine) SetLaneRegister(lane, vreg int, value float64) error {
	if lane < 0 || lane >= w.config.WaveWidth {
		return fmt.Errorf("lane %d out of range [0, %d)", lane, w.config.WaveWidth)
	}
	w.VRF.Write(vreg, lane, value)
	return w.lanes[lane].Registers.WriteFloat(vreg, value)
}

// SetScalarRegister sets a scalar register value (shared across all lanes).
//
// Returns an error if the scalar register is out of range.
func (w *WavefrontEngine) SetScalarRegister(sreg int, value float64) error {
	if sreg < 0 || sreg >= w.config.NumSGPRs {
		return fmt.Errorf("scalar register %d out of range [0, %d)", sreg, w.config.NumSGPRs)
	}
	w.SRF.Write(sreg, value)
	return nil
}

// SetExecMask explicitly sets the EXEC mask.
//
// Returns an error if the mask length doesn't match the wave width.
func (w *WavefrontEngine) SetExecMask(mask []bool) error {
	if len(mask) != w.config.WaveWidth {
		return fmt.Errorf("mask length %d != wave_width %d", len(mask), w.config.WaveWidth)
	}
	copy(w.execMask, mask)
	return nil
}

// --- Execution ---

// Step executes one cycle: issue one instruction to all active lanes.
//
// Unlike SIMT, ALL lanes share the same PC. The EXEC mask determines
// which lanes actually execute. Masked-off lanes still advance their PC
// to stay in sync with the rest of the wavefront.
func (w *WavefrontEngine) Step(edge clock.ClockEdge) EngineTrace {
	w.cycle++

	if w.halted {
		return w.makeHaltedTrace()
	}

	maskBefore := make([]bool, len(w.execMask))
	copy(maskBefore, w.execMask)

	// Execute on all lanes. Active lanes execute normally,
	// masked-off lanes still step to keep PCs in sync but results are discarded.
	unitTraces := make(map[int]string)
	for laneID := 0; laneID < w.config.WaveWidth; laneID++ {
		laneCore := w.lanes[laneID]
		if w.execMask[laneID] && !laneCore.Halted() {
			trace, err := laneCore.Step()
			if err != nil {
				unitTraces[laneID] = "(error)"
			} else if trace.Halted {
				unitTraces[laneID] = "HALTED"
			} else {
				unitTraces[laneID] = trace.Description
			}
		} else if laneCore.Halted() {
			unitTraces[laneID] = "(halted)"
		} else {
			// Masked-off lane: step to keep PC in sync but discard result.
			if !laneCore.Halted() {
				_, err := laneCore.Step()
				if err != nil {
					unitTraces[laneID] = "(masked -- error)"
				} else {
					unitTraces[laneID] = "(masked -- result discarded)"
				}
			} else {
				unitTraces[laneID] = "(halted)"
			}
		}
	}

	// Sync VRF with internal core registers for active lanes.
	syncRegs := 32
	if w.config.NumVGPRs < syncRegs {
		syncRegs = w.config.NumVGPRs
	}
	for laneID := 0; laneID < w.config.WaveWidth; laneID++ {
		if w.execMask[laneID] {
			for vreg := 0; vreg < syncRegs; vreg++ {
				val, _ := w.lanes[laneID].Registers.ReadFloat(vreg)
				w.VRF.Write(vreg, laneID, val)
			}
		}
	}

	// Check if all lanes halted.
	allDone := true
	for _, lane := range w.lanes {
		if !lane.Halted() {
			allDone = false
			break
		}
	}
	if allDone {
		w.halted = true
	}

	// Count active lanes.
	activeCount := 0
	for i := 0; i < w.config.WaveWidth; i++ {
		if w.execMask[i] && !w.lanes[i].Halted() {
			activeCount++
		}
	}
	total := w.config.WaveWidth

	// Build description from first active lane's trace.
	desc := "no active lanes"
	for i := 0; i < w.config.WaveWidth; i++ {
		if tr, ok := unitTraces[i]; ok {
			if tr != "(masked -- result discarded)" && tr != "(halted)" &&
				tr != "(error)" && tr != "(masked -- error)" && tr != "HALTED" {
				desc = tr
				break
			}
		}
	}

	currentMask := make([]bool, w.config.WaveWidth)
	for i := 0; i < w.config.WaveWidth; i++ {
		currentMask[i] = w.execMask[i] && !w.lanes[i].Halted()
	}

	utilization := 0.0
	if total > 0 {
		utilization = float64(activeCount) / float64(total)
	}

	return EngineTrace{
		Cycle:       w.cycle,
		EngineName:  w.Name(),
		Model:       w.ExecutionModel(),
		Description: fmt.Sprintf("%s -- %d/%d lanes active", desc, activeCount, total),
		UnitTraces:  unitTraces,
		ActiveMask:  currentMask,
		ActiveCount: activeCount,
		TotalCount:  total,
		Utilization: utilization,
		Divergence: &DivergenceInfo{
			ActiveMaskBefore: maskBefore,
			ActiveMaskAfter:  w.ExecMask(),
			ReconvergencePC:  -1,
			DivergenceDepth:  0,
		},
	}
}

// Run executes until all lanes halt or maxCycles is reached.
func (w *WavefrontEngine) Run(maxCycles int) ([]EngineTrace, error) {
	var traces []EngineTrace
	for cycleNum := 1; cycleNum <= maxCycles; cycleNum++ {
		edge := clock.ClockEdge{
			Cycle:    cycleNum,
			Value:    1,
			IsRising: true,
		}
		trace := w.Step(edge)
		traces = append(traces, trace)
		if w.halted {
			return traces, nil
		}
	}
	return traces, fmt.Errorf("WavefrontEngine: max_cycles (%d) reached", maxCycles)
}

// makeHaltedTrace produces a trace for when all lanes are halted.
func (w *WavefrontEngine) makeHaltedTrace() EngineTrace {
	unitTraces := make(map[int]string)
	for i := 0; i < w.config.WaveWidth; i++ {
		unitTraces[i] = "(halted)"
	}
	return EngineTrace{
		Cycle:       w.cycle,
		EngineName:  w.Name(),
		Model:       w.ExecutionModel(),
		Description: "All lanes halted",
		UnitTraces:  unitTraces,
		ActiveMask:  make([]bool, w.config.WaveWidth),
		ActiveCount: 0,
		TotalCount:  w.config.WaveWidth,
		Utilization: 0.0,
	}
}

// Reset resets the engine to its initial state.
func (w *WavefrontEngine) Reset() {
	for _, lane := range w.lanes {
		lane.Reset()
		if len(w.program) > 0 {
			lane.LoadProgram(w.program)
		}
	}
	for i := range w.execMask {
		w.execMask[i] = true
	}
	w.halted = false
	w.cycle = 0
	w.VRF = NewVectorRegisterFile(w.config.NumVGPRs, w.config.WaveWidth, w.config.FloatFormat)
	w.SRF = NewScalarRegisterFile(w.config.NumSGPRs, w.config.FloatFormat)
}

// String returns a human-readable representation of the engine.
func (w *WavefrontEngine) String() string {
	active := 0
	for _, m := range w.execMask {
		if m {
			active++
		}
	}
	return fmt.Sprintf("WavefrontEngine(width=%d, active_lanes=%d, halted=%t)",
		w.config.WaveWidth, active, w.halted)
}
