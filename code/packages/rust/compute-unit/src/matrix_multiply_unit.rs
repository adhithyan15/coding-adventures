//! MatrixMultiplyUnit -- Google TPU MXU simulator.
//!
//! # What is an MXU?
//!
//! The Matrix Multiply Unit is the heart of Google's TPU (Tensor Processing
//! Unit). It's fundamentally different from GPU compute units -- there are NO
//! threads, NO warps, NO schedulers. Instead, it has:
//!
//! 1. **Systolic arrays** -- the main compute engine (from Layer 8)
//! 2. **Vector unit** -- for element-wise operations (activation functions)
//! 3. **Accumulators** -- for storing partial matrix results
//! 4. **Control sequencer** -- manages the tiling schedule
//!
//! # Why No Threads?
//!
//! Matrix multiplication is perfectly predictable. You know exactly which
//! values need to be multiplied together and in what order. There's no
//! branching, no data-dependent control flow, no need for a runtime scheduler.
//!
//! ```text
//! GPU:  Complex hardware scheduler decides at runtime
//! TPU:  Simple hardware follows compile-time plan
//! ```
//!
//! # Architecture Diagram
//!
//! ```text
//! MatrixMultiplyUnit (TPU v2-style)
//! +---------------------------------------------------------------+
//! |  Control Sequencer                                             |
//! |  +-----------------------------------------------------------+ |
//! |  | Tile schedule: load A[0:128], matmul, load A[128:256]     | |
//! |  +-----------------------------------------------------------+ |
//! |                                                                |
//! |  +---------------------------------------------+               |
//! |  | Systolic Array (128x128)                     |               |
//! |  |   Weights pre-loaded into PEs                |               |
//! |  |   Activations stream in from left            |               |
//! |  +---------------------------------------------+               |
//! |                    |                                            |
//! |                    v                                            |
//! |  +---------------------------------------------+               |
//! |  | Accumulators (128 x FP32)                    |               |
//! |  +---------------------------------------------+               |
//! |                    |                                            |
//! |                    v                                            |
//! |  +---------------------------------------------+               |
//! |  | Vector Unit (128-wide)                       |               |
//! |  | ReLU, sigmoid, add bias, normalize           |               |
//! |  +---------------------------------------------+               |
//! +---------------------------------------------------------------+
//! ```

use std::collections::HashMap;
use std::fmt;

use fp_arithmetic::{FloatFormat, BF16, FP32};
use parallel_execution_engine::{SystolicArray, SystolicConfig};
use parallel_execution_engine::protocols::ParallelExecutionEngine;

use crate::protocols::{
    Architecture, ComputeUnit, ComputeUnitTrace, ResourceError, WorkItem,
};

// ---------------------------------------------------------------------------
// MXUConfig -- configuration for a TPU-style Matrix Multiply Unit
// ---------------------------------------------------------------------------

/// Configuration for a TPU-style Matrix Multiply Unit.
///
/// ```text
/// Parameter           | TPU v1       | TPU v2/v3    | TPU v4
/// --------------------+--------------+--------------+--------
/// Array size          | 256x256      | 128x128      | 128x128
/// Input format        | INT8         | BF16         | BF16
/// Accumulator format  | INT32        | FP32         | FP32
/// Vector width        | 256          | 128          | 128
/// ```
#[derive(Debug, Clone)]
pub struct MXUConfig {
    /// Systolic array rows.
    pub array_rows: usize,
    /// Systolic array columns.
    pub array_cols: usize,
    /// FP format for systolic array inputs.
    pub systolic_format: FloatFormat,
    /// FP format for accumulation (higher precision).
    pub accumulator_format: FloatFormat,
    /// Width of the vector unit.
    pub vector_width: usize,
    /// FP format for vector operations.
    pub vector_format: FloatFormat,
    /// Number of accumulator registers.
    pub accumulator_count: usize,
    /// Weight staging buffer in bytes.
    pub weight_buffer_size: usize,
    /// Activation buffer in bytes.
    pub activation_buffer_size: usize,
}

impl Default for MXUConfig {
    fn default() -> Self {
        Self {
            array_rows: 128,
            array_cols: 128,
            systolic_format: BF16,
            accumulator_format: FP32,
            vector_width: 128,
            vector_format: FP32,
            accumulator_count: 128,
            weight_buffer_size: 4_194_304,
            activation_buffer_size: 2_097_152,
        }
    }
}

// ---------------------------------------------------------------------------
// MatrixMultiplyUnit -- the main MXU simulator
// ---------------------------------------------------------------------------

/// Google TPU Matrix Multiply Unit simulator.
///
/// Uses a systolic array from Layer 8 to perform matrix multiplication,
/// with tiling logic for matrices larger than the array, and a vector
/// unit for post-processing (activation functions, bias add).
///
/// # Execution Model
///
/// The MXU has no threads or schedulers. Instead, it processes **tiles**
/// of a larger matrix operation. The control sequencer manages:
///
/// 1. Loading weight tiles into the systolic array
/// 2. Streaming activation tiles through the array
/// 3. Accumulating partial results
/// 4. Applying vector operations (activation functions)
/// 5. Storing output tiles
pub struct MatrixMultiplyUnit {
    config: MXUConfig,
    cycle: u64,
    array: SystolicArray,
    accumulators: Vec<Vec<f64>>,
    current_result: Vec<Vec<f64>>,
    work_items: Vec<WorkItem>,
    is_idle: bool,
}

impl MatrixMultiplyUnit {
    /// Create a new Matrix Multiply Unit with the given configuration.
    pub fn new(config: MXUConfig) -> Self {
        let array = SystolicArray::new(SystolicConfig {
            rows: config.array_rows,
            cols: config.array_cols,
            float_format: FP32,
            accumulator_format: FP32,
        });
        Self {
            config,
            cycle: 0,
            array,
            accumulators: Vec::new(),
            current_result: Vec::new(),
            work_items: Vec::new(),
            is_idle: true,
        }
    }

    /// Access to the MXU configuration.
    pub fn config(&self) -> &MXUConfig {
        &self.config
    }

    /// The result matrix from the last matmul.
    pub fn result(&self) -> &[Vec<f64>] {
        &self.current_result
    }

    /// Access to the underlying systolic array.
    pub fn systolic_array(&self) -> &SystolicArray {
        &self.array
    }

    /// Run a complete matmul with optional activation function.
    ///
    /// # Supported Activation Functions
    ///
    /// ```text
    /// none:    f(x) = x              (identity)
    /// relu:    f(x) = max(0, x)      (most popular)
    /// sigmoid: f(x) = 1/(1+e^-x)    (squashes to [0,1])
    /// tanh:    f(x) = tanh(x)        (squashes to [-1,1])
    /// ```
    pub fn run_matmul(
        &mut self,
        activations: &[Vec<f64>],
        weights: &[Vec<f64>],
        activation_fn: &str,
    ) -> Vec<Vec<f64>> {
        // Convert to the format expected by SystolicArray::run_matmul.
        let act_slices: Vec<&[f64]> = activations.iter().map(|r| r.as_slice()).collect();
        let wt_slices: Vec<&[f64]> = weights.iter().map(|r| r.as_slice()).collect();

        let mut result = self.array.run_matmul(&act_slices, &wt_slices);

        if activation_fn != "none" {
            result = Self::apply_activation(&result, activation_fn);
        }

        self.current_result = result.clone();
        result
    }

    /// Apply an activation function element-wise to a matrix.
    ///
    /// This simulates the MXU's vector unit, which processes one row
    /// at a time, applying the activation function to each element.
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

    /// Produce a trace for when the MXU is idle.
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
            shared_memory_total: self.config.weight_buffer_size,
            register_file_used: 0,
            register_file_total: self.config.accumulator_count,
            occupancy: 0.0,
            l1_hits: 0,
            l1_misses: 0,
        }
    }
}

impl ComputeUnit for MatrixMultiplyUnit {
    fn name(&self) -> &str {
        "MXU"
    }

    fn architecture(&self) -> Architecture {
        Architecture::GoogleMxu
    }

    fn idle(&self) -> bool {
        self.is_idle
    }

    fn dispatch(&mut self, work: WorkItem) -> Result<(), ResourceError> {
        self.work_items.push(work);
        self.is_idle = false;
        Ok(())
    }

    fn step(&mut self) -> ComputeUnitTrace {
        self.cycle += 1;

        if self.is_idle || self.work_items.is_empty() {
            return self.make_idle_trace();
        }

        // Process the first pending work item.
        let work = self.work_items.remove(0);

        if let (Some(input_data), Some(weight_data)) = (work.input_data, work.weight_data) {
            let act_slices: Vec<&[f64]> = input_data.iter().map(|r| r.as_slice()).collect();
            let wt_slices: Vec<&[f64]> = weight_data.iter().map(|r| r.as_slice()).collect();
            self.current_result = self.array.run_matmul(&act_slices, &wt_slices);
        } else {
            self.current_result = Vec::new();
        }

        if self.work_items.is_empty() {
            self.is_idle = true;
        }

        let rows = self.current_result.len();
        let cols = if rows > 0 {
            self.current_result[0].len()
        } else {
            0
        };

        ComputeUnitTrace {
            cycle: self.cycle,
            unit_name: self.name().to_string(),
            architecture: self.architecture(),
            scheduler_action: format!("matmul complete: {}x{} result", rows, cols),
            active_warps: if self.is_idle { 0 } else { 1 },
            total_warps: 1,
            engine_traces: HashMap::new(),
            shared_memory_used: 0,
            shared_memory_total: self.config.weight_buffer_size,
            register_file_used: self.config.accumulator_count,
            register_file_total: self.config.accumulator_count,
            occupancy: if self.is_idle { 0.0 } else { 1.0 },
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
        self.array.reset();
        self.accumulators.clear();
        self.current_result.clear();
        self.work_items.clear();
        self.is_idle = true;
        self.cycle = 0;
    }
}

impl fmt::Display for MatrixMultiplyUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MatrixMultiplyUnit({}x{}, idle={})",
            self.config.array_rows, self.config.array_cols, self.is_idle,
        )
    }
}
