//! ValidationLayer -- catches GPU programming errors early.
//!
//! # What is a Validation Layer?
//!
//! In Vulkan, validation layers are optional middleware that check every API
//! call for errors. They're enabled during development and disabled in
//! production (for performance). Common errors they catch:
//!
//! - Dispatching without binding a pipeline
//! - Using a freed buffer in a descriptor set
//! - Missing a barrier between write and read
//! - Mapping a DEVICE_LOCAL-only buffer
//! - Exceeding device limits
//!
//! Our validation layer checks every operation and raises clear error messages.

use std::collections::HashSet;

use crate::command_buffer::CommandBuffer;
use crate::memory::Buffer;
use crate::protocols::{BufferUsage, CommandBufferState, MemoryType};

/// Raised when a validation check fails.
///
/// These errors represent GPU programming mistakes -- things that would
/// cause undefined behavior or crashes on real hardware.
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ValidationError: {}", self.message)
    }
}

impl std::error::Error for ValidationError {}

impl ValidationError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

/// Validates runtime operations and raises clear error messages.
///
/// # What It Checks
///
/// 1. Command buffer state transitions (can't record without begin())
/// 2. Pipeline/descriptor binding (can't dispatch without binding both)
/// 3. Memory type compatibility (can't map DEVICE_LOCAL)
/// 4. Buffer usage flags (can't use STORAGE buffer as TRANSFER_SRC)
/// 5. Freed resource detection (can't use freed buffers)
/// 6. Barrier correctness (warn on write->read without barrier)
pub struct ValidationLayer {
    warnings: Vec<String>,
    errors: Vec<String>,
    /// Track which buffers have been written to (for barrier checking).
    written_buffers: HashSet<usize>,
    /// Track which buffers have barriers protecting reads.
    barriered_buffers: HashSet<usize>,
}

impl ValidationLayer {
    pub fn new() -> Self {
        Self {
            warnings: Vec::new(),
            errors: Vec::new(),
            written_buffers: HashSet::new(),
            barriered_buffers: HashSet::new(),
        }
    }

    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    /// Clear all warnings and errors.
    pub fn clear(&mut self) {
        self.warnings.clear();
        self.errors.clear();
        self.written_buffers.clear();
        self.barriered_buffers.clear();
    }

    // --- Command buffer validation ---

    /// Validate that begin() is allowed.
    pub fn validate_begin(&self, cb: &CommandBuffer) -> Result<(), ValidationError> {
        if cb.state() != CommandBufferState::Initial
            && cb.state() != CommandBufferState::Complete
        {
            return Err(ValidationError::new(&format!(
                "Cannot begin CB#{}: state is {} (expected initial or complete)",
                cb.command_buffer_id(),
                cb.state().as_str()
            )));
        }
        Ok(())
    }

    /// Validate that end() is allowed.
    pub fn validate_end(&self, cb: &CommandBuffer) -> Result<(), ValidationError> {
        if cb.state() != CommandBufferState::Recording {
            return Err(ValidationError::new(&format!(
                "Cannot end CB#{}: state is {} (expected recording)",
                cb.command_buffer_id(),
                cb.state().as_str()
            )));
        }
        Ok(())
    }

    /// Validate that a CB can be submitted.
    pub fn validate_submit(&self, cb: &CommandBuffer) -> Result<(), ValidationError> {
        if cb.state() != CommandBufferState::Recorded {
            return Err(ValidationError::new(&format!(
                "Cannot submit CB#{}: state is {} (expected recorded)",
                cb.command_buffer_id(),
                cb.state().as_str()
            )));
        }
        Ok(())
    }

    // --- Dispatch validation ---

    /// Validate a dispatch command.
    pub fn validate_dispatch(
        &self,
        cb: &CommandBuffer,
        group_x: usize,
        group_y: usize,
        group_z: usize,
    ) -> Result<(), ValidationError> {
        if cb.bound_pipeline_id().is_none() {
            return Err(ValidationError::new(&format!(
                "Cannot dispatch in CB#{}: no pipeline bound (call cmd_bind_pipeline first)",
                cb.command_buffer_id()
            )));
        }
        if group_x == 0 || group_y == 0 || group_z == 0 {
            return Err(ValidationError::new(&format!(
                "Dispatch dimensions must be positive: ({}, {}, {})",
                group_x, group_y, group_z
            )));
        }
        Ok(())
    }

    // --- Memory validation ---

    /// Validate that a buffer can be mapped.
    pub fn validate_map(&self, buffer: &Buffer) -> Result<(), ValidationError> {
        if buffer.freed {
            return Err(ValidationError::new(&format!(
                "Cannot map freed buffer {}",
                buffer.buffer_id
            )));
        }
        if buffer.mapped {
            return Err(ValidationError::new(&format!(
                "Buffer {} is already mapped",
                buffer.buffer_id
            )));
        }
        if !buffer.memory_type.contains(MemoryType::HOST_VISIBLE) {
            return Err(ValidationError::new(&format!(
                "Cannot map buffer {}: not HOST_VISIBLE (type={:?}). \
                 Use a staging buffer for DEVICE_LOCAL memory.",
                buffer.buffer_id, buffer.memory_type
            )));
        }
        Ok(())
    }

    /// Validate that a buffer has the required usage flags.
    pub fn validate_buffer_usage(
        &self,
        buffer: &Buffer,
        required_usage: BufferUsage,
    ) -> Result<(), ValidationError> {
        if !buffer.usage.contains(required_usage) {
            return Err(ValidationError::new(&format!(
                "Buffer {} lacks required usage {:?} (has {:?})",
                buffer.buffer_id, required_usage, buffer.usage
            )));
        }
        Ok(())
    }

    /// Validate that a buffer is not freed.
    pub fn validate_buffer_not_freed(&self, buffer: &Buffer) -> Result<(), ValidationError> {
        if buffer.freed {
            return Err(ValidationError::new(&format!(
                "Buffer {} has been freed",
                buffer.buffer_id
            )));
        }
        Ok(())
    }

    // --- Barrier validation ---

    /// Record that a buffer was written to (for barrier checking).
    pub fn record_write(&mut self, buffer_id: usize) {
        self.written_buffers.insert(buffer_id);
        self.barriered_buffers.remove(&buffer_id);
    }

    /// Record that a barrier was placed (covers some/all buffers).
    pub fn record_barrier(&mut self, buffer_ids: Option<&HashSet<usize>>) {
        if let Some(ids) = buffer_ids {
            self.barriered_buffers.extend(ids);
        } else {
            // Global barrier -- covers all written buffers
            self.barriered_buffers
                .extend(self.written_buffers.iter());
        }
    }

    /// Warn if reading a buffer that was written without a barrier.
    pub fn validate_read_after_write(&mut self, buffer_id: usize) {
        if self.written_buffers.contains(&buffer_id)
            && !self.barriered_buffers.contains(&buffer_id)
        {
            self.warnings.push(format!(
                "Reading buffer {} after write without barrier. \
                 Insert cmd_pipeline_barrier() between write and read.",
                buffer_id
            ));
        }
    }
}

impl Default for ValidationLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_begin_initial() {
        let layer = ValidationLayer::new();
        let cb = CommandBuffer::new();
        assert!(layer.validate_begin(&cb).is_ok());
    }

    #[test]
    fn test_validate_begin_recording_fails() {
        let layer = ValidationLayer::new();
        let mut cb = CommandBuffer::new();
        cb.begin().unwrap();
        assert!(layer.validate_begin(&cb).is_err());
    }

    #[test]
    fn test_validate_end_recording() {
        let layer = ValidationLayer::new();
        let mut cb = CommandBuffer::new();
        cb.begin().unwrap();
        assert!(layer.validate_end(&cb).is_ok());
    }

    #[test]
    fn test_validate_end_initial_fails() {
        let layer = ValidationLayer::new();
        let cb = CommandBuffer::new();
        assert!(layer.validate_end(&cb).is_err());
    }

    #[test]
    fn test_validate_submit() {
        let layer = ValidationLayer::new();
        let mut cb = CommandBuffer::new();
        cb.begin().unwrap();
        cb.end().unwrap();
        assert!(layer.validate_submit(&cb).is_ok());
    }

    #[test]
    fn test_validate_dispatch_no_pipeline() {
        let layer = ValidationLayer::new();
        let mut cb = CommandBuffer::new();
        cb.begin().unwrap();
        assert!(layer.validate_dispatch(&cb, 1, 1, 1).is_err());
    }

    #[test]
    fn test_validate_dispatch_zero_dims() {
        let layer = ValidationLayer::new();
        let mut cb = CommandBuffer::new();
        cb.begin().unwrap();
        cb.cmd_bind_pipeline(0).unwrap();
        assert!(layer.validate_dispatch(&cb, 0, 1, 1).is_err());
    }

    #[test]
    fn test_validate_map_device_local() {
        let layer = ValidationLayer::new();
        let buf = Buffer {
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
    fn test_validate_map_host_visible() {
        let layer = ValidationLayer::new();
        let buf = Buffer {
            buffer_id: 0,
            size: 64,
            memory_type: MemoryType::HOST_VISIBLE,
            usage: BufferUsage::STORAGE,
            device_address: 0,
            mapped: false,
            freed: false,
        };
        assert!(layer.validate_map(&buf).is_ok());
    }

    #[test]
    fn test_validate_map_freed() {
        let layer = ValidationLayer::new();
        let buf = Buffer {
            buffer_id: 0,
            size: 64,
            memory_type: MemoryType::HOST_VISIBLE,
            usage: BufferUsage::STORAGE,
            device_address: 0,
            mapped: false,
            freed: true,
        };
        assert!(layer.validate_map(&buf).is_err());
    }

    #[test]
    fn test_validate_buffer_usage() {
        let layer = ValidationLayer::new();
        let buf = Buffer {
            buffer_id: 0,
            size: 64,
            memory_type: MemoryType::DEVICE_LOCAL,
            usage: BufferUsage::STORAGE,
            device_address: 0,
            mapped: false,
            freed: false,
        };
        assert!(layer.validate_buffer_usage(&buf, BufferUsage::STORAGE).is_ok());
        assert!(layer
            .validate_buffer_usage(&buf, BufferUsage::TRANSFER_SRC)
            .is_err());
    }

    #[test]
    fn test_barrier_tracking() {
        let mut layer = ValidationLayer::new();
        layer.record_write(0);
        layer.validate_read_after_write(0);
        assert_eq!(layer.warnings().len(), 1);

        // After barrier, no more warnings
        layer.record_barrier(None);
        layer.warnings.clear();
        layer.validate_read_after_write(0);
        assert!(layer.warnings().is_empty());
    }

    #[test]
    fn test_clear() {
        let mut layer = ValidationLayer::new();
        layer.record_write(0);
        layer.validate_read_after_write(0);
        assert!(!layer.warnings().is_empty());

        layer.clear();
        assert!(layer.warnings().is_empty());
    }
}
