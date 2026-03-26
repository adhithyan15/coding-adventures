# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Switch Matrix -- programmable routing crossbar for the FPGA fabric.
# ---------------------------------------------------------------------------
#
# === What is a Switch Matrix? ===
#
# The routing fabric is what makes an FPGA truly programmable. LUTs and
# CLBs compute boolean functions, but the switch matrix determines how
# those functions connect to each other.
#
# A switch matrix sits at each intersection of the routing grid. It is a
# crossbar that can connect any of its input wires to any of its output
# wires, based on configuration bits stored in SRAM.
#
# === Grid Layout ===
#
#     +-----+     +-----+     +-----+
#     | CLB |--SW--| CLB |--SW--| CLB |
#     +--+--+     +--+--+     +--+--+
#        |SW          |SW          |SW
#     +--+--+     +--+--+     +--+--+
#     | CLB |--SW--| CLB |--SW--| CLB |
#     +-----+     +-----+     +-----+
#
#     SW = Switch Matrix
#
# Each switch matrix connects wire segments from four directions (North,
# South, East, West) plus the adjacent CLB's outputs.
#
# === Connection Model ===
#
# We model the switch matrix as a set of named ports and a configurable
# connection map. Each connection maps a source port to a destination port.
# Multiple routes can share the same source (fan-out) but each destination
# can only have one source (no bus contention).
# ---------------------------------------------------------------------------

module CodingAdventures
  module FPGA
    # Programmable routing crossbar.
    #
    # Connects named signal ports via configurable routes. Multiple routes
    # can share the same source (fan-out) but each destination can only
    # have one source.
    #
    # @example
    #   sm = SwitchMatrix.new(Set["north", "south", "east", "west", "clb_out"])
    #   sm.connect("clb_out", "east")
    #   sm.connect("north", "south")
    #   sm.route({"clb_out" => 1, "north" => 0})
    #   # => {"east" => 1, "south" => 0}
    class SwitchMatrix
      # @param ports [Set<String>, Array<String>] set of port names
      # @raise [ArgumentError] if ports is empty or contains non-string/empty names
      def initialize(ports)
        ports = ports.to_set if ports.respond_to?(:to_set)

        if ports.empty?
          raise ArgumentError, "ports must be non-empty"
        end

        ports.each do |p|
          unless p.is_a?(String) && !p.empty?
            raise ArgumentError, "port names must be non-empty strings, got #{p.inspect}"
          end
        end

        @ports = ports.freeze
        # Maps destination -> source
        @connections = {}
      end

      # Create a route from source to destination.
      #
      # @param source [String] name of the input port
      # @param destination [String] name of the output port
      # @raise [ArgumentError] if ports are unknown, same, or destination already connected
      def connect(source, destination)
        unless @ports.include?(source)
          raise ArgumentError, "unknown source port: #{source.inspect}"
        end
        unless @ports.include?(destination)
          raise ArgumentError, "unknown destination port: #{destination.inspect}"
        end
        if source == destination
          raise ArgumentError, "cannot connect port #{source.inspect} to itself"
        end
        if @connections.key?(destination)
          raise ArgumentError,
            "destination #{destination.inspect} already connected to #{@connections[destination].inspect}"
        end

        @connections[destination] = source
      end

      # Remove the route to a destination port.
      #
      # @param destination [String] the port to disconnect
      # @raise [ArgumentError] if port is unknown or not connected
      def disconnect(destination)
        unless @ports.include?(destination)
          raise ArgumentError, "unknown port: #{destination.inspect}"
        end
        unless @connections.key?(destination)
          raise ArgumentError, "port #{destination.inspect} is not connected"
        end

        @connections.delete(destination)
      end

      # Remove all connections (reset the switch matrix).
      def clear
        @connections.clear
      end

      # Propagate signals through the switch matrix.
      #
      # @param inputs [Hash<String, Integer>] map of port name -> signal value
      # @return [Hash<String, Integer>] map of destination -> routed signal value
      def route(inputs)
        outputs = {}
        @connections.each do |dest, src|
          outputs[dest] = inputs[src] if inputs.key?(src)
        end
        outputs
      end

      # Set of all port names.
      # @return [Set<String>]
      def ports
        @ports
      end

      # Current connection map (destination -> source). Returns a copy.
      # @return [Hash<String, String>]
      def connections
        @connections.dup
      end

      # Number of active connections.
      # @return [Integer]
      def connection_count
        @connections.length
      end
    end
  end
end
