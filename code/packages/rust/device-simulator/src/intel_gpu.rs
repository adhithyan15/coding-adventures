//! Intel GPU -- device simulator with Xe-Slices.
//!
//! # Intel GPU Architecture (Xe-HPG / Arc)
//!
//! Intel organizes Xe-Cores into **Xe-Slices**, with each slice sharing
//! a large L1 cache. Similar to AMD's Shader Engines but at a different
//! granularity.
//!
//! ```text
//! +---------------------------------------------------+
//! |                Intel GPU                           |
//! |  +---------------------------------------------+  |
//! |  |     Command Streamer (distributor)           |  |
//! |  +---------------------+-----------------------+  |
//! |                        |                          |
//! |  +---------------------+-----+                    |
//! |  |      Xe-Slice 0           |                    |
//! |  |  +--------+ +--------+   |                    |
//! |  |  |XeCore 0| |XeCore 1|   |                    |
//! |  |  | 8 EUs  | | 8 EUs  |   |                    |
//! |  |  +--------+ +--------+   |                    |
//! |  |  L1 Cache (192 KB)       |                    |
//! |  +---------------------------+                    |
//! |  ... more Xe-Slices ...                           |
//! |                                                   |
//! |  +---------------------------------------------+  |
//! |  |         L2 Cache (16 MB shared)              |  |
//! |  +---------------------------------------------+  |
//! |  +---------------------------------------------+  |
//! |  |        GDDR6 (16 GB, 512 GB/s)               |  |
//! |  +---------------------------------------------+  |
//! +---------------------------------------------------+
//! ```

use compute_unit::{ComputeUnit as ComputeUnitTrait, XeCore, XeCoreConfig};

use crate::global_memory::SimpleGlobalMemory;
use crate::protocols::{
    AcceleratorDevice, DeviceConfig, DeviceStats, DeviceTrace, KernelDescriptor,
};
use crate::work_distributor::GPUWorkDistributor;

/// A group of Xe-Cores sharing an L1 cache.
#[derive(Debug)]
pub struct XeSlice {
    /// Slice identifier.
    pub slice_id: usize,
    /// Indices of Xe-Cores belonging to this slice.
    pub core_indices: Vec<usize>,
}

/// Intel GPU device simulator.
///
/// Features Xe-Slice grouping, shared L1 per slice, L2 cache, and
/// the Command Streamer for work distribution.
pub struct IntelGPU {
    /// Device configuration.
    config: DeviceConfig,
    /// All Xe-Cores, stored as trait objects.
    all_cores: Vec<Box<dyn ComputeUnitTrait>>,
    /// Xe-Slices (groups of core indices).
    xe_slices: Vec<XeSlice>,
    /// Global memory (GDDR6).
    global_memory: SimpleGlobalMemory,
    /// Work distributor (Command Streamer).
    distributor: GPUWorkDistributor,
    /// Current simulation cycle.
    cycle: u64,
    /// Total kernel launches.
    kernels_launched: u64,
}

impl IntelGPU {
    /// Create a new Intel GPU simulator.
    ///
    /// # Arguments
    ///
    /// * `config` - Full DeviceConfig (uses a small default if None).
    /// * `num_cores` - Total Xe-Cores (used only if config is None).
    pub fn new(config: Option<DeviceConfig>, num_cores: usize) -> Self {
        let config = config.unwrap_or_else(|| DeviceConfig {
            name: format!("Intel GPU ({} Xe-Cores)", num_cores),
            architecture: "intel_xe_core".to_string(),
            num_compute_units: num_cores,
            l2_cache_size: 4096,
            l2_cache_latency: 180,
            l2_cache_associativity: 4,
            l2_cache_line_size: 64,
            global_memory_size: 16 * 1024 * 1024,
            global_memory_bandwidth: 512.0,
            global_memory_latency: 350,
            memory_channels: 4,
            host_bandwidth: 32.0,
            host_latency: 100,
            unified_memory: false,
            max_concurrent_kernels: 16,
            work_distribution_policy: "round_robin".to_string(),
        });

        let core_config = XeCoreConfig::default();
        let all_cores: Vec<Box<dyn ComputeUnitTrait>> = (0..config.num_compute_units)
            .map(|_| {
                Box::new(XeCore::new(core_config.clone()))
                    as Box<dyn ComputeUnitTrait>
            })
            .collect();

        // Group into Xe-Slices (2 cores per slice by default)
        let cores_per_slice = (config.num_compute_units / 2).max(1);
        let mut xe_slices = Vec::new();
        let mut i = 0;
        while i < config.num_compute_units {
            let end = (i + cores_per_slice).min(config.num_compute_units);
            xe_slices.push(XeSlice {
                slice_id: xe_slices.len(),
                core_indices: (i..end).collect(),
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
            all_cores,
            xe_slices,
            global_memory,
            distributor,
            cycle: 0,
            kernels_launched: 0,
        }
    }

    /// Access to Xe-Slices.
    pub fn xe_slices(&self) -> &[XeSlice] {
        &self.xe_slices
    }

    /// Access to device memory.
    pub fn global_memory(&self) -> &SimpleGlobalMemory {
        &self.global_memory
    }
}

impl AcceleratorDevice for IntelGPU {
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

        let dist_actions = self.distributor.step(&mut self.all_cores);

        let mut cu_traces = Vec::new();
        let mut total_active_warps = 0;
        let mut total_max_warps = 0;

        for core in &mut self.all_cores {
            let trace = core.step();
            total_active_warps += trace.active_warps;
            total_max_warps += trace.total_warps;
            cu_traces.push(trace);
        }

        let device_occupancy = if total_max_warps > 0 {
            total_active_warps as f64 / total_max_warps as f64
        } else {
            0.0
        };

        let active_blocks = self.all_cores.iter().filter(|c| !c.idle()).count();

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
        self.distributor.pending_count() == 0 && self.all_cores.iter().all(|c| c.idle())
    }

    fn reset(&mut self) {
        for core in &mut self.all_cores {
            core.reset();
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
