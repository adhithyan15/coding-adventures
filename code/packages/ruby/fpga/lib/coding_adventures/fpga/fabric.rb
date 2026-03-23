# frozen_string_literal: true

# ---------------------------------------------------------------------------
# FPGA Fabric -- the top-level FPGA model.
# ---------------------------------------------------------------------------
#
# === What is an FPGA? ===
#
# An FPGA (Field-Programmable Gate Array) is a chip containing:
# - A grid of CLBs (Configurable Logic Blocks) for computation
# - A routing fabric (switch matrices) for interconnection
# - I/O blocks at the perimeter for external connections
# - Block RAM tiles for on-chip memory
#
# The key property: **all of this is programmable**. By loading a
# bitstream (configuration data), the same physical chip can become
# any digital circuit.
#
# === Our FPGA Model ===
#
#     +----------------------------------------------------+
#     |                    FPGA Fabric                      |
#     |                                                     |
#     |  [IO] [IO] [IO] [IO] [IO] [IO] [IO] [IO]          |
#     |                                                     |
#     |  [IO] [CLB]--[SW]--[CLB]--[SW]--[CLB] [IO]        |
#     |         |            |            |                 |
#     |        [SW]         [SW]         [SW]               |
#     |         |            |            |                 |
#     |  [IO] [CLB]--[SW]--[CLB]--[SW]--[CLB] [IO]        |
#     |                                                     |
#     |  [IO] [IO] [IO] [IO] [IO] [IO] [IO] [IO]          |
#     |                                                     |
#     |            [BRAM]        [BRAM]                     |
#     +----------------------------------------------------+
#
# The FPGA class:
# 1. Creates CLBs, switch matrices, and I/O blocks from a bitstream
# 2. Configures each element according to the bitstream
# 3. Provides methods for evaluating CLBs, routing signals, and I/O
# ---------------------------------------------------------------------------

module CodingAdventures
  module FPGA
    # Result of an FPGA simulation.
    SimResult = Struct.new(:outputs, :cycles, keyword_init: true)

    # Top-level FPGA fabric model.
    #
    # Creates and configures CLBs, switch matrices, and I/O blocks
    # from a Bitstream, then provides methods for interacting with
    # the configured circuit.
    #
    # @example simple AND gate
    #   config = {
    #     "clbs" => {
    #       "clb_0" => {
    #         "slice0" => {
    #           "lut_a" => [0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,0]
    #         }
    #       }
    #     },
    #     "io" => {
    #       "in_a" => {"mode" => "input"},
    #       "out"  => {"mode" => "output"}
    #     }
    #   }
    #   bs = Bitstream.from_hash(config)
    #   fpga = FPGAFabric.new(bs)
    class FPGAFabric
      attr_reader :bitstream

      # @param bitstream [Bitstream] configuration data for the fabric
      def initialize(bitstream)
        @bitstream = bitstream
        @clbs = {}
        @switches = {}
        @ios = {}

        configure(bitstream)
      end

      # Evaluate a specific CLB.
      #
      # @param clb_name [String] name of the CLB to evaluate
      # @param slice0_inputs_a [Array<Integer>] inputs for slice 0 LUT A
      # @param slice0_inputs_b [Array<Integer>] inputs for slice 0 LUT B
      # @param slice1_inputs_a [Array<Integer>] inputs for slice 1 LUT A
      # @param slice1_inputs_b [Array<Integer>] inputs for slice 1 LUT B
      # @param clock [Integer] clock signal (0 or 1)
      # @param carry_in [Integer] external carry input
      # @return [CLBOutput]
      # @raise [KeyError] if clb_name not found
      def evaluate_clb(clb_name, slice0_inputs_a:, slice0_inputs_b:,
        slice1_inputs_a:, slice1_inputs_b:,
        clock:, carry_in: 0)
        unless @clbs.key?(clb_name)
          raise KeyError, "CLB #{clb_name.inspect} not found"
        end

        @clbs[clb_name].evaluate(
          slice0_inputs_a, slice0_inputs_b,
          slice1_inputs_a, slice1_inputs_b,
          clock: clock, carry_in: carry_in
        )
      end

      # Route signals through a switch matrix.
      #
      # @param switch_name [String] name of the switch matrix
      # @param signals [Hash<String, Integer>] input signals
      # @return [Hash<String, Integer>] routed output signals
      # @raise [KeyError] if switch_name not found
      def route(switch_name, signals)
        unless @switches.key?(switch_name)
          raise KeyError, "Switch matrix #{switch_name.inspect} not found"
        end

        @switches[switch_name].route(signals)
      end

      # Drive an input pin.
      #
      # @param pin_name [String] name of the I/O pin
      # @param value [Integer] signal value (0 or 1)
      # @raise [KeyError] if pin_name not found
      def set_input(pin_name, value)
        unless @ios.key?(pin_name)
          raise KeyError, "I/O pin #{pin_name.inspect} not found"
        end
        @ios[pin_name].drive_pad(value)
      end

      # Read an output pin.
      #
      # @param pin_name [String] name of the I/O pin
      # @return [Integer, nil] signal value (0, 1, or nil for tri-state)
      # @raise [KeyError] if pin_name not found
      def read_output(pin_name)
        unless @ios.key?(pin_name)
          raise KeyError, "I/O pin #{pin_name.inspect} not found"
        end
        @ios[pin_name].read_pad
      end

      # Drive the internal side of an output pin (fabric -> external).
      #
      # @param pin_name [String] name of the I/O pin
      # @param value [Integer] signal value (0 or 1)
      # @raise [KeyError] if pin_name not found
      def drive_output(pin_name, value)
        unless @ios.key?(pin_name)
          raise KeyError, "I/O pin #{pin_name.inspect} not found"
        end
        @ios[pin_name].drive_internal(value)
      end

      # All CLBs in the fabric.
      # @return [Hash<String, CLB>]
      def clbs
        @clbs.dup
      end

      # All switch matrices in the fabric.
      # @return [Hash<String, SwitchMatrix>]
      def switches
        @switches.dup
      end

      # All I/O blocks.
      # @return [Hash<String, IOBlock>]
      def ios
        @ios.dup
      end

      private

      # Apply bitstream configuration to create and program all elements.
      def configure(bs)
        # Create and configure CLBs
        bs.clbs.each do |name, clb_cfg|
          clb = CLB.new(lut_inputs: bs.lut_k)

          clb.slice0.configure(
            lut_a_table: clb_cfg.slice0.lut_a,
            lut_b_table: clb_cfg.slice0.lut_b,
            ff_a_enabled: clb_cfg.slice0.ff_a_enabled,
            ff_b_enabled: clb_cfg.slice0.ff_b_enabled,
            carry_enabled: clb_cfg.slice0.carry_enabled
          )
          clb.slice1.configure(
            lut_a_table: clb_cfg.slice1.lut_a,
            lut_b_table: clb_cfg.slice1.lut_b,
            ff_a_enabled: clb_cfg.slice1.ff_a_enabled,
            ff_b_enabled: clb_cfg.slice1.ff_b_enabled,
            carry_enabled: clb_cfg.slice1.carry_enabled
          )

          @clbs[name] = clb
        end

        # Create and configure switch matrices
        bs.routing.each do |sw_name, routes|
          ports = Set.new
          routes.each do |r|
            ports.add(r.source)
            ports.add(r.destination)
          end

          unless ports.empty?
            sm = SwitchMatrix.new(ports)
            routes.each { |r| sm.connect(r.source, r.destination) }
            @switches[sw_name] = sm
          end
        end

        # Create I/O blocks
        mode_map = {
          "input" => IOMode::INPUT,
          "output" => IOMode::OUTPUT,
          "tristate" => IOMode::TRISTATE
        }
        bs.io.each do |pin_name, io_cfg|
          mode = mode_map.fetch(io_cfg.mode, IOMode::INPUT)
          @ios[pin_name] = IOBlock.new(pin_name, mode: mode)
        end
      end
    end
  end
end
