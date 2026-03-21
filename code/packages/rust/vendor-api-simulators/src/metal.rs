//! Metal Runtime Simulator -- Apple's unified memory GPU programming model.
//!
//! # What is Metal?
//!
//! Metal is Apple's GPU API, designed exclusively for Apple hardware (macOS,
//! iOS, iPadOS, tvOS). Its key innovation is **unified memory** -- on Apple
//! Silicon (M1/M2/M3/M4), the CPU and GPU share the same physical RAM. This
//! eliminates the host-to-device copies that CUDA and OpenCL require.
//!
//! # The Command Encoder Model
//!
//! Metal uses a distinctive pattern for recording GPU commands:
//!
//! 1. Get a command buffer from the command queue
//! 2. Create a **command encoder** (compute, blit, render)
//! 3. Record commands into the encoder
//! 4. End the encoder
//! 5. Commit the command buffer
//!
//! The encoder adds a layer of scoping that Vulkan doesn't have. This makes
//! it clear what type of commands are being recorded.
//!
//! # Unified Memory
//!
//! On Apple Silicon, all memory is both CPU-accessible and GPU-accessible:
//!
//! ```text
//! CUDA:   cudaMalloc -> device-only, need cudaMemcpy to access from CPU
//! Metal:  make_buffer -> unified, buffer.contents() gives CPU access directly
//! ```

use compute_runtime::protocols::DescriptorBinding;
use gpu_core::Instruction;

use crate::base::BaseSimulator;

// =========================================================================
// Metal-specific types
// =========================================================================

/// MTLSize -- grid/threadgroup dimensions in Metal.
///
/// Metal uses (width, height, depth) instead of (x, y, z). Same concept,
/// different naming -- Apple convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MtlSize {
    pub width: usize,
    pub height: usize,
    pub depth: usize,
}

impl MtlSize {
    pub fn new(width: usize, height: usize, depth: usize) -> Self {
        Self { width, height, depth }
    }
}

/// Metal storage mode options for buffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MtlResourceOptions {
    /// CPU + GPU access (default on Apple Silicon).
    StorageModeShared,
    /// GPU-only access.
    StorageModePrivate,
    /// CPU + GPU with explicit synchronization (macOS only).
    StorageModeManaged,
}

/// Status of a Metal command buffer in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MtlCommandBufferStatus {
    NotEnqueued,
    Enqueued,
    Committed,
    Scheduled,
    Completed,
    Error,
}

// =========================================================================
// MTLBuffer -- unified memory buffer
// =========================================================================

/// A Metal buffer -- always accessible from both CPU and GPU.
///
/// Because Apple Silicon uses unified memory, you can:
///
/// ```text
/// let buf = device.make_buffer(1024);
/// buf.write_bytes(&data);           // CPU writes directly
/// // ... GPU computes on buf ...
/// let result = buf.contents();      // CPU reads directly
/// ```
///
/// No staging buffers, no memcpy, no map/unmap ceremony.
pub struct MtlBuffer {
    pub buffer_id: usize,
    pub length: usize,
}

// =========================================================================
// MTLFunction and MTLLibrary -- shader management
// =========================================================================

/// A Metal shader function extracted from a library.
pub struct MtlFunction {
    pub name: String,
    pub code: Option<Vec<Instruction>>,
}

/// A Metal shader library -- a collection of compiled functions.
pub struct MtlLibrary {
    pub source: String,
    functions: std::collections::HashMap<String, Option<Vec<Instruction>>>,
}

impl MtlLibrary {
    fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
            functions: std::collections::HashMap::new(),
        }
    }

    /// Register a function with optional GPU code.
    pub fn register_function(&mut self, name: &str, code: Option<Vec<Instruction>>) {
        self.functions.insert(name.to_string(), code);
    }

    /// Extract a function from the library by name.
    pub fn make_function(&self, name: &str) -> MtlFunction {
        let code = self.functions.get(name).cloned().flatten();
        MtlFunction {
            name: name.to_string(),
            code,
        }
    }
}

// =========================================================================
// MTLComputePipelineState -- compiled compute pipeline
// =========================================================================

/// A compiled Metal compute pipeline state.
///
/// In Metal, a pipeline state object (PSO) encapsulates the compiled
/// kernel function ready for dispatch.
pub struct MtlComputePipelineState {
    pub function_name: String,
    pub code: Option<Vec<Instruction>>,
    pipeline_id: usize,
}

// =========================================================================
// MTLComputeCommandEncoder -- records compute commands
// =========================================================================

/// A Metal compute command encoder -- records compute commands.
///
/// Instead of recording commands directly into a command buffer (Vulkan
/// style), Metal uses typed encoders that scope commands by type:
///
/// ```text
/// encoder = cb.make_compute_command_encoder();
/// encoder.set_compute_pipeline_state(pso);
/// encoder.set_buffer(buf_x, 0, 0);
/// encoder.dispatch_threadgroups(groups, threads);
/// encoder.end_encoding();
/// ```
pub struct MtlComputeCommandEncoder {
    pipeline_state: Option<MtlComputePipelineStateRef>,
    buffers: std::collections::HashMap<usize, usize>, // index -> buffer_id
    ended: bool,
}

impl MtlComputeCommandEncoder {
    /// Check if the encoder has been ended.
    pub fn is_ended(&self) -> bool {
        self.ended
    }
}

/// Reference to a pipeline state for the encoder.
#[allow(dead_code)]
struct MtlComputePipelineStateRef {
    code: Option<Vec<Instruction>>,
    pipeline_id: usize,
}

impl MtlComputeCommandEncoder {
    fn new() -> Self {
        Self {
            pipeline_state: None,
            buffers: std::collections::HashMap::new(),
            ended: false,
        }
    }

    /// Set which compute pipeline to use for dispatches.
    pub fn set_compute_pipeline_state(&mut self, pso: &MtlComputePipelineState) {
        self.pipeline_state = Some(MtlComputePipelineStateRef {
            code: pso.code.clone(),
            pipeline_id: pso.pipeline_id,
        });
    }

    /// Bind a buffer to an argument index.
    pub fn set_buffer(&mut self, buffer: &MtlBuffer, _offset: usize, index: usize) {
        self.buffers.insert(index, buffer.buffer_id);
    }

    /// End recording into this encoder.
    pub fn end_encoding(&mut self) {
        self.ended = true;
    }
}

// =========================================================================
// MTLBlitCommandEncoder -- records data transfer commands
// =========================================================================

/// A Metal blit command encoder -- records copy/fill operations.
///
/// "Blit" stands for "block image transfer."
pub struct MtlBlitCommandEncoder {
    copies: Vec<BlitCopy>,
    fills: Vec<BlitFill>,
    ended: bool,
}

impl MtlBlitCommandEncoder {
    /// Check if the encoder has been ended.
    pub fn is_ended(&self) -> bool {
        self.ended
    }
}

struct BlitCopy {
    src_id: usize,
    dst_id: usize,
    size: usize,
    src_offset: usize,
    dst_offset: usize,
}

struct BlitFill {
    buffer_id: usize,
    value: u8,
    offset: usize,
    size: usize,
}

impl MtlBlitCommandEncoder {
    fn new() -> Self {
        Self {
            copies: Vec::new(),
            fills: Vec::new(),
            ended: false,
        }
    }

    /// Copy data between buffers.
    pub fn copy_from_buffer(
        &mut self,
        src: &MtlBuffer,
        src_offset: usize,
        to_buffer: &MtlBuffer,
        dst_offset: usize,
        size: usize,
    ) {
        self.copies.push(BlitCopy {
            src_id: src.buffer_id,
            dst_id: to_buffer.buffer_id,
            size,
            src_offset,
            dst_offset,
        });
    }

    /// Fill a buffer region with a byte value.
    pub fn fill_buffer(&mut self, buffer: &MtlBuffer, value: u8, offset: usize, size: usize) {
        self.fills.push(BlitFill {
            buffer_id: buffer.buffer_id,
            value,
            offset,
            size,
        });
    }

    /// End recording into this blit encoder.
    pub fn end_encoding(&mut self) {
        self.ended = true;
    }
}

// =========================================================================
// MTLDevice -- the main Metal device object
// =========================================================================

/// A Metal device -- the main entry point for Metal programming.
///
/// # Apple's Simplified Model
///
/// In Vulkan, you have PhysicalDevice (read-only) and LogicalDevice (usable).
/// In Metal, there's just MTLDevice -- it's both. You get properties AND
/// create resources from it.
///
/// Metal always uses unified memory. All buffers are CPU-accessible
/// (storageModeShared by default), so there's no need for staging buffers.
///
/// # Usage
///
/// ```text
/// let mut device = MtlDevice::new().unwrap();
/// let queue = device.make_command_queue();
/// let buf = device.make_buffer(1024);
/// device.write_buffer(&buf, &data);
///
/// let library = device.make_library("shader_source");
/// let function = library.make_function("compute_fn");
/// let pso = device.make_compute_pipeline_state(&function);
///
/// // Dispatch via command buffer + encoder
/// ```
pub struct MtlDevice {
    base: BaseSimulator,
}

impl MtlDevice {
    /// Create a Metal device, preferring Apple hardware.
    pub fn new() -> Result<Self, String> {
        let base = BaseSimulator::new(None, Some("apple"))?;
        Ok(Self { base })
    }

    /// Device name.
    pub fn name(&self) -> String {
        let devices = self.base.instance.enumerate_physical_devices();
        devices[self.base.device_index].name().to_string()
    }

    /// Allocate a buffer on the device.
    ///
    /// All Metal buffers use unified memory by default.
    pub fn make_buffer(&mut self, length: usize) -> Result<MtlBuffer, String> {
        let buf_id = self.base.allocate_buffer(length)?;
        Ok(MtlBuffer {
            buffer_id: buf_id,
            length,
        })
    }

    /// Write bytes to a buffer from CPU side.
    pub fn write_buffer(&mut self, buffer: &MtlBuffer, data: &[u8]) -> Result<(), String> {
        let mm = self.base.device.memory_manager_mut();
        {
            let mut mapped = mm.map(buffer.buffer_id)?;
            let copy_size = data.len().min(buffer.length);
            mapped.write(0, &data[..copy_size])?;
        }
        mm.unmap(buffer.buffer_id)?;
        Ok(())
    }

    /// Read the contents of a buffer.
    pub fn read_buffer(&mut self, buffer: &MtlBuffer) -> Result<Vec<u8>, String> {
        let mm = self.base.device.memory_manager_mut();
        mm.invalidate(buffer.buffer_id, 0, 0)?;
        let data = {
            let mapped = mm.map(buffer.buffer_id)?;
            mapped.read(0, buffer.length)?
        };
        mm.unmap(buffer.buffer_id)?;
        Ok(data)
    }

    /// Create a shader library from source code.
    pub fn make_library(&self, source: &str) -> MtlLibrary {
        MtlLibrary::new(source)
    }

    /// Create a compute pipeline state from a shader function.
    pub fn make_compute_pipeline_state(
        &mut self,
        function: &MtlFunction,
    ) -> Result<MtlComputePipelineState, String> {
        let shader = self.base.device.create_shader_module(
            function.code.clone(),
            "",
            "main",
            (1, 1, 1),
        );
        let ds_layout = self.base.device.create_descriptor_set_layout(vec![]);
        let pl_layout = self
            .base
            .device
            .create_pipeline_layout(vec![ds_layout], 0);
        let pipeline_id = self.base.device.create_compute_pipeline(shader, pl_layout);

        Ok(MtlComputePipelineState {
            function_name: function.name.clone(),
            code: function.code.clone(),
            pipeline_id,
        })
    }

    /// Dispatch a compute kernel using the encoder model.
    ///
    /// This combines: create CB, create encoder, record, commit, wait.
    pub fn dispatch_threadgroups(
        &mut self,
        encoder: &MtlComputeCommandEncoder,
        threadgroups_per_grid: MtlSize,
        threads_per_threadgroup: MtlSize,
    ) -> Result<(), String> {
        let pso = encoder
            .pipeline_state
            .as_ref()
            .ok_or("No compute pipeline state set")?;

        // Create pipeline with correct local size
        let shader = self.base.device.create_shader_module(
            pso.code.clone(),
            "",
            "main",
            (
                threads_per_threadgroup.width,
                threads_per_threadgroup.height,
                threads_per_threadgroup.depth,
            ),
        );

        let sorted_indices: Vec<usize> = {
            let mut keys: Vec<usize> = encoder.buffers.keys().copied().collect();
            keys.sort();
            keys
        };

        let descriptor_bindings: Vec<DescriptorBinding> = sorted_indices
            .iter()
            .map(|&i| DescriptorBinding::new(i))
            .collect();

        let ds_layout = self
            .base
            .device
            .create_descriptor_set_layout(descriptor_bindings);
        let pl_layout = self
            .base
            .device
            .create_pipeline_layout(vec![ds_layout.clone()], 0);
        let pipeline_id = self.base.device.create_compute_pipeline(shader, pl_layout);

        let mut ds = self.base.device.create_descriptor_set(ds_layout);
        for &i in &sorted_indices {
            ds.write(i, encoder.buffers[&i])?;
        }
        let ds_id = ds.set_id();

        let gw = threadgroups_per_grid.width;
        let gh = threadgroups_per_grid.height;
        let gd = threadgroups_per_grid.depth;

        self.base.create_and_submit_cb(move |cb| {
            cb.cmd_bind_pipeline(pipeline_id)?;
            cb.cmd_bind_descriptor_set(ds_id)?;
            cb.cmd_dispatch(gw, gh, gd)
        })
    }

    /// Dispatch a blit (copy/fill) encoder.
    pub fn commit_blit_encoder(
        &mut self,
        encoder: &MtlBlitCommandEncoder,
    ) -> Result<(), String> {
        // Process copies
        for copy in &encoder.copies {
            let src_id = copy.src_id;
            let dst_id = copy.dst_id;
            let size = copy.size;
            let src_off = copy.src_offset;
            let dst_off = copy.dst_offset;
            self.base.create_and_submit_cb(|cb| {
                cb.cmd_copy_buffer(src_id, dst_id, size, src_off, dst_off)
            })?;
        }
        // Process fills
        for fill in &encoder.fills {
            let buf_id = fill.buffer_id;
            let value = fill.value;
            let offset = fill.offset;
            let size = fill.size;
            self.base.create_and_submit_cb(|cb| {
                cb.cmd_fill_buffer(buf_id, value, offset, size)
            })?;
        }
        Ok(())
    }

    /// Create a new compute command encoder.
    pub fn make_compute_command_encoder(&self) -> MtlComputeCommandEncoder {
        MtlComputeCommandEncoder::new()
    }

    /// Create a new blit command encoder.
    pub fn make_blit_command_encoder(&self) -> MtlBlitCommandEncoder {
        MtlBlitCommandEncoder::new()
    }

    /// Wait for all work to complete.
    pub fn wait_until_completed(&self) {
        self.base.device.wait_idle();
    }

    /// Free a buffer.
    pub fn release_buffer(&mut self, buffer: MtlBuffer) -> Result<(), String> {
        self.base.device.memory_manager_mut().free(buffer.buffer_id)
    }
}
