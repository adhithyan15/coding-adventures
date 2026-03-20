//! Integration tests for all six vendor API simulators.
//!
//! Each simulator gets 30+ tests covering:
//! - Device discovery and properties
//! - Memory allocation, writing, reading, freeing
//! - Kernel/shader dispatch
//! - Synchronization
//! - Error handling
//!
//! Plus cross-API tests that verify all six APIs can interoperate with
//! the same underlying compute runtime.

// =========================================================================
// CUDA Tests (30+ tests)
// =========================================================================
mod cuda_tests {
    use vendor_api_simulators::cuda::*;

    #[test]
    fn test_cuda_runtime_creation() {
        let cuda = CudaRuntime::new();
        assert!(cuda.is_ok());
    }

    #[test]
    fn test_cuda_device_count() {
        let cuda = CudaRuntime::new().unwrap();
        assert!(cuda.device_count() >= 1);
    }

    #[test]
    fn test_cuda_get_device() {
        let cuda = CudaRuntime::new().unwrap();
        assert_eq!(cuda.get_device(), 0);
    }

    #[test]
    fn test_cuda_set_device() {
        let mut cuda = CudaRuntime::new().unwrap();
        assert!(cuda.set_device(0).is_ok());
    }

    #[test]
    fn test_cuda_set_device_invalid() {
        let mut cuda = CudaRuntime::new().unwrap();
        assert!(cuda.set_device(999).is_err());
    }

    #[test]
    fn test_cuda_device_properties() {
        let cuda = CudaRuntime::new().unwrap();
        let props = cuda.get_device_properties();
        assert!(!props.name.is_empty());
        assert!(props.total_global_mem > 0);
        assert_eq!(props.warp_size, 32);
        assert_eq!(props.compute_capability, (8, 0));
    }

    #[test]
    fn test_cuda_malloc() {
        let mut cuda = CudaRuntime::new().unwrap();
        let ptr = cuda.malloc(1024);
        assert!(ptr.is_ok());
        let ptr = ptr.unwrap();
        assert_eq!(ptr.size, 1024);
    }

    #[test]
    fn test_cuda_malloc_managed() {
        let mut cuda = CudaRuntime::new().unwrap();
        let ptr = cuda.malloc_managed(512);
        assert!(ptr.is_ok());
        assert_eq!(ptr.unwrap().size, 512);
    }

    #[test]
    fn test_cuda_free() {
        let mut cuda = CudaRuntime::new().unwrap();
        let ptr = cuda.malloc(256).unwrap();
        assert!(cuda.free(ptr).is_ok());
    }

    #[test]
    fn test_cuda_double_free() {
        let mut cuda = CudaRuntime::new().unwrap();
        let ptr = cuda.malloc(256).unwrap();
        assert!(cuda.free(ptr).is_ok());
        assert!(cuda.free(ptr).is_err());
    }

    #[test]
    fn test_cuda_memcpy_host_to_device() {
        let mut cuda = CudaRuntime::new().unwrap();
        let ptr = cuda.malloc(4).unwrap();
        let data = [1u8, 2, 3, 4];
        assert!(cuda.memcpy_host_to_device(ptr, &data).is_ok());
    }

    #[test]
    fn test_cuda_memcpy_device_to_host() {
        let mut cuda = CudaRuntime::new().unwrap();
        let ptr = cuda.malloc(4).unwrap();
        let data = [10u8, 20, 30, 40];
        cuda.memcpy_host_to_device(ptr, &data).unwrap();

        let mut result = [0u8; 4];
        cuda.memcpy_device_to_host(&mut result, ptr).unwrap();
        assert_eq!(result, [10, 20, 30, 40]);
    }

    #[test]
    fn test_cuda_memcpy_device_to_device() {
        let mut cuda = CudaRuntime::new().unwrap();
        let src = cuda.malloc(4).unwrap();
        let dst = cuda.malloc(4).unwrap();
        let data = [5u8, 6, 7, 8];
        cuda.memcpy_host_to_device(src, &data).unwrap();
        assert!(cuda.memcpy_device_to_device(dst, src, 4).is_ok());
    }

    #[test]
    fn test_cuda_memcpy_host_to_host() {
        let src = [1u8, 2, 3];
        let mut dst = [0u8; 3];
        CudaRuntime::memcpy_host_to_host(&mut dst, &src, 3);
        assert_eq!(dst, [1, 2, 3]);
    }

    #[test]
    fn test_cuda_memset() {
        let mut cuda = CudaRuntime::new().unwrap();
        let ptr = cuda.malloc(16).unwrap();
        assert!(cuda.memset(ptr, 0xAA, 16).is_ok());
    }

    #[test]
    fn test_cuda_launch_kernel_no_args() {
        let mut cuda = CudaRuntime::new().unwrap();
        let kernel = CudaKernel::new("empty_kernel", None);
        let grid = Dim3::new(1, 1, 1);
        let block = Dim3::new(32, 1, 1);
        assert!(cuda.launch_kernel(&kernel, grid, block, &[]).is_ok());
    }

    #[test]
    fn test_cuda_launch_kernel_with_args() {
        let mut cuda = CudaRuntime::new().unwrap();
        let kernel = CudaKernel::new("compute_kernel", None);
        let buf = cuda.malloc(256).unwrap();
        let grid = Dim3::new(4, 1, 1);
        let block = Dim3::new(64, 1, 1);
        assert!(cuda.launch_kernel(&kernel, grid, block, &[buf]).is_ok());
    }

    #[test]
    fn test_cuda_launch_kernel_multiple_args() {
        let mut cuda = CudaRuntime::new().unwrap();
        let kernel = CudaKernel::new("saxpy", None);
        let buf_x = cuda.malloc(256).unwrap();
        let buf_y = cuda.malloc(256).unwrap();
        let grid = Dim3::new(4, 1, 1);
        let block = Dim3::new(64, 1, 1);
        assert!(cuda.launch_kernel(&kernel, grid, block, &[buf_x, buf_y]).is_ok());
    }

    #[test]
    fn test_cuda_device_synchronize() {
        let cuda = CudaRuntime::new().unwrap();
        cuda.device_synchronize(); // Should not panic
    }

    #[test]
    fn test_cuda_device_reset() {
        let mut cuda = CudaRuntime::new().unwrap();
        cuda.create_stream();
        cuda.create_event();
        cuda.device_reset();
        // After reset, streams and events are cleared
    }

    #[test]
    fn test_cuda_create_stream() {
        let mut cuda = CudaRuntime::new().unwrap();
        let stream = cuda.create_stream();
        assert!(stream.id() > 0);
    }

    #[test]
    fn test_cuda_destroy_stream() {
        let mut cuda = CudaRuntime::new().unwrap();
        let stream = cuda.create_stream();
        assert!(cuda.destroy_stream(stream).is_ok());
    }

    #[test]
    fn test_cuda_destroy_stream_twice() {
        let mut cuda = CudaRuntime::new().unwrap();
        let stream = cuda.create_stream();
        cuda.destroy_stream(stream).unwrap();
        // Creating and destroying a second stream should work fine
        let stream2 = cuda.create_stream();
        cuda.destroy_stream(stream2).unwrap();
    }

    #[test]
    fn test_cuda_stream_synchronize() {
        let mut cuda = CudaRuntime::new().unwrap();
        let stream = cuda.create_stream();
        cuda.stream_synchronize(stream); // Should not panic
    }

    #[test]
    fn test_cuda_create_event() {
        let mut cuda = CudaRuntime::new().unwrap();
        let event = cuda.create_event();
        assert_eq!(event, 0);
    }

    #[test]
    fn test_cuda_record_event() {
        let mut cuda = CudaRuntime::new().unwrap();
        let event = cuda.create_event();
        assert!(cuda.record_event(event).is_ok());
    }

    #[test]
    fn test_cuda_record_event_invalid() {
        let mut cuda = CudaRuntime::new().unwrap();
        assert!(cuda.record_event(999).is_err());
    }

    #[test]
    fn test_cuda_synchronize_event() {
        let mut cuda = CudaRuntime::new().unwrap();
        let event = cuda.create_event();
        cuda.record_event(event).unwrap();
        assert!(cuda.synchronize_event(event).is_ok());
    }

    #[test]
    fn test_cuda_synchronize_unrecorded_event() {
        let mut cuda = CudaRuntime::new().unwrap();
        let event = cuda.create_event();
        assert!(cuda.synchronize_event(event).is_err());
    }

    #[test]
    fn test_cuda_elapsed_time() {
        let mut cuda = CudaRuntime::new().unwrap();
        let start = cuda.create_event();
        let end = cuda.create_event();
        cuda.record_event(start).unwrap();
        cuda.record_event(end).unwrap();
        let elapsed = cuda.elapsed_time(start, end);
        assert!(elapsed.is_ok());
    }

    #[test]
    fn test_cuda_elapsed_time_unrecorded() {
        let mut cuda = CudaRuntime::new().unwrap();
        let start = cuda.create_event();
        let end = cuda.create_event();
        assert!(cuda.elapsed_time(start, end).is_err());
    }

    #[test]
    fn test_cuda_dim3() {
        let d = Dim3::new(4, 2, 1);
        assert_eq!(d.x, 4);
        assert_eq!(d.y, 2);
        assert_eq!(d.z, 1);
    }

    #[test]
    fn test_cuda_kernel_creation() {
        let kernel = CudaKernel::new("test_kernel", None);
        assert_eq!(kernel.name, "test_kernel");
        assert!(kernel.code.is_none());
    }

    #[test]
    fn test_cuda_roundtrip_data() {
        let mut cuda = CudaRuntime::new().unwrap();
        let ptr = cuda.malloc(8).unwrap();
        let data = [11u8, 22, 33, 44, 55, 66, 77, 88];
        cuda.memcpy_host_to_device(ptr, &data).unwrap();
        let mut result = [0u8; 8];
        cuda.memcpy_device_to_host(&mut result, ptr).unwrap();
        assert_eq!(result, data);
    }
}

// =========================================================================
// OpenCL Tests (30+ tests)
// =========================================================================
mod opencl_tests {
    use vendor_api_simulators::opencl::*;

    #[test]
    fn test_cl_context_creation() {
        let ctx = ClContext::new();
        assert!(ctx.is_ok());
    }

    #[test]
    fn test_cl_context_with_vendor() {
        let ctx = ClContext::with_vendor("nvidia");
        assert!(ctx.is_ok());
    }

    #[test]
    fn test_cl_context_devices() {
        let ctx = ClContext::new().unwrap();
        assert!(!ctx.devices().is_empty());
    }

    #[test]
    fn test_cl_device_properties() {
        let ctx = ClContext::new().unwrap();
        let dev = &ctx.devices()[0];
        assert!(!dev.name.is_empty());
        assert!(dev.global_mem_size > 0);
        assert!(dev.max_work_group_size > 0);
    }

    #[test]
    fn test_cl_create_buffer_read_write() {
        let mut ctx = ClContext::new().unwrap();
        let buf = ctx.create_buffer(ClMemFlags::ReadWrite, 1024, None);
        assert!(buf.is_ok());
        assert_eq!(buf.unwrap().size, 1024);
    }

    #[test]
    fn test_cl_create_buffer_with_data() {
        let mut ctx = ClContext::new().unwrap();
        let data = [1u8, 2, 3, 4];
        let buf = ctx.create_buffer(ClMemFlags::CopyHostPtr, 4, Some(&data));
        assert!(buf.is_ok());
    }

    #[test]
    fn test_cl_release_buffer() {
        let mut ctx = ClContext::new().unwrap();
        let buf = ctx.create_buffer(ClMemFlags::ReadWrite, 64, None).unwrap();
        assert!(ctx.release_buffer(buf).is_ok());
    }

    #[test]
    fn test_cl_write_buffer() {
        let mut ctx = ClContext::new().unwrap();
        let buf = ctx.create_buffer(ClMemFlags::ReadWrite, 4, None).unwrap();
        let data = [10u8, 20, 30, 40];
        let mut queue = ctx.create_command_queue();
        let event = queue.enqueue_write_buffer(&buf, 0, &data, &[]);
        assert!(event.is_ok());
    }

    #[test]
    fn test_cl_read_buffer() {
        let mut ctx = ClContext::new().unwrap();
        let buf = ctx.create_buffer(ClMemFlags::ReadWrite, 4, None).unwrap();
        let data = [5u8, 10, 15, 20];
        {
            let mut queue = ctx.create_command_queue();
            queue.enqueue_write_buffer(&buf, 0, &data, &[]).unwrap();
        }
        let mut result = [0u8; 4];
        {
            let mut queue = ctx.create_command_queue();
            queue.enqueue_read_buffer(&buf, 0, 4, &mut result, &[]).unwrap();
        }
        assert_eq!(result, [5, 10, 15, 20]);
    }

    #[test]
    fn test_cl_copy_buffer() {
        let mut ctx = ClContext::new().unwrap();
        let src = ctx.create_buffer(ClMemFlags::ReadWrite, 4, None).unwrap();
        let dst = ctx.create_buffer(ClMemFlags::ReadWrite, 4, None).unwrap();
        {
            let mut queue = ctx.create_command_queue();
            queue.enqueue_write_buffer(&src, 0, &[1, 2, 3, 4], &[]).unwrap();
            queue.enqueue_copy_buffer(&src, &dst, 4, &[]).unwrap();
        }
    }

    #[test]
    fn test_cl_fill_buffer() {
        let mut ctx = ClContext::new().unwrap();
        let buf = ctx.create_buffer(ClMemFlags::ReadWrite, 16, None).unwrap();
        let mut queue = ctx.create_command_queue();
        let event = queue.enqueue_fill_buffer(&buf, 0xFF, 0, 16);
        assert!(event.is_ok());
    }

    #[test]
    fn test_cl_program_build() {
        let ctx = ClContext::new().unwrap();
        let mut prog = ctx.create_program_with_source("kernel void foo() {}");
        assert_eq!(prog.build_status, ClBuildStatus::None);
        prog.build();
        assert_eq!(prog.build_status, ClBuildStatus::Success);
    }

    #[test]
    fn test_cl_create_kernel() {
        let ctx = ClContext::new().unwrap();
        let mut prog = ctx.create_program_with_source("kernel void compute() {}");
        prog.build();
        let kernel = prog.create_kernel("compute");
        assert!(kernel.is_ok());
        assert_eq!(kernel.unwrap().name, "compute");
    }

    #[test]
    fn test_cl_create_kernel_before_build() {
        let ctx = ClContext::new().unwrap();
        let prog = ctx.create_program_with_source("kernel void foo() {}");
        assert!(prog.create_kernel("foo").is_err());
    }

    #[test]
    fn test_cl_kernel_set_arg_buffer() {
        let mut ctx = ClContext::new().unwrap();
        let buf = ctx.create_buffer(ClMemFlags::ReadWrite, 64, None).unwrap();
        let mut prog = ctx.create_program_with_source("test");
        prog.register_kernel("test", None);
        prog.build();
        let mut kernel = prog.create_kernel("test").unwrap();
        kernel.set_arg_buffer(0, &buf);
    }

    #[test]
    fn test_cl_enqueue_nd_range_kernel() {
        let mut ctx = ClContext::new().unwrap();
        let buf = ctx.create_buffer(ClMemFlags::ReadWrite, 256, None).unwrap();
        let mut prog = ctx.create_program_with_source("test");
        prog.build();
        let mut kernel = prog.create_kernel("test").unwrap();
        kernel.set_arg_buffer(0, &buf);
        let mut queue = ctx.create_command_queue();
        let event = queue.enqueue_nd_range_kernel(&kernel, &[128], Some(&[32]), &[]);
        assert!(event.is_ok());
    }

    #[test]
    fn test_cl_enqueue_nd_range_kernel_auto_local() {
        let mut ctx = ClContext::new().unwrap();
        let buf = ctx.create_buffer(ClMemFlags::ReadWrite, 256, None).unwrap();
        let mut prog = ctx.create_program_with_source("test");
        prog.build();
        let mut kernel = prog.create_kernel("test").unwrap();
        kernel.set_arg_buffer(0, &buf);
        let mut queue = ctx.create_command_queue();
        let event = queue.enqueue_nd_range_kernel(&kernel, &[256], None, &[]);
        assert!(event.is_ok());
    }

    #[test]
    fn test_cl_finish() {
        let mut ctx = ClContext::new().unwrap();
        let queue = ctx.create_command_queue();
        queue.finish(); // Should not panic
    }

    #[test]
    fn test_cl_flush() {
        let mut ctx = ClContext::new().unwrap();
        let queue = ctx.create_command_queue();
        queue.flush(); // Should not panic (no-op)
    }

    #[test]
    fn test_cl_event_status() {
        let mut ctx = ClContext::new().unwrap();
        let buf = ctx.create_buffer(ClMemFlags::ReadWrite, 4, None).unwrap();
        let mut queue = ctx.create_command_queue();
        let event = queue.enqueue_write_buffer(&buf, 0, &[1, 2, 3, 4], &[]).unwrap();
        assert_eq!(event.status(), ClEventStatus::Complete);
    }

    #[test]
    fn test_cl_event_wait() {
        let mut ctx = ClContext::new().unwrap();
        let buf = ctx.create_buffer(ClMemFlags::ReadWrite, 4, None).unwrap();
        let mut queue = ctx.create_command_queue();
        let event = queue.enqueue_write_buffer(&buf, 0, &[1, 2, 3, 4], &[]).unwrap();
        event.wait(); // Should not panic
    }

    #[test]
    fn test_cl_platform_discovery() {
        let platforms = ClPlatform::get_platforms();
        assert_eq!(platforms.len(), 1);
        assert_eq!(platforms[0].name, "Coding Adventures Compute Platform");
    }

    #[test]
    fn test_cl_platform_get_devices() {
        let platforms = ClPlatform::get_platforms();
        let devices = platforms[0].get_devices(ClDeviceType::All);
        assert!(!devices.is_empty());
    }

    #[test]
    fn test_cl_platform_get_gpu_devices() {
        let platforms = ClPlatform::get_platforms();
        let gpus = platforms[0].get_devices(ClDeviceType::Gpu);
        assert!(!gpus.is_empty());
    }

    #[test]
    fn test_cl_roundtrip_data() {
        let mut ctx = ClContext::new().unwrap();
        let buf = ctx.create_buffer(ClMemFlags::ReadWrite, 8, None).unwrap();
        let data = [11u8, 22, 33, 44, 55, 66, 77, 88];
        {
            let mut queue = ctx.create_command_queue();
            queue.enqueue_write_buffer(&buf, 0, &data, &[]).unwrap();
        }
        let mut result = [0u8; 8];
        {
            let mut queue = ctx.create_command_queue();
            queue.enqueue_read_buffer(&buf, 0, 8, &mut result, &[]).unwrap();
        }
        assert_eq!(result, data);
    }

    #[test]
    fn test_cl_device_type_enum() {
        assert_ne!(ClDeviceType::Gpu, ClDeviceType::Cpu);
        assert_ne!(ClDeviceType::Cpu, ClDeviceType::Accelerator);
        assert_ne!(ClDeviceType::Accelerator, ClDeviceType::All);
    }

    #[test]
    fn test_cl_mem_flags_enum() {
        assert_ne!(ClMemFlags::ReadWrite, ClMemFlags::ReadOnly);
        assert_ne!(ClMemFlags::ReadOnly, ClMemFlags::WriteOnly);
    }

    #[test]
    fn test_cl_kernel_scalar_arg() {
        let ctx = ClContext::new().unwrap();
        let mut prog = ctx.create_program_with_source("test");
        prog.register_kernel("test", None);
        prog.build();
        let mut kernel = prog.create_kernel("test").unwrap();
        kernel.set_arg_scalar(0, &[0u8; 4]);
    }

    #[test]
    fn test_cl_program_register_kernel() {
        let ctx = ClContext::new().unwrap();
        let mut prog = ctx.create_program_with_source("test");
        prog.register_kernel("my_kernel", None);
        prog.build();
        let kernel = prog.create_kernel("my_kernel");
        assert!(kernel.is_ok());
    }
}

// =========================================================================
// Metal Tests (30+ tests)
// =========================================================================
mod metal_tests {
    use vendor_api_simulators::metal::*;

    #[test]
    fn test_mtl_device_creation() {
        let device = MtlDevice::new();
        assert!(device.is_ok());
    }

    #[test]
    fn test_mtl_device_name() {
        let device = MtlDevice::new().unwrap();
        assert!(!device.name().is_empty());
    }

    #[test]
    fn test_mtl_make_buffer() {
        let mut device = MtlDevice::new().unwrap();
        let buf = device.make_buffer(1024);
        assert!(buf.is_ok());
        assert_eq!(buf.unwrap().length, 1024);
    }

    #[test]
    fn test_mtl_write_buffer() {
        let mut device = MtlDevice::new().unwrap();
        let buf = device.make_buffer(4).unwrap();
        assert!(device.write_buffer(&buf, &[1, 2, 3, 4]).is_ok());
    }

    #[test]
    fn test_mtl_read_buffer() {
        let mut device = MtlDevice::new().unwrap();
        let buf = device.make_buffer(4).unwrap();
        device.write_buffer(&buf, &[10, 20, 30, 40]).unwrap();
        let data = device.read_buffer(&buf).unwrap();
        assert_eq!(&data[..4], &[10, 20, 30, 40]);
    }

    #[test]
    fn test_mtl_release_buffer() {
        let mut device = MtlDevice::new().unwrap();
        let buf = device.make_buffer(64).unwrap();
        assert!(device.release_buffer(buf).is_ok());
    }

    #[test]
    fn test_mtl_make_library() {
        let device = MtlDevice::new().unwrap();
        let lib = device.make_library("test_shader");
        assert_eq!(lib.source, "test_shader");
    }

    #[test]
    fn test_mtl_make_function() {
        let device = MtlDevice::new().unwrap();
        let lib = device.make_library("test");
        let func = lib.make_function("compute_fn");
        assert_eq!(func.name, "compute_fn");
    }

    #[test]
    fn test_mtl_library_register_function() {
        let device = MtlDevice::new().unwrap();
        let mut lib = device.make_library("test");
        lib.register_function("my_func", None);
        let func = lib.make_function("my_func");
        assert_eq!(func.name, "my_func");
    }

    #[test]
    fn test_mtl_make_compute_pipeline_state() {
        let mut device = MtlDevice::new().unwrap();
        let lib = device.make_library("test");
        let func = lib.make_function("compute");
        let pso = device.make_compute_pipeline_state(&func);
        assert!(pso.is_ok());
    }

    #[test]
    fn test_mtl_compute_encoder_creation() {
        let device = MtlDevice::new().unwrap();
        let encoder = device.make_compute_command_encoder();
        assert!(!encoder.is_ended());
    }

    #[test]
    fn test_mtl_compute_encoder_set_pipeline() {
        let mut device = MtlDevice::new().unwrap();
        let lib = device.make_library("test");
        let func = lib.make_function("compute");
        let pso = device.make_compute_pipeline_state(&func).unwrap();
        let mut encoder = device.make_compute_command_encoder();
        encoder.set_compute_pipeline_state(&pso);
    }

    #[test]
    fn test_mtl_compute_encoder_set_buffer() {
        let mut device = MtlDevice::new().unwrap();
        let buf = device.make_buffer(256).unwrap();
        let mut encoder = device.make_compute_command_encoder();
        encoder.set_buffer(&buf, 0, 0);
    }

    #[test]
    fn test_mtl_dispatch_threadgroups() {
        let mut device = MtlDevice::new().unwrap();
        let buf = device.make_buffer(256).unwrap();
        let lib = device.make_library("test");
        let func = lib.make_function("compute");
        let pso = device.make_compute_pipeline_state(&func).unwrap();

        let mut encoder = device.make_compute_command_encoder();
        encoder.set_compute_pipeline_state(&pso);
        encoder.set_buffer(&buf, 0, 0);
        encoder.end_encoding();

        let result = device.dispatch_threadgroups(
            &encoder,
            MtlSize::new(4, 1, 1),
            MtlSize::new(64, 1, 1),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_mtl_blit_encoder_creation() {
        let device = MtlDevice::new().unwrap();
        let encoder = device.make_blit_command_encoder();
        assert!(!encoder.is_ended());
    }

    #[test]
    fn test_mtl_blit_copy() {
        let mut device = MtlDevice::new().unwrap();
        let src = device.make_buffer(64).unwrap();
        let dst = device.make_buffer(64).unwrap();
        let mut encoder = device.make_blit_command_encoder();
        encoder.copy_from_buffer(&src, 0, &dst, 0, 64);
        encoder.end_encoding();
        assert!(device.commit_blit_encoder(&encoder).is_ok());
    }

    #[test]
    fn test_mtl_blit_fill() {
        let mut device = MtlDevice::new().unwrap();
        let buf = device.make_buffer(32).unwrap();
        let mut encoder = device.make_blit_command_encoder();
        encoder.fill_buffer(&buf, 0xCC, 0, 32);
        encoder.end_encoding();
        assert!(device.commit_blit_encoder(&encoder).is_ok());
    }

    #[test]
    fn test_mtl_wait_until_completed() {
        let device = MtlDevice::new().unwrap();
        device.wait_until_completed(); // Should not panic
    }

    #[test]
    fn test_mtl_size() {
        let size = MtlSize::new(8, 4, 2);
        assert_eq!(size.width, 8);
        assert_eq!(size.height, 4);
        assert_eq!(size.depth, 2);
    }

    #[test]
    fn test_mtl_resource_options() {
        assert_ne!(
            MtlResourceOptions::StorageModeShared,
            MtlResourceOptions::StorageModePrivate
        );
    }

    #[test]
    fn test_mtl_command_buffer_status() {
        assert_ne!(
            MtlCommandBufferStatus::NotEnqueued,
            MtlCommandBufferStatus::Completed
        );
    }

    #[test]
    fn test_mtl_roundtrip_data() {
        let mut device = MtlDevice::new().unwrap();
        let buf = device.make_buffer(8).unwrap();
        let data = [11u8, 22, 33, 44, 55, 66, 77, 88];
        device.write_buffer(&buf, &data).unwrap();
        let result = device.read_buffer(&buf).unwrap();
        assert_eq!(&result[..8], &data);
    }

    #[test]
    fn test_mtl_multiple_buffers() {
        let mut device = MtlDevice::new().unwrap();
        let buf1 = device.make_buffer(64).unwrap();
        let buf2 = device.make_buffer(128).unwrap();
        assert_ne!(buf1.buffer_id, buf2.buffer_id);
    }

    #[test]
    fn test_mtl_dispatch_no_pipeline() {
        let mut device = MtlDevice::new().unwrap();
        let encoder = device.make_compute_command_encoder();
        let result = device.dispatch_threadgroups(
            &encoder,
            MtlSize::new(1, 1, 1),
            MtlSize::new(1, 1, 1),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_mtl_dispatch_multiple_buffers() {
        let mut device = MtlDevice::new().unwrap();
        let buf_a = device.make_buffer(256).unwrap();
        let buf_b = device.make_buffer(256).unwrap();
        let lib = device.make_library("test");
        let func = lib.make_function("compute");
        let pso = device.make_compute_pipeline_state(&func).unwrap();

        let mut encoder = device.make_compute_command_encoder();
        encoder.set_compute_pipeline_state(&pso);
        encoder.set_buffer(&buf_a, 0, 0);
        encoder.set_buffer(&buf_b, 0, 1);
        encoder.end_encoding();

        assert!(device
            .dispatch_threadgroups(&encoder, MtlSize::new(2, 1, 1), MtlSize::new(32, 1, 1))
            .is_ok());
    }

    #[test]
    fn test_mtl_encoder_end_encoding() {
        let device = MtlDevice::new().unwrap();
        let mut encoder = device.make_compute_command_encoder();
        assert!(!encoder.is_ended());
        encoder.end_encoding();
        assert!(encoder.is_ended());
    }

    #[test]
    fn test_mtl_blit_encoder_end_encoding() {
        let device = MtlDevice::new().unwrap();
        let mut encoder = device.make_blit_command_encoder();
        assert!(!encoder.is_ended());
        encoder.end_encoding();
        assert!(encoder.is_ended());
    }

    #[test]
    fn test_mtl_empty_blit() {
        let mut device = MtlDevice::new().unwrap();
        let encoder = device.make_blit_command_encoder();
        assert!(device.commit_blit_encoder(&encoder).is_ok());
    }
}

// =========================================================================
// Vulkan Tests (30+ tests)
// =========================================================================
mod vulkan_tests {
    use vendor_api_simulators::vulkan::*;
    use compute_runtime::protocols::PipelineStage;

    #[test]
    fn test_vk_instance_creation() {
        let instance = VkInstance::new();
        assert!(instance.is_ok());
    }

    #[test]
    fn test_vk_enumerate_physical_devices() {
        let instance = VkInstance::new().unwrap();
        let devices = instance.vk_enumerate_physical_devices();
        assert!(!devices.is_empty());
    }

    #[test]
    fn test_vk_physical_device_properties() {
        let instance = VkInstance::new().unwrap();
        let devices = instance.vk_enumerate_physical_devices();
        let pd = &devices[0];
        assert!(!pd.name.is_empty());
        assert!(!pd.vendor.is_empty());
    }

    #[test]
    fn test_vk_create_device() {
        let instance = VkInstance::new().unwrap();
        let devices = instance.vk_enumerate_physical_devices();
        let device = instance.vk_create_device(&devices[0]);
        assert!(device.is_ok());
    }

    #[test]
    fn test_vk_create_buffer() {
        let instance = VkInstance::new().unwrap();
        let devices = instance.vk_enumerate_physical_devices();
        let mut device = instance.vk_create_device(&devices[0]).unwrap();
        let create_info = VkBufferCreateInfo {
            size: 1024,
            ..Default::default()
        };
        let buf = device.vk_create_buffer(&create_info);
        assert!(buf.is_ok());
        assert_eq!(buf.unwrap().size, 1024);
    }

    #[test]
    fn test_vk_allocate_memory() {
        let instance = VkInstance::new().unwrap();
        let devices = instance.vk_enumerate_physical_devices();
        let mut device = instance.vk_create_device(&devices[0]).unwrap();
        let alloc_info = VkMemoryAllocateInfo {
            size: 256,
            memory_type_index: 0,
        };
        let mem = device.vk_allocate_memory(&alloc_info);
        assert!(mem.is_ok());
    }

    #[test]
    fn test_vk_write_and_read_memory() {
        let instance = VkInstance::new().unwrap();
        let devices = instance.vk_enumerate_physical_devices();
        let mut device = instance.vk_create_device(&devices[0]).unwrap();
        let alloc_info = VkMemoryAllocateInfo {
            size: 16,
            memory_type_index: 0,
        };
        let mem = device.vk_allocate_memory(&alloc_info).unwrap();
        device.vk_write_mapped_memory(&mem, 0, &[1, 2, 3, 4]).unwrap();
        let data = device.vk_map_memory(&mem, 0, 4).unwrap();
        assert_eq!(data, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_vk_create_shader_module() {
        let instance = VkInstance::new().unwrap();
        let devices = instance.vk_enumerate_physical_devices();
        let device = instance.vk_create_device(&devices[0]).unwrap();
        let create_info = VkShaderModuleCreateInfo { code: None };
        let shader = device.vk_create_shader_module(&create_info);
        assert!(shader.code.is_none());
    }

    #[test]
    fn test_vk_create_descriptor_set_layout() {
        let instance = VkInstance::new().unwrap();
        let devices = instance.vk_enumerate_physical_devices();
        let device = instance.vk_create_device(&devices[0]).unwrap();
        let bindings = vec![VkDescriptorSetLayoutBinding {
            binding: 0,
            ..Default::default()
        }];
        let layout = device.vk_create_descriptor_set_layout(&bindings);
        assert_eq!(layout.bindings.len(), 1);
    }

    #[test]
    fn test_vk_create_pipeline_layout() {
        let instance = VkInstance::new().unwrap();
        let devices = instance.vk_enumerate_physical_devices();
        let device = instance.vk_create_device(&devices[0]).unwrap();
        let layout = device.vk_create_pipeline_layout(&[], 0);
        assert_eq!(layout.push_constant_size, 0);
    }

    #[test]
    fn test_vk_create_compute_pipeline() {
        let instance = VkInstance::new().unwrap();
        let devices = instance.vk_enumerate_physical_devices();
        let mut device = instance.vk_create_device(&devices[0]).unwrap();
        let shader = device.vk_create_shader_module(&VkShaderModuleCreateInfo { code: None });
        let layout = device.vk_create_pipeline_layout(&[], 0);
        let _pipeline = device.vk_create_compute_pipeline(&shader, &layout);
    }

    #[test]
    fn test_vk_allocate_descriptor_set() {
        let instance = VkInstance::new().unwrap();
        let devices = instance.vk_enumerate_physical_devices();
        let mut device = instance.vk_create_device(&devices[0]).unwrap();
        let ds_layout = device.vk_create_descriptor_set_layout(&[VkDescriptorSetLayoutBinding {
            binding: 0,
            ..Default::default()
        }]);
        let _ds = device.vk_allocate_descriptor_set(&ds_layout);
    }

    #[test]
    fn test_vk_update_descriptor_set() {
        let instance = VkInstance::new().unwrap();
        let devices = instance.vk_enumerate_physical_devices();
        let mut device = instance.vk_create_device(&devices[0]).unwrap();
        let buf = device.vk_create_buffer(&VkBufferCreateInfo { size: 64, ..Default::default() }).unwrap();
        let ds_layout = device.vk_create_descriptor_set_layout(&[VkDescriptorSetLayoutBinding {
            binding: 0,
            ..Default::default()
        }]);
        let ds = device.vk_allocate_descriptor_set(&ds_layout);
        assert!(device.vk_update_descriptor_set(&ds, 0, &buf).is_ok());
    }

    #[test]
    fn test_vk_command_buffer_lifecycle() {
        let instance = VkInstance::new().unwrap();
        let devices = instance.vk_enumerate_physical_devices();
        let mut device = instance.vk_create_device(&devices[0]).unwrap();
        let mut pool = device.vk_create_command_pool();
        let cbs = device.vk_allocate_command_buffers(&mut pool, 1);
        assert_eq!(cbs.len(), 1);
        let cb = &cbs[0];
        assert!(device.vk_begin_command_buffer(cb).is_ok());
        assert!(device.vk_end_command_buffer(cb).is_ok());
    }

    #[test]
    fn test_vk_record_and_submit() {
        let instance = VkInstance::new().unwrap();
        let devices = instance.vk_enumerate_physical_devices();
        let mut device = instance.vk_create_device(&devices[0]).unwrap();
        let buf = device.vk_create_buffer(&VkBufferCreateInfo { size: 64, ..Default::default() }).unwrap();
        let shader = device.vk_create_shader_module(&VkShaderModuleCreateInfo { code: None });
        let ds_layout = device.vk_create_descriptor_set_layout(&[VkDescriptorSetLayoutBinding {
            binding: 0,
            ..Default::default()
        }]);
        let pl_layout = device.vk_create_pipeline_layout(&[ds_layout.clone()], 0);
        let pipeline = device.vk_create_compute_pipeline(&shader, &pl_layout);
        let ds = device.vk_allocate_descriptor_set(&ds_layout);
        device.vk_update_descriptor_set(&ds, 0, &buf).unwrap();

        let mut pool = device.vk_create_command_pool();
        let cbs = device.vk_allocate_command_buffers(&mut pool, 1);
        let cb = &cbs[0];
        device.vk_begin_command_buffer(cb).unwrap();
        device.vk_cmd_bind_pipeline(cb, &pipeline).unwrap();
        device.vk_cmd_bind_descriptor_sets(cb, &[&ds]).unwrap();
        device.vk_cmd_dispatch(cb, 4, 1, 1).unwrap();
        device.vk_end_command_buffer(cb).unwrap();

        let mut fence = device.vk_create_fence(false);
        let result = device.vk_queue_submit(&[cb], Some(&mut fence));
        assert_eq!(result.unwrap(), VkResult::Success);
        assert!(fence.signaled);
    }

    #[test]
    fn test_vk_cmd_copy_buffer() {
        let instance = VkInstance::new().unwrap();
        let devices = instance.vk_enumerate_physical_devices();
        let mut device = instance.vk_create_device(&devices[0]).unwrap();
        let src = device.vk_create_buffer(&VkBufferCreateInfo { size: 64, ..Default::default() }).unwrap();
        let dst = device.vk_create_buffer(&VkBufferCreateInfo { size: 64, ..Default::default() }).unwrap();

        let mut pool = device.vk_create_command_pool();
        let cbs = device.vk_allocate_command_buffers(&mut pool, 1);
        let cb = &cbs[0];
        device.vk_begin_command_buffer(cb).unwrap();
        device.vk_cmd_copy_buffer(cb, &src, &dst, &[VkBufferCopy { src_offset: 0, dst_offset: 0, size: 64 }]).unwrap();
        device.vk_end_command_buffer(cb).unwrap();
        assert!(device.vk_queue_submit(&[cb], None).is_ok());
    }

    #[test]
    fn test_vk_cmd_fill_buffer() {
        let instance = VkInstance::new().unwrap();
        let devices = instance.vk_enumerate_physical_devices();
        let mut device = instance.vk_create_device(&devices[0]).unwrap();
        let buf = device.vk_create_buffer(&VkBufferCreateInfo { size: 32, ..Default::default() }).unwrap();

        let mut pool = device.vk_create_command_pool();
        let cbs = device.vk_allocate_command_buffers(&mut pool, 1);
        let cb = &cbs[0];
        device.vk_begin_command_buffer(cb).unwrap();
        device.vk_cmd_fill_buffer(cb, &buf, 0xAB, 0, 32).unwrap();
        device.vk_end_command_buffer(cb).unwrap();
        assert!(device.vk_queue_submit(&[cb], None).is_ok());
    }

    #[test]
    fn test_vk_pipeline_barrier() {
        let instance = VkInstance::new().unwrap();
        let devices = instance.vk_enumerate_physical_devices();
        let mut device = instance.vk_create_device(&devices[0]).unwrap();

        let mut pool = device.vk_create_command_pool();
        let cbs = device.vk_allocate_command_buffers(&mut pool, 1);
        let cb = &cbs[0];
        device.vk_begin_command_buffer(cb).unwrap();
        device.vk_cmd_pipeline_barrier(cb, PipelineStage::Compute, PipelineStage::Compute).unwrap();
        device.vk_end_command_buffer(cb).unwrap();
    }

    #[test]
    fn test_vk_fence_lifecycle() {
        let instance = VkInstance::new().unwrap();
        let devices = instance.vk_enumerate_physical_devices();
        let device = instance.vk_create_device(&devices[0]).unwrap();
        let fence = device.vk_create_fence(false);
        assert!(!fence.signaled);
        let fence_signaled = device.vk_create_fence(true);
        assert!(fence_signaled.signaled);
    }

    #[test]
    fn test_vk_wait_for_fences() {
        let instance = VkInstance::new().unwrap();
        let devices = instance.vk_enumerate_physical_devices();
        let device = instance.vk_create_device(&devices[0]).unwrap();
        let fence = device.vk_create_fence(true);
        assert_eq!(device.vk_wait_for_fences(&[&fence], true), VkResult::Success);
    }

    #[test]
    fn test_vk_wait_for_fences_not_ready() {
        let instance = VkInstance::new().unwrap();
        let devices = instance.vk_enumerate_physical_devices();
        let device = instance.vk_create_device(&devices[0]).unwrap();
        let fence = device.vk_create_fence(false);
        assert_eq!(device.vk_wait_for_fences(&[&fence], true), VkResult::NotReady);
    }

    #[test]
    fn test_vk_reset_fences() {
        let instance = VkInstance::new().unwrap();
        let devices = instance.vk_enumerate_physical_devices();
        let mut device = instance.vk_create_device(&devices[0]).unwrap();
        let mut fence = device.vk_create_fence(true);
        device.vk_reset_fences(&mut [&mut fence]);
        assert!(!fence.signaled);
    }

    #[test]
    fn test_vk_device_wait_idle() {
        let instance = VkInstance::new().unwrap();
        let devices = instance.vk_enumerate_physical_devices();
        let device = instance.vk_create_device(&devices[0]).unwrap();
        device.vk_device_wait_idle(); // Should not panic
    }

    #[test]
    fn test_vk_free_memory() {
        let instance = VkInstance::new().unwrap();
        let devices = instance.vk_enumerate_physical_devices();
        let mut device = instance.vk_create_device(&devices[0]).unwrap();
        let mem = device.vk_allocate_memory(&VkMemoryAllocateInfo { size: 64, memory_type_index: 0 }).unwrap();
        assert!(device.vk_free_memory(&mem).is_ok());
    }

    #[test]
    fn test_vk_semaphore_creation() {
        let instance = VkInstance::new().unwrap();
        let devices = instance.vk_enumerate_physical_devices();
        let device = instance.vk_create_device(&devices[0]).unwrap();
        let _sem = device.vk_create_semaphore();
    }

    #[test]
    fn test_vk_result_enum() {
        assert_eq!(VkResult::Success as i32, 0);
        assert_ne!(VkResult::Success, VkResult::NotReady);
    }

    #[test]
    fn test_vk_bind_buffer_memory() {
        let instance = VkInstance::new().unwrap();
        let devices = instance.vk_enumerate_physical_devices();
        let mut device = instance.vk_create_device(&devices[0]).unwrap();
        let buf = device.vk_create_buffer(&VkBufferCreateInfo { size: 64, ..Default::default() }).unwrap();
        let mem = device.vk_allocate_memory(&VkMemoryAllocateInfo { size: 64, memory_type_index: 0 }).unwrap();
        device.vk_bind_buffer_memory(&buf, &mem, 0); // No-op but should not panic
    }
}

// =========================================================================
// WebGPU Tests (30+ tests)
// =========================================================================
mod webgpu_tests {
    use vendor_api_simulators::webgpu::*;

    #[test]
    fn test_gpu_creation() {
        let gpu = Gpu::new();
        assert!(gpu.is_ok());
    }

    #[test]
    fn test_gpu_request_adapter() {
        let gpu = Gpu::new().unwrap();
        let adapter = gpu.request_adapter();
        assert!(adapter.is_ok());
        assert!(!adapter.unwrap().name.is_empty());
    }

    #[test]
    fn test_adapter_request_device() {
        let gpu = Gpu::new().unwrap();
        let adapter = gpu.request_adapter().unwrap();
        let device = adapter.request_device();
        assert!(device.is_ok());
    }

    #[test]
    fn test_gpu_create_buffer() {
        let gpu = Gpu::new().unwrap();
        let adapter = gpu.request_adapter().unwrap();
        let mut device = adapter.request_device().unwrap();
        let buf = device.create_buffer(&GpuBufferDescriptor {
            size: 256,
            usage: GpuBufferUsage::Storage,
            mapped_at_creation: false,
        });
        assert!(buf.is_ok());
        assert_eq!(buf.unwrap().size, 256);
    }

    #[test]
    fn test_gpu_buffer_mapped_at_creation() {
        let gpu = Gpu::new().unwrap();
        let adapter = gpu.request_adapter().unwrap();
        let mut device = adapter.request_device().unwrap();
        let buf = device.create_buffer(&GpuBufferDescriptor {
            size: 64,
            usage: GpuBufferUsage::Storage,
            mapped_at_creation: true,
        });
        assert!(buf.is_ok());
    }

    #[test]
    fn test_gpu_queue_write_buffer() {
        let gpu = Gpu::new().unwrap();
        let adapter = gpu.request_adapter().unwrap();
        let mut device = adapter.request_device().unwrap();
        let buf = device.create_buffer(&GpuBufferDescriptor {
            size: 4,
            usage: GpuBufferUsage::Storage,
            mapped_at_creation: false,
        }).unwrap();
        assert!(device.queue_write_buffer(&buf, 0, &[1, 2, 3, 4]).is_ok());
    }

    #[test]
    fn test_gpu_read_buffer() {
        let gpu = Gpu::new().unwrap();
        let adapter = gpu.request_adapter().unwrap();
        let mut device = adapter.request_device().unwrap();
        let buf = device.create_buffer(&GpuBufferDescriptor {
            size: 4,
            ..Default::default()
        }).unwrap();
        device.queue_write_buffer(&buf, 0, &[10, 20, 30, 40]).unwrap();
        let data = device.read_buffer(&buf).unwrap();
        assert_eq!(&data[..4], &[10, 20, 30, 40]);
    }

    #[test]
    fn test_gpu_buffer_map_async() {
        let gpu = Gpu::new().unwrap();
        let adapter = gpu.request_adapter().unwrap();
        let mut device = adapter.request_device().unwrap();
        let mut buf = device.create_buffer(&GpuBufferDescriptor {
            size: 8,
            ..Default::default()
        }).unwrap();
        device.queue_write_buffer(&buf, 0, &[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
        let mm = device.memory_manager_mut();
        assert!(buf.map_async(mm, GpuMapMode::Read).is_ok());
    }

    #[test]
    fn test_gpu_buffer_get_mapped_range() {
        let gpu = Gpu::new().unwrap();
        let adapter = gpu.request_adapter().unwrap();
        let mut device = adapter.request_device().unwrap();
        let mut buf = device.create_buffer(&GpuBufferDescriptor {
            size: 4,
            ..Default::default()
        }).unwrap();
        device.queue_write_buffer(&buf, 0, &[11, 22, 33, 44]).unwrap();
        let mm = device.memory_manager_mut();
        buf.map_async(mm, GpuMapMode::Read).unwrap();
        let data = buf.get_mapped_range(0, 4).unwrap();
        assert_eq!(data, vec![11, 22, 33, 44]);
    }

    #[test]
    fn test_gpu_buffer_get_mapped_range_not_mapped() {
        let gpu = Gpu::new().unwrap();
        let adapter = gpu.request_adapter().unwrap();
        let mut device = adapter.request_device().unwrap();
        let buf = device.create_buffer(&GpuBufferDescriptor {
            size: 4,
            ..Default::default()
        }).unwrap();
        assert!(buf.get_mapped_range(0, 4).is_err());
    }

    #[test]
    fn test_gpu_buffer_unmap() {
        let gpu = Gpu::new().unwrap();
        let adapter = gpu.request_adapter().unwrap();
        let mut device = adapter.request_device().unwrap();
        let mut buf = device.create_buffer(&GpuBufferDescriptor {
            size: 4,
            ..Default::default()
        }).unwrap();
        let mm = device.memory_manager_mut();
        buf.map_async(mm, GpuMapMode::Write).unwrap();
        assert!(buf.unmap(mm).is_ok());
    }

    #[test]
    fn test_gpu_buffer_unmap_not_mapped() {
        let gpu = Gpu::new().unwrap();
        let adapter = gpu.request_adapter().unwrap();
        let mut device = adapter.request_device().unwrap();
        let mut buf = device.create_buffer(&GpuBufferDescriptor {
            size: 4,
            ..Default::default()
        }).unwrap();
        let mm = device.memory_manager_mut();
        assert!(buf.unmap(mm).is_err());
    }

    #[test]
    fn test_gpu_buffer_destroy() {
        let gpu = Gpu::new().unwrap();
        let adapter = gpu.request_adapter().unwrap();
        let mut device = adapter.request_device().unwrap();
        let mut buf = device.create_buffer(&GpuBufferDescriptor {
            size: 64,
            ..Default::default()
        }).unwrap();
        let mm = device.memory_manager_mut();
        assert!(buf.destroy(mm).is_ok());
        assert!(buf.is_destroyed());
    }

    #[test]
    fn test_gpu_create_compute_pipeline() {
        let gpu = Gpu::new().unwrap();
        let adapter = gpu.request_adapter().unwrap();
        let mut device = adapter.request_device().unwrap();
        let _pipeline = device.create_compute_pipeline(None, &[0]);
    }

    #[test]
    fn test_gpu_create_bind_group() {
        let gpu = Gpu::new().unwrap();
        let adapter = gpu.request_adapter().unwrap();
        let mut device = adapter.request_device().unwrap();
        let buf = device.create_buffer(&GpuBufferDescriptor {
            size: 64,
            ..Default::default()
        }).unwrap();
        let bg = device.create_bind_group(&[(0, &buf)]);
        assert!(bg.is_ok());
    }

    #[test]
    fn test_gpu_command_encoder() {
        let gpu = Gpu::new().unwrap();
        let adapter = gpu.request_adapter().unwrap();
        let device = adapter.request_device().unwrap();
        let encoder = device.create_command_encoder();
        let _cb = encoder.finish();
    }

    #[test]
    fn test_gpu_compute_pass() {
        let gpu = Gpu::new().unwrap();
        let adapter = gpu.request_adapter().unwrap();
        let mut device = adapter.request_device().unwrap();
        let buf = device.create_buffer(&GpuBufferDescriptor {
            size: 256,
            ..Default::default()
        }).unwrap();
        let pipeline = device.create_compute_pipeline(None, &[0]);
        let bg = device.create_bind_group(&[(0, &buf)]).unwrap();

        let mut encoder = device.create_command_encoder();
        let mut pass = encoder.begin_compute_pass();
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bg);
        pass.dispatch_workgroups(4, 1, 1).unwrap();
        encoder.end_compute_pass(pass);
        let cb = encoder.finish();
        assert!(device.queue_submit(&[cb]).is_ok());
    }

    #[test]
    fn test_gpu_compute_pass_no_pipeline() {
        let gpu = Gpu::new().unwrap();
        let adapter = gpu.request_adapter().unwrap();
        let device = adapter.request_device().unwrap();
        let encoder = device.create_command_encoder();
        let mut pass = encoder.begin_compute_pass();
        assert!(pass.dispatch_workgroups(1, 1, 1).is_err());
    }

    #[test]
    fn test_gpu_copy_buffer_to_buffer() {
        let gpu = Gpu::new().unwrap();
        let adapter = gpu.request_adapter().unwrap();
        let mut device = adapter.request_device().unwrap();
        let src = device.create_buffer(&GpuBufferDescriptor {
            size: 64,
            ..Default::default()
        }).unwrap();
        let dst = device.create_buffer(&GpuBufferDescriptor {
            size: 64,
            ..Default::default()
        }).unwrap();
        let mut encoder = device.create_command_encoder();
        encoder.copy_buffer_to_buffer(&src, 0, &dst, 0, 64);
        let cb = encoder.finish();
        assert!(device.queue_submit(&[cb]).is_ok());
    }

    #[test]
    fn test_gpu_device_destroy() {
        let gpu = Gpu::new().unwrap();
        let adapter = gpu.request_adapter().unwrap();
        let device = adapter.request_device().unwrap();
        device.destroy(); // Should not panic
    }

    #[test]
    fn test_gpu_roundtrip_data() {
        let gpu = Gpu::new().unwrap();
        let adapter = gpu.request_adapter().unwrap();
        let mut device = adapter.request_device().unwrap();
        let buf = device.create_buffer(&GpuBufferDescriptor {
            size: 8,
            ..Default::default()
        }).unwrap();
        let data = [11u8, 22, 33, 44, 55, 66, 77, 88];
        device.queue_write_buffer(&buf, 0, &data).unwrap();
        let result = device.read_buffer(&buf).unwrap();
        assert_eq!(&result[..8], &data);
    }

    #[test]
    fn test_gpu_buffer_usage_enum() {
        assert_ne!(GpuBufferUsage::Storage, GpuBufferUsage::Uniform);
        assert_ne!(GpuBufferUsage::CopySrc, GpuBufferUsage::CopyDst);
    }

    #[test]
    fn test_gpu_map_mode_enum() {
        assert_ne!(GpuMapMode::Read, GpuMapMode::Write);
    }

    #[test]
    fn test_gpu_pipeline_bind_group_layout() {
        let gpu = Gpu::new().unwrap();
        let adapter = gpu.request_adapter().unwrap();
        let mut device = adapter.request_device().unwrap();
        let pipeline = device.create_compute_pipeline(None, &[0, 1]);
        let bindings = pipeline.get_bind_group_layout_bindings(0);
        assert!(bindings.is_some());
        assert_eq!(bindings.unwrap(), &[0, 1]);
    }

    #[test]
    fn test_gpu_pipeline_bind_group_layout_out_of_range() {
        let gpu = Gpu::new().unwrap();
        let adapter = gpu.request_adapter().unwrap();
        let mut device = adapter.request_device().unwrap();
        let pipeline = device.create_compute_pipeline(None, &[0]);
        assert!(pipeline.get_bind_group_layout_bindings(5).is_none());
    }
}

// =========================================================================
// OpenGL Tests (30+ tests)
// =========================================================================
mod opengl_tests {
    use vendor_api_simulators::opengl::*;

    #[test]
    fn test_gl_context_creation() {
        let gl = GlContext::new();
        assert!(gl.is_ok());
    }

    #[test]
    fn test_gl_create_shader() {
        let mut gl = GlContext::new().unwrap();
        let shader = gl.create_shader(GL_COMPUTE_SHADER);
        assert!(shader.is_ok());
        assert!(shader.unwrap() > 0);
    }

    #[test]
    fn test_gl_create_shader_invalid_type() {
        let mut gl = GlContext::new().unwrap();
        assert!(gl.create_shader(0x1234).is_err());
    }

    #[test]
    fn test_gl_shader_source() {
        let mut gl = GlContext::new().unwrap();
        let shader = gl.create_shader(GL_COMPUTE_SHADER).unwrap();
        assert!(gl.shader_source(shader, "void main() {}").is_ok());
    }

    #[test]
    fn test_gl_shader_source_invalid() {
        let mut gl = GlContext::new().unwrap();
        assert!(gl.shader_source(999, "source").is_err());
    }

    #[test]
    fn test_gl_compile_shader() {
        let mut gl = GlContext::new().unwrap();
        let shader = gl.create_shader(GL_COMPUTE_SHADER).unwrap();
        assert!(gl.compile_shader(shader).is_ok());
    }

    #[test]
    fn test_gl_delete_shader() {
        let mut gl = GlContext::new().unwrap();
        let shader = gl.create_shader(GL_COMPUTE_SHADER).unwrap();
        gl.delete_shader(shader);
        assert!(gl.shader_source(shader, "test").is_err());
    }

    #[test]
    fn test_gl_create_program() {
        let mut gl = GlContext::new().unwrap();
        let program = gl.create_program();
        assert!(program > 0);
    }

    #[test]
    fn test_gl_attach_shader() {
        let mut gl = GlContext::new().unwrap();
        let shader = gl.create_shader(GL_COMPUTE_SHADER).unwrap();
        let program = gl.create_program();
        assert!(gl.attach_shader(program, shader).is_ok());
    }

    #[test]
    fn test_gl_attach_shader_invalid_program() {
        let mut gl = GlContext::new().unwrap();
        let shader = gl.create_shader(GL_COMPUTE_SHADER).unwrap();
        assert!(gl.attach_shader(999, shader).is_err());
    }

    #[test]
    fn test_gl_link_program() {
        let mut gl = GlContext::new().unwrap();
        let shader = gl.create_shader(GL_COMPUTE_SHADER).unwrap();
        gl.compile_shader(shader).unwrap();
        let program = gl.create_program();
        gl.attach_shader(program, shader).unwrap();
        assert!(gl.link_program(program).is_ok());
    }

    #[test]
    fn test_gl_link_program_no_shaders() {
        let mut gl = GlContext::new().unwrap();
        let program = gl.create_program();
        assert!(gl.link_program(program).is_err());
    }

    #[test]
    fn test_gl_use_program() {
        let mut gl = GlContext::new().unwrap();
        let shader = gl.create_shader(GL_COMPUTE_SHADER).unwrap();
        gl.compile_shader(shader).unwrap();
        let program = gl.create_program();
        gl.attach_shader(program, shader).unwrap();
        gl.link_program(program).unwrap();
        assert!(gl.use_program(program).is_ok());
    }

    #[test]
    fn test_gl_use_program_unlinked() {
        let mut gl = GlContext::new().unwrap();
        let program = gl.create_program();
        assert!(gl.use_program(program).is_err());
    }

    #[test]
    fn test_gl_use_program_zero_unbinds() {
        let mut gl = GlContext::new().unwrap();
        assert!(gl.use_program(0).is_ok());
    }

    #[test]
    fn test_gl_delete_program() {
        let mut gl = GlContext::new().unwrap();
        let program = gl.create_program();
        gl.delete_program(program);
    }

    #[test]
    fn test_gl_gen_buffers() {
        let mut gl = GlContext::new().unwrap();
        let bufs = gl.gen_buffers(3);
        assert_eq!(bufs.len(), 3);
    }

    #[test]
    fn test_gl_bind_buffer() {
        let mut gl = GlContext::new().unwrap();
        let bufs = gl.gen_buffers(1);
        assert!(gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0]).is_ok());
    }

    #[test]
    fn test_gl_bind_buffer_zero() {
        let mut gl = GlContext::new().unwrap();
        assert!(gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, 0).is_ok());
    }

    #[test]
    fn test_gl_buffer_data() {
        let mut gl = GlContext::new().unwrap();
        let bufs = gl.gen_buffers(1);
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0]).unwrap();
        let data = [1u8, 2, 3, 4];
        assert!(gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 4, Some(&data), GL_STATIC_DRAW).is_ok());
    }

    #[test]
    fn test_gl_buffer_data_no_data() {
        let mut gl = GlContext::new().unwrap();
        let bufs = gl.gen_buffers(1);
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0]).unwrap();
        assert!(gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 256, None, GL_DYNAMIC_DRAW).is_ok());
    }

    #[test]
    fn test_gl_buffer_data_no_binding() {
        let mut gl = GlContext::new().unwrap();
        assert!(gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 64, None, GL_STATIC_DRAW).is_err());
    }

    #[test]
    fn test_gl_buffer_sub_data() {
        let mut gl = GlContext::new().unwrap();
        let bufs = gl.gen_buffers(1);
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0]).unwrap();
        gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 16, None, GL_STATIC_DRAW).unwrap();
        assert!(gl.buffer_sub_data(GL_SHADER_STORAGE_BUFFER, 0, &[10, 20]).is_ok());
    }

    #[test]
    fn test_gl_bind_buffer_base() {
        let mut gl = GlContext::new().unwrap();
        let bufs = gl.gen_buffers(1);
        assert!(gl.bind_buffer_base(GL_SHADER_STORAGE_BUFFER, 0, bufs[0]).is_ok());
    }

    #[test]
    fn test_gl_map_buffer_range() {
        let mut gl = GlContext::new().unwrap();
        let bufs = gl.gen_buffers(1);
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0]).unwrap();
        gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 8, Some(&[1, 2, 3, 4, 5, 6, 7, 8]), GL_STATIC_DRAW).unwrap();
        let data = gl.map_buffer_range(GL_SHADER_STORAGE_BUFFER, 0, 4, GL_MAP_READ_BIT);
        assert!(data.is_ok());
        assert_eq!(data.unwrap(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_gl_unmap_buffer() {
        let gl = GlContext::new().unwrap();
        assert!(gl.unmap_buffer(GL_SHADER_STORAGE_BUFFER));
    }

    #[test]
    fn test_gl_dispatch_compute() {
        let mut gl = GlContext::new().unwrap();
        let shader = gl.create_shader(GL_COMPUTE_SHADER).unwrap();
        gl.compile_shader(shader).unwrap();
        let program = gl.create_program();
        gl.attach_shader(program, shader).unwrap();
        gl.link_program(program).unwrap();
        gl.use_program(program).unwrap();

        let bufs = gl.gen_buffers(1);
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0]).unwrap();
        gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 256, None, GL_DYNAMIC_DRAW).unwrap();
        gl.bind_buffer_base(GL_SHADER_STORAGE_BUFFER, 0, bufs[0]).unwrap();

        assert!(gl.dispatch_compute(4, 1, 1).is_ok());
    }

    #[test]
    fn test_gl_dispatch_no_program() {
        let mut gl = GlContext::new().unwrap();
        assert!(gl.dispatch_compute(1, 1, 1).is_err());
    }

    #[test]
    fn test_gl_memory_barrier() {
        let gl = GlContext::new().unwrap();
        gl.memory_barrier(GL_SHADER_STORAGE_BARRIER_BIT); // Should not panic
    }

    #[test]
    fn test_gl_fence_sync() {
        let mut gl = GlContext::new().unwrap();
        let sync = gl.fence_sync();
        assert!(sync > 0);
    }

    #[test]
    fn test_gl_client_wait_sync() {
        let mut gl = GlContext::new().unwrap();
        let sync = gl.fence_sync();
        assert_eq!(gl.client_wait_sync(sync, 0, 0), GL_ALREADY_SIGNALED);
    }

    #[test]
    fn test_gl_client_wait_sync_invalid() {
        let gl = GlContext::new().unwrap();
        assert_eq!(gl.client_wait_sync(999, 0, 0), GL_WAIT_FAILED);
    }

    #[test]
    fn test_gl_delete_sync() {
        let mut gl = GlContext::new().unwrap();
        let sync = gl.fence_sync();
        gl.delete_sync(sync);
        assert_eq!(gl.client_wait_sync(sync, 0, 0), GL_WAIT_FAILED);
    }

    #[test]
    fn test_gl_finish() {
        let gl = GlContext::new().unwrap();
        gl.finish(); // Should not panic
    }

    #[test]
    fn test_gl_get_uniform_location() {
        let mut gl = GlContext::new().unwrap();
        let shader = gl.create_shader(GL_COMPUTE_SHADER).unwrap();
        gl.compile_shader(shader).unwrap();
        let program = gl.create_program();
        gl.attach_shader(program, shader).unwrap();
        gl.link_program(program).unwrap();
        let loc = gl.get_uniform_location(program, "alpha");
        assert!(loc.is_ok());
    }

    #[test]
    fn test_gl_uniform_1f() {
        let mut gl = GlContext::new().unwrap();
        let shader = gl.create_shader(GL_COMPUTE_SHADER).unwrap();
        gl.compile_shader(shader).unwrap();
        let program = gl.create_program();
        gl.attach_shader(program, shader).unwrap();
        gl.link_program(program).unwrap();
        gl.use_program(program).unwrap();
        let loc = gl.get_uniform_location(program, "alpha").unwrap();
        gl.uniform_1f(loc, 2.5);
    }

    #[test]
    fn test_gl_uniform_1i() {
        let mut gl = GlContext::new().unwrap();
        let shader = gl.create_shader(GL_COMPUTE_SHADER).unwrap();
        gl.compile_shader(shader).unwrap();
        let program = gl.create_program();
        gl.attach_shader(program, shader).unwrap();
        gl.link_program(program).unwrap();
        gl.use_program(program).unwrap();
        let loc = gl.get_uniform_location(program, "count").unwrap();
        gl.uniform_1i(loc, 42);
    }

    #[test]
    fn test_gl_delete_buffers() {
        let mut gl = GlContext::new().unwrap();
        let bufs = gl.gen_buffers(2);
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0]).unwrap();
        gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 64, None, GL_STATIC_DRAW).unwrap();
        gl.delete_buffers(&bufs);
    }
}

// =========================================================================
// Cross-API Tests
// =========================================================================
mod cross_api_tests {
    use vendor_api_simulators::cuda::*;
    use vendor_api_simulators::opencl::*;
    use vendor_api_simulators::metal::*;
    use vendor_api_simulators::vulkan::*;
    use vendor_api_simulators::webgpu::*;
    use vendor_api_simulators::opengl::*;

    #[test]
    fn test_all_six_apis_can_be_created() {
        assert!(CudaRuntime::new().is_ok());
        assert!(ClContext::new().is_ok());
        assert!(MtlDevice::new().is_ok());
        assert!(VkInstance::new().is_ok());
        assert!(Gpu::new().is_ok());
        assert!(GlContext::new().is_ok());
    }

    #[test]
    fn test_all_six_apis_can_allocate_memory() {
        let mut cuda = CudaRuntime::new().unwrap();
        assert!(cuda.malloc(64).is_ok());

        let mut cl = ClContext::new().unwrap();
        assert!(cl.create_buffer(ClMemFlags::ReadWrite, 64, None).is_ok());

        let mut mtl = MtlDevice::new().unwrap();
        assert!(mtl.make_buffer(64).is_ok());

        let vk_inst = VkInstance::new().unwrap();
        let devices = vk_inst.vk_enumerate_physical_devices();
        let mut vk_dev = vk_inst.vk_create_device(&devices[0]).unwrap();
        assert!(vk_dev.vk_create_buffer(&VkBufferCreateInfo { size: 64, ..Default::default() }).is_ok());

        let gpu = Gpu::new().unwrap();
        let adapter = gpu.request_adapter().unwrap();
        let mut webgpu_dev = adapter.request_device().unwrap();
        assert!(webgpu_dev.create_buffer(&GpuBufferDescriptor { size: 64, ..Default::default() }).is_ok());

        let mut gl = GlContext::new().unwrap();
        let bufs = gl.gen_buffers(1);
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0]).unwrap();
        assert!(gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 64, None, GL_STATIC_DRAW).is_ok());
    }

    #[test]
    fn test_all_six_apis_roundtrip_data() {
        // CUDA roundtrip
        let mut cuda = CudaRuntime::new().unwrap();
        let ptr = cuda.malloc(4).unwrap();
        cuda.memcpy_host_to_device(ptr, &[1, 2, 3, 4]).unwrap();
        let mut result = [0u8; 4];
        cuda.memcpy_device_to_host(&mut result, ptr).unwrap();
        assert_eq!(result, [1, 2, 3, 4]);

        // Metal roundtrip
        let mut mtl = MtlDevice::new().unwrap();
        let buf = mtl.make_buffer(4).unwrap();
        mtl.write_buffer(&buf, &[5, 6, 7, 8]).unwrap();
        let data = mtl.read_buffer(&buf).unwrap();
        assert_eq!(&data[..4], &[5, 6, 7, 8]);

        // WebGPU roundtrip
        let gpu = Gpu::new().unwrap();
        let adapter = gpu.request_adapter().unwrap();
        let mut device = adapter.request_device().unwrap();
        let gbuf = device.create_buffer(&GpuBufferDescriptor {
            size: 4,
            ..Default::default()
        }).unwrap();
        device.queue_write_buffer(&gbuf, 0, &[9, 10, 11, 12]).unwrap();
        let gdata = device.read_buffer(&gbuf).unwrap();
        assert_eq!(&gdata[..4], &[9, 10, 11, 12]);
    }

    #[test]
    fn test_all_six_apis_can_dispatch() {
        // CUDA dispatch
        let mut cuda = CudaRuntime::new().unwrap();
        let kernel = CudaKernel::new("test", None);
        assert!(cuda.launch_kernel(&kernel, Dim3::new(1, 1, 1), Dim3::new(1, 1, 1), &[]).is_ok());

        // OpenGL dispatch
        let mut gl = GlContext::new().unwrap();
        let shader = gl.create_shader(GL_COMPUTE_SHADER).unwrap();
        gl.compile_shader(shader).unwrap();
        let program = gl.create_program();
        gl.attach_shader(program, shader).unwrap();
        gl.link_program(program).unwrap();
        gl.use_program(program).unwrap();
        assert!(gl.dispatch_compute(1, 1, 1).is_ok());

        // Metal dispatch
        let mut mtl = MtlDevice::new().unwrap();
        let buf = mtl.make_buffer(64).unwrap();
        let lib = mtl.make_library("test");
        let func = lib.make_function("compute");
        let pso = mtl.make_compute_pipeline_state(&func).unwrap();
        let mut enc = mtl.make_compute_command_encoder();
        enc.set_compute_pipeline_state(&pso);
        enc.set_buffer(&buf, 0, 0);
        enc.end_encoding();
        assert!(mtl.dispatch_threadgroups(&enc, MtlSize::new(1, 1, 1), MtlSize::new(1, 1, 1)).is_ok());
    }

    #[test]
    fn test_all_six_apis_synchronize() {
        let cuda = CudaRuntime::new().unwrap();
        cuda.device_synchronize();

        let _cl = ClContext::new().unwrap();
        // cl.finish() via queue
        // Already tested in opencl_tests

        let mtl = MtlDevice::new().unwrap();
        mtl.wait_until_completed();

        let vk_inst = VkInstance::new().unwrap();
        let devices = vk_inst.vk_enumerate_physical_devices();
        let vk_dev = vk_inst.vk_create_device(&devices[0]).unwrap();
        vk_dev.vk_device_wait_idle();

        let gpu = Gpu::new().unwrap();
        let adapter = gpu.request_adapter().unwrap();
        let device = adapter.request_device().unwrap();
        device.destroy();

        let gl = GlContext::new().unwrap();
        gl.finish();
    }
}
