//! Tests for GPU backends: CUDA, Metal, OpenCL, Vulkan, WebGPU, OpenGL.
//!
//! Each GPU backend delegates arithmetic to the CPU reference but exercises
//! the GPU memory pipeline (allocate, upload, download, free). Since all
//! GPU backends produce the same results as CPU (by design), these tests
//! verify correctness AND that the GPU pipeline doesn't corrupt data.

use blas_library::traits::BlasBackend;
use blas_library::{
    CpuBlas, CudaBlas, GpuBlasWrapper, Matrix, MetalBlas, OpenClBlas, OpenGlBlas, Side,
    Transpose, Vector, VulkanBlas, WebGpuBlas,
};

// =========================================================================
// Helper: create all GPU backends as BlasBackend trait objects
// =========================================================================

fn all_gpu_backends() -> Vec<(&'static str, Box<dyn BlasBackend>)> {
    vec![
        (
            "cuda",
            Box::new(GpuBlasWrapper::new(CudaBlas::new().unwrap())) as Box<dyn BlasBackend>,
        ),
        (
            "metal",
            Box::new(GpuBlasWrapper::new(MetalBlas::new().unwrap())) as Box<dyn BlasBackend>,
        ),
        (
            "opencl",
            Box::new(GpuBlasWrapper::new(OpenClBlas::new().unwrap())) as Box<dyn BlasBackend>,
        ),
        (
            "vulkan",
            Box::new(GpuBlasWrapper::new(VulkanBlas::new().unwrap())) as Box<dyn BlasBackend>,
        ),
        (
            "webgpu",
            Box::new(GpuBlasWrapper::new(WebGpuBlas::new().unwrap())) as Box<dyn BlasBackend>,
        ),
        (
            "opengl",
            Box::new(GpuBlasWrapper::new(OpenGlBlas::new().unwrap())) as Box<dyn BlasBackend>,
        ),
    ]
}

// =========================================================================
// GPU backend creation
// =========================================================================

#[test]
fn test_cuda_creation() {
    let gpu = CudaBlas::new().unwrap();
    let wrapper = GpuBlasWrapper::new(gpu);
    assert_eq!(wrapper.name(), "cuda");
}

#[test]
fn test_metal_creation() {
    let gpu = MetalBlas::new().unwrap();
    let wrapper = GpuBlasWrapper::new(gpu);
    assert_eq!(wrapper.name(), "metal");
}

#[test]
fn test_opencl_creation() {
    let gpu = OpenClBlas::new().unwrap();
    let wrapper = GpuBlasWrapper::new(gpu);
    assert_eq!(wrapper.name(), "opencl");
}

#[test]
fn test_vulkan_creation() {
    let gpu = VulkanBlas::new().unwrap();
    let wrapper = GpuBlasWrapper::new(gpu);
    assert_eq!(wrapper.name(), "vulkan");
}

#[test]
fn test_webgpu_creation() {
    let gpu = WebGpuBlas::new().unwrap();
    let wrapper = GpuBlasWrapper::new(gpu);
    assert_eq!(wrapper.name(), "webgpu");
}

#[test]
fn test_opengl_creation() {
    let gpu = OpenGlBlas::new().unwrap();
    let wrapper = GpuBlasWrapper::new(gpu);
    assert_eq!(wrapper.name(), "opengl");
}

// =========================================================================
// GPU device names
// =========================================================================

#[test]
fn test_cuda_device_name() {
    let gpu = CudaBlas::new().unwrap();
    let wrapper = GpuBlasWrapper::new(gpu);
    assert!(!wrapper.device_name().is_empty());
}

#[test]
fn test_metal_device_name() {
    let gpu = MetalBlas::new().unwrap();
    let wrapper = GpuBlasWrapper::new(gpu);
    assert!(!wrapper.device_name().is_empty());
}

#[test]
fn test_vulkan_device_name() {
    let gpu = VulkanBlas::new().unwrap();
    let wrapper = GpuBlasWrapper::new(gpu);
    assert_eq!(wrapper.device_name(), "Vulkan Device");
}

#[test]
fn test_webgpu_device_name() {
    let gpu = WebGpuBlas::new().unwrap();
    let wrapper = GpuBlasWrapper::new(gpu);
    assert_eq!(wrapper.device_name(), "WebGPU Device");
}

#[test]
fn test_opengl_device_name() {
    let gpu = OpenGlBlas::new().unwrap();
    let wrapper = GpuBlasWrapper::new(gpu);
    assert_eq!(wrapper.device_name(), "OpenGL Device");
}

// =========================================================================
// GPU backends match CPU for Level 1 operations
// =========================================================================

#[test]
fn test_gpu_saxpy_matches_cpu() {
    let cpu = CpuBlas;
    let x = Vector::new(vec![1.0, 2.0, 3.0, 4.0]);
    let y = Vector::new(vec![5.0, 6.0, 7.0, 8.0]);
    let cpu_result = cpu.saxpy(2.0, &x, &y).unwrap();

    for (name, backend) in all_gpu_backends() {
        let gpu_result = backend.saxpy(2.0, &x, &y).unwrap();
        assert_eq!(
            gpu_result.data(),
            cpu_result.data(),
            "{} SAXPY mismatch",
            name
        );
    }
}

#[test]
fn test_gpu_sdot_matches_cpu() {
    let cpu = CpuBlas;
    let x = Vector::new(vec![1.0, 2.0, 3.0]);
    let y = Vector::new(vec![4.0, 5.0, 6.0]);
    let cpu_result = cpu.sdot(&x, &y).unwrap();

    for (name, backend) in all_gpu_backends() {
        let gpu_result = backend.sdot(&x, &y).unwrap();
        assert_eq!(gpu_result, cpu_result, "{} SDOT mismatch", name);
    }
}

#[test]
fn test_gpu_snrm2_matches_cpu() {
    let cpu = CpuBlas;
    let x = Vector::new(vec![3.0, 4.0]);
    let cpu_result = cpu.snrm2(&x);

    for (name, backend) in all_gpu_backends() {
        let gpu_result = backend.snrm2(&x);
        assert_eq!(gpu_result, cpu_result, "{} SNRM2 mismatch", name);
    }
}

#[test]
fn test_gpu_sscal_matches_cpu() {
    let cpu = CpuBlas;
    let x = Vector::new(vec![1.0, 2.0, 3.0]);
    let cpu_result = cpu.sscal(3.0, &x);

    for (name, backend) in all_gpu_backends() {
        let gpu_result = backend.sscal(3.0, &x);
        assert_eq!(
            gpu_result.data(),
            cpu_result.data(),
            "{} SSCAL mismatch",
            name
        );
    }
}

#[test]
fn test_gpu_sasum_matches_cpu() {
    let cpu = CpuBlas;
    let x = Vector::new(vec![-1.0, 2.0, -3.0]);
    let cpu_result = cpu.sasum(&x);

    for (name, backend) in all_gpu_backends() {
        let gpu_result = backend.sasum(&x);
        assert_eq!(gpu_result, cpu_result, "{} SASUM mismatch", name);
    }
}

#[test]
fn test_gpu_isamax_matches_cpu() {
    let cpu = CpuBlas;
    let x = Vector::new(vec![1.0, -5.0, 3.0]);
    let cpu_result = cpu.isamax(&x);

    for (name, backend) in all_gpu_backends() {
        let gpu_result = backend.isamax(&x);
        assert_eq!(gpu_result, cpu_result, "{} ISAMAX mismatch", name);
    }
}

#[test]
fn test_gpu_scopy_matches_cpu() {
    let cpu = CpuBlas;
    let x = Vector::new(vec![1.0, 2.0, 3.0]);
    let cpu_result = cpu.scopy(&x);

    for (name, backend) in all_gpu_backends() {
        let gpu_result = backend.scopy(&x);
        assert_eq!(
            gpu_result.data(),
            cpu_result.data(),
            "{} SCOPY mismatch",
            name
        );
    }
}

#[test]
fn test_gpu_sswap_matches_cpu() {
    let cpu = CpuBlas;
    let x = Vector::new(vec![1.0, 2.0]);
    let y = Vector::new(vec![3.0, 4.0]);
    let (cpu_x, cpu_y) = cpu.sswap(&x, &y).unwrap();

    for (name, backend) in all_gpu_backends() {
        let (gpu_x, gpu_y) = backend.sswap(&x, &y).unwrap();
        assert_eq!(gpu_x.data(), cpu_x.data(), "{} SSWAP X mismatch", name);
        assert_eq!(gpu_y.data(), cpu_y.data(), "{} SSWAP Y mismatch", name);
    }
}

// =========================================================================
// GPU backends match CPU for Level 2 operations
// =========================================================================

#[test]
fn test_gpu_sgemv_matches_cpu() {
    let cpu = CpuBlas;
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let x = Vector::new(vec![1.0, 1.0]);
    let y = Vector::zeros(2);
    let cpu_result = cpu
        .sgemv(Transpose::NoTrans, 1.0, &a, &x, 0.0, &y)
        .unwrap();

    for (name, backend) in all_gpu_backends() {
        let gpu_result = backend
            .sgemv(Transpose::NoTrans, 1.0, &a, &x, 0.0, &y)
            .unwrap();
        assert_eq!(
            gpu_result.data(),
            cpu_result.data(),
            "{} SGEMV mismatch",
            name
        );
    }
}

#[test]
fn test_gpu_sger_matches_cpu() {
    let cpu = CpuBlas;
    let x = Vector::new(vec![1.0, 2.0]);
    let y = Vector::new(vec![3.0, 4.0]);
    let a = Matrix::zeros(2, 2);
    let cpu_result = cpu.sger(1.0, &x, &y, &a).unwrap();

    for (name, backend) in all_gpu_backends() {
        let gpu_result = backend.sger(1.0, &x, &y, &a).unwrap();
        assert_eq!(
            gpu_result.data(),
            cpu_result.data(),
            "{} SGER mismatch",
            name
        );
    }
}

// =========================================================================
// GPU backends match CPU for Level 3 operations
// =========================================================================

#[test]
fn test_gpu_sgemm_matches_cpu() {
    let cpu = CpuBlas;
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let b = Matrix::new(vec![5.0, 6.0, 7.0, 8.0], 2, 2);
    let c = Matrix::zeros(2, 2);
    let cpu_result = cpu
        .sgemm(Transpose::NoTrans, Transpose::NoTrans, 1.0, &a, &b, 0.0, &c)
        .unwrap();

    for (name, backend) in all_gpu_backends() {
        let gpu_result = backend
            .sgemm(Transpose::NoTrans, Transpose::NoTrans, 1.0, &a, &b, 0.0, &c)
            .unwrap();
        assert_eq!(
            gpu_result.data(),
            cpu_result.data(),
            "{} SGEMM mismatch",
            name
        );
    }
}

#[test]
fn test_gpu_ssymm_matches_cpu() {
    let cpu = CpuBlas;
    let a = Matrix::new(vec![1.0, 2.0, 2.0, 3.0], 2, 2);
    let b = Matrix::new(vec![1.0, 0.0, 0.0, 1.0], 2, 2);
    let c = Matrix::zeros(2, 2);
    let cpu_result = cpu.ssymm(Side::Left, 1.0, &a, &b, 0.0, &c).unwrap();

    for (name, backend) in all_gpu_backends() {
        let gpu_result = backend
            .ssymm(Side::Left, 1.0, &a, &b, 0.0, &c)
            .unwrap();
        assert_eq!(
            gpu_result.data(),
            cpu_result.data(),
            "{} SSYMM mismatch",
            name
        );
    }
}

#[test]
fn test_gpu_sgemm_batched_matches_cpu() {
    let cpu = CpuBlas;
    let a = vec![Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2)];
    let b = vec![Matrix::new(vec![5.0, 6.0, 7.0, 8.0], 2, 2)];
    let c = vec![Matrix::zeros(2, 2)];
    let cpu_results = cpu
        .sgemm_batched(Transpose::NoTrans, Transpose::NoTrans, 1.0, &a, &b, 0.0, &c)
        .unwrap();

    for (name, backend) in all_gpu_backends() {
        let gpu_results = backend
            .sgemm_batched(Transpose::NoTrans, Transpose::NoTrans, 1.0, &a, &b, 0.0, &c)
            .unwrap();
        assert_eq!(gpu_results.len(), cpu_results.len(), "{} batch len", name);
        assert_eq!(
            gpu_results[0].data(),
            cpu_results[0].data(),
            "{} SGEMM_BATCHED mismatch",
            name
        );
    }
}

// =========================================================================
// GPU memory pipeline exercise (upload/download/free)
// =========================================================================

#[test]
fn test_gpu_base_helpers() {
    use blas_library::backends::gpu_base::{bytes_to_floats, floats_to_bytes};

    let floats = vec![1.0_f32, 2.0, 3.0, 4.0];
    let bytes = floats_to_bytes(&floats);
    assert_eq!(bytes.len(), 16); // 4 floats * 4 bytes each
    let recovered = bytes_to_floats(&bytes, 4);
    assert_eq!(recovered, floats);
}

#[test]
fn test_gpu_base_round_trip_empty() {
    use blas_library::backends::gpu_base::{bytes_to_floats, floats_to_bytes};

    let floats: Vec<f32> = vec![];
    let bytes = floats_to_bytes(&floats);
    assert!(bytes.is_empty());
    let recovered = bytes_to_floats(&bytes, 0);
    assert!(recovered.is_empty());
}

#[test]
fn test_gpu_base_round_trip_special_values() {
    use blas_library::backends::gpu_base::{bytes_to_floats, floats_to_bytes};

    let floats = vec![0.0_f32, -0.0, f32::INFINITY, f32::NEG_INFINITY];
    let bytes = floats_to_bytes(&floats);
    let recovered = bytes_to_floats(&bytes, 4);
    assert_eq!(recovered[0], 0.0);
    assert!(recovered[2].is_infinite() && recovered[2] > 0.0);
    assert!(recovered[3].is_infinite() && recovered[3] < 0.0);
}

// =========================================================================
// GPU pipeline exercise tests (upload -> compute -> download)
// =========================================================================

#[test]
fn test_cuda_upload_download() {
    use blas_library::GpuBlasBackend;
    use blas_library::backends::gpu_base::floats_to_bytes;

    let mut gpu = CudaBlas::new().unwrap();
    let data = floats_to_bytes(&[1.0, 2.0, 3.0, 4.0]);
    let handle = gpu.upload(&data).unwrap();
    let result = gpu.download(handle, data.len()).unwrap();
    assert_eq!(data, result);
    gpu.free(handle).unwrap();
}

#[test]
fn test_metal_upload_download() {
    use blas_library::GpuBlasBackend;
    use blas_library::backends::gpu_base::floats_to_bytes;

    let mut gpu = MetalBlas::new().unwrap();
    let data = floats_to_bytes(&[1.0, 2.0, 3.0, 4.0]);
    let handle = gpu.upload(&data).unwrap();
    let result = gpu.download(handle, data.len()).unwrap();
    assert_eq!(data, result);
    gpu.free(handle).unwrap();
}

#[test]
fn test_opencl_upload_download() {
    use blas_library::GpuBlasBackend;
    use blas_library::backends::gpu_base::floats_to_bytes;

    let mut gpu = OpenClBlas::new().unwrap();
    let data = floats_to_bytes(&[1.0, 2.0, 3.0, 4.0]);
    let handle = gpu.upload(&data).unwrap();
    let result = gpu.download(handle, data.len()).unwrap();
    assert_eq!(data, result);
    // OpenCL doesn't need explicit free (context manages it)
    gpu.free(handle).unwrap();
}

#[test]
fn test_vulkan_upload_download() {
    use blas_library::GpuBlasBackend;
    use blas_library::backends::gpu_base::floats_to_bytes;

    let mut gpu = VulkanBlas::new().unwrap();
    let data = floats_to_bytes(&[1.0, 2.0, 3.0, 4.0]);
    let handle = gpu.upload(&data).unwrap();
    let result = gpu.download(handle, data.len()).unwrap();
    assert_eq!(data, result);
    gpu.free(handle).unwrap();
}

#[test]
fn test_webgpu_upload_download() {
    use blas_library::GpuBlasBackend;
    use blas_library::backends::gpu_base::floats_to_bytes;

    let mut gpu = WebGpuBlas::new().unwrap();
    let data = floats_to_bytes(&[1.0, 2.0, 3.0, 4.0]);
    let handle = gpu.upload(&data).unwrap();
    let result = gpu.download(handle, data.len()).unwrap();
    assert_eq!(data, result);
    gpu.free(handle).unwrap();
}

#[test]
fn test_opengl_upload_download() {
    use blas_library::GpuBlasBackend;
    use blas_library::backends::gpu_base::floats_to_bytes;

    let mut gpu = OpenGlBlas::new().unwrap();
    let data = floats_to_bytes(&[1.0, 2.0, 3.0, 4.0]);
    let handle = gpu.upload(&data).unwrap();
    let result = gpu.download(handle, data.len()).unwrap();
    assert_eq!(data, result);
    gpu.free(handle).unwrap();
}
