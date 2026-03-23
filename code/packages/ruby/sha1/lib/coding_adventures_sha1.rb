# frozen_string_literal: true

# ca_sha1 — SHA-1 cryptographic hash function (FIPS 180-4).
#
# What Is SHA-1?
# ==============
# SHA-1 (Secure Hash Algorithm 1) takes any sequence of bytes and produces a
# fixed-size 20-byte (160-bit) "fingerprint" called a digest. The same input
# always produces the same digest. Change even one bit of input and the digest
# changes completely — the "avalanche effect". You cannot reverse a digest back
# to the original input.
#
# We implement SHA-1 from scratch rather than using Ruby's built-in Digest::SHA1
# so that every step of the algorithm is visible and explained.
#
# The Big Picture: Merkle-Damgård Construction
# =============================================
# SHA-1 processes data in 512-bit (64-byte) blocks:
#
#   message ──► [pad] ──► block₀ ──► block₁ ──► ... ──► 20-byte digest
#                              │           │
#                      [H₀..H₄]──►compress──►compress──►...
#
# The "state" is five 32-bit words (H₀..H₄), initialized to fixed constants.
# For each block, 80 rounds of bit mixing fold the block into the state.
# The final state is the digest.
#
# Analogy: a blender. Start with a base liquid (the initial constants). Add
# ingredients one chunk at a time (message blocks). Each blend mixes the new
# ingredient with everything before it. You cannot un-blend.
#
# Ruby Integer Note
# =================
# Ruby integers have arbitrary precision, so they never overflow. This means:
#   - We must mask to 32 bits after every addition:  (a + b) & 0xFFFFFFFF
#   - Bitwise NOT (~x) produces a negative integer in Ruby; mask it too:
#     (~b & d) & 0xFFFFFFFF
#
# FIPS 180-4 Test Vectors
# =======================
#   sha1("")    → "da39a3ee5e6b4b0d3255bfef95601890afd80709"
#   sha1("abc") → "a9993e364706816aba3e25717850c26c9cd0d89d"

require_relative "coding_adventures/sha1/version"

module CodingAdventures
  module Sha1
    # ─── Initialization Constants ──────────────────────────────────────────────
    #
    # SHA-1 starts with these five 32-bit words as its initial state. They are
    # "nothing up my sleeve" numbers — their obvious counting-sequence structure
    # (01234567, 89ABCDEF, … reversed in byte pairs) proves no backdoor is hidden.
    #
    #   H₀ = 0x67452301 → bytes 67 45 23 01 → reverse: 01 23 45 67
    #   H₁ = 0xEFCDAB89 → bytes EF CD AB 89 → reverse: 89 AB CD EF
    INIT = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0].freeze

    # Round constants — one per 20-round stage, derived from square roots.
    #   K₀ = floor(sqrt(2)  × 2^30) = 0x5A827999
    #   K₁ = floor(sqrt(3)  × 2^30) = 0x6ED9EBA1
    #   K₂ = floor(sqrt(5)  × 2^30) = 0x8F1BBCDC
    #   K₃ = floor(sqrt(10) × 2^30) = 0xCA62C1D6
    K = [0x5A827999, 0x6ED9EBA1, 0x8F1BBCDC, 0xCA62C1D6].freeze

    # ─── Helper: Circular Left Shift ─────────────────────────────────────────
    #
    # rotl(n, x) rotates x left by n bit positions within a 32-bit word.
    # Bits that "fall off" the left end reappear on the right.
    #
    # Example: n=2, x=0b01101001 (8-bit for clarity)
    #   Regular:  01101001 << 2 = 10100100  (01 on the left is lost)
    #   Circular: 01101001 ROTL 2 = 10100110  (01 wraps around to the right)
    #
    # Implementation: left half (x << n) OR right half (x >> (32-n)),
    # masked to 32 bits because Ruby integers don't overflow.
    def self.rotl(n, x)
      ((x << n) | (x >> (32 - n))) & 0xFFFFFFFF
    end

    # ─── Padding ──────────────────────────────────────────────────────────────
    #
    # The compression function needs exactly 64-byte (512-bit) blocks. Padding
    # extends the message per FIPS 180-4 §5.1.1:
    #
    #   1. Append 0x80 (the '1' bit followed by seven '0' bits).
    #   2. Append 0x00 bytes until length ≡ 56 (mod 64).
    #   3. Append the original bit length as a 64-bit big-endian integer.
    #
    # Example — "abc" (3 bytes = 24 bits):
    #   61 62 63 80 [52 zero bytes] 00 00 00 00 00 00 00 18
    #                                                   ^^ 24 in hex
    #
    # Why 56 mod 64? We need 8 bytes for the length, and 56 + 8 = 64 (one block).
    # String#pack with ">N" writes a 32-bit big-endian integer.
    # "Q>" is the pack directive for a 64-bit big-endian integer.
    def self.pad(data)
      bit_len = data.bytesize * 8
      msg = data.dup.b # work in binary encoding
      msg << "\x80".b
      msg << "\x00".b while msg.bytesize % 64 != 56
      msg << [bit_len >> 32, bit_len & 0xFFFFFFFF].pack("N2") # big-endian 64-bit
      msg
    end

    # ─── Message Schedule ──────────────────────────────────────────────────────
    #
    # Each 64-byte block is parsed as 16 big-endian 32-bit words (W[0..15]),
    # then expanded to 80 words:
    #
    #   W[i] = ROTL(1, W[i-3] XOR W[i-8] XOR W[i-14] XOR W[i-16])  for i ≥ 16
    #
    # Why expand to 80? More words → more mixing → better avalanche. A single
    # bit flip in the input propagates through all 80 words via the XORs.
    #
    # "N16" unpacks 16 32-bit big-endian unsigned integers.
    def self.schedule(block)
      w = block.unpack("N16") # 16 big-endian uint32 words
      (16...80).each do |i|
        w << rotl(1, w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16])
      end
      w
    end

    # ─── Compression Function ─────────────────────────────────────────────────
    #
    # 80 rounds of mixing fold one 64-byte block into the five-word state.
    #
    # Four stages of 20 rounds each, each using a different auxiliary function:
    #
    #   Stage  Rounds  f(b,c,d)                    Purpose
    #   ─────  ──────  ──────────────────────────  ─────────────────
    #     1    0–19    (b & c) | (~b & d)          Selector / mux
    #     2    20–39   b ^ c ^ d                   Parity
    #     3    40–59   (b&c) | (b&d) | (c&d)       Majority vote
    #     4    60–79   b ^ c ^ d                   Parity again
    #
    # Each round:
    #   temp = ROTL(5, a) + f(b,c,d) + e + K + W[t]  (mod 2^32)
    #   shift: e=d, d=c, c=ROTL(30,b), b=a, a=temp
    #
    # Davies-Meyer feed-forward: add the compressed output back to the input
    # state. This makes the function non-invertible — even if you reversed all
    # 80 rounds, you'd need to subtract the input state you don't have.
    def self.compress(state, block)
      w = schedule(block)
      h0, h1, h2, h3, h4 = state
      a, b, c, d, e = h0, h1, h2, h3, h4

      80.times do |t|
        f, k = if t < 20
          # Selector: if b=1 output c, if b=0 output d
          [((b & c) | (~b & d)) & 0xFFFFFFFF, K[0]]
        elsif t < 40
          # Parity: 1 if an odd number of inputs are 1
          [(b ^ c ^ d) & 0xFFFFFFFF, K[1]]
        elsif t < 60
          # Majority: 1 if at least 2 of the 3 inputs are 1
          [((b & c) | (b & d) | (c & d)) & 0xFFFFFFFF, K[2]]
        else
          # Parity again (same formula, different constant)
          [(b ^ c ^ d) & 0xFFFFFFFF, K[3]]
        end

        temp = (rotl(5, a) + f + e + k + w[t]) & 0xFFFFFFFF
        e = d
        d = c
        c = rotl(30, b)
        b = a
        a = temp
      end

      [
        (h0 + a) & 0xFFFFFFFF,
        (h1 + b) & 0xFFFFFFFF,
        (h2 + c) & 0xFFFFFFFF,
        (h3 + d) & 0xFFFFFFFF,
        (h4 + e) & 0xFFFFFFFF,
      ]
    end

    # ─── Public API ───────────────────────────────────────────────────────────

    # Compute the SHA-1 digest of data. Returns a 20-byte binary String.
    #
    # This is the one-shot API: hash a complete message in a single call.
    #
    #   CodingAdventures::Sha1.sha1("abc").unpack1("H*")
    #   # → "a9993e364706816aba3e25717850c26c9cd0d89d"
    #
    # "5N" packs five 32-bit big-endian unsigned integers into 20 bytes.
    def self.sha1(data)
      padded = pad(data.b)
      state = INIT.dup
      0.step(padded.bytesize - 1, 64) do |i|
        state = compress(state, padded[i, 64])
      end
      state.pack("N5") # finalize: 5 × 32-bit big-endian = 20 bytes
    end

    # Compute SHA-1 and return the 40-character lowercase hex string.
    #
    #   CodingAdventures::Sha1.sha1_hex("abc")
    #   # → "a9993e364706816aba3e25717850c26c9cd0d89d"
    def self.sha1_hex(data)
      sha1(data).unpack1("H*")
    end

    # Streaming SHA-1 hasher that accepts data in multiple chunks.
    #
    # Useful when the full message is not available at once — for example when
    # reading a large file in chunks or hashing a network stream.
    #
    #   h = CodingAdventures::Sha1::Digest.new
    #   h.update("ab")
    #   h.update("c")
    #   h.hexdigest  # → "a9993e364706816aba3e25717850c26c9cd0d89d"
    #
    # Multiple update() calls are equivalent to sha1(all_data).
    class Digest
      def initialize
        @state = INIT.dup
        @buffer = "".b # binary string
        @byte_count = 0
      end

      # Feed more bytes into the hash. Returns self for chaining.
      def update(data)
        data = data.b
        @buffer << data
        @byte_count += data.bytesize
        # Compress any complete 64-byte blocks to keep the buffer small
        while @buffer.bytesize >= 64
          @state = Sha1.compress(@state, @buffer[0, 64])
          @buffer = @buffer[64..]
        end
        self
      end
      alias << update

      # Return the 20-byte binary digest of all data fed so far.
      #
      # Non-destructive: calling digest twice returns the same bytes, and you
      # can continue calling update after digest.
      def digest
        # Pad the remaining buffer using the TOTAL byte count (not buffer length)
        bit_len = @byte_count * 8
        tail = @buffer.dup
        tail << "\x80".b
        tail << "\x00".b while tail.bytesize % 64 != 56
        tail << [bit_len >> 32, bit_len & 0xFFFFFFFF].pack("N2")

        # Compress padding tail against a copy of the live state
        state = @state.dup
        0.step(tail.bytesize - 1, 64) do |i|
          state = Sha1.compress(state, tail[i, 64])
        end
        state.pack("N5")
      end

      # Return the 40-character hex string of the digest.
      def hexdigest
        digest.unpack1("H*")
      end

      # Return an independent copy of the current hasher state.
      #
      # Useful for computing multiple digests that share a common prefix:
      #   h = Digest.new.update(common_prefix)
      #   h1 = h.copy.update("suffix_a")
      #   h2 = h.copy.update("suffix_b")
      def copy
        other = Digest.new
        other.instance_variable_set(:@state, @state.dup)
        other.instance_variable_set(:@buffer, @buffer.dup)
        other.instance_variable_set(:@byte_count, @byte_count)
        other
      end
    end
  end
end
