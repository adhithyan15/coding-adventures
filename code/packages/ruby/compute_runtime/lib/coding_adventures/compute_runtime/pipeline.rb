# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Pipeline -- compiled kernels, descriptor sets, shader modules.
# ---------------------------------------------------------------------------
#
# === What is a Pipeline? ===
#
# A pipeline is a **compiled kernel ready to execute**. In Vulkan terms, it
# packages three things together:
#
#     1. ShaderModule -- the compiled program (instructions)
#     2. PipelineLayout -- what data the kernel expects (descriptor set layout)
#     3. Pipeline -- the combined, ready-to-dispatch object
#
# Think of it like a function call:
#     - ShaderModule = the function body (code)
#     - DescriptorSetLayout = the function signature (parameter types)
#     - DescriptorSet = the actual arguments (concrete buffers)
#     - Pipeline = the compiled function ready to call
#
# === Why Separate Shader from Pipeline? ===
#
# The same shader code can be used in multiple pipelines with different
# layouts. And the same pipeline can be used with different descriptor sets
# (different data). This separation enables reuse.

module CodingAdventures
  module ComputeRuntime
    # =====================================================================
    # ShaderModule -- compiled program
    # =====================================================================
    #
    # === GPU vs Dataflow ===
    #
    # For GPU-style devices (NVIDIA, AMD, Intel), the code is a list of
    # instructions from our GenericISA (gpu-core package).
    #
    # For dataflow-style devices (TPU, ANE), the code is an operation
    # descriptor -- just the operation name and parameters.
    #
    # The shader module doesn't care which -- it stores whatever code was
    # given. The pipeline compilation step adapts it to the target device.
    class ShaderModule
      @@next_id = 0

      attr_reader :module_id, :code, :operation, :entry_point, :local_size

      def initialize(code: nil, operation: "", entry_point: "main", local_size: [32, 1, 1])
        @module_id = @@next_id
        @@next_id += 1
        @code = code
        @operation = operation
        @entry_point = entry_point
        @local_size = local_size
      end

      # True if this is a GPU-style shader (has instruction code).
      def gpu_style?
        !@code.nil?
      end

      # True if this is a dataflow-style shader (has operation name).
      def dataflow_style?
        !@operation.empty?
      end
    end

    # =====================================================================
    # DescriptorSetLayout -- describes the shape of data bindings
    # =====================================================================
    #
    # === What is a Layout? ===
    #
    # A layout is like a function signature -- it says "this kernel takes
    # 3 storage buffers." It doesn't say WHICH buffers, just how many
    # and what type.
    #
    # The actual buffer assignments happen when you create a DescriptorSet
    # from this layout and call write on it.
    #
    # Example:
    #     layout = DescriptorSetLayout.new([
    #         DescriptorBinding.new(binding: 0, type: "storage"),  # input X
    #         DescriptorBinding.new(binding: 1, type: "storage"),  # input Y
    #         DescriptorBinding.new(binding: 2, type: "storage"),  # output Z
    #     ])
    class DescriptorSetLayout
      @@next_id = 0

      attr_reader :layout_id, :bindings

      def initialize(bindings)
        @layout_id = @@next_id
        @@next_id += 1
        @bindings = bindings.freeze
      end
    end

    # =====================================================================
    # PipelineLayout -- shader + descriptor layout + push constants
    # =====================================================================
    #
    # Combines:
    # - Descriptor set layouts (what buffers the kernel reads/writes)
    # - Push constant size (small inline data like alpha in SAXPY)
    class PipelineLayout
      @@next_id = 0

      attr_reader :layout_id, :set_layouts, :push_constant_size

      def initialize(set_layouts, push_constant_size: 0)
        @layout_id = @@next_id
        @@next_id += 1
        @set_layouts = set_layouts.dup
        @push_constant_size = push_constant_size
      end
    end

    # =====================================================================
    # Pipeline -- compiled, ready to dispatch
    # =====================================================================
    #
    # === Creating a Pipeline ===
    #
    #     shader = device.create_shader_module(code: [...], local_size: [256, 1, 1])
    #     layout = device.create_pipeline_layout(set_layouts: [ds_layout])
    #     pipeline = device.create_compute_pipeline(shader, layout)
    #
    # Once created, bind it in a command buffer:
    #     cb.cmd_bind_pipeline(pipeline)
    #     cb.cmd_dispatch(grid_x, grid_y, grid_z)
    class Pipeline
      @@next_id = 0

      attr_reader :pipeline_id, :shader, :layout

      def initialize(shader, layout)
        @pipeline_id = @@next_id
        @@next_id += 1
        @shader = shader
        @layout = layout
      end

      # Local workgroup dimensions from the shader.
      def workgroup_size
        @shader.local_size
      end
    end

    # =====================================================================
    # DescriptorSet -- concrete buffer bindings
    # =====================================================================
    #
    # === Layout vs Set ===
    #
    # Layout says: "binding 0 is a storage buffer"
    # Set says:    "binding 0 is buf_x (address 0x1000, 4096 bytes)"
    #
    # You create a set from a layout, then write buffers into it.
    # Multiple sets can share the same layout with different buffers.
    class DescriptorSet
      @@next_id = 0

      attr_reader :set_id, :layout

      def initialize(layout)
        @set_id = @@next_id
        @@next_id += 1
        @layout = layout
        @bindings = {}
      end

      # Current buffer bindings (binding number -> Buffer).
      def bindings
        @bindings.dup
      end

      # Bind a buffer to a slot.
      #
      # @param binding [Integer] Slot number (must exist in layout).
      # @param buffer [Buffer] The buffer to bind.
      # @raise [ArgumentError] If binding doesn't exist in layout or buffer is freed.
      def write(binding, buffer)
        valid_bindings = @layout.bindings.map(&:binding).to_set
        unless valid_bindings.include?(binding)
          raise ArgumentError,
            "Binding #{binding} not in layout (valid: #{valid_bindings.to_a})"
        end
        if buffer.freed
          raise ArgumentError,
            "Cannot bind freed buffer #{buffer.buffer_id} to binding #{binding}"
        end
        @bindings[binding] = buffer
      end

      # Get the buffer at a binding slot, or nil if not bound.
      def get_buffer(binding)
        @bindings[binding]
      end
    end
  end
end
