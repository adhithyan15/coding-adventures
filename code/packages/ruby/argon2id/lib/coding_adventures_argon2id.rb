# frozen_string_literal: true

# coding_adventures_argon2id -- Argon2id (RFC 9106) -- the RFC-recommended
# memory-hard password hashing function.
#
# == What Is Argon2id?
#
# Argon2id is the "hybrid" member of the Argon2 family. In the first
# two of the four slices of the first pass it uses Argon2i
# (data-independent) addressing, and everything afterwards uses
# Argon2d (data-dependent) addressing. This gives you side-channel
# resistance for the initial memory-fill phase -- when the attacker
# has the best chance of observing a timing leak -- together with
# Argon2d's GPU/ASIC resistance for the bulk of the work.
#
# Pick this variant unless you have a specific reason to prefer
# {CodingAdventures::Argon2d} (proof-of-work, no side-channel threat)
# or {CodingAdventures::Argon2i} (strict side-channel requirements).
#
# Reference: https://datatracker.ietf.org/doc/html/rfc9106
# See also: code/specs/KD03-argon2.md

require "coding_adventures_blake2b"
require_relative "coding_adventures/argon2id/version"

module CodingAdventures
  # Argon2id -- hybrid memory-hard KDF (RFC 9106, recommended).
  module Argon2id
    MASK32 = 0xFFFFFFFF
    MASK64 = 0xFFFFFFFFFFFFFFFF

    BLOCK_SIZE = 1024
    BLOCK_WORDS = BLOCK_SIZE / 8
    SYNC_POINTS = 4
    ADDRESSES_PER_BLOCK = BLOCK_WORDS

    ARGON2_VERSION = 0x13
    TYPE_ID = 2

    class << self
      def rotr64(x, n)
        ((x >> n) | (x << (64 - n))) & MASK64
      end

      def g_b(v, a, b, c, d)
        va = v[a]; vb = v[b]; vc = v[c]; vd = v[d]
        va = (va + vb + 2 * (va & MASK32) * (vb & MASK32)) & MASK64
        vd = rotr64(vd ^ va, 32)
        vc = (vc + vd + 2 * (vc & MASK32) * (vd & MASK32)) & MASK64
        vb = rotr64(vb ^ vc, 24)
        va = (va + vb + 2 * (va & MASK32) * (vb & MASK32)) & MASK64
        vd = rotr64(vd ^ va, 16)
        vc = (vc + vd + 2 * (vc & MASK32) * (vd & MASK32)) & MASK64
        vb = rotr64(vb ^ vc, 63)
        v[a] = va; v[b] = vb; v[c] = vc; v[d] = vd
      end

      def permutation_p(v, off = 0)
        g_b(v, off + 0, off + 4, off + 8, off + 12)
        g_b(v, off + 1, off + 5, off + 9, off + 13)
        g_b(v, off + 2, off + 6, off + 10, off + 14)
        g_b(v, off + 3, off + 7, off + 11, off + 15)
        g_b(v, off + 0, off + 5, off + 10, off + 15)
        g_b(v, off + 1, off + 6, off + 11, off + 12)
        g_b(v, off + 2, off + 7, off + 8, off + 13)
        g_b(v, off + 3, off + 4, off + 9, off + 14)
      end

      def compress(x, y)
        r = Array.new(BLOCK_WORDS) { |i| x[i] ^ y[i] }
        q = r.dup
        8.times { |i| permutation_p(q, i * 16) }
        col = Array.new(16)
        8.times do |c|
          8.times do |rr|
            col[2 * rr] = q[rr * 16 + 2 * c]
            col[2 * rr + 1] = q[rr * 16 + 2 * c + 1]
          end
          permutation_p(col, 0)
          8.times do |rr|
            q[rr * 16 + 2 * c] = col[2 * rr]
            q[rr * 16 + 2 * c + 1] = col[2 * rr + 1]
          end
        end
        Array.new(BLOCK_WORDS) { |i| r[i] ^ q[i] }
      end

      def block_to_bytes(block)
        block.pack("Q<#{BLOCK_WORDS}")
      end

      def bytes_to_block(data)
        data.unpack("Q<#{BLOCK_WORDS}")
      end

      def le32(n)
        [n].pack("V")
      end

      def blake2b_long(t, x)
        raise ArgumentError, "H' output length must be positive" if t <= 0

        input = le32(t) + x
        return CodingAdventures::Blake2b.blake2b(input, digest_size: t) if t <= 64

        r = (t + 31) / 32 - 2
        v = CodingAdventures::Blake2b.blake2b(input, digest_size: 64)
        out = v[0, 32].b
        (1...r).each do
          v = CodingAdventures::Blake2b.blake2b(v, digest_size: 64)
          out << v[0, 32]
        end
        final_size = t - 32 * r
        v = CodingAdventures::Blake2b.blake2b(v, digest_size: final_size)
        out << v
        out
      end

      def index_alpha(j1, r, sl, c, same_lane, q, sl_len)
        if r == 0 && sl == 0
          w = c - 1
          start = 0
        elsif r == 0
          w = if same_lane
            sl * sl_len + c - 1
          elsif c == 0
            sl * sl_len - 1
          else
            sl * sl_len
          end
          start = 0
        else
          w = if same_lane
            q - sl_len + c - 1
          elsif c == 0
            q - sl_len - 1
          else
            q - sl_len
          end
          start = ((sl + 1) * sl_len) % q
        end

        x = (j1 * j1) >> 32
        y = (w * x) >> 32
        rel = w - 1 - y
        (start + rel) % q
      end

      # Argon2id fill: data-independent in the first two slices of the
      # first pass, data-dependent thereafter.
      def fill_segment(memory, r, lane, sl, q, sl_len, p, m_prime, t_total)
        data_independent = r == 0 && sl < 2

        input_block = Array.new(BLOCK_WORDS, 0)
        address_block = Array.new(BLOCK_WORDS, 0)
        zero_block = Array.new(BLOCK_WORDS, 0)
        if data_independent
          input_block[0] = r
          input_block[1] = lane
          input_block[2] = sl
          input_block[3] = m_prime
          input_block[4] = t_total
          input_block[5] = TYPE_ID
        end

        next_addresses = lambda do
          input_block[6] += 1
          z = compress(zero_block, input_block)
          address_block.replace(compress(zero_block, z))
        end

        starting_c = (r == 0 && sl == 0) ? 2 : 0
        next_addresses.call if data_independent && starting_c != 0

        (starting_c...sl_len).each do |i|
          if data_independent && i % ADDRESSES_PER_BLOCK == 0 && !(r == 0 && sl == 0 && i == 2)
            next_addresses.call
          end

          col = sl * sl_len + i
          prev_col = col == 0 ? q - 1 : col - 1
          prev_block = memory[lane][prev_col]

          pseudo_rand = data_independent ? address_block[i % ADDRESSES_PER_BLOCK] : prev_block[0]
          j1 = pseudo_rand & MASK32
          j2 = (pseudo_rand >> 32) & MASK32

          l_prime = lane
          l_prime = j2 % p unless r == 0 && sl == 0
          z_prime = index_alpha(j1, r, sl, i, l_prime == lane, q, sl_len)
          ref_block = memory[l_prime][z_prime]

          new_block = compress(prev_block, ref_block)
          if r == 0
            memory[lane][col] = new_block
          else
            existing = memory[lane][col]
            memory[lane][col] = Array.new(BLOCK_WORDS) { |k| existing[k] ^ new_block[k] }
          end
        end
      end

      def validate(password, salt, t, m, p, tag_length, key, ad, version)
        raise ArgumentError, "password length must fit in 32 bits" if password.bytesize > MASK32
        raise ArgumentError, "salt must be at least 8 bytes" if salt.bytesize < 8
        raise ArgumentError, "salt length must fit in 32 bits" if salt.bytesize > MASK32
        raise ArgumentError, "key length must fit in 32 bits" if key.bytesize > MASK32
        raise ArgumentError, "associated_data length must fit in 32 bits" if ad.bytesize > MASK32
        raise ArgumentError, "tag_length must be >= 4" if tag_length < 4
        raise ArgumentError, "tag_length must fit in 32 bits" if tag_length > MASK32
        unless p.is_a?(Integer) && p >= 1 && p <= 0xFFFFFF
          raise ArgumentError, "parallelism must be in [1, 2^24-1]"
        end
        raise ArgumentError, "memory_cost must be >= 8*parallelism" if m < 8 * p
        raise ArgumentError, "memory_cost must fit in 32 bits" if m > MASK32
        raise ArgumentError, "time_cost must be >= 1" if t < 1
        raise ArgumentError, "only Argon2 v1.3 (0x13) is supported" unless version == ARGON2_VERSION
      end

      # argon2id -- compute the Argon2id tag (RFC 9106 §3).
      #
      # @param password    [String]  secret input
      # @param salt        [String]  >= 8 bytes, 16+ recommended
      # @param time_cost   [Integer] passes (t), >= 1
      # @param memory_cost [Integer] KiB (m), >= 8*parallelism
      # @param parallelism [Integer] lanes (p)
      # @param tag_length  [Integer] output bytes (T), >= 4
      # @return [String]  +tag_length+ bytes, binary encoding
      def argon2id(password, salt, time_cost, memory_cost, parallelism, tag_length,
                   key: "".b, associated_data: "".b, version: ARGON2_VERSION)
        password = password.b
        salt = salt.b
        key = key.b
        ad = associated_data.b
        validate(password, salt, time_cost, memory_cost, parallelism, tag_length, key, ad, version)

        segment_length = memory_cost / (SYNC_POINTS * parallelism)
        m_prime = segment_length * SYNC_POINTS * parallelism
        q = m_prime / parallelism
        sl_len = segment_length
        p = parallelism
        t = time_cost

        h0_in = le32(p) + le32(tag_length) + le32(memory_cost) + le32(t) +
                le32(version) + le32(TYPE_ID) +
                le32(password.bytesize) + password +
                le32(salt.bytesize) + salt +
                le32(key.bytesize) + key +
                le32(ad.bytesize) + ad
        h0 = CodingAdventures::Blake2b.blake2b(h0_in, digest_size: 64)

        memory = Array.new(p) { Array.new(q) { nil } }
        (0...p).each do |i|
          b0 = blake2b_long(BLOCK_SIZE, h0 + le32(0) + le32(i))
          b1 = blake2b_long(BLOCK_SIZE, h0 + le32(1) + le32(i))
          memory[i][0] = bytes_to_block(b0)
          memory[i][1] = bytes_to_block(b1)
        end

        t.times do |r|
          SYNC_POINTS.times do |sl|
            (0...p).each do |lane|
              fill_segment(memory, r, lane, sl, q, sl_len, p, m_prime, t)
            end
          end
        end

        final_block = memory[0][q - 1].dup
        (1...p).each do |lane|
          BLOCK_WORDS.times { |k| final_block[k] ^= memory[lane][q - 1][k] }
        end

        blake2b_long(tag_length, block_to_bytes(final_block))
      end

      def argon2id_hex(password, salt, time_cost, memory_cost, parallelism, tag_length,
                      key: "".b, associated_data: "".b, version: ARGON2_VERSION)
        argon2id(password, salt, time_cost, memory_cost, parallelism, tag_length,
                 key: key, associated_data: associated_data, version: version).unpack1("H*")
      end
    end
  end
end
