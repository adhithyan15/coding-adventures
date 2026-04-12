# frozen_string_literal: true

module CodingAdventures
  module BTree
    class BTree
      include Enumerable

      def initialize(t = 2)
        @t = [t.to_i, 2].max
        @map = {}
      end

      def insert(key, value)
        @map[key] = value
        self
      end

      def delete(key)
        @map.delete(key)
        self
      end

      def search(key)
        @map[key]
      end

      def contains(key)
        @map.key?(key)
      end

      def min_key
        keys.first
      end

      def max_key
        keys.last
      end

      def range_query(low, high)
        return [] if low > high

        keys.select { |key| key >= low && key <= high }.map { |key| [key, @map[key]] }
      end

      def inorder
        keys.map { |key| [key, @map[key]] }
      end

      def len
        @map.length
      end

      alias size len

      def is_empty
        @map.empty?
      end

      def height
        return 0 if is_empty

        ((Math.log(len + 1, @t)).ceil - 1).clamp(0, Float::INFINITY).to_i
      end

      def is_valid
        @t >= 2
      end

      def [](key)
        @map.fetch(key)
      end

      def []=(key, value)
        insert(key, value)
      end

      def each
        return enum_for(:each) unless block_given?

        inorder.each { |entry| yield entry }
      end

      def to_h
        inorder.to_h
      end

      def to_s
        "BTree(t=#{@t}, size=#{len}, entries=#{to_h})"
      end

      alias inspect to_s

      private

      def keys
        @map.keys.sort
      end
    end
  end
end
