# frozen_string_literal: true

# ==========================================================================
# WasmExecutionEngine --- The Core WASM Interpreter
# ==========================================================================
#
# The WasmExecutionEngine takes a validated WASM module's runtime state
# (memory, tables, globals, functions) and executes function calls using
# the GenericVM infrastructure.
#
# The flow for calling a function:
#
#   WasmExecutionEngine#call_function(func_index, args)
#     1. Look up the function body
#     2. Decode the bytecodes into Instruction[]
#     3. Build the control flow map
#     4. Initialize locals (args + zero-initialized declared locals)
#     5. Create the WasmExecutionContext (a Hash)
#     6. Run GenericVM#execute_with_context(code, context)
#     7. Collect return values from the typed stack
# ==========================================================================

module CodingAdventures
  module WasmExecution
    MAX_CALL_DEPTH = 1024

    class WasmExecutionEngine
      # @param config [Hash] with keys:
      #   :memory, :tables, :globals, :global_types,
      #   :func_types, :func_bodies, :host_functions
      def initialize(config)
        @memory = config[:memory]
        @tables = config[:tables]
        @globals = config[:globals]
        @global_types = config[:global_types]
        @func_types = config[:func_types]
        @func_bodies = config[:func_bodies]
        @host_functions = config[:host_functions]
        @decoded_cache = {}

        # Create and configure the GenericVM.
        @vm = CodingAdventures::VirtualMachine::GenericVM.new
        @vm.set_max_recursion_depth(MAX_CALL_DEPTH)

        # Register all WASM instruction handlers.
        Instructions::Dispatch.register_all(@vm)
        Instructions::Control.register(@vm)
      end

      # Call a WASM function by index.
      #
      # @param func_index [Integer] the function index
      # @param args [Array<WasmValue>] the function arguments
      # @return [Array<WasmValue>] the function's return values
      def call_function(func_index, args)
        func_type = @func_types[func_index]
        raise TrapError, "undefined function index #{func_index}" unless func_type

        if args.length != func_type.params.length
          raise TrapError,
                "function #{func_index} expects #{func_type.params.length} arguments, got #{args.length}"
        end

        # Check if this is a host function.
        host_func = @host_functions[func_index]
        return host_func.call(args) if host_func

        # Module-defined function.
        body = @func_bodies[func_index]
        raise TrapError, "no body for function #{func_index}" unless body

        # Decode the function body (cached).
        decoded = @decoded_cache[func_index] ||= Decoder.decode_function_body(body)

        # Build the control flow map.
        control_flow_map = Decoder.build_control_flow_map(decoded)

        # Convert to GenericVM instruction format.
        vm_instructions = Decoder.to_vm_instructions(decoded)

        # Initialize locals: arguments + zero-initialized declared locals.
        typed_locals = args + body.locals.map { |t| WasmExecution.default_value(t) }

        # Build the execution context.
        ctx = {
          memory: @memory,
          tables: @tables,
          globals: @globals,
          global_types: @global_types,
          func_types: @func_types,
          func_bodies: @func_bodies,
          host_functions: @host_functions,
          typed_locals: typed_locals,
          label_stack: [],
          control_flow_map: control_flow_map,
          saved_frames: [],
          returned: false,
          return_values: [],
          current_instructions: vm_instructions
        }

        # Build the CodeObject.
        code = CodingAdventures::VirtualMachine::CodeObject.new(
          instructions: vm_instructions,
          constants: [],
          names: []
        )

        # Reset the VM and execute.
        @vm.reset
        current_code = code
        loop do
          @vm.execute_with_context(current_code, ctx)

          pending_code = ctx.delete(:pending_code)
          unless pending_code.nil?
            current_code = pending_code
            @vm.halted = false
            next
          end

          if ctx[:returned] && !ctx[:saved_frames].empty?
            current_code = resume_saved_frame(ctx)
            @vm.halted = false
            ctx[:returned] = false
            next
          end

          break
        end

        # Collect return values from the typed stack.
        result_count = func_type.results.length
        results = []
        result_count.times do
          break if @vm.typed_stack.empty?
          results.unshift(@vm.pop_typed)
        end

        results
      end

      private

      def resume_saved_frame(ctx)
        frame = ctx[:saved_frames].pop
        raise TrapError, "callee returned fewer values than expected" if @vm.typed_stack.length < frame[:return_arity]

        results = []
        frame[:return_arity].times { results.unshift(@vm.pop_typed) }

        @vm.pop_typed while @vm.typed_stack.length > frame[:stack_height]
        results.each { |result| @vm.push_typed(result) }

        ctx[:typed_locals] = frame[:locals]
        ctx[:label_stack] = frame[:label_stack]
        ctx[:control_flow_map] = frame[:control_flow_map]
        @vm.jump_to(frame[:return_pc])
        frame[:code]
      end
    end
  end
end
