# frozen_string_literal: true

module CodingAdventures
  module InterpreterIr
    module SlotKind
      UNINITIALIZED = :uninitialized
      MONOMORPHIC = :monomorphic
      POLYMORPHIC = :polymorphic
      MEGAMORPHIC = :megamorphic
    end

    class SlotState
      attr_reader :kind, :observations, :count

      def initialize(kind: SlotKind::UNINITIALIZED, observations: [], count: 0)
        @kind = kind
        @observations = observations.dup
        @count = count
      end

      def record(runtime_type)
        @count += 1
        @observations << runtime_type unless @observations.include?(runtime_type)

        @kind = case @observations.length
        when 0
          SlotKind::UNINITIALIZED
        when 1
          SlotKind::MONOMORPHIC
        when 2..4
          SlotKind::POLYMORPHIC
        else
          SlotKind::MEGAMORPHIC
        end
        self
      end

      def monomorphic?
        @kind == SlotKind::MONOMORPHIC
      end

      def polymorphic?
        @kind == SlotKind::POLYMORPHIC || @kind == SlotKind::MEGAMORPHIC
      end
    end
  end
end
