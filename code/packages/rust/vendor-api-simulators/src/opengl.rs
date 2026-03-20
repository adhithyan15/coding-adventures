//! OpenGL Compute Simulator -- the legacy global state machine.
//!
//! # What is OpenGL?
//!
//! OpenGL is the oldest surviving GPU API (1992). Compute shaders were bolted
//! on in OpenGL 4.3 (2012), long after the core API was designed around
//! graphics rendering. This heritage shows: OpenGL uses a **global state
//! machine** model where you bind things to "current" state and then issue
//! commands that operate on whatever is currently bound.
//!
//! # The State Machine Model
//!
//! Unlike Vulkan (explicit objects) or Metal (scoped encoders), OpenGL
//! maintains global state:
//!
//! ```text
//! gl.use_program(prog);                 // Sets "current program" globally
//! gl.bind_buffer_base(SSBO, 0, buf_a);  // Sets "buffer at binding 0"
//! gl.dispatch_compute(4, 1, 1);         // Uses WHATEVER is currently bound
//! ```
//!
//! # Integer Handles
//!
//! OpenGL uses integer handles (GLuint) for everything. You never get a
//! typed object -- just a number:
//!
//! ```text
//! let shader = gl.create_shader(GL_COMPUTE_SHADER);  // Returns 1
//! let program = gl.create_program();                  // Returns 2
//! let buffers = gl.gen_buffers(2);                   // Returns [3, 4]
//! ```
//!
//! These integers are IDs in internal lookup tables.

use std::collections::HashMap;

use compute_runtime::protocols::DescriptorBinding;
use gpu_core::Instruction;

use crate::base::BaseSimulator;

// =========================================================================
// OpenGL constants -- module-level, just like real OpenGL
// =========================================================================

/// Shader types.
pub const GL_COMPUTE_SHADER: u32 = 0x91B9;

/// Buffer targets.
pub const GL_SHADER_STORAGE_BUFFER: u32 = 0x90D2;
pub const GL_ARRAY_BUFFER: u32 = 0x8892;
pub const GL_UNIFORM_BUFFER: u32 = 0x8A11;

/// Buffer usage hints.
pub const GL_STATIC_DRAW: u32 = 0x88E4;
pub const GL_DYNAMIC_DRAW: u32 = 0x88E8;
pub const GL_STREAM_DRAW: u32 = 0x88E0;

/// Map access bits.
pub const GL_MAP_READ_BIT: u32 = 0x0001;
pub const GL_MAP_WRITE_BIT: u32 = 0x0002;

/// Memory barrier bits.
pub const GL_SHADER_STORAGE_BARRIER_BIT: u32 = 0x00002000;
pub const GL_BUFFER_UPDATE_BARRIER_BIT: u32 = 0x00000200;
pub const GL_ALL_BARRIER_BITS: u32 = 0xFFFFFFFF;

/// Sync object results.
pub const GL_ALREADY_SIGNALED: u32 = 0x911A;
pub const GL_CONDITION_SATISFIED: u32 = 0x911C;
pub const GL_TIMEOUT_EXPIRED: u32 = 0x911B;
pub const GL_WAIT_FAILED: u32 = 0x911D;

// =========================================================================
// Internal shader/program data
// =========================================================================

struct ShaderInfo {
    source: String,
    code: Option<Vec<Instruction>>,
    compiled: bool,
    #[allow(dead_code)]
    shader_type: u32,
}

struct ProgramInfo {
    shaders: Vec<u32>,
    linked: bool,
    pipeline_id: Option<usize>,
    shader_code: Option<Vec<Instruction>>,
}

// =========================================================================
// GLContext -- the main OpenGL state machine
// =========================================================================

/// OpenGL context -- a global state machine for GPU programming.
///
/// # The State Machine
///
/// GLContext maintains global state that commands operate on:
///
/// - `current_program`:  Which program is currently active (`use_program`)
/// - `bound_buffers`:    Which buffers are bound to which targets/indices
/// - `shaders`:          Map of GL handle -> shader data
/// - `programs`:         Map of GL handle -> pipeline
/// - `buffers`:          Map of GL handle -> Layer 5 Buffer ID
///
/// Every OpenGL call reads and/or modifies this global state.
pub struct GlContext {
    base: BaseSimulator,

    // === Global State ===
    current_program: Option<u32>,
    bound_buffers: HashMap<(u32, u32), u32>,    // (target, index) -> handle
    target_buffers: HashMap<u32, u32>,           // target -> handle

    // === Internal lookup tables ===
    shaders: HashMap<u32, ShaderInfo>,
    programs: HashMap<u32, ProgramInfo>,
    buffers: HashMap<u32, Option<usize>>,        // handle -> buffer_id
    syncs: HashMap<u32, bool>,                   // handle -> signaled
    uniforms: HashMap<(u32, String), f64>,

    next_id: u32,
}

impl GlContext {
    /// Create a new OpenGL context.
    pub fn new() -> Result<Self, String> {
        let base = BaseSimulator::new(None, None)?;
        Ok(Self {
            base,
            current_program: None,
            bound_buffers: HashMap::new(),
            target_buffers: HashMap::new(),
            shaders: HashMap::new(),
            programs: HashMap::new(),
            buffers: HashMap::new(),
            syncs: HashMap::new(),
            uniforms: HashMap::new(),
            next_id: 1,
        })
    }

    fn gen_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    // =================================================================
    // Shader management
    // =================================================================

    /// Create a shader object (`glCreateShader`).
    pub fn create_shader(&mut self, shader_type: u32) -> Result<u32, String> {
        if shader_type != GL_COMPUTE_SHADER {
            return Err(format!(
                "Only GL_COMPUTE_SHADER (0x{:04X}) is supported, got 0x{:04X}",
                GL_COMPUTE_SHADER, shader_type
            ));
        }
        let handle = self.gen_id();
        self.shaders.insert(
            handle,
            ShaderInfo {
                source: String::new(),
                code: None,
                compiled: false,
                shader_type,
            },
        );
        Ok(handle)
    }

    /// Set the source code for a shader (`glShaderSource`).
    pub fn shader_source(&mut self, shader: u32, source: &str) -> Result<(), String> {
        let info = self
            .shaders
            .get_mut(&shader)
            .ok_or(format!("Invalid shader handle {}", shader))?;
        info.source = source.to_string();
        Ok(())
    }

    /// Set GPU instruction code for a shader (simulator extension).
    pub fn shader_code(&mut self, shader: u32, code: Vec<Instruction>) -> Result<(), String> {
        let info = self
            .shaders
            .get_mut(&shader)
            .ok_or(format!("Invalid shader handle {}", shader))?;
        info.code = Some(code);
        Ok(())
    }

    /// Compile a shader (`glCompileShader`).
    pub fn compile_shader(&mut self, shader: u32) -> Result<(), String> {
        let info = self
            .shaders
            .get_mut(&shader)
            .ok_or(format!("Invalid shader handle {}", shader))?;
        info.compiled = true;
        Ok(())
    }

    /// Delete a shader object (`glDeleteShader`).
    pub fn delete_shader(&mut self, shader: u32) {
        self.shaders.remove(&shader);
    }

    // =================================================================
    // Program management
    // =================================================================

    /// Create a program object (`glCreateProgram`).
    pub fn create_program(&mut self) -> u32 {
        let handle = self.gen_id();
        self.programs.insert(
            handle,
            ProgramInfo {
                shaders: Vec::new(),
                linked: false,
                pipeline_id: None,
                shader_code: None,
            },
        );
        handle
    }

    /// Attach a shader to a program (`glAttachShader`).
    pub fn attach_shader(&mut self, program: u32, shader: u32) -> Result<(), String> {
        if !self.programs.contains_key(&program) {
            return Err(format!("Invalid program handle {}", program));
        }
        if !self.shaders.contains_key(&shader) {
            return Err(format!("Invalid shader handle {}", shader));
        }
        self.programs.get_mut(&program).unwrap().shaders.push(shader);
        Ok(())
    }

    /// Link a program (`glLinkProgram`).
    ///
    /// Creates the Layer 5 Pipeline from attached shaders.
    pub fn link_program(&mut self, program: u32) -> Result<(), String> {
        let prog = self
            .programs
            .get(&program)
            .ok_or(format!("Invalid program handle {}", program))?;

        if prog.shaders.is_empty() {
            return Err(format!("Program {} has no attached shaders", program));
        }

        let shader_handle = prog.shaders[0];
        let code = self
            .shaders
            .get(&shader_handle)
            .and_then(|s| s.code.clone());

        let shader = self
            .base
            .device
            .create_shader_module(code.clone(), "", "main", (1, 1, 1));
        let ds_layout = self.base.device.create_descriptor_set_layout(vec![]);
        let pl_layout = self
            .base
            .device
            .create_pipeline_layout(vec![ds_layout], 0);
        let pipeline_id = self.base.device.create_compute_pipeline(shader, pl_layout);

        let prog = self.programs.get_mut(&program).unwrap();
        prog.pipeline_id = Some(pipeline_id);
        prog.shader_code = code;
        prog.linked = true;
        Ok(())
    }

    /// Set the active program (`glUseProgram`).
    pub fn use_program(&mut self, program: u32) -> Result<(), String> {
        if program == 0 {
            self.current_program = None;
            return Ok(());
        }
        let prog = self
            .programs
            .get(&program)
            .ok_or(format!("Invalid program handle {}", program))?;
        if !prog.linked {
            return Err(format!("Program {} is not linked", program));
        }
        self.current_program = Some(program);
        Ok(())
    }

    /// Delete a program object (`glDeleteProgram`).
    pub fn delete_program(&mut self, program: u32) {
        if self.current_program == Some(program) {
            self.current_program = None;
        }
        self.programs.remove(&program);
    }

    // =================================================================
    // Buffer management
    // =================================================================

    /// Generate buffer objects (`glGenBuffers`).
    pub fn gen_buffers(&mut self, count: usize) -> Vec<u32> {
        let mut handles = Vec::new();
        for _ in 0..count {
            let handle = self.gen_id();
            self.buffers.insert(handle, None);
            handles.push(handle);
        }
        handles
    }

    /// Delete buffer objects (`glDeleteBuffers`).
    pub fn delete_buffers(&mut self, handles: &[u32]) {
        for &handle in handles {
            if let Some(Some(buf_id)) = self.buffers.get(&handle) {
                let _ = self.base.device.memory_manager_mut().free(*buf_id);
            }
            self.buffers.remove(&handle);
            // Remove from bindings
            self.bound_buffers.retain(|_, v| *v != handle);
            self.target_buffers.retain(|_, v| *v != handle);
        }
    }

    /// Bind a buffer to a target (`glBindBuffer`).
    pub fn bind_buffer(&mut self, target: u32, buffer: u32) -> Result<(), String> {
        if buffer == 0 {
            self.target_buffers.remove(&target);
            return Ok(());
        }
        if !self.buffers.contains_key(&buffer) {
            return Err(format!("Invalid buffer handle {}", buffer));
        }
        self.target_buffers.insert(target, buffer);
        Ok(())
    }

    /// Allocate and optionally fill a buffer (`glBufferData`).
    pub fn buffer_data(
        &mut self,
        target: u32,
        size: usize,
        data: Option<&[u8]>,
        _usage: u32,
    ) -> Result<(), String> {
        let handle = *self
            .target_buffers
            .get(&target)
            .ok_or(format!("No buffer bound to target 0x{:04X}", target))?;

        // Free old allocation if exists
        if let Some(Some(old_buf_id)) = self.buffers.get(&handle) {
            let _ = self.base.device.memory_manager_mut().free(*old_buf_id);
        }

        // Allocate new buffer
        let buf_id = self.base.allocate_buffer(size)?;
        self.buffers.insert(handle, Some(buf_id));

        // Upload initial data if provided
        if let Some(d) = data {
            let mm = self.base.device.memory_manager_mut();
            {
                let mut mapped = mm.map(buf_id)?;
                let copy_size = d.len().min(size);
                mapped.write(0, &d[..copy_size])?;
            }
            mm.unmap(buf_id)?;
        }

        Ok(())
    }

    /// Update a portion of a buffer (`glBufferSubData`).
    pub fn buffer_sub_data(
        &mut self,
        target: u32,
        offset: usize,
        data: &[u8],
    ) -> Result<(), String> {
        let handle = *self
            .target_buffers
            .get(&target)
            .ok_or(format!("No buffer bound to target 0x{:04X}", target))?;
        let buf_id = self.buffers[&handle]
            .ok_or(format!("Buffer {} has no data store", handle))?;

        let mm = self.base.device.memory_manager_mut();
        {
            let mut mapped = mm.map(buf_id)?;
            mapped.write(offset, data)?;
        }
        mm.unmap(buf_id)?;
        Ok(())
    }

    /// Bind a buffer to an indexed binding point (`glBindBufferBase`).
    pub fn bind_buffer_base(
        &mut self,
        target: u32,
        index: u32,
        buffer: u32,
    ) -> Result<(), String> {
        if !self.buffers.contains_key(&buffer) {
            return Err(format!("Invalid buffer handle {}", buffer));
        }
        self.bound_buffers.insert((target, index), buffer);
        Ok(())
    }

    /// Map a buffer region for CPU access (`glMapBufferRange`).
    pub fn map_buffer_range(
        &mut self,
        target: u32,
        offset: usize,
        length: usize,
        _access: u32,
    ) -> Result<Vec<u8>, String> {
        let handle = *self
            .target_buffers
            .get(&target)
            .ok_or(format!("No buffer bound to target 0x{:04X}", target))?;
        let buf_id = self.buffers[&handle]
            .ok_or(format!("Buffer {} has no data store", handle))?;

        let mm = self.base.device.memory_manager_mut();
        mm.invalidate(buf_id, 0, 0)?;
        let data = {
            let mapped = mm.map(buf_id)?;
            mapped.read(offset, length)?
        };
        mm.unmap(buf_id)?;
        Ok(data)
    }

    /// Unmap a buffer (`glUnmapBuffer`). Returns true on success.
    pub fn unmap_buffer(&self, _target: u32) -> bool {
        true
    }

    // =================================================================
    // Compute dispatch
    // =================================================================

    /// Dispatch compute work groups (`glDispatchCompute`).
    ///
    /// Uses whatever program and SSBO bindings are currently active.
    pub fn dispatch_compute(
        &mut self,
        num_groups_x: usize,
        num_groups_y: usize,
        num_groups_z: usize,
    ) -> Result<(), String> {
        let program = self
            .current_program
            .ok_or("No program is currently active (call use_program first)")?;

        let prog = &self.programs[&program];

        // Get shader code from the program
        let shader_code = prog.shader_code.clone();

        // Find all SSBO bindings
        let mut ssbo_bindings: Vec<(usize, usize)> = Vec::new();
        for (&(target, index), &handle) in &self.bound_buffers {
            if target == GL_SHADER_STORAGE_BUFFER {
                if let Some(Some(buf_id)) = self.buffers.get(&handle) {
                    ssbo_bindings.push((index as usize, *buf_id));
                }
            }
        }
        ssbo_bindings.sort_by_key(|(idx, _)| *idx);

        // Create shader module
        let shader = self.base.device.create_shader_module(
            shader_code,
            "",
            "main",
            (1, 1, 1),
        );

        // Create descriptor set with SSBO bindings
        let descriptor_bindings: Vec<DescriptorBinding> = ssbo_bindings
            .iter()
            .map(|(idx, _)| DescriptorBinding::new(*idx))
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
        for (idx, buf_id) in &ssbo_bindings {
            ds.write(*idx, *buf_id)?;
        }
        let ds_id = ds.set_id();

        self.base.create_and_submit_cb(move |cb| {
            cb.cmd_bind_pipeline(pipeline_id)?;
            cb.cmd_bind_descriptor_set(ds_id)?;
            cb.cmd_dispatch(num_groups_x, num_groups_y, num_groups_z)
        })
    }

    // =================================================================
    // Synchronization
    // =================================================================

    /// Insert a memory barrier (`glMemoryBarrier`).
    ///
    /// In our synchronous simulator, this is a no-op.
    pub fn memory_barrier(&self, _barriers: u32) {
        // Synchronous execution means barriers are automatically satisfied.
    }

    /// Create a sync object (`glFenceSync`).
    pub fn fence_sync(&mut self) -> u32 {
        let handle = self.gen_id();
        self.syncs.insert(handle, true);
        handle
    }

    /// Wait for a sync object (`glClientWaitSync`).
    pub fn client_wait_sync(&self, sync: u32, _flags: u32, _timeout: u64) -> u32 {
        match self.syncs.get(&sync) {
            None => GL_WAIT_FAILED,
            Some(true) => GL_ALREADY_SIGNALED,
            Some(false) => GL_TIMEOUT_EXPIRED,
        }
    }

    /// Delete a sync object (`glDeleteSync`).
    pub fn delete_sync(&mut self, sync: u32) {
        self.syncs.remove(&sync);
    }

    /// Block until all GL commands complete (`glFinish`).
    pub fn finish(&self) {
        self.base.device.wait_idle();
    }

    // =================================================================
    // Uniforms (push constants)
    // =================================================================

    /// Get the location of a uniform variable (`glGetUniformLocation`).
    pub fn get_uniform_location(&self, program: u32, name: &str) -> Result<u32, String> {
        if !self.programs.contains_key(&program) {
            return Err(format!("Invalid program handle {}", program));
        }
        // Deterministic location based on name hash
        let mut hash: u32 = 0;
        for b in name.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(b as u32);
        }
        Ok(hash & 0x7FFFFFFF)
    }

    /// Set a float uniform (`glUniform1f`).
    pub fn uniform_1f(&mut self, location: u32, value: f64) {
        if let Some(program) = self.current_program {
            self.uniforms
                .insert((program, location.to_string()), value);
        }
    }

    /// Set an integer uniform (`glUniform1i`).
    pub fn uniform_1i(&mut self, location: u32, value: i32) {
        if let Some(program) = self.current_program {
            self.uniforms
                .insert((program, location.to_string()), value as f64);
        }
    }
}
