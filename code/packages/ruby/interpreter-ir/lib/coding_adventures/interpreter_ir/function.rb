# frozen_string_literal: true

require_relative "instr"

module CodingAdventures
  module InterpreterIr
    module FunctionTypeStatus
      FULLY_TYPED = :fully_typed
      PARTIALLY_TYPED = :partially_typed
      UNTYPED = :untyped
    end

    class IIRFunction
      attr_accessor :name, :params, :return_type, :instructions, :register_count,
        :type_status, :call_count, :feedback_slots, :source_map

      def initialize(name:, params: [], return_type: Types::DYNAMIC, instructions: [],
        register_count: 8, type_status: nil, call_count: 0,
        feedback_slots: {}, source_map: [])
        @name = name
        @params = params
        @return_type = return_type
        @instructions = instructions
        @register_count = register_count
        @type_status = type_status || infer_type_status(params, instructions)
        @call_count = call_count
        @feedback_slots = feedback_slots
        @source_map = source_map
      end

      def param_names
        @params.map(&:first)
      end

      def param_types
        @params.map(&:last)
      end

      def infer_type_status(params = @params, instructions = @instructions)
        hints = params.map(&:last) + instructions.map(&:type_hint)
        typed = hints.count { |hint| Types.concrete?(hint) }
        return FunctionTypeStatus::UNTYPED if hints.empty? || typed.zero?
        return FunctionTypeStatus::FULLY_TYPED if typed == hints.length

        FunctionTypeStatus::PARTIALLY_TYPED
      end

      def label_index(label_name)
        idx = @instructions.index do |instr|
          instr.op == "label" && instr.srcs.first == label_name
        end
        raise KeyError, "label #{label_name.inspect} not found in #{@name.inspect}" unless idx

        idx
      end
    end
  end
end
