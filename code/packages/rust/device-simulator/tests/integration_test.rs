//! Integration tests for device-simulator.
//!
//! These tests exercise all five device types end-to-end: NVIDIA GPU, AMD GPU,
//! Google TPU, Intel GPU, and Apple ANE. They cover the full device lifecycle:
//! memory allocation, host transfers, kernel launches, simulation, and stats.

use device_simulator::{
    AcceleratorDevice, DeviceTrace, KernelDescriptor,
    NvidiaGPU, AmdGPU, GoogleTPU, IntelGPU, AppleANE,
    SimpleGlobalMemory, GlobalMemoryStats, MemoryTransaction,
};
use gpu_core::opcodes::{halt, limm};

// =========================================================================
// Helpers -- create devices for testing
// =========================================================================

/// Create one of each GPU-style device (small configs for testing).
fn all_gpu_devices() -> Vec<(&'static str, Box<dyn AcceleratorDevice>)> {
    vec![
        ("NVIDIA", Box::new(NvidiaGPU::new(None, 2)) as Box<dyn AcceleratorDevice>),
        ("AMD", Box::new(AmdGPU::new(None, 2)) as Box<dyn AcceleratorDevice>),
        ("Intel", Box::new(IntelGPU::new(None, 2)) as Box<dyn AcceleratorDevice>),
    ]
}

/// Create one of each dataflow device (small configs for testing).
fn all_dataflow_devices() -> Vec<(&'static str, Box<dyn AcceleratorDevice>)> {
    vec![
        ("TPU", Box::new(GoogleTPU::new(None, 2)) as Box<dyn AcceleratorDevice>),
        ("ANE", Box::new(AppleANE::new(None, 2)) as Box<dyn AcceleratorDevice>),
    ]
}

/// Create one of every device type.
fn all_devices() -> Vec<(&'static str, Box<dyn AcceleratorDevice>)> {
    let mut devices = all_gpu_devices();
    devices.extend(all_dataflow_devices());
    devices
}

// =========================================================================
// Basic lifecycle tests
// =========================================================================

#[test]
fn test_all_devices_start_idle() {
    for (name, device) in all_devices() {
        assert!(device.idle(), "{} should start idle", name);
    }
}

#[test]
fn test_all_have_non_empty_names() {
    for (name, device) in all_devices() {
        assert!(!device.name().is_empty(), "{} should have a name", name);
    }
}

#[test]
fn test_all_have_config() {
    for (name, device) in all_devices() {
        let config = device.config();
        assert!(config.num_compute_units > 0, "{} should have compute units", name);
    }
}

#[test]
fn test_all_can_step_when_idle() {
    for (name, mut device) in all_devices() {
        let trace = device.step();
        assert!(trace.cycle > 0, "{} step should produce a trace", name);
    }
}

#[test]
fn test_all_reset_to_idle() {
    for (name, mut device) in all_devices() {
        device.step();
        device.step();
        device.reset();
        assert!(device.idle(), "{} should be idle after reset", name);
    }
}

// =========================================================================
// GPU-style kernel execution
// =========================================================================

#[test]
fn test_all_gpus_run_simple_kernel() {
    for (name, mut device) in all_gpu_devices() {
        let kernel = KernelDescriptor {
            name: "test_simple".to_string(),
            program: Some(vec![limm(0, 42.0), halt()]),
            grid_dim: (2, 1, 1),
            block_dim: (32, 1, 1),
            ..KernelDescriptor::default()
        };
        device.launch_kernel(kernel);
        let traces = device.run(2000);
        assert!(!traces.is_empty(), "{}: should produce traces", name);
        assert!(device.idle(), "{}: should be idle after completion", name);
    }
}

#[test]
fn test_nvidia_multi_block_kernel() {
    let mut gpu = NvidiaGPU::new(None, 4);
    let kernel = KernelDescriptor {
        name: "multi_block".to_string(),
        program: Some(vec![limm(0, 1.0), limm(1, 2.0), halt()]),
        grid_dim: (8, 1, 1),
        block_dim: (32, 1, 1),
        ..KernelDescriptor::default()
    };
    gpu.launch_kernel(kernel);
    let traces = gpu.run(5000);
    assert!(!traces.is_empty());
    assert!(gpu.idle());
    let stats = gpu.stats();
    assert_eq!(stats.total_kernels_launched, 1);
    assert_eq!(stats.total_blocks_dispatched, 8);
}

#[test]
fn test_amd_multi_block_kernel() {
    let mut gpu = AmdGPU::new(None, 4);
    let kernel = KernelDescriptor {
        name: "multi_block".to_string(),
        program: Some(vec![limm(0, 1.0), halt()]),
        grid_dim: (4, 1, 1),
        block_dim: (64, 1, 1),
        ..KernelDescriptor::default()
    };
    gpu.launch_kernel(kernel);
    let traces = gpu.run(5000);
    assert!(!traces.is_empty());
    assert!(gpu.idle());
}

#[test]
fn test_intel_multi_block_kernel() {
    let mut gpu = IntelGPU::new(None, 4);
    let kernel = KernelDescriptor {
        name: "multi_block".to_string(),
        program: Some(vec![limm(0, 1.0), halt()]),
        grid_dim: (4, 1, 1),
        block_dim: (32, 1, 1),
        ..KernelDescriptor::default()
    };
    gpu.launch_kernel(kernel);
    let traces = gpu.run(5000);
    assert!(!traces.is_empty());
    assert!(gpu.idle());
}

// =========================================================================
// Dataflow-style execution
// =========================================================================

#[test]
fn test_all_dataflow_run_matmul() {
    for (name, mut device) in all_dataflow_devices() {
        let kernel = KernelDescriptor {
            name: "matmul".to_string(),
            operation: "matmul".to_string(),
            input_data: Some(vec![vec![1.0, 2.0], vec![3.0, 4.0]]),
            weight_data: Some(vec![vec![5.0, 6.0], vec![7.0, 8.0]]),
            ..KernelDescriptor::default()
        };
        device.launch_kernel(kernel);
        let traces = device.run(1000);
        assert!(!traces.is_empty(), "{}: should produce traces", name);
        assert!(device.idle(), "{}: should be idle after matmul", name);
    }
}

#[test]
fn test_tpu_single_tile_operation() {
    let mut tpu = GoogleTPU::new(None, 4);
    let kernel = KernelDescriptor {
        name: "small_matmul".to_string(),
        operation: "matmul".to_string(),
        input_data: Some(vec![vec![1.0, 2.0]]),
        weight_data: Some(vec![vec![3.0], vec![4.0]]),
        ..KernelDescriptor::default()
    };
    tpu.launch_kernel(kernel);
    let traces = tpu.run(500);
    assert!(!traces.is_empty());
    assert!(tpu.idle());
    let stats = tpu.stats();
    assert_eq!(stats.total_kernels_launched, 1);
}

#[test]
fn test_ane_schedule_replay() {
    let mut ane = AppleANE::new(None, 4);
    let kernel = KernelDescriptor {
        name: "inference".to_string(),
        operation: "matmul".to_string(),
        input_data: Some(vec![vec![1.0, 2.0], vec![3.0, 4.0]]),
        weight_data: Some(vec![vec![5.0, 6.0], vec![7.0, 8.0]]),
        ..KernelDescriptor::default()
    };
    ane.launch_kernel(kernel);
    let traces = ane.run(500);
    assert!(!traces.is_empty());
    assert!(ane.idle());
    let stats = ane.stats();
    assert_eq!(stats.total_kernels_launched, 1);
    assert!(stats.total_blocks_dispatched > 0);
}

// =========================================================================
// Memory management
// =========================================================================

#[test]
fn test_all_can_malloc_and_free() {
    for (name, mut device) in all_devices() {
        let addr = device.malloc(256);
        assert!(addr < u64::MAX, "{}: malloc should return valid address", name);
        device.free(addr);
    }
}

#[test]
fn test_all_can_transfer_data() {
    for (name, mut device) in all_devices() {
        let addr = device.malloc(64);
        let _cycles_h2d = device.memcpy_host_to_device(addr, &[0x42u8; 64]);
        let (data, _cycles_d2h) = device.memcpy_device_to_host(addr, 64);
        assert_eq!(data, vec![0x42u8; 64], "{}: data should round-trip", name);
    }
}

#[test]
fn test_unified_vs_discrete_transfer_cost() {
    let mut ane = AppleANE::new(None, 2);
    let mut nvidia = NvidiaGPU::new(None, 2);

    let ane_addr = ane.malloc(256);
    let nvidia_addr = nvidia.malloc(256);

    let ane_cycles = ane.memcpy_host_to_device(ane_addr, &[0u8; 256]);
    let nvidia_cycles = nvidia.memcpy_host_to_device(nvidia_addr, &[0u8; 256]);

    assert_eq!(ane_cycles, 0, "ANE unified memory should be zero-cost");
    assert!(nvidia_cycles > 0, "NVIDIA discrete should have transfer cost");
}

#[test]
fn test_apple_ane_unified_memory_flag() {
    let ane = AppleANE::new(None, 2);
    assert!(ane.is_unified_memory());
}

// =========================================================================
// Global memory direct tests
// =========================================================================

#[test]
fn test_global_memory_write_and_read() {
    let mut mem = SimpleGlobalMemory::with_capacity(1024);
    mem.write(0, &[0x41, 0x42, 0x43, 0x44]).unwrap();
    let data = mem.read(0, 4).unwrap();
    assert_eq!(data, vec![0x41, 0x42, 0x43, 0x44]);
}

#[test]
fn test_global_memory_uninitialized_reads_zero() {
    let mut mem = SimpleGlobalMemory::with_capacity(1024);
    let data = mem.read(0, 8).unwrap();
    assert_eq!(data, vec![0; 8]);
}

#[test]
fn test_global_memory_allocate_aligned() {
    let mut mem = SimpleGlobalMemory::with_capacity(1024 * 1024);
    let addr = mem.allocate(256, 256).unwrap();
    assert_eq!(addr % 256, 0);
}

#[test]
fn test_global_memory_sequential_allocations() {
    let mut mem = SimpleGlobalMemory::with_capacity(1024 * 1024);
    let a1 = mem.allocate(256, 256).unwrap();
    let a2 = mem.allocate(256, 256).unwrap();
    assert!(a2 >= a1 + 256);
}

#[test]
fn test_global_memory_out_of_memory() {
    let mut mem = SimpleGlobalMemory::with_capacity(512);
    mem.allocate(256, 256).unwrap();
    assert!(mem.allocate(512, 256).is_err());
}

#[test]
fn test_global_memory_coalescing_perfect() {
    let mut mem = SimpleGlobalMemory::with_capacity(1024);
    let addrs: Vec<u64> = (0..32).map(|i| i * 4).collect();
    let transactions = mem.coalesce(&addrs);
    assert_eq!(transactions.len(), 1);
    assert_eq!(transactions[0].size, 128);
}

#[test]
fn test_global_memory_coalescing_scattered() {
    let mut mem = SimpleGlobalMemory::with_capacity(1024 * 1024);
    let addrs: Vec<u64> = (0..4).map(|i| i * 512).collect();
    let transactions = mem.coalesce(&addrs);
    assert_eq!(transactions.len(), 4);
}

#[test]
fn test_global_memory_coalescing_thread_mask() {
    let mut mem = SimpleGlobalMemory::with_capacity(1024);
    let addrs = vec![0u64, 4, 256]; // threads 0,1 in region 0; thread 2 in region 2
    let transactions = mem.coalesce(&addrs);
    assert_eq!(transactions.len(), 2);
    let first = transactions.iter().find(|t| t.address == 0).unwrap();
    assert_eq!(first.thread_mask & 0b11, 0b11); // threads 0 and 1
}

#[test]
fn test_global_memory_coalescing_stats() {
    let mut mem = SimpleGlobalMemory::with_capacity(1024);
    let addrs: Vec<u64> = (0..32).map(|i| i * 4).collect();
    mem.coalesce(&addrs);
    let stats = mem.stats();
    assert_eq!(stats.total_requests, 32);
    assert_eq!(stats.total_transactions, 1);
    assert_eq!(stats.coalescing_efficiency, 32.0);
}

#[test]
fn test_global_memory_reset() {
    let mut mem = SimpleGlobalMemory::with_capacity(1024);
    mem.write(0, &[0xFF; 4]).unwrap();
    mem.allocate(512, 256).unwrap();
    mem.reset();
    let data = mem.read(0, 4).unwrap();
    assert_eq!(data, vec![0; 4]);
    let addr = mem.allocate(512, 256).unwrap();
    assert_eq!(addr, 0);
}

#[test]
fn test_global_memory_properties() {
    let mem = SimpleGlobalMemory::with_capacity(4096);
    assert_eq!(mem.capacity(), 4096);
    assert_eq!(mem.bandwidth(), 1000.0);
}

#[test]
fn test_global_memory_host_transfer_stats() {
    let mut mem = SimpleGlobalMemory::with_capacity(1024);
    mem.copy_from_host(0, &[0; 128]);
    let stats = mem.stats();
    assert_eq!(stats.host_to_device_bytes, 128);
    assert!(stats.host_transfer_cycles > 0);
}

#[test]
fn test_global_memory_device_to_host_stats() {
    let mut mem = SimpleGlobalMemory::with_capacity(1024);
    mem.write(0, &[0; 64]).unwrap();
    mem.copy_to_host(0, 64);
    let stats = mem.stats();
    assert_eq!(stats.device_to_host_bytes, 64);
}

// =========================================================================
// Stats
// =========================================================================

#[test]
fn test_all_track_kernels() {
    for (name, mut device) in all_devices() {
        let kernel = if name == "NVIDIA" || name == "AMD" || name == "Intel" {
            KernelDescriptor {
                name: "test".to_string(),
                program: Some(vec![limm(0, 1.0), halt()]),
                grid_dim: (1, 1, 1),
                block_dim: (32, 1, 1),
                ..KernelDescriptor::default()
            }
        } else {
            KernelDescriptor {
                name: "test".to_string(),
                operation: "matmul".to_string(),
                input_data: Some(vec![vec![1.0]]),
                weight_data: Some(vec![vec![1.0]]),
                ..KernelDescriptor::default()
            }
        };
        device.launch_kernel(kernel);
        device.run(1000);
        let stats = device.stats();
        assert_eq!(
            stats.total_kernels_launched, 1,
            "{}: should track kernel launches",
            name
        );
    }
}

#[test]
fn test_stats_total_cycles() {
    let mut gpu = NvidiaGPU::new(None, 2);
    gpu.step();
    gpu.step();
    gpu.step();
    let stats = gpu.stats();
    assert_eq!(stats.total_cycles, 3);
}

#[test]
fn test_stats_after_reset() {
    let mut gpu = NvidiaGPU::new(None, 2);
    let kernel = KernelDescriptor {
        name: "test".to_string(),
        program: Some(vec![limm(0, 1.0), halt()]),
        grid_dim: (1, 1, 1),
        block_dim: (32, 1, 1),
        ..KernelDescriptor::default()
    };
    gpu.launch_kernel(kernel);
    gpu.run(1000);
    gpu.reset();
    let stats = gpu.stats();
    assert_eq!(stats.total_cycles, 0);
    assert_eq!(stats.total_kernels_launched, 0);
}

// =========================================================================
// Trace format
// =========================================================================

#[test]
fn test_all_produce_readable_traces() {
    for (name, mut device) in all_devices() {
        let trace = device.step();
        let formatted = trace.format();
        assert!(!formatted.is_empty(), "{}: format() should be non-empty", name);
        assert!(
            formatted.contains("[Cycle"),
            "{}: format() should contain cycle info",
            name
        );
    }
}

#[test]
fn test_trace_contains_device_name() {
    let mut gpu = NvidiaGPU::new(None, 2);
    let trace = gpu.step();
    let formatted = trace.format();
    assert!(formatted.contains("NVIDIA"));
}

#[test]
fn test_trace_shows_occupancy() {
    let mut gpu = NvidiaGPU::new(None, 2);
    let trace = gpu.step();
    let formatted = trace.format();
    assert!(formatted.contains("occupancy"));
}

// =========================================================================
// Cross-device tests
// =========================================================================

#[test]
fn test_all_devices_have_different_architectures() {
    let devices = all_devices();
    let mut architectures: Vec<String> = devices
        .iter()
        .map(|(_, d)| d.config().architecture.clone())
        .collect();
    architectures.sort();
    architectures.dedup();
    assert_eq!(architectures.len(), 5, "Should have 5 unique architectures");
}

#[test]
fn test_gpu_devices_use_block_distribution() {
    for (name, mut device) in all_gpu_devices() {
        let kernel = KernelDescriptor {
            name: "test".to_string(),
            program: Some(vec![limm(0, 1.0), halt()]),
            grid_dim: (4, 1, 1),
            block_dim: (32, 1, 1),
            ..KernelDescriptor::default()
        };
        device.launch_kernel(kernel);
        // First step should distribute some blocks
        let trace = device.step();
        // Either blocks were distributed or are pending
        assert!(
            trace.pending_blocks > 0 || !trace.distributor_actions.is_empty(),
            "{}: should show distribution activity",
            name
        );
    }
}

#[test]
fn test_multiple_kernel_launches() {
    let mut gpu = NvidiaGPU::new(None, 4);

    // Launch first kernel
    let k1 = KernelDescriptor {
        name: "kernel_1".to_string(),
        program: Some(vec![limm(0, 1.0), halt()]),
        grid_dim: (2, 1, 1),
        block_dim: (32, 1, 1),
        ..KernelDescriptor::default()
    };
    gpu.launch_kernel(k1);
    gpu.run(2000);
    assert!(gpu.idle());

    // Launch second kernel (no reset)
    let k2 = KernelDescriptor {
        name: "kernel_2".to_string(),
        program: Some(vec![limm(0, 2.0), halt()]),
        grid_dim: (2, 1, 1),
        block_dim: (32, 1, 1),
        ..KernelDescriptor::default()
    };
    gpu.launch_kernel(k2);
    gpu.run(2000);
    assert!(gpu.idle());

    let stats = gpu.stats();
    assert_eq!(stats.total_kernels_launched, 2);
}

// =========================================================================
// Device-specific feature tests
// =========================================================================

#[test]
fn test_amd_has_shader_engines() {
    let gpu = AmdGPU::new(None, 4);
    let engines = gpu.shader_engines();
    assert!(!engines.is_empty(), "AMD GPU should have shader engines");
}

#[test]
fn test_intel_has_xe_slices() {
    let gpu = IntelGPU::new(None, 4);
    let slices = gpu.xe_slices();
    assert!(!slices.is_empty(), "Intel GPU should have Xe-Slices");
}

#[test]
fn test_tpu_processes_larger_matrix() {
    let mut tpu = GoogleTPU::new(None, 4);
    // 4x4 matrix -- should tile into multiple tiles with mxu_size=4
    let input = vec![
        vec![1.0, 2.0, 3.0, 4.0],
        vec![5.0, 6.0, 7.0, 8.0],
        vec![9.0, 10.0, 11.0, 12.0],
        vec![13.0, 14.0, 15.0, 16.0],
    ];
    let weights = vec![
        vec![1.0, 0.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0, 0.0],
        vec![0.0, 0.0, 1.0, 0.0],
        vec![0.0, 0.0, 0.0, 1.0],
    ];
    let kernel = KernelDescriptor {
        name: "large_matmul".to_string(),
        operation: "matmul".to_string(),
        input_data: Some(input),
        weight_data: Some(weights),
        ..KernelDescriptor::default()
    };
    tpu.launch_kernel(kernel);
    let traces = tpu.run(1000);
    assert!(!traces.is_empty());
    assert!(tpu.idle());
}

#[test]
fn test_ane_multiple_operations() {
    let mut ane = AppleANE::new(None, 4);

    // First operation
    let k1 = KernelDescriptor {
        name: "op1".to_string(),
        operation: "matmul".to_string(),
        input_data: Some(vec![vec![1.0]]),
        weight_data: Some(vec![vec![2.0]]),
        ..KernelDescriptor::default()
    };
    ane.launch_kernel(k1);
    ane.run(500);
    assert!(ane.idle());

    // Second operation (without reset)
    let k2 = KernelDescriptor {
        name: "op2".to_string(),
        operation: "matmul".to_string(),
        input_data: Some(vec![vec![3.0]]),
        weight_data: Some(vec![vec![4.0]]),
        ..KernelDescriptor::default()
    };
    ane.launch_kernel(k2);
    ane.run(500);
    assert!(ane.idle());

    let stats = ane.stats();
    assert_eq!(stats.total_kernels_launched, 2);
}

// =========================================================================
// Memory + Compute integration
// =========================================================================

#[test]
fn test_nvidia_memory_and_compute() {
    let mut gpu = NvidiaGPU::new(None, 2);

    // Allocate and write data
    let addr = gpu.malloc(1024);
    let cycles = gpu.memcpy_host_to_device(addr, &[0xAB; 1024]);
    assert!(cycles > 0);

    // Run a kernel
    let kernel = KernelDescriptor {
        name: "test".to_string(),
        program: Some(vec![limm(0, 1.0), halt()]),
        grid_dim: (1, 1, 1),
        block_dim: (32, 1, 1),
        ..KernelDescriptor::default()
    };
    gpu.launch_kernel(kernel);
    gpu.run(2000);

    // Read back data
    let (data, read_cycles) = gpu.memcpy_device_to_host(addr, 4);
    assert_eq!(data, vec![0xAB; 4]);
    assert!(read_cycles > 0);

    // Check stats
    let stats = gpu.stats();
    let mem_stats = stats.global_memory_stats;
    assert!(mem_stats.host_to_device_bytes > 0);
    assert!(mem_stats.device_to_host_bytes > 0);
}

#[test]
fn test_ane_zero_cost_memory_and_compute() {
    let mut ane = AppleANE::new(None, 2);

    let addr = ane.malloc(512);
    let h2d_cycles = ane.memcpy_host_to_device(addr, &[0xCD; 512]);
    assert_eq!(h2d_cycles, 0, "ANE host-to-device should be zero-cost");

    let (data, d2h_cycles) = ane.memcpy_device_to_host(addr, 4);
    assert_eq!(data, vec![0xCD; 4]);
    assert_eq!(d2h_cycles, 0, "ANE device-to-host should be zero-cost");
}

// =========================================================================
// Partition conflict tests
// =========================================================================

#[test]
fn test_no_partition_conflict() {
    let mut mem = SimpleGlobalMemory::with_capacity(4096);
    let addrs: Vec<u64> = (0..4).map(|i| i * 128).collect();
    mem.coalesce(&addrs);
    let stats = mem.stats();
    assert_eq!(stats.partition_conflicts, 0);
}

#[test]
fn test_partition_conflict_detected() {
    use device_simulator::global_memory::SimpleGlobalMemory;
    let mut mem = SimpleGlobalMemory::new(4096, 1000.0, 400, 4, 128, 64.0, 100, false);
    // Addresses 0 and 512 both map to channel 0 (with 4 channels)
    let addrs = vec![0u64, 512];
    mem.coalesce(&addrs);
    let stats = mem.stats();
    assert!(stats.partition_conflicts >= 1);
}
