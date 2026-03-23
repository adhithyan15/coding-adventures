# frozen_string_literal: true

# --------------------------------------------------------------------------
# bitset.rb -- Bitset: A Compact Boolean Array Packed into 64-bit Words
# ==========================================================================
#
# A bitset stores a sequence of bits -- each one either 0 or 1 -- packed
# into machine-word-sized integers. Instead of using an entire object
# reference to represent a single true/false value, a bitset packs 64 of
# them into a single Integer.
#
# Why does this matter?
#
# 1. **Space**: 10,000 booleans as an Array of true/false ~ 80,000 bytes
#    (each is a heap-allocated object with a pointer in the array).
#    As a bitset ~ 1,250 bytes. That's a 64x improvement.
#
# 2. **Speed**: AND-ing two boolean arrays loops over 10,000 elements.
#    AND-ing two bitsets loops over ~157 words. Ruby's Integer supports
#    bitwise operations natively, operating on 64 bits at once.
#
# 3. **Ubiquity**: Bitsets appear in Bloom filters, register allocators,
#    graph algorithms (visited sets), database bitmap indexes, filesystem
#    free-block bitmaps, network subnet masks, and garbage collectors.
#
# Bit Ordering: LSB-First
# -----------------------
#
# We use Least Significant Bit first ordering. Bit 0 is the least significant
# bit of word 0. Bit 63 is the most significant bit of word 0. Bit 64 is the
# least significant bit of word 1. And so on.
#
#     Word 0                              Word 1
#     +-----------------------------+     +-----------------------------+
#     | bit 63  ...  bit 2  bit 1  bit 0| | bit 127 ... bit 65  bit 64 |
#     +-----------------------------+     +-----------------------------+
#     MSB <------------------- LSB        MSB <------------------- LSB
#
# The three fundamental formulas that drive every bitset operation:
#
#     word_index = i / 64       (which word contains bit i?)
#     bit_offset = i % 64       (which position within that word?)
#     bitmask    = 1 << (i % 64)  (a mask with only bit i set)
#
# These are the heart of the entire implementation.
#
# Ruby-specific notes
# -------------------
#
# Ruby's Integer is arbitrary-precision (Bignum), so there's no overflow
# concern. However, we must be careful: Ruby integers are signed, and
# bitwise NOT (~) on a positive integer produces a negative result. For
# example, ~5 == -6 in Ruby (two's complement). To work around this,
# whenever we need to invert bits within a word, we XOR with WORD_MASK
# (0xFFFFFFFFFFFFFFFF) instead of using ~. This keeps all values positive
# and avoids signed-integer surprises.
#
#     Instead of:  ~word          (produces negative number)
#     We use:      word ^ WORD_MASK  (produces positive 64-bit inverted value)
#
# --------------------------------------------------------------------------

require_relative "bitset/version"

module CodingAdventures
  module Bitset

    # ======================================================================
    # Constants
    # ======================================================================
    #
    # BITS_PER_WORD is 64 because we treat each element of our @words Array
    # as a 64-bit unsigned integer. Every formula in this module uses this
    # constant rather than a magic number.

    BITS_PER_WORD = 64

    # WORD_MASK is a 64-bit value with all bits set: 0xFFFFFFFFFFFFFFFF.
    # We use this instead of bitwise NOT (~) because Ruby's ~ on positive
    # integers returns negative values (two's complement), which would
    # break our unsigned-word abstraction.
    #
    #     ~0  in Ruby => -1 (all bits set in two's complement, but negative)
    #     WORD_MASK   =>  0xFFFFFFFFFFFFFFFF (all 64 bits set, positive)
    #
    # By XOR-ing with WORD_MASK, we flip exactly the lower 64 bits while
    # keeping the result non-negative.

    WORD_MASK = (1 << BITS_PER_WORD) - 1 # 0xFFFFFFFFFFFFFFFF

    # ======================================================================
    # Error type
    # ======================================================================
    #
    # We have exactly one error class: BitsetError, raised when an invalid
    # binary string is passed to from_binary_str. This keeps the error
    # hierarchy minimal and focused.

    class BitsetError < StandardError; end

    # ======================================================================
    # The Bitset class
    # ======================================================================
    #
    # Internal Representation
    # ~~~~~~~~~~~~~~~~~~~~~~~
    #
    # We store bits in an Array of Integer called @words. Each Integer is
    # treated as a 64-bit unsigned value (masked to WORD_MASK). We also
    # track @len, the logical size -- the number of bits the user considers
    # "addressable". The capacity is always @words.length * 64.
    #
    #     +--------------------------------------------------------------+
    #     |                    capacity (256 bits = 4 words)              |
    #     |                                                              |
    #     |  +------------------------------------------+                |
    #     |  |              len (200 bits)                | ... unused .. |
    #     |  |  (highest addressable bit index + 1)      | (always zero) |
    #     |  +------------------------------------------+                |
    #     +--------------------------------------------------------------+
    #
    # **Clean-trailing-bits invariant**: Bits beyond @len in the last word
    # are always zero. This is critical for correctness of popcount, any?,
    # all?, none?, equality, and to_integer. Every operation that modifies
    # the last word must clean trailing bits afterwards.

    class Bitset
      include Comparable

      # ----------------------------------------------------------------
      # Constructors
      # ----------------------------------------------------------------

      # Create a new bitset with all bits initially zero.
      #
      # The +size+ parameter sets the logical length (@len). The capacity
      # is rounded up to the next multiple of 64.
      #
      #   bs = Bitset.new(100)
      #   bs.size       # => 100
      #   bs.capacity   # => 128  (2 words * 64 bits/word)
      #   bs.popcount   # => 0    (all bits start as zero)
      #
      # Bitset.new(0) is valid and creates an empty bitset:
      #
      #   bs = Bitset.new(0)
      #   bs.size       # => 0
      #   bs.capacity   # => 0
      #
      def initialize(size)
        raise ArgumentError, "size must be non-negative" if size < 0

        @len = size
        @words = Array.new(words_needed(size), 0)
      end

      # Create a bitset from a non-negative integer.
      #
      # Bit 0 of the bitset is the least significant bit of +value+.
      # The @len of the result is the position of the highest set bit + 1.
      # If +value+ == 0, then @len = 0.
      #
      # Ruby's Integer is arbitrary-precision, so we can handle values of
      # any size. We split the integer into 64-bit words by repeatedly
      # masking and shifting.
      #
      #   bs = Bitset.from_integer(5)  # binary: 101
      #   bs.size       # => 3  (highest bit is position 2)
      #   bs.test?(0)   # => true
      #   bs.test?(1)   # => false
      #   bs.test?(2)   # => true
      #
      def self.from_integer(value)
        raise ArgumentError, "value must be non-negative" if value < 0

        # Special case: zero produces an empty bitset.
        if value == 0
          return new(0)
        end

        # Determine the logical length: position of highest set bit + 1.
        # Ruby's Integer#bit_length returns exactly this.
        #
        #   5.bit_length   => 3  (binary 101, highest bit at position 2)
        #   255.bit_length => 8  (binary 11111111)
        #   0.bit_length   => 0
        #
        len = value.bit_length

        # Split the integer into 64-bit words. We extract the lowest 64 bits
        # with & WORD_MASK, then shift right by 64 to get the next word.
        #
        #     value = 0x1_FFFFFFFF_00000005
        #     word 0 = value & 0xFFFFFFFFFFFFFFFF = 0x00000005
        #     value >>= 64
        #     word 1 = value & 0xFFFFFFFFFFFFFFFF = 0x1FFFFFFFF
        #     ...
        words = []
        remaining = value
        while remaining > 0
          words << (remaining & WORD_MASK)
          remaining >>= BITS_PER_WORD
        end

        bs = allocate
        bs.send(:initialize_internal, len, words)
        bs
      end

      # Create a bitset from a binary string like "1010".
      #
      # The leftmost character is the highest-indexed bit (conventional binary
      # notation, matching how humans write numbers). The rightmost character
      # is bit 0.
      #
      #     String-to-bits mapping:
      #
      #     Input string: "1 0 1 0"
      #     Position:      3 2 1 0    (leftmost = highest bit index)
      #
      #     Bit 0 = '0' (rightmost char)
      #     Bit 1 = '1'
      #     Bit 2 = '0'
      #     Bit 3 = '1' (leftmost char)
      #
      #     This is the same as the integer 10 (binary 1010).
      #
      # Raises BitsetError if the string contains characters other than
      # '0' and '1'.
      #
      #   bs = Bitset.from_binary_str("1010")
      #   bs.size     # => 4
      #   bs.test?(1) # => true   (bit 1 = '1')
      #   bs.test?(3) # => true   (bit 3 = '1')
      #   bs.test?(0) # => false  (bit 0 = '0')
      #
      def self.from_binary_str(str)
        # Validate: every character must be '0' or '1'.
        unless str.match?(/\A[01]*\z/)
          raise BitsetError, "invalid binary string: #{str.inspect}"
        end

        # Empty string produces an empty bitset.
        return new(0) if str.empty?

        # The string length is the logical len of the bitset.
        len = str.length
        bs = new(len)

        # Walk the string from right to left (LSB to MSB).
        # The rightmost character (index str.length-1) is bit 0.
        # The leftmost character (index 0) is bit str.length-1.
        str.reverse.each_char.with_index do |ch, bit_idx|
          if ch == "1"
            wi = bit_idx / BITS_PER_WORD
            bs.instance_variable_get(:@words)[wi] |= (1 << (bit_idx % BITS_PER_WORD))
          end
        end

        # Clean trailing bits defensively.
        bs.send(:clean_trailing_bits)
        bs
      end

      # ----------------------------------------------------------------
      # Single-bit operations
      # ----------------------------------------------------------------
      #
      # These are the bread-and-butter operations: set a bit, clear a bit,
      # test whether a bit is set, toggle a bit. Each one translates to a
      # single bitwise operation on the containing word.
      #
      # Growth semantics:
      #   - set(i) and toggle(i) AUTO-GROW the bitset if i >= @len.
      #   - test?(i) and clear(i) do NOT grow. They return false / do nothing
      #     for out-of-range indices. This is safe because unallocated bits
      #     are conceptually zero.

      # Set bit +i+ to 1. Auto-grows the bitset if +i+ >= @len.
      #
      # Returns +self+ for method chaining.
      #
      # How auto-growth works:
      #
      # If +i+ is beyond the current capacity, we double the capacity
      # repeatedly until it's large enough (with a minimum of 64 bits).
      # This is the same amortized O(1) strategy used by ArrayList and Vec.
      #
      #     Before: len=100, capacity=128 (2 words)
      #     set(200): 200 >= 128, so double: 128 -> 256. Now 200 < 256.
      #     After: len=201, capacity=256 (4 words)
      #
      #   bs = Bitset.new(10)
      #   bs.set(5)
      #   bs.test?(5)   # => true
      #
      #   # Auto-growth:
      #   bs.set(100)   # grows from len=10 to len=101
      #   bs.size       # => 101
      #
      def set(i)
        ensure_capacity(i)
        # The core operation: OR the bitmask into the word.
        #
        #     @words[2] = 0b...0000_0000
        #     mask      = 0b...0010_0000   (bit 5 within the word)
        #     result    = 0b...0010_0000   (bit 5 is now set)
        #
        # OR is idempotent: setting an already-set bit is a no-op.
        @words[i / BITS_PER_WORD] |= (1 << (i % BITS_PER_WORD))
        self
      end

      # Clear bit +i+ (set to 0). No-op if +i+ >= @len (does not grow).
      #
      # Returns +self+ for method chaining.
      #
      # How it works:
      #
      # We AND the word with the inverted bitmask. The inverted mask has all
      # bits set EXCEPT the target bit, so every other bit is preserved:
      #
      #     @words[2] = 0b...0010_0100   (bits 2 and 5 set)
      #     mask      = 0b...0010_0000   (bit 5)
      #     inv_mask  = 0b...1101_1111   (everything except bit 5)
      #     result    = 0b...0000_0100   (bit 5 cleared, bit 2 preserved)
      #
      # Note: We use XOR with WORD_MASK instead of ~ to avoid negative numbers.
      #
      #   bs = Bitset.new(10)
      #   bs.set(5).clear(5)
      #   bs.test?(5)   # => false
      #
      #   # Clearing beyond len is a no-op:
      #   bs.clear(999)
      #   bs.size       # => 10  (no growth)
      #
      def clear(i)
        return self if i >= @len # out of range: nothing to clear

        mask = (1 << (i % BITS_PER_WORD))
        inv_mask = mask ^ WORD_MASK
        @words[i / BITS_PER_WORD] &= inv_mask
        self
      end

      # Test whether bit +i+ is set. Returns false if +i+ >= @len.
      #
      # This is a pure read operation -- it never modifies the bitset.
      # Testing a bit beyond the bitset's length returns false because
      # unallocated bits are conceptually zero.
      #
      # How it works:
      #
      #     @words[2] = 0b...0010_0100   (bits 2 and 5 set)
      #     mask      = 0b...0010_0000   (bit 5)
      #     result    = 0b...0010_0000   (non-zero -> bit 5 is set)
      #
      #   bs = Bitset.new(10)
      #   bs.set(5)
      #   bs.test?(5)    # => true
      #   bs.test?(3)    # => false
      #   bs.test?(999)  # => false (beyond len)
      #
      def test?(i)
        return false if i >= @len # out of range: conceptually zero

        (@words[i / BITS_PER_WORD] & (1 << (i % BITS_PER_WORD))) != 0
      end

      # Alias for test? to match the spec's "test(i)" naming.
      alias_method :test, :test?

      # Toggle (flip) bit +i+. Auto-grows if +i+ >= @len.
      #
      # If the bit is 0, it becomes 1. If it's 1, it becomes 0.
      # Returns +self+ for method chaining.
      #
      # How it works:
      #
      # XOR with the bitmask flips exactly one bit:
      #
      #     @words[2] = 0b...0010_0100   (bits 2 and 5 set)
      #     mask      = 0b...0010_0000   (bit 5)
      #     result    = 0b...0000_0100   (bit 5 flipped to 0)
      #
      #   bs = Bitset.new(10)
      #   bs.toggle(5)     # 0 -> 1
      #   bs.test?(5)      # => true
      #   bs.toggle(5)     # 1 -> 0
      #   bs.test?(5)      # => false
      #
      def toggle(i)
        ensure_capacity(i)
        @words[i / BITS_PER_WORD] ^= (1 << (i % BITS_PER_WORD))
        # Toggle might set a bit in the last word's trailing region after
        # growth. Clean just in case.
        clean_trailing_bits
        self
      end

      # ----------------------------------------------------------------
      # Bulk bitwise operations
      # ----------------------------------------------------------------
      #
      # All bulk operations return a NEW bitset. They don't modify either
      # operand. The result has len = max(a.len, b.len).
      #
      # When two bitsets have different lengths, the shorter one is
      # "zero-extended" conceptually. In practice, we just stop reading
      # from the shorter one's words once they run out and treat missing
      # words as zero.
      #
      # Performance: each operation processes one 64-bit word per loop
      # iteration, so 64 bits are handled in a single CPU instruction.

      # Bitwise AND: result bit is 1 only if BOTH input bits are 1.
      #
      # Truth table:
      #     A  B  A&B
      #     0  0   0
      #     0  1   0
      #     1  0   0
      #     1  1   1
      #
      # AND is used for **intersection**: elements that are in both sets.
      #
      #   a = Bitset.from_integer(0b1100)  # bits 2,3
      #   b = Bitset.from_integer(0b1010)  # bits 1,3
      #   c = a.bitwise_and(b)
      #   c.to_integer  # => 0b1000 (8) -- only bit 3
      #
      def bitwise_and(other)
        result_len = [@len, other.size].max
        max_words = [@words.length, other.send(:word_count)].max
        result_words = Array.new(max_words, 0)

        max_words.times do |i|
          a = i < @words.length ? @words[i] : 0
          b = i < other.send(:word_count) ? other.send(:word_at, i) : 0
          result_words[i] = a & b
        end

        self.class.send(:from_raw, result_len, result_words)
      end

      # Bitwise OR: result bit is 1 if EITHER (or both) input bits are 1.
      #
      # Truth table:
      #     A  B  A|B
      #     0  0   0
      #     0  1   1
      #     1  0   1
      #     1  1   1
      #
      # OR is used for **union**: elements that are in either set.
      #
      #   a = Bitset.from_integer(0b1100)  # bits 2,3
      #   b = Bitset.from_integer(0b1010)  # bits 1,3
      #   c = a.bitwise_or(b)
      #   c.to_integer  # => 0b1110 (14) -- bits 1,2,3
      #
      def bitwise_or(other)
        result_len = [@len, other.size].max
        max_words = [@words.length, other.send(:word_count)].max
        result_words = Array.new(max_words, 0)

        max_words.times do |i|
          a = i < @words.length ? @words[i] : 0
          b = i < other.send(:word_count) ? other.send(:word_at, i) : 0
          result_words[i] = (a | b) & WORD_MASK
        end

        self.class.send(:from_raw, result_len, result_words)
      end

      # Bitwise XOR: result bit is 1 if the input bits DIFFER.
      #
      # Truth table:
      #     A  B  A^B
      #     0  0   0
      #     0  1   1
      #     1  0   1
      #     1  1   0
      #
      # XOR is used for **symmetric difference**: elements in either set
      # but not both.
      #
      #   a = Bitset.from_integer(0b1100)  # bits 2,3
      #   b = Bitset.from_integer(0b1010)  # bits 1,3
      #   c = a.bitwise_xor(b)
      #   c.to_integer  # => 0b0110 (6) -- bits 1,2
      #
      def bitwise_xor(other)
        result_len = [@len, other.size].max
        max_words = [@words.length, other.send(:word_count)].max
        result_words = Array.new(max_words, 0)

        max_words.times do |i|
          a = i < @words.length ? @words[i] : 0
          b = i < other.send(:word_count) ? other.send(:word_at, i) : 0
          result_words[i] = (a ^ b) & WORD_MASK
        end

        self.class.send(:from_raw, result_len, result_words)
      end

      # Bitwise NOT: flip every bit within @len.
      #
      # Truth table:
      #     A  ~A
      #     0   1
      #     1   0
      #
      # NOT is used for **complement**: elements NOT in the set.
      #
      # **Important**: NOT flips bits within @len, NOT within capacity.
      # Bits beyond @len remain zero (clean-trailing-bits invariant).
      # The result has the same @len as the input.
      #
      # We use XOR with WORD_MASK instead of Ruby's ~ operator because
      # ~ on a positive Integer produces a negative result in Ruby:
      #
      #     ~5 == -6   (two's complement)
      #
      # XOR with WORD_MASK flips exactly the lower 64 bits and stays positive.
      #
      #   a = Bitset.from_integer(0b1010)  # len=4, bits 1,3 set
      #   b = a.bitwise_not
      #   b.to_integer  # => 0b0101 (5) -- len=4, bits 0,2 set
      #
      def bitwise_not
        result_words = @words.map { |w| w ^ WORD_MASK }

        # Critical: clean trailing bits! The XOR flipped ALL bits in every
        # word, including the trailing bits beyond @len that were zero.
        # We must zero them out again to maintain the invariant.
        self.class.send(:from_raw, @len, result_words)
      end

      # AND-NOT (set difference): bits in +self+ that are NOT in +other+.
      #
      # This is equivalent to self & (~other), but more efficient because
      # we don't need to create an intermediate NOT result.
      #
      # Truth table:
      #     A  B  A & ~B
      #     0  0    0
      #     0  1    0
      #     1  0    1
      #     1  1    0
      #
      # AND-NOT is used for **set difference**: elements in A but not in B.
      #
      #   a = Bitset.from_integer(0b1110)  # bits 1,2,3
      #   b = Bitset.from_integer(0b1010)  # bits 1,3
      #   c = a.and_not(b)
      #   c.to_integer  # => 0b0100 (4) -- only bit 2
      #
      def and_not(other)
        result_len = [@len, other.size].max
        max_words = [@words.length, other.send(:word_count)].max
        result_words = Array.new(max_words, 0)

        max_words.times do |i|
          a = i < @words.length ? @words[i] : 0
          b = i < other.send(:word_count) ? other.send(:word_at, i) : 0
          # a & ~b: keep bits from a that are NOT in b.
          # We use XOR with WORD_MASK to invert b, then AND with a.
          result_words[i] = a & (b ^ WORD_MASK) & WORD_MASK
        end

        self.class.send(:from_raw, result_len, result_words)
      end

      # ----------------------------------------------------------------
      # Operator overloads
      # ----------------------------------------------------------------
      #
      # Ruby lets us define &, |, ^, and ~ so bitset expressions read
      # naturally: intersection = a & b, union = a | b, etc.

      # Bitwise AND operator. Same as bitwise_and.
      #   a & b
      def &(other)
        bitwise_and(other)
      end

      # Bitwise OR operator. Same as bitwise_or.
      #   a | b
      def |(other)
        bitwise_or(other)
      end

      # Bitwise XOR operator. Same as bitwise_xor.
      #   a ^ b
      def ^(other)
        bitwise_xor(other)
      end

      # Bitwise NOT operator. Same as bitwise_not.
      #   ~a
      def ~
        bitwise_not
      end

      # ----------------------------------------------------------------
      # Counting and query operations
      # ----------------------------------------------------------------

      # Count the number of set (1) bits. Named after the CPU instruction
      # POPCNT (population count) that does this for a single word.
      #
      # How it works:
      #
      # We count the set bits in each word using Ruby's Integer#digits(2)
      # which gives us the binary digits, then count the 1s. Alternatively,
      # we use a bit-counting trick. For clarity, we use Ruby's built-in
      # to_s(2).count("1") which is clean and fast enough.
      #
      # For a bitset with N bits, this runs in O(N/64) time -- we process
      # 64 bits per word.
      #
      #   bs = Bitset.from_integer(0b10110)  # bits 1,2,4 set
      #   bs.popcount  # => 3
      #
      def popcount
        @words.sum { |w| popcount_word(w) }
      end

      # Returns the logical length: the number of addressable bits.
      #
      # This is the value passed to Bitset.new(size), or the highest bit
      # index + 1 after any auto-growth operations.
      def size
        @len
      end

      # Returns the capacity: the total allocated bits (always a multiple of 64).
      #
      # Capacity >= size. The difference (capacity - size) is "slack space" --
      # bits that exist in memory but are always zero.
      def capacity
        @words.length * BITS_PER_WORD
      end

      # Returns true if at least one bit is set.
      #
      # Short-circuits: returns as soon as it finds a non-zero word,
      # without scanning the rest. This is O(1) in the best case
      # (first word is non-zero) and O(N/64) in the worst case.
      #
      #   bs = Bitset.new(100)
      #   bs.any?       # => false
      #   bs.set(50)
      #   bs.any?       # => true
      #
      def any?
        @words.any? { |w| w != 0 }
      end

      # Returns true if ALL bits in 0..@len are set.
      #
      # For an empty bitset (@len = 0), returns true -- this is
      # **vacuous truth**, the same convention used by Ruby's
      # Enumerable#all? on an empty collection, Python's all([]),
      # and mathematical logic ("for all x in {}, P(x)" is true).
      #
      # How it works:
      #
      # For each full word (all words except possibly the last), we check
      # if every bit is set (word == WORD_MASK, i.e., all 64 bits are 1).
      #
      # For the last word, we only check the bits within @len. We create
      # a mask of the valid bits and check that all valid bits are set.
      #
      #   Bitset.new(0).all?                          # => true (vacuous truth)
      #   Bitset.from_binary_str("1111").all?          # => true
      #   Bitset.from_binary_str("1110").all?          # => false
      #
      def all?
        # Vacuous truth: all bits of nothing are set.
        return true if @len == 0

        num_words = @words.length

        # Check all full words (all bits must be 1 = WORD_MASK).
        (num_words - 1).times do |i|
          return false if @words[i] != WORD_MASK
        end

        # Check the last word: only the bits within @len matter.
        remaining = @len % BITS_PER_WORD
        if remaining == 0
          # @len is a multiple of 64, so the last word is a full word.
          @words[num_words - 1] == WORD_MASK
        else
          # Create a mask for the valid bits: (1 << remaining) - 1
          # Example: remaining = 8 -> mask = 0xFF (bits 0-7)
          mask = (1 << remaining) - 1
          @words[num_words - 1] == mask
        end
      end

      # Returns true if no bits are set. Equivalent to !any?.
      #
      #   bs = Bitset.new(100)
      #   bs.none?  # => true
      #
      def none?
        !any?
      end

      # Returns true if the bitset has zero length.
      def empty?
        @len == 0
      end

      # ----------------------------------------------------------------
      # Iteration
      # ----------------------------------------------------------------

      # Iterate over the indices of all set bits in ascending order.
      #
      # If a block is given, yields each set bit index to the block and
      # returns self. If no block is given, returns an Enumerator.
      #
      # How it works: trailing-zero-count trick
      #
      # For each non-zero word, we use bit manipulation to find the lowest
      # set bit, yield its index, then clear it:
      #
      #     word = 0b10100100   (bits 2, 5, 7 are set)
      #
      #     Step 1: find lowest set bit position (trailing zeros)
      #             word & -word isolates the lowest set bit
      #             trailing_zeros gives us the position
      #             yield base_index + 2
      #             word &= word - 1   -> clear lowest set bit
      #
      #     Step 2: trailing_zeros = 5  -> yield base_index + 5
      #             word &= word - 1   -> clear lowest set bit
      #
      #     Step 3: trailing_zeros = 7  -> yield base_index + 7
      #             word &= word - 1   -> clear lowest set bit
      #
      #     word == 0, move to next word.
      #
      # The trick word &= (word - 1) clears the lowest set bit. Here's why:
      #
      #     word     = 0b10100100
      #     word - 1 = 0b10100011  (borrow propagates through trailing zeros)
      #     AND      = 0b10100000  (lowest set bit is cleared)
      #
      # This is O(k) where k is the number of set bits, and it skips zero
      # words entirely, making it very efficient for sparse bitsets.
      #
      #   bs = Bitset.from_integer(0b10100101)
      #   bs.each_set_bit { |i| print "#{i} " }
      #   # Output: 0 2 5 7
      #
      def each_set_bit(&block)
        return enum_for(:each_set_bit) unless block_given?

        @words.each_with_index do |word, word_idx|
          next if word == 0 # Skip zero words entirely -- 64 bits at once!

          base = word_idx * BITS_PER_WORD
          w = word
          while w != 0
            # Find the position of the lowest set bit.
            # Ruby doesn't have a trailing_zeros built-in, but we can
            # compute it: (w & -w) isolates the lowest set bit, then
            # bit_length - 1 gives its position.
            #
            # Example: w = 0b10100100
            #   w & -w = 0b00000100  (isolates bit 2)
            #   (w & -w).bit_length - 1 = 2
            #
            # Note: In Ruby, -w for a positive integer w uses two's complement,
            # but since we only care about the lowest set bit pattern, this works
            # correctly even though Ruby integers are arbitrary precision. The
            # key insight is that (w & -w) always isolates exactly the lowest
            # set bit regardless of integer width.
            lowest_bit = w & (-w)
            bit_pos = lowest_bit.bit_length - 1
            idx = base + bit_pos

            # Only yield bits within @len (skip any trailing capacity bits).
            yield idx if idx < @len

            # Clear the lowest set bit: word &= word - 1
            w &= (w - 1)
          end
        end

        self
      end

      # ----------------------------------------------------------------
      # Conversion operations
      # ----------------------------------------------------------------

      # Convert the bitset to a non-negative integer.
      #
      # Ruby has arbitrary-precision integers, so this always succeeds
      # regardless of bitset size.
      #
      # How it works:
      #
      # We reconstruct the integer by OR-ing each word shifted to its
      # correct position:
      #
      #     result = words[0] | (words[1] << 64) | (words[2] << 128) | ...
      #
      #   bs = Bitset.from_integer(42)
      #   bs.to_integer  # => 42
      #
      #   bs = Bitset.new(0)
      #   bs.to_integer  # => 0
      #
      def to_integer
        result = 0
        @words.each_with_index do |word, i|
          result |= (word << (i * BITS_PER_WORD))
        end
        result
      end

      # Convert to a binary string with the highest bit on the left.
      #
      # This is the inverse of from_binary_str. An empty bitset produces
      # an empty string "".
      #
      #   bs = Bitset.from_integer(5)  # binary 101
      #   bs.to_binary_str  # => "101"
      #
      #   bs = Bitset.new(0)
      #   bs.to_binary_str  # => ""
      #
      def to_binary_str
        return "" if @len == 0

        # Build the string from the highest bit (len-1) down to bit 0.
        # This produces conventional binary notation: MSB on the left.
        (@len - 1).downto(0).map { |i| test?(i) ? "1" : "0" }.join
      end

      # Human-readable debug representation.
      #
      # Format: "Bitset(101)" where the contents are the binary string.
      # An empty bitset produces "Bitset()".
      #
      #   bs = Bitset.from_integer(5)
      #   bs.to_s  # => "Bitset(101)"
      #
      def to_s
        "Bitset(#{to_binary_str})"
      end

      # Same as to_s for inspect.
      def inspect
        to_s
      end

      # ----------------------------------------------------------------
      # Equality
      # ----------------------------------------------------------------
      #
      # Two bitsets are equal if and only if they have the same @len and
      # the same bits set. Capacity is irrelevant to equality -- a bitset
      # with capacity = 128 can equal one with capacity = 256 if their
      # @len and set bits match.

      def ==(other)
        return false unless other.is_a?(Bitset)
        return false unless @len == other.size

        # Compare words. If one has more words (due to different capacity),
        # the extra words must all be zero (clean-trailing-bits invariant
        # guarantees this, but we check anyway for robustness).
        max_words = [@words.length, other.send(:word_count)].max
        max_words.times do |i|
          a = i < @words.length ? @words[i] : 0
          b = i < other.send(:word_count) ? other.send(:word_at, i) : 0
          return false if a != b
        end

        true
      end

      def eql?(other)
        self == other
      end

      def hash
        [@len, @words].hash
      end

      # ----------------------------------------------------------------
      # Private helpers
      # ----------------------------------------------------------------

      private

      # Internal constructor: set @len and @words directly, then clean.
      def initialize_internal(len, words)
        @len = len
        @words = words
        clean_trailing_bits
      end

      # Build a Bitset from raw len and words, cleaning trailing bits.
      # This is used by bulk operations to avoid going through the public
      # constructor which would allocate zeroed words.
      def self.from_raw(len, words)
        bs = allocate
        bs.send(:initialize_internal, len, words)
        bs
      end

      # How many words do we need to store +bit_count+ bits?
      #
      # This is ceiling division: (bit_count + 63) / 64.
      #
      #     words_needed(0)   = 0   (no bits, no words)
      #     words_needed(1)   = 1   (1 bit needs 1 word)
      #     words_needed(64)  = 1   (64 bits fit exactly in 1 word)
      #     words_needed(65)  = 2   (65 bits need 2 words)
      #     words_needed(200) = 4   (200 bits need ceil(200/64) = 4 words)
      #
      def words_needed(bit_count)
        (bit_count + BITS_PER_WORD - 1) / BITS_PER_WORD
      end

      # Count the number of set bits in a single 64-bit word.
      #
      # We use Ruby's Integer#to_s(2).count("1"). While not as fast as
      # a hardware POPCNT instruction, it's clear, correct, and the
      # performance is acceptable for a pure-Ruby implementation.
      #
      # Alternative: the Hamming weight algorithm (divide-and-conquer with
      # magic constants) could be used for speed, but clarity wins here.
      def popcount_word(word)
        word.to_s(2).count("1")
      end

      # Ensure the bitset has capacity for bit +i+. If not, grow by doubling.
      #
      # After this call, i < capacity and @len >= i + 1.
      #
      # Growth strategy:
      #
      # We double the capacity repeatedly until it exceeds +i+. The minimum
      # capacity after growth is 64 (one word). This doubling strategy gives
      # amortized O(1) growth, just like Vec::push or ArrayList.add.
      #
      #     Example: capacity=128, set(500)
      #       128 -> 256 -> 512 -> 1024  (stop: 500 < 1024)
      #
      def ensure_capacity(i)
        if i < capacity
          # Already have room. But we might need to update len.
          @len = i + 1 if i >= @len
          return
        end

        # Need to grow. Start with current capacity (or 64 as minimum).
        new_cap = [capacity, BITS_PER_WORD].max
        new_cap *= 2 while new_cap <= i

        # Extend the word array with zeros.
        new_word_count = new_cap / BITS_PER_WORD
        (@words.length...new_word_count).each { @words << 0 }

        # Update len to include the new bit.
        @len = i + 1 if i >= @len
      end

      # Zero out trailing bits beyond @len in the last word.
      #
      # This maintains the clean-trailing-bits invariant, which is critical
      # for correctness of popcount, any?, all?, none?, equality, and
      # to_integer.
      #
      #     Example: @len = 200, capacity = 256
      #
      #     Word 3 holds bits 192-255, but only bits 192-199 are "real".
      #     Bits 200-255 must always be zero.
      #
      #     used_bits_in_last_word = 200 % 64 = 8
      #     mask = (1 << 8) - 1 = 0xFF
      #     @words[3] &= mask   # zero out bits 8-63 of word 3
      #
      def clean_trailing_bits
        return if @len == 0 || @words.empty?

        remaining = @len % BITS_PER_WORD
        return if remaining == 0 # @len is a multiple of 64, no trailing bits

        # Create a mask that keeps only the lower `remaining` bits.
        mask = (1 << remaining) - 1
        @words[-1] &= mask
      end

      # Provide read access to internal word count for other Bitset instances.
      def word_count
        @words.length
      end

      # Provide read access to a specific word for other Bitset instances.
      def word_at(i)
        @words[i]
      end
    end
  end
end
