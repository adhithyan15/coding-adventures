# frozen_string_literal: true

module CodingAdventures
  module BPlusTree
    class BPlusTree < CodingAdventures::BTree::BTree
      def range_scan(low, high)
        range_query(low, high)
      end

      def full_scan
        inorder
      end

      def iter
        return enum_for(:iter) unless block_given?

        full_scan.each { |(key, _)| yield key }
      end

      def items
        return enum_for(:items) unless block_given?

        full_scan.each { |entry| yield entry }
      end

      def to_s
        "BPlusTree(t=#{instance_variable_get(:@t)}, size=#{len}, entries=#{to_h})"
      end

      alias inspect to_s
    end
  end
end
