# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    module Fuzz
      # The mutator algebra (VER06 section 3, VER06i section 3).
      #
      # Every mutator takes a String (read as raw bytes) and returns a NEW binary
      # String; none modifies, freezes or returns its argument.  Offsets wrap with
      # Ruby's floored `%`, which is already non-negative for a positive modulus
      # (`-1 % 8 == 7`), and Ruby integers never overflow, so the corpus's
      # 1,000-digit values need no special "big" path.
      module Mutate
        # Interest values of section 3.4, by width in bytes.
        INTEREST_8 = [0, 1, 127, 128, 255].freeze
        INTEREST_16 = [0, 1, 255, 256, 32_767, 32_768, 65_535].freeze
        INTEREST_32 = [0, 1, 65_535, 65_536, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF].freeze
        INTEREST_64 = [0, 1, 0xFFFF_FFFF, 0x1_0000_0000, 0x7FFF_FFFF_FFFF_FFFF,
          0x8000_0000_0000_0000, 0xFFFF_FFFF_FFFF_FFFF].freeze
        INTEREST_BY_WIDTH = {1 => INTEREST_8, 2 => INTEREST_16, 4 => INTEREST_32, 8 => INTEREST_64}.freeze
        # Arithmetic deltas of section 3.3.
        DELTAS = [-35, -8, -1, 1, 8, 35].freeze

        PACK = {
          1 => {"little" => "C", "big" => "C"},
          2 => {"little" => "S<", "big" => "S>"},
          4 => {"little" => "L<", "big" => "L>"},
          8 => {"little" => "Q<", "big" => "Q>"}
        }.freeze

        # Raised when a mutate_sequence call would copy more than its work cap.
        class WorkExhausted < StandardError
          attr_reader :work

          def initialize(cap, work)
            super("mutate_sequence work cap of #{cap} bytes exceeded")
            @work = work
          end
        end

        def self.int!(value, name) = Limits.check_int!(value, name)

        def self.bytes_of(data, name)
          raise TypeError, "#{name} must be a String, not #{data.class}" unless data.is_a?(String)

          data.b # always a fresh copy
        end

        def self.endian_of(endian)
          text = endian.is_a?(Symbol) ? endian.to_s : endian
          return text if text == "little" || text == "big"

          raise ArgumentError, "endian must be 'little' or 'big'"
        end

        # Inverts `count` consecutive units (bits, unit 1, or bytes, unit 8) from
        # `offset` mod n, wrapping around the buffer.  Only the PARITY of each
        # unit's flips matters (XOR twice is a no-op): every unit is hit
        # `count / n` times and the first `count % n` (from the wrapped start) once
        # more, so the work is O(len) for ANY count where a loop of `count` single
        # flips would never end (VER06 section 9.2).
        #
        # The buffer is read as one little-endian Integer: bit i is bit (i % 8) of
        # byte (i / 8), LSB first, exactly how section 3.1 numbers bits.
        #
        # Worked example, 2 bytes = 16 bits, bit_flip(offset 0, count 17):
        # 17 / 16 = 1 (every bit once, odd) and 17 % 16 = 1 (bit 0 once more, so
        # twice, even) -> bits 1..15 inverted, bit 0 unchanged.
        def self.parity_flip(data, unit, offset, count)
          raw = bytes_of(data, "data")
          return raw if raw.empty? || count <= 0

          total_bits = raw.bytesize * 8
          units = total_bits / unit
          base = offset % units
          full, rem = count.divmod(units)
          value = raw.reverse.unpack1("H*").to_i(16)
          value ^= (1 << total_bits) - 1 if full.odd?
          if rem.positive?
            mask = ((1 << (rem * unit)) - 1) << (base * unit)
            wrapped = mask >> total_bits # the part past the end re-enters at bit 0
            value ^= (mask & ((1 << total_bits) - 1)) | wrapped
          end
          [value.to_s(16).rjust(raw.bytesize * 2, "0")].pack("H*").reverse
        end

        # Invert `count` consecutive bits from `bit_offset mod (8 * len)`.
        # bit_flip("\x00\x00", 9) is 00 02.
        def self.bit_flip(data, bit_offset, count = 1)
          int!(bit_offset, "bit_offset")
          int!(count, "count")
          parity_flip(data, 1, bit_offset, count)
        end

        # XOR `count` consecutive bytes with 0xFF from `byte_offset mod len`.
        def self.byte_flip(data, byte_offset, count = 1)
          int!(byte_offset, "byte_offset")
          int!(count, "count")
          parity_flip(data, 8, byte_offset, count)
        end

        # Add signed `delta` modulo 2**(8*width) to the integer at the offset.
        # u16 little-endian: ff 00 is 0x00ff; +1 gives 0x0100, stored 00 01.
        def self.byte_arithmetic(data, byte_offset, delta, width = 1, endian = "little")
          int!(byte_offset, "byte_offset")
          int!(delta, "delta")
          int!(width, "width")
          order = endian_of(endian)
          raw = bytes_of(data, "data")
          return raw unless INTEREST_BY_WIDTH.key?(width) && raw.bytesize >= width

          at = byte_offset % (raw.bytesize - width + 1)
          fmt = PACK.fetch(width).fetch(order)
          old = raw.byteslice(at, width).unpack1(fmt)
          raw[at, width] = [(old + delta) % (1 << (8 * width))].pack(fmt)
          raw
        end

        # Overwrite `width` bytes with `val mod 2**(8*width)` at the offset.
        # boundary_insert(00 00 00 00, 0, 4, 0x80000000, "little") is 00 00 00 80.
        def self.boundary_insert(data, byte_offset, width, val, endian = "little")
          int!(byte_offset, "byte_offset")
          int!(width, "width")
          int!(val, "val")
          order = endian_of(endian)
          raw = bytes_of(data, "data")
          return raw unless INTEREST_BY_WIDTH.key?(width) && raw.bytesize >= width

          at = byte_offset % (raw.bytesize - width + 1)
          raw[at, width] = [val % (1 << (8 * width))].pack(PACK.fetch(width).fetch(order))
          raw
        end

        # Delete up to `length` bytes at `offset mod len`; clamped at the end.
        def self.block_delete(data, offset, length)
          int!(offset, "offset")
          int!(length, "length")
          raw = bytes_of(data, "data")
          return raw if raw.empty? || length <= 0

          size = raw.bytesize
          at = offset % size
          raw.byteslice(0, at) + raw.byteslice(at + [length, size - at].min, size)
        end

        # Insert at `offset mod (len + 1)`; an empty input becomes the insert.
        def self.block_insert(data, offset, insert_bytes)
          int!(offset, "offset")
          raw = bytes_of(data, "data")
          ins = bytes_of(insert_bytes, "insert_bytes")
          return ins if raw.empty?

          at = offset % (raw.bytesize + 1)
          raw.byteslice(0, at) + ins + raw.byteslice(at, raw.bytesize)
        end

        # Copy up to `length` bytes from `src_offset mod len` to `dst_offset mod
        # (len + 1)` (the end of the buffer is a valid insertion point).
        def self.block_duplicate(data, src_offset, length, dst_offset)
          int!(src_offset, "src_offset")
          int!(length, "length")
          int!(dst_offset, "dst_offset")
          raw = bytes_of(data, "data")
          return raw if raw.empty? || length <= 0

          size = raw.bytesize
          src = src_offset % size
          chunk = raw.byteslice(src, [length, size - src].min)
          dst = dst_offset % (size + 1)
          raw.byteslice(0, dst) + chunk + raw.byteslice(dst, size)
        end

        # target[0...ts] + donor[ds..] with both splits clamped into their buffers.
        def self.splice(target, donor, target_split, donor_split)
          int!(target_split, "target_split")
          int!(donor_split, "donor_split")
          tgt = bytes_of(target, "target")
          don = bytes_of(donor, "donor")
          t = target_split.clamp(0, tgt.bytesize)
          d = donor_split.clamp(0, don.bytesize)
          tgt.byteslice(0, t) + don.byteslice(d, don.bytesize)
        end

        # `steps` seeded mutations, each chosen by one bounded(7) draw:
        #
        #   0 bit_flip, count 1            3 boundary_insert, width 1, INTEREST_8
        #   1 byte_flip, count 1           4 block_delete (len 1..n/2), skipped if n <= 1
        #   2 byte_arithmetic, DELTAS      5 block_insert of one random byte
        #                                  6 block_duplicate (len 1..n/2), skipped if n <= 1
        #
        # An empty buffer is refilled to 00 before every step.  The draw order is
        # the cross-lane contract.  `max_len` is VER06 section 3.5's length budget:
        # an insert or duplicate that would pass it is skipped AFTER its draws, so
        # the stream stays aligned and memory stays bounded.
        #
        # Returns [bytes, work] where work is the bytes copied (the initial copy
        # plus one buffer per step); raises WorkExhausted (carrying the work done)
        # rather than pass `work_cap`.
        def self.mutate_sequence_counted(data, seed, steps, max_len, work_cap)
          int!(seed, "seed")
          int!(steps, "steps")
          int!(max_len, "max_len")
          int!(work_cap, "work_cap")
          raise ArgumentError, "steps exceeds MAX_ITERATIONS (#{Limits::MAX_ITERATIONS})" if steps > Limits::MAX_ITERATIONS

          cur = bytes_of(data, "data")
          raise ArgumentError, "input exceeds MAX_INPUT_BYTES" if cur.bytesize > Limits::MAX_INPUT_BYTES

          work = cur.bytesize # the initial copy is work too
          raise WorkExhausted.new(work_cap, 0) if work > work_cap

          rng = Prng::SplitMix64.new(seed)
          [steps, 0].max.times do
            cur = "\x00".b if cur.empty?
            raise WorkExhausted.new(work_cap, work) if work + cur.bytesize > work_cap

            work += cur.bytesize
            n = cur.bytesize
            case rng.bounded(7)
            when 0 then cur = bit_flip(cur, rng.bounded(n * 8), 1)
            when 1 then cur = byte_flip(cur, rng.bounded(n), 1)
            when 2
              delta = DELTAS[rng.bounded(DELTAS.size)]
              cur = byte_arithmetic(cur, rng.bounded(n), delta, 1)
            when 3
              val = INTEREST_8[rng.bounded(INTEREST_8.size)]
              cur = boundary_insert(cur, rng.bounded(n), 1, val)
            when 4
              if n > 1
                length = 1 + rng.bounded([1, n / 2].max)
                cur = block_delete(cur, rng.bounded(n), length)
              end
            when 5
              ins = [rng.bounded(256)].pack("C")
              at = rng.bounded(n + 1)
              cur = block_insert(cur, at, ins) if n + 1 <= max_len
            else
              if n > 1
                length = 1 + rng.bounded([1, n / 2].max)
                src = rng.bounded(n)
                dst = rng.bounded(n + 1)
                cur = block_duplicate(cur, src, length, dst) if n + [length, n - src].min <= max_len
              end
            end
          end
          [cur, work]
        end

        def self.mutate_sequence(data, seed, steps, max_len = Limits::MAX_INPUT_BYTES)
          mutate_sequence_counted(data, seed, steps, max_len, Limits::MAX_SEQUENCE_WORK).first
        end
      end
    end
  end
end
