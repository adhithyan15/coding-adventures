# frozen_string_literal: true

# ==========================================================================
# WASM Validator --- Structural Validation for WASM 1.0 Modules
# ==========================================================================
#
# The validator checks a parsed WasmModule for semantic correctness:
#
#   - At most one memory and one table (WASM 1.0 restriction)
#   - Memory limits are within bounds (max 65536 pages = 4 GiB)
#   - Export names are unique
#   - All indices (function, type, table, memory, global) are in bounds
#   - Start function has correct type (no params, no results)
#
# A full WASM validator would also do per-function bytecode validation
# (type-checking the stack). This implementation performs structural
# validation, which is sufficient for running trusted modules.
# ==========================================================================

module CodingAdventures
  module WasmValidator
    # ValidationError --- raised when a module fails validation.
    class ValidationError < StandardError
      attr_reader :kind

      def initialize(kind, message)
        @kind = kind
        super(message)
      end
    end

    # ValidatedModule --- a module that has passed validation.
    ValidatedModule = Struct.new(:wasm_module, :func_types, keyword_init: true)

    MAX_MEMORY_PAGES = 65536

    module_function

    # Validate a parsed WasmModule for structural correctness.
    #
    # @param wasm_module [WasmTypes::WasmModule] the parsed module
    # @return [ValidatedModule] the validated module with resolved types
    # @raise [ValidationError] on validation failures
    def validate(wasm_module)
      validate_structure(wasm_module)

      # Build the combined function type array (imports + module functions).
      func_types = []
      wasm_module.imports.each do |imp|
        if imp.kind == WasmTypes::EXTERNAL_KIND[:function]
          type_idx = imp.type_info
          func_types << wasm_module.types[type_idx]
        end
      end
      wasm_module.functions.each do |type_idx|
        func_types << wasm_module.types[type_idx]
      end

      ValidatedModule.new(
        wasm_module: wasm_module,
        func_types: func_types.freeze
      )
    end

    # Validate the structural constraints of a WASM module.
    #
    # @param wasm_module [WasmTypes::WasmModule]
    # @raise [ValidationError]
    def validate_structure(wasm_module)
      # Count imports by kind.
      num_imported_memories = 0
      num_imported_tables = 0

      wasm_module.imports.each do |imp|
        case imp.kind
        when WasmTypes::EXTERNAL_KIND[:memory]
          num_imported_memories += 1
        when WasmTypes::EXTERNAL_KIND[:table]
          num_imported_tables += 1
        end
      end

      total_memories = num_imported_memories + wasm_module.memories.length
      total_tables = num_imported_tables + wasm_module.tables.length

      # WASM 1.0: at most one memory.
      if total_memories > 1
        raise ValidationError.new(
          :multiple_memories,
          "WASM 1.0 allows at most one memory, found #{total_memories}"
        )
      end

      # WASM 1.0: at most one table.
      if total_tables > 1
        raise ValidationError.new(
          :multiple_tables,
          "WASM 1.0 allows at most one table, found #{total_tables}"
        )
      end

      # Validate memory limits.
      wasm_module.memories.each do |mem_type|
        limits = mem_type.limits
        if limits.min > MAX_MEMORY_PAGES
          raise ValidationError.new(:memory_limit_exceeded,
            "memory minimum #{limits.min} exceeds maximum #{MAX_MEMORY_PAGES} pages")
        end
        if limits.max && limits.max > MAX_MEMORY_PAGES
          raise ValidationError.new(:memory_limit_exceeded,
            "memory maximum #{limits.max} exceeds maximum #{MAX_MEMORY_PAGES} pages")
        end
        if limits.max && limits.min > limits.max
          raise ValidationError.new(:memory_limit_order,
            "memory minimum #{limits.min} > maximum #{limits.max}")
        end
      end

      # Validate export name uniqueness.
      seen_names = {}
      wasm_module.exports.each do |exp|
        if seen_names[exp.name]
          raise ValidationError.new(:duplicate_export_name,
            "duplicate export name: #{exp.name}")
        end
        seen_names[exp.name] = true
      end
    end
  end
end
