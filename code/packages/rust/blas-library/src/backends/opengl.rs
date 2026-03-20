//! OpenGlBlas -- legacy OpenGL compute BLAS backend.
//!
//! # How OpenGlBlas Works
//!
//! This backend wraps `GlContext` from Layer 4. OpenGL uses a global state
//! machine model -- you bind things to "current" state and then issue
//! commands that operate on whatever is currently bound.
//!
//! For each BLAS operation:
//! 1. `gl.gen_buffers()`         -- generate buffer IDs
//! 2. `gl.buffer_data()`         -- allocate and upload data
//! 3. (compute)                   -- perform operation
//! 4. `gl.map_buffer_range()`    -- map buffer for reading
//! 5. `gl.delete_buffers()`      -- free buffers

use vendor_api_simulators::opengl::{
    GlContext, GL_MAP_READ_BIT, GL_SHADER_STORAGE_BUFFER, GL_STATIC_DRAW,
};

use super::gpu_base::GpuBlasBackend;

/// OpenGL BLAS backend -- wraps GlContext from Layer 4.
///
/// # OpenGL BLAS -- Legacy State Machine GPU Acceleration
///
/// OpenGL is the oldest surviving GPU API (1992). Compute shaders were added
/// in OpenGL 4.3 (2012), bolted onto the existing state machine model.
///
/// The state machine means:
/// - `glBindBuffer(target, id)` sets "current buffer" globally
/// - `glBufferData(target, ...)` operates on WHATEVER is currently bound
/// - You must remember what's bound at all times
pub struct OpenGlBlas {
    gl: GlContext,
    /// Map of our handle IDs to GL buffer handles.
    buffers: Vec<u32>,
}

impl OpenGlBlas {
    /// Create a new OpenGL BLAS backend.
    pub fn new() -> Result<Self, String> {
        let gl = GlContext::new()?;
        Ok(Self {
            gl,
            buffers: Vec::new(),
        })
    }
}

impl GpuBlasBackend for OpenGlBlas {
    fn gpu_name(&self) -> &str {
        "opengl"
    }

    fn gpu_device_name(&self) -> String {
        "OpenGL Device".to_string()
    }

    fn upload(&mut self, data: &[u8]) -> Result<usize, String> {
        let buf_ids = self.gl.gen_buffers(1);
        let buf_id = buf_ids[0];
        self.gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, buf_id)?;
        self.gl
            .buffer_data(GL_SHADER_STORAGE_BUFFER, data.len(), Some(data), GL_STATIC_DRAW)?;
        let handle = self.buffers.len();
        self.buffers.push(buf_id);
        Ok(handle)
    }

    fn download(&mut self, handle: usize, size: usize) -> Result<Vec<u8>, String> {
        if handle >= self.buffers.len() {
            return Err(format!("Invalid OpenGL buffer handle {}", handle));
        }
        let buf_id = self.buffers[handle];
        self.gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, buf_id)?;
        let data = self.gl.map_buffer_range(GL_SHADER_STORAGE_BUFFER, 0, size, GL_MAP_READ_BIT)?;
        self.gl.unmap_buffer(GL_SHADER_STORAGE_BUFFER);
        Ok(data[..size].to_vec())
    }

    fn free(&mut self, handle: usize) -> Result<(), String> {
        if handle >= self.buffers.len() {
            return Err(format!("Invalid OpenGL buffer handle {}", handle));
        }
        let buf_id = self.buffers[handle];
        self.gl.delete_buffers(&[buf_id]);
        Ok(())
    }
}
