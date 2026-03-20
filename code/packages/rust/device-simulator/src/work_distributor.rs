//! Work Distributor -- assigns work to compute units.
//!
//! # Three Distribution Strategies
//!
//! Different accelerator architectures distribute work in fundamentally
//! different ways. This module implements all three:
//!
//! 1. **GPU Block Distributor** (NVIDIA, AMD, Intel)
//!    - Takes a kernel launch with grid/block dimensions
//!    - Decomposes into thread blocks
//!    - Assigns blocks to compute units that have free resources
//!    - Continues assigning as CUs complete blocks (multi-wave)
//!
//! 2. **TPU Sequencer** (Google TPU)
//!    - Takes operations (matmul, add, relu, etc.)
//!    - Tiles large operations to fit the MXU
//!    - Pipelines through Scalar -> MXU -> Vector units
//!    - One operation at a time (no thread blocks)
//!
//! 3. **ANE Schedule Replayer** (Apple Neural Engine)
//!    - Compiler generates a complete execution schedule at compile time
//!    - The "distributor" simply replays the schedule
//!    - No dynamic scheduling decisions -- everything is predetermined
//!
//! # The GPU Work Distribution Problem
//!
//! A kernel launch like `matmul<<<grid(256,256), block(16,16)>>>` creates
//! 65,536 thread blocks. An H100 has 132 SMs, each holding ~8 blocks.
//! The GigaThread Engine must:
//!
//! 1. Queue all 65,536 blocks
//! 2. Assign ~1,056 to SMs in wave 1
//! 3. As SMs complete blocks, assign more from the queue
//! 4. Repeat for ~62 waves until all blocks are done

use std::collections::VecDeque;

use compute_unit::{ComputeUnit, WorkItem};

use crate::protocols::KernelDescriptor;

// =========================================================================
// GPU Block Distributor
// =========================================================================

/// Distributes thread blocks to compute units.
///
/// Used by NVIDIA (GigaThread Engine), AMD (Command Processor),
/// and Intel (Command Streamer). The same algorithm works for all
/// three -- they differ only in CU-level resource limits.
///
/// # Distribution Policies
///
/// ```text
/// round_robin:  Cycle through CUs evenly. Fair, simple.
/// fill_first:   Fill one CU before moving to next. Max occupancy per CU.
/// least_loaded: Assign to CU with fewest active warps. Best balance.
/// ```
pub struct GPUWorkDistributor {
    /// Pending thread blocks waiting to be assigned.
    pending: VecDeque<WorkItem>,
    /// Round-robin index for fair distribution.
    rr_index: usize,
    /// Total blocks dispatched so far.
    total_dispatched: u64,
    /// Distribution policy name.
    policy: String,
    /// Number of CUs we distribute to.
    num_cus: usize,
}

impl GPUWorkDistributor {
    /// Create a new GPU work distributor.
    ///
    /// # Arguments
    ///
    /// * `num_cus` - Number of compute units to distribute across.
    /// * `policy` - Distribution policy ("round_robin", "fill_first", "least_loaded").
    pub fn new(num_cus: usize, policy: &str) -> Self {
        Self {
            pending: VecDeque::new(),
            rr_index: 0,
            total_dispatched: 0,
            policy: policy.to_string(),
            num_cus,
        }
    }

    /// Number of blocks waiting to be assigned.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Total blocks dispatched so far.
    pub fn total_dispatched(&self) -> u64 {
        self.total_dispatched
    }

    /// Decompose a kernel into thread blocks and queue them.
    ///
    /// Each thread block becomes a WorkItem. The block's position in
    /// the grid is encoded in the work_id (linear index).
    pub fn submit_kernel(&mut self, kernel: &KernelDescriptor) {
        for block_id in 0..kernel.total_blocks() {
            let work = WorkItem {
                work_id: block_id,
                program: kernel.program.clone(),
                thread_count: kernel.threads_per_block(),
                registers_per_thread: kernel.registers_per_thread,
                shared_mem_bytes: kernel.shared_mem_bytes,
                ..WorkItem::default()
            };
            self.pending.push_back(work);
        }
    }

    /// Try to assign pending blocks to available CUs.
    ///
    /// Returns a list of human-readable assignment descriptions.
    ///
    /// The CU ordering depends on the policy:
    /// - round_robin: rotate through CUs evenly
    /// - fill_first: try CU 0 first, then CU 1, etc.
    /// - least_loaded: try idle CUs first
    pub fn step(&mut self, cus: &mut [Box<dyn ComputeUnit>]) -> Vec<String> {
        if self.pending.is_empty() {
            return Vec::new();
        }

        let mut assignments = Vec::new();
        let order = self.cu_order(cus);

        for cu_idx in order {
            while !self.pending.is_empty() {
                let block = self.pending.front().unwrap().clone();
                match cus[cu_idx].dispatch(block) {
                    Ok(()) => {
                        let dispatched = self.pending.pop_front().unwrap();
                        self.total_dispatched += 1;
                        assignments.push(format!(
                            "Block {} -> {}",
                            dispatched.work_id,
                            cus[cu_idx].name()
                        ));
                    }
                    Err(_) => {
                        // CU can't accept this block -- try next CU
                        break;
                    }
                }
            }
        }

        assignments
    }

    /// Return CU indices in the order dictated by the policy.
    fn cu_order(&mut self, cus: &[Box<dyn ComputeUnit>]) -> Vec<usize> {
        let n = self.num_cus.min(cus.len());
        if n == 0 {
            return Vec::new();
        }

        match self.policy.as_str() {
            "fill_first" => (0..n).collect(),
            "least_loaded" => {
                let mut indices: Vec<usize> = (0..n).collect();
                indices.sort_by_key(|&i| if cus[i].idle() { 0 } else { 1 });
                indices
            }
            _ => {
                // round_robin
                let mut ordered = Vec::with_capacity(n);
                for i in 0..n {
                    ordered.push((self.rr_index + i) % n);
                }
                self.rr_index = (self.rr_index + 1) % n;
                ordered
            }
        }
    }

    /// Clear all pending work and reset counters.
    pub fn reset(&mut self) {
        self.pending.clear();
        self.rr_index = 0;
        self.total_dispatched = 0;
    }
}

// =========================================================================
// TileOperation -- a single tile in the TPU pipeline
// =========================================================================

/// A single tile operation in the TPU pipeline.
///
/// The TPU processes tiles through a three-stage pipeline:
/// Scalar (prepare) -> MXU (matrix multiply) -> Vector (post-process).
#[derive(Debug, Clone)]
pub struct TileOperation {
    /// Unique tile identifier.
    pub tile_id: usize,
    /// Operation name ("matmul", "add", "relu", etc.).
    pub operation: String,
    /// Current status: "pending", "scalar", "mxu", "vector", "done".
    pub status: String,
    /// Cycles remaining in the current stage.
    pub cycles_remaining: i64,
}

// =========================================================================
// TPU Sequencer
// =========================================================================

/// Orchestrates operations through the Scalar + Vector + MXU pipeline.
///
/// # TPU Execution Pipeline
///
/// ```text
/// Scalar Unit -> MXU -> Vector Unit
/// ```
///
/// Stage 1 (Scalar): Prepare addresses, loop counters, control flow.
/// Stage 2 (MXU):    Matrix multiply on the systolic array.
/// Stage 3 (Vector): Post-processing -- activation functions, normalization.
///
/// These three stages overlap: while the MXU crunches tile N, the Vector
/// unit processes tile N-1, and the Scalar unit prepares tile N+1.
///
/// ```text
/// Time ->
/// Scalar: [tile 0] [tile 1] [tile 2] ...
/// MXU:           [tile 0] [tile 1] ...
/// Vector:               [tile 0] ...
/// ```
pub struct TPUSequencer {
    /// Pending tiles waiting to enter the pipeline.
    pending: VecDeque<TileOperation>,
    /// Tile currently in the scalar stage.
    scalar_tile: Option<TileOperation>,
    /// Tile currently in the MXU stage.
    mxu_tile: Option<TileOperation>,
    /// Tile currently in the vector stage.
    vector_tile: Option<TileOperation>,
    /// Completed tiles.
    completed: Vec<TileOperation>,
    /// Total tiles dispatched through the pipeline.
    total_dispatched: u64,

    /// Systolic array dimension (e.g., 128 for 128x128).
    mxu_size: usize,
    /// Cycles for scalar setup per tile.
    scalar_latency: i64,
    /// Cycles for MXU processing per tile.
    mxu_latency: i64,
    /// Cycles for vector post-processing per tile.
    vector_latency: i64,
}

impl TPUSequencer {
    /// Create a new TPU sequencer.
    pub fn new(
        mxu_size: usize,
        scalar_latency: i64,
        mxu_latency: i64,
        vector_latency: i64,
    ) -> Self {
        Self {
            pending: VecDeque::new(),
            scalar_tile: None,
            mxu_tile: None,
            vector_tile: None,
            completed: Vec::new(),
            total_dispatched: 0,
            mxu_size,
            scalar_latency,
            mxu_latency,
            vector_latency,
        }
    }

    /// Number of tiles waiting to be processed.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Total tiles dispatched so far.
    pub fn total_dispatched(&self) -> u64 {
        self.total_dispatched
    }

    /// Tile a large operation and queue the tiles.
    ///
    /// If the input matrix is 256x256 but the MXU is 128x128, we need
    /// 4 tiles (2 row tiles x 2 column tiles).
    pub fn submit_operation(&mut self, kernel: &KernelDescriptor) {
        let default_input = [vec![0.0]];
        let default_weight = [vec![0.0]];
        let input_data = kernel.input_data.as_deref().unwrap_or(&default_input);
        let weight_data = kernel.weight_data.as_deref().unwrap_or(&default_weight);

        let rows = input_data.len();
        let cols = if weight_data.is_empty() { 1 } else { weight_data[0].len() };
        let mxu = self.mxu_size;

        let num_row_tiles = ((rows + mxu - 1) / mxu).max(1);
        let num_col_tiles = ((cols + mxu - 1) / mxu).max(1);

        let mut tile_id = 0;
        for _rt in 0..num_row_tiles {
            for _ct in 0..num_col_tiles {
                let tile = TileOperation {
                    tile_id,
                    operation: if kernel.operation.is_empty() {
                        "matmul".to_string()
                    } else {
                        kernel.operation.clone()
                    },
                    status: "pending".to_string(),
                    cycles_remaining: self.scalar_latency,
                };
                self.pending.push_back(tile);
                tile_id += 1;
            }
        }
    }

    /// Advance the pipeline by one cycle.
    ///
    /// Returns descriptions of what happened this cycle.
    pub fn step(&mut self) -> Vec<String> {
        let mut actions = Vec::new();

        // Vector stage: finish processing
        if let Some(ref mut tile) = self.vector_tile {
            tile.cycles_remaining -= 1;
            if tile.cycles_remaining <= 0 {
                let mut done_tile = self.vector_tile.take().unwrap();
                done_tile.status = "done".to_string();
                actions.push(format!("Vector: completed tile {}", done_tile.tile_id));
                self.completed.push(done_tile);
            }
        }

        // MXU stage: process matrix multiply
        if let Some(ref mut tile) = self.mxu_tile {
            tile.cycles_remaining -= 1;
            if tile.cycles_remaining <= 0 {
                if self.vector_tile.is_none() {
                    let mut moved_tile = self.mxu_tile.take().unwrap();
                    moved_tile.status = "vector".to_string();
                    moved_tile.cycles_remaining = self.vector_latency;
                    actions.push(format!("MXU -> Vector: tile {}", moved_tile.tile_id));
                    self.vector_tile = Some(moved_tile);
                }
            }
        }

        // Scalar stage: prepare next tile
        if let Some(ref mut tile) = self.scalar_tile {
            tile.cycles_remaining -= 1;
            if tile.cycles_remaining <= 0 {
                if self.mxu_tile.is_none() {
                    let mut moved_tile = self.scalar_tile.take().unwrap();
                    moved_tile.status = "mxu".to_string();
                    moved_tile.cycles_remaining = self.mxu_latency;
                    self.total_dispatched += 1;
                    actions.push(format!("Scalar -> MXU: tile {}", moved_tile.tile_id));
                    self.mxu_tile = Some(moved_tile);
                }
            }
        }

        // Feed from pending queue to scalar stage
        if self.scalar_tile.is_none() {
            if let Some(mut tile) = self.pending.pop_front() {
                tile.status = "scalar".to_string();
                tile.cycles_remaining = self.scalar_latency;
                actions.push(format!("Scalar: started tile {}", tile.tile_id));
                self.scalar_tile = Some(tile);
            }
        }

        actions
    }

    /// True when all tiles are processed.
    pub fn idle(&self) -> bool {
        self.pending.is_empty()
            && self.scalar_tile.is_none()
            && self.mxu_tile.is_none()
            && self.vector_tile.is_none()
    }

    /// Clear all state.
    pub fn reset(&mut self) {
        self.pending.clear();
        self.scalar_tile = None;
        self.mxu_tile = None;
        self.vector_tile = None;
        self.completed.clear();
        self.total_dispatched = 0;
    }
}

// =========================================================================
// ScheduleEntry -- one step in a compiler-generated ANE schedule
// =========================================================================

/// One step in a compiler-generated ANE schedule.
///
/// The CoreML compiler pre-determines everything:
/// - Which core processes which tile
/// - When DMA loads happen
/// - When DMA stores happen
/// - The exact order of operations
#[derive(Debug, Clone)]
pub struct ScheduleEntry {
    /// Cycle when this entry should execute.
    pub cycle: u64,
    /// Action type: "dma_load", "compute", "dma_store", "activate".
    pub action: String,
    /// Which core processes this entry (-1 for DMA-only).
    pub core_id: i32,
    /// Human-readable description.
    pub description: String,
}

// =========================================================================
// ANE Schedule Replayer
// =========================================================================

/// Replays a compiler-generated execution schedule.
///
/// Unlike GPUs (which have hardware schedulers that decide at runtime
/// which warp to execute), the Apple Neural Engine relies entirely on
/// the compiler. The CoreML compiler analyzes the neural network graph,
/// determines the optimal tiling strategy, generates DMA transfer
/// schedules, and produces a fixed execution plan.
///
/// This makes the hardware simpler and more power-efficient, but less
/// flexible -- the ANE can only run workloads the compiler supports.
pub struct ANEScheduleReplayer {
    /// The complete schedule to replay.
    schedule: Vec<ScheduleEntry>,
    /// Current position in the schedule.
    current_step: usize,
    /// Total operations dispatched so far.
    total_dispatched: u64,
    /// Number of compute units.
    num_cus: usize,
    /// DMA transfer latency in cycles.
    dma_latency: u64,
    /// Compute (MAC array) latency in cycles.
    compute_latency: u64,
    /// Activation function latency in cycles.
    activate_latency: u64,
}

impl ANEScheduleReplayer {
    /// Create a new ANE schedule replayer.
    pub fn new(
        num_cus: usize,
        dma_latency: u64,
        compute_latency: u64,
        activate_latency: u64,
    ) -> Self {
        Self {
            schedule: Vec::new(),
            current_step: 0,
            total_dispatched: 0,
            num_cus,
            dma_latency,
            compute_latency,
            activate_latency,
        }
    }

    /// Number of schedule steps remaining.
    pub fn pending_count(&self) -> usize {
        if self.current_step >= self.schedule.len() {
            0
        } else {
            self.schedule.len() - self.current_step
        }
    }

    /// Total operations dispatched so far.
    pub fn total_dispatched(&self) -> u64 {
        self.total_dispatched
    }

    /// Generate a schedule from a kernel descriptor.
    ///
    /// The compiler (us, acting as the compiler) determines:
    /// 1. How to tile the input across available cores
    /// 2. When to load data via DMA
    /// 3. When each core computes
    /// 4. When to apply activation functions
    /// 5. When to store results via DMA
    pub fn submit_operation(&mut self, kernel: &KernelDescriptor) {
        let default_input = [vec![0.0]];
        let input_data = kernel.input_data.as_deref().unwrap_or(&default_input);

        let rows = input_data.len();
        let num_cores = self.num_cus;

        let mut cycle = 0u64;
        for core_id in 0..num_cores.min(rows) {
            // DMA load input
            self.schedule.push(ScheduleEntry {
                cycle,
                action: "dma_load".to_string(),
                core_id: core_id as i32,
                description: format!("DMA load input tile -> Core {}", core_id),
            });
            cycle += self.dma_latency;

            // DMA load weights
            self.schedule.push(ScheduleEntry {
                cycle,
                action: "dma_load".to_string(),
                core_id: core_id as i32,
                description: format!("DMA load weights -> Core {}", core_id),
            });
            cycle += self.dma_latency;

            // Compute
            self.schedule.push(ScheduleEntry {
                cycle,
                action: "compute".to_string(),
                core_id: core_id as i32,
                description: format!("Core {}: MAC array compute", core_id),
            });
            cycle += self.compute_latency;

            // Activate
            self.schedule.push(ScheduleEntry {
                cycle,
                action: "activate".to_string(),
                core_id: core_id as i32,
                description: format!("Core {}: activation (ReLU)", core_id),
            });
            cycle += self.activate_latency;

            // DMA store
            self.schedule.push(ScheduleEntry {
                cycle,
                action: "dma_store".to_string(),
                core_id: core_id as i32,
                description: format!("DMA store result from Core {}", core_id),
            });
            cycle += self.dma_latency;
        }
    }

    /// Execute the next step in the pre-computed schedule.
    ///
    /// Returns descriptions of what happened this cycle.
    pub fn step(&mut self) -> Vec<String> {
        if self.current_step >= self.schedule.len() {
            return Vec::new();
        }

        let entry = &self.schedule[self.current_step];
        let desc = entry.description.clone();
        self.current_step += 1;
        self.total_dispatched += 1;

        vec![desc]
    }

    /// True when the entire schedule has been replayed.
    pub fn idle(&self) -> bool {
        self.current_step >= self.schedule.len()
    }

    /// Clear the schedule and reset.
    pub fn reset(&mut self) {
        self.schedule.clear();
        self.current_step = 0;
        self.total_dispatched = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_tpu_sequencer_idle_initially() {
        let seq = TPUSequencer::new(128, 5, 20, 10);
        assert!(seq.idle());
        assert_eq!(seq.pending_count(), 0);
    }

    #[test]
    fn test_tpu_sequencer_submit_and_run() {
        let mut seq = TPUSequencer::new(2, 2, 3, 2);
        let kernel = KernelDescriptor {
            operation: "matmul".to_string(),
            input_data: Some(vec![vec![1.0, 2.0], vec![3.0, 4.0]]),
            weight_data: Some(vec![vec![5.0, 6.0], vec![7.0, 8.0]]),
            ..KernelDescriptor::default()
        };
        seq.submit_operation(&kernel);
        assert!(!seq.idle());

        // Run until idle
        for _ in 0..100 {
            seq.step();
            if seq.idle() {
                break;
            }
        }
        assert!(seq.idle());
    }

    #[test]
    fn test_ane_replayer_idle_initially() {
        let replayer = ANEScheduleReplayer::new(4, 10, 20, 5);
        assert!(replayer.idle());
        assert_eq!(replayer.pending_count(), 0);
    }

    #[test]
    fn test_ane_replayer_submit_and_run() {
        let mut replayer = ANEScheduleReplayer::new(2, 10, 20, 5);
        let kernel = KernelDescriptor {
            operation: "matmul".to_string(),
            input_data: Some(vec![vec![1.0, 2.0], vec![3.0, 4.0]]),
            weight_data: Some(vec![vec![5.0, 6.0], vec![7.0, 8.0]]),
            ..KernelDescriptor::default()
        };
        replayer.submit_operation(&kernel);
        assert!(!replayer.idle());

        // Run until idle
        for _ in 0..100 {
            replayer.step();
            if replayer.idle() {
                break;
            }
        }
        assert!(replayer.idle());
        assert!(replayer.total_dispatched() > 0);
    }

    #[test]
    fn test_ane_replayer_reset() {
        let mut replayer = ANEScheduleReplayer::new(2, 10, 20, 5);
        let kernel = KernelDescriptor {
            operation: "matmul".to_string(),
            input_data: Some(vec![vec![1.0]]),
            weight_data: Some(vec![vec![1.0]]),
            ..KernelDescriptor::default()
        };
        replayer.submit_operation(&kernel);
        replayer.step();
        replayer.reset();
        assert!(replayer.idle());
        assert_eq!(replayer.total_dispatched(), 0);
    }

    #[test]
    fn test_tpu_sequencer_reset() {
        let mut seq = TPUSequencer::new(2, 2, 3, 2);
        let kernel = KernelDescriptor {
            operation: "matmul".to_string(),
            input_data: Some(vec![vec![1.0]]),
            weight_data: Some(vec![vec![1.0]]),
            ..KernelDescriptor::default()
        };
        seq.submit_operation(&kernel);
        seq.step();
        seq.reset();
        assert!(seq.idle());
        assert_eq!(seq.total_dispatched(), 0);
    }
}
