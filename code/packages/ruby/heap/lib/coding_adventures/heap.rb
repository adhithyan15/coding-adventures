# frozen_string_literal: true

module CodingAdventures
  module Heap
    class Heap
      include Enumerable

      def initialize(&comparator)
        @compare = comparator || ->(left, right) { left <=> right }
        @values = []
      end

      def size
        @values.length
      end

      def empty?
        @values.empty?
      end

      def peek
        @values.first
      end

      def push(value)
        @values << value
        bubble_up(@values.length - 1)
        self
      end
      alias << push

      def pop
        return nil if empty?

        swap(0, @values.length - 1)
        value = @values.pop
        bubble_down(0)
        value
      end

      def replace(value)
        if empty?
          @values << value
        else
          @values[0] = value
          bubble_down(0)
        end
        self
      end

      def each
        return enum_for(:each) unless block_given?

        @values.each { |value| yield value }
      end

      def to_a
        @values.dup
      end

      private

      def bubble_up(index)
        while index.positive?
          parent = (index - 1) / 2
          break if ordered?(@values[parent], @values[index])

          swap(parent, index)
          index = parent
        end
      end

      def bubble_down(index)
        loop do
          left = index * 2 + 1
          right = left + 1
          smallest = index

          smallest = left if left < @values.length && !ordered?(@values[smallest], @values[left])
          smallest = right if right < @values.length && !ordered?(@values[smallest], @values[right])

          break if smallest == index

          swap(index, smallest)
          index = smallest
        end
      end

      def swap(left, right)
        @values[left], @values[right] = @values[right], @values[left]
      end

      def ordered?(left, right)
        @compare.call(left, right) <= 0
      end
    end

    class MinHeap < Heap
      def initialize(&comparator)
        super(&comparator)
      end
    end

    class MaxHeap < Heap
      def initialize(&comparator)
        super { |left, right| -(comparator ? comparator.call(left, right) : (left <=> right)) }
      end
    end
  end
end
