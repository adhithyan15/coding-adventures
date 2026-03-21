//! CUDA Runtime Simulator -- NVIDIA's "just launch it" GPU programming model.
//!
//! # What is CUDA?
//!
//! CUDA (Compute Unified Device Architecture) is NVIDIA's proprietary GPU
//! computing platform. It's the most popular GPU programming API, used by
//! PyTorch, TensorFlow, and virtually all ML research.
//!
//! CUDA's design philosophy is **"make the common case easy."** The common
//! case for GPU programming is:
//!
//! 1. Allocate memory on the GPU          --> `cuda.malloc()`
//! 2. Copy data from CPU to GPU           --> `cuda.memcpy(HostToDevice)`
//! 3. Launch a kernel                     --> `cuda.launch_kernel()`
//! 4. Copy results back                   --> `cuda.memcpy(DeviceToHost)`
//! 5. Free memory                         --> `cuda.free()`
//!
//! Each of these is a single function call. Compare this to Vulkan, where
//! launching a kernel requires creating a pipeline, descriptor set, command
//! buffer, recording commands, submitting, and waiting.
//!
//! # How CUDA Hides Complexity
//!
//! When you write `kernel<<<grid, block>>>(args)` in CUDA, here's what
//! happens internally (and what our simulator does):
//!
//! 1. Create a Pipeline from the kernel's code
//! 2. Create a DescriptorSet and bind the argument buffers
//! 3. Create a CommandBuffer
//! 4. Record: bind_pipeline, bind_descriptor_set, dispatch
//! 5. Submit the CommandBuffer to the default stream's queue
//! 6. Wait for completion (synchronous in default stream)
//!
//! You never see steps 1-6. That's the magic of CUDA -- it feels like
//! calling a function, but underneath it's the full Vulkan-style pipeline.
//!
//! # Streams
//!
//! CUDA streams are independent execution queues. The default stream (stream 0)
//! is synchronous -- every operation completes before the next starts.
//!
//! # Memory Model
//!
//! CUDA simplifies memory into two main types:
//!
//! - `malloc()`:          GPU-only memory (DEVICE_LOCAL in Layer 5)
//! - `malloc_managed()`:  Unified memory accessible from both CPU and GPU

use compute_runtime::protocols::DescriptorBinding;
use gpu_core::Instruction;

use crate::base::BaseSimulator;

// =========================================================================
// CUDA-specific types
// =========================================================================

/// dim3 -- the classic CUDA grid/block dimension type.
///
/// In real CUDA, dim3 is a struct with x, y, z fields. When you write
/// `kernel<<<dim3(4, 1, 1), dim3(64, 1, 1)>>>`, you're saying:
///   "Launch 4 blocks of 64 threads each, in 1D."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dim3 {
    pub x: usize,
    pub y: usize,
    pub z: usize,
}

impl Dim3 {
    /// Create a new dim3 with the given dimensions.
    pub fn new(x: usize, y: usize, z: usize) -> Self {
        Self { x, y, z }
    }
}

/// Direction of a CUDA memory copy.
///
/// # The Four Copy Directions
///
/// - `HostToDevice`:    CPU RAM -> GPU VRAM (upload)
/// - `DeviceToHost`:    GPU VRAM -> CPU RAM (download)
/// - `DeviceToDevice`:  GPU VRAM -> GPU VRAM (on-device copy)
/// - `HostToHost`:      CPU RAM -> CPU RAM (plain memcpy)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CudaMemcpyKind {
    HostToDevice,
    DeviceToHost,
    DeviceToDevice,
    HostToHost,
}

/// Properties of a CUDA device, similar to cudaDeviceProp.
///
/// In real CUDA, you query these with `cudaGetDeviceProperties()`. They
/// tell you what the GPU can do -- how much memory, how many threads,
/// what compute capability.
#[derive(Debug, Clone)]
pub struct CudaDeviceProperties {
    pub name: String,
    pub total_global_mem: u64,
    pub shared_mem_per_block: usize,
    pub max_threads_per_block: usize,
    pub max_grid_size: (usize, usize, usize),
    pub warp_size: usize,
    pub compute_capability: (usize, usize),
}

/// A CUDA kernel -- compiled GPU code ready to launch.
///
/// In real CUDA, kernels are C++ functions decorated with `__global__`.
/// In our simulator, a kernel wraps a list of GPU instructions from
/// the gpu-core package (Layer 9).
#[derive(Debug, Clone)]
pub struct CudaKernel {
    pub code: Option<Vec<Instruction>>,
    pub name: String,
}

impl CudaKernel {
    /// Create a new kernel with optional GPU instructions.
    pub fn new(name: &str, code: Option<Vec<Instruction>>) -> Self {
        Self {
            code,
            name: name.to_string(),
        }
    }
}

/// A CUDA device pointer -- a handle to GPU memory.
///
/// In real CUDA, `cudaMalloc()` returns a `void*` pointer to device memory.
/// You can't dereference it on the CPU -- it's only valid on the GPU.
///
/// In our simulator, `CudaDevicePtr` wraps a Layer 5 buffer ID.
#[derive(Debug, Clone, Copy)]
pub struct CudaDevicePtr {
    pub buffer_id: usize,
    pub device_address: u64,
    pub size: usize,
}

/// A CUDA stream -- an independent execution queue.
///
/// A stream is a sequence of GPU operations that execute in order.
/// Operations in the same stream are guaranteed to execute sequentially.
/// Operations in different streams MAY execute concurrently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CudaStream {
    id: usize,
}

impl CudaStream {
    /// Get the stream ID.
    pub fn id(&self) -> usize {
        self.id
    }
}

/// A CUDA event -- a timestamp marker in a stream.
///
/// Events are used for two things in CUDA:
/// 1. GPU timing -- record event before and after a kernel, measure elapsed
/// 2. Stream synchronization -- one stream can wait for another's event
#[derive(Debug)]
pub struct CudaEvent {
    pub timestamp: u64,
    pub recorded: bool,
    signaled: bool,
}

// =========================================================================
// CudaRuntime -- the main simulator class
// =========================================================================

/// CUDA runtime simulator -- wraps Layer 5 with CUDA semantics.
///
/// # Usage
///
/// ```text
/// let mut cuda = CudaRuntime::new().unwrap();
///
/// // Allocate, copy, launch, synchronize -- just like real CUDA
/// let d_x = cuda.malloc(1024).unwrap();
/// cuda.memcpy_host_to_device(d_x, &data).unwrap();
/// cuda.launch_kernel(&kernel, grid, block, &[d_x]).unwrap();
/// cuda.device_synchronize();
/// cuda.free(d_x).unwrap();
/// ```
pub struct CudaRuntime {
    base: BaseSimulator,
    device_id: usize,
    streams: Vec<CudaStream>,
    next_stream_id: usize,
    events: Vec<CudaEvent>,
}

impl CudaRuntime {
    /// Initialize CUDA runtime, selecting an NVIDIA GPU.
    pub fn new() -> Result<Self, String> {
        let base = BaseSimulator::new(None, Some("nvidia"))?;
        Ok(Self {
            base,
            device_id: 0,
            streams: Vec::new(),
            next_stream_id: 1,
            events: Vec::new(),
        })
    }

    // =================================================================
    // Device management
    // =================================================================

    /// Select which GPU to use (`cudaSetDevice`).
    ///
    /// In multi-GPU systems, this switches the "current" device. In our
    /// simulator, we only model one device, so this validates the ID.
    pub fn set_device(&mut self, device_id: usize) -> Result<(), String> {
        let num_devices = self.base.instance.enumerate_physical_devices().len();
        if device_id >= num_devices {
            return Err(format!(
                "Invalid device ID {}. Available: 0-{}",
                device_id,
                num_devices - 1
            ));
        }
        self.device_id = device_id;
        Ok(())
    }

    /// Get the current device ID (`cudaGetDevice`).
    pub fn get_device(&self) -> usize {
        self.device_id
    }

    /// Query device properties (`cudaGetDeviceProperties`).
    pub fn get_device_properties(&self) -> CudaDeviceProperties {
        let devices = self.base.instance.enumerate_physical_devices();
        let pd = devices[self.base.device_index];
        let mem_size: u64 = pd.memory_properties().heaps.iter().map(|h| h.size).sum();
        CudaDeviceProperties {
            name: pd.name().to_string(),
            total_global_mem: mem_size,
            shared_mem_per_block: 49152, // 48 KB
            max_threads_per_block: pd.limits().max_workgroup_size.0,
            max_grid_size: pd.limits().max_workgroup_count,
            warp_size: 32,
            compute_capability: (8, 0),
        }
    }

    /// Wait for all GPU work to complete (`cudaDeviceSynchronize`).
    ///
    /// This is the bluntest synchronization tool -- it blocks the CPU
    /// until every kernel, every copy, every operation has finished.
    pub fn device_synchronize(&self) {
        self.base.device.wait_idle();
    }

    /// Reset the device (`cudaDeviceReset`).
    ///
    /// Clears all state (streams, events).
    pub fn device_reset(&mut self) {
        self.streams.clear();
        self.events.clear();
    }

    // =================================================================
    // Memory management
    // =================================================================

    /// Allocate device memory (`cudaMalloc`).
    ///
    /// Allocates GPU memory with HOST_VISIBLE flags so we can read/write
    /// data in our simulation. Returns a `CudaDevicePtr` handle.
    pub fn malloc(&mut self, size: usize) -> Result<CudaDevicePtr, String> {
        let buf_id = self.base.allocate_buffer(size)?;
        let buf = self.base.device.memory_manager().get_buffer(buf_id)?;
        Ok(CudaDevicePtr {
            buffer_id: buf_id,
            device_address: buf.device_address,
            size,
        })
    }

    /// Allocate unified/managed memory (`cudaMallocManaged`).
    ///
    /// In our simulator, this is identical to `malloc()` since all our
    /// memory is already HOST_VISIBLE.
    pub fn malloc_managed(&mut self, size: usize) -> Result<CudaDevicePtr, String> {
        self.malloc(size)
    }

    /// Free device memory (`cudaFree`).
    pub fn free(&mut self, ptr: CudaDevicePtr) -> Result<(), String> {
        self.base.device.memory_manager_mut().free(ptr.buffer_id)
    }

    /// Copy data from host (CPU) to device (GPU).
    ///
    /// Maps the device buffer, writes the host data, and unmaps.
    pub fn memcpy_host_to_device(
        &mut self,
        dst: CudaDevicePtr,
        src: &[u8],
    ) -> Result<(), String> {
        let mm = self.base.device.memory_manager_mut();
        {
            let mut mapped = mm.map(dst.buffer_id)?;
            let copy_size = src.len().min(dst.size);
            mapped.write(0, &src[..copy_size])?;
        }
        mm.unmap(dst.buffer_id)?;
        Ok(())
    }

    /// Copy data from device (GPU) to host (CPU).
    ///
    /// Invalidates the buffer to get latest GPU data, then reads it.
    pub fn memcpy_device_to_host(
        &mut self,
        dst: &mut [u8],
        src: CudaDevicePtr,
    ) -> Result<(), String> {
        let mm = self.base.device.memory_manager_mut();
        mm.invalidate(src.buffer_id, 0, 0)?;
        {
            let mapped = mm.map(src.buffer_id)?;
            let copy_size = dst.len().min(src.size);
            let data = mapped.read(0, copy_size)?;
            dst[..copy_size].copy_from_slice(&data);
        }
        mm.unmap(src.buffer_id)?;
        Ok(())
    }

    /// Copy data between two device buffers.
    ///
    /// Uses a command buffer with `cmd_copy_buffer`.
    pub fn memcpy_device_to_device(
        &mut self,
        dst: CudaDevicePtr,
        src: CudaDevicePtr,
        size: usize,
    ) -> Result<(), String> {
        let src_id = src.buffer_id;
        let dst_id = dst.buffer_id;
        self.base.create_and_submit_cb(|cb| {
            cb.cmd_copy_buffer(src_id, dst_id, size, 0, 0)
        })
    }

    /// Copy data between host buffers (no GPU involvement).
    pub fn memcpy_host_to_host(dst: &mut [u8], src: &[u8], size: usize) {
        let copy_size = size.min(dst.len()).min(src.len());
        dst[..copy_size].copy_from_slice(&src[..copy_size]);
    }

    /// Generalized memcpy with direction enum.
    pub fn memcpy(
        &mut self,
        dst_ptr: CudaDevicePtr,
        dst_host: Option<&mut [u8]>,
        src_ptr: CudaDevicePtr,
        src_host: Option<&[u8]>,
        size: usize,
        kind: CudaMemcpyKind,
    ) -> Result<(), String> {
        match kind {
            CudaMemcpyKind::HostToDevice => {
                let src_data = src_host.ok_or("src_host required for HostToDevice")?;
                self.memcpy_host_to_device(dst_ptr, &src_data[..size])
            }
            CudaMemcpyKind::DeviceToHost => {
                let dst_data = dst_host.ok_or("dst_host required for DeviceToHost")?;
                self.memcpy_device_to_host(&mut dst_data[..size], src_ptr)
            }
            CudaMemcpyKind::DeviceToDevice => {
                self.memcpy_device_to_device(dst_ptr, src_ptr, size)
            }
            CudaMemcpyKind::HostToHost => {
                let src_data = src_host.ok_or("src_host required for HostToHost")?;
                let dst_data = dst_host.ok_or("dst_host required for HostToHost")?;
                Self::memcpy_host_to_host(dst_data, src_data, size);
                Ok(())
            }
        }
    }

    /// Set device memory to a value (`cudaMemset`).
    ///
    /// Fills the first `size` bytes with `value`.
    pub fn memset(
        &mut self,
        ptr: CudaDevicePtr,
        value: u8,
        size: usize,
    ) -> Result<(), String> {
        let buf_id = ptr.buffer_id;
        self.base.create_and_submit_cb(|cb| {
            cb.cmd_fill_buffer(buf_id, value, 0, size)
        })
    }

    // =================================================================
    // Kernel launch -- the heart of CUDA
    // =================================================================

    /// Launch a CUDA kernel (the `<<<grid, block>>>` operator).
    ///
    /// # What Happens Internally
    ///
    /// This single call hides the entire Vulkan-style pipeline:
    ///
    /// 1. Create a ShaderModule from the kernel's code, with the
    ///    block dimensions as the local workgroup size.
    /// 2. Create a DescriptorSetLayout and PipelineLayout.
    /// 3. Create a Pipeline binding the shader to the layout.
    /// 4. Create a DescriptorSet and bind the argument buffers.
    /// 5. Create a CommandBuffer.
    /// 6. Record: bind_pipeline -> bind_descriptor_set -> dispatch.
    /// 7. Submit to the queue.
    /// 8. Wait for completion.
    pub fn launch_kernel(
        &mut self,
        kernel: &CudaKernel,
        grid: Dim3,
        block: Dim3,
        args: &[CudaDevicePtr],
    ) -> Result<(), String> {
        let shader = self.base.device.create_shader_module(
            kernel.code.clone(),
            "",
            "main",
            (block.x, block.y, block.z),
        );

        let binding_indices: Vec<usize> = (0..args.len()).collect();
        let descriptor_bindings: Vec<DescriptorBinding> = binding_indices
            .iter()
            .map(|&b| DescriptorBinding::new(b))
            .collect();

        let ds_layout = self
            .base
            .device
            .create_descriptor_set_layout(descriptor_bindings);
        let pl_layout = self
            .base
            .device
            .create_pipeline_layout(vec![ds_layout.clone()], 0);
        let pipeline_id = self
            .base
            .device
            .create_compute_pipeline(shader, pl_layout);

        let mut ds = self.base.device.create_descriptor_set(ds_layout);
        for (i, arg) in args.iter().enumerate() {
            ds.write(i, arg.buffer_id)?;
        }
        let ds_id = ds.set_id();

        let gx = grid.x;
        let gy = grid.y;
        let gz = grid.z;
        self.base.create_and_submit_cb(move |cb| {
            cb.cmd_bind_pipeline(pipeline_id)?;
            cb.cmd_bind_descriptor_set(ds_id)?;
            cb.cmd_dispatch(gx, gy, gz)
        })
    }

    // =================================================================
    // Streams
    // =================================================================

    /// Create a new CUDA stream (`cudaStreamCreate`).
    pub fn create_stream(&mut self) -> CudaStream {
        let stream = CudaStream {
            id: self.next_stream_id,
        };
        self.next_stream_id += 1;
        self.streams.push(stream);
        stream
    }

    /// Destroy a CUDA stream (`cudaStreamDestroy`).
    pub fn destroy_stream(&mut self, stream: CudaStream) -> Result<(), String> {
        if let Some(pos) = self.streams.iter().position(|s| s.id == stream.id) {
            self.streams.remove(pos);
            Ok(())
        } else {
            Err("Stream not found or already destroyed".to_string())
        }
    }

    /// Wait for all operations in a stream (`cudaStreamSynchronize`).
    ///
    /// In our synchronous simulator, this is effectively a no-op since
    /// all operations complete immediately.
    pub fn stream_synchronize(&self, _stream: CudaStream) {
        self.base.device.wait_idle();
    }

    // =================================================================
    // Events (for GPU timing)
    // =================================================================

    /// Create a CUDA event (`cudaEventCreate`).
    pub fn create_event(&mut self) -> usize {
        let event = CudaEvent {
            timestamp: 0,
            recorded: false,
            signaled: false,
        };
        self.events.push(event);
        self.events.len() - 1
    }

    /// Record an event (`cudaEventRecord`).
    pub fn record_event(&mut self, event_idx: usize) -> Result<(), String> {
        if event_idx >= self.events.len() {
            return Err(format!("Invalid event index {}", event_idx));
        }
        self.events[event_idx].recorded = true;
        self.events[event_idx].signaled = true;
        // Timestamp from the first compute queue's total_cycles
        let cycles: u64 = self
            .base
            .device
            .queues()
            .get("compute")
            .and_then(|qs: &Vec<compute_runtime::command_queue::CommandQueue>| qs.first())
            .map(|q: &compute_runtime::command_queue::CommandQueue| q.total_cycles())
            .unwrap_or(0);
        self.events[event_idx].timestamp = cycles;
        Ok(())
    }

    /// Wait for an event to complete (`cudaEventSynchronize`).
    pub fn synchronize_event(&self, event_idx: usize) -> Result<(), String> {
        if event_idx >= self.events.len() {
            return Err(format!("Invalid event index {}", event_idx));
        }
        if !self.events[event_idx].recorded {
            return Err("Event was never recorded".to_string());
        }
        Ok(())
    }

    /// Measure elapsed time between two events (`cudaEventElapsedTime`).
    ///
    /// Returns the time in milliseconds (simulated from cycle counts).
    pub fn elapsed_time(&self, start: usize, end: usize) -> Result<f64, String> {
        if start >= self.events.len() || end >= self.events.len() {
            return Err("Invalid event index".to_string());
        }
        if !self.events[start].recorded {
            return Err("Start event was never recorded".to_string());
        }
        if !self.events[end].recorded {
            return Err("End event was never recorded".to_string());
        }
        let cycles = self.events[end].timestamp as f64 - self.events[start].timestamp as f64;
        Ok(cycles / 1_000_000.0) // 1 GHz -> 1 cycle = 1 ns = 0.000001 ms
    }

    /// Get the number of available devices.
    pub fn device_count(&self) -> usize {
        self.base.instance.enumerate_physical_devices().len()
    }
}
