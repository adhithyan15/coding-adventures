# frozen_string_literal: true

require_relative "opcodes"
require_relative "slot_state"

module CodingAdventures
  module InterpreterIr
    class IIRInstr
      attr_accessor :op, :dest, :srcs, :type_hint, :observed_type,
        :observation_count, :observed_slot, :deopt_anchor, :may_alloc

      def initialize(op, dest = nil, srcs = [], type_hint = Types::DYNAMIC,
        observed_type: nil, observation_count: 0, observed_slot: nil,
        deopt_anchor: nil, may_alloc: false)
        @op = op
        @dest = dest
        @srcs = srcs
        @type_hint = type_hint
        @observed_type = observed_type
        @observation_count = observation_count
        @observed_slot = observed_slot
        @deopt_anchor = deopt_anchor
        @may_alloc = may_alloc
      end

      def typed?
        Types.concrete?(@type_hint)
      end

      def has_observation?
        @observation_count.positive?
      end

      def polymorphic?
        @observed_type == Types::POLYMORPHIC
      end

      def effective_type
        return @type_hint if typed?
        return @observed_type if @observed_type && !polymorphic?

        Types::DYNAMIC
      end

      def record_observation(runtime_type)
        @observed_slot ||= SlotState.new
        @observed_slot.record(runtime_type)
        @observation_count = @observed_slot.count
        @observed_type = if @observed_slot.kind == SlotKind::MONOMORPHIC
          @observed_slot.observations.first
        elsif @observed_slot.kind == SlotKind::POLYMORPHIC ||
            @observed_slot.kind == SlotKind::MEGAMORPHIC
          Types::POLYMORPHIC
        end
      end

      def to_s
        lhs = @dest ? "#{@dest} = " : ""
        args = @srcs.map(&:inspect).join(", ")
        "#{lhs}#{@op}(#{args}) : #{@type_hint}"
      end
    end
  end
end
