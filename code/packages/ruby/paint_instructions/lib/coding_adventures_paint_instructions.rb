# frozen_string_literal: true

require "ostruct"
require_relative "coding_adventures/paint_instructions/version"

module CodingAdventures
  module PaintInstructions
    module_function

    def paint_rect(x:, y:, width:, height:, fill: "#000000", metadata: {})
      OpenStruct.new(
        kind: "rect",
        x: x,
        y: y,
        width: width,
        height: height,
        fill: fill,
        metadata: metadata,
      )
    end

    # paint_path creates a PaintPath instruction — an arbitrary vector path
    # built from an array of PathCommand hashes. Each command has a :kind key
    # and optional coordinate fields:
    #
    #   { kind: "move_to", x: 10, y: 20 }
    #   { kind: "line_to", x: 50, y: 60 }
    #   { kind: "close" }
    #
    # This is used by hex-module grids (MaxiCode) where each dark module is
    # rendered as a filled flat-top hexagon made of six line_to commands.
    def paint_path(commands, fill: "#000000", stroke: nil, stroke_width: nil, metadata: {})
      OpenStruct.new(
        kind: "path",
        commands: commands,
        fill: fill,
        stroke: stroke,
        stroke_width: stroke_width,
        metadata: metadata,
      )
    end

    def paint_scene(width:, height:, instructions:, background: "#ffffff", metadata: {})
      OpenStruct.new(
        width: width,
        height: height,
        instructions: instructions,
        background: background,
        metadata: metadata,
      )
    end

    def create_scene(width:, height:, instructions:, background: "#ffffff", metadata: {})
      paint_scene(
        width: width,
        height: height,
        instructions: instructions,
        background: background,
        metadata: metadata,
      )
    end
  end
end
