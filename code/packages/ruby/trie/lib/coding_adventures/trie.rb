# frozen_string_literal: true

module CodingAdventures
  module Trie
    class Trie
      include Enumerable

      def initialize(entries = [])
        @root = Node.new
        @size = 0
        entries.each { |key, value| insert(key, value) }
      end

      attr_reader :size
      alias length size

      def insert(key, value = true)
        assert_key!(key)
        node = @root
        key.each_char do |char|
          node.children[char] ||= Node.new
          node = node.children[char]
        end

        @size += 1 unless node.terminal
        node.terminal = true
        node.value = value
        nil
      end

      def search(key)
        assert_key!(key)
        node = find_node(key)
        node&.terminal ? node.value : nil
      end

      def key?(key)
        assert_key!(key)
        node = find_node(key)
        !!(node&.terminal)
      end

      alias contains? key?

      def delete(key)
        assert_key!(key)
        return false unless key?(key)

        delete_recursive(@root, key.chars, 0)
        @size -= 1
        true
      end

      def starts_with?(prefix)
        assert_key!(prefix)
        return @size.positive? if prefix.empty?

        !find_node(prefix).nil?
      end

      def words_with_prefix(prefix)
        assert_key!(prefix)
        node = find_node(prefix)
        return [] unless node

        results = []
        collect(node, prefix, results)
        results
      end

      def all_words
        results = []
        collect(@root, "", results)
        results
      end

      alias entries all_words
      alias to_a all_words

      def keys
        all_words.map(&:first)
      end

      def longest_prefix_match(input)
        assert_key!(input)
        node = @root
        current = +""
        best = node.terminal ? ["", node.value] : nil

        input.each_char do |char|
          child = node.children[char]
          break unless child

          current << char
          node = child
          best = [current.dup, node.value] if node.terminal
        end

        best
      end

      def each(&block)
        return enum_for(:each) unless block

        all_words.each { |entry| block.call(entry) }
      end

      def empty?
        @size.zero?
      end

      def valid?
        count_endpoints(@root) == @size
      end

      def inspect
        preview = all_words.first(5)
        "Trie(#{@size} keys: #{preview.inspect})"
      end

      alias to_s inspect

      private

      class Node
        attr_accessor :children, :terminal, :value

        def initialize
          @children = {}
          @terminal = false
          @value = nil
        end
      end

      def assert_key!(key)
        raise ArgumentError, "key must be a String" unless key.is_a?(String)
      end

      def find_node(key)
        key.each_char.reduce(@root) do |node, char|
          return nil unless node.children.key?(char)

          node.children[char]
        end
      end

      def collect(node, current, results)
        results << [current.dup, node.value] if node.terminal

        node.children.keys.sort.each do |char|
          collect(node.children.fetch(char), current + char, results)
        end
      end

      def delete_recursive(node, chars, depth)
        if depth == chars.length
          node.terminal = false
          node.value = nil
          return node.children.empty?
        end

        char = chars.fetch(depth)
        child = node.children.fetch(char)
        node.children.delete(char) if delete_recursive(child, chars, depth + 1)

        node.children.empty? && !node.terminal
      end

      def count_endpoints(node)
        count = node.terminal ? 1 : 0
        node.children.each_value do |child|
          count += count_endpoints(child)
        end
        count
      end
    end
  end
end
