# frozen_string_literal: true

# ---------------------------------------------------------------------------
# CommandQueue -- FIFO submission of command buffers to a device.
# ---------------------------------------------------------------------------
#
# === How Submission Works ===
#
# When you submit command buffers to a queue, the runtime processes them
# sequentially, executing each recorded command against the Layer 6 device:
#
#     queue.submit([cb1, cb2], fence: fence)
#         |
#         +-- Execute cb1's commands:
#         |   +-- bind_pipeline -> set current pipeline
#         |   +-- bind_descriptor_set -> set current descriptors
#         |   +-- dispatch(4, 1, 1) -> device.launch_kernel + device.run
#         |   +-- pipeline_barrier -> (ensure completion, log trace)
#         |
#         +-- Execute cb2's commands:
#         |   +-- copy_buffer -> device.memcpy
#         |   +-- ...
#         |
#         +-- Signal semaphores (if any)
#         +-- Signal fence (if any)
#
# === Multiple Queues ===
#
# A device can have multiple queues. Queues of different types (compute,
# transfer) can execute in parallel -- while the compute queue runs a kernel,
# the transfer queue can copy data.

module CodingAdventures
  module ComputeRuntime
    class CommandQueue
      attr_reader :queue_type, :queue_index, :total_cycles

      def initialize(queue_type:, queue_index:, device:, memory_manager:, stats:)
        @queue_type = queue_type
        @queue_index = queue_index
        @device = device
        @memory_manager = memory_manager
        @stats = stats
        @total_cycles = 0

        # Execution state
        @current_pipeline = nil
        @current_descriptor_set = nil
        @current_push_constants = "".b
      end

      # Submit command buffers for execution.
      #
      # === Submission Flow ===
      #
      # 1. Wait for all wait_semaphores to be signaled
      # 2. Execute each command buffer sequentially
      # 3. Signal all signal_semaphores
      # 4. Signal the fence (if provided)
      #
      # @param command_buffers [Array<CommandBuffer>] CBs to execute (in order).
      # @param wait_semaphores [Array<Semaphore>, nil] Wait for these before starting.
      # @param signal_semaphores [Array<Semaphore>, nil] Signal these when all CBs complete.
      # @param fence [Fence, nil] Signal this fence when done (for CPU waiting).
      # @return [Array<RuntimeTrace>] List of events generated during execution.
      # @raise [RuntimeError] If any CB is not in :recorded state.
      # @raise [RuntimeError] If a wait_semaphore is not signaled.
      def submit(command_buffers, wait_semaphores: nil, signal_semaphores: nil, fence: nil)
        traces = []
        wait_sems = wait_semaphores || []
        signal_sems = signal_semaphores || []

        # Validate CB states
        command_buffers.each do |cb|
          unless cb.state == :recorded
            raise RuntimeError,
              "CB##{cb.command_buffer_id} is in state #{cb.state}, " \
              "expected recorded"
          end
        end

        # Wait on semaphores
        wait_sems.each do |sem|
          unless sem.signaled
            raise RuntimeError,
              "Semaphore #{sem.semaphore_id} is not signaled -- " \
              "cannot proceed (possible deadlock)"
          end
          traces << RuntimeTrace.new(
            timestamp_cycles: @total_cycles,
            event_type: :semaphore_wait,
            description: "Wait on semaphore S#{sem.semaphore_id}",
            queue_type: @queue_type,
            semaphore_id: sem.semaphore_id
          )
          sem.reset # Consume the semaphore
        end

        # Log submission
        @stats.total_submissions += 1
        @stats.total_command_buffers += command_buffers.length

        cb_ids = command_buffers.map(&:command_buffer_id)
        traces << RuntimeTrace.new(
          timestamp_cycles: @total_cycles,
          event_type: :submit,
          description: "Submit CB #{cb_ids} to #{@queue_type} queue",
          queue_type: @queue_type
        )

        # Execute each command buffer
        command_buffers.each do |cb|
          cb._mark_pending
          cb_traces = _execute_command_buffer(cb)
          traces.concat(cb_traces)
          cb._mark_complete
        end

        # Signal semaphores
        signal_sems.each do |sem|
          sem.signal
          @stats.total_semaphore_signals += 1
          traces << RuntimeTrace.new(
            timestamp_cycles: @total_cycles,
            event_type: :semaphore_signal,
            description: "Signal semaphore S#{sem.semaphore_id}",
            queue_type: @queue_type,
            semaphore_id: sem.semaphore_id
          )
        end

        # Signal fence
        if fence
          fence.signal
          traces << RuntimeTrace.new(
            timestamp_cycles: @total_cycles,
            event_type: :fence_signal,
            description: "Signal fence F#{fence.fence_id}",
            queue_type: @queue_type,
            fence_id: fence.fence_id
          )
        end

        # Update stats
        @stats.total_device_cycles = @total_cycles
        @stats.update_utilization
        @stats.traces.concat(traces)

        traces
      end

      # Block until this queue has no pending work.
      #
      # In our synchronous simulation, submit always runs to completion,
      # so this is a no-op.
      def wait_idle
        # No-op in synchronous simulation
      end

      private

      # Execute all commands in a command buffer.
      def _execute_command_buffer(cb)
        traces = []

        # Replay the CB's bind state
        @current_pipeline = cb.bound_pipeline
        @current_descriptor_set = cb.bound_descriptor_set

        traces << RuntimeTrace.new(
          timestamp_cycles: @total_cycles,
          event_type: :begin_execution,
          description: "Begin CB##{cb.command_buffer_id}",
          queue_type: @queue_type,
          command_buffer_id: cb.command_buffer_id
        )

        cb.commands.each do |cmd|
          cmd_traces = _execute_command(cmd)
          traces.concat(cmd_traces)
        end

        traces << RuntimeTrace.new(
          timestamp_cycles: @total_cycles,
          event_type: :end_execution,
          description: "End CB##{cb.command_buffer_id}",
          queue_type: @queue_type,
          command_buffer_id: cb.command_buffer_id
        )

        traces
      end

      # Execute a single recorded command against the device.
      def _execute_command(cmd)
        case cmd.command
        when "bind_pipeline"       then _exec_bind_pipeline(cmd.args)
        when "bind_descriptor_set" then _exec_bind_descriptor_set(cmd.args)
        when "push_constants"      then _exec_push_constants(cmd.args)
        when "dispatch"            then _exec_dispatch(cmd.args)
        when "dispatch_indirect"   then _exec_dispatch_indirect(cmd.args)
        when "copy_buffer"         then _exec_copy_buffer(cmd.args)
        when "fill_buffer"         then _exec_fill_buffer(cmd.args)
        when "update_buffer"       then _exec_update_buffer(cmd.args)
        when "pipeline_barrier"    then _exec_pipeline_barrier(cmd.args)
        when "set_event"           then _exec_set_event(cmd.args)
        when "wait_event"          then _exec_wait_event(cmd.args)
        when "reset_event"         then _exec_reset_event(cmd.args)
        else
          raise RuntimeError, "Unknown command: #{cmd.command}"
        end
      end

      # =================================================================
      # Command executors
      # =================================================================

      def _exec_bind_pipeline(_args) = []

      def _exec_bind_descriptor_set(_args) = []

      def _exec_push_constants(_args) = []

      def _exec_dispatch(args)
        group_x = args[:group_x]
        group_y = args[:group_y]
        group_z = args[:group_z]

        pipeline = @current_pipeline
        raise RuntimeError, "No pipeline bound for dispatch" if pipeline.nil?

        shader = pipeline.shader

        if shader.gpu_style?
          kernel = DeviceSimulator::KernelDescriptor.new(
            name: "dispatch_#{group_x}x#{group_y}x#{group_z}",
            program: shader.code,
            grid_dim: [group_x, group_y, group_z],
            block_dim: shader.local_size
          )
        else
          # Dataflow-style dispatch
          kernel = DeviceSimulator::KernelDescriptor.new(
            name: "op_#{shader.operation}",
            operation: shader.operation,
            input_data: [[1.0]],
            weight_data: [[1.0]]
          )
        end

        @device.launch_kernel(kernel)
        device_traces = @device.run(10_000)
        cycles = device_traces.length
        @total_cycles += cycles

        @stats.total_dispatches += 1

        [
          RuntimeTrace.new(
            timestamp_cycles: @total_cycles,
            event_type: :end_execution,
            description: "Dispatch (#{group_x},#{group_y},#{group_z}) " \
                         "completed in #{cycles} cycles",
            queue_type: @queue_type,
            device_traces: device_traces
          )
        ]
      end

      def _exec_dispatch_indirect(args)
        buffer_id = args[:buffer_id]
        offset = args[:offset]

        data = @memory_manager._get_buffer_data(buffer_id)
        group_x, group_y, group_z = data.unpack("VVV") # little-endian uint32

        _exec_dispatch({group_x: group_x, group_y: group_y, group_z: group_z})
      end

      def _exec_copy_buffer(args)
        src_id = args[:src_id]
        dst_id = args[:dst_id]
        size = args[:size]
        src_offset = args.fetch(:src_offset, 0)
        dst_offset = args.fetch(:dst_offset, 0)

        src_data = @memory_manager._get_buffer_data(src_id)
        dst_data = @memory_manager._get_buffer_data(dst_id)

        # Copy the bytes
        dst_data[dst_offset, size] = src_data.byteslice(src_offset, size)

        # Also sync to device memory
        src_buf = @memory_manager.get_buffer(src_id)
        dst_buf = @memory_manager.get_buffer(dst_id)

        data_bytes, read_cycles = @device.memcpy_device_to_host(
          src_buf.device_address + src_offset, size
        )
        write_cycles = @device.memcpy_host_to_device(
          dst_buf.device_address + dst_offset, data_bytes
        )

        cycles = read_cycles + write_cycles
        @total_cycles += cycles
        @stats.total_transfers += 1

        [
          RuntimeTrace.new(
            timestamp_cycles: @total_cycles,
            event_type: :memory_transfer,
            description: "Copy #{size} bytes: buf##{src_id} -> buf##{dst_id} " \
                         "(#{cycles} cycles)",
            queue_type: @queue_type
          )
        ]
      end

      def _exec_fill_buffer(args)
        buffer_id = args[:buffer_id]
        value = args[:value]
        offset = args[:offset]
        size = args[:size]

        buf_data = @memory_manager._get_buffer_data(buffer_id)
        fill_byte = (value & 0xFF).chr.b
        buf_data[offset, size] = fill_byte * size

        # Sync to device
        buf = @memory_manager.get_buffer(buffer_id)
        @device.memcpy_host_to_device(
          buf.device_address + offset, fill_byte * size
        )

        @stats.total_transfers += 1
        []
      end

      def _exec_update_buffer(args)
        buffer_id = args[:buffer_id]
        offset = args[:offset]
        data = args[:data]

        buf_data = @memory_manager._get_buffer_data(buffer_id)
        buf_data[offset, data.bytesize] = data

        # Sync to device
        buf = @memory_manager.get_buffer(buffer_id)
        @device.memcpy_host_to_device(buf.device_address + offset, data)

        @stats.total_transfers += 1
        []
      end

      def _exec_pipeline_barrier(args)
        @stats.total_barriers += 1
        [
          RuntimeTrace.new(
            timestamp_cycles: @total_cycles,
            event_type: :barrier,
            description: "Barrier: #{args[:src_stage]} -> #{args[:dst_stage]}",
            queue_type: @queue_type
          )
        ]
      end

      def _exec_set_event(_args) = []

      def _exec_wait_event(_args) = []

      def _exec_reset_event(_args) = []
    end
  end
end
