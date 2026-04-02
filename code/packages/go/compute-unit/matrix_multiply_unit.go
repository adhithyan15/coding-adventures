package computeunit

// MatrixMultiplyUnit -- Google TPU MXU simulator.
//
// # What is an MXU?
//
// The Matrix Multiply Unit is the heart of Google's TPU (Tensor Processing
// Unit). It's fundamentally different from GPU compute units -- there are NO
// threads, NO warps, NO schedulers. Instead, it has:
//
//  1. Systolic arrays -- the main compute engine (from Layer 8)
//  2. Vector unit -- for element-wise operations (activation functions)
//  3. Accumulators -- for storing partial matrix results
//  4. Control sequencer -- manages the tiling schedule
//
// # Why No Threads?
//
// Matrix multiplication is perfectly predictable. You know exactly which
// values need to be multiplied together and in what order. There's no
// branching, no data-dependent control flow, no need for a runtime scheduler.
//
// This predictability lets the compiler (XLA) generate a complete execution
// plan at compile time. The MXU hardware just follows this plan cycle by cycle.
//
//	GPU:  Complex hardware scheduler decides at runtime
//	TPU:  Simple hardware follows compile-time plan
//
// # Architecture Diagram
//
//	MatrixMultiplyUnit (TPU v2-style)
//	+---------------------------------------------------------------+
//	|  Control Sequencer                                            |
//	|  +----------------------------------------------------------+ |
//	|  | Tile schedule: load A[0:128], matmul, load A[128:256]    | |
//	|  +----------------------------------------------------------+ |
//	|                                                               |
//	|  +---------------------------------------------+              |
//	|  | Systolic Array (128x128)                     |              |
//	|  |   Weights pre-loaded into PEs                |              |
//	|  |   Activations stream in from left            |              |
//	|  |   Partial sums flow down to accumulators     |              |
//	|  +---------------------------------------------+              |
//	|                    |                                          |
//	|                    v                                          |
//	|  +---------------------------------------------+              |
//	|  | Accumulators (128 x FP32)                    |              |
//	|  +---------------------------------------------+              |
//	|                    |                                          |
//	|                    v                                          |
//	|  +---------------------------------------------+              |
//	|  | Vector Unit (128-wide)                       |              |
//	|  | ReLU, sigmoid, add bias, normalize           |              |
//	|  +---------------------------------------------+              |
//	+---------------------------------------------------------------+

import (
	"fmt"
	"math"

	fp "github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic"
	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	pee "github.com/adhithyan15/coding-adventures/code/packages/go/parallel-execution-engine"
)

// =========================================================================
// MXUConfig -- configuration for a TPU-style Matrix Multiply Unit
// =========================================================================

// MXUConfig holds configuration for a TPU-style Matrix Multiply Unit.
//
// Real-world MXU configurations:
//
//	Parameter           | TPU v1       | TPU v2/v3    | TPU v4
//	--------------------+--------------+--------------+----------
//	Array size          | 256x256      | 128x128      | 128x128
//	Input format        | INT8         | BF16         | BF16
//	Accumulator format  | INT32        | FP32         | FP32
//	Vector width        | 256          | 128          | 128
//	HBM bandwidth       | 30 GB/s      | 900 GB/s     | 1200 GB/s
type MXUConfig struct {
	ArrayRows           int
	ArrayCols           int
	SystolicFormat      fp.FloatFormat
	AccumulatorFormat   fp.FloatFormat
	VectorWidth         int
	VectorFormat        fp.FloatFormat
	AccumulatorCount    int
	WeightBufferSize    int
	ActivationBufferSize int
}

// DefaultMXUConfig returns an MXUConfig with sensible defaults.
func DefaultMXUConfig() MXUConfig {
	result, _ := StartNew[MXUConfig]("compute-unit.DefaultMXUConfig", MXUConfig{},
		func(op *Operation[MXUConfig], rf *ResultFactory[MXUConfig]) *OperationResult[MXUConfig] {
			return rf.Generate(true, false, MXUConfig{
				ArrayRows:           128,
				ArrayCols:           128,
				SystolicFormat:      fp.BF16,
				AccumulatorFormat:   fp.FP32,
				VectorWidth:         128,
				VectorFormat:        fp.FP32,
				AccumulatorCount:    128,
				WeightBufferSize:    4194304,
				ActivationBufferSize: 2097152,
			})
		}).GetResult()
	return result
}

// =========================================================================
// MatrixMultiplyUnit -- the main MXU simulator
// =========================================================================

// MatrixMultiplyUnit is a Google TPU Matrix Multiply Unit simulator.
//
// Uses a systolic array from Layer 8 to perform matrix multiplication,
// with tiling logic for matrices larger than the array, and a vector
// unit for post-processing (activation functions, bias add).
//
// === Execution Model ===
//
// The MXU has no threads or schedulers. Instead, it processes tiles
// of a larger matrix operation. The control sequencer manages:
//
//  1. Loading weight tiles into the systolic array
//  2. Streaming activation tiles through the array
//  3. Accumulating partial results
//  4. Applying vector operations (activation functions)
//  5. Storing output tiles
type MatrixMultiplyUnit struct {
	config       MXUConfig
	clk          *clock.Clock
	cycle        int
	array        *pee.SystolicArray
	accumulators [][]float64
	currentResult [][]float64
	workItems    []WorkItem
	idleFlag     bool
}

// NewMatrixMultiplyUnit creates a new TPU-style MXU simulator.
func NewMatrixMultiplyUnit(config MXUConfig, clk *clock.Clock) *MatrixMultiplyUnit {
	result, _ := StartNew[*MatrixMultiplyUnit]("compute-unit.NewMatrixMultiplyUnit", nil,
		func(op *Operation[*MatrixMultiplyUnit], rf *ResultFactory[*MatrixMultiplyUnit]) *OperationResult[*MatrixMultiplyUnit] {
			array := pee.NewSystolicArray(
				pee.SystolicConfig{
					Rows:              config.ArrayRows,
					Cols:              config.ArrayCols,
					FloatFormat:       fp.FP32,
					AccumulatorFormat: fp.FP32,
				},
				clk,
			)
			return rf.Generate(true, false, &MatrixMultiplyUnit{
				config:   config,
				clk:      clk,
				array:    array,
				idleFlag: true,
			})
		}).GetResult()
	return result
}

// --- ComputeUnit interface ---

// Name returns the compute unit name.
func (mxu *MatrixMultiplyUnit) Name() string {
	result, _ := StartNew[string]("compute-unit.MatrixMultiplyUnit.Name", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, "MXU")
		}).GetResult()
	return result
}

// Arch returns Google MXU architecture.
func (mxu *MatrixMultiplyUnit) Arch() Architecture {
	result, _ := StartNew[Architecture]("compute-unit.MatrixMultiplyUnit.Arch", 0,
		func(op *Operation[Architecture], rf *ResultFactory[Architecture]) *OperationResult[Architecture] {
			return rf.Generate(true, false, ArchGoogleMXU)
		}).GetResult()
	return result
}

// Idle returns true if no work remains.
func (mxu *MatrixMultiplyUnit) Idle() bool {
	result, _ := StartNew[bool]("compute-unit.MatrixMultiplyUnit.Idle", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, mxu.idleFlag)
		}).GetResult()
	return result
}

// Config returns the MXU configuration.
func (mxu *MatrixMultiplyUnit) Config() MXUConfig {
	result, _ := StartNew[MXUConfig]("compute-unit.MatrixMultiplyUnit.Config", MXUConfig{},
		func(op *Operation[MXUConfig], rf *ResultFactory[MXUConfig]) *OperationResult[MXUConfig] {
			return rf.Generate(true, false, mxu.config)
		}).GetResult()
	return result
}

// Result returns the result matrix from the last matmul.
func (mxu *MatrixMultiplyUnit) Result() [][]float64 {
	result, _ := StartNew[[][]float64]("compute-unit.MatrixMultiplyUnit.Result", nil,
		func(op *Operation[[][]float64], rf *ResultFactory[[][]float64]) *OperationResult[[][]float64] {
			return rf.Generate(true, false, mxu.currentResult)
		}).GetResult()
	return result
}

// SystolicArray returns access to the underlying systolic array.
func (mxu *MatrixMultiplyUnit) SystolicArray() *pee.SystolicArray {
	result, _ := StartNew[*pee.SystolicArray]("compute-unit.MatrixMultiplyUnit.SystolicArray", nil,
		func(op *Operation[*pee.SystolicArray], rf *ResultFactory[*pee.SystolicArray]) *OperationResult[*pee.SystolicArray] {
			return rf.Generate(true, false, mxu.array)
		}).GetResult()
	return result
}

// --- Dispatch ---

// Dispatch dispatches a matrix multiply operation.
//
// The WorkItem must provide InputData (activation matrix) and
// WeightData (weight matrix). The MXU will perform:
//
//	result = InputData x WeightData
func (mxu *MatrixMultiplyUnit) Dispatch(work WorkItem) error {
	_, err := StartNew[struct{}]("compute-unit.MatrixMultiplyUnit.Dispatch", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("work_id", work.WorkID)
			mxu.workItems = append(mxu.workItems, work)
			mxu.idleFlag = false
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// --- Execution ---

// Step advances one cycle of the MXU.
//
// If work is pending, performs the matmul using the systolic array.
func (mxu *MatrixMultiplyUnit) Step(edge clock.ClockEdge) ComputeUnitTrace {
	result, _ := StartNew[ComputeUnitTrace]("compute-unit.MatrixMultiplyUnit.Step", ComputeUnitTrace{},
		func(op *Operation[ComputeUnitTrace], rf *ResultFactory[ComputeUnitTrace]) *OperationResult[ComputeUnitTrace] {
			op.AddProperty("cycle", edge.Cycle)
			mxu.cycle++

			if mxu.idleFlag || len(mxu.workItems) == 0 {
				return rf.Generate(true, false, mxu.makeIdleTrace())
			}

			work := mxu.workItems[0]

			if work.InputData != nil && work.WeightData != nil {
				mxu.currentResult = mxu.array.RunMatmul(work.InputData, work.WeightData)
			} else {
				mxu.currentResult = nil
			}

			mxu.workItems = mxu.workItems[1:]
			if len(mxu.workItems) == 0 {
				mxu.idleFlag = true
			}

			rows := len(mxu.currentResult)
			cols := 0
			if rows > 0 {
				cols = len(mxu.currentResult[0])
			}

			activeWarps := 0
			occ := 0.0
			if !mxu.idleFlag {
				activeWarps = 1
				occ = 1.0
			}

			return rf.Generate(true, false, ComputeUnitTrace{
				Cycle:             mxu.cycle,
				UnitName:          mxu.Name(),
				Arch:              mxu.Arch(),
				SchedulerAction:   fmt.Sprintf("matmul complete: %dx%d result", rows, cols),
				ActiveWarps:       activeWarps,
				TotalWarps:        1,
				EngineTraces:      make(map[int]pee.EngineTrace),
				SharedMemoryUsed:  0,
				SharedMemoryTotal: mxu.config.WeightBufferSize,
				RegisterFileUsed:  mxu.config.AccumulatorCount,
				RegisterFileTotal: mxu.config.AccumulatorCount,
				Occupancy:         occ,
			})
		}).GetResult()
	return result
}

// Run runs until all work completes or maxCycles is reached.
func (mxu *MatrixMultiplyUnit) Run(maxCycles int) []ComputeUnitTrace {
	result, _ := StartNew[[]ComputeUnitTrace]("compute-unit.MatrixMultiplyUnit.Run", nil,
		func(op *Operation[[]ComputeUnitTrace], rf *ResultFactory[[]ComputeUnitTrace]) *OperationResult[[]ComputeUnitTrace] {
			op.AddProperty("max_cycles", maxCycles)
			var traces []ComputeUnitTrace
			for cycleNum := 1; cycleNum <= maxCycles; cycleNum++ {
				edge := clock.ClockEdge{
					Cycle:    cycleNum,
					Value:    1,
					IsRising: true,
				}
				trace := mxu.Step(edge)
				traces = append(traces, trace)
				if mxu.Idle() {
					break
				}
			}
			return rf.Generate(true, false, traces)
		}).GetResult()
	return result
}

// RunMatmul is a convenience method: run a complete matmul with optional activation.
//
// === Supported Activation Functions ===
//
//	"none":    f(x) = x              (identity)
//	"relu":    f(x) = max(0, x)      (most popular)
//	"sigmoid": f(x) = 1/(1+e^-x)    (squashes to [0,1])
//	"tanh":    f(x) = tanh(x)        (squashes to [-1,1])
func (mxu *MatrixMultiplyUnit) RunMatmul(
	activations, weights [][]float64,
	activationFn string,
) [][]float64 {
	result, _ := StartNew[[][]float64]("compute-unit.MatrixMultiplyUnit.RunMatmul", nil,
		func(op *Operation[[][]float64], rf *ResultFactory[[][]float64]) *OperationResult[[][]float64] {
			op.AddProperty("activation_fn", activationFn)
			res := mxu.array.RunMatmul(activations, weights)
			if activationFn != "none" {
				res = applyActivation(res, activationFn)
			}
			mxu.currentResult = res
			return rf.Generate(true, false, res)
		}).GetResult()
	return result
}

// Reset resets all state.
func (mxu *MatrixMultiplyUnit) Reset() {
	_, _ = StartNew[struct{}]("compute-unit.MatrixMultiplyUnit.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			mxu.array.Reset()
			mxu.accumulators = nil
			mxu.currentResult = nil
			mxu.workItems = nil
			mxu.idleFlag = true
			mxu.cycle = 0
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// makeIdleTrace produces a trace for when the MXU is idle.
func (mxu *MatrixMultiplyUnit) makeIdleTrace() ComputeUnitTrace {
	return ComputeUnitTrace{
		Cycle:             mxu.cycle,
		UnitName:          mxu.Name(),
		Arch:              mxu.Arch(),
		SchedulerAction:   "idle",
		ActiveWarps:       0,
		TotalWarps:        1,
		EngineTraces:      make(map[int]pee.EngineTrace),
		SharedMemoryUsed:  0,
		SharedMemoryTotal: mxu.config.WeightBufferSize,
		RegisterFileUsed:  0,
		RegisterFileTotal: mxu.config.AccumulatorCount,
		Occupancy:         0.0,
	}
}

// String returns a human-readable representation.
func (mxu *MatrixMultiplyUnit) String() string {
	result, _ := StartNew[string]("compute-unit.MatrixMultiplyUnit.String", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, fmt.Sprintf("MatrixMultiplyUnit(%dx%d, idle=%t)",
				mxu.config.ArrayRows, mxu.config.ArrayCols, mxu.idleFlag))
		}).GetResult()
	return result
}

// =========================================================================
// Activation functions -- shared by MXU and ANE
// =========================================================================

// applyActivation applies an activation function element-wise to a matrix.
//
// Simulates the vector unit / activation pipeline found in both TPUs
// and NPUs. Real hardware implements these functions in dedicated
// silicon for speed and power efficiency.
func applyActivation(matrix [][]float64, fnName string) [][]float64 {
	result := make([][]float64, len(matrix))
	for i, row := range matrix {
		newRow := make([]float64, len(row))
		for j, val := range row {
			switch fnName {
			case "relu":
				newRow[j] = math.Max(0.0, val)
			case "sigmoid":
				clamped := math.Max(-500.0, math.Min(500.0, val))
				newRow[j] = 1.0 / (1.0 + math.Exp(-clamped))
			case "tanh":
				newRow[j] = math.Tanh(val)
			default:
				newRow[j] = val
			}
		}
		result[i] = newRow
	}
	return result
}
