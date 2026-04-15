# frozen_string_literal: true

module CodingAdventures
  module HashMap
    Entry = Struct.new(:key, :value, keyword_init: true)

    class HashMap
      include Enumerable

      DEFAULT_CAPACITY = 16
      DEFAULT_LOAD_FACTOR = 0.75

      attr_reader :size

      def initialize(initial_capacity = DEFAULT_CAPACITY, load_factor: DEFAULT_LOAD_FACTOR, hash_fn: nil)
        raise ArgumentError, "initial_capacity must be positive" if initial_capacity <= 0
        raise ArgumentError, "load_factor must be positive" if load_factor <= 0

        @load_factor = load_factor
        @hash_fn = hash_fn || ->(key) { key.hash }
        @buckets = Array.new(next_power_of_two(initial_capacity)) { [] }
        @size = 0
      end

      def [](key)
        fetch(key, nil)
      end

      def []=(key, value)
        insert(key, value)
      end

      def insert(key, value)
        ensure_capacity_for!(@size + 1)
        bucket = bucket_for(key)

        if (entry = bucket.find { |candidate| candidate.key.eql?(key) })
          entry.value = value
          return value
        end

        bucket << Entry.new(key: key, value: value)
        @size += 1
        value
      end

      def fetch(key, default = nil)
        if (entry = find_entry(key))
          entry.value
        elsif block_given?
          yield(key)
        else
          default
        end
      end

      def delete(key)
        bucket = bucket_for(key)
        index = bucket.index { |entry| entry.key.eql?(key) }
        return nil if index.nil?

        entry = bucket.delete_at(index)
        @size -= 1
        entry.value
      end

      def key?(key)
        !find_entry(key).nil?
      end
      alias include? key?

      def empty?
        @size.zero?
      end

      def clear
        @buckets.each(&:clear)
        @size = 0
        self
      end

      def keys
        each.map { |key, _value| key }
      end

      def values
        each.map { |_key, value| value }
      end

      def to_a
        each.map { |key, value| [key, value] }
      end

      def to_h
        each_with_object({}) do |(key, value), memo|
          memo[key] = value
        end
      end

      def each
        return enum_for(:each) unless block_given?

        @buckets.each do |bucket|
          bucket.each { |entry| yield entry.key, entry.value }
        end
      end

      private

      def bucket_for(key)
        @buckets[index_for(key)]
      end

      def find_entry(key)
        bucket_for(key).find { |entry| entry.key.eql?(key) }
      end

      def index_for(key)
        (@hash_fn.call(key) & 0x7fffffff) % @buckets.length
      end

      def ensure_capacity_for!(desired_size)
        return if desired_size <= (@buckets.length * @load_factor)

        old_entries = to_a
        @buckets = Array.new(@buckets.length * 2) { [] }
        @size = 0
        old_entries.each { |key, value| insert(key, value) }
      end

      def next_power_of_two(value)
        power = 1
        power <<= 1 while power < value
        power
      end
    end
  end
end
