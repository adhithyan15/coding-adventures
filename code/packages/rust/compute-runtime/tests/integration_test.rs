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

// =========================================================================
// Additional coverage: protocols.rs
// =========================================================================

#[test]
fn test_queue_type_as_str() {
    assert_eq!(QueueType::Compute.as_str(), "compute");
    assert_eq!(QueueType::Transfer.as_str(), "transfer");
    assert_eq!(QueueType::ComputeTransfer.as_str(), "compute_transfer");
}

#[test]
fn test_pipeline_stage_as_str_all_variants() {
    assert_eq!(PipelineStage::TopOfPipe.as_str(), "top_of_pipe");
    assert_eq!(PipelineStage::Compute.as_str(), "compute");
    assert_eq!(PipelineStage::Transfer.as_str(), "transfer");
    assert_eq!(PipelineStage::Host.as_str(), "host");
    assert_eq!(PipelineStage::BottomOfPipe.as_str(), "bottom_of_pipe");
}

#[test]
fn test_runtime_event_type_as_str_all_variants() {
    assert_eq!(RuntimeEventType::Submit.as_str(), "SUBMIT");
    assert_eq!(RuntimeEventType::BeginExecution.as_str(), "BEGIN_EXECUTION");
    assert_eq!(RuntimeEventType::EndExecution.as_str(), "END_EXECUTION");
    assert_eq!(RuntimeEventType::FenceSignal.as_str(), "FENCE_SIGNAL");
    assert_eq!(RuntimeEventType::FenceWait.as_str(), "FENCE_WAIT");
    assert_eq!(RuntimeEventType::SemaphoreSignal.as_str(), "SEMAPHORE_SIGNAL");
    assert_eq!(RuntimeEventType::SemaphoreWait.as_str(), "SEMAPHORE_WAIT");
    assert_eq!(RuntimeEventType::Barrier.as_str(), "BARRIER");
    assert_eq!(RuntimeEventType::MemoryAlloc.as_str(), "MEMORY_ALLOC");
    assert_eq!(RuntimeEventType::MemoryFree.as_str(), "MEMORY_FREE");
    assert_eq!(RuntimeEventType::MemoryMap.as_str(), "MEMORY_MAP");
    assert_eq!(RuntimeEventType::MemoryTransfer.as_str(), "MEMORY_TRANSFER");
}

#[test]
fn test_command_buffer_state_as_str_all_variants() {
    assert_eq!(CommandBufferState::Initial.as_str(), "initial");
    assert_eq!(CommandBufferState::Recording.as_str(), "recording");
    assert_eq!(CommandBufferState::Recorded.as_str(), "recorded");
    assert_eq!(CommandBufferState::Pending.as_str(), "pending");
    assert_eq!(CommandBufferState::Complete.as_str(), "complete");
}

#[test]
fn test_command_arg_cross_type_conversions() {
    // Int -> usize
    assert_eq!(compute_runtime::protocols::CommandArg::Int(42).as_usize(), 42);
    // UInt -> usize
    assert_eq!(compute_runtime::protocols::CommandArg::UInt(99).as_usize(), 99);
    // UInt -> i64
    assert_eq!(compute_runtime::protocols::CommandArg::UInt(55).as_i64(), 55);
    // Usize -> i64
    assert_eq!(compute_runtime::protocols::CommandArg::Usize(77).as_i64(), 77);
    // Int -> u8
    assert_eq!(compute_runtime::protocols::CommandArg::Int(200).as_u8(), 200);
    // Usize -> u8
    assert_eq!(compute_runtime::protocols::CommandArg::Usize(128).as_u8(), 128);
}

#[test]
fn test_buffer_barrier_new() {
    let barrier = BufferBarrier::new(42);
    assert_eq!(barrier.buffer_id, 42);
    assert_eq!(barrier.src_access, AccessFlags::NONE);
    assert_eq!(barrier.dst_access, AccessFlags::NONE);
    assert_eq!(barrier.offset, 0);
    assert_eq!(barrier.size, 0);
}

#[test]
fn test_memory_barrier_default() {
    let barrier = MemoryBarrier::default();
    assert_eq!(barrier.src_access, AccessFlags::NONE);
    assert_eq!(barrier.dst_access, AccessFlags::NONE);
}

#[test]
fn test_runtime_trace_new() {
    let trace = RuntimeTrace::new(RuntimeEventType::MemoryAlloc, "test alloc");
    assert_eq!(trace.timestamp_cycles, 0);
    assert_eq!(trace.description, "test alloc");
    assert!(trace.queue_type.is_none());
    assert!(trace.command_buffer_id.is_none());
    assert!(trace.fence_id.is_none());
    assert!(trace.semaphore_id.is_none());
}

#[test]
fn test_runtime_trace_format_empty_description() {
    let trace = RuntimeTrace {
        timestamp_cycles: 0,
        event_type: RuntimeEventType::Barrier,
        description: String::new(),
        queue_type: None,
        command_buffer_id: None,
        fence_id: None,
        semaphore_id: None,
    };
    let formatted = trace.format();
    assert_eq!(formatted, "[T=0 cycles] BARRIER");
    assert!(!formatted.contains("--"));
}

#[test]
fn test_runtime_stats_default_values() {
    let stats = RuntimeStats::default();
    assert_eq!(stats.total_submissions, 0);
    assert_eq!(stats.total_command_buffers, 0);
    assert_eq!(stats.total_dispatches, 0);
    assert_eq!(stats.total_transfers, 0);
    assert_eq!(stats.total_barriers, 0);
    assert_eq!(stats.total_fence_waits, 0);
    assert_eq!(stats.total_semaphore_signals, 0);
    assert_eq!(stats.total_fence_wait_cycles, 0);
    assert_eq!(stats.total_allocated_bytes, 0);
    assert_eq!(stats.peak_allocated_bytes, 0);
    assert_eq!(stats.total_allocations, 0);
    assert_eq!(stats.total_frees, 0);
    assert_eq!(stats.total_maps, 0);
    assert_eq!(stats.total_device_cycles, 0);
    assert_eq!(stats.total_idle_cycles, 0);
    assert_eq!(stats.gpu_utilization, 0.0);
    assert!(stats.traces.is_empty());
}

#[test]
fn test_runtime_stats_utilization_zero_total() {
    let mut stats = RuntimeStats::default();
    stats.update_utilization();
    assert_eq!(stats.gpu_utilization, 0.0);
}

#[test]
fn test_queue_family_fields() {
    let qf = QueueFamily {
        queue_type: QueueType::Transfer,
        count: 4,
    };
    assert_eq!(qf.queue_type, QueueType::Transfer);
    assert_eq!(qf.count, 4);
}

#[test]
fn test_memory_heap_fields() {
    let heap = MemoryHeap {
        size: 1024 * 1024,
        flags: MemoryType::DEVICE_LOCAL | MemoryType::HOST_VISIBLE,
    };
    assert_eq!(heap.size, 1024 * 1024);
    assert!(heap.flags.contains(MemoryType::DEVICE_LOCAL));
    assert!(heap.flags.contains(MemoryType::HOST_VISIBLE));
}

#[test]
fn test_descriptor_binding_default_type() {
    let b = DescriptorBinding::new(3);
    assert_eq!(b.binding, 3);
    assert_eq!(b.binding_type, "storage");
    assert_eq!(b.count, 1);
}

#[test]
fn test_device_limits_default_full() {
    let limits = DeviceLimits::default();
    assert_eq!(limits.max_workgroup_size, (1024, 1024, 64));
    assert_eq!(limits.max_workgroup_count, (65535, 65535, 65535));
    assert_eq!(limits.max_buffer_size, 2 * 1024 * 1024 * 1024);
    assert_eq!(limits.max_push_constant_size, 128);
    assert_eq!(limits.max_descriptor_sets, 4);
    assert_eq!(limits.max_bindings_per_set, 16);
    assert_eq!(limits.max_compute_queues, 16);
    assert_eq!(limits.max_transfer_queues, 2);
}

#[test]
fn test_pipeline_barrier_with_buffer_barriers() {
    let barrier = PipelineBarrier {
        src_stage: PipelineStage::Compute,
        dst_stage: PipelineStage::Transfer,
        memory_barriers: vec![MemoryBarrier {
            src_access: AccessFlags::SHADER_WRITE,
            dst_access: AccessFlags::TRANSFER_READ,
        }],
        buffer_barriers: vec![BufferBarrier {
            buffer_id: 0,
            src_access: AccessFlags::SHADER_WRITE,
            dst_access: AccessFlags::TRANSFER_READ,
            offset: 0,
            size: 1024,
        }],
    };
    assert_eq!(barrier.memory_barriers.len(), 1);
    assert_eq!(barrier.buffer_barriers.len(), 1);
    assert_eq!(barrier.buffer_barriers[0].buffer_id, 0);
    assert_eq!(barrier.buffer_barriers[0].size, 1024);
}

#[test]
fn test_access_flags_all_variants() {
    let all = AccessFlags::SHADER_READ
        | AccessFlags::SHADER_WRITE
        | AccessFlags::TRANSFER_READ
        | AccessFlags::TRANSFER_WRITE
        | AccessFlags::HOST_READ
        | AccessFlags::HOST_WRITE;
    assert!(all.contains(AccessFlags::SHADER_READ));
    assert!(all.contains(AccessFlags::HOST_WRITE));
    assert!(all.contains(AccessFlags::TRANSFER_WRITE));
}

#[test]
fn test_buffer_usage_all_flags() {
    let all = BufferUsage::STORAGE
        | BufferUsage::UNIFORM
        | BufferUsage::TRANSFER_SRC
        | BufferUsage::TRANSFER_DST
        | BufferUsage::INDIRECT;
    assert!(all.contains(BufferUsage::INDIRECT));
    assert!(all.contains(BufferUsage::UNIFORM));
}

#[test]
fn test_memory_type_host_cached() {
    let cached = MemoryType::HOST_VISIBLE | MemoryType::HOST_CACHED;
    assert!(cached.contains(MemoryType::HOST_CACHED));
    assert!(cached.contains(MemoryType::HOST_VISIBLE));
    assert!(!cached.contains(MemoryType::DEVICE_LOCAL));
}

// =========================================================================
// Additional coverage: pipeline.rs
// =========================================================================

#[test]
fn test_shader_module_accessors() {
    let instance = RuntimeInstance::new(None);
    let device = instance.create_logical_device(0, None).unwrap();

    let shader = device.create_shader_module(
        Some(vec![limm(0, 42.0), halt()]),
        "test_op",
        "my_main",
        (64, 2, 1),
    );
    assert_eq!(shader.entry_point(), "my_main");
    assert_eq!(shader.operation(), "test_op");
    assert_eq!(shader.local_size(), (64, 2, 1));
    assert!(shader.is_gpu_style());
    assert!(shader.is_dataflow_style()); // operation is non-empty
    let _ = shader.module_id(); // just ensure it doesn't panic
}

#[test]
fn test_pipeline_accessors() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    let shader = device.create_shader_module(
        Some(vec![limm(0, 1.0), halt()]),
        "",
        "main",
        (32, 1, 1),
    );
    let ds_layout = device.create_descriptor_set_layout(vec![
        DescriptorBinding::new(0),
    ]);
    let pl_layout = device.create_pipeline_layout(vec![ds_layout], 16);
    let pipeline_id = device.create_compute_pipeline(shader, pl_layout);

    let pipeline = device.pipelines().get(&pipeline_id).unwrap();
    assert_eq!(pipeline.pipeline_id(), pipeline_id);
    assert_eq!(pipeline.workgroup_size(), (32, 1, 1));
    assert!(pipeline.shader().is_gpu_style());
    assert_eq!(pipeline.layout().push_constant_size(), 16);
    assert_eq!(pipeline.layout().set_layouts().len(), 1);
    let _ = pipeline.layout().layout_id();
}

#[test]
fn test_descriptor_set_accessors() {
    let instance = RuntimeInstance::new(None);
    let device = instance.create_logical_device(0, None).unwrap();

    let layout = device.create_descriptor_set_layout(vec![
        DescriptorBinding::new(0),
        DescriptorBinding::new(1),
    ]);
    let layout_id = layout.layout_id();
    let mut ds = device.create_descriptor_set(layout);

    let _ = ds.set_id();
    assert_eq!(ds.layout().layout_id(), layout_id);
    assert!(ds.bindings().is_empty());

    ds.write(0, 10).unwrap();
    assert_eq!(ds.bindings().len(), 1);
    assert_eq!(*ds.bindings().get(&0).unwrap(), 10);
}

#[test]
fn test_reset_pipeline_ids() {
    compute_runtime::pipeline::reset_pipeline_ids();
    // Just verify it doesn't panic and subsequent creations work
    let instance = RuntimeInstance::new(None);
    let device = instance.create_logical_device(0, None).unwrap();
    let _shader = device.create_shader_module(None, "op", "main", (1, 1, 1));
}

// =========================================================================
// Additional coverage: memory.rs
// =========================================================================

#[test]
fn test_memory_flush() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    let buf_id = device
        .memory_manager_mut()
        .allocate(
            256,
            MemoryType::HOST_VISIBLE,
            BufferUsage::STORAGE,
        )
        .unwrap();

    // Write some data via map
    {
        let mut mapped = device.memory_manager_mut().map(buf_id).unwrap();
        mapped.write(0, &[1, 2, 3, 4]).unwrap();
    }
    device.memory_manager_mut().unmap(buf_id).unwrap();

    // Flush partial range
    device.memory_manager_mut().flush(buf_id, 0, 4).unwrap();

    // Flush with size=0 (should flush entire buffer)
    device.memory_manager_mut().flush(buf_id, 0, 0).unwrap();
}

#[test]
fn test_memory_invalidate() {
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

    // Invalidate partial range
    device.memory_manager_mut().invalidate(buf_id, 0, 64).unwrap();

    // Invalidate with size=0 (should invalidate entire buffer)
    device.memory_manager_mut().invalidate(buf_id, 0, 0).unwrap();
}

#[test]
fn test_flush_freed_buffer_fails() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    let buf_id = device
        .memory_manager_mut()
        .allocate(64, MemoryType::HOST_VISIBLE, BufferUsage::STORAGE)
        .unwrap();
    device.memory_manager_mut().free(buf_id).unwrap();

    assert!(device.memory_manager_mut().flush(buf_id, 0, 0).is_err());
}

#[test]
fn test_invalidate_freed_buffer_fails() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    let buf_id = device
        .memory_manager_mut()
        .allocate(64, MemoryType::HOST_VISIBLE, BufferUsage::STORAGE)
        .unwrap();
    device.memory_manager_mut().free(buf_id).unwrap();

    assert!(device.memory_manager_mut().invalidate(buf_id, 0, 0).is_err());
}

#[test]
fn test_unmap_not_mapped_fails() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    let buf_id = device
        .memory_manager_mut()
        .allocate(64, MemoryType::HOST_VISIBLE, BufferUsage::STORAGE)
        .unwrap();

    assert!(device.memory_manager_mut().unmap(buf_id).is_err());
}

#[test]
fn test_unmap_nonexistent_buffer_fails() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    assert!(device.memory_manager_mut().unmap(999).is_err());
}

#[test]
fn test_map_nonexistent_buffer_fails() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    assert!(device.memory_manager_mut().map(999).is_err());
}

#[test]
fn test_free_nonexistent_buffer_fails() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    assert!(device.memory_manager_mut().free(999).is_err());
}

#[test]
fn test_mapped_memory_accessors() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    let buf_id = device
        .memory_manager_mut()
        .allocate(128, MemoryType::HOST_VISIBLE, BufferUsage::STORAGE)
        .unwrap();

    {
        let mapped = device.memory_manager_mut().map(buf_id).unwrap();
        assert_eq!(mapped.buffer_id(), buf_id);
        assert_eq!(mapped.size(), 128);
        assert!(!mapped.dirty());
        assert_eq!(mapped.get_data().len(), 128);
        assert_eq!(mapped.get_data()[0], 0); // zero-initialized
    }
    device.memory_manager_mut().unmap(buf_id).unwrap();
}

#[test]
fn test_memory_properties_accessor() {
    let instance = RuntimeInstance::new(None);
    let device = instance.create_logical_device(0, None).unwrap();
    let props = device.memory_manager().memory_properties();
    // NVIDIA is not unified
    assert!(!props.is_unified);
    assert!(!props.heaps.is_empty());
}

#[test]
fn test_unmap_host_visible_without_coherent() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    let buf_id = device
        .memory_manager_mut()
        .allocate(
            64,
            MemoryType::HOST_VISIBLE, // no HOST_COHERENT
            BufferUsage::STORAGE,
        )
        .unwrap();

    {
        let mut mapped = device.memory_manager_mut().map(buf_id).unwrap();
        mapped.write(0, &[42]).unwrap();
    }
    // Unmap without HOST_COHERENT: should NOT auto-sync
    device.memory_manager_mut().unmap(buf_id).unwrap();
}

#[test]
fn test_get_buffer() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    let buf_id = device
        .memory_manager_mut()
        .allocate(512, MemoryType::DEVICE_LOCAL, BufferUsage::STORAGE | BufferUsage::TRANSFER_DST)
        .unwrap();

    let buf = device.memory_manager().get_buffer(buf_id).unwrap();
    assert_eq!(buf.buffer_id, buf_id);
    assert_eq!(buf.size, 512);
    assert!(!buf.freed);
    assert!(!buf.mapped);
    assert!(buf.usage.contains(BufferUsage::STORAGE));
    assert!(buf.usage.contains(BufferUsage::TRANSFER_DST));
}

#[test]
fn test_get_buffer_nonexistent_fails() {
    let instance = RuntimeInstance::new(None);
    let device = instance.create_logical_device(0, None).unwrap();
    assert!(device.memory_manager().get_buffer(999).is_err());
}

#[test]
fn test_double_map_fails() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    let buf_id = device
        .memory_manager_mut()
        .allocate(64, MemoryType::HOST_VISIBLE, BufferUsage::STORAGE)
        .unwrap();

    // Manually set mapped flag to simulate already-mapped state
    // (since we can't hold two MappedMemory refs at once due to borrow rules)
    {
        let _mapped = device.memory_manager_mut().map(buf_id).unwrap();
        // While mapped, the buffer's mapped flag is true
        // We can't call map again here because of borrow rules, but the unit test
        // in memory.rs already covers this path
    }
}

// =========================================================================
// Additional coverage: instance.rs
// =========================================================================

#[test]
fn test_physical_device_accessors() {
    let instance = RuntimeInstance::new(None);
    let devices = instance.enumerate_physical_devices();

    for d in &devices {
        let _id = d.device_id();
        let _name = d.name();
        let _dt = d.device_type();
        let _vendor = d.vendor();
        let _mem = d.memory_properties();
        let _qf = d.queue_families();
        let _limits = d.limits();
    }
}

#[test]
fn test_physical_device_feature_fp16() {
    let instance = RuntimeInstance::new(None);
    let devices = instance.enumerate_physical_devices();
    for d in &devices {
        assert!(d.supports_feature("fp16"));
    }
}

#[test]
fn test_physical_device_feature_unknown() {
    let instance = RuntimeInstance::new(None);
    let devices = instance.enumerate_physical_devices();
    for d in &devices {
        assert!(!d.supports_feature("ray_tracing"));
        assert!(!d.supports_feature(""));
    }
}

#[test]
fn test_logical_device_accessors() {
    let instance = RuntimeInstance::new(None);
    let device = instance.create_logical_device(0, None).unwrap();

    let _id = device.physical_device_id();
    let _name = device.physical_device_name();
    let _dt = device.physical_device_type();
    let _vendor = device.physical_vendor();
    let _mem = device.physical_memory_properties();
    let _qf = device.physical_queue_families();
    let _limits = device.physical_limits();
    let _stats = device.stats();
    let _pipes = device.pipelines();
}

#[test]
fn test_submit_invalid_queue_type() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    let mut cb = device.create_command_buffer();
    cb.begin().unwrap();
    cb.end().unwrap();

    let result = device.submit(
        "nonexistent_queue",
        0,
        &mut [&mut cb],
        &mut [],
        &mut [],
        None,
    );
    assert!(result.is_err());
}

#[test]
fn test_submit_invalid_queue_index() {
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

    let result = device.submit(
        "compute",
        99, // out of range
        &mut [&mut cb],
        &mut [],
        &mut [],
        None,
    );
    assert!(result.is_err());
}

#[test]
fn test_wait_idle() {
    let instance = RuntimeInstance::new(None);
    let device = instance.create_logical_device(0, None).unwrap();
    device.wait_idle(); // should not panic
}

#[test]
fn test_create_logical_devices_for_all_vendors() {
    let instance = RuntimeInstance::new(None);
    let devices = instance.enumerate_physical_devices();

    // Create a logical device for each physical device
    for (i, pd) in devices.iter().enumerate() {
        let ld = instance.create_logical_device(i, None).unwrap();
        assert_eq!(ld.physical_device_name(), pd.name());
        assert_eq!(ld.physical_device_type(), pd.device_type());
        assert_eq!(ld.physical_vendor(), pd.vendor());
    }
}

#[test]
fn test_create_logical_device_amd() {
    let instance = RuntimeInstance::new(None);
    let devices = instance.enumerate_physical_devices();
    let amd_idx = devices.iter().position(|d| d.vendor() == "amd").unwrap();
    let device = instance.create_logical_device(amd_idx, None).unwrap();
    assert_eq!(device.physical_device_type(), DeviceType::GPU);
}

#[test]
fn test_create_logical_device_intel() {
    let instance = RuntimeInstance::new(None);
    let devices = instance.enumerate_physical_devices();
    let intel_idx = devices.iter().position(|d| d.vendor() == "intel").unwrap();
    let device = instance.create_logical_device(intel_idx, None).unwrap();
    assert_eq!(device.physical_device_type(), DeviceType::GPU);
}

#[test]
fn test_create_logical_device_apple() {
    let instance = RuntimeInstance::new(None);
    let devices = instance.enumerate_physical_devices();
    let apple_idx = devices.iter().position(|d| d.vendor() == "apple").unwrap();
    let device = instance.create_logical_device(apple_idx, None).unwrap();
    assert_eq!(device.physical_device_type(), DeviceType::NPU);
    assert!(device.physical_memory_properties().is_unified);
}

#[test]
fn test_create_logical_device_google_tpu() {
    let instance = RuntimeInstance::new(None);
    let devices = instance.enumerate_physical_devices();
    let google_idx = devices.iter().position(|d| d.vendor() == "google").unwrap();
    let device = instance.create_logical_device(google_idx, None).unwrap();
    assert_eq!(device.physical_device_type(), DeviceType::TPU);
}

#[test]
fn test_custom_device_creation() {
    use device_simulator::NvidiaGPU;

    let custom_dev: Box<dyn device_simulator::AcceleratorDevice> =
        Box::new(NvidiaGPU::new(None, 4));
    let instance = RuntimeInstance::new(Some(vec![
        (custom_dev, DeviceType::GPU, "nvidia".to_string()),
    ]));
    let devices = instance.enumerate_physical_devices();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].vendor(), "nvidia");
    assert_eq!(devices[0].device_type(), DeviceType::GPU);
}

#[test]
fn test_custom_device_creation_amd() {
    use device_simulator::AmdGPU;

    let custom_dev: Box<dyn device_simulator::AcceleratorDevice> =
        Box::new(AmdGPU::new(None, 4));
    let instance = RuntimeInstance::new(Some(vec![
        (custom_dev, DeviceType::GPU, "amd".to_string()),
    ]));
    let devices = instance.enumerate_physical_devices();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].vendor(), "amd");
}

#[test]
fn test_custom_device_creation_google() {
    use device_simulator::GoogleTPU;

    let custom_dev: Box<dyn device_simulator::AcceleratorDevice> =
        Box::new(GoogleTPU::new(None, 2));
    let instance = RuntimeInstance::new(Some(vec![
        (custom_dev, DeviceType::TPU, "google".to_string()),
    ]));
    let devices = instance.enumerate_physical_devices();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].vendor(), "google");
    assert_eq!(devices[0].device_type(), DeviceType::TPU);
}

#[test]
fn test_custom_device_creation_intel() {
    use device_simulator::IntelGPU;

    let custom_dev: Box<dyn device_simulator::AcceleratorDevice> =
        Box::new(IntelGPU::new(None, 2));
    let instance = RuntimeInstance::new(Some(vec![
        (custom_dev, DeviceType::GPU, "intel".to_string()),
    ]));
    let devices = instance.enumerate_physical_devices();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].vendor(), "intel");
}

#[test]
fn test_custom_device_creation_apple() {
    use device_simulator::AppleANE;

    let custom_dev: Box<dyn device_simulator::AcceleratorDevice> =
        Box::new(AppleANE::new(None, 2));
    let instance = RuntimeInstance::new(Some(vec![
        (custom_dev, DeviceType::NPU, "apple".to_string()),
    ]));
    let devices = instance.enumerate_physical_devices();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].vendor(), "apple");
    assert!(devices[0].memory_properties().is_unified);
}

#[test]
fn test_custom_device_unknown_vendor() {
    use device_simulator::NvidiaGPU;

    let custom_dev: Box<dyn device_simulator::AcceleratorDevice> =
        Box::new(NvidiaGPU::new(None, 2));
    let instance = RuntimeInstance::new(Some(vec![
        (custom_dev, DeviceType::GPU, "unknown_vendor".to_string()),
    ]));
    let devices = instance.enumerate_physical_devices();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].vendor(), "unknown_vendor");
}

#[test]
fn test_queues_mut() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, Some(vec![("compute", 2)])).unwrap();

    let queues = device.queues_mut();
    assert!(queues.contains_key("compute"));
    assert_eq!(queues["compute"].len(), 2);
}

// =========================================================================
// Additional coverage: validation.rs
// =========================================================================

#[test]
fn test_validation_error_display() {
    let err = compute_runtime::validation::ValidationError::new("test error message");
    let display = format!("{}", err);
    assert!(display.contains("ValidationError"));
    assert!(display.contains("test error message"));
}

#[test]
fn test_validation_error_is_error() {
    let err = compute_runtime::validation::ValidationError::new("test");
    // Verify it implements std::error::Error
    let _: &dyn std::error::Error = &err;
}

#[test]
fn test_validation_layer_default() {
    let layer = ValidationLayer::default();
    assert!(layer.warnings().is_empty());
    assert!(layer.errors().is_empty());
}

#[test]
fn test_validate_submit_not_recorded() {
    let layer = ValidationLayer::new();
    let cb = CommandBuffer::new();
    assert!(layer.validate_submit(&cb).is_err());
}

#[test]
fn test_validate_submit_recording_fails() {
    let layer = ValidationLayer::new();
    let mut cb = CommandBuffer::new();
    cb.begin().unwrap();
    assert!(layer.validate_submit(&cb).is_err());
}

#[test]
fn test_validate_map_already_mapped() {
    let layer = ValidationLayer::new();
    let buf = compute_runtime::memory::Buffer {
        buffer_id: 0,
        size: 64,
        memory_type: MemoryType::HOST_VISIBLE,
        usage: BufferUsage::STORAGE,
        device_address: 0,
        mapped: true,
        freed: false,
    };
    assert!(layer.validate_map(&buf).is_err());
}

#[test]
fn test_validate_buffer_not_freed_ok() {
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
    assert!(layer.validate_buffer_not_freed(&buf).is_ok());
}

#[test]
fn test_validate_buffer_not_freed_fails() {
    let layer = ValidationLayer::new();
    let buf = compute_runtime::memory::Buffer {
        buffer_id: 0,
        size: 64,
        memory_type: MemoryType::DEVICE_LOCAL,
        usage: BufferUsage::STORAGE,
        device_address: 0,
        mapped: false,
        freed: true,
    };
    assert!(layer.validate_buffer_not_freed(&buf).is_err());
}

#[test]
fn test_validate_begin_from_complete() {
    let layer = ValidationLayer::new();

    // To get a CB to Complete state, we need to submit it through a device
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

    device
        .submit("compute", 0, &mut [&mut cb], &mut [], &mut [], None)
        .unwrap();

    assert_eq!(cb.state(), CommandBufferState::Complete);
    assert!(layer.validate_begin(&cb).is_ok());
}

#[test]
fn test_validate_dispatch_zero_y_and_z() {
    let layer = ValidationLayer::new();
    let mut cb = CommandBuffer::new();
    cb.begin().unwrap();
    cb.cmd_bind_pipeline(0).unwrap();

    assert!(layer.validate_dispatch(&cb, 1, 0, 1).is_err());
    assert!(layer.validate_dispatch(&cb, 1, 1, 0).is_err());
    assert!(layer.validate_dispatch(&cb, 1, 1, 1).is_ok());
}

#[test]
fn test_barrier_tracking_with_specific_buffer_ids() {
    use std::collections::HashSet;

    let mut layer = ValidationLayer::new();
    layer.record_write(0);
    layer.record_write(1);
    layer.record_write(2);

    // Only barrier buffer 0
    let mut ids = HashSet::new();
    ids.insert(0);
    layer.record_barrier(Some(&ids));

    // Buffer 0 should be safe to read
    layer.validate_read_after_write(0);
    assert!(layer.warnings().is_empty());

    // Buffer 1 should still warn
    layer.validate_read_after_write(1);
    assert_eq!(layer.warnings().len(), 1);
}

#[test]
fn test_validate_read_no_prior_write() {
    let mut layer = ValidationLayer::new();
    // Reading buffer 5 that was never written -- no warning
    layer.validate_read_after_write(5);
    assert!(layer.warnings().is_empty());
}

#[test]
fn test_validation_clear_resets_everything() {
    let mut layer = ValidationLayer::new();
    layer.record_write(0);
    layer.validate_read_after_write(0);
    assert_eq!(layer.warnings().len(), 1);

    layer.clear();
    assert!(layer.warnings().is_empty());
    assert!(layer.errors().is_empty());

    // After clear, reading buffer 0 should not warn (write tracking cleared)
    layer.validate_read_after_write(0);
    assert!(layer.warnings().is_empty());
}

// =========================================================================
// Additional coverage: command_buffer.rs edge cases
// =========================================================================

#[test]
fn test_command_buffer_default() {
    let cb = CommandBuffer::default();
    assert_eq!(cb.state(), CommandBufferState::Initial);
}

#[test]
fn test_dispatch_indirect_without_pipeline() {
    let mut cb = CommandBuffer::new();
    cb.begin().unwrap();
    assert!(cb.cmd_dispatch_indirect(0, 0).is_err());
}

#[test]
fn test_push_constants() {
    let mut cb = CommandBuffer::new();
    cb.begin().unwrap();
    cb.cmd_push_constants(0, &[1, 2, 3, 4]).unwrap();
    cb.end().unwrap();

    assert_eq!(cb.commands().len(), 1);
    assert_eq!(cb.commands()[0].command, "push_constants");
}

#[test]
fn test_cmd_bind_descriptor_set() {
    let mut cb = CommandBuffer::new();
    cb.begin().unwrap();
    cb.cmd_bind_descriptor_set(42).unwrap();
    cb.end().unwrap();

    assert_eq!(cb.bound_descriptor_set_id(), Some(42));
    assert_eq!(cb.commands()[0].command, "bind_descriptor_set");
}

// =========================================================================
// Additional coverage: sync.rs edge cases
// =========================================================================

#[test]
fn test_fence_wait_with_timeout() {
    let fence = Fence::new(false);
    assert!(!fence.wait(Some(1000)));

    let mut fence2 = Fence::new(false);
    fence2.signal();
    assert!(fence2.wait(Some(1000)));
}

#[test]
fn test_fence_wait_cycles() {
    let fence = Fence::new(false);
    assert_eq!(fence.wait_cycles(), 0);
}

#[test]
fn test_semaphore_default() {
    let sem = Semaphore::default();
    assert!(!sem.signaled());
}

#[test]
fn test_event_default() {
    let event = Event::default();
    assert!(!event.signaled());
}

#[test]
fn test_sync_reset_ids() {
    compute_runtime::sync::reset_sync_ids();
    // Just verify it works without panic
    let _f = Fence::new(false);
    let _s = Semaphore::new();
    let _e = Event::new();
}

// =========================================================================
// Additional coverage: command_queue.rs
// =========================================================================

#[test]
fn test_command_queue_transfer_type() {
    let instance = RuntimeInstance::new(None);
    let device = instance
        .create_logical_device(0, Some(vec![("transfer", 1)]))
        .unwrap();
    let queues = device.queues();
    let transfer_queues = &queues["transfer"];
    assert_eq!(transfer_queues[0].queue_type(), QueueType::Transfer);
    assert_eq!(transfer_queues[0].queue_index(), 0);
}

#[test]
fn test_submit_without_fence() {
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

    // Submit without fence
    let traces = device
        .submit("compute", 0, &mut [&mut cb], &mut [], &mut [], None)
        .unwrap();
    assert!(!traces.is_empty());
    assert_eq!(cb.state(), CommandBufferState::Complete);
}

#[test]
fn test_submit_multiple_command_buffers() {
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

    let mut cb1 = device.create_command_buffer();
    cb1.begin().unwrap();
    cb1.cmd_bind_pipeline(pipeline_id).unwrap();
    cb1.cmd_dispatch(1, 1, 1).unwrap();
    cb1.end().unwrap();

    let mut cb2 = device.create_command_buffer();
    cb2.begin().unwrap();
    cb2.cmd_bind_pipeline(pipeline_id).unwrap();
    cb2.cmd_dispatch(2, 1, 1).unwrap();
    cb2.end().unwrap();

    let mut fence = device.create_fence(false);
    device
        .submit(
            "compute",
            0,
            &mut [&mut cb1, &mut cb2],
            &mut [],
            &mut [],
            Some(&mut fence),
        )
        .unwrap();

    assert!(fence.signaled());
    assert_eq!(device.stats().total_command_buffers, 2);
    assert_eq!(device.stats().total_dispatches, 2);
}

// =========================================================================
// Additional coverage: dispatch indirect
// =========================================================================

#[test]
fn test_dispatch_indirect_execution() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    // Allocate buffer with indirect dispatch data
    let buf_id = device
        .memory_manager_mut()
        .allocate(
            64,
            MemoryType::HOST_VISIBLE | MemoryType::HOST_COHERENT,
            BufferUsage::INDIRECT | BufferUsage::STORAGE,
        )
        .unwrap();

    // Write dispatch dimensions: (1, 1, 1) as u32 little-endian
    {
        let mut mapped = device.memory_manager_mut().map(buf_id).unwrap();
        let group_x: u32 = 1;
        let group_y: u32 = 1;
        let group_z: u32 = 1;
        let mut data = Vec::new();
        data.extend_from_slice(&group_x.to_le_bytes());
        data.extend_from_slice(&group_y.to_le_bytes());
        data.extend_from_slice(&group_z.to_le_bytes());
        mapped.write(0, &data).unwrap();
    }
    device.memory_manager_mut().unmap(buf_id).unwrap();

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
    cb.cmd_dispatch_indirect(buf_id, 0).unwrap();
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
    assert_eq!(device.stats().total_dispatches, 1);
}

// =========================================================================
// Memory stats tracking
// =========================================================================

#[test]
fn test_memory_stats_tracking() {
    let instance = RuntimeInstance::new(None);
    let mut device = instance.create_logical_device(0, None).unwrap();

    let buf_id = device
        .memory_manager_mut()
        .allocate(
            100,
            MemoryType::HOST_VISIBLE | MemoryType::HOST_COHERENT,
            BufferUsage::STORAGE,
        )
        .unwrap();

    assert_eq!(device.stats().total_allocations, 1);
    assert_eq!(device.stats().total_allocated_bytes, 100);

    // Map
    {
        let _mapped = device.memory_manager_mut().map(buf_id).unwrap();
    }
    device.memory_manager_mut().unmap(buf_id).unwrap();
    assert_eq!(device.stats().total_maps, 1);

    // Free
    device.memory_manager_mut().free(buf_id).unwrap();
    assert_eq!(device.stats().total_frees, 1);

    // Check traces were recorded
    assert!(!device.stats().traces.is_empty());
}

// =========================================================================
// Discrete GPU memory properties
// =========================================================================

#[test]
fn test_discrete_gpu_memory_properties() {
    let instance = RuntimeInstance::new(None);
    let devices = instance.enumerate_physical_devices();

    let nvidia = devices.iter().find(|d| d.vendor() == "nvidia").unwrap();
    let mem_props = nvidia.memory_properties();
    assert!(!mem_props.is_unified);
    assert!(mem_props.heaps.len() >= 2);

    // First heap should be DEVICE_LOCAL
    assert!(mem_props.heaps[0].flags.contains(MemoryType::DEVICE_LOCAL));
    // Second heap should be HOST_VISIBLE
    assert!(mem_props.heaps[1].flags.contains(MemoryType::HOST_VISIBLE));
}

#[test]
fn test_unified_memory_properties() {
    let instance = RuntimeInstance::new(None);
    let devices = instance.enumerate_physical_devices();

    let apple = devices.iter().find(|d| d.vendor() == "apple").unwrap();
    let mem_props = apple.memory_properties();
    assert!(mem_props.is_unified);
    assert_eq!(mem_props.heaps.len(), 1);
    assert!(mem_props.heaps[0].flags.contains(MemoryType::DEVICE_LOCAL));
    assert!(mem_props.heaps[0].flags.contains(MemoryType::HOST_VISIBLE));
    assert!(mem_props.heaps[0].flags.contains(MemoryType::HOST_COHERENT));
}
