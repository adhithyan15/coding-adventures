//! MultiCoreCPU -- multiple cores sharing L3 cache and memory.
//!
//! # Architecture
//!
//! ```text
//!   Core 0: L1I + L1D + L2 (private)
//!   Core 1: L1I + L1D + L2 (private)
//!          |    |
//!   =============================
//!   Shared L3 Cache (optional)
//!   =============================
//!             |
//!   Memory Controller
//!             |
//!   Shared Main Memory (DRAM)
//! ```
//!
//! # Execution Model
//!
//! All cores run on the same clock. Each call to step() advances every core
//! by one cycle. Cores are independent -- they do not share register files
//! or pipeline state. They only interact through shared memory.
//!
//! # Cache Coherence
//!
//! This implementation does NOT model cache coherence (MESI protocol, etc.).
//! Writes by one core become visible to other cores only when they reach
//! main memory. Cache coherence is a future extension.

use cache::Cache;
use cpu_pipeline::PipelineSnapshot;

use crate::config::MultiCoreConfig;
use crate::core::Core;
use crate::decoder::ISADecoder;
use crate::interrupt_controller::InterruptController;
use crate::stats::CoreStats;

/// Connects multiple processor cores to shared resources.
pub struct MultiCoreCPU {
    /// Multi-core configuration.
    #[allow(dead_code)]
    config: MultiCoreConfig,

    /// Array of processor cores.
    cores: Vec<Core>,

    /// Optional shared L3 cache.
    _l3_cache: Option<Cache>,

    /// Interrupt controller.
    interrupt_ctrl: InterruptController,

    /// Global cycle count.
    cycle: i64,
}

impl MultiCoreCPU {
    /// Creates a multi-core processor.
    ///
    /// All cores share the same main memory. Each core gets its own ISA decoder
    /// (from the decoders vec). If decoders.len() < num_cores, the last decoder
    /// pattern is reused (all get their own instance through the factory).
    ///
    /// Returns an error if any core fails to initialize.
    pub fn new(
        config: MultiCoreConfig,
        decoder_factory: &dyn Fn() -> Box<dyn ISADecoder>,
    ) -> Result<Self, String> {
        let mem_size = if config.memory_size == 0 {
            1048576
        } else {
            config.memory_size
        };
        let mem_latency = if config.memory_latency == 0 {
            100
        } else {
            config.memory_latency
        };

        // Optional shared L3 cache.
        let l3 = config.l3_cache.as_ref().map(|cfg| Cache::new(cfg.clone()));

        // Create cores.
        let num_cores = if config.num_cores == 0 { 1 } else { config.num_cores };
        let mut cores = Vec::with_capacity(num_cores);

        for _ in 0..num_cores {
            let mut core_cfg = config.core_config.clone();
            core_cfg.memory_size = mem_size;
            core_cfg.memory_latency = mem_latency;

            let decoder = decoder_factory();
            let c = Core::new(core_cfg, decoder)?;
            cores.push(c);
        }

        Ok(MultiCoreCPU {
            config,
            cores,
            _l3_cache: l3,
            interrupt_ctrl: InterruptController::new(num_cores),
            cycle: 0,
        })
    }

    /// Loads a program into memory for a specific core.
    ///
    /// The program is written to the core's local memory at the given address.
    /// The specified core's PC is set to `start_address`.
    pub fn load_program(&mut self, core_id: usize, program: &[u8], start_address: usize) {
        if core_id >= self.cores.len() {
            return;
        }
        self.cores[core_id].load_program(program, start_address);
    }

    /// Advances all cores by one clock cycle.
    ///
    /// Returns a pipeline snapshot from each core.
    pub fn step(&mut self) -> Vec<PipelineSnapshot> {
        self.cycle += 1;

        let mut snapshots = Vec::with_capacity(self.cores.len());
        for core in &mut self.cores {
            snapshots.push(core.step());
        }

        snapshots
    }

    /// Executes all cores until all have halted or `max_cycles` is reached.
    ///
    /// Returns per-core statistics.
    pub fn run(&mut self, max_cycles: i64) -> Vec<CoreStats> {
        while self.cycle < max_cycles {
            if self.all_halted() {
                break;
            }
            self.step();
        }
        self.stats()
    }

    /// Returns the array of cores (for direct access).
    pub fn cores(&self) -> &[Core] {
        &self.cores
    }

    /// Returns mutable access to the cores.
    pub fn cores_mut(&mut self) -> &mut [Core] {
        &mut self.cores
    }

    /// Returns per-core statistics.
    pub fn stats(&self) -> Vec<CoreStats> {
        self.cores.iter().map(|c| c.stats()).collect()
    }

    /// Returns a reference to the interrupt controller.
    pub fn interrupt_controller(&self) -> &InterruptController {
        &self.interrupt_ctrl
    }

    /// Returns a mutable reference to the interrupt controller.
    pub fn interrupt_controller_mut(&mut self) -> &mut InterruptController {
        &mut self.interrupt_ctrl
    }

    /// Returns the global cycle count.
    pub fn cycle(&self) -> i64 {
        self.cycle
    }

    /// Returns true if every core has halted.
    pub fn all_halted(&self) -> bool {
        self.cores.iter().all(|c| c.is_halted())
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{simple_config, MultiCoreConfig};
    use crate::decoder::{encode_addi, encode_halt, encode_program, MockDecoder};

    fn mock_decoder_factory() -> Box<dyn ISADecoder> {
        Box::new(MockDecoder::new())
    }

    fn make_multi_core(num_cores: usize) -> MultiCoreCPU {
        let config = MultiCoreConfig {
            num_cores,
            core_config: simple_config(),
            l3_cache: None,
            memory_size: 1048576,
            memory_latency: 100,
        };
        MultiCoreCPU::new(config, &mock_decoder_factory).expect("failed to create multi-core CPU")
    }

    #[test]
    fn test_multi_core_construction() {
        let mc = make_multi_core(2);
        assert_eq!(mc.cores().len(), 2);
        assert_eq!(mc.cycle(), 0);
        assert!(!mc.all_halted());
    }

    #[test]
    fn test_multi_core_default_config() {
        let config = MultiCoreConfig::default();
        let mc = MultiCoreCPU::new(config, &mock_decoder_factory)
            .expect("failed to create multi-core CPU");
        assert_eq!(mc.cores().len(), 2);
    }

    #[test]
    fn test_multi_core_run() {
        let mut mc = make_multi_core(2);

        // Load same program on both cores.
        let program = encode_program(&[encode_addi(1, 0, 42), encode_halt()]);
        mc.load_program(0, &program, 0);
        mc.load_program(1, &program, 0);

        let stats = mc.run(100);
        assert_eq!(stats.len(), 2);
        assert!(mc.all_halted());
    }

    #[test]
    fn test_multi_core_step() {
        let mut mc = make_multi_core(2);
        let program = encode_program(&[encode_halt()]);
        mc.load_program(0, &program, 0);
        mc.load_program(1, &program, 0);

        let snapshots = mc.step();
        assert_eq!(snapshots.len(), 2);
        assert_eq!(mc.cycle(), 1);
    }

    #[test]
    fn test_load_program_out_of_range_core() {
        let mut mc = make_multi_core(2);
        let program = encode_program(&[encode_halt()]);
        mc.load_program(99, &program, 0); // should not panic
    }

    #[test]
    fn test_interrupt_controller_access() {
        let mut mc = make_multi_core(2);
        mc.interrupt_controller_mut().raise_interrupt(0, 0);
        assert_eq!(mc.interrupt_controller().pending_count(), 1);
    }

    #[test]
    fn test_all_halted_false_initially() {
        let mc = make_multi_core(2);
        assert!(!mc.all_halted());
    }
}
