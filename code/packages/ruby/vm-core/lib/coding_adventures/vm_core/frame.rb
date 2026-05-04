# frozen_string_literal: true

module CodingAdventures
  module VmCore
    class VMFrame
      attr_reader :fn, :registers, :slots
      attr_accessor :ip

      def initialize(fn, args = [])
        @fn = fn
        @ip = 0
        @registers = {}
        @slots = Array.new([fn.register_count, fn.params.length, 8].max)
        fn.params.each_with_index do |(name, _type), idx|
          value = args[idx]
          @registers[name] = value
          @slots[idx] = value
        end
      end

      def resolve(value)
        if value.is_a?(String) && @registers.key?(value)
          @registers[value]
        else
          value
        end
      end

      def write(name, value)
        @registers[name] = value if name
        value
      end

      def load_slot(index)
        @slots.fetch(index, nil)
      end

      def store_slot(index, value)
        @slots[index] = value
      end
    end
  end
end
