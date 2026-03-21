package devicesimulator

// Work Distributor -- assigns work to compute units.
//
// # Three Distribution Strategies
//
// Different accelerator architectures distribute work in fundamentally
// different ways. This file implements all three:
//
//  1. **GPU Block Distributor** (NVIDIA, AMD, Intel)
//     - Takes a kernel launch with grid/block dimensions
//     - Decomposes into thread blocks
//     - Assigns blocks to compute units that have free resources
//     - Continues assigning as CUs complete blocks (multi-wave)
//
//  2. **TPU Sequencer** (Google TPU)
//     - Takes HLO operations (matmul, add, relu, etc.)
//     - Tiles large operations to fit the MXU
//     - Pipelines through Scalar -> MXU -> Vector units
//     - One operation at a time (no thread blocks)
//
//  3. **ANE Schedule Replayer** (Apple Neural Engine)
//     - Compiler generates a complete execution schedule at compile time
//     - The "distributor" simply replays the schedule
//     - No dynamic scheduling decisions -- everything is predetermined

import (
	"fmt"

	computeunit "github.com/adhithyan15/coding-adventures/code/packages/go/compute-unit"
)

// =========================================================================
// GPU Block Distributor
// =========================================================================

// GPUWorkDistributor distributes thread blocks to compute units.
//
// Used by NVIDIA (GigaThread Engine), AMD (Command Processor),
// and Intel (Command Streamer). The same algorithm works for all
// three -- they differ only in CU-level resource limits.
//
// Distribution policies:
//
//	round_robin:  Cycle through CUs evenly. Fair, simple.
//	fill_first:   Fill one CU before moving to next. Max occupancy per CU.
//	least_loaded: Assign to CU with fewest active warps. Best balance.
type GPUWorkDistributor struct {
	cus             []computeunit.ComputeUnit
	policy          string
	pending         []computeunit.WorkItem
	rrIndex         int // For round-robin policy
	totalDispatched int
}

// NewGPUWorkDistributor creates a new GPU work distributor.
func NewGPUWorkDistributor(cus []computeunit.ComputeUnit, policy string) *GPUWorkDistributor {
	return &GPUWorkDistributor{
		cus:    cus,
		policy: policy,
	}
}

// PendingCount returns the number of blocks waiting to be assigned.
func (d *GPUWorkDistributor) PendingCount() int {
	return len(d.pending)
}

// TotalDispatched returns the total blocks dispatched so far.
func (d *GPUWorkDistributor) TotalDispatched() int {
	return d.totalDispatched
}

// SubmitKernel decomposes a kernel into thread blocks and queues them.
//
// Each thread block becomes a WorkItem. The block's position in
// the grid is encoded in the WorkID (we use a linear index).
//
// Grid Linearization:
//
//	A 3D grid (gx, gy, gz) is linearized:
//	    block_id = bz * gx * gy + by * gx + bx
//
// This is the same order CUDA uses for blockIdx.
func (d *GPUWorkDistributor) SubmitKernel(kernel KernelDescriptor) {
	for blockID := 0; blockID < kernel.TotalBlocks(); blockID++ {
		work := computeunit.WorkItem{
			WorkID:             blockID,
			Program:            kernel.Program,
			ThreadCount:        kernel.ThreadsPerBlock(),
			RegistersPerThread: kernel.RegistersPerThread,
			SharedMemBytes:     kernel.SharedMemBytes,
		}
		d.pending = append(d.pending, work)
	}
}

// Step tries to assign pending blocks to available CUs.
//
// Returns a list of human-readable assignment descriptions.
// Each entry looks like: "Block 42 -> SM 7"
//
// Algorithm:
//
//	For each CU (in policy order):
//	    While there are pending blocks:
//	        Try to dispatch the next block to this CU
//	        If CU rejects it (ResourceError), move to next CU
//	        If CU accepts it, log the assignment
func (d *GPUWorkDistributor) Step() []string {
	if len(d.pending) == 0 {
		return nil
	}

	var assignments []string
	order := d.cuOrder()

	for _, cu := range order {
		for len(d.pending) > 0 {
			block := d.pending[0]
			err := cu.Dispatch(block)
			if err != nil {
				// CU can't accept this block (full) -- try next CU
				break
			}
			d.pending = d.pending[1:]
			d.totalDispatched++
			assignments = append(assignments,
				fmt.Sprintf("Block %d -> %s", block.WorkID, cu.Name()))
		}
	}

	return assignments
}

// cuOrder returns CUs in the order dictated by the policy.
//
//	round_robin:  Start from rrIndex, wrap around.
//	fill_first:   Just return in order (fill CU 0 first, then CU 1, ...).
//	least_loaded: Sort by idle status (idle CUs first).
func (d *GPUWorkDistributor) cuOrder() []computeunit.ComputeUnit {
	n := len(d.cus)
	if n == 0 {
		return nil
	}

	if d.policy == "fill_first" {
		result := make([]computeunit.ComputeUnit, n)
		copy(result, d.cus)
		return result
	}

	if d.policy == "least_loaded" {
		// Idle CUs first, then busy ones
		result := make([]computeunit.ComputeUnit, n)
		copy(result, d.cus)
		// Stable sort: idle CUs first
		idleFirst := make([]computeunit.ComputeUnit, 0, n)
		busyOnes := make([]computeunit.ComputeUnit, 0, n)
		for _, cu := range result {
			if cu.Idle() {
				idleFirst = append(idleFirst, cu)
			} else {
				busyOnes = append(busyOnes, cu)
			}
		}
		return append(idleFirst, busyOnes...)
	}

	// Default: round_robin
	ordered := make([]computeunit.ComputeUnit, n)
	for i := 0; i < n; i++ {
		idx := (d.rrIndex + i) % n
		ordered[i] = d.cus[idx]
	}
	d.rrIndex = (d.rrIndex + 1) % n
	return ordered
}

// Reset clears all pending work and resets counters.
func (d *GPUWorkDistributor) Reset() {
	d.pending = nil
	d.rrIndex = 0
	d.totalDispatched = 0
}

// =========================================================================
// TPU Sequencer
// =========================================================================

// TileOperation is a single tile operation in the TPU pipeline.
type TileOperation struct {
	TileID          int
	Operation       string // "matmul", "add", "relu", etc.
	InputData       [][]float64
	WeightData      [][]float64
	Status          string // "pending", "scalar", "mxu", "vector", "done"
	CyclesRemaining int
}

// TPUSequencer orchestrates operations through Scalar + Vector + MXU units.
//
// # TPU Execution Pipeline
//
// The TPU processes operations through a three-stage pipeline:
//
//	Scalar Unit -> MXU -> Vector Unit
//
// Stage 1 (Scalar): Prepare addresses, loop counters, control flow.
// Stage 2 (MXU):    The heavy lifting -- matrix multiply on the systolic array.
// Stage 3 (Vector): Post-processing -- activation functions, normalization.
//
// These three stages overlap: while the MXU crunches tile N, the Vector
// unit processes tile N-1, and the Scalar unit prepares tile N+1.
//
//	Time ->
//	Scalar: [tile 0] [tile 1] [tile 2] [tile 3] ...
//	MXU:           [tile 0] [tile 1] [tile 2] ...
//	Vector:               [tile 0] [tile 1] ...
type TPUSequencer struct {
	mxu            computeunit.ComputeUnit
	mxuSize        int
	vectorWidth    int
	scalarLatency  int
	mxuLatency     int
	vectorLatency  int

	pending         []*TileOperation
	scalarTile      *TileOperation
	mxuTile         *TileOperation
	vectorTile      *TileOperation
	completed       []*TileOperation
	totalDispatched int
}

// NewTPUSequencer creates a new TPU sequencer.
func NewTPUSequencer(
	mxu computeunit.ComputeUnit,
	mxuSize int,
	vectorWidth int,
	scalarLatency int,
	mxuLatency int,
	vectorLatency int,
) *TPUSequencer {
	return &TPUSequencer{
		mxu:           mxu,
		mxuSize:       mxuSize,
		vectorWidth:   vectorWidth,
		scalarLatency: scalarLatency,
		mxuLatency:    mxuLatency,
		vectorLatency: vectorLatency,
	}
}

// PendingCount returns the number of tiles waiting to be processed.
func (s *TPUSequencer) PendingCount() int {
	return len(s.pending)
}

// TotalDispatched returns the total tiles dispatched so far.
func (s *TPUSequencer) TotalDispatched() int {
	return s.totalDispatched
}

// SubmitOperation tiles a large operation and queues the tiles.
//
// If the input matrix is 256x256 but the MXU is 128x128, we need
// to split it into 4 tiles:
//
//	Tile 0: rows 0-127,   cols 0-127
//	Tile 1: rows 0-127,   cols 128-255
//	Tile 2: rows 128-255, cols 0-127
//	Tile 3: rows 128-255, cols 128-255
func (s *TPUSequencer) SubmitOperation(kernel KernelDescriptor) {
	inputData := kernel.InputData
	if inputData == nil {
		inputData = [][]float64{{0.0}}
	}
	weightData := kernel.WeightData
	if weightData == nil {
		weightData = [][]float64{{0.0}}
	}

	rows := len(inputData)
	cols := 1
	if len(weightData) > 0 && len(weightData[0]) > 0 {
		cols = len(weightData[0])
	}
	mxu := s.mxuSize

	numRowTiles := max(1, (rows+mxu-1)/mxu)
	numColTiles := max(1, (cols+mxu-1)/mxu)

	tileID := 0
	for rt := 0; rt < numRowTiles; rt++ {
		for ct := 0; ct < numColTiles; ct++ {
			_ = rt
			_ = ct
			tile := &TileOperation{
				TileID:          tileID,
				Operation:       kernel.Operation,
				InputData:       inputData,
				WeightData:      weightData,
				Status:          "pending",
				CyclesRemaining: s.scalarLatency,
			}
			if tile.Operation == "" {
				tile.Operation = "matmul"
			}
			s.pending = append(s.pending, tile)
			tileID++
		}
	}
}

// Step advances the pipeline by one cycle.
//
// Returns descriptions of what happened this cycle.
func (s *TPUSequencer) Step() []string {
	var actions []string

	// Vector stage: finish processing
	if s.vectorTile != nil {
		s.vectorTile.CyclesRemaining--
		if s.vectorTile.CyclesRemaining <= 0 {
			s.vectorTile.Status = "done"
			s.completed = append(s.completed, s.vectorTile)
			actions = append(actions,
				fmt.Sprintf("Vector: completed tile %d", s.vectorTile.TileID))
			s.vectorTile = nil
		}
	}

	// MXU stage: process matrix multiply
	if s.mxuTile != nil {
		s.mxuTile.CyclesRemaining--
		if s.mxuTile.CyclesRemaining <= 0 {
			s.mxuTile.Status = "vector"
			s.mxuTile.CyclesRemaining = s.vectorLatency
			// Move to vector stage (if free)
			if s.vectorTile == nil {
				s.vectorTile = s.mxuTile
				s.mxuTile = nil
				actions = append(actions,
					fmt.Sprintf("MXU -> Vector: tile %d", s.vectorTile.TileID))
			}
		}
	}

	// Scalar stage: prepare next tile
	if s.scalarTile != nil {
		s.scalarTile.CyclesRemaining--
		if s.scalarTile.CyclesRemaining <= 0 {
			s.scalarTile.Status = "mxu"
			s.scalarTile.CyclesRemaining = s.mxuLatency
			// Move to MXU stage (if free)
			if s.mxuTile == nil {
				s.mxuTile = s.scalarTile
				s.scalarTile = nil
				s.totalDispatched++
				actions = append(actions,
					fmt.Sprintf("Scalar -> MXU: tile %d", s.mxuTile.TileID))
			}
		}
	}

	// Feed from pending queue to scalar stage
	if s.scalarTile == nil && len(s.pending) > 0 {
		s.scalarTile = s.pending[0]
		s.pending = s.pending[1:]
		s.scalarTile.Status = "scalar"
		s.scalarTile.CyclesRemaining = s.scalarLatency
		actions = append(actions,
			fmt.Sprintf("Scalar: started tile %d", s.scalarTile.TileID))
	}

	return actions
}

// Idle returns true when all tiles are processed.
func (s *TPUSequencer) Idle() bool {
	return len(s.pending) == 0 &&
		s.scalarTile == nil &&
		s.mxuTile == nil &&
		s.vectorTile == nil
}

// Reset clears all state.
func (s *TPUSequencer) Reset() {
	s.pending = nil
	s.scalarTile = nil
	s.mxuTile = nil
	s.vectorTile = nil
	s.completed = nil
	s.totalDispatched = 0
}

// =========================================================================
// ANE Schedule Replayer
// =========================================================================

// ScheduleEntry is one step in a compiler-generated ANE schedule.
//
// The CoreML compiler pre-determines everything:
//   - Which core processes which tile
//   - When DMA loads happen
//   - When DMA stores happen
//   - The exact order of operations
type ScheduleEntry struct {
	Cycle       int
	Action      string // "dma_load", "compute", "dma_store", "activate"
	CoreID      int
	Description string
	Data        [][]float64
	Weights     [][]float64
}

// ANEScheduleReplayer replays a compiler-generated execution schedule.
//
// # Why No Dynamic Scheduling?
//
// Unlike GPUs (which have hardware schedulers that decide at runtime
// which warp to execute), the Apple Neural Engine relies entirely on
// the compiler. The CoreML compiler analyzes the neural network graph,
// determines the optimal tiling strategy, generates DMA transfer
// schedules, and produces a fixed execution plan.
//
// This makes the hardware simpler (no complex scheduler) and more
// power-efficient (no scheduling overhead), but less flexible --
// the ANE can only run workloads the compiler knows how to schedule.
//
// Schedule structure:
//
//	Step 0: DMA load input tile 0 -> Core 0 SRAM
//	Step 1: DMA load weights -> Core 0 SRAM
//	Step 2: Core 0 compute (MAC array)
//	Step 3: Core 0 activate (ReLU)
//	Step 4: DMA store result -> output buffer
//	Step 5: DMA load input tile 1 -> Core 1 SRAM (overlaps with step 2-4!)
type ANEScheduleReplayer struct {
	cus              []computeunit.ComputeUnit
	dmaLatency       int
	computeLatency   int
	activateLatency  int

	schedule        []ScheduleEntry
	currentStep     int
	totalDispatched int
}

// NewANEScheduleReplayer creates a new ANE schedule replayer.
func NewANEScheduleReplayer(
	cus []computeunit.ComputeUnit,
	dmaLatency int,
	computeLatency int,
	activateLatency int,
) *ANEScheduleReplayer {
	return &ANEScheduleReplayer{
		cus:             cus,
		dmaLatency:      dmaLatency,
		computeLatency:  computeLatency,
		activateLatency: activateLatency,
	}
}

// PendingCount returns the number of schedule steps remaining.
func (r *ANEScheduleReplayer) PendingCount() int {
	remaining := len(r.schedule) - r.currentStep
	if remaining < 0 {
		return 0
	}
	return remaining
}

// TotalDispatched returns the total operations dispatched so far.
func (r *ANEScheduleReplayer) TotalDispatched() int {
	return r.totalDispatched
}

// SubmitOperation generates a schedule from a kernel descriptor.
//
// The compiler (us, acting as the compiler) determines:
//  1. How to tile the input across available cores
//  2. When to load data via DMA
//  3. When each core computes
//  4. When to apply activation functions
//  5. When to store results via DMA
func (r *ANEScheduleReplayer) SubmitOperation(kernel KernelDescriptor) {
	inputData := kernel.InputData
	if inputData == nil {
		inputData = [][]float64{{0.0}}
	}
	weightData := kernel.WeightData
	if weightData == nil {
		weightData = [][]float64{{0.0}}
	}

	numCores := len(r.cus)
	rows := len(inputData)

	cycle := 0
	limit := min(numCores, rows)
	for coreID := 0; coreID < limit; coreID++ {
		// DMA load input
		r.schedule = append(r.schedule, ScheduleEntry{
			Cycle:       cycle,
			Action:      "dma_load",
			CoreID:      coreID,
			Description: fmt.Sprintf("DMA load input tile -> Core %d", coreID),
			Data:        inputData,
		})
		cycle += r.dmaLatency

		// DMA load weights
		r.schedule = append(r.schedule, ScheduleEntry{
			Cycle:       cycle,
			Action:      "dma_load",
			CoreID:      coreID,
			Description: fmt.Sprintf("DMA load weights -> Core %d", coreID),
			Weights:     weightData,
		})
		cycle += r.dmaLatency

		// Compute
		r.schedule = append(r.schedule, ScheduleEntry{
			Cycle:       cycle,
			Action:      "compute",
			CoreID:      coreID,
			Description: fmt.Sprintf("Core %d: MAC array compute", coreID),
		})
		cycle += r.computeLatency

		// Activate
		r.schedule = append(r.schedule, ScheduleEntry{
			Cycle:       cycle,
			Action:      "activate",
			CoreID:      coreID,
			Description: fmt.Sprintf("Core %d: activation (ReLU)", coreID),
		})
		cycle += r.activateLatency

		// DMA store
		r.schedule = append(r.schedule, ScheduleEntry{
			Cycle:       cycle,
			Action:      "dma_store",
			CoreID:      coreID,
			Description: fmt.Sprintf("DMA store result from Core %d", coreID),
		})
		cycle += r.dmaLatency
	}
}

// Step executes the next step in the pre-computed schedule.
//
// Returns descriptions of what happened this cycle.
func (r *ANEScheduleReplayer) Step() []string {
	if r.currentStep >= len(r.schedule) {
		return nil
	}

	entry := r.schedule[r.currentStep]
	r.currentStep++
	r.totalDispatched++

	return []string{entry.Description}
}

// Idle returns true when the entire schedule has been replayed.
func (r *ANEScheduleReplayer) Idle() bool {
	return r.currentStep >= len(r.schedule)
}

// Reset clears the schedule and resets.
func (r *ANEScheduleReplayer) Reset() {
	r.schedule = nil
	r.currentStep = 0
	r.totalDispatched = 0
}
