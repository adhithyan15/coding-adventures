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
