# frozen_string_literal: true

# ---------------------------------------------------------------------------
# OpenGL Compute Simulator -- the legacy global state machine.
# ---------------------------------------------------------------------------
#
# === What is OpenGL? ===
#
# OpenGL is the oldest surviving GPU API (1992). Compute shaders were bolted
# on in OpenGL 4.3 (2012), long after the core API was designed around
# graphics rendering. This heritage shows: OpenGL uses a *global state
# machine* model where you bind things to "current" state and then issue
# commands that operate on whatever is currently bound.
#
# === The State Machine Model ===
#
# Unlike Vulkan (explicit objects) or Metal (scoped encoders), OpenGL
# maintains global state:
#
#     gl.use_program(prog)           # Sets "current program" globally
#     gl.bind_buffer_base(GL_SHADER_STORAGE_BUFFER, 0, buf_a)
#     gl.dispatch_compute(4, 1, 1)   # Uses WHATEVER is currently bound
#
# === Integer Handles ===
#
# OpenGL uses integer handles (GLuint) for everything. You never get a
# typed object -- just a number:
#
#     shader = gl.create_shader(GL_COMPUTE_SHADER)  # Returns 1
#     program = gl.create_program                     # Returns 2
#     buffers = gl.gen_buffers(2)                    # Returns [3, 4]

module CodingAdventures
  module VendorApiSimulators
    # =====================================================================
    # OpenGL constants -- module-level, just like real OpenGL
    # =====================================================================

    # Shader types
    GL_COMPUTE_SHADER = 0x91B9

    # Buffer targets
    GL_SHADER_STORAGE_BUFFER = 0x90D2
    GL_ARRAY_BUFFER          = 0x8892
    GL_UNIFORM_BUFFER        = 0x8A11

    # Buffer usage hints
    GL_STATIC_DRAW  = 0x88E4
    GL_DYNAMIC_DRAW = 0x88E8
    GL_STREAM_DRAW  = 0x88E0

    # Map access bits
    GL_MAP_READ_BIT  = 0x0001
    GL_MAP_WRITE_BIT = 0x0002

    # Memory barrier bits
    GL_SHADER_STORAGE_BARRIER_BIT = 0x00002000
    GL_BUFFER_UPDATE_BARRIER_BIT  = 0x00000200
    GL_ALL_BARRIER_BITS           = 0xFFFFFFFF

    # Sync object results
    GL_ALREADY_SIGNALED    = 0x911A
    GL_CONDITION_SATISFIED = 0x911C
    GL_TIMEOUT_EXPIRED     = 0x911B
    GL_WAIT_FAILED         = 0x911D

    # Sync flags
    GL_SYNC_FLUSH_COMMANDS_BIT    = 0x00000001
    GL_SYNC_GPU_COMMANDS_COMPLETE = 0x9117

    # =====================================================================
    # GLContext -- the main OpenGL state machine
    # =====================================================================

    # OpenGL context -- a global state machine for GPU programming.
    #
    # === Usage ===
    #
    #     gl = GLContext.new
    #
    #     # Create shader and program
    #     shader = gl.create_shader(GL_COMPUTE_SHADER)
    #     gl.shader_source(shader, "saxpy")
    #     gl.compile_shader(shader)
    #     program = gl.create_program
    #     gl.attach_shader(program, shader)
    #     gl.link_program(program)
    #
    #     # Create and fill buffers
    #     bufs = gl.gen_buffers(2)
    #     gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
    #     gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 256, data, GL_DYNAMIC_DRAW)
    #
    #     # Bind and dispatch
    #     gl.bind_buffer_base(GL_SHADER_STORAGE_BUFFER, 0, bufs[0])
    #     gl.use_program(program)
    #     gl.dispatch_compute(4, 1, 1)
    #     gl.memory_barrier(GL_SHADER_STORAGE_BARRIER_BIT)
    class GLContext < BaseVendorSimulator
      def initialize
        super

        # === Global State ===
        @current_program = nil
        @bound_buffers = {}       # (target, index) -> GL handle
        @target_buffers = {}      # target -> GL handle

        # === Internal lookup tables ===
        @shaders = {}             # handle -> { source:, code:, compiled:, type: }
        @programs = {}            # handle -> { pipeline:, shaders:, linked:, shader_module: }
        @buffers = {}             # handle -> Layer 5 Buffer
        @syncs = {}               # handle -> Layer 5 Fence
        @uniforms = {}            # (program, name) -> value

        @next_id = 1
      end

      private

      def _gen_id
        handle = @next_id
        @next_id += 1
        handle
      end

      public

      # ===============================================================
      # Shader management
      # ===============================================================

      # Create a shader object (glCreateShader).
      #
      # @param shader_type [Integer] Must be GL_COMPUTE_SHADER.
      # @return [Integer] A GL handle for the shader.
      # @raise [ArgumentError] If shader_type is not GL_COMPUTE_SHADER.
      def create_shader(shader_type)
        unless shader_type == GL_COMPUTE_SHADER
          raise ArgumentError,
            "Only GL_COMPUTE_SHADER (0x#{GL_COMPUTE_SHADER.to_s(16).upcase}) is supported, " \
            "got 0x#{shader_type.to_s(16).upcase}"
        end
        handle = _gen_id
        @shaders[handle] = {
          "source" => "",
          "code" => nil,
          "compiled" => false,
          "type" => shader_type
        }
        handle
      end

      # Set the source code for a shader (glShaderSource).
      #
      # @param shader [Integer] GL shader handle.
      # @param source [String] Shader source code string.
      # @raise [ArgumentError] If shader handle is invalid.
      def shader_source(shader, source)
        raise ArgumentError, "Invalid shader handle #{shader}" unless @shaders.key?(shader)
        @shaders[shader]["source"] = source
      end

      # Compile a shader (glCompileShader).
      #
      # @param shader [Integer] GL shader handle.
      # @raise [ArgumentError] If shader handle is invalid.
      def compile_shader(shader)
        raise ArgumentError, "Invalid shader handle #{shader}" unless @shaders.key?(shader)
        @shaders[shader]["compiled"] = true
      end

      # Delete a shader object (glDeleteShader).
      #
      # @param shader [Integer] GL shader handle.
      def delete_shader(shader)
        @shaders.delete(shader)
      end

      # ===============================================================
      # Program management
      # ===============================================================

      # Create a program object (glCreateProgram).
      #
      # @return [Integer] A GL handle for the program.
      def create_program
        handle = _gen_id
        @programs[handle] = {
          "pipeline" => nil,
          "shaders" => [],
          "linked" => false,
          "shader_module" => nil
        }
        handle
      end

      # Attach a shader to a program (glAttachShader).
      #
      # @param program [Integer] GL program handle.
      # @param shader [Integer] GL shader handle.
      # @raise [ArgumentError] If either handle is invalid.
      def attach_shader(program, shader)
        raise ArgumentError, "Invalid program handle #{program}" unless @programs.key?(program)
        raise ArgumentError, "Invalid shader handle #{shader}" unless @shaders.key?(shader)
        @programs[program]["shaders"] << shader
      end

      # Link a program (glLinkProgram).
      #
      # @param program [Integer] GL program handle.
      # @raise [ArgumentError] If program handle is invalid.
      # @raise [RuntimeError] If no shaders are attached.
      def link_program(program)
        raise ArgumentError, "Invalid program handle #{program}" unless @programs.key?(program)

        prog = @programs[program]
        raise RuntimeError, "Program #{program} has no attached shaders" if prog["shaders"].empty?

        shader_handle = prog["shaders"][0]
        shader_info = @shaders[shader_handle]
        code = shader_info["code"]

        shader = @_logical_device.create_shader_module(code: code)
        ds_layout = @_logical_device.create_descriptor_set_layout([])
        pl_layout = @_logical_device.create_pipeline_layout([ds_layout])
        pipeline = @_logical_device.create_compute_pipeline(shader, pl_layout)

        prog["pipeline"] = pipeline
        prog["shader_module"] = shader
        prog["linked"] = true
      end

      # Set the active program (glUseProgram).
      #
      # @param program [Integer] GL program handle. 0 = unbind.
      # @raise [ArgumentError] If program handle is invalid (and not 0).
      def use_program(program)
        if program == 0
          @current_program = nil
          return
        end
        raise ArgumentError, "Invalid program handle #{program}" unless @programs.key?(program)
        raise RuntimeError, "Program #{program} is not linked" unless @programs[program]["linked"]
        @current_program = program
      end

      # Delete a program object (glDeleteProgram).
      #
      # @param program [Integer] GL program handle.
      def delete_program(program)
        @current_program = nil if @current_program == program
        @programs.delete(program)
      end

      # ===============================================================
      # Buffer management
      # ===============================================================

      # Generate buffer objects (glGenBuffers).
      #
      # @param count [Integer] Number of buffers to create.
      # @return [Array<Integer>] List of GL handles.
      def gen_buffers(count)
        handles = []
        count.times do
          handle = _gen_id
          @buffers[handle] = nil
          handles << handle
        end
        handles
      end

      # Delete buffer objects (glDeleteBuffers).
      #
      # @param buffers [Array<Integer>] List of GL handles to delete.
      def delete_buffers(buffers)
        buffers.each do |handle|
          if @buffers.key?(handle) && !@buffers[handle].nil?
            buf = @buffers[handle]
            @_memory_manager.free(buf) unless buf.freed
          end
          @buffers.delete(handle)
          @bound_buffers.delete_if { |_k, v| v == handle }
          @target_buffers.delete_if { |_k, v| v == handle }
        end
      end

      # Bind a buffer to a target (glBindBuffer).
      #
      # @param target [Integer] Buffer target.
      # @param buffer [Integer] GL buffer handle. 0 = unbind.
      def bind_buffer(target, buffer)
        if buffer == 0
          @target_buffers.delete(target)
          return
        end
        raise ArgumentError, "Invalid buffer handle #{buffer}" unless @buffers.key?(buffer)
        @target_buffers[target] = buffer
      end

      # Allocate and optionally fill a buffer (glBufferData).
      #
      # @param target [Integer] Buffer target.
      # @param size [Integer] Buffer size in bytes.
      # @param data [String, nil] Optional initial data.
      # @param usage [Integer] Usage hint.
      # @raise [RuntimeError] If no buffer is bound to the target.
      def buffer_data(target, size, data, usage)
        raise RuntimeError, "No buffer bound to target 0x#{target.to_s(16).upcase}" unless @target_buffers.key?(target)

        handle = @target_buffers[target]

        # Free old allocation if exists
        if !@buffers[handle].nil?
          old_buf = @buffers[handle]
          @_memory_manager.free(old_buf) unless old_buf.freed
        end

        # Allocate new buffer via Layer 5
        mem_type = ComputeRuntime::MemoryType::DEVICE_LOCAL |
          ComputeRuntime::MemoryType::HOST_VISIBLE |
          ComputeRuntime::MemoryType::HOST_COHERENT
        buf_usage = ComputeRuntime::BufferUsage::STORAGE |
          ComputeRuntime::BufferUsage::TRANSFER_SRC |
          ComputeRuntime::BufferUsage::TRANSFER_DST
        buf = @_memory_manager.allocate(size, mem_type, usage: buf_usage)
        @buffers[handle] = buf

        # Upload initial data if provided
        if data
          mapped = @_memory_manager.map(buf)
          mapped.write(0, data.byteslice(0, size))
          @_memory_manager.unmap(buf)
        end
      end

      # Update a portion of a buffer (glBufferSubData).
      #
      # @param target [Integer] Buffer target.
      # @param offset [Integer] Byte offset into the buffer.
      # @param data [String] Data to write.
      # @raise [RuntimeError] If no buffer is bound to the target.
      def buffer_sub_data(target, offset, data)
        raise RuntimeError, "No buffer bound to target 0x#{target.to_s(16).upcase}" unless @target_buffers.key?(target)
        handle = @target_buffers[target]
        buf = @buffers[handle]
        raise RuntimeError, "Buffer #{handle} has no data store" if buf.nil?

        mapped = @_memory_manager.map(buf)
        mapped.write(offset, data)
        @_memory_manager.unmap(buf)
      end

      # Bind a buffer to an indexed binding point (glBindBufferBase).
      #
      # @param target [Integer] Buffer target.
      # @param index [Integer] Binding point index.
      # @param buffer [Integer] GL buffer handle.
      def bind_buffer_base(target, index, buffer)
        raise ArgumentError, "Invalid buffer handle #{buffer}" unless @buffers.key?(buffer)
        @bound_buffers[[target, index]] = buffer
      end

      # Map a buffer region for CPU access (glMapBufferRange).
      #
      # @param target [Integer] Buffer target.
      # @param offset [Integer] Byte offset.
      # @param length [Integer] Bytes to map.
      # @param access [Integer] Access bits.
      # @return [String] A binary string of the buffer contents.
      # @raise [RuntimeError] If no buffer is bound to the target.
      def map_buffer_range(target, offset, length, access)
        raise RuntimeError, "No buffer bound to target 0x#{target.to_s(16).upcase}" unless @target_buffers.key?(target)
        handle = @target_buffers[target]
        buf = @buffers[handle]
        raise RuntimeError, "Buffer #{handle} has no data store" if buf.nil?

        @_memory_manager.invalidate(buf)
        data = @_memory_manager._get_buffer_data(buf.buffer_id)
        data.byteslice(offset, length).dup
      end

      # Unmap a buffer (glUnmapBuffer).
      #
      # @param target [Integer] Buffer target.
      # @return [true]
      def unmap_buffer(target)
        true
      end

      # ===============================================================
      # Compute dispatch
      # ===============================================================

      # Dispatch compute work groups (glDispatchCompute).
      #
      # @param num_groups_x [Integer] Workgroups in X.
      # @param num_groups_y [Integer] Workgroups in Y.
      # @param num_groups_z [Integer] Workgroups in Z.
      # @raise [RuntimeError] If no program is active.
      def dispatch_compute(num_groups_x, num_groups_y = 1, num_groups_z = 1)
        raise RuntimeError, "No program is currently active (call use_program first)" if @current_program.nil?

        prog = @programs[@current_program]
        device = @_logical_device

        # Get shader code from the program
        shader_code = nil
        unless prog["shaders"].empty?
          shader_handle = prog["shaders"][0]
          shader_code = @shaders[shader_handle]["code"] if @shaders.key?(shader_handle)
        end

        # Find all SSBO bindings
        ssbo_bindings = {}
        @bound_buffers.each do |(target, index), handle|
          if target == GL_SHADER_STORAGE_BUFFER && @buffers.key?(handle)
            buf = @buffers[handle]
            ssbo_bindings[index] = buf unless buf.nil?
          end
        end

        # Create shader module
        shader = device.create_shader_module(code: shader_code)

        # Create descriptor set with SSBO bindings
        bindings = ssbo_bindings.keys.sort.map do |i|
          ComputeRuntime::DescriptorBinding.new(binding: i, type: "storage")
        end
        ds_layout = device.create_descriptor_set_layout(bindings)
        pl_layout = device.create_pipeline_layout([ds_layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        ds = device.create_descriptor_set(ds_layout)
        ssbo_bindings.keys.sort.each do |i|
          ds.write(i, ssbo_bindings[i])
        end

        _create_and_submit_cb do |cb|
          cb.cmd_bind_pipeline(pipeline)
          cb.cmd_bind_descriptor_set(ds)
          cb.cmd_dispatch(num_groups_x, num_groups_y, num_groups_z)
        end
      end

      # ===============================================================
      # Synchronization
      # ===============================================================

      # Insert a memory barrier (glMemoryBarrier).
      #
      # @param barriers [Integer] Barrier bits.
      def memory_barrier(barriers)
        # No-op in synchronous execution
      end

      # Create a sync object (glFenceSync).
      #
      # @return [Integer] A GL handle for the sync object.
      def fence_sync
        handle = _gen_id
        fence = @_logical_device.create_fence(signaled: true)
        @syncs[handle] = fence
        handle
      end

      # Wait for a sync object (glClientWaitSync).
      #
      # @param sync [Integer] GL sync handle.
      # @param flags [Integer] Wait flags.
      # @param timeout [Integer] Timeout in nanoseconds.
      # @return [Integer] GL_ALREADY_SIGNALED, GL_CONDITION_SATISFIED,
      #     GL_TIMEOUT_EXPIRED, or GL_WAIT_FAILED.
      def client_wait_sync(sync, flags, timeout)
        return GL_WAIT_FAILED unless @syncs.key?(sync)

        fence = @syncs[sync]
        return GL_ALREADY_SIGNALED if fence.signaled

        result = fence.wait(timeout_cycles: timeout)
        result ? GL_CONDITION_SATISFIED : GL_TIMEOUT_EXPIRED
      end

      # Delete a sync object (glDeleteSync).
      #
      # @param sync [Integer] GL sync handle.
      def delete_sync(sync)
        @syncs.delete(sync)
      end

      # Block until all GL commands complete (glFinish).
      def finish
        @_logical_device.wait_idle
      end

      # ===============================================================
      # Uniforms (push constants)
      # ===============================================================

      # Get the location of a uniform variable (glGetUniformLocation).
      #
      # @param program [Integer] GL program handle.
      # @param name [String] Uniform variable name.
      # @return [Integer] Location index.
      def get_uniform_location(program, name)
        raise ArgumentError, "Invalid program handle #{program}" unless @programs.key?(program)
        name.hash.abs & 0x7FFFFFFF
      end

      # Set a float uniform (glUniform1f).
      #
      # @param location [Integer] Uniform location.
      # @param value [Float] Float value to set.
      def uniform_1f(location, value)
        @uniforms[[@current_program, location.to_s]] = value if @current_program
      end

      # Set an integer uniform (glUniform1i).
      #
      # @param location [Integer] Uniform location.
      # @param value [Integer] Integer value to set.
      def uniform_1i(location, value)
        @uniforms[[@current_program, location.to_s]] = value if @current_program
      end
    end
  end
end
