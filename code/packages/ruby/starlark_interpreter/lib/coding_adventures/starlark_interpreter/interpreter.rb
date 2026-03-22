# frozen_string_literal: true

# ==========================================================================
# Starlark Interpreter -- Full Pipeline with load() Support
# ==========================================================================
#
# The Interpreter class chains together the entire Starlark execution
# pipeline: lexing, parsing, compilation, and VM execution. It adds one
# critical feature on top of the bare VM: the `load()` statement.
#
# == The load() Problem
#
# Starlark's `load()` statement imports symbols from other files:
#
#   load("//rules.star", "http_archive")
#
# This requires the interpreter to:
#   1. Resolve the file path ("//rules.star" -> actual file contents)
#   2. Recursively compile and execute the loaded file
#   3. Extract the requested symbols from the loaded file's namespace
#   4. Cache the result so repeated loads don't re-execute
#
# The bare VM (starlark_vm) can't do this -- it has a stub LOAD_MODULE
# handler that pushes an empty hash. The Interpreter replaces that stub
# with a real implementation that uses a configurable file_resolver.
#
# == File Resolver
#
# The file_resolver is a callable (Proc/lambda) that takes a label string
# (e.g., "//rules.star") and returns the file contents as a string, or
# nil if the file doesn't exist.
#
# For testing, you can use a simple hash-based resolver:
#
#   resolver = ->(label) {
#     files = { "//math.star" => "def double(n):\n    return n * 2\n" }
#     files[label]
#   }
#
# For production use, you'd resolve against the filesystem:
#
#   resolver = ->(label) {
#     path = label.sub("//", "/workspace/")
#     File.exist?(path) ? File.read(path) : nil
#   }
#
# == Caching
#
# The interpreter caches the results of load() calls. If two files both
# load("//common.star", "helper"), the common.star file is only compiled
# and executed once. The cached variable namespace is reused.
#
# == Usage
#
#   # Simple interpretation (no load support needed):
#   result = CodingAdventures::StarlarkInterpreter.interpret("x = 1 + 2\n")
#   result.variables["x"]  # => 3
#
#   # With load support:
#   resolver = ->(label) { files[label] }
#   result = CodingAdventures::StarlarkInterpreter.interpret(source, file_resolver: resolver)
#
#   # From a file:
#   result = CodingAdventures::StarlarkInterpreter.interpret_file("path/to/file.star")
# ==========================================================================

module CodingAdventures
  module StarlarkInterpreter
    class Interpreter
      attr_reader :file_resolver, :max_recursion_depth

      # Create a new Starlark interpreter.
      #
      # @param file_resolver [Proc, nil] a callable that resolves load() labels
      #   to file contents. Receives a string label, returns contents or nil.
      # @param max_recursion_depth [Integer] maximum function call depth (default: 200)
      def initialize(file_resolver: nil, max_recursion_depth: 200)
        @file_resolver = file_resolver
        @max_recursion_depth = max_recursion_depth
        @load_cache = {}
      end

      # Interpret a Starlark source string.
      #
      # Compiles the source to bytecode, creates a VM with all handlers
      # and builtins, registers a real LOAD_MODULE handler if a file_resolver
      # is configured, and executes the bytecode.
      #
      # @param source [String] Starlark source code (should end with newline)
      # @return [StarlarkVM::StarlarkResult] execution result
      def interpret(source)
        code = StarlarkAstToBytecodeCompiler::Compiler.compile_starlark(source)
        vm = StarlarkVM.create_starlark_vm(max_recursion_depth: @max_recursion_depth)
        register_load_handler(vm)
        traces = vm.execute(code)
        StarlarkVM::StarlarkResult.new(
          variables: vm.variables.dup,
          output: vm.output.dup,
          traces: traces
        )
      end

      # Interpret a Starlark file from disk.
      #
      # Reads the file, ensures it ends with a newline, and interprets it.
      #
      # @param path [String] path to the .star file
      # @return [StarlarkVM::StarlarkResult] execution result
      def interpret_file(path)
        source = File.read(path)
        source += "\n" unless source.end_with?("\n")
        interpret(source)
      end

      private

      # Register a real LOAD_MODULE handler that supports load() statements.
      #
      # This replaces the stub handler from starlark_vm with one that:
      #   1. Looks up the module label in the cache
      #   2. If not cached, calls the file_resolver to get contents
      #   3. Recursively interprets the loaded file
      #   4. Caches the result for future loads
      #   5. Pushes the loaded module's variable namespace onto the stack
      #
      # The IMPORT_FROM handler (from starlark_vm) then extracts individual
      # symbols from the namespace hash.
      def register_load_handler(vm)
        op = StarlarkAstToBytecodeCompiler::Op
        interpreter = self

        vm.register_opcode(op::LOAD_MODULE, ->(v, instr, c) {
          # The module path is stored in the constants pool.
          idx = instr.operand
          label = c.constants[idx]

          # Check the cache first -- don't re-execute already-loaded modules.
          cache = interpreter.instance_variable_get(:@load_cache)
          unless cache.key?(label)
            # Resolve the label to file contents using the configured resolver.
            if interpreter.file_resolver.nil?
              raise "load() called but no file_resolver configured"
            end
            contents = interpreter.file_resolver.call(label)
            if contents.nil?
              raise "load(): file not found: #{label}"
            end
            contents += "\n" unless contents.end_with?("\n")

            # Recursively interpret the loaded file.
            # This creates a new VM and executes the file independently.
            result = interpreter.interpret(contents)
            cache[label] = result.variables.dup
          end

          # Push the loaded module's namespace onto the stack.
          # IMPORT_FROM will extract individual symbols from this hash.
          v.push(cache[label].dup)
          v.advance_pc
          nil
        })
      end
    end

    # ================================================================
    # Module-Level Convenience Methods
    # ================================================================
    #
    # These methods provide a simpler API when you don't need to reuse
    # an interpreter instance across multiple calls.

    # Interpret a Starlark source string.
    #
    # @param source [String] Starlark source code
    # @param file_resolver [Proc, nil] optional load() resolver
    # @param max_recursion_depth [Integer] max call depth (default: 200)
    # @return [StarlarkVM::StarlarkResult]
    #
    # @example
    #   result = CodingAdventures::StarlarkInterpreter.interpret("x = 42\n")
    #   result.variables["x"]  # => 42
    def self.interpret(source, file_resolver: nil, max_recursion_depth: 200)
      interp = Interpreter.new(
        file_resolver: file_resolver,
        max_recursion_depth: max_recursion_depth
      )
      interp.interpret(source)
    end

    # Interpret a Starlark file from disk.
    #
    # @param path [String] path to the .star file
    # @param file_resolver [Proc, nil] optional load() resolver
    # @param max_recursion_depth [Integer] max call depth (default: 200)
    # @return [StarlarkVM::StarlarkResult]
    def self.interpret_file(path, file_resolver: nil, max_recursion_depth: 200)
      interp = Interpreter.new(
        file_resolver: file_resolver,
        max_recursion_depth: max_recursion_depth
      )
      interp.interpret_file(path)
    end
  end
end
