# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_codegen_core"
require "coding_adventures_interpreter_ir"

class CodegenCoreTest < Minitest::Test
  IR = CodingAdventures::InterpreterIr

  def test_default_registry_emits_all_requested_targets
    mod = IR::IIRModule.new(
      name: "demo",
      language: "test",
      functions: [
        IR::IIRFunction.new(
          name: "main",
          instructions: [
            IR::IIRInstr.new("const", "x", [1], "u8"),
            IR::IIRInstr.new("ret", nil, ["x"], "u8")
          ]
        )
      ]
    )
    registry = CodingAdventures::CodegenCore::BackendRegistry.default

    %i[pure_vm jvm clr wasm].each do |target|
      artifact = registry.compile(mod, target: target)
      assert_equal target, artifact.target
      assert_includes artifact.body, ".function main"
    end
  end
end
