# frozen_string_literal: true

module CodingAdventures
  module BinarySearchTree
    BSTNode = Struct.new(:value, :left, :right, :size, keyword_init: true)

    class BinarySearchTree < CodingAdventures::BinaryTree::BinaryTree

      def self.empty
        new
      end

      def self.from_sorted_array(array)
        values = array.to_a
        tree = new
        tree.instance_variable_set(:@root, tree.send(:build_balanced, values))
        tree
      end

      def initialize(root = nil)
        super(nil)
        @root = normalize_root(root)
      end

      def insert(value)
        self.class.new(bst_insert(@root, value))
      end

      def delete(value)
        self.class.new(bst_delete(@root, value))
      end

      def search(value)
        bst_search(@root, value)
      end

      def contains(value)
        !search(value).nil?
      end

      def min_value
        bst_min_value(@root)
      end

      def max_value
        bst_max_value(@root)
      end

      def predecessor(value)
        bst_predecessor(@root, value)
      end

      def successor(value)
        bst_successor(@root, value)
      end

      def kth_smallest(k)
        bst_kth_smallest(@root, k)
      end

      def rank(value)
        bst_rank(@root, value)
      end

      def to_sorted_array
        bst_to_sorted_array(@root)
      end

      def is_valid
        bst_is_valid(@root)
      end

      def height
        bst_height(@root)
      end

      def size
        bst_size(@root)
      end

      def root_node
        @root
      end

      alias root root_node

      def to_s
        "BinarySearchTree(root=#{@root&.value.inspect}, size=#{size})"
      end

      alias inspect to_s

      private

      def normalize_root(root)
        return nil if root.nil?
        return root if root.is_a?(BSTNode)

        BSTNode.new(value: root, size: 1)
      end

      def bst_search(root, value)
        current = root
        until current.nil?
          case value <=> current.value
          when -1 then current = current.left
          when 1 then current = current.right
          else return current
          end
        end
        nil
      end

      def bst_insert(root, value)
        return BSTNode.new(value: value, size: 1) if root.nil?

        case value <=> root.value
        when -1
          root.left = bst_insert(root.left, value)
        when 1
          root.right = bst_insert(root.right, value)
        else
          return root
        end
        update_size(root)
      end

      def bst_delete(root, value)
        return nil if root.nil?

        case value <=> root.value
        when -1
          root.left = bst_delete(root.left, value)
          update_size(root)
        when 1
          root.right = bst_delete(root.right, value)
          update_size(root)
        else
          return root.right if root.left.nil?
          return root.left if root.right.nil?

          new_right, successor = extract_min(root.right)
          root.value = successor
          root.right = new_right
          update_size(root)
        end
      end

      def bst_min_value(root)
        current = root
        current = current.left while current&.left
        current&.value
      end

      def bst_max_value(root)
        current = root
        current = current.right while current&.right
        current&.value
      end

      def bst_predecessor(root, value)
        current = root
        best = nil
        until current.nil?
          case value <=> current.value
          when -1
            current = current.left
          when 0
            current = current.left
          else
            best = current.value
            current = current.right
          end
        end
        best
      end

      def bst_successor(root, value)
        current = root
        best = nil
        until current.nil?
          case value <=> current.value
          when 1
            current = current.right
          when 0
            current = current.right
          else
            best = current.value
            current = current.left
          end
        end
        best
      end

      def bst_kth_smallest(root, k)
        return nil if k <= 0 || root.nil?

        left_size = bst_size(root.left)
        if k == left_size + 1
          root.value
        elsif k <= left_size
          bst_kth_smallest(root.left, k)
        else
          bst_kth_smallest(root.right, k - left_size - 1)
        end
      end

      def bst_rank(root, value)
        return 0 if root.nil?

        case value <=> root.value
        when -1
          bst_rank(root.left, value)
        when 1
          1 + bst_size(root.left) + bst_rank(root.right, value)
        else
          bst_size(root.left)
        end
      end

      def bst_to_sorted_array(root)
        result = []
        inorder(root, result)
        result
      end

      def inorder(root, result)
        return if root.nil?

        inorder(root.left, result)
        result << root.value
        inorder(root.right, result)
      end

      def bst_is_valid(root)
        validate(root, nil, nil)
      end

      def validate(node, min, max)
        return true if node.nil?
        return false if !min.nil? && node.value <= min
        return false if !max.nil? && node.value >= max

        validate(node.left, min, node.value) &&
          validate(node.right, node.value, max) &&
          node.size == 1 + bst_size(node.left) + bst_size(node.right)
      end

      def bst_height(root)
        return -1 if root.nil?

        1 + [bst_height(root.left), bst_height(root.right)].max
      end

      def bst_size(root)
        root&.size || 0
      end

      def update_size(node)
        node.size = 1 + bst_size(node.left) + bst_size(node.right)
        node
      end

      def extract_min(node)
        return [node.right, node.value] if node.left.nil?

        successor, new_left = extract_min(node.left)
        node.left = successor
        update_size(node)
        [node, new_left]
      end

      def build_balanced(values)
        return nil if values.empty?

        mid = values.length / 2
        node = BSTNode.new(value: values[mid], size: 1)
        node.left = build_balanced(values[0...mid])
        node.right = build_balanced(values[(mid + 1)..] || [])
        update_size(node)
      end
    end
  end
end
