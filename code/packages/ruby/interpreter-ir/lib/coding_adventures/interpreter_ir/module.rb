# frozen_string_literal: true

require_relative "function"

module CodingAdventures
  module InterpreterIr
    class IIRModule
      attr_accessor :name, :functions, :entry_point, :language, :metadata

      def initialize(name:, functions: [], entry_point: "main", language: "unknown", metadata: {})
        @name = name
        @functions = functions
        @entry_point = entry_point
        @language = language
        @metadata = metadata
      end

      def get_function(fn_name)
        @functions.find { |fn| fn.name == fn_name }
      end

      def function_names
        @functions.map(&:name)
      end

      def add_or_replace(fn)
        idx = @functions.index { |existing| existing.name == fn.name }
        if idx
          @functions[idx] = fn
        else
          @functions << fn
        end
      end

      def validate
        errors = []
        seen = {}
        @functions.each do |fn|
          errors << "duplicate function name: #{fn.name.inspect}" if seen[fn.name]
          seen[fn.name] = true
        end
        if @entry_point && !seen[@entry_point]
          errors << "entry_point #{@entry_point.inspect} not found in module functions"
        end
        @functions.each do |fn|
          labels = fn.instructions.select { |i| i.op == "label" }.map { |i| i.srcs.first }
          fn.instructions.each do |instr|
            next unless %w[jmp jmp_if_true jmp_if_false].include?(instr.op)

            label = instr.srcs.last
            errors << "function #{fn.name.inspect}: branch to undefined label #{label.inspect}" if label.is_a?(String) && !labels.include?(label)
          end
        end
        errors
      end
    end
  end
end
