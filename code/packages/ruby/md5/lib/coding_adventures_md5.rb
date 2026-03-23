# frozen_string_literal: true

# coding_adventures_md5.rb — MD5 message digest algorithm (RFC 1321) from scratch.
#
# What Is MD5?
# ============
# MD5 (Message Digest 5) takes any sequence of bytes and produces a fixed-size
# 16-byte (128-bit) "fingerprint" called a digest. The same input always produces
# the same digest. Change even one bit of input and the digest changes completely.
#
# Created by Ron Rivest in 1991 as an improvement over MD4. Standardized in
# RFC 1321. MD5 is cryptographically broken (collision attacks since 2004) and
# should NOT be used for security purposes (digital signatures, password hashing,
# TLS certificates). It remains valid for: non-security checksums, UUID v3, and
# legacy systems that already use it.
#
# How MD5 Differs From SHA-1
# ===========================
# The most important difference is byte order:
#
#   Property     SHA-1       MD5
#   ──────────   ─────────   ─────────────
#   Output size  20 bytes    16 bytes
#   State words  5 (H₀..H₄)  4 (A,B,C,D)
#   Rounds       80          64
#   Block size   512 bits    512 bits
#   Word order   Big-endian  LITTLE-ENDIAN  ← key difference!
#
# Big-endian (SHA-1): most significant byte first.  0x0A0B0C0D → 0A 0B 0C 0D
# Little-endian (MD5): LEAST significant byte first. 0x0A0B0C0D → 0D 0C 0B 0A
#
# This is the #1 source of MD5 implementation bugs. In Ruby's pack/unpack:
#   "N"  = network / big-endian 32-bit unsigned (like SHA-1)
#   "V"  = little-endian 32-bit unsigned ← used by MD5
#
# Concretely:
#   SHA-1 reads block words:  block.unpack("N16")   # big-endian
#   MD5 reads block words:    block.unpack("V16")   # little-endian ← this file
#   SHA-1 writes hash:        state.pack("N5")      # big-endian
#   MD5 writes hash:          state.pack("V4")      # little-endian ← this file
#
# The T-Table (64 Precomputed Constants)
# ========================================
# MD5 uses 64 constants T[1..64], one per round. Each is derived from the sine
# function — a transcendental number with unpredictable bit patterns, ensuring no
# hidden mathematical backdoor:
#
#   T[i] = floor(abs(sin(i)) × 2^32)   for i = 1..64
#
# Why sine? Because sin(i) for integer i produces pseudo-random values between
# -1 and 1. Scaling by 2^32 and flooring gives a 32-bit integer. The pattern is
# "obviously" derived from a standard mathematical function, which proves to
# anyone that the constants were not chosen to weaken the algorithm.
#
# Example:
#   sin(1) ≈ 0.8414709848...
#   abs(sin(1)) × 2^32 = 0.8414709848 × 4294967296 = 3614090360.02...
#   floor(3614090360.02) = 3614090360 = 0xD76AA478 = T[1]
#
# RFC 1321 Test Vectors
# =====================
#   md5("")              → "d41d8cd98f00b204e9800998ecf8427e"
#   md5("a")             → "0cc175b9c0f1b6a831c399e269772661"
#   md5("abc")           → "900150983cd24fb0d6963f7d28e17f72"
#   md5("message digest") → "f96b697d7cb7938d525a2f31aaf161d0"

require_relative "coding_adventures/md5/version"

module CodingAdventures
  # MD5 message digest algorithm (RFC 1321) implemented from scratch.
  #
  # Public API:
  #   CodingAdventures::Md5.md5(data)      → 16-byte binary String
  #   CodingAdventures::Md5.md5_hex(data)  → 32-char hex String
  #   CodingAdventures::Md5::Digest        → streaming hasher class
  module Md5
    # ─── Internal Computation Module ──────────────────────────────────────────
    #
    # We isolate the low-level algorithm in a nested module so that:
    #  1. The Digest class can access compress() without it being a public
    #     method on the Md5 module itself.
    #  2. The module boundary makes the literate-programming structure clear.
    module Core # :nodoc:
      # ── T-Table: 64 Constants Derived From Sine ──────────────────────────
      #
      # T[i] = floor(abs(sin(i+1)) × 2^32)  for i in 0..63 (0-indexed)
      #
      # We index from 0 internally (T[0] corresponds to T[1] in RFC 1321).
      #
      # Ruby note: Ruby integers are arbitrary-precision (Bignum). We MUST mask
      # with & 0xFFFFFFFF after each arithmetic operation to stay in 32 bits.
      # Without the mask, intermediate values grow beyond 32 bits and produce
      # wrong output.
      T_TABLE = (0..63).map { |i| (Math.sin(i + 1).abs * (2**32)).floor & 0xFFFFFFFF }.freeze

      # ── Round Shift Amounts ───────────────────────────────────────────────
      #
      # Each of the 64 rounds rotates the working variable by a fixed number of
      # bit positions. The amounts are organized in 4 groups of 16:
      #
      # Group 1 (rounds  0–15): 7, 12, 17, 22 × 4
      # Group 2 (rounds 16–31): 5,  9, 14, 20 × 4
      # Group 3 (rounds 32–47): 4, 11, 16, 23 × 4
      # Group 4 (rounds 48–63): 6, 10, 15, 21 × 4
      SHIFT_TABLE = [
        7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22, # rounds  0–15
        5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20, # rounds 16–31
        4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23, # rounds 32–47
        6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21  # rounds 48–63
      ].freeze

      # ── Initialization Constants ──────────────────────────────────────────
      #
      # These four values are the initial state of the MD5 accumulator. They are
      # "nothing up my sleeve" numbers — the pattern 01 23 45 67 89 AB CD EF
      # arranged in little-endian 32-bit words:
      #
      #   A = 0x67452301 → stored as bytes: 01 23 45 67  (little-endian)
      #   B = 0xEFCDAB89 → stored as bytes: 89 AB CD EF
      #   C = 0x98BADCFE → stored as bytes: FE DC BA 98
      #   D = 0x10325476 → stored as bytes: 76 54 32 10
      INIT_STATE = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476].freeze

      # ── Helper: Circular Left Shift (ROTL) ───────────────────────────────
      #
      # Rotate the 32-bit value x left by n positions. Bits that fall off the
      # left edge wrap around to the right.
      #
      # Example (8-bit illustration): ROTL(3, 0b10110001) = 0b10001101
      #   Original:  1 0 1 1 0 0 0 1
      #   Shift 3:   [wraps: 1 0 1] then [0 0 0 1 1 0 1] → 1 0 0 0 1 1 0 1
      #
      # The & 0xFFFFFFFF mask is CRITICAL in Ruby because Ruby integers are
      # arbitrary-precision. Without it, x << n grows beyond 32 bits.
      def self.rotl(n, x)
        ((x << n) | (x >> (32 - n))) & 0xFFFFFFFF
      end

      # ── Compression Function ──────────────────────────────────────────────
      #
      # Mix one 64-byte block into the four-word state via 64 rounds.
      #
      # Block parsing: "V16" = 16 unsigned 32-bit little-endian integers.
      # This is the KEY difference from SHA-1 ("N16" = big-endian).
      #
      # Four stages of 16 rounds each, each using a different auxiliary function:
      #
      #   Stage  Rounds  Auxiliary f(b,c,d)            Message index g
      #   ─────  ──────  ────────────────────────────  ─────────────────
      #     1    0–15    (b & c) | (~b & d)            g = i
      #     2    16–31   (d & b) | (~d & c)            g = (5*i+1) % 16
      #     3    32–47   b ^ c ^ d                     g = (3*i+5) % 16
      #     4    48–63   c ^ (b | ~d)                  g = (7*i) % 16
      #
      # Stage 1 — the "F" function (multiplexer):
      #   (b & c) | (~b & d) means: if b then c else d
      #   When b=1, the result is c; when b=0, the result is d.
      #
      # Stage 2 — the "G" function (b and d roles swapped from F):
      #   (d & b) | (~d & c) means: if d then b else c
      #
      # Stage 3 — the "H" function (parity):
      #   b ^ c ^ d  →  1 if an odd number of inputs are 1
      #
      # Stage 4 — the "I" function (unusual mix):
      #   c ^ (b | ~d)
      #   When d=0 → ~d=1 → (b | ~d)=1 → result = c ^ 1 (inverts c)
      #   When d=1 → ~d=0 → (b | ~d)=b → result = c ^ b (parity of c and b)
      #
      # Each round:
      #   f  = auxiliary(b, c, d)         ← depends on stage
      #   new_a = (b + ROTL(shift[i], (a + f + m[g] + T[i]) mod 2^32)) mod 2^32
      #   a, b, c, d = d, new_a, b, c    ← rotate registers left
      #
      # Davies-Meyer feed-forward: after all 64 rounds, add back the original
      # state. This prevents the compression from being invertible.
      def self.compress(state, block)
        m = block.unpack("V16")           # V = unsigned 32-bit LITTLE-endian
        a0, b0, c0, d0 = state
        a, b, c, d = a0, b0, c0, d0

        64.times do |i|
          if i < 16
            # Stage 1 — F: selector "if b then c else d"
            f = ((b & c) | (~b & d)) & 0xFFFFFFFF
            g = i
          elsif i < 32
            # Stage 2 — G: selector "if d then b else c"
            f = ((d & b) | (~d & c)) & 0xFFFFFFFF
            g = (5 * i + 1) % 16
          elsif i < 48
            # Stage 3 — H: parity XOR
            f = (b ^ c ^ d) & 0xFFFFFFFF
            g = (3 * i + 5) % 16
          else
            # Stage 4 — I: "c ^ (b | ~d)"
            f = (c ^ (b | ~d)) & 0xFFFFFFFF
            g = (7 * i) % 16
          end

          new_a = (b + rotl(SHIFT_TABLE[i], (a + f + m[g] + T_TABLE[i]) & 0xFFFFFFFF)) & 0xFFFFFFFF
          a, b, c, d = d, new_a, b, c
        end

        # Davies-Meyer feed-forward: add original state back (mod 2^32)
        [
          (a0 + a) & 0xFFFFFFFF,
          (b0 + b) & 0xFFFFFFFF,
          (c0 + c) & 0xFFFFFFFF,
          (d0 + d) & 0xFFFFFFFF
        ]
      end

      # ── Padding ────────────────────────────────────────────────────────────
      #
      # Extend data to a multiple of 64 bytes, per RFC 1321 §3.1.
      #
      # Almost identical to SHA-1 padding, except the length is appended as a
      # 64-bit LITTLE-ENDIAN integer (not big-endian).
      #
      # Algorithm:
      #   1. Append the byte 0x80 (single 1-bit followed by 7 zero bits).
      #   2. Append zero bytes until length ≡ 56 (mod 64).
      #      This leaves 8 bytes (64 bits) for the length field.
      #   3. Append the original bit length as a 64-bit little-endian integer.
      #
      # Example — "abc" (3 bytes = 24 bits):
      #   61 62 63 80 [52 zero bytes] 18 00 00 00 00 00 00 00
      #                               ^^
      #   24 = 0x18, stored little-endian: 18 00 00 00 00 00 00 00
      #
      # We split the 64-bit length into two 32-bit LE words using "V2" pack:
      #   lo32 = bit_len & 0xFFFFFFFF   (lower 32 bits)
      #   hi32 = bit_len >> 32          (upper 32 bits, 0 for all sane inputs)
      def self.pad(data)
        bit_len = data.bytesize * 8
        msg = data.b.dup                   # .b = binary encoding
        msg << "\x80".b
        msg << "\x00".b while msg.bytesize % 64 != 56
        lo32 = bit_len & 0xFFFFFFFF
        hi32 = (bit_len >> 32) & 0xFFFFFFFF
        msg << [lo32, hi32].pack("V2")
        msg
      end
    end

    # ─── Public API: One-Shot Hash ─────────────────────────────────────────────

    # Compute the MD5 digest of data. Returns a 16-byte binary String.
    #
    # NOTE: MD5 is cryptographically broken. Do NOT use for passwords, digital
    # signatures, or security-sensitive checksums. Use for UUID v3 or legacy
    # compatibility only.
    #
    # Example:
    #   CodingAdventures::Md5.md5("abc").unpack1("H*")
    #   # => "900150983cd24fb0d6963f7d28e17f72"
    def self.md5(data)
      data = data.b if data.encoding != Encoding::BINARY
      padded = Core.pad(data)
      state = Core::INIT_STATE.dup
      # Process each 64-byte block through the compression function
      0.step(padded.bytesize - 1, 64) do |offset|
        block = padded.byteslice(offset, 64)
        state = Core.compress(state, block)
      end
      # Finalize: pack four state words as little-endian 32-bit integers (16 bytes)
      # "V4" = four unsigned 32-bit little-endian values
      state.pack("V4")
    end

    # Compute the MD5 digest of data. Returns a 32-character lowercase hex string.
    #
    # Example:
    #   CodingAdventures::Md5.md5_hex("abc")
    #   # => "900150983cd24fb0d6963f7d28e17f72"
    def self.md5_hex(data)
      md5(data).unpack1("H*")
    end

    # ─── Streaming Digest Class ────────────────────────────────────────────────
    #
    # Digest accepts data in multiple chunks — useful when the full message is not
    # available at once (e.g., reading a large file in parts, or hashing network
    # data as it arrives).
    #
    # The interface is similar to Ruby's OpenSSL::Digest:
    #
    #   d = CodingAdventures::Md5::Digest.new
    #   d.update("Hello, ")
    #   d << "world!"        # << is an alias for update
    #   d.hexdigest          # => "e5a00d6eeab1a4e0901b0ef31f645a0a"
    #
    # Multiple update() calls are equivalent to a single md5(all_data):
    #   Md5.md5_hex("abc") == Digest.new.update("ab").update("c").hexdigest
    class Digest
      # Initialize a new hasher with the MD5 starting state.
      def initialize
        @state = Core::INIT_STATE.dup
        @buffer = String.new("", encoding: Encoding::BINARY)
        @byte_count = 0
      end

      # Feed more bytes into the hash. Returns self for chaining.
      #
      # Internally, we accumulate bytes in a buffer and process complete 64-byte
      # blocks as they become available. Partial blocks are held until more data
      # arrives or digest() is called.
      def update(data)
        data = data.b if data.encoding != Encoding::BINARY
        @buffer << data
        @byte_count += data.bytesize

        # Process all complete 64-byte blocks from the front of the buffer
        while @buffer.bytesize >= 64
          block = @buffer.byteslice(0, 64)
          @state = Core.compress(@state, block)
          @buffer = @buffer.byteslice(64..) || String.new("", encoding: Encoding::BINARY)
        end

        self   # return self for chaining: d.update("a").update("b")
      end

      # << is an alias for update, enabling the stream operator:
      #   d << "chunk"  is equivalent to  d.update("chunk")
      alias << update

      # Return the 16-byte binary digest of all data fed so far.
      #
      # Non-destructive: calling digest() multiple times returns the same bytes.
      # Internally, we finalize a COPY of the current state so the original
      # accumulator is unchanged for further update() calls.
      def digest
        bit_len = @byte_count * 8

        # Build the padding tail from the remaining buffer (< 64 bytes)
        tail = @buffer.b.dup
        tail << "\x80".b
        tail << "\x00".b while tail.bytesize % 64 != 56
        # Append 64-bit LE bit length as two 32-bit LE words
        lo32 = bit_len & 0xFFFFFFFF
        hi32 = (bit_len >> 32) & 0xFFFFFFFF
        tail << [lo32, hi32].pack("V2")

        # Process the tail blocks on a SNAPSHOT of the current state
        state = @state.dup
        0.step(tail.bytesize - 1, 64) do |offset|
          block = tail.byteslice(offset, 64)
          state = Core.compress(state, block)
        end
        state.pack("V4")
      end

      # Return the 32-character hex string of the digest.
      def hexdigest
        digest.unpack1("H*")
      end

      # Return an independent copy of the current hasher state.
      #
      # Useful when you want to branch: hash a common prefix, then copy and
      # hash different suffixes independently without re-processing the prefix.
      #
      # Example:
      #   base = Digest.new.update("common prefix")
      #   branch1 = base.copy.update(" suffix A")
      #   branch2 = base.copy.update(" suffix B")
      def copy
        other = Digest.new
        other.instance_variable_set(:@state, @state.dup)
        other.instance_variable_set(:@buffer, @buffer.b.dup)
        other.instance_variable_set(:@byte_count, @byte_count)
        other
      end
    end
  end
end
