# frozen_string_literal: true

require_relative "artifact"

module CodingAdventures
  module CodegenCore
    class TextBackend
      attr_reader :target

      def initialize(target)
        @target = target.to_sym
      end

      def compile_module(mod)
        lines = []
        lines << "; LANG target=#{@target} module=#{mod.name} language=#{mod.language}"
        lines << ".entry #{mod.entry_point}" if mod.entry_point
        mod.functions.each do |fn|
          lines << ""
          lines << ".function #{fn.name} #{fn.params.map { |n, t| "#{n}:#{t}" }.join(" ")} -> #{fn.return_type}"
          fn.instructions.each_with_index do |instr, idx|
            lhs = instr.dest ? "#{instr.dest} = " : ""
            args = instr.srcs.map(&:inspect).join(", ")
            lines << format("  %04d  %s%s(%s) : %s", idx, lhs, instr.op, args, instr.type_hint)
          end
          lines << ".end"
        end
        Artifact.new(
          target: @target,
          format: "#{@target}-lang-ir-text",
          body: lines.join("\n") + "\n",
          metadata: {functions: mod.function_names, entry_point: mod.entry_point}
        )
      end
    end
  end
end
