//! SystolicArray -- dataflow execution for matrix multiplication (Google TPU style).
//!
//! # What is a Systolic Array?
//!
//! The word "systolic" comes from the Greek "systole" (contraction), like a
//! heartbeat. In a systolic array, data pulses through a grid of processing
//! elements on each clock cycle, just like blood pulses through the body with
//! each heartbeat.
//!
//! A systolic array is radically different from GPU execution:
//!
//! ```text
//! GPU (SIMT/SIMD):                   TPU (Systolic):
//! +--------------------------+       +--------------------------+
//! | Has instructions         |       | NO instructions           |
//! | Has program counter      |       | NO program counter        |
//! | Has branches             |       | NO branches               |
//! | Complex control logic    |       | Dead-simple PEs           |
//! | General-purpose          |       | Matrix multiply ONLY      |
//! +--------------------------+       +--------------------------+
//! ```
//!
//! Each PE in the array does exactly ONE thing on each clock cycle:
//!
//! ```text
//! accumulator += input_from_left * local_weight
//! ```
//!
//! Then it passes the input to the right neighbor. That's it. No instruction
//! fetch, no decode, no branch prediction. Just multiply, accumulate, and pass.
//!
//! # How Matrix Multiplication Maps to a Systolic Array
//!
//! Computing C = A x W (activation matrix times weight matrix):
//!
//! 1. Pre-load weights into each PE: PE(i,j) gets W[i][j]
//! 2. Feed activation rows from the left, STAGGERED in time
//! 3. Data flows right through each row, partial sums flow down
//! 4. After 2N-1 cycles, the result matrix C emerges at the bottom
//!
//! # Why TPUs Use Systolic Arrays
//!
//! Neural network inference and training are dominated by matrix multiplication.
//! A systolic array is the most efficient hardware for matrix multiply because:
//!
//! 1. No instruction overhead (no fetch, decode, branch)
//! 2. Maximum data reuse (each value is used N times as it flows through)
//! 3. Nearest-neighbor communication only (each PE talks to adjacent PEs)
//! 4. Regular, predictable data movement (no cache misses)
//! 5. Simple PE design -> high clock frequency, low power
//!
//! Google's TPU v1 has a 256x256 systolic array that performs 65,536 MAC
//! operations per clock cycle. At 700 MHz, that's ~46 TOPS (tera-ops/second).

use std::collections::HashMap;

use fp_arithmetic::{FloatBits, FloatFormat, FP32, float_to_bits, bits_to_float, fp_fma};

use crate::protocols::{
    DataflowInfo, EngineTrace, ExecutionModel, ParallelExecutionEngine,
};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for a systolic array engine.
///
/// Real-world reference values:
///
/// ```text
/// Hardware    | Rows | Cols | Format | Accumulator
/// ------------+------+------+--------+------------
/// TPU v1      | 256  | 256  | INT8   | INT32
/// TPU v2/v3   | 128  | 128  | BF16   | FP32
/// TPU v4      | 128  | 128  | BF16   | FP32
/// Our default | 4    | 4    | FP32   | FP32
/// ```
#[derive(Debug, Clone)]
pub struct SystolicConfig {
    /// Number of PE rows in the array.
    pub rows: usize,
    /// Number of PE columns in the array.
    pub cols: usize,
    /// Format for inputs and weights.
    pub float_format: FloatFormat,
    /// Format for the accumulator (usually higher precision).
    pub accumulator_format: FloatFormat,
}

impl Default for SystolicConfig {
    fn default() -> Self {
        Self {
            rows: 4,
            cols: 4,
            float_format: FP32,
            accumulator_format: FP32,
        }
    }
}

// ---------------------------------------------------------------------------
// SystolicPE -- one processing element in the grid
// ---------------------------------------------------------------------------

/// One processing element in the systolic array.
///
/// Each PE is extremely simple -- it's just a multiply-accumulate unit
/// with two data ports:
///
/// ```text
/// Input from left --> [  weight  ] --> Output to right
///                     [  x + acc ]
///                          |
///                   Partial sum flows down
/// ```
///
/// On each clock cycle, a PE does:
/// 1. If there's an input: accumulator += input * weight
/// 2. Pass the input to the right neighbor
/// 3. (Partial sums flow down at the end of computation)
pub struct SystolicPE {
    pub row: usize,
    pub col: usize,
    pub weight: FloatBits,
    pub accumulator: FloatBits,
    pub input_buffer: Option<FloatBits>,
}

impl SystolicPE {
    /// Perform one MAC cycle.
    ///
    /// If there's an input waiting in the buffer:
    ///     accumulator += input_buffer * weight
    /// Returns the input (to be passed to the right neighbor), or None.
    ///
    /// This is the heart of the systolic array -- the simplest possible
    /// processing element. No instruction fetch, no decode, no branch.
    /// Just: multiply, accumulate, pass.
    pub fn compute(&mut self) -> Option<FloatBits> {
        let input_val = self.input_buffer.take()?;

        // MAC: accumulator = input * weight + accumulator
        // Using fp_fma for fused multiply-add (more accurate than mul+add)
        self.accumulator = fp_fma(&input_val, &self.weight, &self.accumulator);

        Some(input_val) // Pass to right neighbor
    }
}

// ---------------------------------------------------------------------------
// SystolicArray -- the dataflow execution engine
// ---------------------------------------------------------------------------

/// Systolic dataflow execution engine (Google TPU style).
///
/// An NxN grid of processing elements. Data flows through the array --
/// activations left-to-right, partial sums accumulate in each PE.
/// No instruction stream. Just data in, results out.
///
/// # Data Flow Pattern
///
/// ```text
/// Inputs feed from the left edge:
///
/// a[0] --> PE(0,0) --> PE(0,1) --> PE(0,2) --> PE(0,3)
/// a[1] --> PE(1,0) --> PE(1,1) --> PE(1,2) --> PE(1,3)
/// a[2] --> PE(2,0) --> PE(2,1) --> PE(2,2) --> PE(2,3)
/// a[3] --> PE(3,0) --> PE(3,1) --> PE(3,2) --> PE(3,3)
///
/// Each PE accumulates: acc += input * weight
/// After all inputs flow through, drain accumulators as the result.
/// ```
///
/// # Example
///
/// ```
/// use parallel_execution_engine::systolic_array::{SystolicArray, SystolicConfig};
///
/// let mut config = SystolicConfig::default();
/// config.rows = 2;
/// config.cols = 2;
/// let mut array = SystolicArray::new(config);
/// let result = array.run_matmul(
///     &[&[1.0, 2.0][..], &[3.0, 4.0][..]],
///     &[&[5.0, 6.0][..], &[7.0, 8.0][..]],
/// );
/// // result[0][0] = 1*5 + 2*7 = 19.0
/// ```
pub struct SystolicArray {
    config: SystolicConfig,
    cycle: u64,
    halted: bool,
    /// The NxN grid of PEs.
    grid: Vec<Vec<SystolicPE>>,
    /// Input queues: one per row, feeding from the left edge.
    input_queues: Vec<Vec<FloatBits>>,
    /// Track how many total inputs have been fed (for halting detection).
    total_inputs_fed: usize,
}

impl SystolicArray {
    /// Create a new SystolicArray with the given configuration.
    pub fn new(config: SystolicConfig) -> Self {
        let grid: Vec<Vec<SystolicPE>> = (0..config.rows)
            .map(|r| {
                (0..config.cols)
                    .map(|c| SystolicPE {
                        row: r,
                        col: c,
                        weight: float_to_bits(0.0, config.float_format),
                        accumulator: float_to_bits(0.0, config.accumulator_format),
                        input_buffer: None,
                    })
                    .collect()
            })
            .collect();
        let input_queues = vec![Vec::new(); config.rows];

        Self {
            config,
            cycle: 0,
            halted: false,
            grid,
            input_queues,
            total_inputs_fed: 0,
        }
    }

    /// The configuration this array was created with.
    pub fn config(&self) -> &SystolicConfig {
        &self.config
    }

    /// Access to the PE grid (for inspection).
    pub fn grid(&self) -> &Vec<Vec<SystolicPE>> {
        &self.grid
    }

    /// Pre-load the weight matrix into the PE array.
    ///
    /// weights[row][col] goes to PE(row, col). In real TPU hardware, weight
    /// loading happens before the matrix multiply begins. The weights stay
    /// fixed while activations flow through.
    pub fn load_weights(&mut self, weights: &[&[f64]]) {
        for r in 0..weights.len().min(self.config.rows) {
            for c in 0..weights[r].len().min(self.config.cols) {
                self.grid[r][c].weight =
                    float_to_bits(weights[r][c], self.config.float_format);
            }
        }
    }

    /// Feed one activation value into the left edge of the specified row.
    ///
    /// The value will enter PE(row, 0) on the next step, then flow right
    /// through PE(row, 1), PE(row, 2), etc. on subsequent steps.
    pub fn feed_input(&mut self, row: usize, value: f64) {
        assert!(
            row < self.config.rows,
            "Row {} out of range [0, {})",
            row,
            self.config.rows
        );
        self.input_queues[row].push(float_to_bits(value, self.config.float_format));
        self.total_inputs_fed += 1;
    }

    /// Run a complete matrix multiplication C = A x W.
    ///
    /// # How the Systolic Matmul Works
    ///
    /// For C = A x W where A is MxK and W is KxN:
    ///     C[i][j] = sum_k( A[i][k] * W[k][j] )
    ///
    /// We compute this one output row at a time:
    ///     For each row i of A:
    ///         1. Reset accumulators
    ///         2. Feed A[i][k] into PE row k (with staggered timing)
    ///         3. PE(k, j) computes: acc += A[i][k] * W[k][j]
    ///         4. After all activations flow through, column j accumulates
    ///            sum_k(A[i][k] * W[k][j]) = C[i][j]
    ///         5. Drain results for row i
    pub fn run_matmul(
        &mut self,
        activations: &[&[f64]],
        weights: &[&[f64]],
    ) -> Vec<Vec<f64>> {
        let num_output_rows = activations.len();
        let inner_dim = if !activations.is_empty() {
            activations[0].len()
        } else {
            0
        };
        let num_output_cols = if !weights.is_empty() {
            weights[0].len()
        } else {
            0
        };

        // Load weights: PE(k, j) gets W[k][j]
        self.reset();
        self.load_weights(weights);

        let mut result: Vec<Vec<f64>> = Vec::new();

        // Compute one output row at a time
        for i in 0..num_output_rows {
            // Reset accumulators (but keep weights)
            let zero_acc = float_to_bits(0.0, self.config.accumulator_format);
            for r in 0..self.config.rows {
                for c in 0..self.config.cols {
                    self.grid[r][c].accumulator = zero_acc.clone();
                    self.grid[r][c].input_buffer = None;
                }
            }
            self.input_queues = vec![Vec::new(); self.config.rows];
            self.halted = false;

            // Feed A[i][k] into row k with staggered timing.
            let mut feed_schedule: HashMap<usize, Vec<(usize, f64)>> = HashMap::new();
            for k in 0..inner_dim {
                let cycle = k;
                feed_schedule
                    .entry(cycle)
                    .or_default()
                    .push((k, activations[i][k]));
            }

            // Run until all data has flowed through
            let total_steps = inner_dim + self.config.cols + 1;
            for step_num in 0..total_steps {
                if let Some(feeds) = feed_schedule.get(&step_num) {
                    for &(row, val) in feeds {
                        self.feed_input(row, val);
                    }
                }
                self.step();
            }

            // Drain: sum accumulators vertically for each column j.
            // C[i][j] = sum_k PE(k, j).accumulator
            let mut row_result: Vec<f64> = Vec::new();
            for j in 0..num_output_cols {
                let mut col_sum = 0.0;
                for k in 0..inner_dim.min(self.config.rows) {
                    col_sum += bits_to_float(&self.grid[k][j].accumulator);
                }
                row_result.push(col_sum);
            }
            result.push(row_result);
        }

        result
    }

    /// Read the accumulated results from all PEs.
    ///
    /// After computation, each PE's accumulator holds one element of the
    /// result matrix. PE(r, c) holds C[r][c].
    pub fn drain_outputs(&self) -> Vec<Vec<f64>> {
        let mut result: Vec<Vec<f64>> = Vec::new();
        for r in 0..self.config.rows {
            let mut row: Vec<f64> = Vec::new();
            for c in 0..self.config.cols {
                row.push(bits_to_float(&self.grid[r][c].accumulator));
            }
            result.push(row);
        }
        result
    }
}

impl ParallelExecutionEngine for SystolicArray {
    fn name(&self) -> &str {
        "SystolicArray"
    }

    fn width(&self) -> usize {
        self.config.rows * self.config.cols
    }

    fn execution_model(&self) -> ExecutionModel {
        ExecutionModel::Systolic
    }

    /// Advance one cycle: data moves one PE to the right.
    ///
    /// On each cycle:
    /// 1. For each PE (from right to left, to avoid overwriting):
    ///    a. Compute: acc += input * weight
    ///    b. Pass input to the right neighbor.
    /// 2. Feed new inputs from queues into the leftmost column.
    /// 3. Build a trace showing the state of the array.
    ///
    /// We process PEs from right to left so that the "pass to right"
    /// doesn't interfere with the current cycle's computation.
    fn step(&mut self) -> EngineTrace {
        self.cycle += 1;

        let mut active_count = 0;
        let mut pe_states: Vec<Vec<String>> = Vec::new();

        // Phase 1: Move data rightward through the array.
        // Process from right to left to avoid data collision.
        for r in 0..self.config.rows {
            for c in (0..self.config.cols).rev() {
                let output = self.grid[r][c].compute();

                if output.is_some() {
                    active_count += 1;
                    // Pass input to right neighbor (if exists)
                    if c + 1 < self.config.cols {
                        // We need to clone the output to pass it
                        let output_val = output.unwrap();
                        self.grid[r][c + 1].input_buffer = Some(output_val);
                    }
                }
            }

            // Build state strings (left to right for display)
            let mut row_states: Vec<String> = Vec::new();
            for c in 0..self.config.cols {
                let pe = &self.grid[r][c];
                let acc_val = bits_to_float(&pe.accumulator);
                let mut state = format!("acc={:.4}", acc_val);
                if let Some(ref input) = pe.input_buffer {
                    let in_val = bits_to_float(input);
                    state += &format!(", in={:.4}", in_val);
                }
                row_states.push(state);
            }
            pe_states.push(row_states);
        }

        // Phase 2: Feed new inputs from queues into column 0
        for r in 0..self.config.rows {
            if !self.input_queues[r].is_empty() {
                let val = self.input_queues[r].remove(0);
                self.grid[r][0].input_buffer = Some(val);
            }
        }

        // Check if computation is complete
        let total = self.config.rows * self.config.cols;
        let any_input_remaining = self.input_queues.iter().any(|q| !q.is_empty());
        let any_input_in_flight = self.grid.iter().any(|row| {
            row.iter().any(|pe| pe.input_buffer.is_some())
        });

        if !any_input_remaining && !any_input_in_flight {
            self.halted = true;
        }

        let utilization = if total > 0 {
            active_count as f64 / total as f64
        } else {
            0.0
        };

        // Build unit_traces map
        let mut unit_traces = HashMap::new();
        for r in 0..self.config.rows {
            for c in 0..self.config.cols {
                unit_traces.insert(
                    r * self.config.cols + c,
                    pe_states[r][c].clone(),
                );
            }
        }

        // Build active mask (simplified)
        let active_mask: Vec<bool> = (0..total)
            .map(|i| {
                let r = i / self.config.cols;
                let c = i % self.config.cols;
                self.grid[r][c].input_buffer.is_some() || i < active_count
            })
            .collect();

        EngineTrace {
            cycle: self.cycle,
            engine_name: self.name().to_string(),
            execution_model: ExecutionModel::Systolic,
            description: format!(
                "Systolic step -- {}/{} PEs active",
                active_count, total
            ),
            unit_traces,
            active_mask,
            active_count,
            total_count: total,
            utilization,
            divergence_info: None,
            dataflow_info: Some(DataflowInfo {
                pe_states,
                data_positions: HashMap::new(),
            }),
        }
    }

    fn halted(&self) -> bool {
        self.halted
    }

    /// Reset the array to its initial state.
    ///
    /// Clears all accumulators, input buffers, and queues. Weights are
    /// preserved -- call load_weights() to change them.
    fn reset(&mut self) {
        let zero_acc = float_to_bits(0.0, self.config.accumulator_format);
        for r in 0..self.config.rows {
            for c in 0..self.config.cols {
                self.grid[r][c].accumulator = zero_acc.clone();
                self.grid[r][c].input_buffer = None;
            }
        }
        self.input_queues = vec![Vec::new(); self.config.rows];
        self.cycle = 0;
        self.halted = false;
        self.total_inputs_fed = 0;
    }
}

impl std::fmt::Debug for SystolicArray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SystolicArray({}x{}, cycle={}, halted={})",
            self.config.rows, self.config.cols, self.cycle, self.halted
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_systolic_creation() {
        let config = SystolicConfig::default();
        let array = SystolicArray::new(config);
        assert_eq!(array.width(), 16); // 4x4
        assert_eq!(array.name(), "SystolicArray");
        assert_eq!(array.execution_model(), ExecutionModel::Systolic);
        assert!(!array.halted());
    }

    #[test]
    fn test_systolic_simple_matmul() {
        // C = A x W where A = [[1,2],[3,4]], W = [[5,6],[7,8]]
        // C[0][0] = 1*5 + 2*7 = 19
        // C[0][1] = 1*6 + 2*8 = 22
        // C[1][0] = 3*5 + 4*7 = 43
        // C[1][1] = 3*6 + 4*8 = 50
        let mut config = SystolicConfig::default();
        config.rows = 2;
        config.cols = 2;
        let mut array = SystolicArray::new(config);

        let result = array.run_matmul(
            &[&[1.0, 2.0], &[3.0, 4.0]],
            &[&[5.0, 6.0], &[7.0, 8.0]],
        );

        assert_eq!(result.len(), 2);
        assert!((result[0][0] - 19.0).abs() < 0.01);
        assert!((result[0][1] - 22.0).abs() < 0.01);
        assert!((result[1][0] - 43.0).abs() < 0.01);
        assert!((result[1][1] - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_systolic_identity_matmul() {
        // A x I = A
        let mut config = SystolicConfig::default();
        config.rows = 2;
        config.cols = 2;
        let mut array = SystolicArray::new(config);

        let result = array.run_matmul(
            &[&[3.0, 4.0], &[5.0, 6.0]],
            &[&[1.0, 0.0], &[0.0, 1.0]],
        );

        assert!((result[0][0] - 3.0).abs() < 0.01);
        assert!((result[0][1] - 4.0).abs() < 0.01);
        assert!((result[1][0] - 5.0).abs() < 0.01);
        assert!((result[1][1] - 6.0).abs() < 0.01);
    }

    #[test]
    fn test_systolic_step_trace() {
        let mut config = SystolicConfig::default();
        config.rows = 2;
        config.cols = 2;
        let mut array = SystolicArray::new(config);
        array.load_weights(&[&[1.0, 2.0], &[3.0, 4.0]]);
        array.feed_input(0, 5.0);

        let trace = array.step();
        assert!(trace.description.contains("Systolic step"));
        assert!(trace.dataflow_info.is_some());
    }

    #[test]
    fn test_systolic_drain_outputs() {
        let config = SystolicConfig::default();
        let array = SystolicArray::new(config);
        let outputs = array.drain_outputs();
        assert_eq!(outputs.len(), 4);
        for row in &outputs {
            assert_eq!(row.len(), 4);
            for &val in row {
                assert_eq!(val, 0.0);
            }
        }
    }

    #[test]
    fn test_systolic_reset() {
        let mut config = SystolicConfig::default();
        config.rows = 2;
        config.cols = 2;
        let mut array = SystolicArray::new(config);
        array.load_weights(&[&[1.0, 2.0], &[3.0, 4.0]]);
        array.feed_input(0, 5.0);
        array.step();

        array.reset();
        assert!(!array.halted());
        let outputs = array.drain_outputs();
        for row in &outputs {
            for &val in row {
                assert_eq!(val, 0.0);
            }
        }
    }

    #[test]
    fn test_systolic_debug_format() {
        let config = SystolicConfig::default();
        let array = SystolicArray::new(config);
        let debug = format!("{:?}", array);
        assert!(debug.contains("SystolicArray"));
        assert!(debug.contains("4x4"));
    }
}
