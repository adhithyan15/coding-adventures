# frozen_string_literal: true

# ca_sha256 -- SHA-256 cryptographic hash function (FIPS 180-4).
#
# What Is SHA-256?
# ================
# SHA-256 (Secure Hash Algorithm 256) belongs to the SHA-2 family, designed by
# the NSA and published by NIST in 2001 (FIPS 180-2, updated FIPS 180-4). It
# produces a 256-bit (32-byte) digest and is the workhorse of modern cryptography
# -- TLS, Bitcoin, git, code signing, and password hashing all depend on SHA-256.
#
# Unlike MD5 (broken 2004) and SHA-1 (broken 2017), SHA-256 remains secure with
# no known practical collision or preimage attacks. The birthday bound is 2^128.
#
# How It Differs from SHA-1
# =========================
# SHA-256 shares the same Merkle-Damgard construction as SHA-1 but with:
#   - 8 state words (not 5), each 32 bits wide
#   - 64 rounds (not 80) per block
#   - 64 round constants K[0..63] from cube roots of first 64 primes
#   - More complex message schedule with two "small sigma" functions
#   - Two "big Sigma" functions and uniform Ch/Maj usage every round
#
# The Big Picture: Merkle-Damgard Construction
# ============================================
#
#   message --> [pad] --> block0 --> block1 --> ... --> 32-byte digest
#                             |           |
#                     [H0..H7]-->compress-->compress-->...
#
# Ruby Integer Note
# =================
# Ruby integers have arbitrary precision, so they never overflow. We must mask
# to 32 bits after every addition or bitwise NOT:
#   (a + b) & 0xFFFFFFFF
#   (~x) & 0xFFFFFFFF
#
# FIPS 180-4 Test Vectors
# =======================
#   sha256("")    -> "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
#   sha256("abc") -> "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"

require_relative "coding_adventures/sha256/version"

module CodingAdventures
  module Sha256
    # 32-bit mask. Ruby integers are arbitrary precision; we mask to simulate
    # 32-bit unsigned arithmetic.
    MASK = 0xFFFFFFFF

    # === Initialization Constants ============================================
    #
    # Eight 32-bit words from the FRACTIONAL parts of the square roots of the
    # first 8 primes (2, 3, 5, 7, 11, 13, 17, 19).
    #
    # Example: sqrt(2) = 1.4142... -> frac = 0.4142... -> * 2^32 -> 0x6A09E667
    INIT = [
      0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A,
      0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19
    ].freeze

    # 64 round constants from cube roots of the first 64 primes.
    K = [
      0x428A2F98, 0x71374491, 0xB5C0FBCF, 0xE9B5DBA5,
      0x3956C25B, 0x59F111F1, 0x923F82A4, 0xAB1C5ED5,
      0xD807AA98, 0x12835B01, 0x243185BE, 0x550C7DC3,
      0x72BE5D74, 0x80DEB1FE, 0x9BDC06A7, 0xC19BF174,
      0xE49B69C1, 0xEFBE4786, 0x0FC19DC6, 0x240CA1CC,
      0x2DE92C6F, 0x4A7484AA, 0x5CB0A9DC, 0x76F988DA,
      0x983E5152, 0xA831C66D, 0xB00327C8, 0xBF597FC7,
      0xC6E00BF3, 0xD5A79147, 0x06CA6351, 0x14292967,
      0x27B70A85, 0x2E1B2138, 0x4D2C6DFC, 0x53380D13,
      0x650A7354, 0x766A0ABB, 0x81C2C92E, 0x92722C85,
      0xA2BFE8A1, 0xA81A664B, 0xC24B8B70, 0xC76C51A3,
      0xD192E819, 0xD6990624, 0xF40E3585, 0x106AA070,
      0x19A4C116, 0x1E376C08, 0x2748774C, 0x34B0BCB5,
      0x391C0CB3, 0x4ED8AA4A, 0x5B9CCA4F, 0x682E6FF3,
      0x748F82EE, 0x78A5636F, 0x84C87814, 0x8CC70208,
      0x90BEFFFA, 0xA4506CEB, 0xBEF9A3F7, 0xC67178F2
    ].freeze

    # === Bit Manipulation Helpers ============================================

    # Circular right rotation of a 32-bit word by n positions.
    # Bits that fall off the right wrap to the left.
    #
    # Example: n=3, x=0b11010010 (8-bit)
    #   Shift:  11010010 >> 3  = 00011010  (110 lost)
    #   Rotate: 11010010 ROTR3 = 01011010  (010 wraps left)
    def self.rotr(n, x)
      ((x >> n) | (x << (32 - n))) & MASK
    end

    # Choice function: if x=1 choose y, else choose z (per bit position).
    #
    # Truth table:
    #   x=0 -> output = z
    #   x=1 -> output = y
    #
    # Think of it as a 1-bit multiplexer.
    def self.ch(x, y, z)
      ((x & y) ^ (~x & z)) & MASK
    end

    # Majority function: output is 1 if at least 2 of 3 inputs are 1.
    def self.maj(x, y, z)
      ((x & y) ^ (x & z) ^ (y & z)) & MASK
    end

    # Big Sigma 0: used on working variable 'a'.
    # Sigma0(x) = ROTR(2,x) XOR ROTR(13,x) XOR ROTR(22,x)
    def self.big_sigma0(x)
      rotr(2, x) ^ rotr(13, x) ^ rotr(22, x)
    end

    # Big Sigma 1: used on working variable 'e'.
    # Sigma1(x) = ROTR(6,x) XOR ROTR(11,x) XOR ROTR(25,x)
    def self.big_sigma1(x)
      rotr(6, x) ^ rotr(11, x) ^ rotr(25, x)
    end

    # Small sigma 0: used in message schedule expansion.
    # sigma0(x) = ROTR(7,x) XOR ROTR(18,x) XOR SHR(3,x)
    #
    # Note the SHR (shift, not rotate) -- it destroys information intentionally,
    # making the schedule a one-way function.
    def self.small_sigma0(x)
      rotr(7, x) ^ rotr(18, x) ^ (x >> 3)
    end

    # Small sigma 1: used in message schedule expansion.
    # sigma1(x) = ROTR(17,x) XOR ROTR(19,x) XOR SHR(10,x)
    def self.small_sigma1(x)
      rotr(17, x) ^ rotr(19, x) ^ (x >> 10)
    end

    # === Padding ==============================================================
    #
    # Extends the message to a multiple of 64 bytes:
    #   1. Append 0x80
    #   2. Append zeros until length == 56 mod 64
    #   3. Append original bit length as 64-bit big-endian
    #
    # "N2" packs two 32-bit big-endian integers (high word, low word) to form
    # a 64-bit big-endian length.
    def self.pad(data)
      bit_len = data.bytesize * 8
      msg = data.dup.b
      msg << "\x80".b
      msg << "\x00".b while msg.bytesize % 64 != 56
      msg << [bit_len >> 32, bit_len & MASK].pack("N2")
      msg
    end

    # === Message Schedule =====================================================
    #
    # Each 64-byte block -> 64-word schedule.
    # First 16 words: parsed from block (big-endian uint32).
    # Words 16..63: W[t] = sigma1(W[t-2]) + W[t-7] + sigma0(W[t-15]) + W[t-16]
    #
    # "N16" unpacks 16 big-endian 32-bit unsigned integers.
    def self.schedule(block)
      w = block.unpack("N16")
      (16...64).each do |i|
        w << ((small_sigma1(w[i - 2]) + w[i - 7] +
               small_sigma0(w[i - 15]) + w[i - 16]) & MASK)
      end
      w
    end

    # === Compression Function =================================================
    #
    # 64 rounds of mixing fold one 64-byte block into the eight-word state.
    #
    # Each round:
    #   T1 = h + Sigma1(e) + Ch(e,f,g) + K[t] + W[t]
    #   T2 = Sigma0(a) + Maj(a,b,c)
    #   shift variables, add T1 and T2 into the pipeline
    #
    # Davies-Meyer feed-forward: add compressed result back to input state.
    def self.compress(state, block)
      w = schedule(block)
      h0, h1, h2, h3, h4, h5, h6, h7 = state
      a, b, c, d, e, f, g, h = h0, h1, h2, h3, h4, h5, h6, h7

      64.times do |t|
        t1 = (h + big_sigma1(e) + ch(e, f, g) + K[t] + w[t]) & MASK
        t2 = (big_sigma0(a) + maj(a, b, c)) & MASK
        h = g
        g = f
        f = e
        e = (d + t1) & MASK
        d = c
        c = b
        b = a
        a = (t1 + t2) & MASK
      end

      [
        (h0 + a) & MASK, (h1 + b) & MASK,
        (h2 + c) & MASK, (h3 + d) & MASK,
        (h4 + e) & MASK, (h5 + f) & MASK,
        (h6 + g) & MASK, (h7 + h) & MASK
      ]
    end

    # === Public API ===========================================================

    # Compute SHA-256 digest. Returns a 32-byte binary String.
    #
    #   CodingAdventures::Sha256.sha256("abc").unpack1("H*")
    #   # -> "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    #
    # "N8" packs eight 32-bit big-endian unsigned integers into 32 bytes.
    def self.sha256(data)
      padded = pad(data.b)
      state = INIT.dup
      0.step(padded.bytesize - 1, 64) do |i|
        state = compress(state, padded[i, 64])
      end
      state.pack("N8")
    end

    # Compute SHA-256 and return the 64-character lowercase hex string.
    def self.sha256_hex(data)
      sha256(data).unpack1("H*")
    end

    # Streaming SHA-256 hasher that accepts data in multiple chunks.
    #
    #   h = CodingAdventures::Sha256::Digest.new
    #   h.update("ab")
    #   h.update("c")
    #   h.hexdigest  # -> "ba7816bf..."
    class Digest
      def initialize
        @state = INIT.dup
        @buffer = "".b
        @byte_count = 0
      end

      # Feed more bytes into the hash. Returns self for chaining.
      def update(data)
        data = data.b
        @buffer << data
        @byte_count += data.bytesize
        while @buffer.bytesize >= 64
          @state = Sha256.compress(@state, @buffer[0, 64])
          @buffer = @buffer[64..]
        end
        self
      end
      alias << update

      # Return the 32-byte binary digest of all data fed so far.
      #
      # Non-destructive: calling digest twice returns the same bytes.
      def digest
        bit_len = @byte_count * 8
        tail = @buffer.dup
        tail << "\x80".b
        tail << "\x00".b while tail.bytesize % 64 != 56
        tail << [bit_len >> 32, bit_len & MASK].pack("N2")

        state = @state.dup
        0.step(tail.bytesize - 1, 64) do |i|
          state = Sha256.compress(state, tail[i, 64])
        end
        state.pack("N8")
      end

      # Return the 64-character hex string of the digest.
      def hexdigest
        digest.unpack1("H*")
      end

      # Return an independent deep copy of the current hasher state.
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
