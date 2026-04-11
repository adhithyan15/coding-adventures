# frozen_string_literal: true

module CodingAdventures
  module SkipList
    class SkipList
      Node = Struct.new(:key, :value, :forward, keyword_init: true)

      include Enumerable

      attr_reader :size

      def initialize(max_level: 16, probability: 0.5, &comparator)
        raise ArgumentError, "max_level must be positive" if max_level <= 0
        raise ArgumentError, "probability must be between 0 and 1" unless probability > 0.0 && probability < 1.0

        @max_level = max_level
        @probability = probability
        @compare = comparator || ->(left, right) { left <=> right }
        @head = Node.new(key: nil, value: nil, forward: Array.new(@max_level))
        @level = 0
        @size = 0
      end

      def insert(key, value = key)
        update = Array.new(@max_level)
        node = @head

        (@level).downto(0) do |level|
          node = advance(node, level, key)
          update[level] = node
        end

        node = node.forward[0]
        if node && compare(node.key, key).zero?
          old_value = node.value
          node.value = value
          return old_value
        end

        new_level = random_level
        if new_level > @level
          (@level + 1..new_level).each { |level| update[level] = @head }
          @level = new_level
        end

        inserted = Node.new(key: key, value: value, forward: Array.new(new_level + 1))
        (0..new_level).each do |level|
          inserted.forward[level] = update[level].forward[level]
          update[level].forward[level] = inserted
        end
        @size += 1
        value
      end

      def search(key)
        node = find_node(key)
        node&.value
      end

      def delete(key)
        update = Array.new(@max_level)
        node = @head

        (@level).downto(0) do |level|
          node = advance(node, level, key)
          update[level] = node
        end

        node = node.forward[0]
        return nil unless node && compare(node.key, key).zero?

        (0..@level).each do |level|
          break if update[level].forward[level] != node

          update[level].forward[level] = node.forward[level]
        end

        @level -= 1 while @level.positive? && @head.forward[@level].nil?
        @size -= 1
        node.value
      end

      def include?(key)
        !find_node(key).nil?
      end

      def first
        node = @head.forward[0]
        node && [node.key, node.value]
      end

      def last
        node = @head
        @level.downto(0) do |level|
          node = advance(node, level, nil, true)
        end
        node.equal?(@head) ? nil : [node.key, node.value]
      end

      def rank(key)
        index = 0
        node = @head.forward[0]
        while node
          return index if compare(node.key, key).zero?

          index += 1
          node = node.forward[0]
        end
        nil
      end

      def range(min_key, max_key, inclusive: true)
        result = []
        node = @head.forward[0]

        while node && compare(node.key, min_key) < 0
          node = node.forward[0]
        end

        while node
          comparison = compare(node.key, max_key)
          break if comparison.positive? || (!inclusive && comparison.zero?)

          result << [node.key, node.value]
          node = node.forward[0]
        end

        result
      end

      def each
        return enum_for(:each) unless block_given?

        node = @head.forward[0]
        while node
          yield node.key, node.value
          node = node.forward[0]
        end
      end

      def to_a
        each.map { |key, value| [key, value] }
      end

      private

      def compare(left, right)
        @compare.call(left, right)
      end

      def advance(node, level, key, to_end = false)
        loop do
          next_node = node.forward[level]
          break if next_node.nil?
          break if !to_end && compare(next_node.key, key) >= 0

          node = next_node
        end
        node
      end

      def find_node(key)
        node = @head
        (@level).downto(0) do |level|
          node = advance(node, level, key)
        end
        node = node.forward[0]
        node if node && compare(node.key, key).zero?
      end

      def random_level
        level = 0
        while level < (@max_level - 1) && rand < @probability
          level += 1
        end
        level
      end
    end
  end
end
