//! Google TPU -- device simulator with Scalar/Vector/MXU pipeline.
//!
//! # TPU Architecture
//!
//! The TPU is fundamentally different from GPUs. Instead of thousands of
//! small cores executing thread programs, the TPU has:
//!
//! 1. **One large MXU** (Matrix Multiply Unit) -- a 128x128 systolic array
//!    that multiplies entire matrices in hardware.
//! 2. **A vector unit** -- handles element-wise operations (activation
//!    functions, normalization, softmax).
//! 3. **A scalar unit** -- handles control flow, address calculation.
//!
//! These form a **pipeline**: while the MXU crunches tile N, the vector
//! unit post-processes tile N-1, and the scalar unit prepares tile N+1.
//!
//! ```text
//! +--------------------------------------------+
//! |              Google TPU                     |
//! |  +--------------------------------------+  |
//! |  |    Sequencer (control unit)          |  |
//! |  +-----+----------+----------+---------+  |
//! |        |          |          |             |
//! |  +-----+-+ +------+---+ +---+--------+   |
//! |  |Scalar | | Vector   | |    MXU     |   |
//! |  | Unit  | |  Unit    | |  (128x128) |   |
//! |  +-------+ +----------+ +------------+   |
//! |                                           |
//! |  +--------------------------------------+ |
//! |  |      HBM2e (32 GB, 1.2 TB/s)        | |
//! |  +--------------------------------------+ |
//! +--------------------------------------------+
//! ```
//!
//! # No Thread Blocks
//!
//! TPUs don't have threads, warps, or thread blocks. The programming model
//! is completely different:
//!
//! ```text
//! GPU: "Run this program on 65,536 threads"
//! TPU: "Multiply this 1024x512 matrix by this 512x768 matrix"
//! ```

use compute_unit::{ComputeUnit as ComputeUnitTrait, MatrixMultiplyUnit, MXUConfig};

use crate::global_memory::SimpleGlobalMemory;
use crate::protocols::{
    AcceleratorDevice, DeviceConfig, DeviceStats, DeviceTrace, KernelDescriptor,
};
use crate::work_distributor::TPUSequencer;

/// Google TPU device simulator.
///
/// Features a Scalar/Vector/MXU pipeline, HBM memory, and tile-based
/// execution of matrix operations.
pub struct GoogleTPU {
    /// Device configuration.
    config: DeviceConfig,
    /// The MXU compute unit.
    mxu: Box<dyn ComputeUnitTrait>,
    /// The sequencer orchestrates Scalar -> MXU -> Vector pipeline.
    sequencer: TPUSequencer,
    /// Global memory (HBM).
    global_memory: SimpleGlobalMemory,
    /// Current simulation cycle.
    cycle: u64,
    /// Total kernel launches.
    kernels_launched: u64,
}

impl GoogleTPU {
    /// Create a new Google TPU simulator.
    ///
    /// # Arguments
    ///
    /// * `config` - Full DeviceConfig (uses a small default if None).
    /// * `mxu_size` - Systolic array dimension (used only if config is None).
    pub fn new(config: Option<DeviceConfig>, mxu_size: usize) -> Self {
        let config = config.unwrap_or_else(|| DeviceConfig {
            name: format!("Google TPU (MXU {}x{})", mxu_size, mxu_size),
            architecture: "google_mxu".to_string(),
            num_compute_units: 1,
            l2_cache_size: 0,
            l2_cache_latency: 0,
            l2_cache_associativity: 0,
            l2_cache_line_size: 128,
            global_memory_size: 16 * 1024 * 1024,
            global_memory_bandwidth: 1200.0,
            global_memory_latency: 300,
            memory_channels: 4,
            host_bandwidth: 500.0,
            host_latency: 100,
            unified_memory: false,
            max_concurrent_kernels: 1,
            work_distribution_policy: "sequential".to_string(),
        });

        let mxu_config = MXUConfig::default();
        let mxu = Box::new(MatrixMultiplyUnit::new(mxu_config))
            as Box<dyn ComputeUnitTrait>;

        let sequencer = TPUSequencer::new(mxu_size, 5, 20, 10);

        let global_memory = SimpleGlobalMemory::new(
            config.global_memory_size,
            config.global_memory_bandwidth,
            config.global_memory_latency,
            config.memory_channels,
            128,
            config.host_bandwidth,
            config.host_latency as u64,
            config.unified_memory,
        );

        Self {
            config,
            mxu,
            sequencer,
            global_memory,
            cycle: 0,
            kernels_launched: 0,
        }
    }

    /// Access to device memory.
    pub fn global_memory(&self) -> &SimpleGlobalMemory {
        &self.global_memory
    }
}

impl AcceleratorDevice for GoogleTPU {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn config(&self) -> &DeviceConfig {
        &self.config
    }

    fn malloc(&mut self, size: usize) -> u64 {
        self.global_memory.allocate(size as u64, 256).unwrap_or(0)
    }

    fn free(&mut self, address: u64) {
        self.global_memory.free(address);
    }

    fn memcpy_host_to_device(&mut self, dst: u64, data: &[u8]) -> u64 {
        self.global_memory.copy_from_host(dst, data)
    }

    fn memcpy_device_to_host(&mut self, src: u64, size: usize) -> (Vec<u8>, u64) {
        self.global_memory.copy_to_host(src, size)
    }

    fn launch_kernel(&mut self, kernel: KernelDescriptor) {
        self.sequencer.submit_operation(&kernel);
        self.kernels_launched += 1;
    }

    fn step(&mut self) -> DeviceTrace {
        self.cycle += 1;

        // Advance the Scalar -> MXU -> Vector pipeline
        let seq_actions = self.sequencer.step();

        // Also step the MXU compute unit
        let cu_trace = self.mxu.step();

        DeviceTrace {
            cycle: self.cycle,
            device_name: self.config.name.clone(),
            distributor_actions: seq_actions,
            pending_blocks: self.sequencer.pending_count(),
            active_blocks: if self.sequencer.idle() { 0 } else { 1 },
            cu_traces: vec![cu_trace],
            l2_hits: 0,
            l2_misses: 0,
            memory_transactions: 0,
            memory_bandwidth_used: 0.0,
            total_active_warps: 0,
            device_occupancy: if self.sequencer.idle() { 0.0 } else { 1.0 },
            flops_this_cycle: 0,
        }
    }

    fn run(&mut self, max_cycles: usize) -> Vec<DeviceTrace> {
        let mut traces = Vec::new();
        for _ in 0..max_cycles {
            let trace = self.step();
            traces.push(trace);
            if self.idle() {
                break;
            }
        }
        traces
    }

    fn idle(&self) -> bool {
        self.sequencer.idle()
    }

    fn reset(&mut self) {
        self.mxu.reset();
        self.sequencer.reset();
        self.global_memory.reset();
        self.cycle = 0;
        self.kernels_launched = 0;
    }

    fn stats(&self) -> DeviceStats {
        DeviceStats {
            total_cycles: self.cycle,
            active_cycles: self.cycle,
            idle_cycles: 0,
            total_flops: 0,
            global_memory_stats: self.global_memory.stats(),
            total_kernels_launched: self.kernels_launched,
            total_blocks_dispatched: self.sequencer.total_dispatched(),
        }
    }
}
