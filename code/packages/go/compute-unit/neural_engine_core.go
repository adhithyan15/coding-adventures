package computeunit

// NeuralEngineCore -- Apple ANE Core simulator.
//
// # What is the Apple Neural Engine?
//
// Apple's Neural Engine (ANE) is a dedicated neural network accelerator
// found in every Apple chip since the A11 Bionic (2017). It's designed
// for one thing: fast, power-efficient neural network inference.
//
// The ANE is the simplest compute unit in our family -- and that simplicity
// is its strength. By removing hardware schedulers, branch predictors, and
// general-purpose control logic, Apple can dedicate nearly all transistors
// to MAC (multiply-accumulate) units and on-chip memory.
//
// # How ANE Differs from GPUs
//
//	GPU (NVIDIA/AMD):                   ANE (Apple):
//	+----------------------------+     +----------------------------+
//	| Hardware scheduler         |     | NO hardware scheduler      |
//	| Runtime decisions          |     | All decisions at compile    |
//	| Branch prediction          |     | NO branches                |
//	| Dynamic register alloc     |     | Static buffer plan         |
//	| Flexible but complex       |     | Simple but rigid           |
//	| ~5 W per SM                |     | ~1 W per core              |
//	+----------------------------+     +----------------------------+
//
// # Architecture
//
// Each ANE Core has:
//   - MAC array: 16 multiply-accumulate units (our default)
//   - DMA engine: transfers data between main memory and on-chip SRAM
//   - On-chip SRAM: 4 MB (fast, low-power local storage)
//   - Activation pipeline: hardware for ReLU, sigmoid, etc.
//   - Buffers: input, weight, and output buffers
//
//	NeuralEngineCore
//	+---------------------------------------------------------------+
//	|  DMA Engine                                                   |
//	|  +----------------------------------------------------------+ |
//	|  | Transfers data between main memory and on-chip SRAM       | |
//	|  | Bandwidth: 10 elements per cycle                          | |
//	|  +----------------------------------------------------------+ |
//	|                    |                                          |
//	|                    v                                          |
//	|  +------------------+ +------------------+                    |
//	|  | Input Buffer     | | Weight Buffer    |                    |
//	|  | 128 KB           | | 512 KB           |                    |
//	|  +--------+---------+ +--------+---------+                    |
//	|           |                    |                              |
//	|           v                    v                              |
//	|  +---------------------------------------------+              |
//	|  | MAC Array (16 units)                         |              |
//	|  | mac[i] = input[i] * weight[i]                |              |
//	|  +---------------------------------------------+              |
//	|                    |                                          |
//	|                    v                                          |
//	|  +---------------------------------------------+              |
//	|  | Activation Pipeline                          |              |
//	|  | ReLU / sigmoid / tanh / identity             |              |
//	|  +---------------------------------------------+              |
//	|                    |                                          |
//	|                    v                                          |
//	|  +---------------------------------------------+              |
//	|  | Output Buffer (128 KB)                       |              |
//	|  +---------------------------------------------+              |
//	+---------------------------------------------------------------+
//
// # Compiler-Scheduled Execution
//
// The ANE doesn't decide what to do at runtime. Instead, Apple's Core ML
// compiler generates a complete schedule:
//
//	Cycle 0-9:   DMA load input tile (10 elements/cycle)
//	Cycle 10-19: DMA load weight tile
//	Cycle 20:    MAC operation (16 parallel multiplies)
//	Cycle 21:    Reduce (sum MAC results)
//	Cycle 22:    Activate (apply ReLU)
//	Cycle 23:    DMA store output

import (
	"fmt"

	fp "github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic"
	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	pee "github.com/adhithyan15/coding-adventures/code/packages/go/parallel-execution-engine"
)

// =========================================================================
// ANECoreConfig -- configuration for an Apple Neural Engine Core
// =========================================================================

// ANECoreConfig holds configuration for an Apple Neural Engine Core.
//
// Real-world ANE configurations:
//
//	Parameter          | A14 (iPhone 12) | M1          | M2
//	-------------------+-----------------+-------------+----------
//	Cores              | 16              | 16          | 16
//	TOPS               | 11              | 11          | 15.8
//	Format             | FP16/INT8       | FP16/INT8   | FP16/INT8
//	On-chip memory     | varies          | varies      | varies
type ANECoreConfig struct {
	NumMACs           int
	MACFormat         fp.FloatFormat
	AccumulatorFormat fp.FloatFormat
	SRAMSize          int
	ActivationBuffer  int
	WeightBuffer      int
	OutputBuffer      int
	DMABandwidth      int
}

// DefaultANECoreConfig returns an ANECoreConfig with sensible defaults.
func DefaultANECoreConfig() ANECoreConfig {
	return ANECoreConfig{
		NumMACs:           16,
		MACFormat:         fp.FP16,
		AccumulatorFormat: fp.FP32,
		SRAMSize:          4194304,
		ActivationBuffer:  131072,
		WeightBuffer:      524288,
		OutputBuffer:      131072,
		DMABandwidth:      10,
	}
}

// =========================================================================
// NeuralEngineCore -- the main ANE Core simulator
// =========================================================================

// NeuralEngineCore is an Apple Neural Engine Core simulator.
//
// Uses a MACArrayEngine from Layer 8 internally, adding DMA simulation,
// activation pipeline, and compiler-generated schedule support.
//
// === Execution Model ===
//
// The ANE Core has no runtime scheduler. Instead, it follows a
// compiler-generated schedule that specifies exactly what happens
// on each cycle.
type NeuralEngineCore struct {
	config    ANECoreConfig
	clk       *clock.Clock
	cycle     int
	macEngine *pee.MACArrayEngine
	idleFlag  bool
	workItems []WorkItem
	result    [][]float64
}

// NewNeuralEngineCore creates a new Apple ANE Core simulator.
func NewNeuralEngineCore(config ANECoreConfig, clk *clock.Clock) *NeuralEngineCore {
	inputBufSize := config.ActivationBuffer / 4
	if inputBufSize < 1024 {
		inputBufSize = 1024
	}
	weightBufSize := config.WeightBuffer / 4
	if weightBufSize < 4096 {
		weightBufSize = 4096
	}
	outputBufSize := config.OutputBuffer / 4
	if outputBufSize < 1024 {
		outputBufSize = 1024
	}

	macEngine := pee.NewMACArrayEngine(
		pee.MACArrayConfig{
			NumMACs:          config.NumMACs,
			InputBufferSize:  inputBufSize,
			WeightBufferSize: weightBufSize,
			OutputBufferSize: outputBufSize,
			FloatFormat:      fp.FP32, // use FP32 internally
			AccumFormat:      fp.FP32,
			HasActivation:    true,
		},
		clk,
	)

	return &NeuralEngineCore{
		config:    config,
		clk:       clk,
		macEngine: macEngine,
		idleFlag:  true,
	}
}

// --- ComputeUnit interface ---

// Name returns the compute unit name.
func (ane *NeuralEngineCore) Name() string { return "ANECore" }

// Arch returns Apple ANE Core architecture.
func (ane *NeuralEngineCore) Arch() Architecture { return ArchAppleANECore }

// Idle returns true if no work remains.
func (ane *NeuralEngineCore) Idle() bool { return ane.idleFlag }

// Config returns the ANE Core configuration.
func (ane *NeuralEngineCore) Config() ANECoreConfig { return ane.config }

// ResultMatrix returns the result from the last computation.
func (ane *NeuralEngineCore) ResultMatrix() [][]float64 { return ane.result }

// MACEngine returns the underlying MAC array engine.
func (ane *NeuralEngineCore) MACEngine() *pee.MACArrayEngine { return ane.macEngine }

// --- Dispatch ---

// Dispatch dispatches an inference tile to this ANE Core.
//
// The WorkItem must provide InputData and WeightData. The ANE
// Core will compute: result = InputData x WeightData.
func (ane *NeuralEngineCore) Dispatch(work WorkItem) error {
	ane.workItems = append(ane.workItems, work)
	ane.idleFlag = false
	return nil
}

// --- Execution ---

// Step advances one cycle of the ANE Core.
//
// If work is pending, generates a compiler schedule, loads data
// into the MAC engine, and runs it to completion.
func (ane *NeuralEngineCore) Step(edge clock.ClockEdge) ComputeUnitTrace {
	ane.cycle++

	if ane.idleFlag || len(ane.workItems) == 0 {
		return ane.makeIdleTrace()
	}

	work := ane.workItems[0]
	ane.processWorkItem(work)
	ane.workItems = ane.workItems[1:]

	if len(ane.workItems) == 0 {
		ane.idleFlag = true
	}

	rows := len(ane.result)
	cols := 0
	if rows > 0 {
		cols = len(ane.result[0])
	}

	activeWarps := 0
	occ := 0.0
	if !ane.idleFlag {
		activeWarps = 1
		occ = 1.0
	}

	return ComputeUnitTrace{
		Cycle:             ane.cycle,
		UnitName:          ane.Name(),
		Arch:              ane.Arch(),
		SchedulerAction:   fmt.Sprintf("inference complete: %dx%d result", rows, cols),
		ActiveWarps:       activeWarps,
		TotalWarps:        1,
		EngineTraces:      make(map[int]pee.EngineTrace),
		SharedMemoryUsed:  0,
		SharedMemoryTotal: ane.config.SRAMSize,
		RegisterFileUsed:  ane.config.NumMACs,
		RegisterFileTotal: ane.config.NumMACs,
		Occupancy:         occ,
	}
}

// Run runs until all work completes or maxCycles is reached.
func (ane *NeuralEngineCore) Run(maxCycles int) []ComputeUnitTrace {
	var traces []ComputeUnitTrace
	for cycleNum := 1; cycleNum <= maxCycles; cycleNum++ {
		edge := clock.ClockEdge{
			Cycle:    cycleNum,
			Value:    1,
			IsRising: true,
		}
		trace := ane.Step(edge)
		traces = append(traces, trace)
		if ane.Idle() {
			break
		}
	}
	return traces
}

// RunInference is a convenience method: run a complete inference pass.
//
// Performs matmul + activation function, simulating how the ANE
// processes one layer of a neural network.
//
// === Inference Pipeline ===
//
//  1. DMA load inputs into activation buffer
//  2. DMA load weights into weight buffer
//  3. MAC: multiply input elements by weights
//  4. Reduce: sum MAC results
//  5. Activate: apply activation function
//  6. DMA store outputs
func (ane *NeuralEngineCore) RunInference(
	inputs, weights [][]float64,
	activationFn string,
) [][]float64 {
	result := matmul(inputs, weights)

	if activationFn != "none" {
		result = applyActivation(result, activationFn)
	}

	ane.result = result
	return result
}

// Reset resets all state.
func (ane *NeuralEngineCore) Reset() {
	ane.macEngine.Reset()
	ane.workItems = nil
	ane.result = nil
	ane.idleFlag = true
	ane.cycle = 0
}

// --- Private helpers ---

// processWorkItem processes a single work item by performing matmul.
func (ane *NeuralEngineCore) processWorkItem(work WorkItem) {
	if work.InputData != nil && work.WeightData != nil {
		ane.result = matmul(work.InputData, work.WeightData)
	} else {
		ane.result = nil
	}
}

// matmul performs matrix multiplication: C = A x B.
//
// For each element of the output matrix, we compute a dot product.
// This simulates how the ANE processes matrix multiplications tile by tile.
func matmul(a, b [][]float64) [][]float64 {
	if len(a) == 0 || len(b) == 0 {
		return nil
	}

	m := len(a)
	k := len(a[0])
	n := 0
	if len(b) > 0 {
		n = len(b[0])
	}

	result := make([][]float64, m)
	for i := 0; i < m; i++ {
		row := make([]float64, n)
		for j := 0; j < n; j++ {
			dot := 0.0
			for kk := 0; kk < k; kk++ {
				dot += a[i][kk] * b[kk][j]
			}
			row[j] = dot
		}
		result[i] = row
	}
	return result
}

// makeIdleTrace produces a trace for when the ANE Core is idle.
func (ane *NeuralEngineCore) makeIdleTrace() ComputeUnitTrace {
	return ComputeUnitTrace{
		Cycle:             ane.cycle,
		UnitName:          ane.Name(),
		Arch:              ane.Arch(),
		SchedulerAction:   "idle",
		ActiveWarps:       0,
		TotalWarps:        1,
		EngineTraces:      make(map[int]pee.EngineTrace),
		SharedMemoryUsed:  0,
		SharedMemoryTotal: ane.config.SRAMSize,
		RegisterFileUsed:  0,
		RegisterFileTotal: ane.config.NumMACs,
		Occupancy:         0.0,
	}
}

// String returns a human-readable representation.
func (ane *NeuralEngineCore) String() string {
	return fmt.Sprintf("NeuralEngineCore(macs=%d, idle=%t)",
		ane.config.NumMACs, ane.idleFlag)
}
