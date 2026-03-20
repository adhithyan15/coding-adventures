//! NeuralEngineCore -- Apple ANE Core simulator.
//!
//! # What is the Apple Neural Engine?
//!
//! Apple's Neural Engine (ANE) is a dedicated neural network accelerator
//! found in every Apple chip since the A11 Bionic (2017). It's designed
//! for one thing: fast, power-efficient neural network inference.
//!
//! The ANE is the simplest compute unit in our family -- and that simplicity
//! is its strength. By removing hardware schedulers, branch predictors, and
//! general-purpose control logic, Apple can dedicate nearly all transistors
//! to MAC (multiply-accumulate) units and on-chip memory.
//!
//! # How ANE Differs from GPUs
//!
//! ```text
//! GPU (NVIDIA/AMD):                   ANE (Apple):
//! +----------------------------+     +----------------------------+
//! | Hardware scheduler         |     | NO hardware scheduler      |
//! | Runtime decisions          |     | All decisions at compile   |
//! | Branch prediction          |     | NO branches                |
//! | Dynamic register alloc     |     | Static buffer plan         |
//! | Flexible but complex       |     | Simple but rigid           |
//! | ~5 W per SM                |     | ~1 W per core              |
//! +----------------------------+     +----------------------------+
//! ```
//!
//! # Architecture
//!
//! Each ANE Core has:
//! - **MAC array**: 16 multiply-accumulate units (our default)
//! - **DMA engine**: transfers data between main memory and on-chip SRAM
//! - **On-chip SRAM**: 4 MB (fast, low-power local storage)
//! - **Activation pipeline**: hardware for ReLU, sigmoid, etc.
//! - **Buffers**: input, weight, and output buffers
//!
//! ```text
//! NeuralEngineCore
//! +---------------------------------------------------------------+
//! |  DMA Engine                                                    |
//! |  +-----------------------------------------------------------+ |
//! |  | Transfers data between main memory and on-chip SRAM        | |
//! |  +-----------------------------------------------------------+ |
//! |           |                    |                                |
//! |           v                    v                                |
//! |  +------------------+ +------------------+                     |
//! |  | Input Buffer     | | Weight Buffer    |                     |
//! |  | 128 KB           | | 512 KB           |                     |
//! |  +--------+---------+ +--------+---------+                     |
//! |           |                    |                                |
//! |           v                    v                                |
//! |  +---------------------------------------------+               |
//! |  | MAC Array (16 units)                         |               |
//! |  +---------------------------------------------+               |
//! |           |                                                    |
//! |           v                                                    |
//! |  +---------------------------------------------+               |
//! |  | Activation Pipeline (ReLU/sigmoid/tanh)      |               |
//! |  +---------------------------------------------+               |
//! |           |                                                    |
//! |           v                                                    |
//! |  +---------------------------------------------+               |
//! |  | Output Buffer (128 KB)                       |               |
//! |  +---------------------------------------------+               |
//! +---------------------------------------------------------------+
//! ```
//!
//! # Compiler-Scheduled Execution
//!
//! The ANE doesn't decide what to do at runtime. Instead, Apple's Core ML
//! compiler generates a complete schedule:
//!
//! ```text
//! Cycle 0-9:   DMA load input tile (10 elements/cycle)
//! Cycle 10-19: DMA load weight tile
//! Cycle 20:    MAC operation (16 parallel multiplies)
//! Cycle 21:    Reduce (sum MAC results)
//! Cycle 22:    Activate (apply ReLU)
//! Cycle 23:    DMA store output
//! ```

use std::collections::HashMap;
use std::fmt;

use fp_arithmetic::{FloatFormat, FP16, FP32};
use parallel_execution_engine::{MACArrayConfig, MACArrayEngine};
use parallel_execution_engine::protocols::ParallelExecutionEngine;

use crate::protocols::{
    Architecture, ComputeUnit, ComputeUnitTrace, ResourceError, WorkItem,
};

// ---------------------------------------------------------------------------
// ANECoreConfig -- configuration for an Apple Neural Engine Core
// ---------------------------------------------------------------------------

/// Configuration for an Apple Neural Engine Core.
///
/// ```text
/// Parameter          | A14 (iPhone 12) | M1          | M2
/// -------------------+-----------------+-------------+---------
/// Cores              | 16              | 16          | 16
/// TOPS               | 11              | 11          | 15.8
/// Format             | FP16/INT8       | FP16/INT8   | FP16/INT8
/// ```
#[derive(Debug, Clone)]
pub struct ANECoreConfig {
    /// MAC units per core.
    pub num_macs: usize,
    /// FP format for MAC operations.
    pub mac_format: FloatFormat,
    /// FP format for accumulation.
    pub accumulator_format: FloatFormat,
    /// On-chip SRAM in bytes.
    pub sram_size: usize,
    /// Activation (input) buffer in bytes.
    pub activation_buffer: usize,
    /// Weight buffer in bytes.
    pub weight_buffer: usize,
    /// Output buffer in bytes.
    pub output_buffer: usize,
    /// Elements transferred per DMA cycle.
    pub dma_bandwidth: usize,
}

impl Default for ANECoreConfig {
    fn default() -> Self {
        Self {
            num_macs: 16,
            mac_format: FP16,
            accumulator_format: FP32,
            sram_size: 4_194_304,
            activation_buffer: 131072,
            weight_buffer: 524288,
            output_buffer: 131072,
            dma_bandwidth: 10,
        }
    }
}

// ---------------------------------------------------------------------------
// NeuralEngineCore -- the main ANE Core simulator
// ---------------------------------------------------------------------------

/// Apple Neural Engine Core simulator.
///
/// Uses a MACArrayEngine from Layer 8 internally, adding DMA simulation,
/// activation pipeline, and compiler-generated schedule support.
///
/// # Execution Model
///
/// The ANE Core has no runtime scheduler. Instead, it follows a
/// compiler-generated schedule that specifies exactly what happens on
/// each cycle.
pub struct NeuralEngineCore {
    config: ANECoreConfig,
    cycle: u64,
    mac_engine: MACArrayEngine,
    idle_flag: bool,
    work_items: Vec<WorkItem>,
    result: Vec<Vec<f64>>,
}

impl NeuralEngineCore {
    /// Create a new Neural Engine Core with the given configuration.
    pub fn new(config: ANECoreConfig) -> Self {
        let mac_engine = MACArrayEngine::new(MACArrayConfig {
            num_macs: config.num_macs,
            input_buffer_size: (config.activation_buffer / 4).max(1024),
            weight_buffer_size: (config.weight_buffer / 4).max(4096),
            output_buffer_size: (config.output_buffer / 4).max(1024),
            float_format: FP32,
            accumulator_format: FP32,
            has_activation_unit: true,
        });
        Self {
            config,
            cycle: 0,
            mac_engine,
            idle_flag: true,
            work_items: Vec::new(),
            result: Vec::new(),
        }
    }

    /// Access to the ANE Core configuration.
    pub fn config(&self) -> &ANECoreConfig {
        &self.config
    }

    /// The result from the last computation.
    pub fn result(&self) -> &[Vec<f64>] {
        &self.result
    }

    /// Access to the underlying MAC array engine.
    pub fn mac_engine(&self) -> &MACArrayEngine {
        &self.mac_engine
    }

    /// Run a complete inference pass: matmul + activation function.
    ///
    /// # Inference Pipeline
    ///
    /// 1. DMA load inputs into activation buffer
    /// 2. DMA load weights into weight buffer
    /// 3. MAC: multiply input elements by weights
    /// 4. Reduce: sum MAC results
    /// 5. Activate: apply activation function
    /// 6. DMA store outputs
    pub fn run_inference(
        &mut self,
        inputs: &[Vec<f64>],
        weights: &[Vec<f64>],
        activation_fn: &str,
    ) -> Vec<Vec<f64>> {
        let mut result = Self::matmul(inputs, weights);

        if activation_fn != "none" {
            result = Self::apply_activation(&result, activation_fn);
        }

        self.result = result.clone();
        result
    }

    /// Process a single work item by performing matmul.
    fn process_work_item(&mut self, work: &WorkItem) {
        if let (Some(ref input_data), Some(ref weight_data)) =
            (&work.input_data, &work.weight_data)
        {
            self.result = Self::matmul(input_data, weight_data);
        } else {
            self.result = Vec::new();
        }
    }

    /// Perform matrix multiplication.
    ///
    /// For each element of the output matrix, we compute a dot product
    /// using the MAC array. This simulates how the ANE processes
    /// matrix multiplications tile by tile.
    fn matmul(a: &[Vec<f64>], b: &[Vec<f64>]) -> Vec<Vec<f64>> {
        if a.is_empty() || b.is_empty() {
            return Vec::new();
        }

        let m = a.len();
        let k = a[0].len();
        let n = b[0].len();

        let mut result = vec![vec![0.0; n]; m];
        for i in 0..m {
            for j in 0..n {
                let mut dot = 0.0;
                for kk in 0..k {
                    dot += a[i][kk] * b[kk][j];
                }
                result[i][j] = dot;
            }
        }
        result
    }

    /// Apply activation function element-wise.
    ///
    /// Simulates the ANE's dedicated activation pipeline hardware.
    fn apply_activation(matrix: &[Vec<f64>], fn_name: &str) -> Vec<Vec<f64>> {
        matrix
            .iter()
            .map(|row| {
                row.iter()
                    .map(|&val| match fn_name {
                        "relu" => val.max(0.0),
                        "sigmoid" => {
                            let clamped = val.max(-500.0).min(500.0);
                            1.0 / (1.0 + (-clamped).exp())
                        }
                        "tanh" => val.tanh(),
                        _ => val,
                    })
                    .collect()
            })
            .collect()
    }

    /// Produce a trace for when the ANE Core is idle.
    fn make_idle_trace(&self) -> ComputeUnitTrace {
        ComputeUnitTrace {
            cycle: self.cycle,
            unit_name: self.name().to_string(),
            architecture: self.architecture(),
            scheduler_action: "idle".to_string(),
            active_warps: 0,
            total_warps: 1,
            engine_traces: HashMap::new(),
            shared_memory_used: 0,
            shared_memory_total: self.config.sram_size,
            register_file_used: 0,
            register_file_total: self.config.num_macs,
            occupancy: 0.0,
            l1_hits: 0,
            l1_misses: 0,
        }
    }
}

impl ComputeUnit for NeuralEngineCore {
    fn name(&self) -> &str {
        "ANECore"
    }

    fn architecture(&self) -> Architecture {
        Architecture::AppleAneCore
    }

    fn idle(&self) -> bool {
        self.idle_flag
    }

    fn dispatch(&mut self, work: WorkItem) -> Result<(), ResourceError> {
        self.work_items.push(work);
        self.idle_flag = false;
        Ok(())
    }

    fn step(&mut self) -> ComputeUnitTrace {
        self.cycle += 1;

        if self.idle_flag || self.work_items.is_empty() {
            return self.make_idle_trace();
        }

        let work = self.work_items.remove(0);
        self.process_work_item(&work);

        if self.work_items.is_empty() {
            self.idle_flag = true;
        }

        let rows = self.result.len();
        let cols = if rows > 0 { self.result[0].len() } else { 0 };

        ComputeUnitTrace {
            cycle: self.cycle,
            unit_name: self.name().to_string(),
            architecture: self.architecture(),
            scheduler_action: format!("inference complete: {}x{} result", rows, cols),
            active_warps: if self.idle_flag { 0 } else { 1 },
            total_warps: 1,
            engine_traces: HashMap::new(),
            shared_memory_used: 0,
            shared_memory_total: self.config.sram_size,
            register_file_used: self.config.num_macs,
            register_file_total: self.config.num_macs,
            occupancy: if self.idle_flag { 0.0 } else { 1.0 },
            l1_hits: 0,
            l1_misses: 0,
        }
    }

    fn run(&mut self, max_cycles: usize) -> Vec<ComputeUnitTrace> {
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

    fn reset(&mut self) {
        self.mac_engine.reset();
        self.work_items.clear();
        self.result.clear();
        self.idle_flag = true;
        self.cycle = 0;
    }
}

impl fmt::Display for NeuralEngineCore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "NeuralEngineCore(macs={}, idle={})",
            self.config.num_macs, self.idle_flag,
        )
    }
}
