package parallelexecutionengine

// MACArrayEngine -- compiler-scheduled MAC array execution (NPU style).
//
// # What is a MAC Array?
//
// A MAC (Multiply-Accumulate) array is a bank of multiply-accumulate units
// driven entirely by a schedule that the compiler generates at compile time.
// There is NO hardware scheduler -- the compiler decides exactly which MAC
// unit processes which data on which cycle.
//
// This is the execution model used by:
//   - Apple Neural Engine (ANE)
//   - Qualcomm Hexagon NPU
//   - Many custom AI accelerator ASICs
//
// # How It Differs from Other Models
//
//	GPU (SIMT/SIMD):                   NPU (Scheduled MAC):
//	+--------------------------+       +--------------------------+
//	| Hardware scheduler       |       | NO hardware scheduler    |
//	| Runtime decisions        |       | All decisions at compile  |
//	| Branch prediction        |       | NO branches              |
//	| Dynamic resource alloc   |       | Static resource plan     |
//	+--------------------------+       +--------------------------+
//
// # The Execution Pipeline
//
// A MAC array engine has a simple pipeline:
//
//  1. LOAD_INPUT:    Move data from external memory to input buffer
//  2. LOAD_WEIGHTS:  Move weights from external memory to weight buffer
//  3. MAC:           Multiply input[i] * weight[i] for all MACs in parallel
//  4. REDUCE:        Sum the MAC results (adder tree)
//  5. ACTIVATE:      Apply activation function (ReLU, sigmoid, tanh)
//  6. STORE_OUTPUT:  Write result to output buffer
//
// # Why NPUs Are Power-Efficient
//
// By moving all scheduling to compile time, NPUs eliminate:
//   - Branch prediction hardware
//   - Instruction cache (the "program" is a simple schedule table)
//   - Warp/wavefront scheduler
//   - Speculation hardware
//
// The result: NPUs achieve more TOPS/watt than GPUs for neural network
// inference, at the cost of flexibility.

import (
	"fmt"
	"math"

	fp "github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic"
	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
)

// =========================================================================
// MACOperation -- operations in a MAC array schedule
// =========================================================================

// MACOperation represents an operation that can appear in a MAC array schedule.
//
// Each operation corresponds to one stage of the MAC pipeline:
//
//	LoadInput:    Fill the input buffer with activation data.
//	LoadWeights:  Fill the weight buffer with weight data.
//	MAC:          Parallel multiply-accumulate across all MAC units.
//	Reduce:       Sum results from multiple MACs (adder tree).
//	Activate:     Apply a non-linear activation function.
//	StoreOutput:  Write results to the output buffer.
type MACOperation int

const (
	OpLoadInput   MACOperation = iota // Fill input buffer
	OpLoadWeights                     // Fill weight buffer
	OpMAC                             // Parallel multiply-accumulate
	OpReduce                          // Sum MAC results (adder tree)
	OpActivate                        // Apply activation function
	OpStoreOutput                     // Write to output buffer
)

// macOperationNames maps each MACOperation to its string name.
var macOperationNames = map[MACOperation]string{
	OpLoadInput:   "LOAD_INPUT",
	OpLoadWeights: "LOAD_WEIGHTS",
	OpMAC:         "MAC",
	OpReduce:      "REDUCE",
	OpActivate:    "ACTIVATE",
	OpStoreOutput: "STORE_OUTPUT",
}

// String returns the human-readable name of a MACOperation.
func (op MACOperation) String() string {
	if name, ok := macOperationNames[op]; ok {
		return name
	}
	return fmt.Sprintf("UNKNOWN(%d)", int(op))
}

// =========================================================================
// ActivationFunction -- hardware-supported activation functions
// =========================================================================

// ActivationFunction represents a hardware-supported activation function.
//
// Neural networks use non-linear "activation functions" after each layer.
// NPUs typically implement a few common ones in hardware for speed:
//
//	None:    f(x) = x              (identity / linear)
//	ReLU:    f(x) = max(0, x)      (most popular; simple, fast)
//	Sigmoid: f(x) = 1/(1+e^-x)    (classic; squashes to [0,1])
//	Tanh:    f(x) = tanh(x)        (squashes to [-1,1])
type ActivationFunction int

const (
	ActivationNone    ActivationFunction = iota // f(x) = x
	ActivationReLU                              // f(x) = max(0, x)
	ActivationSigmoid                           // f(x) = 1/(1+e^-x)
	ActivationTanh                              // f(x) = tanh(x)
)

// activationNames maps each ActivationFunction to its string name.
var activationNames = map[ActivationFunction]string{
	ActivationNone:    "none",
	ActivationReLU:    "relu",
	ActivationSigmoid: "sigmoid",
	ActivationTanh:    "tanh",
}

// String returns the name of the activation function.
func (a ActivationFunction) String() string {
	if name, ok := activationNames[a]; ok {
		return name
	}
	return fmt.Sprintf("unknown(%d)", int(a))
}

// =========================================================================
// MACScheduleEntry -- one entry in the compiler-generated schedule
// =========================================================================

// MACScheduleEntry is one entry in the MAC array schedule.
//
// The compiler generates these at compile time. Each entry describes
// exactly what happens on one cycle -- which operation, which data indices,
// and where to write the result.
//
// Example schedule for a simple dot product of 4 elements:
//
//	Cycle 0: LOAD_INPUT   indices=[0,1,2,3]
//	Cycle 1: LOAD_WEIGHTS indices=[0,1,2,3]
//	Cycle 2: MAC          input=[0,1,2,3] weight=[0,1,2,3] out=0
//	Cycle 3: REDUCE       out=0
//	Cycle 4: ACTIVATE     out=0, activation=relu
//	Cycle 5: STORE_OUTPUT out=0
type MACScheduleEntry struct {
	Cycle         int
	Operation     MACOperation
	InputIndices  []int
	WeightIndices []int
	OutputIndex   int
	Activation    ActivationFunction
}

// =========================================================================
// MACArrayConfig -- configuration for a scheduled MAC array engine
// =========================================================================

// MACArrayConfig holds the configuration for a scheduled MAC array engine.
//
// Real-world reference values:
//
//	Hardware          | MACs | Input Buf | Weight Buf | Format
//	------------------+------+-----------+------------+-------
//	Apple ANE (M1)    | 16K  | varies    | varies     | FP16/INT8
//	Qualcomm Hexagon  | 2K   | varies    | varies     | INT8
//	Our default       | 8    | 1024      | 4096       | FP16
type MACArrayConfig struct {
	NumMACs          int
	InputBufferSize  int
	WeightBufferSize int
	OutputBufferSize int
	FloatFormat      fp.FloatFormat
	AccumFormat      fp.FloatFormat
	HasActivation    bool
}

// DefaultMACArrayConfig returns a MACArrayConfig with sensible defaults.
func DefaultMACArrayConfig() MACArrayConfig {
	return MACArrayConfig{
		NumMACs:          8,
		InputBufferSize:  1024,
		WeightBufferSize: 4096,
		OutputBufferSize: 1024,
		FloatFormat:      fp.FP16,
		AccumFormat:      fp.FP32,
		HasActivation:    true,
	}
}

// =========================================================================
// MACArrayEngine -- the scheduled execution engine
// =========================================================================

// MACArrayEngine is a compiler-scheduled MAC array execution engine (NPU style).
//
// No hardware scheduler. The compiler generates a static schedule that
// says exactly what each MAC does on each cycle.
//
// # Usage Pattern
//
//  1. Create engine with config and clock.
//  2. Load inputs and weights into the buffers.
//  3. Load a compiler-generated schedule.
//  4. Step or run -- the engine follows the schedule exactly.
//  5. Read results from the output buffer.
type MACArrayEngine struct {
	config          MACArrayConfig
	clk             *clock.Clock
	cycle           int
	inputBuffer     []float64
	weightBuffer    []float64
	outputBuffer    []float64
	macAccumulators []float64
	schedule        []MACScheduleEntry
	halted          bool
}

// NewMACArrayEngine creates a new scheduled MAC array engine.
func NewMACArrayEngine(config MACArrayConfig, clk *clock.Clock) *MACArrayEngine {
	return &MACArrayEngine{
		config:          config,
		clk:             clk,
		inputBuffer:     make([]float64, config.InputBufferSize),
		weightBuffer:    make([]float64, config.WeightBufferSize),
		outputBuffer:    make([]float64, config.OutputBufferSize),
		macAccumulators: make([]float64, config.NumMACs),
	}
}

// --- Interface methods ---

// Name returns the engine name for traces.
func (m *MACArrayEngine) Name() string { return "MACArrayEngine" }

// Width returns the number of parallel MAC units.
func (m *MACArrayEngine) Width() int { return m.config.NumMACs }

// ExecutionModel returns ScheduledMAC.
func (m *MACArrayEngine) ExecutionModel() ExecutionModel { return ScheduledMAC }

// IsHalted returns true if the schedule is complete.
func (m *MACArrayEngine) IsHalted() bool { return m.halted }

// Config returns the configuration this engine was created with.
func (m *MACArrayEngine) Config() MACArrayConfig { return m.config }

// --- Data loading ---

// LoadInputs loads activation data into the input buffer.
//
// In real hardware, this is a DMA transfer from external memory
// to the on-chip input SRAM.
func (m *MACArrayEngine) LoadInputs(data []float64) {
	for i, val := range data {
		if i < m.config.InputBufferSize {
			m.inputBuffer[i] = val
		}
	}
}

// LoadWeights loads weight data into the weight buffer.
func (m *MACArrayEngine) LoadWeights(data []float64) {
	for i, val := range data {
		if i < m.config.WeightBufferSize {
			m.weightBuffer[i] = val
		}
	}
}

// LoadSchedule loads a compiler-generated execution schedule.
func (m *MACArrayEngine) LoadSchedule(schedule []MACScheduleEntry) {
	m.schedule = make([]MACScheduleEntry, len(schedule))
	copy(m.schedule, schedule)
	m.halted = false
}

// --- Execution ---

// Step executes one scheduled cycle.
//
// Looks up the current cycle in the schedule and executes the
// corresponding operation. If no entry exists for this cycle,
// the MAC array idles (like a NOP).
func (m *MACArrayEngine) Step(edge clock.ClockEdge) EngineTrace {
	m.cycle++

	if m.halted {
		return m.makeIdleTrace("Schedule complete")
	}

	// Find schedule entries for this cycle.
	var entries []MACScheduleEntry
	for _, e := range m.schedule {
		if e.Cycle == m.cycle {
			entries = append(entries, e)
		}
	}

	if len(entries) == 0 {
		// Check if we've passed all schedule entries.
		maxCycle := 0
		for _, e := range m.schedule {
			if e.Cycle > maxCycle {
				maxCycle = e.Cycle
			}
		}
		if m.cycle > maxCycle {
			m.halted = true
			return m.makeIdleTrace("Schedule complete")
		}
		return m.makeIdleTrace("No operation this cycle")
	}

	// Execute all entries for this cycle.
	unitTraces := make(map[int]string)
	activeCount := 0
	var descriptions []string

	for _, entry := range entries {
		switch entry.Operation {
		case OpLoadInput:
			desc := fmt.Sprintf("LOAD_INPUT indices=%v", entry.InputIndices)
			descriptions = append(descriptions, desc)
			activeCount = len(entry.InputIndices)

		case OpLoadWeights:
			desc := fmt.Sprintf("LOAD_WEIGHTS indices=%v", entry.WeightIndices)
			descriptions = append(descriptions, desc)
			activeCount = len(entry.WeightIndices)

		case OpMAC:
			desc, traces := m.execMAC(entry)
			descriptions = append(descriptions, desc)
			for k, v := range traces {
				unitTraces[k] = v
			}
			activeCount = len(traces)

		case OpReduce:
			desc := m.execReduce(entry)
			descriptions = append(descriptions, desc)
			activeCount = 1

		case OpActivate:
			desc := m.execActivate(entry)
			descriptions = append(descriptions, desc)
			activeCount = 1

		case OpStoreOutput:
			desc := m.execStore(entry)
			descriptions = append(descriptions, desc)
			activeCount = 1
		}
	}

	total := m.config.NumMACs
	description := ""
	for i, d := range descriptions {
		if i > 0 {
			description += "; "
		}
		description += d
	}

	activeMask := make([]bool, total)
	for i := 0; i < activeCount && i < total; i++ {
		activeMask[i] = true
	}

	utilization := 0.0
	if total > 0 {
		utilization = float64(activeCount) / float64(total)
	}

	return EngineTrace{
		Cycle:       m.cycle,
		EngineName:  m.Name(),
		Model:       m.ExecutionModel(),
		Description: fmt.Sprintf("%s -- %d/%d MACs active", description, activeCount, total),
		UnitTraces:  unitTraces,
		ActiveMask:  activeMask,
		ActiveCount: activeCount,
		TotalCount:  total,
		Utilization: utilization,
	}
}

// Run runs the full schedule.
func (m *MACArrayEngine) Run(maxCycles int) ([]EngineTrace, error) {
	var traces []EngineTrace
	for cycleNum := 1; cycleNum <= maxCycles; cycleNum++ {
		edge := clock.ClockEdge{
			Cycle:    cycleNum,
			Value:    1,
			IsRising: true,
		}
		trace := m.Step(edge)
		traces = append(traces, trace)
		if m.halted {
			return traces, nil
		}
	}
	return traces, fmt.Errorf("MACArrayEngine: max_cycles (%d) reached", maxCycles)
}

// ReadOutputs reads results from the output buffer.
func (m *MACArrayEngine) ReadOutputs() []float64 {
	result := make([]float64, len(m.outputBuffer))
	copy(result, m.outputBuffer)
	return result
}

// Reset resets to initial state.
func (m *MACArrayEngine) Reset() {
	m.inputBuffer = make([]float64, m.config.InputBufferSize)
	m.weightBuffer = make([]float64, m.config.WeightBufferSize)
	m.outputBuffer = make([]float64, m.config.OutputBufferSize)
	m.macAccumulators = make([]float64, m.config.NumMACs)
	m.halted = false
	m.cycle = 0
}

// --- Operation implementations ---

// execMAC executes a MAC operation: multiply input[i] * weight[i] for each MAC.
func (m *MACArrayEngine) execMAC(entry MACScheduleEntry) (string, map[int]string) {
	unitTraces := make(map[int]string)
	numOps := len(entry.InputIndices)
	if len(entry.WeightIndices) < numOps {
		numOps = len(entry.WeightIndices)
	}
	if m.config.NumMACs < numOps {
		numOps = m.config.NumMACs
	}

	for macID := 0; macID < numOps; macID++ {
		inIdx := entry.InputIndices[macID]
		wtIdx := entry.WeightIndices[macID]

		inVal := m.inputBuffer[inIdx]
		wtVal := m.weightBuffer[wtIdx]

		result := inVal * wtVal
		m.macAccumulators[macID] = result

		unitTraces[macID] = fmt.Sprintf("MAC: %.4g * %.4g = %.4g", inVal, wtVal, result)
	}

	return fmt.Sprintf("MAC %d operations", numOps), unitTraces
}

// execReduce sums all MAC accumulators (adder tree).
func (m *MACArrayEngine) execReduce(entry MACScheduleEntry) string {
	total := 0.0
	for _, acc := range m.macAccumulators {
		total += acc
	}
	outIdx := entry.OutputIndex
	if outIdx < m.config.OutputBufferSize {
		m.outputBuffer[outIdx] = total
	}
	return fmt.Sprintf("REDUCE sum=%.4g -> output[%d]", total, outIdx)
}

// execActivate applies an activation function.
//
// Activation functions:
//
//	None:    f(x) = x
//	ReLU:    f(x) = max(0, x)
//	Sigmoid: f(x) = 1 / (1 + e^-x)
//	Tanh:    f(x) = tanh(x)
func (m *MACArrayEngine) execActivate(entry MACScheduleEntry) string {
	if !m.config.HasActivation {
		return "ACTIVATE skipped (no hardware activation unit)"
	}

	outIdx := entry.OutputIndex
	if outIdx >= m.config.OutputBufferSize {
		return fmt.Sprintf("ACTIVATE error: index %d out of range", outIdx)
	}

	val := m.outputBuffer[outIdx]
	var result float64

	switch entry.Activation {
	case ActivationNone:
		result = val
	case ActivationReLU:
		result = math.Max(0.0, val)
	case ActivationSigmoid:
		clamped := math.Max(-500.0, math.Min(500.0, val))
		result = 1.0 / (1.0 + math.Exp(-clamped))
	case ActivationTanh:
		result = math.Tanh(val)
	default:
		result = val
	}

	m.outputBuffer[outIdx] = result
	return fmt.Sprintf("ACTIVATE %s(%.4g) = %.4g", entry.Activation.String(), val, result)
}

// execStore executes a STORE_OUTPUT operation.
func (m *MACArrayEngine) execStore(entry MACScheduleEntry) string {
	outIdx := entry.OutputIndex
	val := 0.0
	if outIdx < m.config.OutputBufferSize {
		val = m.outputBuffer[outIdx]
	}
	return fmt.Sprintf("STORE_OUTPUT output[%d] = %.4g", outIdx, val)
}

// makeIdleTrace produces a trace for idle/halted cycles.
func (m *MACArrayEngine) makeIdleTrace(description string) EngineTrace {
	return EngineTrace{
		Cycle:       m.cycle,
		EngineName:  m.Name(),
		Model:       m.ExecutionModel(),
		Description: description,
		UnitTraces:  make(map[int]string),
		ActiveMask:  make([]bool, m.config.NumMACs),
		ActiveCount: 0,
		TotalCount:  m.config.NumMACs,
		Utilization: 0.0,
	}
}

// String returns a human-readable representation of the engine.
func (m *MACArrayEngine) String() string {
	return fmt.Sprintf("MACArrayEngine(num_macs=%d, cycle=%d, halted=%t)",
		m.config.NumMACs, m.cycle, m.halted)
}
