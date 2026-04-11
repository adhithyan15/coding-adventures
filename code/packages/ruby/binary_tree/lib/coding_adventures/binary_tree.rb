# frozen_string_literal: true

require "set"

module CodingAdventures
  module BinaryTree
    BinaryTreeNode = Struct.new(:value, :left, :right, keyword_init: true)

    class BinaryTree < CodingAdventures::Tree::Tree
      attr_reader :root

      def self.with_root(root)
        new(root)
      end

      def self.from_level_order(values)
        nodes = values.to_a
        tree = new
        root = tree.send(:build_from_level_order, nodes, 0)
        tree.instance_variable_set(:@root, root)
        tree
      end

      def initialize(root = nil)
        super(nil)
        @root = root.is_a?(BinaryTreeNode) || root.nil? ? root : BinaryTreeNode.new(value: root)
      end

      def is_full
        is_full_node?(@root)
      end

      def is_complete
        is_complete_node?(@root)
      end

      def is_perfect
        is_perfect_node?(@root)
      end

      def height
        height_node(@root)
      end

      def size
        size_node(@root)
      end

      def left_child(value)
        self.class.find(@root, value)&.left
      end

      def right_child(value)
        self.class.find(@root, value)&.right
      end

      def find(value)
        self.class.find(@root, value)
      end

      def inorder
        inorder_node(@root, [])
      end

      def preorder
        preorder_node(@root, [])
      end

      def postorder
        postorder_node(@root, [])
      end

      def level_order
        level_order_node(@root)
      end

      def to_array
        return [] if @root.nil?

        values = Array.new((2**(height + 1)) - 1)
        fill_array(@root, 0, values)
        values.pop while values.any? && values.last.nil?
        values
      end

      def to_ascii
        return "" if @root.nil?

        lines = []
        render_ascii(@root, "", true, lines)
        lines.join("\n")
      end

      def nodes
        level_order
      end

      def to_s
        "BinaryTree(root=#{@root&.value.inspect}, size=#{size})"
      end

      alias inspect to_s

      class << self
        def find(root, value)
          return nil if root.nil?
          return root if root.value == value

          find(root.left, value) || find(root.right, value)
        end
      end

      private

      def build_from_level_order(values, index)
        return nil if index >= values.length

        value = values[index]
        return nil if value.nil?

        BinaryTreeNode.new(
          value: value,
          left: build_from_level_order(values, 2 * index + 1),
          right: build_from_level_order(values, 2 * index + 2)
        )
      end

      def is_full_node?(node)
        return true if node.nil?

        left = node.left
        right = node.right
        return true if left.nil? && right.nil?
        return false if left.nil? || right.nil?

        is_full_node?(left) && is_full_node?(right)
      end

      def is_complete_node?(node)
        return true if node.nil?

        queue = [node]
        seen_nil = false
        until queue.empty?
          current = queue.shift
          if current.nil?
            seen_nil = true
            next
          end

          return false if seen_nil

          queue << current.left
          queue << current.right
        end
        true
      end

      def is_perfect_node?(node)
        return true if node.nil?

        h = height_node(node)
        size_node(node) == (2**(h + 1)) - 1
      end

      def height_node(node)
        return -1 if node.nil?

        1 + [height_node(node.left), height_node(node.right)].max
      end

      def size_node(node)
        return 0 if node.nil?

        1 + size_node(node.left) + size_node(node.right)
      end

      def inorder_node(node, out)
        return out if node.nil?

        inorder_node(node.left, out)
        out << node.value
        inorder_node(node.right, out)
        out
      end

      def preorder_node(node, out)
        return out if node.nil?

        out << node.value
        preorder_node(node.left, out)
        preorder_node(node.right, out)
        out
      end

      def postorder_node(node, out)
        return out if node.nil?

        postorder_node(node.left, out)
        postorder_node(node.right, out)
        out << node.value
        out
      end

      def level_order_node(node)
        return [] if node.nil?

        out = []
        queue = [node]
        until queue.empty?
          current = queue.shift
          out << current.value
          queue << current.left if current.left
          queue << current.right if current.right
        end
        out
      end

      def fill_array(node, index, out)
        return if node.nil? || index >= out.length

        out[index] = node.value
        fill_array(node.left, 2 * index + 1, out)
        fill_array(node.right, 2 * index + 2, out)
      end

      def render_ascii(node, prefix, last, lines)
        connector = prefix.empty? ? "" : (last ? "\u2514\u2500\u2500 " : "\u251C\u2500\u2500 ")
        lines << "#{prefix}#{connector}#{node.value}"

        children = [node.left, node.right].compact
        children.each_with_index do |child, index|
          next_prefix = prefix + (prefix.empty? ? "" : (last ? "    " : "\u2502   "))
          render_ascii(child, next_prefix, index == children.length - 1, lines)
        end
      end
    end
  end
end
