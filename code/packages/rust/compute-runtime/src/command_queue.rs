//! CommandQueue -- FIFO submission of command buffers to a device.
//!
//! # How Submission Works
//!
//! When you submit command buffers to a queue, the runtime processes them
//! sequentially, executing each recorded command against the Layer 6 device:
//!
//! ```text
//! queue.submit([cb1, cb2], fence)
//!     |
//!     +-- Execute cb1's commands:
//!     |   +-- bind_pipeline -> set current pipeline
//!     |   +-- dispatch(4, 1, 1) -> device.launch_kernel() + device.run()
//!     |   +-- pipeline_barrier -> (ensure completion, log trace)
//!     |
//!     +-- Execute cb2's commands:
//!     |   +-- copy_buffer -> device.memcpy
//!     |
//!     +-- Signal semaphores (if any)
//!     +-- Signal fence (if any)
//! ```
//!
//! # Multiple Queues
//!
//! A device can have multiple queues. Queues of different types (compute,
//! transfer) can execute in parallel -- while the compute queue runs a kernel,
//! the transfer queue can copy data.

use std::collections::HashMap;

use device_simulator::KernelDescriptor;

use crate::command_buffer::CommandBuffer;
use crate::memory::MemoryManager;
use crate::pipeline::Pipeline;
use crate::protocols::{
    CommandArg, CommandBufferState, QueueType, RecordedCommand, RuntimeEventType, RuntimeTrace,
};
use crate::sync::{Fence, Semaphore};

/// A FIFO queue that submits command buffers to a device.
///
/// # Queue Properties
///
/// - Commands within a CB execute sequentially
/// - CBs within a submission execute sequentially
/// - Multiple submissions execute sequentially (FIFO)
/// - Multiple QUEUES can execute in parallel
pub struct CommandQueue {
    queue_type: QueueType,
    queue_index: usize,
    total_cycles: u64,
}

impl CommandQueue {
    pub fn new(queue_type: QueueType, queue_index: usize) -> Self {
        Self {
            queue_type,
            queue_index,
            total_cycles: 0,
        }
    }

    pub fn queue_type(&self) -> QueueType {
        self.queue_type
    }

    pub fn queue_index(&self) -> usize {
        self.queue_index
    }

    pub fn total_cycles(&self) -> u64 {
        self.total_cycles
    }

    /// Submit command buffers for execution.
    ///
    /// # Submission Flow
    ///
    /// 1. Wait for all wait_semaphores to be signaled
    /// 2. Execute each command buffer sequentially
    /// 3. Signal all signal_semaphores
    /// 4. Signal the fence (if provided)
    pub fn submit(
        &mut self,
        command_buffers: &mut [&mut CommandBuffer],
        wait_semaphores: &mut [&mut Semaphore],
        signal_semaphores: &mut [&mut Semaphore],
        fence: Option<&mut Fence>,
        pipelines: &HashMap<usize, Pipeline>,
        memory_manager: &mut MemoryManager,
    ) -> Result<Vec<RuntimeTrace>, String> {
        let mut traces: Vec<RuntimeTrace> = Vec::new();

        // Validate CB states
        for cb in command_buffers.iter() {
            if cb.state() != CommandBufferState::Recorded {
                return Err(format!(
                    "CB#{} is in state {}, expected recorded",
                    cb.command_buffer_id(),
                    cb.state().as_str()
                ));
            }
        }

        // Wait on semaphores
        for sem in wait_semaphores.iter_mut() {
            if !sem.signaled() {
                return Err(format!(
                    "Semaphore {} is not signaled -- cannot proceed (possible deadlock)",
                    sem.semaphore_id()
                ));
            }
            traces.push(RuntimeTrace {
                timestamp_cycles: self.total_cycles,
                event_type: RuntimeEventType::SemaphoreWait,
                description: format!("Wait on semaphore S{}", sem.semaphore_id()),
                queue_type: Some(self.queue_type),
                semaphore_id: Some(sem.semaphore_id()),
                command_buffer_id: None,
                fence_id: None,
            });
            sem.reset();
        }

        // Log submission
        let stats = memory_manager.stats_ptr();
        unsafe {
            (*stats).total_submissions += 1;
            (*stats).total_command_buffers += command_buffers.len();
        }

        let cb_ids: Vec<usize> = command_buffers.iter().map(|cb| cb.command_buffer_id()).collect();
        traces.push(RuntimeTrace {
            timestamp_cycles: self.total_cycles,
            event_type: RuntimeEventType::Submit,
            description: format!(
                "Submit CB {:?} to {} queue",
                cb_ids,
                self.queue_type.as_str()
            ),
            queue_type: Some(self.queue_type),
            command_buffer_id: None,
            fence_id: None,
            semaphore_id: None,
        });

        // Execute each command buffer
        for cb in command_buffers.iter_mut() {
            cb.mark_pending();

            // Begin execution trace
            traces.push(RuntimeTrace {
                timestamp_cycles: self.total_cycles,
                event_type: RuntimeEventType::BeginExecution,
                description: format!("Begin CB#{}", cb.command_buffer_id()),
                queue_type: Some(self.queue_type),
                command_buffer_id: Some(cb.command_buffer_id()),
                fence_id: None,
                semaphore_id: None,
            });

            // Get the pipeline ID from the CB
            let pipeline_id = cb.bound_pipeline_id();

            // Execute commands
            let commands: Vec<RecordedCommand> = cb.commands().to_vec();
            for cmd in &commands {
                let cmd_traces = self.execute_command(
                    cmd,
                    pipeline_id,
                    pipelines,
                    memory_manager,
                )?;
                traces.extend(cmd_traces);
            }

            // End execution trace
            traces.push(RuntimeTrace {
                timestamp_cycles: self.total_cycles,
                event_type: RuntimeEventType::EndExecution,
                description: format!("End CB#{}", cb.command_buffer_id()),
                queue_type: Some(self.queue_type),
                command_buffer_id: Some(cb.command_buffer_id()),
                fence_id: None,
                semaphore_id: None,
            });

            cb.mark_complete();
        }

        // Signal semaphores
        for sem in signal_semaphores.iter_mut() {
            sem.signal();
            unsafe {
                (*stats).total_semaphore_signals += 1;
            }
            traces.push(RuntimeTrace {
                timestamp_cycles: self.total_cycles,
                event_type: RuntimeEventType::SemaphoreSignal,
                description: format!("Signal semaphore S{}", sem.semaphore_id()),
                queue_type: Some(self.queue_type),
                semaphore_id: Some(sem.semaphore_id()),
                command_buffer_id: None,
                fence_id: None,
            });
        }

        // Signal fence
        if let Some(f) = fence {
            f.signal();
            traces.push(RuntimeTrace {
                timestamp_cycles: self.total_cycles,
                event_type: RuntimeEventType::FenceSignal,
                description: format!("Signal fence F{}", f.fence_id()),
                queue_type: Some(self.queue_type),
                fence_id: Some(f.fence_id()),
                command_buffer_id: None,
                semaphore_id: None,
            });
        }

        // Update stats
        unsafe {
            (*stats).total_device_cycles = self.total_cycles;
            (*stats).update_utilization();
            (*stats).traces.extend(traces.clone());
        }

        Ok(traces)
    }

    /// Block until this queue has no pending work.
    ///
    /// In our synchronous simulation, submit() always runs to completion,
    /// so this is a no-op.
    pub fn wait_idle(&self) {
        // No-op in synchronous simulation
    }

    // =================================================================
    // Command execution
    // =================================================================

    fn execute_command(
        &mut self,
        cmd: &RecordedCommand,
        pipeline_id: Option<usize>,
        pipelines: &HashMap<usize, Pipeline>,
        memory_manager: &mut MemoryManager,
    ) -> Result<Vec<RuntimeTrace>, String> {
        match cmd.command.as_str() {
            "bind_pipeline" | "bind_descriptor_set" | "push_constants" => Ok(vec![]),
            "dispatch" => self.exec_dispatch(&cmd.args, pipeline_id, pipelines, memory_manager),
            "dispatch_indirect" => {
                self.exec_dispatch_indirect(&cmd.args, pipeline_id, pipelines, memory_manager)
            }
            "copy_buffer" => self.exec_copy_buffer(&cmd.args, memory_manager),
            "fill_buffer" => self.exec_fill_buffer(&cmd.args, memory_manager),
            "update_buffer" => self.exec_update_buffer(&cmd.args, memory_manager),
            "pipeline_barrier" => self.exec_pipeline_barrier(&cmd.args, memory_manager),
            "set_event" | "wait_event" | "reset_event" => Ok(vec![]),
            other => Err(format!("Unknown command: {}", other)),
        }
    }

    fn exec_dispatch(
        &mut self,
        args: &HashMap<String, CommandArg>,
        pipeline_id: Option<usize>,
        pipelines: &HashMap<usize, Pipeline>,
        memory_manager: &mut MemoryManager,
    ) -> Result<Vec<RuntimeTrace>, String> {
        let group_x = args.get("group_x").unwrap().as_usize();
        let group_y = args.get("group_y").unwrap().as_usize();
        let group_z = args.get("group_z").unwrap().as_usize();

        let pid = pipeline_id.ok_or("No pipeline bound for dispatch")?;
        let pipeline = pipelines
            .get(&pid)
            .ok_or_else(|| format!("Pipeline {} not found", pid))?;

        let shader = pipeline.shader();

        let kernel = if shader.is_gpu_style() {
            let mut k = KernelDescriptor::default();
            k.name = format!("dispatch_{}x{}x{}", group_x, group_y, group_z);
            k.program = Some(shader.code().unwrap().to_vec());
            k.grid_dim = (group_x, group_y, group_z);
            k.block_dim = shader.local_size();
            k
        } else {
            let mut k = KernelDescriptor::default();
            k.name = format!("op_{}", shader.operation());
            k.operation = shader.operation().to_string();
            k.input_data = Some(vec![vec![1.0]]);
            k.weight_data = Some(vec![vec![1.0]]);
            k
        };

        let device = memory_manager.device_mut();
        device.launch_kernel(kernel);
        let device_traces = device.run(10000);
        let cycles = device_traces.len() as u64;
        self.total_cycles += cycles;

        let stats = memory_manager.stats_ptr();
        unsafe {
            (*stats).total_dispatches += 1;
        }

        Ok(vec![RuntimeTrace {
            timestamp_cycles: self.total_cycles,
            event_type: RuntimeEventType::EndExecution,
            description: format!(
                "Dispatch ({},{},{}) completed in {} cycles",
                group_x, group_y, group_z, cycles
            ),
            queue_type: Some(self.queue_type),
            command_buffer_id: None,
            fence_id: None,
            semaphore_id: None,
        }])
    }

    fn exec_dispatch_indirect(
        &mut self,
        args: &HashMap<String, CommandArg>,
        pipeline_id: Option<usize>,
        pipelines: &HashMap<usize, Pipeline>,
        memory_manager: &mut MemoryManager,
    ) -> Result<Vec<RuntimeTrace>, String> {
        let buffer_id = args.get("buffer_id").unwrap().as_usize();
        let offset = args.get("offset").unwrap().as_usize();

        let data = memory_manager.get_buffer_data(buffer_id);
        if data.len() < offset + 12 {
            return Err("Buffer too small for indirect dispatch".to_string());
        }

        let group_x = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        let group_y = u32::from_le_bytes([
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]) as usize;
        let group_z = u32::from_le_bytes([
            data[offset + 8],
            data[offset + 9],
            data[offset + 10],
            data[offset + 11],
        ]) as usize;

        let mut dispatch_args = HashMap::new();
        dispatch_args.insert("group_x".to_string(), CommandArg::Usize(group_x));
        dispatch_args.insert("group_y".to_string(), CommandArg::Usize(group_y));
        dispatch_args.insert("group_z".to_string(), CommandArg::Usize(group_z));

        self.exec_dispatch(&dispatch_args, pipeline_id, pipelines, memory_manager)
    }

    fn exec_copy_buffer(
        &mut self,
        args: &HashMap<String, CommandArg>,
        memory_manager: &mut MemoryManager,
    ) -> Result<Vec<RuntimeTrace>, String> {
        let src_id = args.get("src_id").unwrap().as_usize();
        let dst_id = args.get("dst_id").unwrap().as_usize();
        let size = args.get("size").unwrap().as_usize();
        let src_offset = args.get("src_offset").unwrap().as_usize();
        let dst_offset = args.get("dst_offset").unwrap().as_usize();

        // Copy between internal buffers
        let src_slice = memory_manager.get_buffer_data(src_id)[src_offset..src_offset + size].to_vec();
        let dst_data = memory_manager.get_buffer_data_mut(dst_id);
        dst_data[dst_offset..dst_offset + size].copy_from_slice(&src_slice);

        // Also sync through device memory
        let src_buf = memory_manager.get_buffer(src_id)?;
        let src_addr = src_buf.device_address;
        let dst_buf = memory_manager.get_buffer(dst_id)?;
        let dst_addr = dst_buf.device_address;

        let device = memory_manager.device_mut();
        let (data_bytes, read_cycles) =
            device.memcpy_device_to_host(src_addr + src_offset as u64, size);
        let write_cycles =
            device.memcpy_host_to_device(dst_addr + dst_offset as u64, &data_bytes);

        let cycles = read_cycles + write_cycles;
        self.total_cycles += cycles;

        let stats = memory_manager.stats_ptr();
        unsafe {
            (*stats).total_transfers += 1;
        }

        Ok(vec![RuntimeTrace {
            timestamp_cycles: self.total_cycles,
            event_type: RuntimeEventType::MemoryTransfer,
            description: format!(
                "Copy {} bytes: buf#{} -> buf#{} ({} cycles)",
                size, src_id, dst_id, cycles
            ),
            queue_type: Some(self.queue_type),
            command_buffer_id: None,
            fence_id: None,
            semaphore_id: None,
        }])
    }

    fn exec_fill_buffer(
        &mut self,
        args: &HashMap<String, CommandArg>,
        memory_manager: &mut MemoryManager,
    ) -> Result<Vec<RuntimeTrace>, String> {
        let buffer_id = args.get("buffer_id").unwrap().as_usize();
        let value = args.get("value").unwrap().as_u8();
        let offset = args.get("offset").unwrap().as_usize();
        let size = args.get("size").unwrap().as_usize();

        let buf_data = memory_manager.get_buffer_data_mut(buffer_id);
        for byte in &mut buf_data[offset..offset + size] {
            *byte = value;
        }

        // Sync to device
        let buf = memory_manager.get_buffer(buffer_id)?;
        let address = buf.device_address;
        let fill_bytes = vec![value; size];
        memory_manager
            .device_mut()
            .memcpy_host_to_device(address + offset as u64, &fill_bytes);

        let stats = memory_manager.stats_ptr();
        unsafe {
            (*stats).total_transfers += 1;
        }

        Ok(vec![])
    }

    fn exec_update_buffer(
        &mut self,
        args: &HashMap<String, CommandArg>,
        memory_manager: &mut MemoryManager,
    ) -> Result<Vec<RuntimeTrace>, String> {
        let buffer_id = args.get("buffer_id").unwrap().as_usize();
        let offset = args.get("offset").unwrap().as_usize();
        let data = args.get("data").unwrap().as_bytes().to_vec();

        let buf_data = memory_manager.get_buffer_data_mut(buffer_id);
        buf_data[offset..offset + data.len()].copy_from_slice(&data);

        // Sync to device
        let buf = memory_manager.get_buffer(buffer_id)?;
        let address = buf.device_address;
        memory_manager
            .device_mut()
            .memcpy_host_to_device(address + offset as u64, &data);

        let stats = memory_manager.stats_ptr();
        unsafe {
            (*stats).total_transfers += 1;
        }

        Ok(vec![])
    }

    fn exec_pipeline_barrier(
        &mut self,
        args: &HashMap<String, CommandArg>,
        memory_manager: &mut MemoryManager,
    ) -> Result<Vec<RuntimeTrace>, String> {
        let stats = memory_manager.stats_ptr();
        unsafe {
            (*stats).total_barriers += 1;
        }

        let src = args.get("src_stage").unwrap().as_str_val();
        let dst = args.get("dst_stage").unwrap().as_str_val();

        Ok(vec![RuntimeTrace {
            timestamp_cycles: self.total_cycles,
            event_type: RuntimeEventType::Barrier,
            description: format!("Barrier: {} -> {}", src, dst),
            queue_type: Some(self.queue_type),
            command_buffer_id: None,
            fence_id: None,
            semaphore_id: None,
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_creation() {
        let q = CommandQueue::new(QueueType::Compute, 0);
        assert_eq!(q.queue_type(), QueueType::Compute);
        assert_eq!(q.queue_index(), 0);
        assert_eq!(q.total_cycles(), 0);
    }

    #[test]
    fn test_wait_idle() {
        let q = CommandQueue::new(QueueType::Transfer, 1);
        q.wait_idle(); // should not panic
    }
}
