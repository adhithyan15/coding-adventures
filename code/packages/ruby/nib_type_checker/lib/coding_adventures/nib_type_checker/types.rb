# frozen_string_literal: true

module CodingAdventures
  module NibTypeChecker
    module Types
      U4 = :u4
      U8 = :u8
      BCD = :bcd
      BOOL = :bool
      VOID = :void
      LITERAL = :literal

      NUMERIC = [U4, U8, BCD].freeze

      def self.parse_type_name(name)
        case name
        when "u4" then U4
        when "u8" then U8
        when "bcd" then BCD
        when "bool" then BOOL
        else nil
        end
      end

      def self.numeric?(type)
        NUMERIC.include?(type)
      end

      def self.compatible?(expected, actual)
        return true if expected == actual
        actual == LITERAL && numeric?(expected)
      end
    end
  end
end
