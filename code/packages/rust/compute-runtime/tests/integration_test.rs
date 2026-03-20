//! Integration tests for the compute runtime.
//!
//! These tests exercise the full stack: RuntimeInstance -> PhysicalDevice ->
//! LogicalDevice -> CommandBuffer -> CommandQueue -> device execution.

use compute_runtime::command_buffer::CommandBuffer;
use compute_runtime::instance::RuntimeInstance;
use compute_runtime::protocols::*;
use compute_runtime::sync::{Event, Fence, Semaphore};
use compute_runtime::validation::ValidationLayer;

use gpu_core::opcodes::{halt, limm};

// =========================================================================
// Device discovery
// =========================================================================

#[test]
fn test_discover_all_default_devices() {
    let instance = RuntimeInstance::new(None);
    let devices = instance.enumerate_physical_devices();
    assert_eq!(devices.len(), 5, "Should have 5 default devices");

    let vendors: Vec<&str> = devices.iter().map(|d| d.vendor()).collect();
    assert!(vendors.contains(&"nvidia"));
    assert!(vendors.contains(&"amd"));
    assert!(vendors.contains(&"google"));
    assert!(vendors.contains(&"intel"));
    assert!(vendors.contains(&"apple"));
}

#[test]
fn test_device_types_correct() {
    let instance = RuntimeInstance::new(None);
    let devices = instance.enumerate_physical_devices();

    for d in &devices {
        match d.vendor() {
            "nvidia" | "amd" | "intel" => assert_eq!(d.device_type(), DeviceType::GPU),
            "google" => assert_eq!(d.device_type(), DeviceType::TPU),
            "apple" => assert_eq!(d.device_type(), DeviceType::NPU),
            _ => panic!("Unknown vendor: {}", d.vendor()),
        }
    }
}

#[test]
fn test_unified_memory_detection() {
    let instance = RuntimeInstance::new(None);
    let devices = instance.enumerate_physical_devices();

    let apple = devices.iter().find(|d| d.vendor() == "apple").unwrap();
    assert!(apple.supports_feature("unified_memory"));
    assert!(apple.memory_properties().is_unified);

    let nvidia = devices.iter().find(|d| d.vendor() == "nvidia").unwrap();
    assert!(!nvidia.supports_feature("unified_memory"));
    assert!(!nvidia.memory_properties().is_unified);
}

#[test]
fn test_transfer_queue_detection() {
    let instance = RuntimeInstance::new(None);
    let devices = instance.enumerate_physical_devices();

    // Discrete GPUs have transfer queues
    let nvidia = devices.iter().find(|d| d.vendor() == "nvidia").unwrap();
    assert!(nvidia.supports_feature("transfer_queue"));

    // Unified memory devices don't
    let apple = devices.iter().find(|d| d.vendor() == "apple").unwrap();
    assert!(!apple.supports_feature("transfer_queue"));
}

// =========================================================================
// Logical device creation
// =========================================================================

#[test]
fn test_create_logical_device_default_queues() {
    let instance = RuntimeInstance::new(None);
    let device = instance.create_logical_device(0, None).unwrap();
    assert!(device.queues().contains_key("compute"));
    assert_eq!(device.queues()["compute"].len(), 1);
}

#[test]
fn test_create_logical_device_custom_queues() {
    let instance = RuntimeInstance::new(None);
    let device = instance
        .create_logical_device(0, Some(vec![("compute", 4), ("transfer", 2)]))
        .unwrap();
    assert_eq!(device.queues()["compute"].len(), 4);
    assert_eq!(device.queues()["transfer"].len(), 2);
}

// =========================================================================
// Memory management
// =========================================================================

#[test]
fn test_allocate_and_free_buffer() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    let buf_id = device
        .memory_manager_mut()
        .allocate(1024, MemoryType::DEVICE_LOCAL, BufferUsage::STORAGE)
        .unwrap();

    assert_eq!(device.memory_manager().allocated_buffer_count(), 1);
    assert_eq!(device.memory_manager().current_allocated_bytes(), 1024);

    device.memory_manager_mut().free(buf_id).unwrap();
    assert_eq!(device.memory_manager().allocated_buffer_count(), 0);
}

#[test]
fn test_map_write_read_unmap() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    let buf_id = device
        .memory_manager_mut()
        .allocate(
            256,
            MemoryType::HOST_VISIBLE | MemoryType::HOST_COHERENT,
            BufferUsage::STORAGE,
        )
        .unwrap();

    // Map, write, read
    {
        let mut mapped = device.memory_manager_mut().map(buf_id).unwrap();
        mapped.write(0, &[10, 20, 30, 40]).unwrap();
        let data = mapped.read(0, 4).unwrap();
        assert_eq!(data, vec![10, 20, 30, 40]);
    }

    // Unmap
    device.memory_manager_mut().unmap(buf_id).unwrap();
}

#[test]
fn test_cannot_map_device_local() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    let buf_id = device
        .memory_manager_mut()
        .allocate(64, MemoryType::DEVICE_LOCAL, BufferUsage::STORAGE)
        .unwrap();

    assert!(device.memory_manager_mut().map(buf_id).is_err());
}

#[test]
fn test_allocate_zero_bytes_fails() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();
    assert!(device
        .memory_manager_mut()
        .allocate(0, MemoryType::DEVICE_LOCAL, BufferUsage::STORAGE)
        .is_err());
}

#[test]
fn test_peak_allocation_tracking() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    let a = device
        .memory_manager_mut()
        .allocate(1000, MemoryType::DEVICE_LOCAL, BufferUsage::STORAGE)
        .unwrap();
    let b = device
        .memory_manager_mut()
        .allocate(2000, MemoryType::DEVICE_LOCAL, BufferUsage::STORAGE)
        .unwrap();

    // Peak should be 3000
    assert_eq!(device.stats().peak_allocated_bytes, 3000);

    device.memory_manager_mut().free(a).unwrap();
    // Peak stays at 3000 even after freeing
    assert_eq!(device.stats().peak_allocated_bytes, 3000);
    assert_eq!(device.memory_manager().current_allocated_bytes(), 2000);

    device.memory_manager_mut().free(b).unwrap();
}

// =========================================================================
// Command buffer lifecycle
// =========================================================================

#[test]
fn test_command_buffer_state_machine() {
    let mut cb = CommandBuffer::new();

    // INITIAL -> RECORDING
    assert_eq!(cb.state(), CommandBufferState::Initial);
    cb.begin().unwrap();
    assert_eq!(cb.state(), CommandBufferState::Recording);

    // RECORDING -> RECORDED
    cb.end().unwrap();
    assert_eq!(cb.state(), CommandBufferState::Recorded);

    // Can reset to Initial
    cb.reset();
    assert_eq!(cb.state(), CommandBufferState::Initial);
}

#[test]
fn test_cannot_record_without_begin() {
    let mut cb = CommandBuffer::new();
    assert!(cb.cmd_bind_pipeline(0).is_err());
    assert!(cb.cmd_dispatch(1, 1, 1).is_err());
}

#[test]
fn test_cannot_dispatch_without_pipeline() {
    let mut cb = CommandBuffer::new();
    cb.begin().unwrap();
    assert!(cb.cmd_dispatch(1, 1, 1).is_err());
}

#[test]
fn test_record_multiple_commands() {
    let mut cb = CommandBuffer::new();
    cb.begin().unwrap();
    cb.cmd_bind_pipeline(0).unwrap();
    cb.cmd_dispatch(4, 1, 1).unwrap();
    cb.cmd_pipeline_barrier(&PipelineBarrier {
        src_stage: PipelineStage::Compute,
        dst_stage: PipelineStage::Compute,
        ..PipelineBarrier::default()
    })
    .unwrap();
    cb.cmd_bind_pipeline(0).unwrap();
    cb.cmd_dispatch(2, 2, 1).unwrap();
    cb.end().unwrap();

    assert_eq!(cb.commands().len(), 5);
}

// =========================================================================
// Pipeline creation
// =========================================================================

#[test]
fn test_create_pipeline_gpu_style() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    let shader = device.create_shader_module(
        Some(vec![limm(0, 42.0), halt()]),
        "",
        "main",
        (32, 1, 1),
    );
    assert!(shader.is_gpu_style());

    let ds_layout = device.create_descriptor_set_layout(vec![]);
    let pl_layout = device.create_pipeline_layout(vec![ds_layout], 0);
    let pipeline_id = device.create_compute_pipeline(shader, pl_layout);
    assert!(device.pipelines().contains_key(&pipeline_id));
}

#[test]
fn test_create_pipeline_dataflow_style() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    let shader = device.create_shader_module(None, "matmul", "main", (1, 1, 1));
    assert!(shader.is_dataflow_style());
    assert!(!shader.is_gpu_style());

    let ds_layout = device.create_descriptor_set_layout(vec![]);
    let pl_layout = device.create_pipeline_layout(vec![ds_layout], 0);
    let _pid = device.create_compute_pipeline(shader, pl_layout);
}

#[test]
fn test_descriptor_set_binding() {
    let instance = RuntimeInstance::new(None);
    let device = instance.create_logical_device(0, None).unwrap();

    let layout = device.create_descriptor_set_layout(vec![
        DescriptorBinding::new(0),
        DescriptorBinding::new(1),
        DescriptorBinding::new(2),
    ]);
    let mut ds = device.create_descriptor_set(layout);
    ds.write(0, 100).unwrap();
    ds.write(1, 200).unwrap();
    ds.write(2, 300).unwrap();

    assert_eq!(ds.get_buffer_id(0), Some(100));
    assert_eq!(ds.get_buffer_id(1), Some(200));
    assert_eq!(ds.get_buffer_id(2), Some(300));
    assert_eq!(ds.get_buffer_id(3), None);
}

#[test]
fn test_descriptor_set_invalid_binding() {
    let instance = RuntimeInstance::new(None);
    let device = instance.create_logical_device(0, None).unwrap();

    let layout = device.create_descriptor_set_layout(vec![DescriptorBinding::new(0)]);
    let mut ds = device.create_descriptor_set(layout);
    assert!(ds.write(5, 10).is_err());
}

// =========================================================================
// Synchronization primitives
// =========================================================================

#[test]
fn test_fence_lifecycle() {
    let mut fence = Fence::new(false);
    assert!(!fence.signaled());

    fence.signal();
    assert!(fence.signaled());
    assert!(fence.wait(None));

    fence.reset();
    assert!(!fence.signaled());
}

#[test]
fn test_semaphore_lifecycle() {
    let mut sem = Semaphore::new();
    assert!(!sem.signaled());

    sem.signal();
    assert!(sem.signaled());

    sem.reset();
    assert!(!sem.signaled());
}

#[test]
fn test_event_lifecycle() {
    let mut event = Event::new();
    assert!(!event.signaled());

    event.set();
    assert!(event.status());

    event.reset();
    assert!(!event.status());
}

// =========================================================================
// Full pipeline: record -> submit -> fence wait
// =========================================================================

#[test]
fn test_full_gpu_pipeline() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    // Create pipeline
    let shader = device.create_shader_module(
        Some(vec![limm(0, 42.0), halt()]),
        "",
        "main",
        (32, 1, 1),
    );
    let ds_layout = device.create_descriptor_set_layout(vec![]);
    let pl_layout = device.create_pipeline_layout(vec![ds_layout], 0);
    let pipeline_id = device.create_compute_pipeline(shader, pl_layout);

    // Record commands
    let mut cb = device.create_command_buffer();
    cb.begin().unwrap();
    cb.cmd_bind_pipeline(pipeline_id).unwrap();
    cb.cmd_dispatch(1, 1, 1).unwrap();
    cb.end().unwrap();

    // Submit with fence
    let mut fence = device.create_fence(false);
    let traces = device
        .submit(
            "compute",
            0,
            &mut [&mut cb],
            &mut [],
            &mut [],
            Some(&mut fence),
        )
        .unwrap();

    // Fence should be signaled
    assert!(fence.signaled());
    assert!(fence.wait(None));

    // CB should be complete
    assert_eq!(cb.state(), CommandBufferState::Complete);

    // Should have generated traces
    assert!(!traces.is_empty());

    // Stats should reflect the submission
    assert_eq!(device.stats().total_submissions, 1);
    assert_eq!(device.stats().total_command_buffers, 1);
    assert_eq!(device.stats().total_dispatches, 1);
}

#[test]
fn test_full_dataflow_pipeline() {
    let instance = RuntimeInstance::new(None);
    // Use Google TPU (index 2)
    let mut device = instance.create_logical_device(2, None).unwrap();

    let shader = device.create_shader_module(None, "matmul", "main", (1, 1, 1));
    let ds_layout = device.create_descriptor_set_layout(vec![]);
    let pl_layout = device.create_pipeline_layout(vec![ds_layout], 0);
    let pipeline_id = device.create_compute_pipeline(shader, pl_layout);

    let mut cb = device.create_command_buffer();
    cb.begin().unwrap();
    cb.cmd_bind_pipeline(pipeline_id).unwrap();
    cb.cmd_dispatch(1, 1, 1).unwrap();
    cb.end().unwrap();

    let mut fence = device.create_fence(false);
    let traces = device
        .submit(
            "compute",
            0,
            &mut [&mut cb],
            &mut [],
            &mut [],
            Some(&mut fence),
        )
        .unwrap();

    assert!(fence.signaled());
    assert!(!traces.is_empty());
}

#[test]
fn test_multiple_dispatches() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    let shader = device.create_shader_module(
        Some(vec![limm(0, 1.0), halt()]),
        "",
        "main",
        (32, 1, 1),
    );
    let ds_layout = device.create_descriptor_set_layout(vec![]);
    let pl_layout = device.create_pipeline_layout(vec![ds_layout], 0);
    let pipeline_id = device.create_compute_pipeline(shader, pl_layout);

    let mut cb = device.create_command_buffer();
    cb.begin().unwrap();
    cb.cmd_bind_pipeline(pipeline_id).unwrap();
    cb.cmd_dispatch(2, 1, 1).unwrap();
    cb.cmd_pipeline_barrier(&PipelineBarrier {
        src_stage: PipelineStage::Compute,
        dst_stage: PipelineStage::Compute,
        ..PipelineBarrier::default()
    })
    .unwrap();
    cb.cmd_dispatch(4, 1, 1).unwrap();
    cb.end().unwrap();

    let mut fence = device.create_fence(false);
    device
        .submit(
            "compute",
            0,
            &mut [&mut cb],
            &mut [],
            &mut [],
            Some(&mut fence),
        )
        .unwrap();

    assert!(fence.signaled());
    assert_eq!(device.stats().total_dispatches, 2);
    assert_eq!(device.stats().total_barriers, 1);
}

// =========================================================================
// Semaphore synchronization
// =========================================================================

#[test]
fn test_semaphore_between_submissions() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance
        .create_logical_device(0, Some(vec![("compute", 2)]))
        .unwrap();

    let shader = device.create_shader_module(
        Some(vec![limm(0, 1.0), halt()]),
        "",
        "main",
        (32, 1, 1),
    );
    let ds_layout = device.create_descriptor_set_layout(vec![]);
    let pl_layout = device.create_pipeline_layout(vec![ds_layout], 0);
    let pipeline_id = device.create_compute_pipeline(shader, pl_layout);

    // First submission signals semaphore
    let mut cb1 = device.create_command_buffer();
    cb1.begin().unwrap();
    cb1.cmd_bind_pipeline(pipeline_id).unwrap();
    cb1.cmd_dispatch(1, 1, 1).unwrap();
    cb1.end().unwrap();

    let mut sem = device.create_semaphore();
    device
        .submit(
            "compute",
            0,
            &mut [&mut cb1],
            &mut [],
            &mut [&mut sem],
            None,
        )
        .unwrap();

    assert!(sem.signaled());

    // Second submission waits on semaphore
    let mut cb2 = device.create_command_buffer();
    cb2.begin().unwrap();
    cb2.cmd_bind_pipeline(pipeline_id).unwrap();
    cb2.cmd_dispatch(1, 1, 1).unwrap();
    cb2.end().unwrap();

    let mut fence = device.create_fence(false);
    device
        .submit(
            "compute",
            0,
            &mut [&mut cb2],
            &mut [&mut sem],
            &mut [],
            Some(&mut fence),
        )
        .unwrap();

    assert!(fence.signaled());
    // Semaphore should be consumed (reset)
    assert!(!sem.signaled());
}

// =========================================================================
// Transfer commands
// =========================================================================

#[test]
fn test_copy_buffer_command() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    // Allocate source and destination
    let src_id = device
        .memory_manager_mut()
        .allocate(
            64,
            MemoryType::HOST_VISIBLE | MemoryType::HOST_COHERENT,
            BufferUsage::STORAGE | BufferUsage::TRANSFER_SRC,
        )
        .unwrap();
    let dst_id = device
        .memory_manager_mut()
        .allocate(
            64,
            MemoryType::HOST_VISIBLE | MemoryType::HOST_COHERENT,
            BufferUsage::STORAGE | BufferUsage::TRANSFER_DST,
        )
        .unwrap();

    // Write to source
    {
        let mut mapped = device.memory_manager_mut().map(src_id).unwrap();
        mapped.write(0, &[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
    }
    device.memory_manager_mut().unmap(src_id).unwrap();

    // Need a pipeline for the CB (even for transfer-only)
    let shader = device.create_shader_module(
        Some(vec![limm(0, 1.0), halt()]),
        "",
        "main",
        (32, 1, 1),
    );
    let ds_layout = device.create_descriptor_set_layout(vec![]);
    let pl_layout = device.create_pipeline_layout(vec![ds_layout], 0);
    let _pid = device.create_compute_pipeline(shader, pl_layout);

    // Record copy
    let mut cb = device.create_command_buffer();
    cb.begin().unwrap();
    cb.cmd_copy_buffer(src_id, dst_id, 8, 0, 0).unwrap();
    cb.end().unwrap();

    let mut fence = device.create_fence(false);
    device
        .submit(
            "compute",
            0,
            &mut [&mut cb],
            &mut [],
            &mut [],
            Some(&mut fence),
        )
        .unwrap();

    assert!(fence.signaled());
    assert!(device.stats().total_transfers > 0);
}

#[test]
fn test_fill_buffer_command() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    let buf_id = device
        .memory_manager_mut()
        .allocate(
            64,
            MemoryType::HOST_VISIBLE | MemoryType::HOST_COHERENT,
            BufferUsage::STORAGE | BufferUsage::TRANSFER_DST,
        )
        .unwrap();

    let mut cb = device.create_command_buffer();
    cb.begin().unwrap();
    cb.cmd_fill_buffer(buf_id, 0xFF, 0, 64).unwrap();
    cb.end().unwrap();

    let mut fence = device.create_fence(false);
    device
        .submit(
            "compute",
            0,
            &mut [&mut cb],
            &mut [],
            &mut [],
            Some(&mut fence),
        )
        .unwrap();

    assert!(fence.signaled());
}

#[test]
fn test_update_buffer_command() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    let buf_id = device
        .memory_manager_mut()
        .allocate(
            64,
            MemoryType::HOST_VISIBLE | MemoryType::HOST_COHERENT,
            BufferUsage::STORAGE,
        )
        .unwrap();

    let mut cb = device.create_command_buffer();
    cb.begin().unwrap();
    cb.cmd_update_buffer(buf_id, 0, &[42, 43, 44]).unwrap();
    cb.end().unwrap();

    let mut fence = device.create_fence(false);
    device
        .submit(
            "compute",
            0,
            &mut [&mut cb],
            &mut [],
            &mut [],
            Some(&mut fence),
        )
        .unwrap();

    assert!(fence.signaled());
}

// =========================================================================
// Validation layer
// =========================================================================

#[test]
fn test_validation_begin_states() {
    let layer = ValidationLayer::new();
    let cb = CommandBuffer::new();
    assert!(layer.validate_begin(&cb).is_ok());
}

#[test]
fn test_validation_dispatch_no_pipeline() {
    let layer = ValidationLayer::new();
    let mut cb = CommandBuffer::new();
    cb.begin().unwrap();
    assert!(layer.validate_dispatch(&cb, 1, 1, 1).is_err());
}

#[test]
fn test_validation_dispatch_zero_dims() {
    let layer = ValidationLayer::new();
    let mut cb = CommandBuffer::new();
    cb.begin().unwrap();
    cb.cmd_bind_pipeline(0).unwrap();
    assert!(layer.validate_dispatch(&cb, 0, 1, 1).is_err());
}

#[test]
fn test_validation_map_device_local() {
    let layer = ValidationLayer::new();
    let buf = compute_runtime::memory::Buffer {
        buffer_id: 0,
        size: 64,
        memory_type: MemoryType::DEVICE_LOCAL,
        usage: BufferUsage::STORAGE,
        device_address: 0,
        mapped: false,
        freed: false,
    };
    assert!(layer.validate_map(&buf).is_err());
}

#[test]
fn test_validation_buffer_usage() {
    let layer = ValidationLayer::new();
    let buf = compute_runtime::memory::Buffer {
        buffer_id: 0,
        size: 64,
        memory_type: MemoryType::DEVICE_LOCAL,
        usage: BufferUsage::STORAGE,
        device_address: 0,
        mapped: false,
        freed: false,
    };
    assert!(layer
        .validate_buffer_usage(&buf, BufferUsage::STORAGE)
        .is_ok());
    assert!(layer
        .validate_buffer_usage(&buf, BufferUsage::TRANSFER_SRC)
        .is_err());
}

#[test]
fn test_validation_barrier_tracking() {
    let mut layer = ValidationLayer::new();
    layer.record_write(0);
    layer.validate_read_after_write(0);
    assert_eq!(layer.warnings().len(), 1);

    layer.record_barrier(None);
    // After barrier, new reads shouldn't warn (but old warning persists)
    // We'd need to create a new layer to test cleanly
    let mut layer2 = ValidationLayer::new();
    layer2.record_write(0);
    layer2.record_barrier(None);
    layer2.validate_read_after_write(0);
    assert!(layer2.warnings().is_empty());
}

// =========================================================================
// Bitflags
// =========================================================================

#[test]
fn test_memory_type_combinations() {
    let unified =
        MemoryType::DEVICE_LOCAL | MemoryType::HOST_VISIBLE | MemoryType::HOST_COHERENT;
    assert!(unified.contains(MemoryType::DEVICE_LOCAL));
    assert!(unified.contains(MemoryType::HOST_VISIBLE));
    assert!(unified.contains(MemoryType::HOST_COHERENT));
    assert!(!unified.contains(MemoryType::HOST_CACHED));
}

#[test]
fn test_buffer_usage_combinations() {
    let usage = BufferUsage::STORAGE | BufferUsage::TRANSFER_SRC | BufferUsage::TRANSFER_DST;
    assert!(usage.contains(BufferUsage::STORAGE));
    assert!(usage.contains(BufferUsage::TRANSFER_SRC));
    assert!(!usage.contains(BufferUsage::UNIFORM));
}

#[test]
fn test_access_flags_combinations() {
    let access = AccessFlags::SHADER_READ | AccessFlags::SHADER_WRITE;
    assert!(access.contains(AccessFlags::SHADER_READ));
    assert!(!access.contains(AccessFlags::HOST_READ));
}

// =========================================================================
// Stats and traces
// =========================================================================

#[test]
fn test_runtime_stats_after_operations() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    // Allocate some buffers
    let _a = device
        .memory_manager_mut()
        .allocate(100, MemoryType::DEVICE_LOCAL, BufferUsage::STORAGE)
        .unwrap();
    let _b = device
        .memory_manager_mut()
        .allocate(200, MemoryType::DEVICE_LOCAL, BufferUsage::STORAGE)
        .unwrap();

    assert_eq!(device.stats().total_allocations, 2);
    assert_eq!(device.stats().total_allocated_bytes, 300);
}

#[test]
fn test_runtime_trace_format() {
    let trace = RuntimeTrace {
        timestamp_cycles: 150,
        event_type: RuntimeEventType::Submit,
        description: "CB#1 to compute queue".to_string(),
        queue_type: Some(QueueType::Compute),
        command_buffer_id: None,
        fence_id: None,
        semaphore_id: None,
    };
    let formatted = trace.format();
    assert!(formatted.contains("150"));
    assert!(formatted.contains("SUBMIT"));
    assert!(formatted.contains("CB#1"));
}

#[test]
fn test_gpu_utilization_calculation() {
    let mut stats = RuntimeStats::default();
    stats.total_device_cycles = 800;
    stats.total_idle_cycles = 200;
    stats.update_utilization();
    assert!((stats.gpu_utilization - 0.8).abs() < 0.001);
}

// =========================================================================
// Command buffer reset and reuse
// =========================================================================

#[test]
fn test_command_buffer_reuse() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    let shader = device.create_shader_module(
        Some(vec![limm(0, 1.0), halt()]),
        "",
        "main",
        (32, 1, 1),
    );
    let ds_layout = device.create_descriptor_set_layout(vec![]);
    let pl_layout = device.create_pipeline_layout(vec![ds_layout], 0);
    let pipeline_id = device.create_compute_pipeline(shader, pl_layout);

    let mut cb = device.create_command_buffer();

    // First use
    cb.begin().unwrap();
    cb.cmd_bind_pipeline(pipeline_id).unwrap();
    cb.cmd_dispatch(1, 1, 1).unwrap();
    cb.end().unwrap();

    let mut fence = device.create_fence(false);
    device
        .submit(
            "compute",
            0,
            &mut [&mut cb],
            &mut [],
            &mut [],
            Some(&mut fence),
        )
        .unwrap();
    assert!(fence.signaled());

    // Reuse: begin from COMPLETE state
    cb.begin().unwrap();
    cb.cmd_bind_pipeline(pipeline_id).unwrap();
    cb.cmd_dispatch(2, 1, 1).unwrap();
    cb.end().unwrap();

    fence.reset();
    device
        .submit(
            "compute",
            0,
            &mut [&mut cb],
            &mut [],
            &mut [],
            Some(&mut fence),
        )
        .unwrap();
    assert!(fence.signaled());

    assert_eq!(device.stats().total_dispatches, 2);
}

// =========================================================================
// Edge cases
// =========================================================================

#[test]
fn test_submit_unrecorded_cb_fails() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    let mut cb = device.create_command_buffer();
    // Don't record anything

    let result = device.submit("compute", 0, &mut [&mut cb], &mut [], &mut [], None);
    assert!(result.is_err());
}

#[test]
fn test_wait_on_unsignaled_semaphore_fails() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    let shader = device.create_shader_module(
        Some(vec![limm(0, 1.0), halt()]),
        "",
        "main",
        (32, 1, 1),
    );
    let ds_layout = device.create_descriptor_set_layout(vec![]);
    let pl_layout = device.create_pipeline_layout(vec![ds_layout], 0);
    let pipeline_id = device.create_compute_pipeline(shader, pl_layout);

    let mut cb = device.create_command_buffer();
    cb.begin().unwrap();
    cb.cmd_bind_pipeline(pipeline_id).unwrap();
    cb.cmd_dispatch(1, 1, 1).unwrap();
    cb.end().unwrap();

    let mut sem = device.create_semaphore();
    // Don't signal the semaphore
    let result = device.submit(
        "compute",
        0,
        &mut [&mut cb],
        &mut [&mut sem],
        &mut [],
        None,
    );
    assert!(result.is_err());
}

#[test]
fn test_double_free_fails() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    let buf_id = device
        .memory_manager_mut()
        .allocate(64, MemoryType::DEVICE_LOCAL, BufferUsage::STORAGE)
        .unwrap();
    device.memory_manager_mut().free(buf_id).unwrap();
    assert!(device.memory_manager_mut().free(buf_id).is_err());
}

// =========================================================================
// Barrier recording
// =========================================================================

#[test]
fn test_pipeline_barrier_in_command_buffer() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    let shader = device.create_shader_module(
        Some(vec![limm(0, 1.0), halt()]),
        "",
        "main",
        (32, 1, 1),
    );
    let ds_layout = device.create_descriptor_set_layout(vec![]);
    let pl_layout = device.create_pipeline_layout(vec![ds_layout], 0);
    let pipeline_id = device.create_compute_pipeline(shader, pl_layout);

    let mut cb = device.create_command_buffer();
    cb.begin().unwrap();
    cb.cmd_bind_pipeline(pipeline_id).unwrap();
    cb.cmd_dispatch(1, 1, 1).unwrap();
    cb.cmd_pipeline_barrier(&PipelineBarrier {
        src_stage: PipelineStage::Compute,
        dst_stage: PipelineStage::Transfer,
        memory_barriers: vec![MemoryBarrier {
            src_access: AccessFlags::SHADER_WRITE,
            dst_access: AccessFlags::TRANSFER_READ,
        }],
        buffer_barriers: vec![],
    })
    .unwrap();
    cb.cmd_dispatch(1, 1, 1).unwrap();
    cb.end().unwrap();

    let mut fence = device.create_fence(false);
    device
        .submit(
            "compute",
            0,
            &mut [&mut cb],
            &mut [],
            &mut [],
            Some(&mut fence),
        )
        .unwrap();
    assert!(fence.signaled());
    assert_eq!(device.stats().total_barriers, 1);
}
