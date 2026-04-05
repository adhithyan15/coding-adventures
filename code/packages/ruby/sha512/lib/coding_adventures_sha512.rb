# frozen_string_literal: true

# ca_sha512 -- SHA-512 cryptographic hash function (FIPS 180-4).
#
# What Is SHA-512?
# ================
# SHA-512 is the 64-bit sibling of SHA-256 in the SHA-2 family. It takes any
# sequence of bytes and produces a fixed-size 64-byte (512-bit) digest. The
# same input always gives the same digest. Change one bit and the entire
# digest changes -- the "avalanche effect".
#
# On 64-bit hardware SHA-512 is often faster than SHA-256 because it processes
# data in 128-byte blocks using native 64-bit arithmetic.
#
# How It Differs From SHA-256
# ===========================
#   Property          SHA-256      SHA-512
#   Word size         32-bit       64-bit
#   State words       8 x 32-bit   8 x 64-bit
#   Block size        64 bytes     128 bytes
#   Rounds            64           80
#   Round constants   64 (32-bit)  80 (64-bit)
#   Length field       64-bit       128-bit
#   Digest size       32 bytes     64 bytes
#
# Ruby Integer Note
# =================
# Ruby integers have arbitrary precision, so they never overflow. This means
# we must mask to 64 bits after every addition: (a + b) & MASK64
# Bitwise NOT (~x) also produces a negative integer; mask it too.
#
# FIPS 180-4 Test Vectors
# =======================
#   sha512("")    -> "cf83e1357eefb8bd...f927da3e"
#   sha512("abc") -> "ddaf35a193617aba...a54ca49f"

require_relative "coding_adventures/sha512/version"

module CodingAdventures
  module Sha512
    # ---- Mask for 64-bit arithmetic ----
    MASK64 = 0xFFFFFFFFFFFFFFFF

    # ---- Initial Hash Values (FIPS 180-4 section 5.3.5) ----
    #
    # Fractional parts of the square roots of the first eight primes
    # (2, 3, 5, 7, 11, 13, 17, 19), truncated to 64 bits.
    # "Nothing up my sleeve" numbers.
    INIT = [
      0x6A09E667F3BCC908, # frac(sqrt(2))
      0xBB67AE8584CAA73B, # frac(sqrt(3))
      0x3C6EF372FE94F82B, # frac(sqrt(5))
      0xA54FF53A5F1D36F1, # frac(sqrt(7))
      0x510E527FADE682D1, # frac(sqrt(11))
      0x9B05688C2B3E6C1F, # frac(sqrt(13))
      0x1F83D9ABFB41BD6B, # frac(sqrt(17))
      0x5BE0CD19137E2179  # frac(sqrt(19))
    ].freeze

    # ---- Round Constants (FIPS 180-4 section 4.2.3) ----
    #
    # 80 constants from fractional parts of cube roots of first 80 primes.
    K = [
      0x428A2F98D728AE22, 0x7137449123EF65CD, 0xB5C0FBCFEC4D3B2F, 0xE9B5DBA58189DBBC,
      0x3956C25BF348B538, 0x59F111F1B605D019, 0x923F82A4AF194F9B, 0xAB1C5ED5DA6D8118,
      0xD807AA98A3030242, 0x12835B0145706FBE, 0x243185BE4EE4B28C, 0x550C7DC3D5FFB4E2,
      0x72BE5D74F27B896F, 0x80DEB1FE3B1696B1, 0x9BDC06A725C71235, 0xC19BF174CF692694,
      0xE49B69C19EF14AD2, 0xEFBE4786384F25E3, 0x0FC19DC68B8CD5B5, 0x240CA1CC77AC9C65,
      0x2DE92C6F592B0275, 0x4A7484AA6EA6E483, 0x5CB0A9DCBD41FBD4, 0x76F988DA831153B5,
      0x983E5152EE66DFAB, 0xA831C66D2DB43210, 0xB00327C898FB213F, 0xBF597FC7BEEF0EE4,
      0xC6E00BF33DA88FC2, 0xD5A79147930AA725, 0x06CA6351E003826F, 0x142929670A0E6E70,
      0x27B70A8546D22FFC, 0x2E1B21385C26C926, 0x4D2C6DFC5AC42AED, 0x53380D139D95B3DF,
      0x650A73548BAF63DE, 0x766A0ABB3C77B2A8, 0x81C2C92E47EDAEE6, 0x92722C851482353B,
      0xA2BFE8A14CF10364, 0xA81A664BBC423001, 0xC24B8B70D0F89791, 0xC76C51A30654BE30,
      0xD192E819D6EF5218, 0xD69906245565A910, 0xF40E35855771202A, 0x106AA07032BBD1B8,
      0x19A4C116B8D2D0C8, 0x1E376C085141AB53, 0x2748774CDF8EEB99, 0x34B0BCB5E19B48A8,
      0x391C0CB3C5C95A63, 0x4ED8AA4AE3418ACB, 0x5B9CCA4F7763E373, 0x682E6FF3D6B2B8A3,
      0x748F82EE5DEFB2FC, 0x78A5636F43172F60, 0x84C87814A1F0AB72, 0x8CC702081A6439EC,
      0x90BEFFFA23631E28, 0xA4506CEBDE82BDE9, 0xBEF9A3F7B2C67915, 0xC67178F2E372532B,
      0xCA273ECEEA26619C, 0xD186B8C721C0C207, 0xEADA7DD6CDE0EB1E, 0xF57D4F7FEE6ED178,
      0x06F067AA72176FBA, 0x0A637DC5A2C898A6, 0x113F9804BEF90DAE, 0x1B710B35131C471B,
      0x28DB77F523047D84, 0x32CAAB7B40C72493, 0x3C9EBE0A15C9BEBC, 0x431D67C49C100D4C,
      0x4CC5D4BECB3E42B6, 0x597F299CFC657E2A, 0x5FCB6FAB3AD6FAEC, 0x6C44198C4A475817
    ].freeze

    # ---- Bitwise Helpers ----
    #
    # SHA-512 uses right-rotations (ROTR) and right-shifts (SHR) on 64-bit
    # words. Ruby integers never overflow, so we mask to 64 bits.

    # Circular right shift of x by n bits within a 64-bit word.
    # Bits that fall off the right wrap around to the left.
    def self.rotr(n, x)
      ((x >> n) | (x << (64 - n))) & MASK64
    end

    # ---- Sigma Functions ----
    #
    # Four mixing functions that XOR multiple rotated/shifted copies of a word.
    # Big-sigma (upper-case) mix working variables during compression.
    # Small-sigma (lower-case) mix words during schedule expansion.

    def self.big_sigma0(x)
      rotr(28, x) ^ rotr(34, x) ^ rotr(39, x)
    end

    def self.big_sigma1(x)
      rotr(14, x) ^ rotr(18, x) ^ rotr(41, x)
    end

    def self.small_sigma0(x)
      rotr(1, x) ^ rotr(8, x) ^ ((x >> 7) & MASK64)
    end

    def self.small_sigma1(x)
      rotr(19, x) ^ rotr(61, x) ^ ((x >> 6) & MASK64)
    end

    # ---- Logical Functions ----
    #
    # Ch(x,y,z) = "Choice": x picks between y and z for each bit.
    # Maj(x,y,z) = "Majority": output is 1 if >= 2 of 3 inputs are 1.

    def self.ch(x, y, z)
      ((x & y) ^ (~x & z)) & MASK64
    end

    def self.maj(x, y, z)
      (x & y) ^ (x & z) ^ (y & z)
    end

    # ---- Padding ----
    #
    # Extends the message to a multiple of 128 bytes:
    #   1. Append 0x80.
    #   2. Append zeros until length == 112 (mod 128).
    #   3. Append 128-bit big-endian bit length (upper 64 bits = 0).
    #
    # We use Ruby's pack: "N" = 32-bit big-endian unsigned.
    # "Q>" is 64-bit big-endian unsigned.
    def self.pad(data)
      bit_len = data.bytesize * 8
      msg = data.dup.b
      msg << "\x80".b
      msg << "\x00".b while msg.bytesize % 128 != 112
      # 128-bit big-endian length field (16 bytes).
      # Upper 64 bits are zero (practical messages never exceed 2^64 bits).
      # Lower 64 bits hold the bit length.
      # Pack as four 32-bit big-endian values: [0, 0, hi32, lo32].
      msg << [0, 0, bit_len >> 32, bit_len & 0xFFFFFFFF].pack("N4")
      msg
    end

    # ---- Message Schedule ----
    #
    # Each 128-byte block is parsed as 16 big-endian 64-bit words, then
    # expanded to 80 words:
    #   W[t] = sigma1(W[t-2]) + W[t-7] + sigma0(W[t-15]) + W[t-16]
    #
    # Ruby's "Q>16" unpacks 16 unsigned 64-bit big-endian integers.
    def self.schedule(block)
      w = block.unpack("Q>16")
      (16...80).each do |i|
        w << ((small_sigma1(w[i - 2]) + w[i - 7] + small_sigma0(w[i - 15]) + w[i - 16]) & MASK64)
      end
      w
    end

    # ---- Compression Function ----
    #
    # 80 rounds of mixing fold one 128-byte block into the eight-word state.
    #
    # Each round:
    #   T1 = h + Sigma1(e) + Ch(e,f,g) + K[t] + W[t]
    #   T2 = Sigma0(a) + Maj(a,b,c)
    #   shift: h=g, g=f, f=e, e=d+T1, d=c, c=b, b=a, a=T1+T2
    #
    # Davies-Meyer feed-forward adds compressed output back to input state.
    def self.compress(state, block)
      w = schedule(block)
      a, b, c, d, e, f, g, h = state

      80.times do |t|
        t1 = (h + big_sigma1(e) + ch(e, f, g) + K[t] + w[t]) & MASK64
        t2 = (big_sigma0(a) + maj(a, b, c)) & MASK64
        h = g
        g = f
        f = e
        e = (d + t1) & MASK64
        d = c
        c = b
        b = a
        a = (t1 + t2) & MASK64
      end

      [
        (state[0] + a) & MASK64,
        (state[1] + b) & MASK64,
        (state[2] + c) & MASK64,
        (state[3] + d) & MASK64,
        (state[4] + e) & MASK64,
        (state[5] + f) & MASK64,
        (state[6] + g) & MASK64,
        (state[7] + h) & MASK64
      ]
    end

    # ---- Public API ----

    # Compute the SHA-512 digest of data. Returns a 64-byte binary String.
    #
    #   CodingAdventures::Sha512.sha512("abc").unpack1("H*")
    #   # -> "ddaf35a193617aba..."
    #
    # "Q>8" packs eight 64-bit big-endian unsigned integers into 64 bytes.
    def self.sha512(data)
      padded = pad(data.b)
      state = INIT.dup
      0.step(padded.bytesize - 1, 128) do |i|
        state = compress(state, padded[i, 128])
      end
      state.pack("Q>8")
    end

    # Compute SHA-512 and return the 128-character lowercase hex string.
    #
    #   CodingAdventures::Sha512.sha512_hex("abc")
    #   # -> "ddaf35a193617aba..."
    def self.sha512_hex(data)
      sha512(data).unpack1("H*")
    end

    # Streaming SHA-512 hasher that accepts data in multiple chunks.
    #
    #   h = CodingAdventures::Sha512::Digest.new
    #   h.update("ab")
    #   h.update("c")
    #   h.hexdigest  # -> "ddaf35a193617aba..."
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
        while @buffer.bytesize >= 128
          @state = Sha512.compress(@state, @buffer[0, 128])
          @buffer = @buffer[128..]
        end
        self
      end
      alias_method :<<, :update

      # Return the 64-byte binary digest of all data fed so far.
      # Non-destructive: calling digest twice returns the same bytes.
      def digest
        bit_len = @byte_count * 8
        tail = @buffer.dup
        tail << "\x80".b
        tail << "\x00".b while tail.bytesize % 128 != 112
        tail << [0, 0, bit_len >> 32, bit_len & 0xFFFFFFFF].pack("N4")

        state = @state.dup
        0.step(tail.bytesize - 1, 128) do |i|
          state = Sha512.compress(state, tail[i, 128])
        end
        state.pack("Q>8")
      end

      # Return the 128-character hex string of the digest.
      def hexdigest
        digest.unpack1("H*")
      end

      # Return an independent copy of the current hasher state.
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
