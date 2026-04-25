# frozen_string_literal: true

module CodingAdventures
  module FenwickTree
    class FenwickError < StandardError; end
    class IndexOutOfRangeError < FenwickError; end
    class EmptyTreeError < FenwickError; end

    class FenwickTree
      attr_reader :length

      def initialize(length)
        raise FenwickError, "size must be a non-negative integer" unless length.is_a?(Integer) && length >= 0

        @length = length
        @bit = Array.new(length + 1, 0.0)
      end

      def self.from_list(values)
        tree = new(values.length)
        (1..tree.length).each do |index|
          tree.instance_variable_get(:@bit)[index] += values.fetch(index - 1)
          parent = index + lowbit(index)
          tree.instance_variable_get(:@bit)[parent] += tree.instance_variable_get(:@bit)[index] if parent <= tree.length
        end
        tree
      end

      def update(index, delta)
        check_index!(index)
        current = index
        while current <= @length
          @bit[current] += delta
          current += self.class.lowbit(current)
        end
        nil
      end

      def prefix_sum(index)
        unless index.is_a?(Integer) && index >= 0 && index <= @length
          raise IndexOutOfRangeError, "prefix_sum index #{index} out of range [0, #{@length}]"
        end

        total = 0.0
        current = index
        while current.positive?
          total += @bit[current]
          current -= self.class.lowbit(current)
        end
        total
      end

      def range_sum(left, right)
        raise FenwickError, "left (#{left}) must be <= right (#{right})" if left > right

        check_index!(left)
        check_index!(right)
        left == 1 ? prefix_sum(right) : prefix_sum(right) - prefix_sum(left - 1)
      end

      def point_query(index)
        check_index!(index)
        range_sum(index, index)
      end

      def find_kth(target)
        raise EmptyTreeError, "find_kth called on empty tree" if @length.zero?
        raise FenwickError, "k must be positive, got #{target}" if target <= 0

        total = prefix_sum(@length)
        raise FenwickError, "k exceeds total sum of the tree" if target > total

        index = 0
        step = highest_power_of_two_at_most(@length)
        while step.positive?
          next_index = index + step
          if next_index <= @length && @bit[next_index] < target
            index = next_index
            target -= @bit[index]
          end
          step >>= 1
        end
        index + 1
      end

      def empty?
        @length.zero?
      end

      def bit_array
        @bit[1..] || []
      end

      def inspect
        "FenwickTree(n=#{@length}, bit=#{bit_array.inspect})"
      end

      alias to_s inspect

      def self.lowbit(index)
        index & -index
      end

      private

      def check_index!(index)
        return if index.is_a?(Integer) && index >= 1 && index <= @length

        raise IndexOutOfRangeError, "Index #{index} out of range [1, #{@length}]"
      end

      def highest_power_of_two_at_most(number)
        return 0 if number.zero?

        power = 1
        power <<= 1 while (power << 1) <= number
        power
      end
    end
  end
end
