# frozen_string_literal: true

module CodingAdventures
  module DeviceDriverFramework
    # SimulatedKeyboard — a character device backed by an internal byte buffer.
    #
    # In a real computer, pressing a key sends a scan code to the keyboard
    # controller, which raises interrupt 33. The keyboard ISR reads the scan
    # code, translates it to an ASCII value, and deposits it into a buffer.
    # When a program calls read(), it gets bytes from that buffer.
    #
    # Our SimulatedKeyboard skips the hardware part and lets you push bytes
    # directly into the buffer (via enqueue_bytes), simulating what the ISR
    # would do. The read() method then pulls bytes out in FIFO order.
    #
    # Why is write() not supported? A keyboard is an input-only device.
    # You cannot "write" to a keyboard — it does not have a display or
    # output mechanism. Calling write() returns -1 to indicate an error,
    # following the Unix convention.
    #
    # Example:
    #   kb = SimulatedKeyboard.new
    #   kb.init
    #   kb.enqueue_bytes([0x48, 0x69])  # Simulate typing "Hi"
    #   kb.read(2)  # => [0x48, 0x69]
    class SimulatedKeyboard < CharacterDevice
      # @param name [String] Device name (default "keyboard0")
      # @param minor [Integer] Minor number (default 0)
      def initialize(name: "keyboard0", minor: 0)
        super(
          name: name,
          major: 2,
          minor: minor,
          interrupt_number: 33
        )
        @buffer = []
      end

      # Initialize the keyboard by clearing any stale data in the buffer.
      def init
        super
        @buffer.clear
      end

      # Read up to `count` bytes from the keyboard buffer.
      #
      # Returns whatever bytes are available, up to `count`. If the buffer
      # is empty, returns an empty array (non-blocking). This mirrors real
      # keyboard behavior: if no keys have been pressed, there is nothing
      # to read.
      #
      # @param count [Integer] Maximum number of bytes to read
      # @return [Array<Integer>] Bytes read (may be fewer than count)
      def read(count)
        result = []
        count.times do
          break if @buffer.empty?
          result << @buffer.shift
        end
        result
      end

      # Write to the keyboard — always fails.
      #
      # Keyboards are input-only devices. In Unix, attempting to write to
      # a read-only device returns an error code. We follow that convention
      # by returning -1.
      #
      # @param _data [Array<Integer>] Ignored
      # @return [Integer] Always -1 (error)
      def write(_data)
        -1
      end

      # Simulate keystrokes by pushing bytes into the keyboard buffer.
      #
      # In a real system, the keyboard ISR (interrupt service routine) does
      # this when interrupt 33 fires. The ISR reads the scan code from the
      # keyboard controller hardware, translates it to ASCII, and deposits
      # it here.
      #
      # @param bytes [Array<Integer>] Bytes to enqueue
      def enqueue_bytes(bytes)
        @buffer.concat(bytes)
      end

      # Check how many bytes are waiting in the buffer.
      #
      # @return [Integer] Number of buffered bytes
      def buffer_size
        @buffer.length
      end
    end
  end
end
