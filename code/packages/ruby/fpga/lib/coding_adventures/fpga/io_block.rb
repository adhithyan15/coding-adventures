# frozen_string_literal: true

# ---------------------------------------------------------------------------
# I/O Block -- bidirectional pad connecting FPGA internals to the outside.
# ---------------------------------------------------------------------------
#
# === What is an I/O Block? ===
#
# I/O blocks sit at the perimeter of the FPGA and provide the interface
# between the internal logic fabric and the external pins of the chip.
#
# Each I/O block can be configured in three modes:
# - **Input**: External signal enters the FPGA (pad -> internal)
# - **Output**: Internal signal exits the FPGA (internal -> pad)
# - **Tri-state**: Output is high-impedance (disconnected) when not enabled
#
# === I/O Block Architecture ===
#
#     External Pin (pad)
#          |
#          v
#     +-------------------+
#     |    I/O Block       |
#     |                    |
#     |  +---------------+ |
#     |  | Tri-State     | | -- output enable controls direction
#     |  | Buffer        | |
#     |  +-------+-------+ |
#     |          |          |
#     +-------------------+
#          |
#          v
#     To/From Internal Fabric
#
# The tri-state buffer uses the tri_state function from logic-gates
# to produce 0, 1, or nil (high-impedance).
# ---------------------------------------------------------------------------

module CodingAdventures
  module FPGA
    # I/O block operating mode.
    #
    # INPUT:     Pad drives internal signal (external -> fabric)
    # OUTPUT:    Fabric drives pad (fabric -> external)
    # TRISTATE:  Output is high-impedance (pad is disconnected)
    module IOMode
      INPUT = :input
      OUTPUT = :output
      TRISTATE = :tristate
    end

    # Bidirectional I/O pad for the FPGA perimeter.
    #
    # Each I/O block connects one external pin to the internal fabric.
    # The mode determines the direction of data flow.
    #
    # @example input pin
    #   io = IOBlock.new("sensor_in", mode: IOMode::INPUT)
    #   io.drive_pad(1)
    #   io.read_internal  # => 1
    #
    # @example output pin
    #   io = IOBlock.new("led_0", mode: IOMode::OUTPUT)
    #   io.drive_internal(1)
    #   io.read_pad  # => 1
    #
    # @example tri-state (disconnected)
    #   io = IOBlock.new("bus_0", mode: IOMode::TRISTATE)
    #   io.drive_internal(1)
    #   io.read_pad  # => nil (high impedance)
    class IOBlock
      attr_reader :name, :mode

      # @param name [String] identifier for this I/O block
      # @param mode [Symbol] initial operating mode (default: IOMode::INPUT)
      def initialize(name, mode: IOMode::INPUT)
        unless name.is_a?(String) && !name.empty?
          raise ArgumentError, "name must be a non-empty string"
        end

        @name = name
        @mode = mode
        @pad_value = 0       # Signal on the external pad
        @internal_value = 0  # Signal on the fabric side
      end

      # Change the I/O block's operating mode.
      #
      # @param mode [Symbol] new operating mode
      def configure(mode)
        unless [IOMode::INPUT, IOMode::OUTPUT, IOMode::TRISTATE].include?(mode)
          raise ArgumentError, "mode must be IOMode::INPUT, OUTPUT, or TRISTATE, got #{mode.inspect}"
        end
        @mode = mode
      end

      # Drive the external pad with a signal (used in INPUT mode).
      #
      # @param value [Integer] signal value (0 or 1)
      def drive_pad(value)
        unless value == 0 || value == 1
          raise ArgumentError, "value must be 0 or 1, got #{value}"
        end
        @pad_value = value
      end

      # Drive the internal (fabric) side with a signal (used in OUTPUT mode).
      #
      # @param value [Integer] signal value (0 or 1)
      def drive_internal(value)
        unless value == 0 || value == 1
          raise ArgumentError, "value must be 0 or 1, got #{value}"
        end
        @internal_value = value
      end

      # Read the signal visible to the internal fabric.
      #
      # In INPUT mode, returns the pad value (external -> fabric).
      # In OUTPUT/TRISTATE mode, returns the internally driven value.
      #
      # @return [Integer] signal value (0 or 1)
      def read_internal
        if @mode == IOMode::INPUT
          @pad_value
        else
          @internal_value
        end
      end

      # Read the signal visible on the external pad.
      #
      # In INPUT mode, returns the pad value.
      # In OUTPUT mode, returns the internally driven value.
      # In TRISTATE mode, returns nil (high impedance).
      #
      # @return [Integer, nil] 0 or 1 in INPUT/OUTPUT mode, nil in TRISTATE
      def read_pad
        case @mode
        when IOMode::INPUT
          @pad_value
        when IOMode::TRISTATE
          LogicGates::Combinational.tri_state(@internal_value, 0)
        else # OUTPUT
          LogicGates::Combinational.tri_state(@internal_value, 1)
        end
      end
    end
  end
end
