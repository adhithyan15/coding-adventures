# frozen_string_literal: true

# ---------------------------------------------------------------------------
# CommandBuffer -- recorded sequence of GPU commands.
# ---------------------------------------------------------------------------
#
# === The Record-Then-Submit Model ===
#
# Instead of calling GPU operations one at a time (like CUDA), Vulkan records
# commands into a buffer and submits the whole buffer at once. This is the
# single most important concept in Vulkan:
#
#     # CUDA style (implicit, one at a time):
#     cudaMemcpy(dst, src, size)     # executes immediately
#     kernel<<<grid, block>>>(args)  # executes immediately
#
#     # Vulkan style (explicit, batched):
#     cb.begin()                     # start recording
#     cb.cmd_copy_buffer(...)        # just records -- doesn't execute
#     cb.cmd_dispatch(...)           # just records -- doesn't execute
#     cb.end_recording()             # stop recording
#     queue.submit([cb])             # NOW everything executes
#
# === Why Batch? ===
#
# 1. **Driver optimization** -- the driver sees all commands at once and can
#    reorder, merge, or eliminate redundancies.
#
# 2. **Reuse** -- submit the same CB multiple times without re-recording.
#
# 3. **Multi-threaded recording** -- different CPU threads record different
#    CBs in parallel, then submit them together.
#
# 4. **Validation** -- check the entire sequence for errors before any GPU
#    work starts.
#
# === State Machine ===
#
# A command buffer has strict states:
#
#     INITIAL --begin()--> RECORDING --end()--> RECORDED --submit()--> PENDING
#        ^                                                                |
#        +---------------------- reset() <-- COMPLETE <--------------------+

module CodingAdventures
  module ComputeRuntime
    class CommandBuffer
      @@next_id = 0

      attr_reader :command_buffer_id

      def initialize
        @command_buffer_id = @@next_id
        @@next_id += 1
        @state = :initial
        @commands = []

        # Currently bound state (for validation)
        @bound_pipeline = nil
        @bound_descriptor_set = nil
        @push_constants = "".b
      end

      # Current lifecycle state (Symbol).
      def state = @state

      # All recorded commands (Array<RecordedCommand>).
      def commands = @commands.dup

      # Currently bound pipeline (for validation).
      def bound_pipeline = @bound_pipeline

      # Currently bound descriptor set (for validation).
      def bound_descriptor_set = @bound_descriptor_set

      # =================================================================
      # Lifecycle
      # =================================================================

      # Start recording commands.
      #
      # Transitions: :initial -> :recording, or :complete -> :recording (reuse).
      #
      # @raise [RuntimeError] If not in :initial or :complete state.
      def begin
        unless %i[initial complete].include?(@state)
          raise RuntimeError,
            "Cannot begin recording: state is #{@state} " \
            "(expected initial or complete)"
        end
        @state = :recording
        @commands.clear
        @bound_pipeline = nil
        @bound_descriptor_set = nil
        @push_constants = "".b
      end

      # Finish recording commands.
      #
      # Transitions: :recording -> :recorded.
      #
      # @raise [RuntimeError] If not in :recording state.
      def end_recording
        unless @state == :recording
          raise RuntimeError,
            "Cannot end recording: state is #{@state} " \
            "(expected recording)"
        end
        @state = :recorded
      end

      # Reset to :initial state for reuse. Clears all recorded commands.
      def reset
        @state = :initial
        @commands.clear
        @bound_pipeline = nil
        @bound_descriptor_set = nil
        @push_constants = "".b
      end

      # Internal: mark as submitted (called by CommandQueue).
      def _mark_pending
        @state = :pending
      end

      # Internal: mark as finished (called by CommandQueue).
      def _mark_complete
        @state = :complete
      end

      # Internal: ensure we're in :recording state.
      def _require_recording
        unless @state == :recording
          raise RuntimeError,
            "Cannot record command: state is #{@state} " \
            "(expected recording)"
        end
      end

      # =================================================================
      # Compute commands
      # =================================================================

      # Bind a compute pipeline for subsequent dispatches.
      #
      # Must be called before cmd_dispatch.
      #
      # @param pipeline [Pipeline] The compiled pipeline to bind.
      def cmd_bind_pipeline(pipeline)
        _require_recording
        @bound_pipeline = pipeline
        @commands << RecordedCommand.new(
          command: "bind_pipeline",
          args: {pipeline_id: pipeline.pipeline_id}
        )
      end

      # Bind a descriptor set for subsequent dispatches.
      #
      # @param descriptor_set [DescriptorSet] The descriptor set with buffer bindings.
      def cmd_bind_descriptor_set(descriptor_set)
        _require_recording
        @bound_descriptor_set = descriptor_set
        @commands << RecordedCommand.new(
          command: "bind_descriptor_set",
          args: {set_id: descriptor_set.set_id}
        )
      end

      # Set push constant data for the next dispatch.
      #
      # Push constants are small pieces of data (<= 128 bytes) sent inline
      # with the dispatch command.
      #
      # @param offset [Integer] Byte offset into the push constant range.
      # @param data [String] The bytes to set (binary string).
      def cmd_push_constants(offset, data)
        _require_recording
        @push_constants = data
        @commands << RecordedCommand.new(
          command: "push_constants",
          args: {offset: offset, size: data.bytesize}
        )
      end

      # Launch a compute kernel.
      #
      # === Dispatch Dimensions ===
      #
      # The dispatch creates a 3D grid of workgroups:
      #
      #     Total threads = (group_x * group_y * group_z) *
      #                    (local_x * local_y * local_z)
      #
      # @param group_x [Integer] Workgroups in X dimension.
      # @param group_y [Integer] Workgroups in Y dimension.
      # @param group_z [Integer] Workgroups in Z dimension.
      # @raise [RuntimeError] If no pipeline is bound.
      def cmd_dispatch(group_x, group_y = 1, group_z = 1)
        _require_recording
        raise RuntimeError, "Cannot dispatch: no pipeline bound" if @bound_pipeline.nil?
        @commands << RecordedCommand.new(
          command: "dispatch",
          args: {group_x: group_x, group_y: group_y, group_z: group_z}
        )
      end

      # Launch a compute kernel with grid dimensions from a GPU buffer.
      #
      # @param buffer [Buffer] Buffer containing dispatch dimensions.
      # @param offset [Integer] Byte offset into the buffer.
      def cmd_dispatch_indirect(buffer, offset: 0)
        _require_recording
        raise RuntimeError, "Cannot dispatch: no pipeline bound" if @bound_pipeline.nil?
        @commands << RecordedCommand.new(
          command: "dispatch_indirect",
          args: {buffer_id: buffer.buffer_id, offset: offset}
        )
      end

      # =================================================================
      # Transfer commands
      # =================================================================

      # Copy data between device buffers.
      #
      # @param src [Buffer] Source buffer.
      # @param dst [Buffer] Destination buffer.
      # @param size [Integer] Bytes to copy.
      # @param src_offset [Integer] Byte offset in source.
      # @param dst_offset [Integer] Byte offset in destination.
      def cmd_copy_buffer(src, dst, size, src_offset: 0, dst_offset: 0)
        _require_recording
        @commands << RecordedCommand.new(
          command: "copy_buffer",
          args: {
            src_id: src.buffer_id,
            dst_id: dst.buffer_id,
            size: size,
            src_offset: src_offset,
            dst_offset: dst_offset
          }
        )
      end

      # Fill a buffer with a constant byte value.
      #
      # @param buffer [Buffer] The buffer to fill.
      # @param value [Integer] Byte value to fill with (0-255).
      # @param offset [Integer] Byte offset to start filling.
      # @param size [Integer] Bytes to fill (0 = whole buffer).
      def cmd_fill_buffer(buffer, value, offset: 0, size: 0)
        _require_recording
        @commands << RecordedCommand.new(
          command: "fill_buffer",
          args: {
            buffer_id: buffer.buffer_id,
            value: value,
            offset: offset,
            size: size > 0 ? size : buffer.size
          }
        )
      end

      # Write small data inline from CPU to device buffer.
      #
      # @param buffer [Buffer] Destination buffer.
      # @param offset [Integer] Byte offset in the buffer.
      # @param data [String] Bytes to write (binary string).
      def cmd_update_buffer(buffer, offset, data)
        _require_recording
        @commands << RecordedCommand.new(
          command: "update_buffer",
          args: {
            buffer_id: buffer.buffer_id,
            offset: offset,
            data: data
          }
        )
      end

      # =================================================================
      # Synchronization commands
      # =================================================================

      # Insert an execution + memory barrier.
      #
      # @param barrier [PipelineBarrier] The barrier specification.
      def cmd_pipeline_barrier(barrier)
        _require_recording
        @commands << RecordedCommand.new(
          command: "pipeline_barrier",
          args: {
            src_stage: barrier.src_stage.to_s,
            dst_stage: barrier.dst_stage.to_s,
            memory_barrier_count: barrier.memory_barriers.length,
            buffer_barrier_count: barrier.buffer_barriers.length
          }
        )
      end

      # Signal an event from the GPU.
      #
      # @param event [Event] The event to signal.
      # @param stage [Symbol] Wait for this stage before signaling.
      def cmd_set_event(event, stage)
        _require_recording
        @commands << RecordedCommand.new(
          command: "set_event",
          args: {event_id: event.event_id, stage: stage.to_s}
        )
      end

      # Wait for an event before proceeding.
      #
      # @param event [Event] The event to wait on.
      # @param src_stage [Symbol] The stage that set the event.
      # @param dst_stage [Symbol] The stage that should wait.
      def cmd_wait_event(event, src_stage, dst_stage)
        _require_recording
        @commands << RecordedCommand.new(
          command: "wait_event",
          args: {
            event_id: event.event_id,
            src_stage: src_stage.to_s,
            dst_stage: dst_stage.to_s
          }
        )
      end

      # Reset an event from the GPU side.
      #
      # @param event [Event] The event to reset.
      # @param stage [Symbol] Wait for this stage before resetting.
      def cmd_reset_event(event, stage)
        _require_recording
        @commands << RecordedCommand.new(
          command: "reset_event",
          args: {event_id: event.event_id, stage: stage.to_s}
        )
      end
    end
  end
end
