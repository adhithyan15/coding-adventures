# frozen_string_literal: true

require "coding_adventures_hash_map"

module CodingAdventures
  module HashSet
    class HashSet
      include Enumerable

      def initialize(values = nil)
        @map = ::CodingAdventures::HashMap::HashMap.new
        values&.each { |value| add(value) }
      end

      def add(value)
        @map[value] = true
        self
      end

      def delete(value)
        !@map.delete(value).nil?
      end

      def include?(value)
        @map.key?(value)
      end
      alias member? include?

      def size
        @map.size
      end

      def empty?
        @map.empty?
      end

      def each
        return enum_for(:each) unless block_given?

        @map.each { |value, _| yield value }
      end

      def to_a
        each.to_a
      end

      def union(other)
        result = self.class.new(to_a)
        other.each { |value| result.add(value) }
        result
      end

      def intersection(other)
        result = self.class.new
        each { |value| result.add(value) if other.include?(value) }
        result
      end

      def difference(other)
        result = self.class.new
        each { |value| result.add(value) unless other.include?(value) }
        result
      end

      def subset?(other)
        all? { |value| other.include?(value) }
      end

      def superset?(other)
        other.subset?(self)
      end
    end
  end
end
