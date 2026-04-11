# frozen_string_literal: true

module CodingAdventures
  module AVLTree
    AVLNode = Struct.new(:value, :left, :right, :height, :size, keyword_init: true)

    class AVLTree < CodingAdventures::BinarySearchTree::BinarySearchTree

      def self.empty
        new
      end

      def initialize(root = nil)
        super(nil)
        @root = normalize_root(root)
      end

      def insert(value)
        self.class.new(avl_insert(@root, value))
      end

      def delete(value)
        self.class.new(avl_delete(@root, value))
      end

      def search(value)
        avl_search(@root, value)
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

      def is_valid_bst
        bst_is_valid(@root)
      end

      def balance_factor(node)
        return 0 if node.nil?

        avl_height(node.left) - avl_height(node.right)
      end

      def is_valid_avl
        validate(@root, nil, nil)&.first ? true : false
      end

      def height
        avl_height(@root)
      end

      def size
        avl_size(@root)
      end

      def root_node
        @root
      end

      alias root root_node

      private

      def normalize_root(root)
        return nil if root.nil?
        return root if root.is_a?(AVLNode)

        AVLNode.new(value: root, height: 0, size: 1)
      end

      def avl_search(root, value)
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

      def avl_insert(root, value)
        return AVLNode.new(value: value, height: 0, size: 1) if root.nil?

        case value <=> root.value
        when -1
          root.left = avl_insert(root.left, value)
        when 1
          root.right = avl_insert(root.right, value)
        else
          return root
        end
        update_metadata(root)
        rebalance(root)
      end

      def avl_delete(root, value)
        return nil if root.nil?

        case value <=> root.value
        when -1
          root.left = avl_delete(root.left, value)
        when 1
          root.right = avl_delete(root.right, value)
        else
          return root.right if root.left.nil?
          return root.left if root.right.nil?

          new_right, successor = extract_min(root.right)
          root.value = successor
          root.right = new_right
        end

        update_metadata(root)
        rebalance(root)
      end

      def rotate_left(root)
        new_root = root.right
        return root if new_root.nil?

        root.right = new_root.left
        new_root.left = root
        update_metadata(root)
        update_metadata(new_root)
        new_root
      end

      def rotate_right(root)
        new_root = root.left
        return root if new_root.nil?

        root.left = new_root.right
        new_root.right = root
        update_metadata(root)
        update_metadata(new_root)
        new_root
      end

      def rebalance(node)
        bf = balance_factor(node)
        if bf > 1
          node.left = rotate_left(node.left) if balance_factor(node.left) < 0
          rotate_right(node)
        elsif bf < -1
          node.right = rotate_right(node.right) if balance_factor(node.right) > 0
          rotate_left(node)
        else
          node
        end
      end

      def avl_height(root)
        root&.height || -1
      end

      def avl_size(root)
        root&.size || 0
      end

      def update_metadata(node)
        node.height = 1 + [avl_height(node.left), avl_height(node.right)].max
        node.size = 1 + avl_size(node.left) + avl_size(node.right)
        node
      end

      def extract_min(node)
        return [node.right, node.value] if node.left.nil?

        new_left, successor = extract_min(node.left)
        node.left = new_left
        update_metadata(node)
        [node, successor]
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
          when -1, 0
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
          when 1, 0
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

        left_size = avl_size(root.left)
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
          1 + avl_size(root.left) + bst_rank(root.right, value)
        else
          avl_size(root.left)
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
        validate_bst(root, nil, nil)
      end

      def validate_bst(node, min, max)
        return true if node.nil?
        return false if !min.nil? && node.value <= min
        return false if !max.nil? && node.value >= max

        validate_bst(node.left, min, node.value) &&
          validate_bst(node.right, node.value, max) &&
          node.size == 1 + avl_size(node.left) + avl_size(node.right)
      end

      def validate(node, min, max)
        return [true, -1, 0] if node.nil?
        return nil if !min.nil? && node.value <= min
        return nil if !max.nil? && node.value >= max

        left = validate(node.left, min, node.value)
        right = validate(node.right, node.value, max)
        return nil if left.nil? || right.nil?

        left_valid, left_h, left_s = left
        right_valid, right_h, right_s = right
        return nil unless left_valid && right_valid

        height = 1 + [left_h, right_h].max
        size = 1 + left_s + right_s
        return nil if node.height != height || node.size != size
        return nil if (left_h - right_h).abs > 1

        [true, height, size]
      end
    end
  end
end
