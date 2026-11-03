# frozen_string_literal: true

module CodingAdventures
  # Constant-time comparison helpers for byte strings and unsigned counters.
  #
  # == Why this exists as its own package
  #
  # A secret comparison that returns early on the first differing byte leaks how
  # far the match got. An attacker who controls one operand can recover the other
  # a byte at a time by measuring how long the comparison takes. The defence is
  # to look at every byte regardless, accumulating differences rather than
  # branching on them.
  #
  # That is a small amount of code and an easy thing to get subtly wrong, which
  # is exactly why it belongs in one audited place rather than being rewritten at
  # each call site. Ten other languages in this repository already have this
  # package; Ruby was the gap.
  #
  # == What "constant time" does and does not mean here
  #
  # These functions have no data-dependent branches and no early exits: the work
  # depends only on the *length* of the inputs, never on their contents. That is
  # the property callers need.
  #
  # It is not a claim about the machine. Ruby is a managed runtime with a
  # garbage collector and a JIT, and +String#getbyte+ is not a constant-time
  # primitive in the hardware sense. What this package guarantees is that the
  # *algorithm* leaks nothing through control flow. Timing that varies because
  # the GC ran is noise an attacker cannot steer; timing that varies because
  # byte 3 differed is a signal they can.
  #
  # Length is deliberately not hidden. +ct_eq+ returns false immediately for
  # mismatched lengths, because the lengths of the operands are almost always
  # public (a 32-byte key is 32 bytes whether or not you know its value), and
  # pretending otherwise would cost work without buying secrecy.
  module CtCompare
    U64_MAX = (1 << 64) - 1

    module_function

    # Return whether two byte strings are equal, without early exit.
    #
    # Every byte of both operands is read exactly once. Differences accumulate
    # into a single value that is compared to zero at the end, so the loop takes
    # the same path whether the strings match on byte 0 or byte 31.
    def ct_eq(left, right)
      left_bytes = as_bytes(left)
      right_bytes = as_bytes(right)
      return false unless left_bytes.bytesize == right_bytes.bytesize

      accumulator = 0
      index = 0
      while index < left_bytes.bytesize
        accumulator |= left_bytes.getbyte(index) ^ right_bytes.getbyte(index)
        index += 1
      end
      accumulator.zero?
    end

    # Fixed-size companion to ct_eq.
    #
    # In statically typed ports this takes fixed-width arrays, which lets the
    # compiler drop the length check entirely. Ruby has no such type, so this is
    # an alias -- kept so the six-language call sites read identically and a
    # reader moving between them is not left wondering what the difference is.
    def ct_eq_fixed(left, right) = ct_eq(left, right)

    # Select +left+ when +choice+ is 1, otherwise +right+ (when it is 0),
    # without branching on +choice+.
    #
    # Why an Integer and not true/false: Ruby has no branch-free way to turn a
    # boolean into a number. Every conversion is a branch on the secret bit
    # (+choice ? 0xFF : 0x00+), which CT01 forbids. Even the old validation
    # leaked it: +[true, false].include?(choice)+ stops at its first match, so
    # it did one comparison for +true+ and two for +false+. The bit is
    # therefore an Integer that must be exactly 0 or 1, as Go's
    # +crypto/subtle+ does.
    #
    # The trick is arithmetic rather than control flow: a mask of all-ones or
    # all-zeros turns the selection into XOR, so both inputs are read and the
    # same instructions run either way.
    #
    #   result = right ^ ((left ^ right) & mask)
    #
    # With mask = 0xFF that reduces to +left+; with mask = 0x00 it reduces to
    # +right+. Nothing observable distinguishes the two cases.
    def ct_select_bytes(left, right, choice)
      left_bytes = as_bytes(left)
      right_bytes = as_bytes(right)
      unless left_bytes.bytesize == right_bytes.bytesize
        raise ArgumentError, "ct_select_bytes requires equal-length byte strings"
      end

      # Only an Integer 0 or 1 is a valid choice. The type check is public; the
      # range check clears bit 0 and tests for zero, a single computation whose
      # result is the same for both valid values, so a valid call takes the
      # same path whichever bit is secret. (true/false are rejected: they are
      # not Integers.)
      unless choice.is_a?(Integer) && (choice & ~1).zero?
        raise ArgumentError, "ct_select_bytes requires choice to be 0 or 1"
      end

      # 0 -> -0 & 0xFF = 0x00 (right);  1 -> -1 & 0xFF = 0xFF (left)
      mask = -choice & 0xFF
      output = +"".b
      index = 0
      while index < left_bytes.bytesize
        left_byte = left_bytes.getbyte(index)
        right_byte = right_bytes.getbyte(index)
        output << (right_byte ^ ((left_byte ^ right_byte) & mask))
        index += 1
      end
      output
    end

    # Return whether two unsigned 64-bit integers are equal, without branching
    # on their values.
    #
    # Folding the XOR down to its sign bit avoids a comparison against zero that
    # a compiler might turn into a branch. Ruby would not, but the ports share
    # this shape so the reasoning transfers.
    def ct_eq_u64(left, right)
      validate_u64!(left, "left")
      validate_u64!(right, "right")
      difference = (left ^ right) & U64_MAX
      folded = (difference | (-difference & U64_MAX)) >> 63
      folded.zero?
    end

    # -- Bit-returning equality (CT01, "How a lane receives the selection bit")
    #
    # ct_select_bytes takes its selection bit as an Integer, because every way
    # Ruby has of turning true/false into 1/0 is an if, a ?: or a lookup. A
    # caller holding only ct_eq's boolean would need exactly that branch, so
    # each equality also returns the bit itself, computed from the difference
    # accumulator with arithmetic only. Ruby's >> on a negative Integer is an
    # arithmetic shift, so for an accumulator in 0..255:
    #
    #   acc = 0          (0 - 1) >> 8 = -1,  -1 & 1 = 1   (equal)
    #   acc = 1 .. 255   (acc - 1) >> 8 = 0,  0 & 1 = 0   (different)
    #
    # Use it as ct_select_bytes(a, b, ct_eq_bit(x, y)).

    # 1 when the byte strings are equal, else 0. A length mismatch is 0 and
    # reads neither input; lengths are public.
    def ct_eq_bit(left, right)
      left_bytes = as_bytes(left)
      right_bytes = as_bytes(right)
      return 0 unless left_bytes.bytesize == right_bytes.bytesize

      accumulator = 0
      index = 0
      while index < left_bytes.bytesize
        accumulator |= left_bytes.getbyte(index) ^ right_bytes.getbyte(index)
        index += 1
      end
      ((accumulator - 1) >> 8) & 1
    end

    # Fixed-size companion to ct_eq_bit (an alias, as ct_eq_fixed is).
    def ct_eq_fixed_bit(left, right) = ct_eq_bit(left, right)

    # 1 when the two unsigned 64-bit values are equal, else 0. The difference
    # lies in 0..2^64-1, so subtracting 1 is negative exactly when it was 0.
    def ct_eq_u64_bit(left, right)
      validate_u64!(left, "left")
      validate_u64!(right, "right")
      ((((left ^ right) & U64_MAX) - 1) >> 64) & 1
    end

    def as_bytes(value)
      case value
      when String
        value.b
      when Array
        # pack("C*") silently reduces mod 256 and truncates Floats, so an
        # unvalidated array would make an EQUALITY primitive answer true for
        # unequal inputs -- ct_eq([256], "\x00") and ct_eq([97.9], "a") both
        # would. That is the worst failure this function can have. The typed
        # ports cannot express it at all and Python raises; validate instead.
        unless value.all? { |byte| byte.is_a?(Integer) && byte.between?(0, 255) }
          raise ArgumentError, "byte values must be integers in 0..255"
        end

        value.pack("C*")
      else
        raise ArgumentError, "expected a String or an Array of byte values"
      end
    end

    def validate_u64!(value, name)
      return if value.is_a?(Integer) && !value.negative? && value <= U64_MAX

      raise ArgumentError, "#{name} must be an unsigned 64-bit integer"
    end

    private_class_method :as_bytes, :validate_u64!
  end
end
