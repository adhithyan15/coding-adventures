# frozen_string_literal: true

require_relative "coding_adventures/hash_functions/version"

module CodingAdventures
  module HashFunctions
    FNV32_OFFSET_BASIS = 0x811C9DC5
    FNV32_PRIME = 0x01000193
    FNV64_OFFSET_BASIS = 0xCBF29CE484222325
    FNV64_PRIME = 0x00000100000001B3
    POLYNOMIAL_ROLLING_DEFAULT_BASE = 31
    POLYNOMIAL_ROLLING_DEFAULT_MODULUS = (1 << 61) - 1

    MASK32 = 0xFFFFFFFF
    MASK64 = 0xFFFFFFFFFFFFFFFF
    MURMUR3_C1 = 0xCC9E2D51
    MURMUR3_C2 = 0x1B873593

    module_function

    def fnv1a32(data)
      hash = FNV32_OFFSET_BASIS
      bytes_for(data).each do |byte|
        hash ^= byte
        hash = (hash * FNV32_PRIME) & MASK32
      end
      hash
    end

    def fnv1a64(data)
      hash = FNV64_OFFSET_BASIS
      bytes_for(data).each do |byte|
        hash ^= byte
        hash = (hash * FNV64_PRIME) & MASK64
      end
      hash
    end

    def djb2(data)
      hash = 5381
      bytes_for(data).each do |byte|
        hash = (((hash << 5) + hash) + byte) & MASK64
      end
      hash
    end

    def polynomial_rolling(
      data,
      base: POLYNOMIAL_ROLLING_DEFAULT_BASE,
      modulus: POLYNOMIAL_ROLLING_DEFAULT_MODULUS
    )
      raise ArgumentError, "modulus must be positive" unless modulus.positive?

      hash = 0
      bytes_for(data).each do |byte|
        hash = (hash * base + byte) % modulus
      end
      hash
    end

    def murmur3_32(data, seed: 0)
      raw = bytes_for(data)
      hash = seed & MASK32
      block_count = raw.length / 4

      block_count.times do |block_index|
        offset = block_index * 4
        k = raw[offset] | (raw[offset + 1] << 8) | (raw[offset + 2] << 16) | (raw[offset + 3] << 24)
        k = (k * MURMUR3_C1) & MASK32
        k = rotl32(k, 15)
        k = (k * MURMUR3_C2) & MASK32

        hash ^= k
        hash = rotl32(hash, 13)
        hash = (hash * 5 + 0xE6546B64) & MASK32
      end

      tail_offset = block_count * 4
      k = 0
      remaining = raw.length & 3
      k ^= raw[tail_offset + 2] << 16 if remaining >= 3
      k ^= raw[tail_offset + 1] << 8 if remaining >= 2
      if remaining >= 1
        k ^= raw[tail_offset]
        k = (k * MURMUR3_C1) & MASK32
        k = rotl32(k, 15)
        k = (k * MURMUR3_C2) & MASK32
        hash ^= k
      end

      fmix32(hash ^ raw.length)
    end

    def avalanche_score(hash_fn, output_bits:, sample_size: 100)
      raise ArgumentError, "output_bits must be in 1..64" unless (1..64).cover?(output_bits)
      raise ArgumentError, "sample_size must be positive" unless sample_size.positive?

      total_bit_flips = 0
      total_trials = 0
      sample_size.times do |sample_index|
        input = deterministic_bytes(sample_index)
        original = hash_fn.call(input)

        (input.length * 8).times do |bit_position|
          flipped = input.dup
          byte_index = bit_position / 8
          flipped.setbyte(byte_index, flipped.getbyte(byte_index) ^ (1 << (bit_position & 7)))
          diff = original ^ hash_fn.call(flipped)
          total_bit_flips += diff.bit_length.downto(0).count { |bit| (diff & (1 << bit)) != 0 }
          total_trials += output_bits
        end
      end

      total_bit_flips.to_f / total_trials
    end

    def distribution_test(hash_fn, inputs, num_buckets:)
      raise ArgumentError, "num_buckets must be positive" unless num_buckets.positive?
      raise ArgumentError, "inputs must not be empty" if inputs.empty?

      counts = Array.new(num_buckets, 0)
      inputs.each do |input|
        counts[hash_fn.call(input) % num_buckets] += 1
      end

      expected = inputs.length.to_f / num_buckets
      counts.sum do |observed|
        delta = observed - expected
        delta * delta / expected
      end
    end

    def bytes_for(data)
      data.b.bytes
    end
    private_class_method :bytes_for

    def rotl32(value, count)
      ((value << count) | (value >> (32 - count))) & MASK32
    end
    private_class_method :rotl32

    def fmix32(hash)
      mixed = hash & MASK32
      mixed ^= mixed >> 16
      mixed = (mixed * 0x85EBCA6B) & MASK32
      mixed ^= mixed >> 13
      mixed = (mixed * 0xC2B2AE35) & MASK32
      mixed ^= mixed >> 16
      mixed & MASK32
    end
    private_class_method :fmix32

    def deterministic_bytes(sample_index)
      state = 0x9E3779B9 ^ sample_index
      bytes = +""
      8.times do
        state = (state * 1_664_525 + 1_013_904_223) & MASK32
        bytes << (state & 0xFF)
      end
      bytes
    end
    private_class_method :deterministic_bytes
  end
end
