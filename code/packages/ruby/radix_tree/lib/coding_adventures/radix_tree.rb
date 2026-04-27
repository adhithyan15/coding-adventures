# frozen_string_literal: true

module CodingAdventures
  module RadixTree
    class RadixTree
      def initialize(entries = [])
        @root = Node.new
        @size = 0
        entries.each { |key, value| insert(key, value) }
      end

      attr_reader :size
      alias length size

      def insert(key, value)
        @size += 1 if insert_recursive(@root, key, value)
        nil
      end

      alias put insert

      def search(key)
        node = @root
        remaining = key
        until remaining.empty?
          edge = node.children[remaining[0]]
          return nil unless edge

          common = common_prefix_len(remaining, edge.label)
          return nil if common < edge.label.length

          remaining = remaining[common..] || ""
          node = edge.child
        end
        node.terminal ? node.value : nil
      end

      alias get search

      def contains?(key)
        key_exists?(key)
      end

      def delete(key)
        deleted, = delete_recursive(@root, key)
        @size -= 1 if deleted
        deleted
      end

      def starts_with?(prefix)
        return @size.positive? if prefix.empty?

        node = @root
        remaining = prefix
        until remaining.empty?
          edge = node.children[remaining[0]]
          return false unless edge

          common = common_prefix_len(remaining, edge.label)
          return true if common == remaining.length
          return false if common < edge.label.length

          remaining = remaining[common..] || ""
          node = edge.child
        end
        node.terminal || !node.children.empty?
      end

      def words_with_prefix(prefix)
        return keys if prefix.empty?

        node = @root
        remaining = prefix
        path = +""
        until remaining.empty?
          edge = node.children[remaining[0]]
          return [] unless edge

          common = common_prefix_len(remaining, edge.label)
          if common == remaining.length
            if common == edge.label.length
              path << edge.label
              node = edge.child
              remaining = ""
            else
              results = []
              collect_keys(edge.child, path + edge.label, results)
              return results
            end
          elsif common < edge.label.length
            return []
          else
            path << edge.label
            remaining = remaining[common..] || ""
            node = edge.child
          end
        end

        results = []
        collect_keys(node, path, results)
        results
      end

      def longest_prefix_match(key)
        node = @root
        remaining = key
        consumed = 0
        best = node.terminal ? "" : nil

        until remaining.empty?
          edge = node.children[remaining[0]]
          break unless edge

          common = common_prefix_len(remaining, edge.label)
          break if common < edge.label.length

          consumed += common
          remaining = remaining[common..] || ""
          node = edge.child
          best = key[0...consumed] if node.terminal
        end
        best
      end

      def keys
        results = []
        collect_keys(@root, "", results)
        results
      end

      def values
        to_h.values
      end

      def to_h
        result = {}
        collect_values(@root, "", result)
        result
      end

      def node_count
        count_nodes(@root)
      end

      def empty?
        @size.zero?
      end

      def inspect
        "RadixTree(#{@size} keys: #{keys.first(5).inspect})"
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

      Edge = Struct.new(:label, :child, keyword_init: true)

      def insert_recursive(node, key, value)
        if key.empty?
          added = !node.terminal
          node.terminal = true
          node.value = value
          return added
        end

        first = key[0]
        edge = node.children[first]
        unless edge
          node.children[first] = Edge.new(label: key, child: leaf(value))
          return true
        end

        common = common_prefix_len(key, edge.label)
        return insert_recursive(edge.child, key[common..] || "", value) if common == edge.label.length

        common_label = edge.label[0...common]
        label_rest = edge.label[common..]
        key_rest = key[common..] || ""
        split = Node.new
        split.children[label_rest[0]] = Edge.new(label: label_rest, child: edge.child)
        if key_rest.empty?
          split.terminal = true
          split.value = value
        else
          split.children[key_rest[0]] = Edge.new(label: key_rest, child: leaf(value))
        end
        node.children[first] = Edge.new(label: common_label, child: split)
        true
      end

      def delete_recursive(node, key)
        if key.empty?
          return [false, false] unless node.terminal

          node.terminal = false
          node.value = nil
          return [true, node.children.length == 1]
        end

        first = key[0]
        edge = node.children[first]
        return [false, false] unless edge

        common = common_prefix_len(key, edge.label)
        return [false, false] if common < edge.label.length

        deleted, child_mergeable = delete_recursive(edge.child, key[common..] || "")
        return [false, false] unless deleted

        if child_mergeable
          grandchild = edge.child.children.values.first
          node.children[first] = Edge.new(label: edge.label + grandchild.label, child: grandchild.child)
        elsif !edge.child.terminal && edge.child.children.empty?
          node.children.delete(first)
        end

        [true, !node.terminal && node.children.length == 1]
      end

      def key_exists?(key)
        node = @root
        remaining = key
        until remaining.empty?
          edge = node.children[remaining[0]]
          return false unless edge

          common = common_prefix_len(remaining, edge.label)
          return false if common < edge.label.length

          remaining = remaining[common..] || ""
          node = edge.child
        end
        node.terminal
      end

      def collect_keys(node, current, results)
        results << current if node.terminal
        node.children.keys.sort.each do |first|
          edge = node.children.fetch(first)
          collect_keys(edge.child, current + edge.label, results)
        end
      end

      def collect_values(node, current, result)
        result[current] = node.value if node.terminal
        node.children.keys.sort.each do |first|
          edge = node.children.fetch(first)
          collect_values(edge.child, current + edge.label, result)
        end
      end

      def count_nodes(node)
        1 + node.children.values.sum { |edge| count_nodes(edge.child) }
      end

      def leaf(value)
        Node.new.tap do |node|
          node.terminal = true
          node.value = value
        end
      end

      def common_prefix_len(left, right)
        index = 0
        index += 1 while index < left.length && index < right.length && left[index] == right[index]
        index
      end
    end
  end
end
