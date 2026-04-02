package parallelexecutionengine

// SystolicArray -- dataflow execution for matrix multiplication (Google TPU style).
//
// # What is a Systolic Array?
//
// The word "systolic" comes from the Greek "systole" (contraction), like a
// heartbeat. In a systolic array, data pulses through a grid of processing
// elements on each clock cycle, just like blood pulses through the body.
//
// A systolic array is radically different from GPU execution:
//
//	GPU (SIMT/SIMD):                   TPU (Systolic):
//	+--------------------------+       +--------------------------+
//	| Has instructions         |       | NO instructions           |
//	| Has program counter      |       | NO program counter        |
//	| Has branches             |       | NO branches               |
//	| Complex control logic    |       | Dead-simple PEs           |
//	| General-purpose          |       | Matrix multiply ONLY      |
//	+--------------------------+       +--------------------------+
//
// Each PE in the array does exactly ONE thing on each clock cycle:
//
//	accumulator += input_from_left * local_weight
//
// Then it passes the input to the right neighbor. That's it. No instruction
// fetch, no decode, no branch prediction. Just multiply, accumulate, and pass.
//
// # How Matrix Multiplication Maps to a Systolic Array
//
// Computing C = A x W (activation matrix times weight matrix):
//
//  1. Pre-load weights into each PE: PE(i,j) gets W[i][j]
//  2. Feed activation rows from the left, STAGGERED in time
//  3. Data flows right through each row, partial sums flow down
//  4. After 2N-1 cycles, the result matrix C emerges at the bottom
//
// # Why TPUs Use Systolic Arrays
//
// Neural network inference and training are dominated by matrix multiplication.
// A systolic array is the most efficient hardware for this because:
//
//  1. No instruction overhead (no fetch, decode, branch)
//  2. Maximum data reuse (each value is used N times as it flows through)
//  3. Nearest-neighbor communication only
//  4. Regular, predictable data movement (no cache misses)
//  5. Simple PE design -> high clock frequency, low power
//
// Google's TPU v1 has a 256x256 systolic array that performs 65,536 MAC
// operations per clock cycle.

import (
	"fmt"

	fp "github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic"
	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
)

// =========================================================================
// SystolicConfig -- configuration for a systolic array
// =========================================================================

// SystolicConfig holds the configuration for a systolic array engine.
//
// Real-world reference values:
//
//	Hardware    | Rows | Cols | Format | Accumulator
//	------------+------+------+--------+------------
//	TPU v1      | 256  | 256  | INT8   | INT32
//	TPU v2/v3   | 128  | 128  | BF16   | FP32
//	Our default | 4    | 4    | FP32   | FP32
type SystolicConfig struct {
	Rows              int
	Cols              int
	FloatFormat       fp.FloatFormat
	AccumulatorFormat fp.FloatFormat
}

// DefaultSystolicConfig returns a SystolicConfig with sensible defaults.
func DefaultSystolicConfig() SystolicConfig {
	result, _ := StartNew[SystolicConfig]("parallel-execution-engine.DefaultSystolicConfig", SystolicConfig{},
		func(op *Operation[SystolicConfig], rf *ResultFactory[SystolicConfig]) *OperationResult[SystolicConfig] {
			return rf.Generate(true, false, SystolicConfig{
				Rows:              4,
				Cols:              4,
				FloatFormat:       fp.FP32,
				AccumulatorFormat: fp.FP32,
			})
		}).GetResult()
	return result
}

// =========================================================================
// SystolicPE -- one processing element in the grid
// =========================================================================

// SystolicPE is one processing element in the systolic array.
//
// Each PE is extremely simple -- it's just a multiply-accumulate unit
// with two data ports:
//
//	Input from left --> [  weight  ] --> Output to right
//	                    [  x + acc ]
//	                         |
//	                  Partial sum flows down
//
// On each clock cycle, a PE does:
//  1. If there's an input: accumulator += input * weight
//  2. Pass the input to the right neighbor
type SystolicPE struct {
	Row         int
	Col         int
	Weight      fp.FloatBits
	Accumulator fp.FloatBits
	InputBuffer *fp.FloatBits // nil if no input waiting
}

// Compute performs one MAC cycle.
//
// If there's an input waiting in the buffer:
//
//	accumulator += input_buffer * weight
//
// Returns the input (to be passed to the right neighbor), or nil.
//
// This is the heart of the systolic array -- the simplest possible
// processing element. No instruction fetch, no decode, no branch.
// Just: multiply, accumulate, pass.
func (pe *SystolicPE) Compute() *fp.FloatBits {
	if pe.InputBuffer == nil {
		return nil
	}

	inputVal := *pe.InputBuffer
	pe.InputBuffer = nil

	// MAC: accumulator = input * weight + accumulator
	// Using FMA for fused multiply-add (more accurate than mul+add).
	pe.Accumulator = fp.FMA(inputVal, pe.Weight, pe.Accumulator)

	return &inputVal // Pass to right neighbor
}

// =========================================================================
// SystolicArray -- the dataflow execution engine
// =========================================================================

// SystolicArray is a systolic dataflow execution engine (Google TPU style).
//
// An NxN grid of processing elements. Data flows through the array --
// activations left-to-right, partial sums accumulate in each PE.
// No instruction stream. Just data in, results out.
//
// # Data Flow Pattern
//
//	Inputs feed from the left edge:
//
//	a[0] --> PE(0,0) --> PE(0,1) --> PE(0,2) --> PE(0,3)
//	a[1] --> PE(1,0) --> PE(1,1) --> PE(1,2) --> PE(1,3)
//	a[2] --> PE(2,0) --> PE(2,1) --> PE(2,2) --> PE(2,3)
//	a[3] --> PE(3,0) --> PE(3,1) --> PE(3,2) --> PE(3,3)
//
//	Each PE accumulates: acc += input * weight
type SystolicArray struct {
	config      SystolicConfig
	clk         *clock.Clock
	cycle       int
	halted      bool
	Grid        [][]*SystolicPE
	inputQueues [][]fp.FloatBits
}

// NewSystolicArray creates a new systolic array engine.
func NewSystolicArray(config SystolicConfig, clk *clock.Clock) *SystolicArray {
	result, _ := StartNew[*SystolicArray]("parallel-execution-engine.NewSystolicArray", nil,
		func(op *Operation[*SystolicArray], rf *ResultFactory[*SystolicArray]) *OperationResult[*SystolicArray] {
			grid := make([][]*SystolicPE, config.Rows)
			zeroWeight := fp.FloatToBits(0.0, config.FloatFormat)
			zeroAcc := fp.FloatToBits(0.0, config.AccumulatorFormat)
			for r := 0; r < config.Rows; r++ {
				grid[r] = make([]*SystolicPE, config.Cols)
				for c := 0; c < config.Cols; c++ {
					grid[r][c] = &SystolicPE{
						Row:         r,
						Col:         c,
						Weight:      zeroWeight,
						Accumulator: zeroAcc,
					}
				}
			}

			inputQueues := make([][]fp.FloatBits, config.Rows)
			for i := range inputQueues {
				inputQueues[i] = nil
			}

			return rf.Generate(true, false, &SystolicArray{
				config:      config,
				clk:         clk,
				Grid:        grid,
				inputQueues: inputQueues,
			})
		}).GetResult()
	return result
}

// --- Interface methods ---

// Name returns the engine name for traces.
func (s *SystolicArray) Name() string {
	result, _ := StartNew[string]("parallel-execution-engine.SystolicArray.Name", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, "SystolicArray")
		}).GetResult()
	return result
}

// Width returns the total number of PEs in the array.
func (s *SystolicArray) Width() int {
	result, _ := StartNew[int]("parallel-execution-engine.SystolicArray.Width", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, s.config.Rows*s.config.Cols)
		}).GetResult()
	return result
}

// ExecutionModel returns Systolic.
func (s *SystolicArray) ExecutionModel() ExecutionModel {
	result, _ := StartNew[ExecutionModel]("parallel-execution-engine.SystolicArray.ExecutionModel", Systolic,
		func(op *Operation[ExecutionModel], rf *ResultFactory[ExecutionModel]) *OperationResult[ExecutionModel] {
			return rf.Generate(true, false, Systolic)
		}).GetResult()
	return result
}

// IsHalted returns true if all data has flowed through.
func (s *SystolicArray) IsHalted() bool {
	result, _ := StartNew[bool]("parallel-execution-engine.SystolicArray.IsHalted", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, s.halted)
		}).GetResult()
	return result
}

// Config returns the configuration this array was created with.
func (s *SystolicArray) Config() SystolicConfig {
	result, _ := StartNew[SystolicConfig]("parallel-execution-engine.SystolicArray.Config", SystolicConfig{},
		func(op *Operation[SystolicConfig], rf *ResultFactory[SystolicConfig]) *OperationResult[SystolicConfig] {
			return rf.Generate(true, false, s.config)
		}).GetResult()
	return result
}

// --- Weight loading ---

// LoadWeights pre-loads the weight matrix into the PE array.
//
// weights[row][col] goes to PE(row, col). In real TPU hardware, weight
// loading happens before the matrix multiply begins. The weights stay
// fixed while activations flow through.
func (s *SystolicArray) LoadWeights(weights [][]float64) {
	_, _ = StartNew[struct{}]("parallel-execution-engine.SystolicArray.LoadWeights", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			for r := 0; r < len(weights) && r < s.config.Rows; r++ {
				for c := 0; c < len(weights[r]) && c < s.config.Cols; c++ {
					s.Grid[r][c].Weight = fp.FloatToBits(weights[r][c], s.config.FloatFormat)
				}
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// --- Input feeding ---

// FeedInput feeds one activation value into the left edge of the specified row.
//
// The value will enter PE(row, 0) on the next step, then flow right
// through PE(row, 1), PE(row, 2), etc. on subsequent steps.
//
// Returns an error if the row is out of range.
func (s *SystolicArray) FeedInput(row int, value float64) error {
	result, err := StartNew[struct{}]("parallel-execution-engine.SystolicArray.FeedInput", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			if row < 0 || row >= s.config.Rows {
				return rf.Fail(struct{}{}, fmt.Errorf("row %d out of range [0, %d)", row, s.config.Rows))
			}
			s.inputQueues[row] = append(s.inputQueues[row], fp.FloatToBits(value, s.config.FloatFormat))
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	_ = result
	return err
}

// FeedInputVector feeds a full column vector to all rows.
func (s *SystolicArray) FeedInputVector(values []float64) {
	_, _ = StartNew[struct{}]("parallel-execution-engine.SystolicArray.FeedInputVector", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			for rowIdx, val := range values {
				if rowIdx < s.config.Rows {
					s.inputQueues[rowIdx] = append(s.inputQueues[rowIdx],
						fp.FloatToBits(val, s.config.FloatFormat))
				}
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// --- Execution ---

// Step advances one cycle: data moves one PE to the right.
//
// On each cycle:
//  1. For each PE (from right to left, to avoid overwriting):
//     a. Compute: acc += input * weight
//     b. Pass input to the right neighbor.
//  2. Feed input from queues into the leftmost column.
//  3. Build a trace showing the state of the array.
//
// We process PEs from right to left so that the "pass to right"
// doesn't interfere with the current cycle's computation.
func (s *SystolicArray) Step(edge clock.ClockEdge) EngineTrace {
	result, _ := StartNew[EngineTrace]("parallel-execution-engine.SystolicArray.Step", EngineTrace{},
		func(op *Operation[EngineTrace], rf *ResultFactory[EngineTrace]) *OperationResult[EngineTrace] {
			s.cycle++

			activeCount := 0
			peStates := make([][]string, s.config.Rows)

			// Phase 1: Move data rightward through the array.
			// Process from right to left to avoid data collision.
			for r := 0; r < s.config.Rows; r++ {
				for c := s.config.Cols - 1; c >= 0; c-- {
					pe := s.Grid[r][c]
					output := pe.Compute()
					if output != nil {
						activeCount++
						// Pass input to right neighbor (if exists).
						if c+1 < s.config.Cols {
							copied := *output
							s.Grid[r][c+1].InputBuffer = &copied
						}
					}
				}

				// Build state strings (left to right for display).
				peStates[r] = make([]string, s.config.Cols)
				for c := 0; c < s.config.Cols; c++ {
					pe := s.Grid[r][c]
					accVal := fp.BitsToFloat(pe.Accumulator)
					state := fmt.Sprintf("acc=%.4g", accVal)
					if pe.InputBuffer != nil {
						inVal := fp.BitsToFloat(*pe.InputBuffer)
						state += fmt.Sprintf(", in=%.4g", inVal)
					}
					peStates[r][c] = state
				}
			}

			// Phase 2: Feed new inputs from queues into column 0.
			for r := 0; r < s.config.Rows; r++ {
				if len(s.inputQueues[r]) > 0 {
					val := s.inputQueues[r][0]
					s.inputQueues[r] = s.inputQueues[r][1:]
					s.Grid[r][0].InputBuffer = &val
				}
			}

			// Check if computation is complete.
			total := s.config.Rows * s.config.Cols
			anyInputRemaining := false
			for _, q := range s.inputQueues {
				if len(q) > 0 {
					anyInputRemaining = true
					break
				}
			}
			anyInputInFlight := false
			for r := 0; r < s.config.Rows; r++ {
				for c := 0; c < s.config.Cols; c++ {
					if s.Grid[r][c].InputBuffer != nil {
						anyInputInFlight = true
						break
					}
				}
				if anyInputInFlight {
					break
				}
			}

			if !anyInputRemaining && !anyInputInFlight {
				s.halted = true
			}

			utilization := 0.0
			if total > 0 {
				utilization = float64(activeCount) / float64(total)
			}

			// Build unit traces and active mask.
			unitTraces := make(map[int]string)
			activeMask := make([]bool, total)
			for r := 0; r < s.config.Rows; r++ {
				for c := 0; c < s.config.Cols; c++ {
					idx := r*s.config.Cols + c
					unitTraces[idx] = peStates[r][c]
					// Mark as active if it computed or has pending input.
					activeMask[idx] = s.Grid[r][c].InputBuffer != nil || idx < activeCount
				}
			}

			return rf.Generate(true, false, EngineTrace{
				Cycle:       s.cycle,
				EngineName:  s.Name(),
				Model:       s.ExecutionModel(),
				Description: fmt.Sprintf("Systolic step -- %d/%d PEs active", activeCount, total),
				UnitTraces:  unitTraces,
				ActiveMask:  activeMask,
				ActiveCount: activeCount,
				TotalCount:  total,
				Utilization: utilization,
				Dataflow:    &DataflowInfo{PEStates: peStates},
			})
		}).GetResult()
	return result
}

// RunMatmul runs a complete matrix multiplication C = A x W.
//
// # How the Systolic Matmul Works
//
// For C = A x W where A is MxK and W is KxN:
//
//	C[i][j] = sum_k( A[i][k] * W[k][j] )
//
// We compute this one output row at a time:
//
//	For each row i of A:
//	  1. Reset accumulators
//	  2. Feed A[i][k] into PE row k (with staggered timing)
//	  3. After all activations flow through, drain results
func (s *SystolicArray) RunMatmul(activations [][]float64, weights [][]float64) [][]float64 {
	result, _ := StartNew[[][]float64]("parallel-execution-engine.SystolicArray.RunMatmul", nil,
		func(op *Operation[[][]float64], rf *ResultFactory[[][]float64]) *OperationResult[[][]float64] {
			numOutputRows := len(activations)
			innerDim := 0
			if numOutputRows > 0 {
				innerDim = len(activations[0])
			}
			numOutputCols := 0
			if len(weights) > 0 {
				numOutputCols = len(weights[0])
			}

			// Load weights: PE(k, j) gets W[k][j].
			s.Reset()
			s.LoadWeights(weights)

			out := make([][]float64, numOutputRows)

			// Compute one output row at a time.
			for i := 0; i < numOutputRows; i++ {
				// Reset accumulators (but keep weights).
				zeroAcc := fp.FloatToBits(0.0, s.config.AccumulatorFormat)
				for r := 0; r < s.config.Rows; r++ {
					for c := 0; c < s.config.Cols; c++ {
						s.Grid[r][c].Accumulator = zeroAcc
						s.Grid[r][c].InputBuffer = nil
					}
				}
				s.inputQueues = make([][]fp.FloatBits, s.config.Rows)
				for r := range s.inputQueues {
					s.inputQueues[r] = nil
				}
				s.halted = false

				// Build a feed schedule: row k gets A[i][k] at cycle k.
				feedSchedule := make(map[int][]struct {
					row int
					val float64
				})
				for k := 0; k < innerDim; k++ {
					feedSchedule[k] = append(feedSchedule[k], struct {
						row int
						val float64
					}{k, activations[i][k]})
				}

				// Run until all data has flowed through.
				totalSteps := innerDim + s.config.Cols + 1
				for stepNum := 0; stepNum < totalSteps; stepNum++ {
					if feeds, ok := feedSchedule[stepNum]; ok {
						for _, f := range feeds {
							_ = s.FeedInput(f.row, f.val)
						}
					}
					edge := clock.ClockEdge{
						Cycle:    stepNum + 1,
						Value:    1,
						IsRising: true,
					}
					s.Step(edge)
				}

				// Drain: sum accumulators vertically for each column j.
				// C[i][j] = sum_k PE(k, j).accumulator
				rowResult := make([]float64, numOutputCols)
				for j := 0; j < numOutputCols; j++ {
					colSum := 0.0
					limit := innerDim
					if s.config.Rows < limit {
						limit = s.config.Rows
					}
					for k := 0; k < limit; k++ {
						colSum += fp.BitsToFloat(s.Grid[k][j].Accumulator)
					}
					rowResult[j] = colSum
				}
				out[i] = rowResult
			}

			return rf.Generate(true, false, out)
		}).GetResult()
	return result
}

// DrainOutputs reads the accumulated results from all PEs.
//
// After computation, each PE's accumulator holds one element of the
// result. PE(r, c) holds C[r][c].
func (s *SystolicArray) DrainOutputs() [][]float64 {
	result, _ := StartNew[[][]float64]("parallel-execution-engine.SystolicArray.DrainOutputs", nil,
		func(op *Operation[[][]float64], rf *ResultFactory[[][]float64]) *OperationResult[[][]float64] {
			out := make([][]float64, s.config.Rows)
			for r := 0; r < s.config.Rows; r++ {
				row := make([]float64, s.config.Cols)
				for c := 0; c < s.config.Cols; c++ {
					row[c] = fp.BitsToFloat(s.Grid[r][c].Accumulator)
				}
				out[r] = row
			}
			return rf.Generate(true, false, out)
		}).GetResult()
	return result
}

// Reset resets the array to its initial state.
//
// Clears all accumulators, input buffers, and queues. Weights are
// preserved -- call LoadWeights() to change them.
func (s *SystolicArray) Reset() {
	_, _ = StartNew[struct{}]("parallel-execution-engine.SystolicArray.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			zeroAcc := fp.FloatToBits(0.0, s.config.AccumulatorFormat)
			for r := 0; r < s.config.Rows; r++ {
				for c := 0; c < s.config.Cols; c++ {
					s.Grid[r][c].Accumulator = zeroAcc
					s.Grid[r][c].InputBuffer = nil
				}
			}
			s.inputQueues = make([][]fp.FloatBits, s.config.Rows)
			for r := range s.inputQueues {
				s.inputQueues[r] = nil
			}
			s.cycle = 0
			s.halted = false
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// String returns a human-readable representation of the array.
func (s *SystolicArray) String() string {
	return fmt.Sprintf("SystolicArray(%dx%d, cycle=%d, halted=%t)",
		s.config.Rows, s.config.Cols, s.cycle, s.halted)
}
