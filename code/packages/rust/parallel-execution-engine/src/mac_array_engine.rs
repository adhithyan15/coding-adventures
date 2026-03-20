//! MACArrayEngine -- compiler-scheduled MAC array execution (NPU style).
//!
//! # What is a MAC Array?
//!
//! A MAC (Multiply-Accumulate) array is a bank of multiply-accumulate units
//! driven entirely by a schedule that the compiler generates at compile time.
//! There is NO hardware scheduler -- the compiler decides exactly which MAC
//! unit processes which data on which cycle.
//!
//! This is the execution model used by:
//! - Apple Neural Engine (ANE)
//! - Qualcomm Hexagon NPU
//! - Many custom AI accelerator ASICs
//!
//! # How It Differs from Other Models
//!
//! ```text
//! GPU (SIMT/SIMD):                   NPU (Scheduled MAC):
//! +--------------------------+       +--------------------------+
//! | Hardware scheduler       |       | NO hardware scheduler    |
//! | Runtime decisions        |       | All decisions at compile  |
//! | Branch prediction        |       | NO branches              |
//! | Dynamic resource alloc   |       | Static resource plan     |
//! | Flexible but complex     |       | Simple but rigid         |
//! +--------------------------+       +--------------------------+
//! ```
//!
//! # The Execution Pipeline
//!
//! A MAC array engine has a simple pipeline:
//!
//! ```text
//! 1. LOAD_INPUT:    Move data from external memory to input buffer
//! 2. LOAD_WEIGHTS:  Move weights from external memory to weight buffer
//! 3. MAC:           Multiply input[i] * weight[i] for all MACs in parallel
//! 4. REDUCE:        Sum the MAC results (adder tree)
//! 5. ACTIVATE:      Apply activation function (ReLU, sigmoid, tanh)
//! 6. STORE_OUTPUT:  Write result to output buffer
//! ```
//!
//! ```text
//! Input Buffer --> +----+ +----+ +----+ +----+
//!                  |MAC0| |MAC1| |MAC2| |MAC3|  (parallel multiply)
//! Weight Buffer--> +--+-+ +--+-+ +--+-+ +--+-+
//!                     |      |      |      |
//!                     +------+------+------+
//!                                |
//!                         +------+------+
//!                         |  Adder Tree |  (reduce / sum)
//!                         +------+------+
//!                                |
//!                         +------+------+
//!                         | Activation  |  (ReLU, sigmoid, etc.)
//!                         +------+------+
//!                                |
//!                         Output Buffer
//! ```
//!
//! # Why NPUs Are Power-Efficient
//!
//! By moving all scheduling to compile time, NPUs eliminate:
//! - Branch prediction hardware (saves transistors and power)
//! - Instruction cache (the "program" is a simple schedule table)
//! - Warp/wavefront scheduler (no runtime thread management)
//! - Speculation hardware (nothing is speculative)

use std::collections::HashMap;

use fp_arithmetic::{FloatFormat, FP16, FP32};

use crate::protocols::{EngineTrace, ExecutionModel, ParallelExecutionEngine};

// ---------------------------------------------------------------------------
// Operations and activation functions
// ---------------------------------------------------------------------------

/// Operations that can appear in a MAC array schedule.
///
/// Each operation corresponds to one stage of the MAC pipeline:
///
/// ```text
/// LoadInput:    Fill the input buffer with activation data.
/// LoadWeights:  Fill the weight buffer with weight data.
/// Mac:          Parallel multiply-accumulate across all MAC units.
/// Reduce:       Sum results from multiple MACs (adder tree).
/// Activate:     Apply a non-linear activation function.
/// StoreOutput:  Write results to the output buffer.
/// ```
///
/// The compiler sequences these operations into a static schedule
/// that the hardware executes cycle by cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MACOperation {
    LoadInput,
    LoadWeights,
    Mac,
    Reduce,
    Activate,
    StoreOutput,
}

/// Hardware-supported activation functions.
///
/// Neural networks use non-linear "activation functions" after each layer.
/// NPUs typically implement a few common ones in hardware for speed:
///
/// ```text
/// None:    f(x) = x              (identity / linear)
/// ReLU:    f(x) = max(0, x)      (most popular; simple, fast)
/// Sigmoid: f(x) = 1/(1+e^-x)    (classic; squashes to [0,1])
/// Tanh:    f(x) = tanh(x)        (squashes to [-1,1])
/// ```
///
/// ReLU is by far the most common because it's trivially cheap in hardware
/// (just check the sign bit and zero-out negatives).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationFunction {
    None,
    Relu,
    Sigmoid,
    Tanh,
}

impl ActivationFunction {
    /// Return the string name of this activation function.
    pub fn name(&self) -> &'static str {
        match self {
            ActivationFunction::None => "none",
            ActivationFunction::Relu => "relu",
            ActivationFunction::Sigmoid => "sigmoid",
            ActivationFunction::Tanh => "tanh",
        }
    }
}

// ---------------------------------------------------------------------------
// Schedule entry
// ---------------------------------------------------------------------------

/// One entry in the MAC array schedule.
///
/// The compiler generates these at compile time. Each entry describes
/// exactly what happens on one cycle -- which operation, which data indices,
/// and where to write the result.
///
/// Example schedule for a simple dot product of 4 elements:
///
/// ```text
/// Cycle 0: LoadInput   indices=[0,1,2,3]
/// Cycle 1: LoadWeights indices=[0,1,2,3]
/// Cycle 2: Mac         input=[0,1,2,3] weight=[0,1,2,3] out=0
/// Cycle 3: Reduce      out=0
/// Cycle 4: Activate    out=0, activation=relu
/// Cycle 5: StoreOutput out=0
/// ```
#[derive(Debug, Clone)]
pub struct MACScheduleEntry {
    /// Which cycle to execute this entry.
    pub cycle: u64,
    /// What to do (LoadInput, Mac, Reduce, Activate, StoreOutput).
    pub operation: MACOperation,
    /// Which input buffer slots to read.
    pub input_indices: Vec<usize>,
    /// Which weight buffer slots to use.
    pub weight_indices: Vec<usize>,
    /// Where to write the result.
    pub output_index: usize,
    /// Which activation function (for Activate operations).
    pub activation: ActivationFunction,
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for a scheduled MAC array engine.
///
/// Real-world reference values:
///
/// ```text
/// Hardware          | MACs | Input Buf | Weight Buf | Format
/// ------------------+------+-----------+------------+-------
/// Apple ANE (M1)    | 16K  | varies    | varies     | FP16/INT8
/// Qualcomm Hexagon  | 2K   | varies    | varies     | INT8
/// Our default       | 8    | 1024      | 4096       | FP16
/// ```
#[derive(Debug, Clone)]
pub struct MACArrayConfig {
    /// Number of parallel MAC units.
    pub num_macs: usize,
    /// Input buffer capacity in elements.
    pub input_buffer_size: usize,
    /// Weight buffer capacity in elements.
    pub weight_buffer_size: usize,
    /// Output buffer capacity in elements.
    pub output_buffer_size: usize,
    /// Compute format for inputs/weights.
    pub float_format: FloatFormat,
    /// Higher-precision format for accumulation.
    pub accumulator_format: FloatFormat,
    /// Whether hardware activation function is available.
    pub has_activation_unit: bool,
}

impl Default for MACArrayConfig {
    fn default() -> Self {
        Self {
            num_macs: 8,
            input_buffer_size: 1024,
            weight_buffer_size: 4096,
            output_buffer_size: 1024,
            float_format: FP16,
            accumulator_format: FP32,
            has_activation_unit: true,
        }
    }
}

// ---------------------------------------------------------------------------
// MACArrayEngine -- the scheduled execution engine
// ---------------------------------------------------------------------------

/// Compiler-scheduled MAC array execution engine (NPU style).
///
/// No hardware scheduler. The compiler generates a static schedule that
/// says exactly what each MAC does on each cycle.
///
/// # Usage Pattern
///
/// 1. Create engine with config.
/// 2. Load inputs and weights into the buffers.
/// 3. Load a compiler-generated schedule.
/// 4. Step or run -- the engine follows the schedule exactly.
/// 5. Read results from the output buffer.
///
/// # Example
///
/// ```
/// use parallel_execution_engine::mac_array_engine::*;
///
/// let mut engine = MACArrayEngine::new(MACArrayConfig::default());
/// engine.load_inputs(&[1.0, 2.0, 3.0, 4.0]);
/// engine.load_weights(&[0.5, 0.5, 0.5, 0.5]);
/// let schedule = vec![
///     MACScheduleEntry {
///         cycle: 1, operation: MACOperation::Mac,
///         input_indices: vec![0,1,2,3], weight_indices: vec![0,1,2,3],
///         output_index: 0, activation: ActivationFunction::None,
///     },
///     MACScheduleEntry {
///         cycle: 2, operation: MACOperation::Reduce,
///         input_indices: vec![], weight_indices: vec![],
///         output_index: 0, activation: ActivationFunction::None,
///     },
///     MACScheduleEntry {
///         cycle: 3, operation: MACOperation::StoreOutput,
///         input_indices: vec![], weight_indices: vec![],
///         output_index: 0, activation: ActivationFunction::None,
///     },
/// ];
/// engine.load_schedule(schedule);
/// let traces = engine.run(10000).unwrap();
/// // output[0] = 1*0.5 + 2*0.5 + 3*0.5 + 4*0.5 = 5.0
/// ```
pub struct MACArrayEngine {
    config: MACArrayConfig,
    cycle: u64,
    /// Buffers: simple vectors of float values.
    /// In real hardware, these are on-chip SRAM banks.
    input_buffer: Vec<f64>,
    weight_buffer: Vec<f64>,
    output_buffer: Vec<f64>,
    /// MAC accumulators: one per MAC unit.
    mac_accumulators: Vec<f64>,
    /// The compiler-generated schedule.
    schedule: Vec<MACScheduleEntry>,
    halted: bool,
}

impl MACArrayEngine {
    /// Create a new MACArrayEngine with the given configuration.
    pub fn new(config: MACArrayConfig) -> Self {
        let input_buffer = vec![0.0; config.input_buffer_size];
        let weight_buffer = vec![0.0; config.weight_buffer_size];
        let output_buffer = vec![0.0; config.output_buffer_size];
        let mac_accumulators = vec![0.0; config.num_macs];

        Self {
            config,
            cycle: 0,
            input_buffer,
            weight_buffer,
            output_buffer,
            mac_accumulators,
            schedule: Vec::new(),
            halted: false,
        }
    }

    /// The configuration this engine was created with.
    pub fn config(&self) -> &MACArrayConfig {
        &self.config
    }

    /// Load activation data into the input buffer.
    pub fn load_inputs(&mut self, data: &[f64]) {
        for (i, &val) in data.iter().enumerate() {
            if i < self.config.input_buffer_size {
                self.input_buffer[i] = val;
            }
        }
    }

    /// Load weight data into the weight buffer.
    pub fn load_weights(&mut self, data: &[f64]) {
        for (i, &val) in data.iter().enumerate() {
            if i < self.config.weight_buffer_size {
                self.weight_buffer[i] = val;
            }
        }
    }

    /// Load a compiler-generated execution schedule.
    pub fn load_schedule(&mut self, schedule: Vec<MACScheduleEntry>) {
        self.schedule = schedule;
        self.halted = false;
    }

    /// Run the full schedule.
    pub fn run(&mut self, max_cycles: usize) -> Result<Vec<EngineTrace>, String> {
        let mut traces = Vec::new();
        for _ in 0..max_cycles {
            let trace = self.step();
            traces.push(trace);
            if self.halted {
                break;
            }
        }
        if !self.halted {
            return Err(format!(
                "MACArrayEngine: max_cycles ({}) reached",
                max_cycles
            ));
        }
        Ok(traces)
    }

    /// Read results from the output buffer.
    pub fn read_outputs(&self) -> &[f64] {
        &self.output_buffer
    }

    // --- Operation implementations ---

    fn exec_load_input(&self, entry: &MACScheduleEntry) -> String {
        format!("LOAD_INPUT indices={:?}", entry.input_indices)
    }

    fn exec_load_weights(&self, entry: &MACScheduleEntry) -> String {
        format!("LOAD_WEIGHTS indices={:?}", entry.weight_indices)
    }

    /// Execute a MAC operation: multiply input[i] * weight[i] for each MAC.
    fn exec_mac(&mut self, entry: &MACScheduleEntry) -> (String, HashMap<usize, String>) {
        let mut unit_traces = HashMap::new();
        let num_ops = entry
            .input_indices
            .len()
            .min(entry.weight_indices.len())
            .min(self.config.num_macs);

        for mac_id in 0..num_ops {
            let in_idx = entry.input_indices[mac_id];
            let wt_idx = entry.weight_indices[mac_id];

            let in_val = self.input_buffer[in_idx];
            let wt_val = self.weight_buffer[wt_idx];

            let result = in_val * wt_val;
            self.mac_accumulators[mac_id] = result;

            unit_traces.insert(
                mac_id,
                format!("MAC: {:.4} * {:.4} = {:.4}", in_val, wt_val, result),
            );
        }

        (format!("MAC {} operations", num_ops), unit_traces)
    }

    /// Execute a REDUCE operation: sum all MAC accumulators.
    ///
    /// The adder tree sums the MAC results into one value and writes
    /// it to the output buffer at the specified index.
    ///
    /// In real hardware, this is a tree of adders:
    ///
    /// ```text
    /// MAC0 + MAC1 -> sum01
    /// MAC2 + MAC3 -> sum23
    /// sum01 + sum23 -> final
    /// ```
    fn exec_reduce(&mut self, entry: &MACScheduleEntry) -> String {
        let total: f64 = self.mac_accumulators.iter().sum();
        let out_idx = entry.output_index;
        if out_idx < self.config.output_buffer_size {
            self.output_buffer[out_idx] = total;
        }
        format!("REDUCE sum={:.4} -> output[{}]", total, out_idx)
    }

    /// Execute an ACTIVATE operation: apply activation function.
    ///
    /// Activation functions:
    ///
    /// ```text
    /// None:    f(x) = x
    /// ReLU:    f(x) = max(0, x)
    /// Sigmoid: f(x) = 1 / (1 + e^-x)
    /// Tanh:    f(x) = tanh(x)
    /// ```
    fn exec_activate(&mut self, entry: &MACScheduleEntry) -> String {
        if !self.config.has_activation_unit {
            return "ACTIVATE skipped (no hardware activation unit)".to_string();
        }

        let out_idx = entry.output_index;
        if out_idx >= self.config.output_buffer_size {
            return format!("ACTIVATE error: index {} out of range", out_idx);
        }

        let val = self.output_buffer[out_idx];

        let result = match entry.activation {
            ActivationFunction::None => val,
            ActivationFunction::Relu => val.max(0.0),
            ActivationFunction::Sigmoid => {
                let clamped = val.clamp(-500.0, 500.0);
                1.0 / (1.0 + (-clamped).exp())
            }
            ActivationFunction::Tanh => val.tanh(),
        };

        self.output_buffer[out_idx] = result;
        format!(
            "ACTIVATE {}({:.4}) = {:.4}",
            entry.activation.name(),
            val,
            result
        )
    }

    fn exec_store(&self, entry: &MACScheduleEntry) -> String {
        let out_idx = entry.output_index;
        let val = if out_idx < self.config.output_buffer_size {
            self.output_buffer[out_idx]
        } else {
            0.0
        };
        format!("STORE_OUTPUT output[{}] = {:.4}", out_idx, val)
    }

    fn make_idle_trace(&self, description: &str) -> EngineTrace {
        EngineTrace {
            cycle: self.cycle,
            engine_name: self.name().to_string(),
            execution_model: ExecutionModel::ScheduledMac,
            description: description.to_string(),
            unit_traces: HashMap::new(),
            active_mask: vec![false; self.config.num_macs],
            active_count: 0,
            total_count: self.config.num_macs,
            utilization: 0.0,
            divergence_info: None,
            dataflow_info: None,
        }
    }
}

impl ParallelExecutionEngine for MACArrayEngine {
    fn name(&self) -> &str {
        "MACArrayEngine"
    }

    fn width(&self) -> usize {
        self.config.num_macs
    }

    fn execution_model(&self) -> ExecutionModel {
        ExecutionModel::ScheduledMac
    }

    /// Execute one scheduled cycle.
    ///
    /// Looks up the current cycle in the schedule and executes the
    /// corresponding operation. If no entry exists for this cycle,
    /// the MAC array idles (like a NOP).
    fn step(&mut self) -> EngineTrace {
        self.cycle += 1;

        if self.halted {
            return self.make_idle_trace("Schedule complete");
        }

        // Find schedule entries for this cycle
        let entries: Vec<MACScheduleEntry> = self
            .schedule
            .iter()
            .filter(|e| e.cycle == self.cycle)
            .cloned()
            .collect();

        if entries.is_empty() {
            // Check if we've passed all schedule entries
            let max_cycle = self.schedule.iter().map(|e| e.cycle).max().unwrap_or(0);
            if self.cycle > max_cycle {
                self.halted = true;
                return self.make_idle_trace("Schedule complete");
            }
            return self.make_idle_trace("No operation this cycle");
        }

        // Execute all entries for this cycle
        let mut unit_traces: HashMap<usize, String> = HashMap::new();
        let mut active_count = 0;
        let mut descriptions: Vec<String> = Vec::new();

        for entry in &entries {
            match entry.operation {
                MACOperation::LoadInput => {
                    let desc = self.exec_load_input(entry);
                    descriptions.push(desc);
                    active_count = entry.input_indices.len();
                }
                MACOperation::LoadWeights => {
                    let desc = self.exec_load_weights(entry);
                    descriptions.push(desc);
                    active_count = entry.weight_indices.len();
                }
                MACOperation::Mac => {
                    let (desc, traces) = self.exec_mac(entry);
                    descriptions.push(desc);
                    unit_traces.extend(traces);
                    active_count = unit_traces.len();
                }
                MACOperation::Reduce => {
                    let desc = self.exec_reduce(entry);
                    descriptions.push(desc);
                    active_count = 1;
                }
                MACOperation::Activate => {
                    let desc = self.exec_activate(entry);
                    descriptions.push(desc);
                    active_count = 1;
                }
                MACOperation::StoreOutput => {
                    let desc = self.exec_store(entry);
                    descriptions.push(desc);
                    active_count = 1;
                }
            }
        }

        let total = self.config.num_macs;
        let description = descriptions.join("; ");

        EngineTrace {
            cycle: self.cycle,
            engine_name: self.name().to_string(),
            execution_model: ExecutionModel::ScheduledMac,
            description: format!("{} -- {}/{} MACs active", description, active_count, total),
            unit_traces,
            active_mask: (0..total).map(|i| i < active_count).collect(),
            active_count,
            total_count: total,
            utilization: if total > 0 {
                active_count as f64 / total as f64
            } else {
                0.0
            },
            divergence_info: None,
            dataflow_info: None,
        }
    }

    fn halted(&self) -> bool {
        self.halted
    }

    /// Reset to initial state.
    fn reset(&mut self) {
        self.input_buffer = vec![0.0; self.config.input_buffer_size];
        self.weight_buffer = vec![0.0; self.config.weight_buffer_size];
        self.output_buffer = vec![0.0; self.config.output_buffer_size];
        self.mac_accumulators = vec![0.0; self.config.num_macs];
        self.halted = false;
        self.cycle = 0;
    }
}

impl std::fmt::Debug for MACArrayEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MACArrayEngine(num_macs={}, cycle={}, halted={})",
            self.config.num_macs, self.cycle, self.halted
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_dot_product_schedule() -> Vec<MACScheduleEntry> {
        vec![
            MACScheduleEntry {
                cycle: 1,
                operation: MACOperation::Mac,
                input_indices: vec![0, 1, 2, 3],
                weight_indices: vec![0, 1, 2, 3],
                output_index: 0,
                activation: ActivationFunction::None,
            },
            MACScheduleEntry {
                cycle: 2,
                operation: MACOperation::Reduce,
                input_indices: vec![],
                weight_indices: vec![],
                output_index: 0,
                activation: ActivationFunction::None,
            },
            MACScheduleEntry {
                cycle: 3,
                operation: MACOperation::StoreOutput,
                input_indices: vec![],
                weight_indices: vec![],
                output_index: 0,
                activation: ActivationFunction::None,
            },
        ]
    }

    #[test]
    fn test_mac_creation() {
        let engine = MACArrayEngine::new(MACArrayConfig::default());
        assert_eq!(engine.width(), 8);
        assert_eq!(engine.name(), "MACArrayEngine");
        assert_eq!(engine.execution_model(), ExecutionModel::ScheduledMac);
        assert!(!engine.halted());
    }

    #[test]
    fn test_mac_dot_product() {
        let mut engine = MACArrayEngine::new(MACArrayConfig::default());
        engine.load_inputs(&[1.0, 2.0, 3.0, 4.0]);
        engine.load_weights(&[0.5, 0.5, 0.5, 0.5]);
        engine.load_schedule(make_dot_product_schedule());

        let traces = engine.run(10000).unwrap();
        assert!(engine.halted());
        // 1*0.5 + 2*0.5 + 3*0.5 + 4*0.5 = 5.0
        assert!((engine.read_outputs()[0] - 5.0).abs() < 0.01);
        assert!(!traces.is_empty());
    }

    #[test]
    fn test_mac_activation_relu() {
        let mut engine = MACArrayEngine::new(MACArrayConfig::default());
        engine.load_inputs(&[1.0, 2.0]);
        engine.load_weights(&[-1.0, -1.0]);
        let schedule = vec![
            MACScheduleEntry {
                cycle: 1,
                operation: MACOperation::Mac,
                input_indices: vec![0, 1],
                weight_indices: vec![0, 1],
                output_index: 0,
                activation: ActivationFunction::None,
            },
            MACScheduleEntry {
                cycle: 2,
                operation: MACOperation::Reduce,
                input_indices: vec![],
                weight_indices: vec![],
                output_index: 0,
                activation: ActivationFunction::None,
            },
            MACScheduleEntry {
                cycle: 3,
                operation: MACOperation::Activate,
                input_indices: vec![],
                weight_indices: vec![],
                output_index: 0,
                activation: ActivationFunction::Relu,
            },
            MACScheduleEntry {
                cycle: 4,
                operation: MACOperation::StoreOutput,
                input_indices: vec![],
                weight_indices: vec![],
                output_index: 0,
                activation: ActivationFunction::None,
            },
        ];
        engine.load_schedule(schedule);
        engine.run(10000).unwrap();

        // 1*(-1) + 2*(-1) = -3.0, relu(-3) = 0.0
        assert_eq!(engine.read_outputs()[0], 0.0);
    }

    #[test]
    fn test_mac_activation_sigmoid() {
        let mut engine = MACArrayEngine::new(MACArrayConfig::default());
        // Pre-load output buffer directly for testing
        engine.output_buffer[0] = 0.0;
        let schedule = vec![MACScheduleEntry {
            cycle: 1,
            operation: MACOperation::Activate,
            input_indices: vec![],
            weight_indices: vec![],
            output_index: 0,
            activation: ActivationFunction::Sigmoid,
        }];
        engine.load_schedule(schedule);
        engine.step();
        // sigmoid(0) = 0.5
        assert!((engine.read_outputs()[0] - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_mac_reset() {
        let mut engine = MACArrayEngine::new(MACArrayConfig::default());
        engine.load_inputs(&[1.0, 2.0, 3.0, 4.0]);
        engine.load_weights(&[0.5, 0.5, 0.5, 0.5]);
        engine.load_schedule(make_dot_product_schedule());
        engine.run(10000).unwrap();

        engine.reset();
        assert!(!engine.halted());
        assert_eq!(engine.read_outputs()[0], 0.0);
    }

    #[test]
    fn test_mac_idle_trace() {
        let mut engine = MACArrayEngine::new(MACArrayConfig::default());
        // No schedule loaded -- should halt immediately
        engine.load_schedule(vec![]);
        let trace = engine.step();
        assert!(trace.description.contains("Schedule complete"));
    }

    #[test]
    fn test_mac_debug_format() {
        let engine = MACArrayEngine::new(MACArrayConfig::default());
        let debug = format!("{:?}", engine);
        assert!(debug.contains("MACArrayEngine"));
        assert!(debug.contains("num_macs=8"));
    }

    #[test]
    fn test_activation_function_names() {
        assert_eq!(ActivationFunction::None.name(), "none");
        assert_eq!(ActivationFunction::Relu.name(), "relu");
        assert_eq!(ActivationFunction::Sigmoid.name(), "sigmoid");
        assert_eq!(ActivationFunction::Tanh.name(), "tanh");
    }
}
