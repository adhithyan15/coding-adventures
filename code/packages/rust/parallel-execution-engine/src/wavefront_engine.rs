//! WavefrontEngine -- SIMD parallel execution (AMD GCN/RDNA style).
//!
//! # What is a Wavefront?
//!
//! AMD calls their parallel execution unit a "wavefront." It's 64 lanes on GCN
//! (Graphics Core Next) or 32 lanes on RDNA (Radeon DNA). A wavefront is
//! fundamentally different from an NVIDIA warp:
//!
//! ```text
//! NVIDIA Warp (SIMT):                AMD Wavefront (SIMD):
//! +--------------------------+       +--------------------------+
//! | 32 threads               |       | 32 lanes                 |
//! | Each has its own regs    |       | ONE vector register file  |
//! | Logically own PC         |       | ONE program counter       |
//! | HW manages divergence    |       | Explicit EXEC mask        |
//! +--------------------------+       +--------------------------+
//! ```
//!
//! The critical architectural difference:
//!
//! ```text
//! SIMT (NVIDIA): "32 independent threads that HAPPEN to run together"
//! SIMD (AMD):    "1 instruction that operates on a 32-wide vector"
//! ```
//!
//! # AMD's Two Register Files
//!
//! AMD wavefronts have TWO types of registers, which is architecturally unique:
//!
//! ```text
//! Vector GPRs (VGPRs):              Scalar GPRs (SGPRs):
//! +------------------------+        +------------------------+
//! | v0: [l0][l1]...[l31]  |        | s0:  42.0              |
//! | v1: [l0][l1]...[l31]  |        | s1:  3.14              |
//! | ...                    |        | ...                    |
//! | v255:[l0][l1]...[l31]  |        | s103: 0.0              |
//! +------------------------+        +------------------------+
//! One value PER LANE                One value for ALL LANES
//! ```
//!
//! SGPRs are used for values that are the SAME across all lanes: constants,
//! loop counters, memory base addresses. This is efficient -- compute the
//! address ONCE in scalar, then use it in every lane.
//!
//! # The EXEC Mask
//!
//! AMD uses a register called EXEC to control which lanes execute each
//! instruction. Unlike NVIDIA's hardware-managed divergence, the EXEC mask
//! is explicitly set by instructions:
//!
//! ```text
//! v_cmp_lt_f32 vcc, v0, s0        // Compare: which lanes have v0 < s0?
//! s_and_saveexec_b32 s[2:3], vcc  // EXEC = EXEC & vcc, save old EXEC
//! // ... only lanes where v0 < s0 execute here ...
//! s_or_b32 exec, exec, s[2:3]     // Restore full EXEC mask
//! ```
//!
//! # Simplification for Our Simulator
//!
//! For educational clarity, we use GPUCore instances per lane internally
//! (just like WarpEngine), but expose the AMD-style interface externally:
//! vector registers, scalar registers, and explicit EXEC mask.

use std::collections::HashMap;

use gpu_core::{GPUCore, GenericISA, Instruction, ProcessingElement};
use fp_arithmetic::{FloatFormat, FP32};

use crate::protocols::{
    DivergenceInfo, EngineTrace, ExecutionModel, ParallelExecutionEngine,
};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for an AMD-style SIMD wavefront engine.
///
/// Real-world reference values:
///
/// ```text
/// Architecture | Wave Width | VGPRs | SGPRs | LDS
/// -------------+------------+-------+-------+---------
/// AMD GCN      | 64         | 256   | 104   | 64 KB
/// AMD RDNA     | 32         | 256   | 104   | 64 KB
/// Our default  | 32         | 256   | 104   | 64 KB
/// ```
#[derive(Debug, Clone)]
pub struct WavefrontConfig {
    /// Number of SIMD lanes (64 for GCN, 32 for RDNA).
    pub wave_width: usize,
    /// Vector general-purpose registers per lane.
    pub num_vgprs: usize,
    /// Scalar general-purpose registers (shared by all lanes).
    pub num_sgprs: usize,
    /// Local Data Store size in bytes (shared memory).
    pub lds_size: usize,
    /// FP format for register values.
    pub float_format: FloatFormat,
}

impl Default for WavefrontConfig {
    fn default() -> Self {
        Self {
            wave_width: 32,
            num_vgprs: 256,
            num_sgprs: 104,
            lds_size: 65536,
            float_format: FP32,
        }
    }
}

// ---------------------------------------------------------------------------
// Vector Register File -- one value per lane per register
// ---------------------------------------------------------------------------

/// AMD-style vector register file: num_vgprs registers x wave_width lanes.
///
/// Each "register" is actually a vector of wave_width values. When you
/// write to v3[lane 5], you're writing to one slot in a 2D array:
///
/// ```text
/// +--------------------------------------------+
/// |         Lane 0   Lane 1   Lane 2  ...      |
/// | v0:    [ 1.0  ] [ 2.0  ] [ 3.0  ]  ...    |
/// | v1:    [ 0.5  ] [ 0.5  ] [ 0.5  ]  ...    |
/// | v2:    [ 0.0  ] [ 0.0  ] [ 0.0  ]  ...    |
/// | ...                                        |
/// +--------------------------------------------+
/// ```
///
/// This is fundamentally different from NVIDIA where each thread has
/// its own separate register file. Here, ALL lanes share ONE register
/// file, but each lane gets its own "column" within each register.
pub struct VectorRegisterFile {
    pub num_vgprs: usize,
    pub wave_width: usize,
    /// The floating-point format for this register file.
    pub fmt: FloatFormat,
    /// 2D storage: data[reg_index][lane_index]
    data: Vec<Vec<f64>>,
}

impl VectorRegisterFile {
    /// Create a new vector register file with all values initialized to 0.0.
    pub fn new(num_vgprs: usize, wave_width: usize, fmt: FloatFormat) -> Self {
        Self {
            num_vgprs,
            wave_width,
            fmt,
            data: vec![vec![0.0; wave_width]; num_vgprs],
        }
    }

    /// Read one lane of a vector register as a float.
    pub fn read(&self, vreg: usize, lane: usize) -> f64 {
        self.data[vreg][lane]
    }

    /// Write a float to one lane of a vector register.
    pub fn write(&mut self, vreg: usize, lane: usize, value: f64) {
        self.data[vreg][lane] = value;
    }

    /// Read all lanes of a vector register.
    pub fn read_all_lanes(&self, vreg: usize) -> Vec<f64> {
        self.data[vreg].clone()
    }
}

// ---------------------------------------------------------------------------
// Scalar Register File -- one value shared across all lanes
// ---------------------------------------------------------------------------

/// AMD-style scalar register file: num_sgprs single-value registers.
///
/// Scalar registers hold values that are the SAME for all lanes:
/// constants, loop counters, memory base addresses. Computing these
/// once in scalar instead of per-lane saves power and register space.
///
/// ```text
/// +-------------------------+
/// | s0:   42.0              |  <- same for all lanes
/// | s1:   3.14159           |
/// | s2:   0.0               |
/// | ...                     |
/// | s103: 0.0               |
/// +-------------------------+
/// ```
pub struct ScalarRegisterFile {
    pub num_sgprs: usize,
    data: Vec<f64>,
}

impl ScalarRegisterFile {
    /// Create a new scalar register file with all values initialized to 0.0.
    pub fn new(num_sgprs: usize) -> Self {
        Self {
            num_sgprs,
            data: vec![0.0; num_sgprs],
        }
    }

    /// Read a scalar register.
    pub fn read(&self, sreg: usize) -> f64 {
        self.data[sreg]
    }

    /// Write to a scalar register.
    pub fn write(&mut self, sreg: usize, value: f64) {
        self.data[sreg] = value;
    }
}

// ---------------------------------------------------------------------------
// WavefrontEngine -- the SIMD parallel execution engine
// ---------------------------------------------------------------------------

/// SIMD wavefront execution engine (AMD GCN/RDNA style).
///
/// One instruction stream, one wide vector ALU, explicit EXEC mask.
/// Internally uses GPUCore per lane for instruction execution, but
/// exposes the AMD-style vector/scalar register interface.
///
/// # Key Differences from WarpEngine
///
/// 1. ONE program counter (not per-thread PCs).
/// 2. Vector registers are a 2D array (vreg x lane), not per-thread.
/// 3. Scalar registers are shared across all lanes.
/// 4. EXEC mask is explicitly controlled, not hardware-managed.
/// 5. No divergence stack -- mask management is programmer/compiler's job.
///
/// # Example
///
/// ```
/// use gpu_core::opcodes::{limm, fmul, halt};
/// use parallel_execution_engine::wavefront_engine::{WavefrontEngine, WavefrontConfig};
///
/// let mut config = WavefrontConfig::default();
/// config.wave_width = 4;
/// let mut engine = WavefrontEngine::new(config);
/// engine.load_program(vec![limm(0, 2.0), fmul(2, 0, 1), halt()]);
/// for lane in 0..4 {
///     engine.set_lane_register(lane, 1, (lane as f64) + 1.0);
/// }
/// let traces = engine.run(10000).unwrap();
/// ```
pub struct WavefrontEngine {
    config: WavefrontConfig,
    cycle: u64,
    program: Vec<Instruction>,
    /// The EXEC mask: true = lane is active, false = lane is masked off.
    exec_mask: Vec<bool>,
    /// Vector and scalar register files (AMD-style).
    vrf: VectorRegisterFile,
    srf: ScalarRegisterFile,
    /// Internal: one GPUCore per lane for instruction execution.
    lanes: Vec<GPUCore>,
    all_halted: bool,
}

impl WavefrontEngine {
    /// Create a new WavefrontEngine with the given configuration.
    pub fn new(config: WavefrontConfig) -> Self {
        let exec_mask = vec![true; config.wave_width];
        let vrf = VectorRegisterFile::new(
            config.num_vgprs,
            config.wave_width,
            config.float_format,
        );
        let srf = ScalarRegisterFile::new(config.num_sgprs);
        let lanes: Vec<GPUCore> = (0..config.wave_width)
            .map(|_| {
                GPUCore::with_config(
                    Box::new(GenericISA),
                    config.float_format,
                    config.num_vgprs.min(256),
                    config.lds_size / config.wave_width.max(1),
                )
            })
            .collect();

        Self {
            config,
            cycle: 0,
            program: Vec::new(),
            exec_mask,
            vrf,
            srf,
            lanes,
            all_halted: false,
        }
    }

    /// The current EXEC mask (which lanes are active).
    pub fn exec_mask(&self) -> &[bool] {
        &self.exec_mask
    }

    /// The configuration this engine was created with.
    pub fn config(&self) -> &WavefrontConfig {
        &self.config
    }

    /// Access to the vector register file.
    pub fn vrf(&self) -> &VectorRegisterFile {
        &self.vrf
    }

    /// Access to the scalar register file.
    pub fn srf(&self) -> &ScalarRegisterFile {
        &self.srf
    }

    /// Load a program into the wavefront.
    ///
    /// The same program is loaded into all lane cores. Unlike SIMT where
    /// each thread can (logically) have a different PC, the wavefront has
    /// ONE shared PC for all lanes.
    pub fn load_program(&mut self, program: Vec<Instruction>) {
        self.program = program.clone();
        for lane in &mut self.lanes {
            lane.load_program(program.clone());
        }
        self.exec_mask = vec![true; self.config.wave_width];
        self.all_halted = false;
        self.cycle = 0;
    }

    /// Set a per-lane vector register value.
    ///
    /// This writes to both the VRF (our AMD-style register file) and
    /// the internal GPUCore for that lane (for execution).
    ///
    /// # Panics
    ///
    /// Panics if `lane` is out of range.
    pub fn set_lane_register(&mut self, lane: usize, vreg: usize, value: f64) {
        assert!(
            lane < self.config.wave_width,
            "Lane {} out of range [0, {})",
            lane,
            self.config.wave_width
        );
        self.vrf.write(vreg, lane, value);
        self.lanes[lane].registers.write_float(vreg, value);
    }

    /// Set a scalar register value (shared across all lanes).
    ///
    /// # Panics
    ///
    /// Panics if `sreg` is out of range.
    pub fn set_scalar_register(&mut self, sreg: usize, value: f64) {
        assert!(
            sreg < self.config.num_sgprs,
            "Scalar register {} out of range [0, {})",
            sreg,
            self.config.num_sgprs
        );
        self.srf.write(sreg, value);
    }

    /// Explicitly set the EXEC mask.
    ///
    /// In AMD hardware, the EXEC mask is set by comparison instructions.
    /// In our simulator, you can set it directly for testing.
    ///
    /// # Panics
    ///
    /// Panics if the mask length doesn't match the wave width.
    pub fn set_exec_mask(&mut self, mask: Vec<bool>) {
        assert_eq!(
            mask.len(),
            self.config.wave_width,
            "Mask length {} != wave_width {}",
            mask.len(),
            self.config.wave_width
        );
        self.exec_mask = mask;
    }

    /// Run until all lanes halt or max_cycles reached.
    pub fn run(&mut self, max_cycles: usize) -> Result<Vec<EngineTrace>, String> {
        let mut traces = Vec::new();
        for _ in 0..max_cycles {
            let trace = self.step();
            traces.push(trace);
            if self.all_halted {
                break;
            }
        }
        if !self.all_halted {
            return Err(format!(
                "WavefrontEngine: max_cycles ({}) reached",
                max_cycles
            ));
        }
        Ok(traces)
    }

    /// Produce a trace for when all lanes are halted.
    fn make_halted_trace(&self) -> EngineTrace {
        let mut unit_traces = HashMap::new();
        for i in 0..self.config.wave_width {
            unit_traces.insert(i, "(halted)".to_string());
        }
        EngineTrace {
            cycle: self.cycle,
            engine_name: self.name().to_string(),
            execution_model: ExecutionModel::Simd,
            description: "All lanes halted".to_string(),
            unit_traces,
            active_mask: vec![false; self.config.wave_width],
            active_count: 0,
            total_count: self.config.wave_width,
            utilization: 0.0,
            divergence_info: None,
            dataflow_info: None,
        }
    }
}

impl ParallelExecutionEngine for WavefrontEngine {
    fn name(&self) -> &str {
        "WavefrontEngine"
    }

    fn width(&self) -> usize {
        self.config.wave_width
    }

    fn execution_model(&self) -> ExecutionModel {
        ExecutionModel::Simd
    }

    /// Execute one cycle: issue one instruction to all active lanes.
    ///
    /// Unlike SIMT, ALL lanes share the same PC. The EXEC mask determines
    /// which lanes actually execute. Masked-off lanes don't update their
    /// registers, but the PC still advances for the whole wavefront.
    fn step(&mut self) -> EngineTrace {
        self.cycle += 1;

        if self.all_halted {
            return self.make_halted_trace();
        }

        let mask_before = self.exec_mask.clone();

        // Execute on all lanes (masked-off lanes still step to keep PCs in sync)
        let mut unit_traces: HashMap<usize, String> = HashMap::new();

        for lane_id in 0..self.config.wave_width {
            let lane_core = &mut self.lanes[lane_id];
            if self.exec_mask[lane_id] && !lane_core.halted() {
                match lane_core.step() {
                    Ok(trace) => {
                        if trace.halted {
                            unit_traces.insert(lane_id, "HALTED".to_string());
                        } else {
                            unit_traces.insert(lane_id, trace.description);
                        }
                    }
                    Err(_) => {
                        unit_traces.insert(lane_id, "(error)".to_string());
                    }
                }
            } else if lane_core.halted() {
                unit_traces.insert(lane_id, "(halted)".to_string());
            } else {
                // Lane is masked off -- still advance its PC to stay in sync.
                // In real AMD HW, masked lanes simply skip the write-back.
                if !lane_core.halted() {
                    match lane_core.step() {
                        Ok(_) => {
                            unit_traces
                                .insert(lane_id, "(masked -- result discarded)".to_string());
                        }
                        Err(_) => {
                            unit_traces.insert(lane_id, "(masked -- error)".to_string());
                        }
                    }
                } else {
                    unit_traces.insert(lane_id, "(halted)".to_string());
                }
            }
        }

        // Sync VRF with internal core registers for active lanes
        for lane_id in 0..self.config.wave_width {
            if self.exec_mask[lane_id] {
                for vreg in 0..self.config.num_vgprs.min(32) {
                    let val = self.lanes[lane_id].registers.read_float(vreg);
                    self.vrf.write(vreg, lane_id, val);
                }
            }
        }

        // Check if all lanes halted
        if self.lanes.iter().all(|lane| lane.halted()) {
            self.all_halted = true;
        }

        let active_count = (0..self.config.wave_width)
            .filter(|&i| self.exec_mask[i] && !self.lanes[i].halted())
            .count();
        let total = self.config.wave_width;

        // Build description from first active lane
        let first_desc = (0..self.config.wave_width)
            .find_map(|i| {
                unit_traces.get(&i).and_then(|desc| {
                    if desc != "(masked -- result discarded)"
                        && desc != "(halted)"
                        && desc != "(error)"
                        && desc != "(masked -- error)"
                        && desc != "HALTED"
                    {
                        Some(desc.clone())
                    } else {
                        None
                    }
                })
            })
            .unwrap_or_else(|| "no active lanes".to_string());

        let current_mask: Vec<bool> = (0..self.config.wave_width)
            .map(|i| self.exec_mask[i] && !self.lanes[i].halted())
            .collect();

        EngineTrace {
            cycle: self.cycle,
            engine_name: self.name().to_string(),
            execution_model: ExecutionModel::Simd,
            description: format!("{} -- {}/{} lanes active", first_desc, active_count, total),
            unit_traces,
            active_mask: current_mask,
            active_count,
            total_count: total,
            utilization: if total > 0 {
                active_count as f64 / total as f64
            } else {
                0.0
            },
            divergence_info: Some(DivergenceInfo {
                active_mask_before: mask_before,
                active_mask_after: self.exec_mask.clone(),
                reconvergence_pc: -1,
                divergence_depth: 0,
            }),
            dataflow_info: None,
        }
    }

    fn halted(&self) -> bool {
        self.all_halted
    }

    /// Reset to initial state.
    fn reset(&mut self) {
        for lane in &mut self.lanes {
            lane.reset();
            if !self.program.is_empty() {
                lane.load_program(self.program.clone());
            }
        }
        self.exec_mask = vec![true; self.config.wave_width];
        self.all_halted = false;
        self.cycle = 0;
        self.vrf = VectorRegisterFile::new(
            self.config.num_vgprs,
            self.config.wave_width,
            self.config.float_format,
        );
        self.srf = ScalarRegisterFile::new(self.config.num_sgprs);
    }
}

impl std::fmt::Debug for WavefrontEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let active = self.exec_mask.iter().filter(|&&m| m).count();
        write!(
            f,
            "WavefrontEngine(width={}, active_lanes={}, halted={})",
            self.config.wave_width, active, self.all_halted
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpu_core::opcodes::{fadd, fmul, halt, limm};

    #[test]
    fn test_wavefront_creation() {
        let mut config = WavefrontConfig::default();
        config.wave_width = 4;
        let engine = WavefrontEngine::new(config);
        assert_eq!(engine.width(), 4);
        assert_eq!(engine.name(), "WavefrontEngine");
        assert_eq!(engine.execution_model(), ExecutionModel::Simd);
        assert!(!engine.halted());
    }

    #[test]
    fn test_wavefront_simple_program() {
        let mut config = WavefrontConfig::default();
        config.wave_width = 4;
        let mut engine = WavefrontEngine::new(config);
        engine.load_program(vec![
            limm(0, 2.0),
            limm(1, 3.0),
            fmul(2, 0, 1),
            halt(),
        ]);

        let traces = engine.run(1000).unwrap();
        assert!(engine.halted());
        // All 4 lanes should have computed R2 = 6.0
        for lane_id in 0..4 {
            assert_eq!(engine.vrf().read(2, lane_id), 6.0);
        }
        assert!(traces.len() >= 4);
    }

    #[test]
    fn test_wavefront_per_lane_data() {
        let mut config = WavefrontConfig::default();
        config.wave_width = 4;
        let mut engine = WavefrontEngine::new(config);
        engine.load_program(vec![fadd(2, 0, 1), halt()]);

        for lane in 0..4 {
            engine.set_lane_register(lane, 0, (lane as f64) * 10.0);
            engine.set_lane_register(lane, 1, 1.0);
        }

        engine.run(1000).unwrap();
        assert!(engine.halted());

        assert_eq!(engine.vrf().read(2, 0), 1.0);
        assert_eq!(engine.vrf().read(2, 1), 11.0);
        assert_eq!(engine.vrf().read(2, 2), 21.0);
        assert_eq!(engine.vrf().read(2, 3), 31.0);
    }

    #[test]
    fn test_wavefront_exec_mask() {
        let mut config = WavefrontConfig::default();
        config.wave_width = 4;
        let mut engine = WavefrontEngine::new(config);
        engine.load_program(vec![limm(0, 42.0), halt()]);

        // Mask off lanes 2 and 3
        engine.set_exec_mask(vec![true, true, false, false]);

        let trace = engine.step();
        // Only 2 lanes should be reported as active
        assert_eq!(trace.active_count, 2);
    }

    #[test]
    fn test_wavefront_scalar_register() {
        let mut config = WavefrontConfig::default();
        config.wave_width = 4;
        let mut engine = WavefrontEngine::new(config);
        engine.set_scalar_register(0, 42.0);
        assert_eq!(engine.srf().read(0), 42.0);
    }

    #[test]
    fn test_wavefront_reset() {
        let mut config = WavefrontConfig::default();
        config.wave_width = 4;
        let mut engine = WavefrontEngine::new(config);
        engine.load_program(vec![limm(0, 42.0), halt()]);
        engine.run(1000).unwrap();
        assert!(engine.halted());

        engine.reset();
        assert!(!engine.halted());
        assert_eq!(engine.vrf().read(0, 0), 0.0);
    }

    #[test]
    fn test_wavefront_debug_format() {
        let mut config = WavefrontConfig::default();
        config.wave_width = 4;
        let engine = WavefrontEngine::new(config);
        let debug = format!("{:?}", engine);
        assert!(debug.contains("WavefrontEngine"));
        assert!(debug.contains("width=4"));
    }

    #[test]
    #[should_panic(expected = "Mask length")]
    fn test_wavefront_set_exec_mask_wrong_length() {
        let mut config = WavefrontConfig::default();
        config.wave_width = 4;
        let mut engine = WavefrontEngine::new(config);
        engine.set_exec_mask(vec![true, false]); // Wrong length
    }
}
