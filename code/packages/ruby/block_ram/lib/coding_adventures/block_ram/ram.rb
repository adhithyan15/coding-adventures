# frozen_string_literal: true

# ---------------------------------------------------------------------------
# RAM Modules -- synchronous memory with read/write ports.
# ---------------------------------------------------------------------------
#
# === From Array to Module ===
#
# An SRAM array (sram.rb) provides raw row-level read/write. A RAM module
# adds the interface that digital circuits actually use:
#
# 1. **Address decoding** -- binary address bits select a row
# 2. **Synchronous operation** -- reads and writes happen on clock edges
# 3. **Read modes** -- what the output shows during a write operation
# 4. **Dual-port access** -- two independent ports for simultaneous operations
#
# === Read Modes ===
#
# During a write operation, what should the data output show? There are
# three valid answers:
#
# 1. **Read-first**: Output shows the OLD value at the address being written.
#    The read happens before the write within the same cycle.
#
# 2. **Write-first** (read-after-write): Output shows the NEW value being
#    written. The write happens first, then the read sees the new value.
#
# 3. **No-change**: Output retains its previous value during writes. This
#    saves power in FPGA Block RAMs.
#
# === Dual-Port RAM ===
#
# Two completely independent ports (A and B), each with its own address,
# data, and write enable. Both can operate simultaneously:
# - Read A + Read B at different addresses -> both get their data
# - Write A + Read B at different addresses -> both succeed
# - Write A + Write B at the SAME address -> collision (we raise an error)
# ---------------------------------------------------------------------------

module CodingAdventures
  module BlockRam
    # Read mode enumeration -- controls what data_out shows during a write.
    #
    # READ_FIRST:  data_out = old value (read before write)
    # WRITE_FIRST: data_out = new value (write before read)
    # NO_CHANGE:   data_out = previous read value (output unchanged)
    module ReadMode
      READ_FIRST = :read_first
      WRITE_FIRST = :write_first
      NO_CHANGE = :no_change
    end

    # Raised when both ports of a dual-port RAM write to the same address.
    #
    # In real hardware, simultaneous writes to the same address produce
    # undefined results. We detect this and raise an error to prevent
    # silent bugs.
    class WriteCollisionError < StandardError
      attr_reader :address

      def initialize(address)
        @address = address
        super("Write collision: both ports writing to address #{address}")
      end
    end

    # Single-port synchronous RAM.
    #
    # One address port, one data bus. Each clock cycle you can do ONE
    # operation: read OR write (controlled by write_enable).
    #
    # Interface:
    #
    #                 +-----------------------------+
    #   address ------+                             |
    #                 |     Single-Port RAM         +---- data_out
    #   data_in ------+                             |
    #                 |     (depth x width)         |
    #   write_en -----+                             |
    #                 |                             |
    #   clock --------+                             |
    #                 +-----------------------------+
    #
    # Operations happen on the rising edge of the clock (transition 0->1).
    #
    # @example
    #   ram = SinglePortRAM.new(depth: 256, width: 8)
    #   ram.tick(0, address: 0, data_in: [1]*8, write_enable: 1)
    #   out = ram.tick(1, address: 0, data_in: [1]*8, write_enable: 1)
    #   ram.tick(0, address: 0, data_in: [0]*8, write_enable: 0)
    #   out = ram.tick(1, address: 0, data_in: [0]*8, write_enable: 0)
    #   # out => [1, 1, 1, 1, 1, 1, 1, 1]
    class SinglePortRAM
      attr_reader :depth, :width

      # @param depth [Integer] number of addressable words (>= 1)
      # @param width [Integer] bits per word (>= 1)
      # @param read_mode [Symbol] what data_out shows during writes
      def initialize(depth:, width:, read_mode: ReadMode::READ_FIRST)
        if depth < 1
          raise ArgumentError, "depth must be >= 1, got #{depth}"
        end
        if width < 1
          raise ArgumentError, "width must be >= 1, got #{width}"
        end

        @depth = depth
        @width = width
        @read_mode = read_mode
        @array = SRAMArray.new(depth, width)
        @prev_clock = 0
        @last_read = Array.new(width, 0)
      end

      # Execute one half-cycle. Operations happen on rising edge (0->1).
      #
      # @param clock [Integer] clock signal (0 or 1)
      # @param address [Integer] word address (0 to depth-1)
      # @param data_in [Array<Integer>] data to write (list of width bits)
      # @param write_enable [Integer] 0 = read, 1 = write
      # @return [Array<Integer>] data_out (list of width bits)
      def tick(clock, address:, data_in:, write_enable:)
        BlockRam.validate_bit(clock, "clock")
        BlockRam.validate_bit(write_enable, "write_enable")
        validate_address(address)
        validate_data(data_in)

        # Detect rising edge
        rising_edge = @prev_clock == 0 && clock == 1
        @prev_clock = clock

        return @last_read.dup unless rising_edge

        # Rising edge: perform the operation
        if write_enable == 0
          # Read operation
          @last_read = @array.read(address)
          return @last_read.dup
        end

        # Write operation -- behavior depends on read mode
        case @read_mode
        when ReadMode::READ_FIRST
          @last_read = @array.read(address)
          @array.write(address, data_in)
          @last_read.dup
        when ReadMode::WRITE_FIRST
          @array.write(address, data_in)
          @last_read = data_in.dup
          @last_read.dup
        else # NO_CHANGE
          @array.write(address, data_in)
          @last_read.dup
        end
      end

      # Return all contents for inspection.
      #
      # @return [Array<Array<Integer>>] list of rows, each row is a list of bits
      def dump
        @depth.times.map { |row| @array.read(row) }
      end

      private

      def validate_address(address)
        unless address.is_a?(Integer) && !address.is_a?(TrueClass) && !address.is_a?(FalseClass)
          raise TypeError, "address must be an Integer, got #{address.class}"
        end
        if address < 0 || address >= @depth
          raise ArgumentError, "address #{address} out of range [0, #{@depth - 1}]"
        end
      end

      def validate_data(data_in)
        unless data_in.is_a?(Array)
          raise TypeError, "data_in must be an Array of bits"
        end
        if data_in.length != @width
          raise ArgumentError,
            "data_in length #{data_in.length} does not match width #{@width}"
        end
        data_in.each_with_index { |bit, i| BlockRam.validate_bit(bit, "data_in[#{i}]") }
      end
    end

    # True dual-port synchronous RAM.
    #
    # Two independent ports (A and B), each with its own address, data,
    # and write enable. Both ports can operate simultaneously on different
    # addresses.
    #
    # Write collision: if both ports write to the same address in the
    # same cycle, a WriteCollisionError is raised.
    #
    # @example
    #   ram = DualPortRAM.new(depth: 256, width: 8)
    #   # Write via port A, read via port B
    class DualPortRAM
      attr_reader :depth, :width

      # @param depth [Integer] number of addressable words (>= 1)
      # @param width [Integer] bits per word (>= 1)
      # @param read_mode_a [Symbol] read mode for port A
      # @param read_mode_b [Symbol] read mode for port B
      def initialize(depth:, width:, read_mode_a: ReadMode::READ_FIRST, read_mode_b: ReadMode::READ_FIRST)
        if depth < 1
          raise ArgumentError, "depth must be >= 1, got #{depth}"
        end
        if width < 1
          raise ArgumentError, "width must be >= 1, got #{width}"
        end

        @depth = depth
        @width = width
        @read_mode_a = read_mode_a
        @read_mode_b = read_mode_b
        @array = SRAMArray.new(depth, width)
        @prev_clock = 0
        @last_read_a = Array.new(width, 0)
        @last_read_b = Array.new(width, 0)
      end

      # Execute one half-cycle on both ports.
      #
      # @param clock [Integer] clock signal (0 or 1)
      # @param address_a [Integer] port A word address
      # @param data_in_a [Array<Integer>] port A write data
      # @param write_enable_a [Integer] port A write enable
      # @param address_b [Integer] port B word address
      # @param data_in_b [Array<Integer>] port B write data
      # @param write_enable_b [Integer] port B write enable
      # @return [Array(Array<Integer>, Array<Integer>)] [data_out_a, data_out_b]
      # @raise [WriteCollisionError] if both ports write to same address
      def tick(clock, address_a:, data_in_a:, write_enable_a:,
        address_b:, data_in_b:, write_enable_b:)
        BlockRam.validate_bit(clock, "clock")
        BlockRam.validate_bit(write_enable_a, "write_enable_a")
        BlockRam.validate_bit(write_enable_b, "write_enable_b")
        validate_address(address_a, "address_a")
        validate_address(address_b, "address_b")
        validate_data(data_in_a, "data_in_a")
        validate_data(data_in_b, "data_in_b")

        rising_edge = @prev_clock == 0 && clock == 1
        @prev_clock = clock

        unless rising_edge
          return [@last_read_a.dup, @last_read_b.dup]
        end

        # Check for write collision
        if write_enable_a == 1 && write_enable_b == 1 && address_a == address_b
          raise WriteCollisionError, address_a
        end

        # Process port A
        @last_read_a = process_port(address_a, data_in_a, write_enable_a,
          @read_mode_a, @last_read_a)

        # Process port B
        @last_read_b = process_port(address_b, data_in_b, write_enable_b,
          @read_mode_b, @last_read_b)

        [@last_read_a.dup, @last_read_b.dup]
      end

      private

      def process_port(address, data_in, write_enable, read_mode, last_read)
        if write_enable == 0
          return @array.read(address)
        end

        case read_mode
        when ReadMode::READ_FIRST
          result = @array.read(address)
          @array.write(address, data_in)
          result
        when ReadMode::WRITE_FIRST
          @array.write(address, data_in)
          data_in.dup
        else # NO_CHANGE
          @array.write(address, data_in)
          last_read.dup
        end
      end

      def validate_address(address, name = "address")
        unless address.is_a?(Integer) && !address.is_a?(TrueClass) && !address.is_a?(FalseClass)
          raise TypeError, "#{name} must be an Integer, got #{address.class}"
        end
        if address < 0 || address >= @depth
          raise ArgumentError, "#{name} #{address} out of range [0, #{@depth - 1}]"
        end
      end

      def validate_data(data_in, name = "data_in")
        unless data_in.is_a?(Array)
          raise TypeError, "#{name} must be an Array of bits"
        end
        if data_in.length != @width
          raise ArgumentError,
            "#{name} length #{data_in.length} does not match width #{@width}"
        end
        data_in.each_with_index { |bit, i| BlockRam.validate_bit(bit, "#{name}[#{i}]") }
      end
    end
  end
end
