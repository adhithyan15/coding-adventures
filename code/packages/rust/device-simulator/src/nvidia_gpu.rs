//! NVIDIA GPU -- device simulator with GigaThread Engine.
//!
//! # NVIDIA GPU Architecture
//!
//! The NVIDIA GPU is the most widely-used accelerator for machine learning.
//! Its architecture is built around Streaming Multiprocessors (SMs), each
//! of which can independently schedule and execute thousands of threads.
//!
//! ```text
//! +---------------------------------------------------+
//! |                  NVIDIA GPU                        |
//! |  +---------------------------------------------+  |
//! |  |        GigaThread Engine (distributor)       |  |
//! |  +---------------------+-----------------------+  |
//! |                        |                          |
//! |  +------+ +------+ +------+ ... +------+          |
//! |  |SM 0  | |SM 1  | |SM 2  |     |SM N  |         |
//! |  +---+--+ +---+--+ +---+--+     +---+--+         |
//! |      +--------+--------+-----------+              |
//! |                |                                   |
//! |  +-------------+-------------------------------+  |
//! |  |            L2 Cache (shared)                |  |
//! |  +-------------+-------------------------------+  |
//! |                |                                   |
//! |  +-------------+-------------------------------+  |
//! |  |          HBM3 (80 GB, 3.35 TB/s)            |  |
//! |  +---------------------------------------------+  |
//! +---------------------------------------------------+
//! ```
//!
//! # GigaThread Engine
//!
//! The GigaThread Engine is the top-level work distributor. When a kernel
//! is launched, it creates thread blocks from grid dimensions, assigns
//! blocks to SMs with available resources, and refills SMs as they complete
//! blocks. This creates **waves** of execution.

use compute_unit::{ComputeUnit as ComputeUnitTrait, SMConfig, StreamingMultiprocessor};

use crate::global_memory::SimpleGlobalMemory;
use crate::protocols::{
    AcceleratorDevice, DeviceConfig, DeviceStats, DeviceTrace, KernelDescriptor,
};
use crate::work_distributor::GPUWorkDistributor;

/// NVIDIA GPU device simulator.
///
/// Creates multiple SMs, global memory (HBM), and a GigaThread Engine
/// to distribute thread blocks across SMs.
///
/// # Usage
///
/// ```
/// use device_simulator::nvidia_gpu::NvidiaGPU;
/// use device_simulator::protocols::{AcceleratorDevice, KernelDescriptor};
/// use gpu_core::opcodes::{limm, halt};
///
/// let mut gpu = NvidiaGPU::new(None, 4);
///
/// // Allocate and copy data
/// let addr = gpu.malloc(1024);
/// gpu.memcpy_host_to_device(addr, &[0u8; 1024]);
///
/// // Launch kernel
/// let mut kernel = KernelDescriptor::default();
/// kernel.name = "saxpy".to_string();
/// kernel.program = Some(vec![limm(0, 2.0), halt()]);
/// kernel.grid_dim = (4, 1, 1);
/// kernel.block_dim = (32, 1, 1);
/// gpu.launch_kernel(kernel);
///
/// // Run to completion
/// let traces = gpu.run(1000);
/// ```
pub struct NvidiaGPU {
    /// Device configuration.
    config: DeviceConfig,
    /// Streaming Multiprocessors, stored as trait objects.
    sms: Vec<Box<dyn ComputeUnitTrait>>,
    /// Global memory (HBM).
    global_memory: SimpleGlobalMemory,
    /// Work distributor (GigaThread Engine).
    distributor: GPUWorkDistributor,
    /// Current simulation cycle.
    cycle: u64,
    /// Total kernel launches.
    kernels_launched: u64,
}

impl NvidiaGPU {
    /// Create a new NVIDIA GPU simulator.
    ///
    /// # Arguments
    ///
    /// * `config` - Full DeviceConfig (uses a small default if None).
    /// * `num_sms` - Number of SMs (used only if config is None).
    pub fn new(config: Option<DeviceConfig>, num_sms: usize) -> Self {
        let config = config.unwrap_or_else(|| DeviceConfig {
            name: format!("NVIDIA GPU ({} SMs)", num_sms),
            architecture: "nvidia_sm".to_string(),
            num_compute_units: num_sms,
            l2_cache_size: 4096,
            l2_cache_latency: 200,
            l2_cache_associativity: 4,
            l2_cache_line_size: 64,
            global_memory_size: 16 * 1024 * 1024,
            global_memory_bandwidth: 1000.0,
            global_memory_latency: 400,
            memory_channels: 4,
            host_bandwidth: 64.0,
            host_latency: 100,
            unified_memory: false,
            max_concurrent_kernels: 128,
            work_distribution_policy: "round_robin".to_string(),
        });

        let mut sm_config = SMConfig::default();
        sm_config.max_warps = 8;

        let sms: Vec<Box<dyn ComputeUnitTrait>> = (0..config.num_compute_units)
            .map(|_| {
                Box::new(StreamingMultiprocessor::new(sm_config.clone()))
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

        let distributor = GPUWorkDistributor::new(
            config.num_compute_units,
            &config.work_distribution_policy,
        );

        Self {
            config,
            sms,
            global_memory,
            distributor,
            cycle: 0,
            kernels_launched: 0,
        }
    }

    /// Access to device memory.
    pub fn global_memory(&self) -> &SimpleGlobalMemory {
        &self.global_memory
    }

    /// Mutable access to device memory.
    pub fn global_memory_mut(&mut self) -> &mut SimpleGlobalMemory {
        &mut self.global_memory
    }
}

impl AcceleratorDevice for NvidiaGPU {
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

        // 1. Distribute pending blocks to SMs
        let dist_actions = self.distributor.step(&mut self.sms);

        // 2. Step all SMs
        let mut cu_traces = Vec::new();
        let mut total_active_warps = 0;
        let mut total_max_warps = 0;

        for sm in &mut self.sms {
            let trace = sm.step();
            total_active_warps += trace.active_warps;
            total_max_warps += trace.total_warps;
            cu_traces.push(trace);
        }

        let device_occupancy = if total_max_warps > 0 {
            total_active_warps as f64 / total_max_warps as f64
        } else {
            0.0
        };

        let active_blocks = self.sms.iter().filter(|sm| !sm.idle()).count();

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
        self.distributor.pending_count() == 0 && self.sms.iter().all(|sm| sm.idle())
    }

    fn reset(&mut self) {
        for sm in &mut self.sms {
            sm.reset();
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
