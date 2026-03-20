# frozen_string_literal: true

# ---------------------------------------------------------------------------
# ValidationLayer -- catches GPU programming errors early.
# ---------------------------------------------------------------------------
#
# === What is a Validation Layer? ===
#
# In Vulkan, validation layers are optional middleware that check every API
# call for errors. They're enabled during development and disabled in
# production (for performance). Common errors they catch:
#
#     - Dispatching without binding a pipeline
#     - Using a freed buffer in a descriptor set
#     - Missing a barrier between write and read
#     - Mapping a DEVICE_LOCAL-only buffer
#     - Exceeding device limits
#
# Our validation layer wraps a LogicalDevice and checks every operation.
# It's always enabled (since we're a simulator, not a production runtime).
#
# === Usage ===
#
#     device = instance.create_logical_device(physical)
#     validated = ValidationLayer.new
#
#     # Use validated.validate_* before each operation
#     validated.validate_begin(cb)
#     cb.begin

module CodingAdventures
  module ComputeRuntime
    # Raised when a validation check fails.
    #
    # These errors represent GPU programming mistakes -- things that would
    # cause undefined behavior or crashes on real hardware.
    class ValidationError < StandardError; end

    # Validates runtime operations and raises clear error messages.
    #
    # === What It Checks ===
    #
    # 1. Command buffer state transitions
    # 2. Pipeline/descriptor binding
    # 3. Memory type compatibility
    # 4. Buffer usage flags
    # 5. Freed resource detection
    # 6. Barrier correctness
    class ValidationLayer
      attr_reader :warnings, :errors

      def initialize
        @warnings = []
        @errors = []
        # Track which buffers have been written to (for barrier checking)
        @written_buffers = Set.new
        # Track which buffers have barriers protecting reads
        @barriered_buffers = Set.new
      end

      # Clear all warnings and errors.
      def clear
        @warnings.clear
        @errors.clear
        @written_buffers.clear
        @barriered_buffers.clear
      end

      # --- Command buffer validation ---

      # Validate that begin is allowed.
      def validate_begin(cb)
        unless %i[initial complete].include?(cb.state)
          raise ValidationError,
            "Cannot begin CB##{cb.command_buffer_id}: " \
            "state is #{cb.state} (expected initial or complete)"
        end
      end

      # Validate that end_recording is allowed.
      def validate_end(cb)
        unless cb.state == :recording
          raise ValidationError,
            "Cannot end CB##{cb.command_buffer_id}: " \
            "state is #{cb.state} (expected recording)"
        end
      end

      # Validate that a CB can be submitted.
      def validate_submit(cb)
        unless cb.state == :recorded
          raise ValidationError,
            "Cannot submit CB##{cb.command_buffer_id}: " \
            "state is #{cb.state} (expected recorded)"
        end
      end

      # --- Dispatch validation ---

      # Validate a dispatch command.
      def validate_dispatch(cb, group_x, group_y, group_z)
        if cb.bound_pipeline.nil?
          raise ValidationError,
            "Cannot dispatch in CB##{cb.command_buffer_id}: " \
            "no pipeline bound (call cmd_bind_pipeline first)"
        end
        if group_x <= 0 || group_y <= 0 || group_z <= 0
          raise ValidationError,
            "Dispatch dimensions must be positive: " \
            "(#{group_x}, #{group_y}, #{group_z})"
        end
      end

      # --- Memory validation ---

      # Validate that a buffer can be mapped.
      def validate_map(buffer)
        if buffer.freed
          raise ValidationError,
            "Cannot map freed buffer #{buffer.buffer_id}"
        end
        if buffer.mapped
          raise ValidationError,
            "Buffer #{buffer.buffer_id} is already mapped"
        end
        unless (buffer.memory_type & MemoryType::HOST_VISIBLE) != 0
          raise ValidationError,
            "Cannot map buffer #{buffer.buffer_id}: " \
            "not HOST_VISIBLE (type=#{buffer.memory_type}). " \
            "Use a staging buffer for DEVICE_LOCAL memory."
        end
      end

      # Validate that a buffer has the required usage flags.
      def validate_buffer_usage(buffer, required_usage)
        unless (buffer.usage & required_usage) != 0
          raise ValidationError,
            "Buffer #{buffer.buffer_id} lacks required usage " \
            "#{required_usage} (has #{buffer.usage})"
        end
      end

      # Validate that a buffer is not freed.
      def validate_buffer_not_freed(buffer)
        if buffer.freed
          raise ValidationError,
            "Buffer #{buffer.buffer_id} has been freed"
        end
      end

      # --- Barrier validation ---

      # Record that a buffer was written to (for barrier checking).
      def record_write(buffer_id)
        @written_buffers.add(buffer_id)
        @barriered_buffers.delete(buffer_id)
      end

      # Record that a barrier was placed (covers some/all buffers).
      def record_barrier(buffer_ids: nil)
        if buffer_ids.nil?
          # Global barrier -- covers all written buffers
          @barriered_buffers.merge(@written_buffers)
        else
          @barriered_buffers.merge(buffer_ids)
        end
      end

      # Warn if reading a buffer that was written without a barrier.
      def validate_read_after_write(buffer_id)
        if @written_buffers.include?(buffer_id) && !@barriered_buffers.include?(buffer_id)
          @warnings << "Reading buffer #{buffer_id} after write without barrier. " \
                       "Insert cmd_pipeline_barrier() between write and read."
        end
      end

      # --- Descriptor set validation ---

      # Validate that a descriptor set is compatible with a pipeline.
      def validate_descriptor_set(descriptor_set, pipeline)
        layout = pipeline.layout
        return if layout.set_layouts.empty?

        expected_layout = layout.set_layouts[0]
        expected_layout.bindings.each do |binding_def|
          buf = descriptor_set.get_buffer(binding_def.binding)
          if buf.nil?
            @warnings << "Binding #{binding_def.binding} not set in " \
                         "descriptor set #{descriptor_set.set_id}"
          elsif buf.freed
            raise ValidationError,
              "Binding #{binding_def.binding} uses freed buffer " \
              "#{buf.buffer_id}"
          end
        end
      end
    end
  end
end
