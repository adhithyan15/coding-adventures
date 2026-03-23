//! CommandBuffer -- recorded sequence of GPU commands.
//!
//! # The Record-Then-Submit Model
//!
//! Instead of calling GPU operations one at a time (like CUDA), Vulkan records
//! commands into a buffer and submits the whole buffer at once:
//!
//! ```text
//! // CUDA style (implicit, one at a time):
//! cudaMemcpy(dst, src, size)     // executes immediately
//! kernel<<<grid, block>>>(args)  // executes immediately
//!
//! // Vulkan style (explicit, batched):
//! cb.begin()                     // start recording
//! cb.cmd_copy_buffer(...)        // just records -- doesn't execute
//! cb.cmd_dispatch(...)           // just records -- doesn't execute
//! cb.end()                       // stop recording
//! queue.submit([cb])             // NOW everything executes
//! ```
//!
//! # Why Batch?
//!
//! 1. **Driver optimization** -- the driver sees all commands at once and can
//!    reorder, merge, or eliminate redundancies.
//! 2. **Reuse** -- submit the same CB multiple times without re-recording.
//! 3. **Multi-threaded recording** -- different CPU threads record different
//!    CBs in parallel, then submit them together.
//! 4. **Validation** -- check the entire sequence for errors before any GPU
//!    work starts.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::protocols::{
    CommandArg, CommandBufferState, PipelineBarrier, PipelineStage, RecordedCommand,
};

// =========================================================================
// ID generator
// =========================================================================

static NEXT_CB_ID: AtomicUsize = AtomicUsize::new(0);

/// Reset the command buffer ID counter (for test isolation).
pub fn reset_cb_ids() {
    NEXT_CB_ID.store(0, Ordering::SeqCst);
}

// =========================================================================
// CommandBuffer
// =========================================================================

/// A recorded sequence of GPU commands.
///
/// # Command Types
///
/// **Compute commands:**
/// - `cmd_bind_pipeline` -- select which kernel to run
/// - `cmd_bind_descriptor_set` -- bind memory to kernel parameters
/// - `cmd_push_constants` -- small inline data (<=128 bytes)
/// - `cmd_dispatch` -- launch kernel with grid dimensions
/// - `cmd_dispatch_indirect` -- read grid dimensions from a GPU buffer
///
/// **Transfer commands:**
/// - `cmd_copy_buffer` -- device-to-device memory copy
/// - `cmd_fill_buffer` -- fill buffer with a constant value
/// - `cmd_update_buffer` -- write small data inline (CPU -> GPU)
///
/// **Synchronization commands:**
/// - `cmd_pipeline_barrier` -- execution + memory ordering
/// - `cmd_set_event` -- signal an event from GPU
/// - `cmd_wait_event` -- wait for event before proceeding
/// - `cmd_reset_event` -- reset event from GPU
pub struct CommandBuffer {
    id: usize,
    state: CommandBufferState,
    commands: Vec<RecordedCommand>,

    // Currently bound state (for validation and execution)
    bound_pipeline_id: Option<usize>,
    bound_descriptor_set_id: Option<usize>,
    push_constants: Vec<u8>,
}

impl CommandBuffer {
    pub fn new() -> Self {
        Self {
            id: NEXT_CB_ID.fetch_add(1, Ordering::SeqCst),
            state: CommandBufferState::Initial,
            commands: Vec::new(),
            bound_pipeline_id: None,
            bound_descriptor_set_id: None,
            push_constants: Vec::new(),
        }
    }

    pub fn command_buffer_id(&self) -> usize {
        self.id
    }

    pub fn state(&self) -> CommandBufferState {
        self.state
    }

    pub fn commands(&self) -> &[RecordedCommand] {
        &self.commands
    }

    pub fn bound_pipeline_id(&self) -> Option<usize> {
        self.bound_pipeline_id
    }

    pub fn bound_descriptor_set_id(&self) -> Option<usize> {
        self.bound_descriptor_set_id
    }

    // =================================================================
    // Lifecycle
    // =================================================================

    /// Start recording commands.
    ///
    /// Transitions: INITIAL -> RECORDING, or COMPLETE -> RECORDING (reuse).
    pub fn begin(&mut self) -> Result<(), String> {
        if self.state != CommandBufferState::Initial
            && self.state != CommandBufferState::Complete
        {
            return Err(format!(
                "Cannot begin recording: state is {} (expected initial or complete)",
                self.state.as_str()
            ));
        }
        self.state = CommandBufferState::Recording;
        self.commands.clear();
        self.bound_pipeline_id = None;
        self.bound_descriptor_set_id = None;
        self.push_constants.clear();
        Ok(())
    }

    /// Finish recording commands.
    ///
    /// Transitions: RECORDING -> RECORDED.
    pub fn end(&mut self) -> Result<(), String> {
        if self.state != CommandBufferState::Recording {
            return Err(format!(
                "Cannot end recording: state is {} (expected recording)",
                self.state.as_str()
            ));
        }
        self.state = CommandBufferState::Recorded;
        Ok(())
    }

    /// Reset to INITIAL state for reuse. Clears all recorded commands.
    pub fn reset(&mut self) {
        self.state = CommandBufferState::Initial;
        self.commands.clear();
        self.bound_pipeline_id = None;
        self.bound_descriptor_set_id = None;
        self.push_constants.clear();
    }

    /// Internal: mark as submitted (called by CommandQueue).
    pub(crate) fn mark_pending(&mut self) {
        self.state = CommandBufferState::Pending;
    }

    /// Internal: mark as finished (called by CommandQueue).
    pub(crate) fn mark_complete(&mut self) {
        self.state = CommandBufferState::Complete;
    }

    fn require_recording(&self) -> Result<(), String> {
        if self.state != CommandBufferState::Recording {
            return Err(format!(
                "Cannot record command: state is {} (expected recording)",
                self.state.as_str()
            ));
        }
        Ok(())
    }

    // =================================================================
    // Compute commands
    // =================================================================

    /// Bind a compute pipeline for subsequent dispatches.
    pub fn cmd_bind_pipeline(&mut self, pipeline_id: usize) -> Result<(), String> {
        self.require_recording()?;
        self.bound_pipeline_id = Some(pipeline_id);
        let mut args = HashMap::new();
        args.insert("pipeline_id".to_string(), CommandArg::Usize(pipeline_id));
        self.commands.push(RecordedCommand {
            command: "bind_pipeline".to_string(),
            args,
        });
        Ok(())
    }

    /// Bind a descriptor set for subsequent dispatches.
    pub fn cmd_bind_descriptor_set(&mut self, set_id: usize) -> Result<(), String> {
        self.require_recording()?;
        self.bound_descriptor_set_id = Some(set_id);
        let mut args = HashMap::new();
        args.insert("set_id".to_string(), CommandArg::Usize(set_id));
        self.commands.push(RecordedCommand {
            command: "bind_descriptor_set".to_string(),
            args,
        });
        Ok(())
    }

    /// Set push constant data for the next dispatch.
    ///
    /// Push constants are small pieces of data (<=128 bytes) sent inline
    /// with the dispatch command.
    pub fn cmd_push_constants(&mut self, offset: usize, data: &[u8]) -> Result<(), String> {
        self.require_recording()?;
        self.push_constants = data.to_vec();
        let mut args = HashMap::new();
        args.insert("offset".to_string(), CommandArg::Usize(offset));
        args.insert("size".to_string(), CommandArg::Usize(data.len()));
        self.commands.push(RecordedCommand {
            command: "push_constants".to_string(),
            args,
        });
        Ok(())
    }

    /// Launch a compute kernel.
    ///
    /// # Dispatch Dimensions
    ///
    /// The dispatch creates a 3D grid of workgroups:
    /// `Total threads = (group_x * group_y * group_z) * (local_x * local_y * local_z)`
    pub fn cmd_dispatch(
        &mut self,
        group_x: usize,
        group_y: usize,
        group_z: usize,
    ) -> Result<(), String> {
        self.require_recording()?;
        if self.bound_pipeline_id.is_none() {
            return Err("Cannot dispatch: no pipeline bound".to_string());
        }
        let mut args = HashMap::new();
        args.insert("group_x".to_string(), CommandArg::Usize(group_x));
        args.insert("group_y".to_string(), CommandArg::Usize(group_y));
        args.insert("group_z".to_string(), CommandArg::Usize(group_z));
        self.commands.push(RecordedCommand {
            command: "dispatch".to_string(),
            args,
        });
        Ok(())
    }

    /// Launch a compute kernel with grid dimensions from a GPU buffer.
    pub fn cmd_dispatch_indirect(
        &mut self,
        buffer_id: usize,
        offset: usize,
    ) -> Result<(), String> {
        self.require_recording()?;
        if self.bound_pipeline_id.is_none() {
            return Err("Cannot dispatch: no pipeline bound".to_string());
        }
        let mut args = HashMap::new();
        args.insert("buffer_id".to_string(), CommandArg::Usize(buffer_id));
        args.insert("offset".to_string(), CommandArg::Usize(offset));
        self.commands.push(RecordedCommand {
            command: "dispatch_indirect".to_string(),
            args,
        });
        Ok(())
    }

    // =================================================================
    // Transfer commands
    // =================================================================

    /// Copy data between device buffers.
    pub fn cmd_copy_buffer(
        &mut self,
        src_id: usize,
        dst_id: usize,
        size: usize,
        src_offset: usize,
        dst_offset: usize,
    ) -> Result<(), String> {
        self.require_recording()?;
        let mut args = HashMap::new();
        args.insert("src_id".to_string(), CommandArg::Usize(src_id));
        args.insert("dst_id".to_string(), CommandArg::Usize(dst_id));
        args.insert("size".to_string(), CommandArg::Usize(size));
        args.insert("src_offset".to_string(), CommandArg::Usize(src_offset));
        args.insert("dst_offset".to_string(), CommandArg::Usize(dst_offset));
        self.commands.push(RecordedCommand {
            command: "copy_buffer".to_string(),
            args,
        });
        Ok(())
    }

    /// Fill a buffer with a constant byte value.
    pub fn cmd_fill_buffer(
        &mut self,
        buffer_id: usize,
        value: u8,
        offset: usize,
        size: usize,
    ) -> Result<(), String> {
        self.require_recording()?;
        let mut args = HashMap::new();
        args.insert("buffer_id".to_string(), CommandArg::Usize(buffer_id));
        args.insert("value".to_string(), CommandArg::UInt(value as u64));
        args.insert("offset".to_string(), CommandArg::Usize(offset));
        args.insert("size".to_string(), CommandArg::Usize(size));
        self.commands.push(RecordedCommand {
            command: "fill_buffer".to_string(),
            args,
        });
        Ok(())
    }

    /// Write small data inline from CPU to device buffer.
    pub fn cmd_update_buffer(
        &mut self,
        buffer_id: usize,
        offset: usize,
        data: &[u8],
    ) -> Result<(), String> {
        self.require_recording()?;
        let mut args = HashMap::new();
        args.insert("buffer_id".to_string(), CommandArg::Usize(buffer_id));
        args.insert("offset".to_string(), CommandArg::Usize(offset));
        args.insert("data".to_string(), CommandArg::Bytes(data.to_vec()));
        self.commands.push(RecordedCommand {
            command: "update_buffer".to_string(),
            args,
        });
        Ok(())
    }

    // =================================================================
    // Synchronization commands
    // =================================================================

    /// Insert an execution + memory barrier.
    pub fn cmd_pipeline_barrier(&mut self, barrier: &PipelineBarrier) -> Result<(), String> {
        self.require_recording()?;
        let mut args = HashMap::new();
        args.insert(
            "src_stage".to_string(),
            CommandArg::Str(barrier.src_stage.as_str().to_string()),
        );
        args.insert(
            "dst_stage".to_string(),
            CommandArg::Str(barrier.dst_stage.as_str().to_string()),
        );
        args.insert(
            "memory_barrier_count".to_string(),
            CommandArg::Usize(barrier.memory_barriers.len()),
        );
        args.insert(
            "buffer_barrier_count".to_string(),
            CommandArg::Usize(barrier.buffer_barriers.len()),
        );
        self.commands.push(RecordedCommand {
            command: "pipeline_barrier".to_string(),
            args,
        });
        Ok(())
    }

    /// Signal an event from the GPU.
    pub fn cmd_set_event(&mut self, event_id: usize, stage: PipelineStage) -> Result<(), String> {
        self.require_recording()?;
        let mut args = HashMap::new();
        args.insert("event_id".to_string(), CommandArg::Usize(event_id));
        args.insert(
            "stage".to_string(),
            CommandArg::Str(stage.as_str().to_string()),
        );
        self.commands.push(RecordedCommand {
            command: "set_event".to_string(),
            args,
        });
        Ok(())
    }

    /// Wait for an event before proceeding.
    pub fn cmd_wait_event(
        &mut self,
        event_id: usize,
        src_stage: PipelineStage,
        dst_stage: PipelineStage,
    ) -> Result<(), String> {
        self.require_recording()?;
        let mut args = HashMap::new();
        args.insert("event_id".to_string(), CommandArg::Usize(event_id));
        args.insert(
            "src_stage".to_string(),
            CommandArg::Str(src_stage.as_str().to_string()),
        );
        args.insert(
            "dst_stage".to_string(),
            CommandArg::Str(dst_stage.as_str().to_string()),
        );
        self.commands.push(RecordedCommand {
            command: "wait_event".to_string(),
            args,
        });
        Ok(())
    }

    /// Reset an event from the GPU side.
    pub fn cmd_reset_event(
        &mut self,
        event_id: usize,
        stage: PipelineStage,
    ) -> Result<(), String> {
        self.require_recording()?;
        let mut args = HashMap::new();
        args.insert("event_id".to_string(), CommandArg::Usize(event_id));
        args.insert(
            "stage".to_string(),
            CommandArg::Str(stage.as_str().to_string()),
        );
        self.commands.push(RecordedCommand {
            command: "reset_event".to_string(),
            args,
        });
        Ok(())
    }
}

impl Default for CommandBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let cb = CommandBuffer::new();
        assert_eq!(cb.state(), CommandBufferState::Initial);
        assert!(cb.commands().is_empty());
    }

    #[test]
    fn test_begin_end_lifecycle() {
        let mut cb = CommandBuffer::new();
        cb.begin().unwrap();
        assert_eq!(cb.state(), CommandBufferState::Recording);

        cb.end().unwrap();
        assert_eq!(cb.state(), CommandBufferState::Recorded);
    }

    #[test]
    fn test_cannot_begin_while_recording() {
        let mut cb = CommandBuffer::new();
        cb.begin().unwrap();
        assert!(cb.begin().is_err());
    }

    #[test]
    fn test_cannot_end_without_begin() {
        let mut cb = CommandBuffer::new();
        assert!(cb.end().is_err());
    }

    #[test]
    fn test_reset() {
        let mut cb = CommandBuffer::new();
        cb.begin().unwrap();
        cb.cmd_bind_pipeline(0).unwrap();
        cb.end().unwrap();

        cb.reset();
        assert_eq!(cb.state(), CommandBufferState::Initial);
        assert!(cb.commands().is_empty());
    }

    #[test]
    fn test_dispatch_without_pipeline_fails() {
        let mut cb = CommandBuffer::new();
        cb.begin().unwrap();
        assert!(cb.cmd_dispatch(1, 1, 1).is_err());
    }

    #[test]
    fn test_record_dispatch() {
        let mut cb = CommandBuffer::new();
        cb.begin().unwrap();
        cb.cmd_bind_pipeline(0).unwrap();
        cb.cmd_dispatch(4, 1, 1).unwrap();
        cb.end().unwrap();

        assert_eq!(cb.commands().len(), 2);
        assert_eq!(cb.commands()[0].command, "bind_pipeline");
        assert_eq!(cb.commands()[1].command, "dispatch");
    }

    #[test]
    fn test_cannot_record_without_begin() {
        let mut cb = CommandBuffer::new();
        assert!(cb.cmd_bind_pipeline(0).is_err());
    }

    #[test]
    fn test_transfer_commands() {
        let mut cb = CommandBuffer::new();
        cb.begin().unwrap();
        cb.cmd_copy_buffer(0, 1, 1024, 0, 0).unwrap();
        cb.cmd_fill_buffer(0, 0, 0, 512).unwrap();
        cb.cmd_update_buffer(0, 0, &[1, 2, 3]).unwrap();
        cb.end().unwrap();
        assert_eq!(cb.commands().len(), 3);
    }

    #[test]
    fn test_sync_commands() {
        let mut cb = CommandBuffer::new();
        cb.begin().unwrap();
        let barrier = PipelineBarrier {
            src_stage: PipelineStage::Compute,
            dst_stage: PipelineStage::Compute,
            ..PipelineBarrier::default()
        };
        cb.cmd_pipeline_barrier(&barrier).unwrap();
        cb.cmd_set_event(0, PipelineStage::Compute).unwrap();
        cb.cmd_wait_event(0, PipelineStage::Compute, PipelineStage::Transfer)
            .unwrap();
        cb.cmd_reset_event(0, PipelineStage::BottomOfPipe).unwrap();
        cb.end().unwrap();
        assert_eq!(cb.commands().len(), 4);
    }

    #[test]
    fn test_complete_to_recording() {
        let mut cb = CommandBuffer::new();
        cb.begin().unwrap();
        cb.end().unwrap();
        cb.mark_pending();
        cb.mark_complete();
        assert_eq!(cb.state(), CommandBufferState::Complete);

        // Can re-begin from Complete state
        cb.begin().unwrap();
        assert_eq!(cb.state(), CommandBufferState::Recording);
    }
}
