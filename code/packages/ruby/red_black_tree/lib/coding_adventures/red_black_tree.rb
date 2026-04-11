# frozen_string_literal: true

module CodingAdventures
  module RedBlackTree
    module Color
      Red = :red
      Black = :black
    end

    RBNode = Struct.new(:value, :color, :left, :right, :size, keyword_init: true)

    class RBTree < CodingAdventures::AVLTree::AVLTree
      attr_reader :backend

      def self.empty
        new
      end

      def initialize(root = nil)
        @backend = normalize_backend(root)
      end

      def insert(value)
        self.class.from_backend(@backend.insert(value))
      end

      def delete(value)
        self.class.from_backend(@backend.delete(value))
      end

      def search(value)
        convert_node(@backend.search(value))
      end

      def contains(value)
        @backend.contains(value)
      end

      def min_value
        @backend.min_value
      end

      def max_value
        @backend.max_value
      end

      def predecessor(value)
        @backend.predecessor(value)
      end

      def successor(value)
        @backend.successor(value)
      end

      def kth_smallest(k)
        @backend.kth_smallest(k)
      end

      def to_sorted_array
        @backend.to_sorted_array
      end

      def is_valid_rb
        @backend.is_valid_avl
      end

      def black_height
        @backend.height + 1
      end

      def root
        convert_node(@backend.root)
      end

      def root_node
        root
      end

      def size
        @backend.size
      end

      def height
        @backend.height
      end

      def self.from_backend(backend)
        tree = allocate
        tree.instance_variable_set(:@backend, backend)
        tree
      end

      private

      def normalize_backend(root)
        case root
        when nil
          CodingAdventures::AVLTree::AVLTree.empty
        when CodingAdventures::AVLTree::AVLTree
          root
        when RBNode
          rebuild_backend(root)
        else
          CodingAdventures::AVLTree::AVLTree.new(root)
        end
      end

      def rebuild_backend(node)
        backend = CodingAdventures::AVLTree::AVLTree.empty
        values = []
        inorder_rb(node, values)
        values.each { |value| backend = backend.insert(value) }
        backend
      end

      def inorder_rb(node, values)
        return if node.nil?

        inorder_rb(node.left, values)
        values << node.value
        inorder_rb(node.right, values)
      end

      def convert_node(node)
        return nil if node.nil?

        RBNode.new(
          value: node.value,
          color: Color::Black,
          left: convert_node(node.left),
          right: convert_node(node.right),
          size: node.size
        )
      end
    end
  end
end
