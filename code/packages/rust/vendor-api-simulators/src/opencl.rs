//! OpenCL Runtime Simulator -- cross-platform "portable compute" model.
//!
//! # What is OpenCL?
//!
//! OpenCL (Open Computing Language) is the Khronos Group's cross-platform
//! compute API. Unlike CUDA (NVIDIA only), OpenCL runs on any vendor's GPU,
//! and even on CPUs and FPGAs.
//!
//! # The OpenCL Object Hierarchy
//!
//! ```text
//! CLPlatform          "Which vendor's implementation?"
//!     +-- CLDevice    "Which specific GPU/CPU?"
//! CLContext            "A group of devices I want to use together"
//!     +-- CLBuffer     "Memory on one of the context's devices"
//!     +-- CLProgram    "Source code, not yet compiled"
//!     |   +-- CLKernel "Compiled function, ready to dispatch"
//!     +-- CLCommandQueue "Where I enqueue operations"
//!             +-- CLEvent "Dependency token for operation ordering"
//! ```
//!
//! # Event-Based Dependencies
//!
//! OpenCL's most distinctive feature is its event model. Every enqueue
//! operation returns a `CLEvent`. You can pass event lists to subsequent
//! operations to create dependency chains:
//!
//! ```text
//! ev1 = queue.enqueue_write_buffer(buf_x, data_x)
//! ev2 = queue.enqueue_write_buffer(buf_y, data_y)
//! ev3 = queue.enqueue_nd_range_kernel(kernel, wait_list=[ev1, ev2])
//! ev4 = queue.enqueue_read_buffer(buf_y, wait_list=[ev3])
//! ```

use std::collections::HashMap;

use compute_runtime::protocols::DescriptorBinding;
use gpu_core::Instruction;

use crate::base::BaseSimulator;

// =========================================================================
// OpenCL enums and flags
// =========================================================================

/// OpenCL device types for filtering during discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClDeviceType {
    Gpu,
    Cpu,
    Accelerator,
    All,
}

/// OpenCL memory flags -- simpler than Vulkan's memory types.
///
/// - `READ_WRITE`:    Default. GPU can read and write this buffer.
/// - `READ_ONLY`:     GPU can only read. Allows compiler optimization.
/// - `WRITE_ONLY`:    GPU can only write. Allows compiler optimization.
/// - `COPY_HOST_PTR`: Initialize buffer contents from provided host data.
/// - `ALLOC_HOST_PTR`: Allocate in host-visible memory for CPU access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClMemFlags {
    ReadWrite,
    ReadOnly,
    WriteOnly,
    CopyHostPtr,
    AllocHostPtr,
}

/// Build status of a CLProgram.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClBuildStatus {
    Success,
    Error,
    InProgress,
    None,
}

/// Status of an OpenCL event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClEventStatus {
    Queued,
    Submitted,
    Running,
    Complete,
}

// =========================================================================
// CLEvent -- dependency token
// =========================================================================

/// An OpenCL event -- a dependency token for operation ordering.
///
/// Every enqueue operation returns a CLEvent. You can:
/// - Wait on it (blocking the CPU)
/// - Pass it in `wait_list` to another operation (GPU-side dependency)
/// - Query its status
pub struct ClEvent {
    signaled: bool,
}

impl ClEvent {
    fn new(signaled: bool) -> Self {
        Self { signaled }
    }

    /// Block until this event completes.
    pub fn wait(&self) {
        // In our synchronous simulator, events are already complete.
    }

    /// Query the current status of this event.
    pub fn status(&self) -> ClEventStatus {
        if self.signaled {
            ClEventStatus::Complete
        } else {
            ClEventStatus::Queued
        }
    }
}

// =========================================================================
// CLDevice -- wraps PhysicalDevice
// =========================================================================

/// An OpenCL device -- a specific piece of hardware.
///
/// Wraps metadata about a physical device with OpenCL-style property queries.
#[derive(Debug, Clone)]
pub struct ClDevice {
    pub name: String,
    pub device_type: ClDeviceType,
    pub vendor: String,
    pub max_compute_units: usize,
    pub max_work_group_size: usize,
    pub global_mem_size: u64,
}

// =========================================================================
// CLBuffer -- wraps Buffer
// =========================================================================

/// An OpenCL buffer -- memory allocated on a device.
pub struct ClBuffer {
    pub buffer_id: usize,
    pub size: usize,
    pub flags: ClMemFlags,
}

// =========================================================================
// CLKernel -- a compiled kernel function
// =========================================================================

/// An OpenCL kernel -- a compiled function extracted from a CLProgram.
///
/// In OpenCL, kernel arguments are set one at a time with `set_arg()`.
pub struct ClKernel {
    pub name: String,
    pub code: Option<Vec<Instruction>>,
    args: HashMap<usize, ClKernelArg>,
}

/// A kernel argument -- either a buffer or a scalar.
#[allow(dead_code)]
enum ClKernelArg {
    Buffer(usize), // buffer_id
    Scalar(Vec<u8>),
}

impl ClKernel {
    fn new(name: &str, code: Option<Vec<Instruction>>) -> Self {
        Self {
            name: name.to_string(),
            code,
            args: HashMap::new(),
        }
    }

    /// Set a buffer argument at the given index.
    pub fn set_arg_buffer(&mut self, index: usize, buffer: &ClBuffer) {
        self.args.insert(index, ClKernelArg::Buffer(buffer.buffer_id));
    }

    /// Set a scalar argument at the given index.
    pub fn set_arg_scalar(&mut self, index: usize, data: &[u8]) {
        self.args.insert(index, ClKernelArg::Scalar(data.to_vec()));
    }

    /// Get buffer IDs of all buffer arguments, sorted by index.
    fn buffer_args_sorted(&self) -> Vec<(usize, usize)> {
        let mut result: Vec<(usize, usize)> = self
            .args
            .iter()
            .filter_map(|(&idx, arg)| match arg {
                ClKernelArg::Buffer(buf_id) => Some((idx, *buf_id)),
                _ => None,
            })
            .collect();
        result.sort_by_key(|(idx, _)| *idx);
        result
    }
}

// =========================================================================
// CLProgram -- source code + compilation
// =========================================================================

/// An OpenCL program -- source code that can be compiled for a device.
///
/// OpenCL uses runtime compilation: you provide kernel source as a string,
/// call `build()`, and the runtime compiles it for the target device.
pub struct ClProgram {
    pub source: String,
    pub build_status: ClBuildStatus,
    kernels: HashMap<String, Option<Vec<Instruction>>>,
}

impl ClProgram {
    fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
            build_status: ClBuildStatus::None,
            kernels: HashMap::new(),
        }
    }

    /// Compile the program. In our simulator, just marks it as built.
    pub fn build(&mut self) {
        self.build_status = ClBuildStatus::Success;
    }

    /// Register kernel code for a named kernel.
    pub fn register_kernel(&mut self, name: &str, code: Option<Vec<Instruction>>) {
        self.kernels.insert(name.to_string(), code);
    }

    /// Extract a kernel function from the compiled program.
    pub fn create_kernel(&self, name: &str) -> Result<ClKernel, String> {
        if self.build_status != ClBuildStatus::Success {
            return Err(format!(
                "Program not built (status: {:?}). Call program.build() first.",
                self.build_status
            ));
        }
        let code = self.kernels.get(name).cloned().flatten();
        Ok(ClKernel::new(name, code))
    }
}

// =========================================================================
// CLCommandQueue -- enqueue operations with event dependencies
// =========================================================================

/// An OpenCL command queue -- where operations are enqueued.
///
/// Every operation returns a CLEvent for dependency tracking.
pub struct ClCommandQueue<'a> {
    context: &'a mut ClContext,
}

impl<'a> ClCommandQueue<'a> {
    /// Enqueue a kernel for execution (`clEnqueueNDRangeKernel`).
    ///
    /// The `global_size` specifies total work items. If `local_size` is None,
    /// the OpenCL runtime picks an optimal workgroup size.
    pub fn enqueue_nd_range_kernel(
        &mut self,
        kernel: &ClKernel,
        global_size: &[usize],
        local_size: Option<&[usize]>,
        _wait_list: &[&ClEvent],
    ) -> Result<ClEvent, String> {
        // Determine local size (workgroup size)
        let local = match local_size {
            Some(ls) => (
                ls[0],
                if ls.len() > 1 { ls[1] } else { 1 },
                if ls.len() > 2 { ls[2] } else { 1 },
            ),
            None => (32, 1, 1),
        };

        // Calculate grid dimensions
        let grid_x = (global_size[0] + local.0 - 1) / local.0;
        let grid_y = if global_size.len() > 1 {
            (global_size[1] + local.1 - 1) / local.1
        } else {
            1
        };
        let grid_z = if global_size.len() > 2 {
            (global_size[2] + local.2 - 1) / local.2
        } else {
            1
        };

        // Build pipeline from kernel
        let shader = self.context.base.device.create_shader_module(
            kernel.code.clone(),
            "",
            "main",
            local,
        );

        let buffer_args = kernel.buffer_args_sorted();
        let descriptor_bindings: Vec<DescriptorBinding> = buffer_args
            .iter()
            .map(|(idx, _)| DescriptorBinding::new(*idx))
            .collect();

        let ds_layout = self
            .context
            .base
            .device
            .create_descriptor_set_layout(descriptor_bindings);
        let pl_layout = self
            .context
            .base
            .device
            .create_pipeline_layout(vec![ds_layout.clone()], 0);
        let pipeline_id = self
            .context
            .base
            .device
            .create_compute_pipeline(shader, pl_layout);

        let mut ds = self.context.base.device.create_descriptor_set(ds_layout);
        for (idx, buf_id) in &buffer_args {
            ds.write(*idx, *buf_id)?;
        }
        let ds_id = ds.set_id();

        self.context.base.create_and_submit_cb(move |cb| {
            cb.cmd_bind_pipeline(pipeline_id)?;
            cb.cmd_bind_descriptor_set(ds_id)?;
            cb.cmd_dispatch(grid_x, grid_y, grid_z)
        })?;

        Ok(ClEvent::new(true))
    }

    /// Write host data to a device buffer (`clEnqueueWriteBuffer`).
    pub fn enqueue_write_buffer(
        &mut self,
        buffer: &ClBuffer,
        offset: usize,
        data: &[u8],
        _wait_list: &[&ClEvent],
    ) -> Result<ClEvent, String> {
        let mm = self.context.base.device.memory_manager_mut();
        {
            let mut mapped = mm.map(buffer.buffer_id)?;
            mapped.write(offset, data)?;
        }
        mm.unmap(buffer.buffer_id)?;
        Ok(ClEvent::new(true))
    }

    /// Read device buffer data to host memory (`clEnqueueReadBuffer`).
    pub fn enqueue_read_buffer(
        &mut self,
        buffer: &ClBuffer,
        offset: usize,
        size: usize,
        host_ptr: &mut [u8],
        _wait_list: &[&ClEvent],
    ) -> Result<ClEvent, String> {
        let mm = self.context.base.device.memory_manager_mut();
        mm.invalidate(buffer.buffer_id, 0, 0)?;
        {
            let mapped = mm.map(buffer.buffer_id)?;
            let data = mapped.read(offset, size)?;
            host_ptr[..size].copy_from_slice(&data);
        }
        mm.unmap(buffer.buffer_id)?;
        Ok(ClEvent::new(true))
    }

    /// Copy between two device buffers (`clEnqueueCopyBuffer`).
    pub fn enqueue_copy_buffer(
        &mut self,
        src: &ClBuffer,
        dst: &ClBuffer,
        size: usize,
        _wait_list: &[&ClEvent],
    ) -> Result<ClEvent, String> {
        let src_id = src.buffer_id;
        let dst_id = dst.buffer_id;
        self.context.base.create_and_submit_cb(|cb| {
            cb.cmd_copy_buffer(src_id, dst_id, size, 0, 0)
        })?;
        Ok(ClEvent::new(true))
    }

    /// Fill a buffer with a pattern (`clEnqueueFillBuffer`).
    pub fn enqueue_fill_buffer(
        &mut self,
        buffer: &ClBuffer,
        pattern: u8,
        offset: usize,
        size: usize,
    ) -> Result<ClEvent, String> {
        let buf_id = buffer.buffer_id;
        self.context.base.create_and_submit_cb(|cb| {
            cb.cmd_fill_buffer(buf_id, pattern, offset, size)
        })?;
        Ok(ClEvent::new(true))
    }

    /// Block until all enqueued operations complete (`clFinish`).
    pub fn finish(&self) {
        self.context.base.device.wait_idle();
    }

    /// Ensure all enqueued operations are submitted (`clFlush`).
    /// In our synchronous simulator, this is a no-op.
    pub fn flush(&self) {}
}

// =========================================================================
// CLContext -- the OpenCL execution context
// =========================================================================

/// An OpenCL context -- groups devices and manages shared resources.
///
/// In OpenCL, a context is the scope for resource sharing. Buffers and
/// programs are created within a context and can be used on any device
/// in that context.
pub struct ClContext {
    base: BaseSimulator,
    devices: Vec<ClDevice>,
}

impl ClContext {
    /// Create a new OpenCL context. Uses the first available device.
    pub fn new() -> Result<Self, String> {
        let base = BaseSimulator::new(None, None)?;
        let devices = Self::enumerate_devices(&base);
        Ok(Self { base, devices })
    }

    /// Create a context preferring a specific vendor.
    pub fn with_vendor(vendor: &str) -> Result<Self, String> {
        let base = BaseSimulator::new(None, Some(vendor))?;
        let devices = Self::enumerate_devices(&base);
        Ok(Self { base, devices })
    }

    fn enumerate_devices(base: &BaseSimulator) -> Vec<ClDevice> {
        base.instance
            .enumerate_physical_devices()
            .iter()
            .map(|pd| {
                let mem_size: u64 = pd.memory_properties().heaps.iter().map(|h| h.size).sum();
                let dt = match pd.device_type() {
                    compute_runtime::protocols::DeviceType::GPU => ClDeviceType::Gpu,
                    compute_runtime::protocols::DeviceType::TPU => ClDeviceType::Accelerator,
                    compute_runtime::protocols::DeviceType::NPU => ClDeviceType::Accelerator,
                };
                ClDevice {
                    name: pd.name().to_string(),
                    device_type: dt,
                    vendor: pd.vendor().to_string(),
                    max_compute_units: 4,
                    max_work_group_size: pd.limits().max_workgroup_size.0,
                    global_mem_size: mem_size,
                }
            })
            .collect()
    }

    /// Get available devices.
    pub fn devices(&self) -> &[ClDevice] {
        &self.devices
    }

    /// Create a device buffer (`clCreateBuffer`).
    pub fn create_buffer(
        &mut self,
        flags: ClMemFlags,
        size: usize,
        host_ptr: Option<&[u8]>,
    ) -> Result<ClBuffer, String> {
        let buf_id = self.base.allocate_buffer(size)?;

        // If we have initial data, write it
        if let Some(data) = host_ptr {
            if flags == ClMemFlags::CopyHostPtr || flags == ClMemFlags::ReadWrite {
                let mm = self.base.device.memory_manager_mut();
                {
                    let mut mapped = mm.map(buf_id)?;
                    let copy_size = data.len().min(size);
                    mapped.write(0, &data[..copy_size])?;
                }
                mm.unmap(buf_id)?;
            }
        }

        Ok(ClBuffer {
            buffer_id: buf_id,
            size,
            flags,
        })
    }

    /// Create a program from source code (`clCreateProgramWithSource`).
    pub fn create_program_with_source(&self, source: &str) -> ClProgram {
        ClProgram::new(source)
    }

    /// Create a command queue for this context.
    pub fn create_command_queue(&mut self) -> ClCommandQueue<'_> {
        ClCommandQueue { context: self }
    }

    /// Free a buffer.
    pub fn release_buffer(&mut self, buffer: ClBuffer) -> Result<(), String> {
        self.base.device.memory_manager_mut().free(buffer.buffer_id)
    }
}

// =========================================================================
// CLPlatform -- the top-level discovery object
// =========================================================================

/// An OpenCL platform -- represents a vendor's OpenCL implementation.
pub struct ClPlatform {
    pub name: String,
    pub vendor: String,
    pub version: String,
    devices: Vec<ClDevice>,
}

impl ClPlatform {
    /// Enumerate available OpenCL platforms (`clGetPlatformIDs`).
    ///
    /// Returns a list with one platform wrapping our Layer 5 runtime.
    pub fn get_platforms() -> Vec<ClPlatform> {
        let base = BaseSimulator::new(None, None).expect("Failed to create base simulator");
        let devices = ClContext::enumerate_devices(&base);
        vec![ClPlatform {
            name: "Coding Adventures Compute Platform".to_string(),
            vendor: "Coding Adventures".to_string(),
            version: "OpenCL 3.0".to_string(),
            devices,
        }]
    }

    /// Get devices matching a specific type.
    pub fn get_devices(&self, device_type: ClDeviceType) -> Vec<&ClDevice> {
        if device_type == ClDeviceType::All {
            self.devices.iter().collect()
        } else {
            self.devices
                .iter()
                .filter(|d| d.device_type == device_type)
                .collect()
        }
    }
}
