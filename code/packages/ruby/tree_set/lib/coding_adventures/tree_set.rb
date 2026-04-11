# frozen_string_literal: true

module CodingAdventures
  module TreeSet
    class ComparableEntry
      attr_reader :value

      def initialize(value, compare)
        @value = value
        @compare = compare
      end

      def <=>(other)
        other_value = other.is_a?(ComparableEntry) ? other.value : other
        @compare.call(@value, other_value)
      end
    end

    class TreeSet
      include Enumerable

      def self.from_values(values, compare = nil, &block)
        new(values, compare, &block)
      end

      def initialize(values = nil, compare = nil, &block)
        @compare = block || compare || method(:default_compare)
        @tree = CodingAdventures::AVLTree::AVLTree.empty
        values&.each { |value| add(value) }
      end

      def add(value)
        @tree = @tree.insert(wrap(value))
        self
      end

      def delete(value)
        return false unless include?(value)

        @tree = @tree.delete(wrap(value))
        true
      end

      alias discard delete

      def include?(value)
        !@tree.search(wrap(value)).nil?
      end
      alias member? include?
      alias contains? include?

      def size
        @tree.size
      end
      alias length size

      def empty?
        size.zero?
      end
      alias is_empty? empty?

      def min
        unwrap(@tree.min_value)
      end
      alias first min

      def max
        unwrap(@tree.max_value)
      end
      alias last max

      def predecessor(value)
        unwrap(@tree.predecessor(wrap(value)))
      end

      def successor(value)
        unwrap(@tree.successor(wrap(value)))
      end

      def rank(value)
        @tree.rank(wrap(value))
      end

      def by_rank(rank)
        return nil if rank.negative? || rank >= size

        unwrap(@tree.kth_smallest(rank + 1))
      end

      def kth_smallest(k)
        return nil if k <= 0

        by_rank(k - 1)
      end

      def to_a
        @tree.to_sorted_array.map { |entry| unwrap(entry) }
      end
      alias to_list to_a
      alias to_sorted_array to_a

      def range(minimum, maximum, inclusive = true)
        return [] if compare(minimum, maximum) > 0

        values = to_a
        start_index = inclusive ? lower_bound(values, minimum) : upper_bound(values, minimum)
        end_index = inclusive ? upper_bound(values, maximum) : lower_bound(values, maximum)
        values[start_index...end_index] || []
      end

      def union(other)
        self.class.new(merge_unique(to_a, other.to_a), @compare)
      end

      def intersection(other)
        self.class.new(intersection_sorted(to_a, other.to_a), @compare)
      end

      def difference(other)
        self.class.new(difference_sorted(to_a, other.to_a), @compare)
      end

      def symmetric_difference(other)
        self.class.new(symmetric_difference_sorted(to_a, other.to_a), @compare)
      end

      def subset?(other)
        is_subset_sorted(to_a, other.to_a)
      end

      def superset?(other)
        other.subset?(self)
      end

      def disjoint?(other)
        is_disjoint_sorted(to_a, other.to_a)
      end

      def equals(other)
        return false unless other.respond_to?(:to_a) && other.to_a.length == size

        other_values = other.to_a
        values = to_a
        values.each_index.all? { |index| compare(values[index], other_values[index]).zero? }
      end

      def each
        return enum_for(:each) unless block_given?

        to_a.each { |value| yield value }
      end

      def inspect
        "TreeSet(#{to_a.inspect})"
      end
      alias to_s inspect

      private

      def wrap(value)
        ComparableEntry.new(value, @compare)
      end

      def unwrap(entry)
        entry&.value
      end

      def compare(left, right)
        @compare.call(left, right)
      end

      def default_compare(left, right)
        result = left <=> right
        return result unless result.nil?

        raise ArgumentError, "values are not comparable"
      end

      def lower_bound(items, value)
        low = 0
        high = items.length
        while low < high
          mid = (low + high) / 2
          if compare(items[mid], value) < 0
            low = mid + 1
          else
            high = mid
          end
        end
        low
      end

      def upper_bound(items, value)
        low = 0
        high = items.length
        while low < high
          mid = (low + high) / 2
          if compare(items[mid], value) <= 0
            low = mid + 1
          else
            high = mid
          end
        end
        low
      end

      def merge_unique(left, right)
        result = []
        li = 0
        ri = 0
        while li < left.length && ri < right.length
          order = compare(left[li], right[ri])
          if order < 0
            result << left[li]
            li += 1
          elsif order > 0
            result << right[ri]
            ri += 1
          else
            result << left[li]
            li += 1
            ri += 1
          end
        end
        result.concat(left[li..] || [])
        result.concat(right[ri..] || [])
        result
      end

      def intersection_sorted(left, right)
        result = []
        li = 0
        ri = 0
        while li < left.length && ri < right.length
          order = compare(left[li], right[ri])
          if order < 0
            li += 1
          elsif order > 0
            ri += 1
          else
            result << left[li]
            li += 1
            ri += 1
          end
        end
        result
      end

      def difference_sorted(left, right)
        result = []
        li = 0
        ri = 0
        while li < left.length && ri < right.length
          order = compare(left[li], right[ri])
          if order < 0
            result << left[li]
            li += 1
          elsif order > 0
            ri += 1
          else
            li += 1
            ri += 1
          end
        end
        result.concat(left[li..] || [])
        result
      end

      def symmetric_difference_sorted(left, right)
        result = []
        li = 0
        ri = 0
        while li < left.length && ri < right.length
          order = compare(left[li], right[ri])
          if order < 0
            result << left[li]
            li += 1
          elsif order > 0
            result << right[ri]
            ri += 1
          else
            li += 1
            ri += 1
          end
        end
        result.concat(left[li..] || [])
        result.concat(right[ri..] || [])
        result
      end

      def is_subset_sorted(left, right)
        li = 0
        ri = 0
        while li < left.length && ri < right.length
          order = compare(left[li], right[ri])
          if order < 0
            return false
          elsif order > 0
            ri += 1
          else
            li += 1
            ri += 1
          end
        end
        li == left.length
      end

      def is_disjoint_sorted(left, right)
        li = 0
        ri = 0
        while li < left.length && ri < right.length
          order = compare(left[li], right[ri])
          if order < 0
            li += 1
          elsif order > 0
            ri += 1
          else
            return false
          end
        end
        true
      end
    end
  end
end
