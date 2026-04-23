# frozen_string_literal: true

module CodingAdventures
  module IrToWasmValidator
    ValidationIssue = Struct.new(:rule, :message, keyword_init: true)

    class WasmIrValidator
      def validate(program, function_signatures = nil)
        CodingAdventures::IrToWasmCompiler.compile(program, function_signatures)
        []
      rescue CodingAdventures::IrToWasmCompiler::WasmLoweringError => error
        [ValidationIssue.new(rule: "lowering", message: error.message)]
      end
    end

    module_function

    def validate(program, function_signatures = nil)
      WasmIrValidator.new.validate(program, function_signatures)
    end
  end
end
