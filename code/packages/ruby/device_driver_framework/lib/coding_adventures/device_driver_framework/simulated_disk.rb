# frozen_string_literal: true

module CodingAdventures
  module DeviceDriverFramework
    # SimulatedDisk — an in-memory block storage device.
    #
    # This is the "hard drive" for our simulated computer. Instead of magnetic
    # platters spinning at 7200 RPM or NAND flash cells storing charge, we use
    # a plain Ruby array of bytes. The interface is identical to what a real
    # disk driver would provide.
    #
    # How it works:
    #   - The disk is divided into fixed-size blocks (default 512 bytes each).
    #   - A 1 MB disk has 2048 blocks (2048 * 512 = 1,048,576 bytes).
    #   - Each block can be read or written independently (random access).
    #   - The backing store is a flat byte array; block N starts at offset
    #     N * block_size.
    #
    # Why 512 bytes per block? This is the standard sector size since the
    # IBM PC/AT in 1984. Modern disks use 4096-byte sectors, but 512 is
    # simpler and matches the classic teaching examples.
    #
    # Example:
    #   disk = SimulatedDisk.new(total_blocks: 2048)
    #   disk.init
    #   disk.write_block(5, [0x48, 0x65, 0x6C, 0x6C, 0x6F] + [0] * 507)
    #   data = disk.read_block(5)  # => the 512 bytes we just wrote
    class SimulatedDisk < BlockDevice
      # @param name [String] Device name (default "disk0")
      # @param minor [Integer] Minor number (default 0)
      # @param total_blocks [Integer] Number of blocks (default 2048 = 1 MB)
      # @param block_size [Integer] Bytes per block (default 512)
      def initialize(name: "disk0", minor: 0, total_blocks: 2048, block_size: 512)
        super(
          name: name,
          major: 3,
          minor: minor,
          interrupt_number: 34,
          block_size: block_size,
          total_blocks: total_blocks
        )
        # The backing store is lazily allocated in init() to mirror real
        # hardware behavior: the disk is not usable until the driver
        # initializes it.
        @storage = nil
      end

      # Initialize the disk by allocating the backing byte array.
      #
      # In real hardware, init() would send reset commands to the disk
      # controller, wait for the drive to spin up, and read the partition
      # table. For our simulation, we just allocate the memory.
      def init
        super
        @storage = Array.new(block_size * total_blocks, 0)
      end

      # Read one block from the disk.
      #
      # The math is straightforward:
      #   offset = block_number * block_size
      #   return storage[offset ... offset + block_size]
      #
      # @param block_number [Integer] Which block to read (0-based)
      # @return [Array<Integer>] Exactly block_size bytes
      # @raise [ArgumentError] If block_number is out of range
      def read_block(block_number)
        validate_block_number!(block_number)
        offset = block_number * block_size
        @storage[offset, block_size].dup
      end

      # Write one block to the disk.
      #
      # @param block_number [Integer] Which block to write (0-based)
      # @param data [Array<Integer>] Must be exactly block_size bytes
      # @raise [ArgumentError] If block_number is out of range
      # @raise [ArgumentError] If data is not exactly block_size bytes
      def write_block(block_number, data)
        validate_block_number!(block_number)

        unless data.length == block_size
          raise ArgumentError,
            "Data must be exactly #{block_size} bytes, got #{data.length}"
        end

        offset = block_number * block_size
        data.each_with_index do |byte, i|
          @storage[offset + i] = byte
        end
      end

      private

      # Validate that a block number is within the valid range.
      #
      # @param block_number [Integer] The block number to validate
      # @raise [ArgumentError] If out of range
      def validate_block_number!(block_number)
        if block_number < 0 || block_number >= total_blocks
          raise ArgumentError,
            "Block number #{block_number} out of range (0..#{total_blocks - 1})"
        end
      end
    end
  end
end
