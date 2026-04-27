# frozen_string_literal: true

require_relative "test_helper"

class TestIrToWasmCompiler < Minitest::Test
  def test_infers_nib_style_function_signatures_from_comments
    program = IR::IrProgram.new("_start")
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::COMMENT, [IR::IrLabel.new("function:add(a, b)")], 0))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::LABEL, [IR::IrLabel.new("_fn_add")], -1))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::HALT, [], 1))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::LABEL, [IR::IrLabel.new("_start")], -1))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::HALT, [], 2))

    signatures = ITWC.infer_function_signatures_from_comments(program)

    assert_equal 2, signatures["_fn_add"].param_count
    assert_equal "add", signatures["_fn_add"].export_name
    assert_equal "_start", signatures["_start"].export_name
  end

  def test_lowers_minimal_start_function
    program = IR::IrProgram.new("_start")
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::LABEL, [IR::IrLabel.new("_start")], -1))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::HALT, [], 0))

    wasm_module = ITWC.compile(program, [ITWC::FunctionSignature.new(label: "_start", param_count: 0, export_name: "_start")])

    assert_equal 1, wasm_module.types.length
    assert_equal 1, wasm_module.functions.length
    assert_equal 1, wasm_module.code.length
    assert_equal "_start", wasm_module.exports.last.name
  end

  def test_lowers_wasi_write_to_import_and_memory
    program = IR::IrProgram.new("_start")
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::LABEL, [IR::IrLabel.new("_start")], -1))
    program.add_instruction(
      IR::IrInstruction.new(
        IR::IrOp::LOAD_IMM,
        [IR::IrRegister.new(4), IR::IrImmediate.new(65)],
        0
      )
    )
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::SYSCALL, [IR::IrImmediate.new(1)], 1))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::HALT, [], 2))

    wasm_module = ITWC.compile(program, [ITWC::FunctionSignature.new(label: "_start", param_count: 0, export_name: "_start")])

    assert_equal ["fd_write"], wasm_module.imports.map(&:name)
    assert_equal 1, wasm_module.memories.length
    assert_includes wasm_module.exports.map(&:name), "memory"
    assert_includes wasm_module.exports.map(&:name), "_start"
  end

  def test_rejects_unsupported_syscalls
    program = IR::IrProgram.new("_start")
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::LABEL, [IR::IrLabel.new("_start")], -1))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::SYSCALL, [IR::IrImmediate.new(99)], 0))

    error = assert_raises(ITWC::WasmLoweringError) do
      ITWC.compile(program, [ITWC::FunctionSignature.new(label: "_start", param_count: 0, export_name: "_start")])
    end

    assert_match(/unsupported SYSCALL/, error.message)
  end

  def test_lowers_memory_arithmetic_and_all_supported_syscalls
    program = IR::IrProgram.new("_start")
    program.add_data(IR::IrDataDecl.new("tape", 16, 0))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::LABEL, [IR::IrLabel.new("_start")], -1))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::LOAD_ADDR, [IR::IrRegister.new(0), IR::IrLabel.new("tape")], 0))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::LOAD_IMM, [IR::IrRegister.new(1), IR::IrImmediate.new(0)], 1))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::LOAD_IMM, [IR::IrRegister.new(2), IR::IrImmediate.new(65)], 2))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::STORE_BYTE, [IR::IrRegister.new(2), IR::IrRegister.new(0), IR::IrRegister.new(1)], 3))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::LOAD_BYTE, [IR::IrRegister.new(3), IR::IrRegister.new(0), IR::IrRegister.new(1)], 4))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::STORE_WORD, [IR::IrRegister.new(3), IR::IrRegister.new(0), IR::IrRegister.new(1)], 5))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::LOAD_WORD, [IR::IrRegister.new(4), IR::IrRegister.new(0), IR::IrRegister.new(1)], 6))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::ADD, [IR::IrRegister.new(5), IR::IrRegister.new(2), IR::IrRegister.new(3)], 7))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::ADD_IMM, [IR::IrRegister.new(6), IR::IrRegister.new(5), IR::IrImmediate.new(1)], 8))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::SUB, [IR::IrRegister.new(7), IR::IrRegister.new(6), IR::IrRegister.new(2)], 9))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::AND, [IR::IrRegister.new(8), IR::IrRegister.new(6), IR::IrRegister.new(7)], 10))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::AND_IMM, [IR::IrRegister.new(9), IR::IrRegister.new(8), IR::IrImmediate.new(255)], 11))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::CMP_EQ, [IR::IrRegister.new(10), IR::IrRegister.new(2), IR::IrRegister.new(3)], 12))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::CMP_NE, [IR::IrRegister.new(11), IR::IrRegister.new(2), IR::IrRegister.new(3)], 13))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::CMP_LT, [IR::IrRegister.new(12), IR::IrRegister.new(1), IR::IrRegister.new(2)], 14))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::CMP_GT, [IR::IrRegister.new(13), IR::IrRegister.new(2), IR::IrRegister.new(1)], 15))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::NOP, [], 16))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::LOAD_IMM, [IR::IrRegister.new(4), IR::IrImmediate.new(65)], 17))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::SYSCALL, [IR::IrImmediate.new(1)], 18))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::SYSCALL, [IR::IrImmediate.new(2)], 19))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::LOAD_IMM, [IR::IrRegister.new(4), IR::IrImmediate.new(0)], 20))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::SYSCALL, [IR::IrImmediate.new(10)], 21))

    wasm_module = ITWC.compile(program, [ITWC::FunctionSignature.new(label: "_start", param_count: 0, export_name: "_start")])

    assert_equal %w[fd_write fd_read proc_exit], wasm_module.imports.map(&:name)
    assert_equal 1, wasm_module.memories.length
    assert_equal 1, wasm_module.data.length
    assert_operator wasm_module.code.first.code.bytesize, :>, 20
  end

  def test_lowers_structured_if_and_loop_patterns
    program = IR::IrProgram.new("_start")
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::LABEL, [IR::IrLabel.new("_start")], -1))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::LOAD_IMM, [IR::IrRegister.new(2), IR::IrImmediate.new(1)], 0))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::BRANCH_Z, [IR::IrRegister.new(2), IR::IrLabel.new("if_0_else")], 1))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::NOP, [], 2))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::JUMP, [IR::IrLabel.new("if_0_end")], 3))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::LABEL, [IR::IrLabel.new("if_0_else")], -1))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::NOP, [], 4))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::LABEL, [IR::IrLabel.new("if_0_end")], -1))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::LABEL, [IR::IrLabel.new("loop_0_start")], -1))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::NOP, [], 5))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::BRANCH_Z, [IR::IrRegister.new(2), IR::IrLabel.new("loop_0_end")], 6))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::NOP, [], 7))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::JUMP, [IR::IrLabel.new("loop_0_start")], 8))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::LABEL, [IR::IrLabel.new("loop_0_end")], -1))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::HALT, [], 9))

    wasm_module = ITWC.compile(program, [ITWC::FunctionSignature.new(label: "_start", param_count: 0, export_name: "_start")])

    assert_equal 1, wasm_module.code.length
    assert_includes wasm_module.code.first.code.bytes, ITWC::OPCODE["if"]
    assert_includes wasm_module.code.first.code.bytes, ITWC::OPCODE["loop"]
    assert_includes wasm_module.code.first.code.bytes, ITWC::OPCODE["br_if"]
  end

  def test_lowers_calls_and_infers_callee_signature_from_comment
    program = IR::IrProgram.new("_start")
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::COMMENT, [IR::IrLabel.new("function:callee(a)")], 0))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::LABEL, [IR::IrLabel.new("_fn_callee")], -1))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::LOAD_IMM, [IR::IrRegister.new(1), IR::IrImmediate.new(7)], 1))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::RET, [], 2))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::LABEL, [IR::IrLabel.new("_start")], -1))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::LOAD_IMM, [IR::IrRegister.new(2), IR::IrImmediate.new(42)], 3))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::CALL, [IR::IrLabel.new("_fn_callee")], 4))
    program.add_instruction(IR::IrInstruction.new(IR::IrOp::HALT, [], 5))

    wasm_module = ITWC.compile(program)

    assert_equal 2, wasm_module.functions.length
    assert_includes wasm_module.exports.map(&:name), "callee"
    assert_includes wasm_module.exports.map(&:name), "_start"
  end

  def test_exposes_helper_primitives_and_validation_errors
    assert_equal 8, ITWC.align_up(5, 4)
    assert_equal "_start", ITWC.function_label_name(IR::IrInstruction.new(IR::IrOp::LABEL, [IR::IrLabel.new("_start")], -1))
    assert_nil ITWC.function_label_name(IR::IrInstruction.new(IR::IrOp::LABEL, [IR::IrLabel.new("plain")], -1))
    assert_equal "plain", ITWC.label_name_from_operand(IR::IrLabel.new("plain"))
    assert_equal [ITWC::OPCODE["i32.const"], 0x00, ITWC::OPCODE["end"]], ITWC.const_expr(0).bytes

    error = assert_raises(ITWC::WasmLoweringError) do
      ITWC.expect_register(IR::IrImmediate.new(1), "bad register")
    end
    assert_match(/expected register/, error.message)

    error = assert_raises(ITWC::WasmLoweringError) do
      ITWC.expect_immediate(IR::IrLabel.new("x"), "bad immediate")
    end
    assert_match(/expected immediate/, error.message)

    error = assert_raises(ITWC::WasmLoweringError) do
      ITWC.expect_label(IR::IrRegister.new(1), "bad label")
    end
    assert_match(/expected label/, error.message)

    error = assert_raises(ITWC::WasmLoweringError) do
      ITWC.make_function_ir(
        label: "_fn_missing",
        instructions: [IR::IrInstruction.new(IR::IrOp::LABEL, [IR::IrLabel.new("_fn_missing")], -1)],
        signatures: {}
      )
    end
    assert_match(/missing function signature/, error.message)
  end
end
