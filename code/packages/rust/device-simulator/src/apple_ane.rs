//! Apple Neural Engine -- device simulator with unified memory.
//!
//! # Apple ANE Architecture
//!
//! The Apple Neural Engine is radically different from GPUs and TPUs.
//! It's a fixed-function accelerator designed for neural network inference,
//! optimized for power efficiency over flexibility.
//!
//! ```text
//! +---------------------------------------------------+
//! |           Apple Neural Engine                      |
//! |  +---------------------------------------------+  |
//! |  |       DMA Controller (schedule replayer)     |  |
//! |  +------+-----+-----+------+------------------+  |
//! |         |     |     |      |                      |
//! |  +------+ +------+ +------+ +------+              |
//! |  |Core 0| |Core 1| |Core 2| |Core N|             |
//! |  | MAC  | | MAC  | | MAC  | | MAC  |             |
//! |  |Array | |Array | |Array | |Array |             |
//! |  +---+--+ +---+--+ +---+--+ +---+--+             |
//! |      +--------+--------+--------+                 |
//! |               |                                    |
//! |  +------------+--------------------------------+  |
//! |  |         Shared SRAM (32 MB)                  |  |
//! |  +------------+--------------------------------+  |
//! |               |                                    |
//! |  +------------+--------------------------------+  |
//! |  |   Unified Memory (shared with CPU & GPU)     |  |
//! |  |   No copy needed -- just remap page tables   |  |
//! |  +---------------------------------------------+  |
//! +---------------------------------------------------+
//! ```
//!
//! # Unified Memory: The Game Changer
//!
//! Apple's unified memory architecture means the ANE, CPU, and GPU all
//! share the same physical memory. When you "copy" data to the ANE, there's
//! no actual data movement -- just page table remapping.
//!
//! ```text
//! Discrete GPU: Copy 8 MB over PCIe -> 125 us overhead
//! Apple ANE:    Remap page tables -> ~0 us overhead
//! ```
//!
//! # Compiler-Driven Scheduling
//!
//! The ANE relies entirely on the CoreML compiler to generate a fixed
//! execution schedule. The hardware simply replays this schedule --
//! no dynamic scheduling overhead.

use compute_unit::{ComputeUnit as ComputeUnitTrait, NeuralEngineCore, ANECoreConfig};

use crate::global_memory::SimpleGlobalMemory;
use crate::protocols::{
    AcceleratorDevice, DeviceConfig, DeviceStats, DeviceTrace, KernelDescriptor,
};
use crate::work_distributor::ANEScheduleReplayer;

/// Apple Neural Engine device simulator.
///
/// Features unified memory (zero-copy host transfers), shared SRAM,
/// compiler-driven schedule replay, and DMA-based data movement.
pub struct AppleANE {
    /// Device configuration.
    config: DeviceConfig,
    /// NE cores, stored as trait objects.
    cores: Vec<Box<dyn ComputeUnitTrait>>,
    /// Global memory (unified -- zero-copy).
    global_memory: SimpleGlobalMemory,
    /// Schedule replayer (compiler-driven).
    replayer: ANEScheduleReplayer,
    /// Current simulation cycle.
    cycle: u64,
    /// Total kernel launches.
    kernels_launched: u64,
}

impl AppleANE {
    /// Create a new Apple ANE simulator.
    ///
    /// # Arguments
    ///
    /// * `config` - Full DeviceConfig (uses a small default if None).
    /// * `num_cores` - Number of NE cores (used only if config is None).
    pub fn new(config: Option<DeviceConfig>, num_cores: usize) -> Self {
        let config = config.unwrap_or_else(|| DeviceConfig {
            name: format!("Apple ANE ({} cores)", num_cores),
            architecture: "apple_ane_core".to_string(),
            num_compute_units: num_cores,
            l2_cache_size: 0,
            l2_cache_latency: 0,
            l2_cache_associativity: 0,
            l2_cache_line_size: 128,
            global_memory_size: 16 * 1024 * 1024,
            global_memory_bandwidth: 200.0,
            global_memory_latency: 100,
            memory_channels: 8,
            host_bandwidth: 200.0,
            host_latency: 0,
            unified_memory: true,
            max_concurrent_kernels: 1,
            work_distribution_policy: "scheduled".to_string(),
        });

        let core_config = ANECoreConfig::default();
        let cores: Vec<Box<dyn ComputeUnitTrait>> = (0..config.num_compute_units)
            .map(|_| {
                Box::new(NeuralEngineCore::new(core_config.clone()))
                    as Box<dyn ComputeUnitTrait>
            })
            .collect();

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

        let replayer = ANEScheduleReplayer::new(
            config.num_compute_units,
            10,  // dma_latency
            20,  // compute_latency
            5,   // activate_latency
        );

        Self {
            config,
            cores,
            global_memory,
            replayer,
            cycle: 0,
            kernels_launched: 0,
        }
    }

    /// Access to device memory.
    pub fn global_memory(&self) -> &SimpleGlobalMemory {
        &self.global_memory
    }

    /// True -- Apple ANE always uses unified memory.
    pub fn is_unified_memory(&self) -> bool {
        true
    }
}

impl AcceleratorDevice for AppleANE {
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
        self.replayer.submit_operation(&kernel);
        self.kernels_launched += 1;
    }

    fn step(&mut self) -> DeviceTrace {
        self.cycle += 1;

        // Replay the next step in the compiler-generated schedule
        let schedule_actions = self.replayer.step();

        // Step all cores
        let mut cu_traces = Vec::new();
        for core in &mut self.cores {
            let trace = core.step();
            cu_traces.push(trace);
        }

        let active_cores = self.cores.iter().filter(|c| !c.idle()).count();

        DeviceTrace {
            cycle: self.cycle,
            device_name: self.config.name.clone(),
            distributor_actions: schedule_actions,
            pending_blocks: self.replayer.pending_count(),
            active_blocks: active_cores,
            cu_traces,
            l2_hits: 0,
            l2_misses: 0,
            memory_transactions: 0,
            memory_bandwidth_used: 0.0,
            total_active_warps: 0,
            device_occupancy: if self.cores.is_empty() {
                0.0
            } else {
                active_cores as f64 / self.cores.len() as f64
            },
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
        self.replayer.idle()
    }

    fn reset(&mut self) {
        for core in &mut self.cores {
            core.reset();
        }
        self.global_memory.reset();
        self.replayer.reset();
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
            total_blocks_dispatched: self.replayer.total_dispatched(),
        }
    }
}
