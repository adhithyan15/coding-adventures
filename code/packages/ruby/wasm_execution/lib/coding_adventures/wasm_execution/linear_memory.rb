# frozen_string_literal: true

# ==========================================================================
# LinearMemory --- WASM Linear Memory Implementation
# ==========================================================================
#
# WebAssembly's memory model is a contiguous, byte-addressable array
# called "linear memory". Think of it as a flat C-style heap: you can
# read and write individual bytes, 16-bit words, 32-bit words, or
# 64-bit words at any byte offset within the allocated region.
#
# Linear memory is measured in "pages", where each page is exactly
# 65,536 bytes (64 KiB). The `memory.grow` instruction can add pages
# at runtime (up to a declared maximum).
#
# Memory accesses are bounds-checked: reading or writing past the end
# causes a trap (TrapError).
#
#   +-------------------------------------------------------------------+
#   |  Page 0 (0x00000 - 0x0FFFF)  |  Page 1 (0x10000 - 0x1FFFF)  |...|
#   +-------------------------------------------------------------------+
#   ^                                                                   ^
#   byte 0                                                   last allocated byte
#
# WASM always uses little-endian byte order.
# ==========================================================================

module CodingAdventures
  module WasmExecution
    class LinearMemory
      # Bytes per WASM memory page: 64 KiB.
      PAGE_SIZE = 65536

      # @param initial_pages [Integer] number of pages to allocate initially
      # @param max_pages [Integer, nil] optional upper bound on page count
      def initialize(initial_pages, max_pages = nil)
        @current_pages = initial_pages
        @max_pages = max_pages
        @buffer = "\x00".b * (initial_pages * PAGE_SIZE)
      end

      # ── Bounds Checking ────────────────────────────────────────────

      private def bounds_check(offset, width)
        if offset < 0 || offset + width > @buffer.bytesize
          raise TrapError,
                "Out of bounds memory access: offset=#{offset}, size=#{width}, " \
                "memory size=#{@buffer.bytesize}"
        end
      end

      # ── Full-Width Loads ───────────────────────────────────────────

      # Load 4 bytes as a signed 32-bit integer (little-endian).
      def load_i32(offset)
        bounds_check(offset, 4)
        @buffer.byteslice(offset, 4).unpack1("l<")
      end

      # Load 8 bytes as a signed 64-bit integer (little-endian).
      def load_i64(offset)
        bounds_check(offset, 8)
        @buffer.byteslice(offset, 8).unpack1("q<")
      end

      # Load 4 bytes as a 32-bit float (little-endian).
      def load_f32(offset)
        bounds_check(offset, 4)
        @buffer.byteslice(offset, 4).unpack1("e")
      end

      # Load 8 bytes as a 64-bit float (little-endian).
      def load_f64(offset)
        bounds_check(offset, 8)
        @buffer.byteslice(offset, 8).unpack1("E")
      end

      # ── Narrow Loads for i32 ─��─────────────────────────────────────

      # Load 1 byte, sign-extend to i32.
      def load_i32_8s(offset)
        bounds_check(offset, 1)
        @buffer.byteslice(offset, 1).unpack1("c")
      end

      # Load 1 byte, zero-extend to i32.
      def load_i32_8u(offset)
        bounds_check(offset, 1)
        @buffer.byteslice(offset, 1).unpack1("C")
      end

      # Load 2 bytes (little-endian), sign-extend to i32.
      def load_i32_16s(offset)
        bounds_check(offset, 2)
        @buffer.byteslice(offset, 2).unpack1("s<")
      end

      # Load 2 bytes (little-endian), zero-extend to i32.
      def load_i32_16u(offset)
        bounds_check(offset, 2)
        @buffer.byteslice(offset, 2).unpack1("v")
      end

      # ── Narrow Loads for i64 ───────────────────────────────────────

      def load_i64_8s(offset)
        bounds_check(offset, 1)
        @buffer.byteslice(offset, 1).unpack1("c").to_i
      end

      def load_i64_8u(offset)
        bounds_check(offset, 1)
        @buffer.byteslice(offset, 1).unpack1("C").to_i
      end

      def load_i64_16s(offset)
        bounds_check(offset, 2)
        @buffer.byteslice(offset, 2).unpack1("s<").to_i
      end

      def load_i64_16u(offset)
        bounds_check(offset, 2)
        @buffer.byteslice(offset, 2).unpack1("v").to_i
      end

      def load_i64_32s(offset)
        bounds_check(offset, 4)
        @buffer.byteslice(offset, 4).unpack1("l<").to_i
      end

      def load_i64_32u(offset)
        bounds_check(offset, 4)
        @buffer.byteslice(offset, 4).unpack1("V").to_i
      end

      # ── Full-Width Stores ──────────────────────────────────────────

      def store_i32(offset, value)
        bounds_check(offset, 4)
        @buffer[offset, 4] = [value].pack("l<")
      end

      def store_i64(offset, value)
        bounds_check(offset, 8)
        @buffer[offset, 8] = [value].pack("q<")
      end

      def store_f32(offset, value)
        bounds_check(offset, 4)
        @buffer[offset, 4] = [value].pack("e")
      end

      def store_f64(offset, value)
        bounds_check(offset, 8)
        @buffer[offset, 8] = [value].pack("E")
      end

      # ── Narrow Stores ──────────────────────────────────────────────

      def store_i32_8(offset, value)
        bounds_check(offset, 1)
        @buffer[offset, 1] = [value].pack("c")
      end

      def store_i32_16(offset, value)
        bounds_check(offset, 2)
        @buffer[offset, 2] = [value].pack("s<")
      end

      def store_i64_8(offset, value)
        bounds_check(offset, 1)
        @buffer[offset, 1] = [value & 0xFF].pack("C")
      end

      def store_i64_16(offset, value)
        bounds_check(offset, 2)
        @buffer[offset, 2] = [value & 0xFFFF].pack("v")
      end

      def store_i64_32(offset, value)
        bounds_check(offset, 4)
        @buffer[offset, 4] = [value & 0xFFFFFFFF].pack("V")
      end

      # ── Memory Growth ──────────────────────────────────────────────

      # Grow memory by +delta_pages+ pages.
      # Returns the old page count on success, or -1 on failure.
      def grow(delta_pages)
        old_pages = @current_pages
        new_pages = old_pages + delta_pages

        return -1 if @max_pages && new_pages > @max_pages
        return -1 if new_pages > 65536

        @buffer << ("\x00".b * (delta_pages * PAGE_SIZE))
        @current_pages = new_pages
        old_pages
      end

      # ── Size Queries ��──────────────────────────────────────────────

      # Return current memory size in pages.
      def page_count
        @current_pages
      end

      # Return current memory size in bytes.
      def byte_length
        @buffer.bytesize
      end

      # ── Raw Byte Access ────────────────────────────────────────────

      # Write raw bytes into memory at the given offset.
      # Used during instantiation to copy data segments.
      def write_bytes(offset, data)
        bytes = data.is_a?(String) ? data : data.pack("C*")
        bounds_check(offset, bytes.bytesize)
        @buffer[offset, bytes.bytesize] = bytes
      end
    end
  end
end
