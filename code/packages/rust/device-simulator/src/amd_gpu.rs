//! AMD GPU -- device simulator with Shader Engines and Infinity Cache.
//!
//! # AMD GPU Architecture
//!
//! AMD organizes compute units (CUs) into **Shader Engines** (SEs). This is
//! a mid-level hierarchy that NVIDIA doesn't have -- CUs within the same SE
//! share a geometry processor and rasterizer (for graphics).
//!
//! ```text
//! +---------------------------------------------------+
//! |                    AMD GPU                         |
//! |  +---------------------------------------------+  |
//! |  |       Command Processor (distributor)        |  |
//! |  +---------------------+-----------------------+  |
//! |                        |                          |
//! |  +---------------------+-----+                    |
//! |  |    Shader Engine 0        |                    |
//! |  |  +----+ +----+ ... +----+ |                    |
//! |  |  |CU 0| |CU 1|     |CU N||                    |
//! |  |  +----+ +----+     +----+ |                    |
//! |  +---------------------------+                    |
//! |  ... more Shader Engines ...                      |
//! |                                                   |
//! |  +---------------------------------------------+  |
//! |  |     Infinity Cache (96 MB)                   |  |
//! |  +---------------------------------------------+  |
//! |  +---------------------------------------------+  |
//! |  |         GDDR6 (24 GB, 960 GB/s)              |  |
//! |  +---------------------------------------------+  |
//! +---------------------------------------------------+
//! ```

use compute_unit::{ComputeUnit as ComputeUnitTrait, AMDComputeUnit, AMDCUConfig};

use crate::global_memory::SimpleGlobalMemory;
use crate::protocols::{
    AcceleratorDevice, DeviceConfig, DeviceStats, DeviceTrace, KernelDescriptor,
};
use crate::work_distributor::GPUWorkDistributor;

/// A group of CUs that share resources within a Shader Engine.
#[derive(Debug)]
pub struct ShaderEngine {
    /// Engine identifier.
    pub engine_id: usize,
    /// Indices of CUs belonging to this engine.
    pub cu_indices: Vec<usize>,
}

/// AMD GPU device simulator.
///
/// Features Shader Engine grouping, and multi-queue dispatch
/// via the Command Processor.
pub struct AmdGPU {
    /// Device configuration.
    config: DeviceConfig,
    /// All compute units (AMD CUs), stored as trait objects.
    all_cus: Vec<Box<dyn ComputeUnitTrait>>,
    /// Shader Engines (groups of CU indices).
    shader_engines: Vec<ShaderEngine>,
    /// Global memory (GDDR6).
    global_memory: SimpleGlobalMemory,
    /// Work distributor (Command Processor).
    distributor: GPUWorkDistributor,
    /// Current simulation cycle.
    cycle: u64,
    /// Total kernel launches.
    kernels_launched: u64,
}

impl AmdGPU {
    /// Create a new AMD GPU simulator.
    ///
    /// # Arguments
    ///
    /// * `config` - Full DeviceConfig (uses a small default if None).
    /// * `num_cus` - Number of CUs (used only if config is None).
    pub fn new(config: Option<DeviceConfig>, num_cus: usize) -> Self {
        let config = config.unwrap_or_else(|| DeviceConfig {
            name: format!("AMD GPU ({} CUs)", num_cus),
            architecture: "amd_cu".to_string(),
            num_compute_units: num_cus,
            l2_cache_size: 4096,
            l2_cache_latency: 150,
            l2_cache_associativity: 4,
            l2_cache_line_size: 64,
            global_memory_size: 16 * 1024 * 1024,
            global_memory_bandwidth: 960.0,
            global_memory_latency: 350,
            memory_channels: 4,
            host_bandwidth: 32.0,
            host_latency: 100,
            unified_memory: false,
            max_concurrent_kernels: 8,
            work_distribution_policy: "round_robin".to_string(),
        });

        let cu_config = AMDCUConfig::default();
        let all_cus: Vec<Box<dyn ComputeUnitTrait>> = (0..config.num_compute_units)
            .map(|_| {
                Box::new(AMDComputeUnit::new(cu_config.clone()))
                    as Box<dyn ComputeUnitTrait>
            })
            .collect();

        // Group into Shader Engines (2 CUs per SE by default)
        let se_size = (config.num_compute_units / 2).max(1);
        let mut shader_engines = Vec::new();
        let mut i = 0;
        while i < config.num_compute_units {
            let end = (i + se_size).min(config.num_compute_units);
            shader_engines.push(ShaderEngine {
                engine_id: shader_engines.len(),
                cu_indices: (i..end).collect(),
            });
            i = end;
        }

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

        let distributor = GPUWorkDistributor::new(
            config.num_compute_units,
            &config.work_distribution_policy,
        );

        Self {
            config,
            all_cus,
            shader_engines,
            global_memory,
            distributor,
            cycle: 0,
            kernels_launched: 0,
        }
    }

    /// Access to Shader Engines.
    pub fn shader_engines(&self) -> &[ShaderEngine] {
        &self.shader_engines
    }

    /// Access to device memory.
    pub fn global_memory(&self) -> &SimpleGlobalMemory {
        &self.global_memory
    }
}

impl AcceleratorDevice for AmdGPU {
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
        self.distributor.submit_kernel(&kernel);
        self.kernels_launched += 1;
    }

    fn step(&mut self) -> DeviceTrace {
        self.cycle += 1;

        let dist_actions = self.distributor.step(&mut self.all_cus);

        let mut cu_traces = Vec::new();
        let mut total_active_warps = 0;
        let mut total_max_warps = 0;

        for cu in &mut self.all_cus {
            let trace = cu.step();
            total_active_warps += trace.active_warps;
            total_max_warps += trace.total_warps;
            cu_traces.push(trace);
        }

        let device_occupancy = if total_max_warps > 0 {
            total_active_warps as f64 / total_max_warps as f64
        } else {
            0.0
        };

        let active_blocks = self.all_cus.iter().filter(|cu| !cu.idle()).count();

        DeviceTrace {
            cycle: self.cycle,
            device_name: self.config.name.clone(),
            distributor_actions: dist_actions,
            pending_blocks: self.distributor.pending_count(),
            active_blocks,
            cu_traces,
            l2_hits: 0,
            l2_misses: 0,
            memory_transactions: 0,
            memory_bandwidth_used: 0.0,
            total_active_warps,
            device_occupancy,
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
        self.distributor.pending_count() == 0 && self.all_cus.iter().all(|cu| cu.idle())
    }

    fn reset(&mut self) {
        for cu in &mut self.all_cus {
            cu.reset();
        }
        self.global_memory.reset();
        self.distributor.reset();
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
            total_blocks_dispatched: self.distributor.total_dispatched(),
        }
    }
}
