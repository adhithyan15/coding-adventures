# frozen_string_literal: true

module CodingAdventures
  module Treap
    TreapNode = Struct.new(:key, :priority, :left, :right, :size, keyword_init: true)

    class Treap < CodingAdventures::BinarySearchTree::BinarySearchTree

      @seed = 0x9E37_79B9

      class << self
        attr_accessor :seed
      end

      def self.empty
        new
      end

      def self.merge(left, right)
        new(merge_nodes(left.root, right.root))
      end

      def initialize(root = nil)
        super(nil)
        @root = normalize_root(root)
      end

      def insert(key, priority = nil)
        self.class.new(treap_insert(@root, key, priority))
      end

      def delete(key)
        self.class.new(treap_delete(@root, key))
      end

      def search(key)
        treap_search(@root, key)
      end

      def contains(key)
        !search(key).nil?
      end

      def split(key)
        left, right = treap_split(@root, key)
        [self.class.new(left), self.class.new(right)]
      end

      def min_key
        treap_min_key(@root)
      end

      def max_key
        treap_max_key(@root)
      end

      def predecessor(key)
        bst_predecessor(@root, key)
      end

      def successor(key)
        bst_successor(@root, key)
      end

      def kth_smallest(k)
        bst_kth_smallest(@root, k)
      end

      def to_sorted_array
        bst_to_sorted_array(@root)
      end

      def is_valid_treap
        is_valid_treap_node?(@root)
      end

      def height
        treap_height(@root)
      end

      def size
        treap_size(@root)
      end

      def root_node
        @root
      end

      alias root root_node

      private

      def normalize_root(root)
        return nil if root.nil?
        return root if root.is_a?(TreapNode)

        TreapNode.new(key: root, priority: next_priority, size: 1)
      end

      def treap_search(root, key)
        current = root
        until current.nil?
          case key <=> current.key
          when -1 then current = current.left
          when 1 then current = current.right
          else return current
          end
        end
        nil
      end

      def treap_insert(root, key, priority)
        return TreapNode.new(key: key, priority: priority || next_priority, size: 1) if root.nil?

        case key <=> root.key
        when -1
          root.left = treap_insert(root.left, key, priority)
          root = rotate_right(root) if root.left&.priority.to_f > root.priority.to_f
        when 1
          root.right = treap_insert(root.right, key, priority)
          root = rotate_left(root) if root.right&.priority.to_f > root.priority.to_f
        else
          root.priority = priority if priority
        end
        update_metadata(root)
      end

      def treap_delete(root, key)
        return nil if root.nil?

        case key <=> root.key
        when -1
          root.left = treap_delete(root.left, key)
        when 1
          root.right = treap_delete(root.right, key)
        else
          return merge_nodes(root.left, root.right)
        end
        update_metadata(root)
        root
      end

      def treap_split(root, key)
        return [nil, nil] if root.nil?

        case key <=> root.key
        when -1, 0
          left, right = treap_split(root.left, key)
          root.left = right
          update_metadata(root)
          [left, root]
        else
          left, right = treap_split(root.right, key)
          root.right = left
          update_metadata(root)
          [root, right]
        end
      end

      def self.merge_nodes(left, right)
        allocate.send(:merge_nodes, left, right)
      end

      def merge_nodes(left, right)
        if left.nil?
          right
        elsif right.nil?
          left
        elsif left.priority >= right.priority
          left.right = merge_nodes(left.right, right)
          update_metadata(left)
        else
          right.left = merge_nodes(left, right.left)
          update_metadata(right)
        end
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

      def update_metadata(node)
        node.size = 1 + treap_size(node.left) + treap_size(node.right)
        node
      end

      def treap_min_key(root)
        current = root
        current = current.left while current&.left
        current&.key
      end

      def treap_max_key(root)
        current = root
        current = current.right while current&.right
        current&.key
      end

      def treap_height(root)
        return -1 if root.nil?

        1 + [treap_height(root.left), treap_height(root.right)].max
      end

      def treap_size(root)
        root&.size || 0
      end

      def is_valid_treap_node?(root)
        validate(root, nil, nil, nil)
      end

      def validate(node, min, max, parent_priority)
        return true if node.nil?
        return false if !min.nil? && node.key <= min
        return false if !max.nil? && node.key > max
        return false if !parent_priority.nil? && node.priority.to_f > parent_priority.to_f

        validate(node.left, min, node.key, node.priority) &&
          validate(node.right, node.key, max, node.priority) &&
          node.size == 1 + treap_size(node.left) + treap_size(node.right)
      end

      def bst_predecessor(root, key)
        current = root
        best = nil
        until current.nil?
          case key <=> current.key
          when -1, 0
            current = current.left
          else
            best = current.key
            current = current.right
          end
        end
        best
      end

      def bst_successor(root, key)
        current = root
        best = nil
        until current.nil?
          case key <=> current.key
          when 1, 0
            current = current.right
          else
            best = current.key
            current = current.left
          end
        end
        best
      end

      def bst_kth_smallest(root, k)
        return nil if k <= 0 || root.nil?

        left_size = treap_size(root.left)
        if k == left_size + 1
          root.key
        elsif k <= left_size
          bst_kth_smallest(root.left, k)
        else
          bst_kth_smallest(root.right, k - left_size - 1)
        end
      end

      def bst_to_sorted_array(root)
        out = []
        inorder(root, out)
        out
      end

      def inorder(root, out)
        return if root.nil?

        inorder(root.left, out)
        out << root.key
        inorder(root.right, out)
      end

      def next_priority
        self.class.seed = ((self.class.seed * 1103515245) + 12345) & 0xffff_ffff
        self.class.seed.to_f / 0xffff_ffff
      end
    end
  end
end
