# frozen_string_literal: true

module CodingAdventures
  module BloomFilter
    class BloomFilter
      DEFAULT_EXPECTED_ITEMS = 1_000
      DEFAULT_FALSE_POSITIVE_RATE = 0.01
      MASK32 = 0xffff_ffff

      attr_reader :bit_count, :hash_count, :bits_set

      def initialize(expected_items: DEFAULT_EXPECTED_ITEMS, false_positive_rate: DEFAULT_FALSE_POSITIVE_RATE)
        validate_expected_items!(expected_items)
        validate_false_positive_rate!(false_positive_rate)

        @bit_count = self.class.optimal_m(expected_items, false_positive_rate)
        @hash_count = self.class.optimal_k(@bit_count, expected_items)
        @expected_items = expected_items
        @bits = Array.new((@bit_count + 7) / 8, 0)
        @bits_set = 0
        @items_added = 0
      end

      def self.from_params(bit_count:, hash_count:)
        raise ArgumentError, "bit_count must be positive" unless bit_count.is_a?(Integer) && bit_count.positive?
        raise ArgumentError, "hash_count must be positive" unless hash_count.is_a?(Integer) && hash_count.positive?

        filter = allocate
        filter.send(:initialize_from_parts, bit_count, hash_count, 0)
        filter
      end

      def add(element)
        hash_indices(element).each do |idx|
          byte_idx = idx / 8
          bit_mask = 1 << (idx % 8)
          next unless (@bits.fetch(byte_idx) & bit_mask).zero?

          @bits[byte_idx] |= bit_mask
          @bits_set += 1
        end
        @items_added += 1
        nil
      end

      def contains?(element)
        hash_indices(element).all? do |idx|
          byte_idx = idx / 8
          bit_mask = 1 << (idx % 8)
          (@bits.fetch(byte_idx) & bit_mask) != 0
        end
      end

      alias include? contains?

      def fill_ratio
        return 0.0 if @bit_count.zero?

        @bits_set.to_f / @bit_count
      end

      def estimated_false_positive_rate
        return 0.0 if @bits_set.zero?

        fill_ratio**@hash_count
      end

      def over_capacity?
        @expected_items.positive? && @items_added > @expected_items
      end

      def size_bytes
        @bits.length
      end

      def inspect
        pct_set = fill_ratio * 100
        est_fp = estimated_false_positive_rate * 100
        format(
          "BloomFilter(m=%<m>d, k=%<k>d, bits_set=%<set>d/%<total>d (%<pct>.2f%%), ~fp=%<fp>.4f%%)",
          m: @bit_count,
          k: @hash_count,
          set: @bits_set,
          total: @bit_count,
          pct: pct_set,
          fp: est_fp
        )
      end

      alias to_s inspect

      def self.optimal_m(expected_items, false_positive_rate)
        (-expected_items * Math.log(false_positive_rate) / (Math.log(2)**2)).ceil
      end

      def self.optimal_k(bit_count, expected_items)
        [1, ((bit_count.to_f / expected_items) * Math.log(2)).round].max
      end

      def self.capacity_for_memory(memory_bytes, false_positive_rate)
        (-memory_bytes * 8 * (Math.log(2)**2) / Math.log(false_positive_rate)).floor
      end

      private

      def initialize_from_parts(bit_count, hash_count, expected_items)
        @bit_count = bit_count
        @hash_count = hash_count
        @expected_items = expected_items
        @bits = Array.new((bit_count + 7) / 8, 0)
        @bits_set = 0
        @items_added = 0
      end

      def validate_expected_items!(expected_items)
        return if expected_items.is_a?(Integer) && expected_items.positive?

        raise ArgumentError, "expected_items must be a positive integer"
      end

      def validate_false_positive_rate!(false_positive_rate)
        return if false_positive_rate.is_a?(Numeric) && false_positive_rate.positive? && false_positive_rate < 1

        raise ArgumentError, "false_positive_rate must be in the open interval (0, 1)"
      end

      def hash_indices(element)
        bytes = element.to_s.bytes
        h1 = self.class.send(:fmix32, self.class.send(:fnv1a32, bytes))
        h2 = self.class.send(:fmix32, self.class.send(:djb2, bytes)) | 1
        Array.new(@hash_count) { |i| (h1 + (i * h2)) % @bit_count }
      end

      def self.fnv1a32(bytes)
        bytes.reduce(0x811c_9dc5) do |hash, byte|
          ((hash ^ byte) * 0x0100_0193) & MASK32
        end
      end

      def self.djb2(bytes)
        bytes.reduce(5_381) do |hash, byte|
          ((hash * 33) + byte) & MASK32
        end
      end

      def self.fmix32(hash)
        hash ^= hash >> 16
        hash = (hash * 0x85eb_ca6b) & MASK32
        hash ^= hash >> 13
        hash = (hash * 0xc2b2_ae35) & MASK32
        hash ^= hash >> 16
        hash & MASK32
      end
    end
  end
end
