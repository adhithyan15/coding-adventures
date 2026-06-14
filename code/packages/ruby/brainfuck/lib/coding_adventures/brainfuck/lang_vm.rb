# frozen_string_literal: true

module CodingAdventures
  module Brainfuck
    IR = CodingAdventures::InterpreterIr

    LangVmResult = Data.define(:output, :memory, :vm, :module)

    class LangVmCompileError < StandardError; end

    def self.compile_to_iir(source, module_name: "brainfuck")
      instructions = [
        IR::IIRInstr.new("const", "ptr", [0], "u64")
      ]
      loop_stack = []
      loop_id = 0

      source.each_char do |char|
        case char
        when ">"
          instructions << IR::IIRInstr.new("add", "ptr", ["ptr", 1], "u64")
        when "<"
          instructions << IR::IIRInstr.new("sub", "ptr", ["ptr", 1], "u64")
        when "+"
          emit_cell_mutation(instructions, 1)
        when "-"
          emit_cell_mutation(instructions, -1)
        when "."
          instructions << IR::IIRInstr.new("load_mem", "cell", ["ptr"], "u8")
          instructions << IR::IIRInstr.new("io_out", nil, ["cell"], "void")
        when ","
          instructions << IR::IIRInstr.new("io_in", "cell", [], "u8")
          instructions << IR::IIRInstr.new("store_mem", nil, ["ptr", "cell"], "void")
        when "["
          start_label = "loop_#{loop_id}_start"
          end_label = "loop_#{loop_id}_end"
          loop_id += 1
          loop_stack << [start_label, end_label]
          instructions << IR::IIRInstr.new("label", nil, [start_label], "void")
          instructions << IR::IIRInstr.new("load_mem", "cell", ["ptr"], "u8")
          instructions << IR::IIRInstr.new("cmp_eq", "zero", ["cell", 0], "bool")
          instructions << IR::IIRInstr.new("jmp_if_true", nil, ["zero", end_label], "void")
        when "]"
          raise LangVmCompileError, "unmatched ']'" if loop_stack.empty?

          start_label, end_label = loop_stack.pop
          instructions << IR::IIRInstr.new("jmp", nil, [start_label], "void")
          instructions << IR::IIRInstr.new("label", nil, [end_label], "void")
        end
      end

      raise LangVmCompileError, "unmatched '['" unless loop_stack.empty?

      instructions << IR::IIRInstr.new("ret_void", nil, [], "void")
      fn = IR::IIRFunction.new(
        name: "main",
        return_type: "void",
        instructions: instructions,
        register_count: 8,
        type_status: IR::FunctionTypeStatus::PARTIALLY_TYPED
      )
      IR::IIRModule.new(name: module_name, functions: [fn], entry_point: "main", language: "brainfuck")
    end

    def self.execute_on_lang_vm(source, input: "", jit: false)
      mod = compile_to_iir(source)
      vm = CodingAdventures::VmCore::VMCore.new(input: input, u8_wrap: true)
      if jit
        CodingAdventures::JitCore::JITCore.new(vm).execute_with_jit(mod)
      else
        vm.execute(mod)
      end
      LangVmResult.new(output: vm.output.string, memory: vm.memory.dup, vm: vm, module: mod)
    end

    def self.emit_cell_mutation(instructions, delta)
      instructions << IR::IIRInstr.new("load_mem", "cell", ["ptr"], "u8")
      instructions << IR::IIRInstr.new("add", "cell", ["cell", delta], "u8")
      instructions << IR::IIRInstr.new("store_mem", nil, ["ptr", "cell"], "void")
    end
    private_class_method :emit_cell_mutation
  end
end
