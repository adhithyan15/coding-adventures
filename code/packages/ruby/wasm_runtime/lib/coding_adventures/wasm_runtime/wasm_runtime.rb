# frozen_string_literal: true

# ==========================================================================
# WasmRuntime --- The Complete WebAssembly Runtime
# ==========================================================================
#
# Composes the parser, validator, and execution engine into a single
# user-facing API:
#
#   .wasm bytes -> Parse -> Validate -> Instantiate -> Execute
#
# Usage:
#   runtime = WasmRuntime.new
#   result = runtime.load_and_run(square_wasm, "square", [5])
#   # result = [25]
# ==========================================================================

module CodingAdventures
  module WasmRuntime
    class Runtime
      def initialize(host = nil)
        @parser = CodingAdventures::WasmModuleParser::Parser.new
        @host = host
      end

      # Parse a .wasm binary into a WasmModule.
      def load(wasm_bytes)
        bytes = wasm_bytes.is_a?(String) ? wasm_bytes : wasm_bytes.pack("C*")
        @parser.parse(bytes)
      end

      # Validate a parsed module.
      def validate(wasm_module)
        WasmValidator.validate(wasm_module)
      end

      # Create a live instance from a parsed module.
      def instantiate(wasm_module)
        func_types = []
        func_bodies = []
        host_functions = []
        global_types = []
        globals = []
        memory = nil
        tables = []

        # Resolve imports.
        wasm_module.imports.each do |imp|
          case imp.kind
          when WasmTypes::EXTERNAL_KIND[:function]
            type_idx = imp.type_info
            func_types << wasm_module.types[type_idx]
            func_bodies << nil
            host_func = @host&.resolve_function(imp.module_name, imp.name)
            host_functions << host_func
          when WasmTypes::EXTERNAL_KIND[:memory]
            imported_mem = @host&.resolve_memory(imp.module_name, imp.name)
            memory = imported_mem if imported_mem
          when WasmTypes::EXTERNAL_KIND[:table]
            imported_table = @host&.resolve_table(imp.module_name, imp.name)
            tables << imported_table if imported_table
          when WasmTypes::EXTERNAL_KIND[:global]
            imported_global = @host&.resolve_global(imp.module_name, imp.name)
            if imported_global
              global_types << imported_global[:type]
              globals << imported_global[:value]
            end
          end
        end

        # Add module-defined functions.
        wasm_module.functions.each_with_index do |type_idx, i|
          func_types << wasm_module.types[type_idx]
          func_bodies << (wasm_module.code[i] || nil)
          host_functions << nil
        end

        # Allocate memory.
        if !memory && !wasm_module.memories.empty?
          mem_type = wasm_module.memories[0]
          memory = WasmExecution::LinearMemory.new(
            mem_type.limits.min,
            mem_type.limits.max
          )
        end

        # Allocate tables.
        wasm_module.tables.each do |table_type|
          tables << WasmExecution::Table.new(
            table_type.limits.min,
            table_type.limits.max
          )
        end

        # Initialize globals.
        wasm_module.globals.each do |global|
          global_types << global.global_type
          value = WasmExecution::ConstExpr.evaluate(global.init_expr, globals)
          globals << value
        end

        # Apply data segments.
        if memory
          wasm_module.data.each do |seg|
            offset = WasmExecution::ConstExpr.evaluate(seg.offset_expr, globals)
            memory.write_bytes(offset.value, seg.data)
          end
        end

        # Apply element segments.
        wasm_module.elements.each do |elem|
          table = tables[elem.table_index]
          next unless table
          offset = WasmExecution::ConstExpr.evaluate(elem.offset_expr, globals)
          elem.function_indices.each_with_index do |func_idx, j|
            table.set(offset.value + j, func_idx)
          end
        end

        # Build the export map.
        exports = {}
        wasm_module.exports.each do |exp|
          exports[exp.name] = {kind: exp.kind, index: exp.index}
        end

        # Set memory on WASI stub if applicable.
        @host.set_memory(memory) if @host.respond_to?(:set_memory) && memory

        instance = WasmInstance.new(
          wasm_module: wasm_module,
          memory: memory,
          tables: tables,
          globals: globals,
          global_types: global_types,
          func_types: func_types,
          func_bodies: func_bodies,
          host_functions: host_functions,
          exports: exports,
          host: @host
        )

        # Call start function if present.
        if wasm_module.start
          engine = WasmExecution::WasmExecutionEngine.new(
            memory: instance.memory,
            tables: instance.tables,
            globals: instance.globals,
            global_types: instance.global_types,
            func_types: instance.func_types,
            func_bodies: instance.func_bodies,
            host_functions: instance.host_functions
          )
          engine.call_function(wasm_module.start, [])
        end

        instance
      end

      # Call an exported function by name.
      #
      # @param instance [WasmInstance]
      # @param name [String] the export name
      # @param args [Array<Integer>] arguments as plain numbers
      # @return [Array<Integer>] return values as plain numbers
      def call(instance, name, args = [])
        exp = instance.exports[name]
        raise WasmExecution::TrapError, "export \"#{name}\" not found" unless exp
        unless exp[:kind] == WasmTypes::EXTERNAL_KIND[:function]
          raise WasmExecution::TrapError, "export \"#{name}\" is not a function"
        end

        func_type = instance.func_types[exp[:index]]
        raise WasmExecution::TrapError, "function type not found for export \"#{name}\"" unless func_type

        # Convert plain numbers to WasmValues.
        wasm_args = args.each_with_index.map do |arg, i|
          param_type = func_type.params[i]
          case param_type
          when WasmTypes::VALUE_TYPE[:i32] then WasmExecution.i32(arg)
          when WasmTypes::VALUE_TYPE[:i64] then WasmExecution.i64(arg)
          when WasmTypes::VALUE_TYPE[:f32] then WasmExecution.f32(arg)
          when WasmTypes::VALUE_TYPE[:f64] then WasmExecution.f64(arg)
          else WasmExecution.i32(arg)
          end
        end

        engine = WasmExecution::WasmExecutionEngine.new(
          memory: instance.memory,
          tables: instance.tables,
          globals: instance.globals,
          global_types: instance.global_types,
          func_types: instance.func_types,
          func_bodies: instance.func_bodies,
          host_functions: instance.host_functions
        )

        results = engine.call_function(exp[:index], wasm_args)

        # Convert back to plain numbers.
        results.map { |r| r.value.is_a?(Float) ? r.value : r.value.to_i }
      end

      # Parse, validate, instantiate, and call in one step.
      def load_and_run(wasm_bytes, entry = "_start", args = [])
        wasm_module = load(wasm_bytes)
        validate(wasm_module)
        instance = instantiate(wasm_module)
        call(instance, entry, args)
      end
    end
  end
end
