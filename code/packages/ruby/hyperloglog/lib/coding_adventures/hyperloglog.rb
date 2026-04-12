# frozen_string_literal: true

require "digest"

module CodingAdventures
  module HyperLogLog
    class HyperLogLog
      attr_reader :precision, :registers

      def initialize(precision: 10)
        raise ArgumentError, "precision must be between 4 and 16" unless precision.between?(4, 16)

        @precision = precision
        @register_count = 1 << precision
        @registers = Array.new(@register_count, 0)
      end

      def add(value)
        hash = stable_hash(value)
        index = hash & (@register_count - 1)
        remaining = hash >> @precision
        width = 64 - @precision
        rho = remaining.zero? ? width + 1 : width - remaining.bit_length + 1
        @registers[index] = [@registers[index], rho].max
        self
      end

      def merge!(other)
        raise ArgumentError, "precision mismatch" unless other.precision == precision

        @registers.each_index do |index|
          @registers[index] = [@registers[index], other.registers[index]].max
        end
        self
      end

      def count
        m = @register_count.to_f
        indicator = @registers.inject(0.0) { |sum, register| sum + (2.0 ** -register) }
        estimate = alpha_for(@register_count) * m * m / indicator
        zero_count = @registers.count(0)

        if estimate <= (2.5 * m) && zero_count.positive?
          m * Math.log(m / zero_count)
        else
          estimate
        end
      end

      def clear
        @registers.fill(0)
        self
      end

      def empty?
        @registers.all?(&:zero?)
      end

      private

      def alpha_for(m)
        case m
        when 16 then 0.673
        when 32 then 0.697
        when 64 then 0.709
        else 0.7213 / (1 + 1.079 / m)
        end
      end

      def stable_hash(value)
        payload =
          begin
            Marshal.dump(value)
          rescue TypeError, ArgumentError
            value.to_s
          end

        Digest::SHA256.digest(payload).unpack("Q>").first
      end
    end
  end
end
