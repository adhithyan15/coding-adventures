# frozen_string_literal: true

module CodingAdventures
  module DeviceDriverFramework
    # DeviceType classifies hardware into three families, each with a different
    # data model and access pattern.
    #
    # Why three types? Because hardware naturally falls into three categories:
    #
    #   Type        Data Model           Examples                  Analogy
    #   ---------   ------------------   -----------------------   --------
    #   CHARACTER   Stream of bytes      Keyboard, serial, display A pipe
    #   BLOCK       Fixed-size chunks    Hard disk, SSD, USB       A filing cabinet
    #   NETWORK     Variable packets     Ethernet NIC, WiFi        A mailbox
    #
    # CHARACTER devices are sequential — bytes flow through like water in a pipe.
    # You cannot "seek" to byte 47 of a keyboard; data arrives when it arrives.
    #
    # BLOCK devices are random-access — you can read block 0, then block 9999,
    # then block 42, in any order. Every block is the same size. This is what
    # makes filesystems possible.
    #
    # NETWORK devices deal in packets — discrete messages with headers and
    # payloads. You send and receive complete packets, not individual bytes.
    module DeviceType
      CHARACTER = 0
      BLOCK = 1
      NETWORK = 2

      # Return a human-readable name for the device type.
      #
      # @param type [Integer] One of CHARACTER, BLOCK, or NETWORK
      # @return [String] The name of the device type
      def self.name_for(type)
        case type
        when CHARACTER then "Character"
        when BLOCK then "Block"
        when NETWORK then "Network"
        else "Unknown"
        end
      end
    end

    # Device is the base class for all device drivers. Every device — whether
    # it is a keyboard, a disk, or a network card — has these common attributes.
    #
    # Fields:
    #   name             — Human-readable identifier (e.g., "disk0")
    #   device_type      — CHARACTER, BLOCK, or NETWORK
    #   major            — Driver identifier (all devices sharing a driver share this)
    #   minor            — Instance identifier within the driver (first disk = 0)
    #   interrupt_number — Which interrupt this device raises (-1 if none)
    #   initialized      — Has init() been called?
    #
    # The major/minor number system comes from Unix. The kernel uses the major
    # number to find the right driver, and passes the minor number to the driver
    # so it knows which physical device to talk to.
    #
    # Example:
    #   Major 3 = disk driver
    #   Minor 0 = first disk (/dev/sda)
    #   Minor 1 = second disk (/dev/sdb)
    class Device
      attr_reader :name, :device_type, :major, :minor, :interrupt_number
      attr_accessor :initialized

      # @param name [String] Human-readable device name
      # @param device_type [Integer] One of DeviceType::CHARACTER, BLOCK, NETWORK
      # @param major [Integer] Driver identifier (major number)
      # @param minor [Integer] Instance identifier (minor number)
      # @param interrupt_number [Integer] Interrupt number, or -1 for none
      def initialize(name:, device_type:, major:, minor:, interrupt_number: -1)
        @name = name
        @device_type = device_type
        @major = major
        @minor = minor
        @interrupt_number = interrupt_number
        @initialized = false
      end

      # Initialize the device. Subclasses override this to perform hardware-
      # specific setup (clearing buffers, resetting state, etc.).
      #
      # Must be called before the device is registered or used.
      def init
        @initialized = true
      end

      # Return a string representation of the device for debugging.
      def to_s
        type_name = DeviceType.name_for(@device_type)
        "#{@name} (#{type_name}, major=#{@major}, minor=#{@minor}, irq=#{@interrupt_number})"
      end
    end

    # CharacterDevice — a device that produces or consumes a stream of bytes.
    #
    # Character devices are sequential: bytes flow through one at a time, like
    # water through a pipe. You read whatever is available and write whatever
    # you have. There is no concept of "seeking" to a position.
    #
    # Examples: keyboard (produces bytes when keys are pressed), serial port
    # (sends/receives bytes over a wire), display terminal (consumes bytes
    # and renders them as characters on screen).
    #
    # Subclasses must implement:
    #   read(count)  — Read up to `count` bytes. Returns an array of bytes.
    #                  Returns empty array if no data available.
    #   write(data)  — Write bytes to the device. Returns bytes written, or -1.
    class CharacterDevice < Device
      def initialize(name:, major:, minor:, interrupt_number: -1)
        super(
          name: name,
          device_type: DeviceType::CHARACTER,
          major: major,
          minor: minor,
          interrupt_number: interrupt_number
        )
      end

      # Read up to `count` bytes from the device.
      #
      # @param count [Integer] Maximum number of bytes to read
      # @return [Array<Integer>] The bytes read (may be fewer than requested)
      def read(count)
        raise NotImplementedError, "#{self.class}#read must be implemented by subclass"
      end

      # Write bytes to the device.
      #
      # @param data [Array<Integer>] The bytes to write
      # @return [Integer] Number of bytes written, or -1 on error
      def write(data)
        raise NotImplementedError, "#{self.class}#write must be implemented by subclass"
      end
    end

    # BlockDevice — a device that reads and writes fixed-size blocks.
    #
    # Block devices are random-access: you can read any block in any order,
    # like pulling drawers out of a filing cabinet. Every block is the same
    # size (typically 512 bytes, the standard sector size since the IBM PC/AT
    # in 1984).
    #
    # Why whole blocks? Physical disks read whole sectors at a time. Even if
    # you only want 1 byte, the disk reads 512. The OS caches the extra bytes
    # for later. This is why filesystems exist — to manage partial-block
    # reads and writes efficiently.
    #
    # Subclasses must implement:
    #   read_block(block_number)        — Returns exactly block_size bytes
    #   write_block(block_number, data) — Writes exactly block_size bytes
    class BlockDevice < Device
      attr_reader :block_size, :total_blocks

      # @param name [String] Device name
      # @param major [Integer] Major number
      # @param minor [Integer] Minor number
      # @param interrupt_number [Integer] Interrupt number (-1 if none)
      # @param block_size [Integer] Bytes per block (default 512)
      # @param total_blocks [Integer] Total number of blocks
      def initialize(name:, major:, minor:, interrupt_number: -1, block_size: 512, total_blocks:)
        super(
          name: name,
          device_type: DeviceType::BLOCK,
          major: major,
          minor: minor,
          interrupt_number: interrupt_number
        )
        @block_size = block_size
        @total_blocks = total_blocks
      end

      # Read one block from the device.
      #
      # @param block_number [Integer] Which block to read (0-based)
      # @return [Array<Integer>] Exactly block_size bytes
      def read_block(block_number)
        raise NotImplementedError, "#{self.class}#read_block must be implemented by subclass"
      end

      # Write one block to the device.
      #
      # @param block_number [Integer] Which block to write (0-based)
      # @param data [Array<Integer>] Exactly block_size bytes
      def write_block(block_number, data)
        raise NotImplementedError, "#{self.class}#write_block must be implemented by subclass"
      end
    end

    # NetworkDevice — a device that sends and receives variable-length packets.
    #
    # Network devices deal in packets — discrete messages with headers,
    # addresses, and payloads. Unlike character devices (continuous byte
    # streams) or block devices (fixed-size chunks), network packets can be
    # any size up to the maximum transmission unit (MTU).
    #
    # Every network device has a MAC address — a 6-byte unique identifier,
    # like a mailing address for the network card. In real hardware, this is
    # burned into the NIC at the factory. In simulation, we assign it at
    # creation time.
    #
    # Subclasses must implement:
    #   send_packet(data)   — Send a packet, returns bytes sent or -1
    #   receive_packet()    — Receive next packet, or nil if none available
    #   has_packet?()       — True if a packet is waiting
    class NetworkDevice < Device
      attr_reader :mac_address

      # @param name [String] Device name
      # @param major [Integer] Major number
      # @param minor [Integer] Minor number
      # @param interrupt_number [Integer] Interrupt number (-1 if none)
      # @param mac_address [Array<Integer>] 6-byte MAC address
      def initialize(name:, major:, minor:, interrupt_number: -1, mac_address:)
        super(
          name: name,
          device_type: DeviceType::NETWORK,
          major: major,
          minor: minor,
          interrupt_number: interrupt_number
        )
        raise ArgumentError, "MAC address must be exactly 6 bytes" unless mac_address.length == 6

        @mac_address = mac_address.dup.freeze
      end

      # Send a packet over the network.
      #
      # @param data [Array<Integer>] The packet data
      # @return [Integer] Bytes sent, or -1 on error
      def send_packet(data)
        raise NotImplementedError, "#{self.class}#send_packet must be implemented by subclass"
      end

      # Receive the next packet from the network.
      #
      # Non-blocking: returns nil immediately if no packet is available.
      #
      # @return [Array<Integer>, nil] Packet data, or nil if no packet waiting
      def receive_packet
        raise NotImplementedError, "#{self.class}#receive_packet must be implemented by subclass"
      end

      # Check whether a packet is waiting to be received.
      #
      # @return [Boolean] true if a packet is available
      def has_packet?
        raise NotImplementedError, "#{self.class}#has_packet? must be implemented by subclass"
      end
    end
  end
end
