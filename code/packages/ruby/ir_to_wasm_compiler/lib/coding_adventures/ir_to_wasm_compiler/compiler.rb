# frozen_string_literal: true

module CodingAdventures
  module IrToWasmCompiler
    IR = CodingAdventures::CompilerIr
    WT = CodingAdventures::WasmTypes
    LEB128 = CodingAdventures::WasmLeb128

    LOOP_START_RE = /^loop_\d+_start$/
    IF_ELSE_RE = /^if_\d+_else$/
    FUNCTION_COMMENT_RE = /^function:\s*([A-Za-z_][A-Za-z0-9_]*)\((.*)\)$/

    SYSCALL_WRITE = 1
    SYSCALL_READ = 2
    SYSCALL_EXIT = 10
    SYSCALL_ARG0 = 4

    WASI_MODULE = "wasi_snapshot_preview1"
    WASI_IOVEC_OFFSET = 0
    WASI_COUNT_OFFSET = 8
    WASI_BYTE_OFFSET = 12
    WASI_SCRATCH_SIZE = 16

    REG_SCRATCH = 1
    REG_VAR_BASE = 2

    MEMORY_OPS = [
      IR::IrOp::LOAD_ADDR,
      IR::IrOp::LOAD_BYTE,
      IR::IrOp::STORE_BYTE,
      IR::IrOp::LOAD_WORD,
      IR::IrOp::STORE_WORD
    ].freeze

    FunctionSignature = Struct.new(:label, :param_count, :export_name, keyword_init: true)
    FunctionIR = Struct.new(:label, :instructions, :signature, :max_reg, keyword_init: true)
    WasiImport = Struct.new(:syscall_number, :name, :func_type, keyword_init: true) do
      def type_key
        "wasi::#{name}"
      end
    end
    WasiContext = Struct.new(:function_indices, :scratch_base, keyword_init: true)

    class WasmLoweringError < StandardError; end

    OPCODE = begin
      names = %w[
        nop block loop if else end br br_if return call
        local.get local.set i32.load i32.load8_u i32.store i32.store8
        i32.const i32.eqz i32.eq i32.ne i32.lt_s i32.gt_s i32.add i32.sub i32.and
      ]
      names.each_with_object({}) do |name, table|
        entry = CodingAdventures::WasmOpcodes::OPCODES_BY_NAME[name]
        raise WasmLoweringError, "missing wasm opcode #{name}" unless entry

        table[name] = entry.opcode
      end.freeze
    end

    class Compiler
      def compile(program, function_signatures = nil)
        signatures = IrToWasmCompiler.infer_function_signatures_from_comments(program)
        Array(function_signatures).each do |signature|
          signatures[signature.label] = signature
        end

        functions = split_functions(program, signatures)
        imports = collect_wasi_imports(program)
        type_indices, types = build_type_table(functions, imports)
        data_offsets = layout_data(program.data)

        scratch_base = nil
        if needs_wasi_scratch?(program)
          scratch_base = IrToWasmCompiler.align_up(program.data.sum(&:size), 4)
        end

        wasm_module = WT::WasmModule.new
        wasm_module.types.concat(types)
        imports.each do |import_value|
          wasm_module.imports << WT::Import.new(
            WASI_MODULE,
            import_value.name,
            WT::EXTERNAL_KIND[:function],
            type_indices[import_value.type_key]
          )
        end

        function_index_base = imports.length
        function_indices = {}
        functions.each_with_index do |function, index|
          function_indices[function.label] = function_index_base + index
          wasm_module.functions << type_indices[function.label]
        end

        total_bytes = program.data.sum(&:size)
        total_bytes = [total_bytes, scratch_base + WASI_SCRATCH_SIZE].max unless scratch_base.nil?

        if needs_memory?(program) || !scratch_base.nil?
          page_count = total_bytes.zero? ? 1 : [1, (total_bytes.to_f / 65_536).ceil].max
          wasm_module.memories << WT::MemoryType.new(WT::Limits.new(page_count, nil))
          wasm_module.exports << WT::Export.new("memory", WT::EXTERNAL_KIND[:memory], 0)
          program.data.each do |decl|
            payload = ([decl.init & 0xFF] * decl.size).pack("C*").b
            wasm_module.data << WT::DataSegment.new(0, IrToWasmCompiler.const_expr(data_offsets[decl.label]), payload)
          end
        end

        wasi_context = WasiContext.new(
          function_indices: imports.each_with_index.each_with_object({}) do |(import_value, index), table|
            table[import_value.syscall_number] = index
          end,
          scratch_base: scratch_base
        )

        functions.each do |function|
          wasm_module.code << FunctionLowerer.new(
            function: function,
            signatures: signatures,
            function_indices: function_indices,
            data_offsets: data_offsets,
            wasi_context: wasi_context
          ).lower
          next if function.signature.export_name.nil?

          wasm_module.exports << WT::Export.new(
            function.signature.export_name,
            WT::EXTERNAL_KIND[:function],
            function_indices[function.label]
          )
        end

        wasm_module
      end

      private

      def build_type_table(functions, imports)
        type_indices = {}
        function_types = []
        function_to_type_index = {}

        imports.each do |import_value|
          unless type_indices.key?(import_value.func_type)
            type_indices[import_value.func_type] = function_types.length
            function_types << import_value.func_type
          end
          function_to_type_index[import_value.type_key] = type_indices[import_value.func_type]
        end

        functions.each do |function|
          func_type = WT::FuncType.new(
            [WT::VALUE_TYPE[:i32]] * function.signature.param_count,
            [WT::VALUE_TYPE[:i32]]
          )
          unless type_indices.key?(func_type)
            type_indices[func_type] = function_types.length
            function_types << func_type
          end
          function_to_type_index[function.label] = type_indices[func_type]
        end

        [function_to_type_index, function_types]
      end

      def layout_data(decls)
        cursor = 0
        decls.each_with_object({}) do |decl, offsets|
          offsets[decl.label] = cursor
          cursor += decl.size
        end
      end

      def needs_memory?(program)
        return true unless program.data.empty?

        program.instructions.any? { |instruction| MEMORY_OPS.include?(instruction.opcode) }
      end

      def needs_wasi_scratch?(program)
        program.instructions.any? do |instruction|
          next false unless instruction.opcode == IR::IrOp::SYSCALL && !instruction.operands.empty?

          syscall = IrToWasmCompiler.expect_immediate(instruction.operands[0], "SYSCALL number").value
          [SYSCALL_WRITE, SYSCALL_READ].include?(syscall)
        end
      end

      def collect_wasi_imports(program)
        required_syscalls = program.instructions.each_with_object([]) do |instruction, syscalls|
          next unless instruction.opcode == IR::IrOp::SYSCALL && !instruction.operands.empty?

          syscalls << IrToWasmCompiler.expect_immediate(instruction.operands[0], "SYSCALL number").value
        end.uniq

        ordered_imports = [
          WasiImport.new(
            syscall_number: SYSCALL_WRITE,
            name: "fd_write",
            func_type: WT::FuncType.new([WT::VALUE_TYPE[:i32]] * 4, [WT::VALUE_TYPE[:i32]])
          ),
          WasiImport.new(
            syscall_number: SYSCALL_READ,
            name: "fd_read",
            func_type: WT::FuncType.new([WT::VALUE_TYPE[:i32]] * 4, [WT::VALUE_TYPE[:i32]])
          ),
          WasiImport.new(
            syscall_number: SYSCALL_EXIT,
            name: "proc_exit",
            func_type: WT::FuncType.new([WT::VALUE_TYPE[:i32]], [])
          )
        ]

        supported = ordered_imports.map(&:syscall_number)
        unsupported = required_syscalls - supported
        unless unsupported.empty?
          raise WasmLoweringError, "unsupported SYSCALL number(s): #{unsupported.sort.join(', ')}"
        end

        ordered_imports.select { |import_value| required_syscalls.include?(import_value.syscall_number) }
      end

      def split_functions(program, signatures)
        functions = []
        start_index = nil
        start_label = nil

        program.instructions.each_with_index do |instruction, index|
          label_name = IrToWasmCompiler.function_label_name(instruction)
          next if label_name.nil?

          unless start_label.nil? || start_index.nil?
            functions << IrToWasmCompiler.make_function_ir(
              label: start_label,
              instructions: program.instructions[start_index...index],
              signatures: signatures
            )
          end

          start_label = label_name
          start_index = index
        end

        unless start_label.nil? || start_index.nil?
          functions << IrToWasmCompiler.make_function_ir(
            label: start_label,
            instructions: program.instructions[start_index..],
            signatures: signatures
          )
        end

        functions
      end
    end

    class FunctionLowerer
      def initialize(function:, signatures:, function_indices:, data_offsets:, wasi_context:)
        @function = function
        @signatures = signatures
        @function_indices = function_indices
        @data_offsets = data_offsets
        @wasi_context = wasi_context
        @param_count = function.signature.param_count
        @bytes = +"".b
        @instructions = function.instructions
        @label_to_index = {}
        @instructions.each_with_index do |instruction, index|
          label = IrToWasmCompiler.label_name(instruction)
          @label_to_index[label] = index unless label.nil?
        end
      end

      def lower
        copy_params_into_ir_registers
        emit_region(1, @instructions.length)
        emit_opcode("end")

        WT::FunctionBody.new(
          [WT::VALUE_TYPE[:i32]] * (@function.max_reg + 1),
          @bytes
        )
      end

      private

      def copy_params_into_ir_registers
        @param_count.times do |param_index|
          emit_opcode("local.get")
          emit_u32(param_index)
          emit_opcode("local.set")
          emit_u32(local_index(REG_VAR_BASE + param_index))
        end
      end

      def emit_region(start_index, end_index)
        index = start_index
        while index < end_index
          instruction = @instructions[index]

          if instruction.opcode == IR::IrOp::COMMENT
            index += 1
            next
          end

          label_name = IrToWasmCompiler.label_name(instruction)
          if label_name&.match?(LOOP_START_RE)
            index = emit_loop(index)
            next
          end

          if [IR::IrOp::BRANCH_Z, IR::IrOp::BRANCH_NZ].include?(instruction.opcode) &&
              instruction.operands.length == 2 &&
              instruction.operands[1].is_a?(IR::IrLabel) &&
              instruction.operands[1].name.match?(IF_ELSE_RE)
            index = emit_if(index)
            next
          end

          if instruction.opcode == IR::IrOp::LABEL
            index += 1
            next
          end

          if [IR::IrOp::JUMP, IR::IrOp::BRANCH_Z, IR::IrOp::BRANCH_NZ].include?(instruction.opcode)
            raise WasmLoweringError, "unexpected unstructured control flow in #{@function.label}"
          end

          emit_simple(instruction)
          index += 1
        end
      end

      def emit_if(branch_index)
        branch = @instructions[branch_index]
        cond_reg = IrToWasmCompiler.expect_register(branch.operands[0], "if condition")
        else_label = IrToWasmCompiler.expect_label(branch.operands[1], "if else label").name
        end_label = "#{else_label.delete_suffix("_else")}_end"

        else_index = require_label_index(else_label)
        end_index = require_label_index(end_label)
        jump_index = find_last_jump_to_label(branch_index + 1, else_index, end_label)

        emit_local_get(cond_reg.index)
        emit_opcode("i32.eqz") if branch.opcode == IR::IrOp::BRANCH_NZ
        emit_opcode("if")
        @bytes << [WT::BLOCK_TYPE_EMPTY].pack("C")

        emit_region(branch_index + 1, jump_index)

        if else_index + 1 < end_index
          emit_opcode("else")
          emit_region(else_index + 1, end_index)
        end

        emit_opcode("end")
        end_index + 1
      end

      def emit_loop(label_index)
        start_label = IrToWasmCompiler.label_name(@instructions[label_index])
        raise WasmLoweringError, "loop lowering expected a start label" if start_label.nil?

        end_label = "#{start_label.delete_suffix("_start")}_end"
        end_index = require_label_index(end_label)
        branch_index = find_first_branch_to_label(label_index + 1, end_index, end_label)
        backedge_index = find_last_jump_to_label(branch_index + 1, end_index, start_label)
        branch = @instructions[branch_index]
        cond_reg = IrToWasmCompiler.expect_register(branch.operands[0], "loop condition")

        emit_opcode("block")
        @bytes << [WT::BLOCK_TYPE_EMPTY].pack("C")
        emit_opcode("loop")
        @bytes << [WT::BLOCK_TYPE_EMPTY].pack("C")

        emit_region(label_index + 1, branch_index)

        emit_local_get(cond_reg.index)
        emit_opcode("i32.eqz") if branch.opcode == IR::IrOp::BRANCH_Z
        emit_opcode("br_if")
        emit_u32(1)

        emit_region(branch_index + 1, backedge_index)
        emit_opcode("br")
        emit_u32(0)

        emit_opcode("end")
        emit_opcode("end")
        end_index + 1
      end

      def emit_simple(instruction)
        case instruction.opcode
        when IR::IrOp::LOAD_IMM
          dst = IrToWasmCompiler.expect_register(instruction.operands[0], "LOAD_IMM dst")
          imm = IrToWasmCompiler.expect_immediate(instruction.operands[1], "LOAD_IMM imm")
          emit_i32_const(imm.value)
          emit_local_set(dst.index)
        when IR::IrOp::LOAD_ADDR
          dst = IrToWasmCompiler.expect_register(instruction.operands[0], "LOAD_ADDR dst")
          label = IrToWasmCompiler.expect_label(instruction.operands[1], "LOAD_ADDR label")
          raise WasmLoweringError, "unknown data label: #{label.name}" unless @data_offsets.key?(label.name)

          emit_i32_const(@data_offsets[label.name])
          emit_local_set(dst.index)
        when IR::IrOp::LOAD_BYTE
          dst = IrToWasmCompiler.expect_register(instruction.operands[0], "LOAD_BYTE dst")
          base = IrToWasmCompiler.expect_register(instruction.operands[1], "LOAD_BYTE base")
          offset = IrToWasmCompiler.expect_register(instruction.operands[2], "LOAD_BYTE offset")
          emit_address(base.index, offset.index)
          emit_opcode("i32.load8_u")
          emit_memarg(0, 0)
          emit_local_set(dst.index)
        when IR::IrOp::STORE_BYTE
          src = IrToWasmCompiler.expect_register(instruction.operands[0], "STORE_BYTE src")
          base = IrToWasmCompiler.expect_register(instruction.operands[1], "STORE_BYTE base")
          offset = IrToWasmCompiler.expect_register(instruction.operands[2], "STORE_BYTE offset")
          emit_address(base.index, offset.index)
          emit_local_get(src.index)
          emit_opcode("i32.store8")
          emit_memarg(0, 0)
        when IR::IrOp::LOAD_WORD
          dst = IrToWasmCompiler.expect_register(instruction.operands[0], "LOAD_WORD dst")
          base = IrToWasmCompiler.expect_register(instruction.operands[1], "LOAD_WORD base")
          offset = IrToWasmCompiler.expect_register(instruction.operands[2], "LOAD_WORD offset")
          emit_address(base.index, offset.index)
          emit_opcode("i32.load")
          emit_memarg(2, 0)
          emit_local_set(dst.index)
        when IR::IrOp::STORE_WORD
          src = IrToWasmCompiler.expect_register(instruction.operands[0], "STORE_WORD src")
          base = IrToWasmCompiler.expect_register(instruction.operands[1], "STORE_WORD base")
          offset = IrToWasmCompiler.expect_register(instruction.operands[2], "STORE_WORD offset")
          emit_address(base.index, offset.index)
          emit_local_get(src.index)
          emit_opcode("i32.store")
          emit_memarg(2, 0)
        when IR::IrOp::ADD
          emit_binary_numeric("i32.add", instruction)
        when IR::IrOp::ADD_IMM
          dst = IrToWasmCompiler.expect_register(instruction.operands[0], "ADD_IMM dst")
          src = IrToWasmCompiler.expect_register(instruction.operands[1], "ADD_IMM src")
          imm = IrToWasmCompiler.expect_immediate(instruction.operands[2], "ADD_IMM imm")
          emit_local_get(src.index)
          emit_i32_const(imm.value)
          emit_opcode("i32.add")
          emit_local_set(dst.index)
        when IR::IrOp::SUB
          emit_binary_numeric("i32.sub", instruction)
        when IR::IrOp::AND
          emit_binary_numeric("i32.and", instruction)
        when IR::IrOp::AND_IMM
          dst = IrToWasmCompiler.expect_register(instruction.operands[0], "AND_IMM dst")
          src = IrToWasmCompiler.expect_register(instruction.operands[1], "AND_IMM src")
          imm = IrToWasmCompiler.expect_immediate(instruction.operands[2], "AND_IMM imm")
          emit_local_get(src.index)
          emit_i32_const(imm.value)
          emit_opcode("i32.and")
          emit_local_set(dst.index)
        when IR::IrOp::CMP_EQ
          emit_binary_numeric("i32.eq", instruction)
        when IR::IrOp::CMP_NE
          emit_binary_numeric("i32.ne", instruction)
        when IR::IrOp::CMP_LT
          emit_binary_numeric("i32.lt_s", instruction)
        when IR::IrOp::CMP_GT
          emit_binary_numeric("i32.gt_s", instruction)
        when IR::IrOp::CALL
          label = IrToWasmCompiler.expect_label(instruction.operands[0], "CALL target")
          signature = @signatures[label.name]
          raise WasmLoweringError, "missing function signature for #{label.name}" if signature.nil?
          raise WasmLoweringError, "unknown function label: #{label.name}" unless @function_indices.key?(label.name)

          signature.param_count.times do |param_index|
            emit_local_get(REG_VAR_BASE + param_index)
          end
          emit_opcode("call")
          emit_u32(@function_indices[label.name])
          emit_local_set(REG_SCRATCH)
        when IR::IrOp::RET, IR::IrOp::HALT
          emit_local_get(REG_SCRATCH)
          emit_opcode("return")
        when IR::IrOp::NOP
          emit_opcode("nop")
        when IR::IrOp::SYSCALL
          emit_syscall(instruction)
        else
          raise WasmLoweringError, "unsupported opcode: #{IR::IrOp.op_name(instruction.opcode)}"
        end
      end

      def emit_syscall(instruction)
        syscall = IrToWasmCompiler.expect_immediate(instruction.operands[0], "SYSCALL number").value

        case syscall
        when SYSCALL_WRITE then emit_wasi_write
        when SYSCALL_READ then emit_wasi_read
        when SYSCALL_EXIT then emit_wasi_exit
        else
          raise WasmLoweringError, "unsupported SYSCALL number: #{syscall}"
        end
      end

      def emit_wasi_write
        scratch_base = require_wasi_scratch
        iovec_ptr = scratch_base + WASI_IOVEC_OFFSET
        nwritten_ptr = scratch_base + WASI_COUNT_OFFSET
        byte_ptr = scratch_base + WASI_BYTE_OFFSET

        emit_i32_const(byte_ptr)
        emit_local_get(SYSCALL_ARG0)
        emit_opcode("i32.store8")
        emit_memarg(0, 0)

        emit_store_const_i32(iovec_ptr, byte_ptr)
        emit_store_const_i32(iovec_ptr + 4, 1)

        emit_i32_const(1)
        emit_i32_const(iovec_ptr)
        emit_i32_const(1)
        emit_i32_const(nwritten_ptr)
        emit_wasi_call(SYSCALL_WRITE)
        emit_local_set(REG_SCRATCH)
      end

      def emit_wasi_read
        scratch_base = require_wasi_scratch
        iovec_ptr = scratch_base + WASI_IOVEC_OFFSET
        nread_ptr = scratch_base + WASI_COUNT_OFFSET
        byte_ptr = scratch_base + WASI_BYTE_OFFSET

        emit_i32_const(byte_ptr)
        emit_i32_const(0)
        emit_opcode("i32.store8")
        emit_memarg(0, 0)

        emit_store_const_i32(iovec_ptr, byte_ptr)
        emit_store_const_i32(iovec_ptr + 4, 1)

        emit_i32_const(0)
        emit_i32_const(iovec_ptr)
        emit_i32_const(1)
        emit_i32_const(nread_ptr)
        emit_wasi_call(SYSCALL_READ)
        emit_local_set(REG_SCRATCH)

        emit_i32_const(byte_ptr)
        emit_opcode("i32.load8_u")
        emit_memarg(0, 0)
        emit_local_set(SYSCALL_ARG0)
      end

      def emit_wasi_exit
        emit_local_get(SYSCALL_ARG0)
        emit_wasi_call(SYSCALL_EXIT)
        emit_i32_const(0)
        emit_opcode("return")
      end

      def emit_store_const_i32(address, value)
        emit_i32_const(address)
        emit_i32_const(value)
        emit_opcode("i32.store")
        emit_memarg(2, 0)
      end

      def emit_wasi_call(syscall_number)
        function_index = @wasi_context.function_indices[syscall_number]
        raise WasmLoweringError, "missing WASI import for SYSCALL #{syscall_number}" if function_index.nil?

        emit_opcode("call")
        emit_u32(function_index)
      end

      def require_wasi_scratch
        raise WasmLoweringError, "SYSCALL lowering requires WASM scratch memory" if @wasi_context.scratch_base.nil?

        @wasi_context.scratch_base
      end

      def emit_binary_numeric(wasm_op, instruction)
        opname = IR::IrOp.op_name(instruction.opcode)
        dst = IrToWasmCompiler.expect_register(instruction.operands[0], "#{opname} dst")
        left = IrToWasmCompiler.expect_register(instruction.operands[1], "#{opname} lhs")
        right = IrToWasmCompiler.expect_register(instruction.operands[2], "#{opname} rhs")
        emit_local_get(left.index)
        emit_local_get(right.index)
        emit_opcode(wasm_op)
        emit_local_set(dst.index)
      end

      def emit_address(base_index, offset_index)
        emit_local_get(base_index)
        emit_local_get(offset_index)
        emit_opcode("i32.add")
      end

      def emit_local_get(reg_index)
        emit_opcode("local.get")
        emit_u32(local_index(reg_index))
      end

      def emit_local_set(reg_index)
        emit_opcode("local.set")
        emit_u32(local_index(reg_index))
      end

      def emit_i32_const(value)
        emit_opcode("i32.const")
        @bytes << LEB128.encode_signed(value)
      end

      def emit_memarg(align, offset)
        emit_u32(align)
        emit_u32(offset)
      end

      def emit_opcode(name)
        @bytes << [OPCODE.fetch(name)].pack("C")
      end

      def emit_u32(value)
        @bytes << LEB128.encode_unsigned(value)
      end

      def local_index(reg_index)
        @param_count + reg_index
      end

      def require_label_index(label)
        index = @label_to_index[label]
        raise WasmLoweringError, "missing label #{label} in #{@function.label}" if index.nil?

        index
      end

      def find_first_branch_to_label(start_index, end_index, label)
        (start_index...end_index).each do |index|
          instruction = @instructions[index]
          next unless [IR::IrOp::BRANCH_Z, IR::IrOp::BRANCH_NZ].include?(instruction.opcode)

          target = IrToWasmCompiler.label_name_from_operand(instruction.operands[1])
          return index if target == label
        end
        raise WasmLoweringError, "expected branch to #{label} in #{@function.label}"
      end

      def find_last_jump_to_label(start_index, end_index, label)
        (end_index - 1).downto(start_index) do |index|
          instruction = @instructions[index]
          next unless instruction.opcode == IR::IrOp::JUMP

          target = IrToWasmCompiler.label_name_from_operand(instruction.operands[0])
          return index if target == label
        end
        raise WasmLoweringError, "expected jump to #{label} in #{@function.label}"
      end
    end

    module_function

    def compile(program, function_signatures = nil)
      Compiler.new.compile(program, function_signatures)
    end

    def new_function_signature(label, param_count, export_name = nil)
      FunctionSignature.new(label:, param_count:, export_name:)
    end

    def infer_function_signatures_from_comments(program)
      signatures = {}
      pending_comment = nil

      program.instructions.each do |instruction|
        if instruction.opcode == IR::IrOp::COMMENT
          pending_comment = label_name_from_operand(instruction.operands[0])
          next
        end

        label = function_label_name(instruction)
        if label
          if label == "_start"
            signatures[label] = FunctionSignature.new(label: label, param_count: 0, export_name: "_start")
          elsif label.start_with?("_fn_") && !pending_comment.nil?
            export_name = label.delete_prefix("_fn_")
            match = FUNCTION_COMMENT_RE.match(pending_comment)
            if match && match[1] == export_name
              params_blob = match[2].to_s.strip
              param_count = params_blob.empty? ? 0 : params_blob.split(",").count { |piece| !piece.strip.empty? }
              signatures[label] = FunctionSignature.new(label: label, param_count: param_count, export_name: export_name)
            end
          end
          pending_comment = nil
        elsif instruction.opcode != IR::IrOp::COMMENT
          pending_comment = nil
        end
      end

      signatures
    end

    def make_function_ir(label:, instructions:, signatures:)
      signature =
        if label == "_start"
          signatures.fetch(label, FunctionSignature.new(label: label, param_count: 0, export_name: "_start"))
        else
          signatures[label]
        end
      raise WasmLoweringError, "missing function signature for #{label}" if signature.nil?

      max_reg = [1, REG_VAR_BASE + [signature.param_count - 1, 0].max].max
      instructions.each do |instruction|
        max_reg = [max_reg, SYSCALL_ARG0].max if instruction.opcode == IR::IrOp::SYSCALL
        instruction.operands.each do |operand|
          next unless operand.is_a?(IR::IrRegister)

          max_reg = [max_reg, operand.index].max
        end
      end

      FunctionIR.new(
        label: label,
        instructions: instructions,
        signature: signature,
        max_reg: max_reg
      )
    end

    def const_expr(value)
      [OPCODE.fetch("i32.const")].pack("C") + LEB128.encode_signed(value) + [OPCODE.fetch("end")].pack("C")
    end

    def function_label_name(instruction)
      label = label_name(instruction)
      return label if label == "_start"
      return label if !label.nil? && label.start_with?("_fn_")

      nil
    end

    def label_name(instruction)
      return nil unless instruction.opcode == IR::IrOp::LABEL
      return nil if instruction.operands.empty?
      return nil unless instruction.operands[0].is_a?(IR::IrLabel)

      instruction.operands[0].name
    end

    def label_name_from_operand(operand)
      raise WasmLoweringError, "expected label operand, got #{operand.inspect}" unless operand.is_a?(IR::IrLabel)

      operand.name
    end

    def expect_register(operand, context)
      raise WasmLoweringError, "#{context}: expected register, got #{operand.inspect}" unless operand.is_a?(IR::IrRegister)

      operand
    end

    def expect_immediate(operand, context)
      raise WasmLoweringError, "#{context}: expected immediate, got #{operand.inspect}" unless operand.is_a?(IR::IrImmediate)

      operand
    end

    def expect_label(operand, context)
      raise WasmLoweringError, "#{context}: expected label, got #{operand.inspect}" unless operand.is_a?(IR::IrLabel)

      operand
    end

    def align_up(value, alignment)
      ((value + alignment - 1) / alignment) * alignment
    end
  end
end
