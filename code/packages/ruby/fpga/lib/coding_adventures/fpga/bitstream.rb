# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Bitstream -- FPGA configuration data.
# ---------------------------------------------------------------------------
#
# === What is a Bitstream? ===
#
# In a real FPGA, a bitstream is a binary blob that programs every
# configurable element: LUT truth tables, flip-flop enables, carry chain
# enables, routing switch states, I/O pad modes, and Block RAM contents.
#
# === Our JSON Configuration ===
#
# Instead of a binary format, we use JSON for readability and education.
# The JSON configuration specifies:
#
# 1. **CLBs**: Which LUTs get which truth tables, FF enables, carry enables
# 2. **Routing**: Which switch matrix ports are connected
# 3. **I/O**: Pin names, modes, and mappings
#
# Example JSON:
#
#     {
#         "clbs": {
#             "clb_0_0": {
#                 "slice0": {
#                     "lut_a": [0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,0],
#                     "lut_b": [0,1,1,0,1,0,0,1,1,0,0,1,0,1,1,0],
#                     "ff_a": true,
#                     "ff_b": false,
#                     "carry": false
#                 },
#                 "slice1": { ... }
#             }
#         },
#         "routing": {
#             "sw_0_0": [
#                 {"src": "clb_out_a", "dst": "east"}
#             ]
#         },
#         "io": {
#             "pin_A0": {"mode": "input"},
#             "pin_B0": {"mode": "output"}
#         }
#     }
# ---------------------------------------------------------------------------

require "json"

module CodingAdventures
  module FPGA
    # Configuration for one slice.
    SliceConfig = Struct.new(:lut_a, :lut_b, :ff_a_enabled, :ff_b_enabled,
      :carry_enabled, keyword_init: true)

    # Configuration for one CLB (2 slices).
    CLBConfig = Struct.new(:slice0, :slice1, keyword_init: true)

    # A single routing connection.
    RouteConfig = Struct.new(:source, :destination, keyword_init: true)

    # Configuration for one I/O block.
    IOConfig = Struct.new(:mode, keyword_init: true)

    # FPGA configuration data -- the "program" for the fabric.
    #
    # @attr clbs [Hash<String, CLBConfig>] CLB configs keyed by name
    # @attr routing [Hash<String, Array<RouteConfig>>] switch matrix connections
    # @attr io [Hash<String, IOConfig>] I/O block configs keyed by pin name
    # @attr lut_k [Integer] number of LUT inputs (default 4)
    class Bitstream
      attr_reader :clbs, :routing, :io, :lut_k

      def initialize(clbs: {}, routing: {}, io: {}, lut_k: 4)
        @clbs = clbs
        @routing = routing
        @io = io
        @lut_k = lut_k
      end

      # Load a bitstream from a JSON file.
      #
      # @param path [String] path to the JSON configuration file
      # @return [Bitstream]
      def self.from_json(path)
        data = JSON.parse(File.read(path))
        from_hash(data)
      end

      # Create a Bitstream from a Hash (same structure as JSON).
      #
      # @param data [Hash] configuration hash
      # @return [Bitstream]
      def self.from_hash(data)
        lut_k = data.fetch("lut_k", 4)
        default_tt = [0] * (1 << lut_k)

        # Parse CLBs
        clbs = {}
        (data["clbs"] || {}).each do |name, clb_data|
          s0 = clb_data.fetch("slice0", {})
          s1 = clb_data.fetch("slice1", {})
          clbs[name] = CLBConfig.new(
            slice0: SliceConfig.new(
              lut_a: s0.fetch("lut_a", default_tt.dup),
              lut_b: s0.fetch("lut_b", default_tt.dup),
              ff_a_enabled: s0.fetch("ff_a", false),
              ff_b_enabled: s0.fetch("ff_b", false),
              carry_enabled: s0.fetch("carry", false)
            ),
            slice1: SliceConfig.new(
              lut_a: s1.fetch("lut_a", default_tt.dup),
              lut_b: s1.fetch("lut_b", default_tt.dup),
              ff_a_enabled: s1.fetch("ff_a", false),
              ff_b_enabled: s1.fetch("ff_b", false),
              carry_enabled: s1.fetch("carry", false)
            )
          )
        end

        # Parse routing
        routing = {}
        (data["routing"] || {}).each do |sw_name, routes|
          routing[sw_name] = routes.map do |r|
            RouteConfig.new(source: r["src"], destination: r["dst"])
          end
        end

        # Parse I/O
        io = {}
        (data["io"] || {}).each do |pin_name, io_data|
          io[pin_name] = IOConfig.new(mode: io_data.fetch("mode", "input"))
        end

        new(clbs: clbs, routing: routing, io: io, lut_k: lut_k)
      end
    end
  end
end
