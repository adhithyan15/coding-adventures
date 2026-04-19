# frozen_string_literal: true

# BLAKE2b -- Cryptographic hash function (RFC 7693)
# ==================================================
#
# BLAKE2b is a modern hash faster than MD5 on 64-bit hardware and as secure
# as SHA-3.  It supports variable output length (1..64 bytes), a single-pass
# keyed mode that replaces HMAC-SHA-512, and 16-byte salt and personalization
# parameters.  See +code/specs/HF06-blake2b.md+ for the full walk-through.
#
# Ruby uses arbitrary-precision integers, so every 64-bit add and XOR must be
# masked back down with MASK64 to prevent the result from silently growing
# past 64 bits.  That's the main difference from Rust or Go where +wrapping+
# semantics are native.
#
# Key invariant (classic BLAKE2 off-by-one): the *last real block* is the
# one flagged final.  For messages whose length is an exact multiple of 128
# bytes, we do NOT emit an extra all-zero final block -- the streaming
# hasher flushes only when the buffer *strictly exceeds* the block size,
# leaving at least one byte for the final compression.

require_relative "coding_adventures/blake2b/version"

module CodingAdventures
  module Blake2b
    # BLAKE2b operates on 64-bit words, so every arithmetic step masks with
    # this value to keep Ruby's Bignum from growing past 64 bits.
    MASK64 = 0xFFFFFFFFFFFFFFFF

    BLOCK_SIZE = 128
    MAX_DIGEST = 64
    MAX_KEY = 64

    # Initial Hash Values -- identical to SHA-512 (fractional parts of the
    # square roots of the first eight primes, truncated to 64 bits).  BLAKE2b
    # reuses these "nothing up my sleeve" constants.
    IV = [
      0x6A09E667F3BCC908,
      0xBB67AE8584CAA73B,
      0x3C6EF372FE94F82B,
      0xA54FF53A5F1D36F1,
      0x510E527FADE682D1,
      0x9B05688C2B3E6C1F,
      0x1F83D9ABFB41BD6B,
      0x5BE0CD19137E2179
    ].freeze

    # Ten message-schedule permutations.  Round i uses SIGMA[i % 10];
    # rounds 10 and 11 therefore reuse rows 0 and 1.
    SIGMA = [
      [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
      [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
      [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
      [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
      [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
      [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
      [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
      [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
      [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
      [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0]
    ].freeze

    class << self
      # Rotate a 64-bit value right by +n+ bits.
      def rotr64(x, n)
        ((x >> n) | (x << (64 - n))) & MASK64
      end

      # BLAKE2b quarter-round G.  Rotation constants (R1..R4) = (32, 24, 16, 63).
      def mix(v, a, b, c, d, x, y)
        v[a] = (v[a] + v[b] + x) & MASK64
        v[d] = rotr64(v[d] ^ v[a], 32)
        v[c] = (v[c] + v[d]) & MASK64
        v[b] = rotr64(v[b] ^ v[c], 24)
        v[a] = (v[a] + v[b] + y) & MASK64
        v[d] = rotr64(v[d] ^ v[a], 16)
        v[c] = (v[c] + v[d]) & MASK64
        v[b] = rotr64(v[b] ^ v[c], 63)
      end

      # Parse a 128-byte block as sixteen little-endian 64-bit words.
      def parse_block(block)
        # "Q<" is 64-bit little-endian unsigned; 16 of them == 128 bytes.
        block.unpack("Q<16")
      end

      # Compression function F.  +t+ is the total byte count fed into the hash
      # so far (including the bytes of this block).  +final+ must be true iff
      # this is the last compression call; that triggers the v[14] inversion
      # that domain-separates the final block from intermediate ones.
      def compress(state, block, t, final)
        m = parse_block(block)
        v = state + IV.dup

        # Fold the 128-bit counter into v[12..13].
        v[12] ^= t & MASK64
        v[13] ^= (t >> 64) & MASK64
        v[14] ^= MASK64 if final

        12.times do |i|
          s = SIGMA[i % 10]
          mix(v, 0, 4, 8, 12, m[s[0]], m[s[1]])
          mix(v, 1, 5, 9, 13, m[s[2]], m[s[3]])
          mix(v, 2, 6, 10, 14, m[s[4]], m[s[5]])
          mix(v, 3, 7, 11, 15, m[s[6]], m[s[7]])
          mix(v, 0, 5, 10, 15, m[s[8]], m[s[9]])
          mix(v, 1, 6, 11, 12, m[s[10]], m[s[11]])
          mix(v, 2, 7, 8, 13, m[s[12]], m[s[13]])
          mix(v, 3, 4, 9, 14, m[s[14]], m[s[15]])
        end

        # Davies-Meyer feed-forward: XOR both halves of v into the state.
        8.times { |i| state[i] ^= v[i] ^ v[i + 8] }
        state
      end

      # Build the parameter-block-XOR-ed starting state.  Sequential mode
      # only (fanout=1, depth=1).
      def initial_state(digest_size, key_len, salt, personal)
        p = Array.new(64, 0)
        p[0] = digest_size
        p[1] = key_len
        p[2] = 1 # fanout
        p[3] = 1 # depth
        salt_bytes = salt.bytes
        personal_bytes = personal.bytes
        salt_bytes.each_with_index { |b, i| p[32 + i] = b } unless salt_bytes.empty?
        personal_bytes.each_with_index { |b, i| p[48 + i] = b } unless personal_bytes.empty?

        words = p.pack("C64").unpack("Q<8")
        IV.each_with_index.map { |iv, i| iv ^ words[i] }
      end

      def validate(digest_size, key, salt, personal)
        unless digest_size.is_a?(Integer) && digest_size >= 1 && digest_size <= MAX_DIGEST
          raise ArgumentError, "digest_size must be in [1, 64], got #{digest_size.inspect}"
        end
        if key.bytesize > MAX_KEY
          raise ArgumentError, "key length must be in [0, 64], got #{key.bytesize}"
        end
        if !salt.empty? && salt.bytesize != 16
          raise ArgumentError, "salt must be exactly 16 bytes (or empty), got #{salt.bytesize}"
        end
        if !personal.empty? && personal.bytesize != 16
          raise ArgumentError, "personal must be exactly 16 bytes (or empty), got #{personal.bytesize}"
        end
      end

      # One-shot BLAKE2b.  Returns raw binary string of length +digest_size+.
      def blake2b(data, digest_size: 64, key: "".b, salt: "".b, personal: "".b)
        h = Hasher.new(digest_size: digest_size, key: key, salt: salt, personal: personal)
        h.update(data)
        h.digest
      end

      # One-shot BLAKE2b, lowercase hex.
      def blake2b_hex(data, digest_size: 64, key: "".b, salt: "".b, personal: "".b)
        blake2b(data, digest_size: digest_size, key: key, salt: salt, personal: personal)
          .unpack1("H*")
      end
    end

    # Streaming BLAKE2b hasher.  +digest+ is non-destructive; the same hasher
    # can emit multiple digests and continue to take +update+ calls.
    class Hasher
      def initialize(digest_size: 64, key: "".b, salt: "".b, personal: "".b)
        key = key.b
        salt = salt.b
        personal = personal.b
        Blake2b.validate(digest_size, key, salt, personal)

        @digest_size = digest_size
        @state = Blake2b.initial_state(digest_size, key.bytesize, salt, personal)
        @byte_count = 0

        # Keyed mode: prepend the key zero-padded to one full block.
        @buffer = if key.empty?
          "".b
        else
          (key + ("\x00".b * (BLOCK_SIZE - key.bytesize))).b
        end
      end

      def update(data)
        @buffer << data.b
        # Flush every full block except the latest -- we keep at least one
        # byte in the buffer until digest() so the final compression is
        # flagged final.
        while @buffer.bytesize > BLOCK_SIZE
          @byte_count += BLOCK_SIZE
          block = @buffer.byteslice(0, BLOCK_SIZE)
          Blake2b.compress(@state, block, @byte_count, false)
          @buffer = @buffer.byteslice(BLOCK_SIZE, @buffer.bytesize - BLOCK_SIZE)
        end
        self
      end

      def digest
        state = @state.dup
        padded = @buffer + ("\x00".b * (BLOCK_SIZE - @buffer.bytesize))
        total = @byte_count + @buffer.bytesize
        Blake2b.compress(state, padded, total, true)
        state.pack("Q<8")[0, @digest_size]
      end

      def hex_digest
        digest.unpack1("H*")
      end

      def copy
        clone = Hasher.allocate
        clone.instance_variable_set(:@state, @state.dup)
        clone.instance_variable_set(:@buffer, @buffer.dup)
        clone.instance_variable_set(:@byte_count, @byte_count)
        clone.instance_variable_set(:@digest_size, @digest_size)
        clone
      end
    end
  end
end
