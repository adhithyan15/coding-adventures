# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_jit_core"

class JitCoreTest < Minitest::Test
  IR = CodingAdventures::InterpreterIr

  def test_fully_typed_function_can_execute_through_jit_handler
    mod = IR::IIRModule.new(
      name: "jit-demo",
      functions: [
        IR::IIRFunction.new(
          name: "main",
          params: [["x", "u8"]],
          return_type: "u8",
          instructions: [
            IR::IIRInstr.new("add", "y", ["x", 1], "u8"),
            IR::IIRInstr.new("ret", nil, ["y"], "u8")
          ]
        )
      ]
    )
    vm = CodingAdventures::VmCore::VMCore.new(u8_wrap: true)
    jit = CodingAdventures::JitCore::JITCore.new(vm)

    assert_equal 42, jit.execute_with_jit(mod, args: [41])
    assert_equal 1, vm.metrics.total_jit_hits
  end

  def test_emit_routes_to_codegen_registry
    mod = IR::IIRModule.new(name: "m", functions: [IR::IIRFunction.new(name: "main")])
    artifact = CodingAdventures::JitCore::JITCore.new(CodingAdventures::VmCore::VMCore.new).emit(mod, target: :wasm)

    assert_equal :wasm, artifact.target
  end
end
