# frozen_string_literal: true

require "ostruct"
require_relative "coding_adventures/draw_instructions/version"

module CodingAdventures
  # Backend-neutral draw scene primitives.
  #
  # The important architectural boundary is:
  # - producers decide what should be drawn
  # - renderers decide how to serialize or paint it
  #
  # This package lives in the middle. It knows about rectangles, text, groups,
  # and scenes. It does not know what a barcode is.
  module DrawInstructions
    module_function

    def draw_rect(x:, y:, width:, height:, fill: "#000000", metadata: {})
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

    def draw_text(x:, y:, value:, fill: "#000000", font_family: "monospace", font_size: 16, align: "middle", metadata: {})
      OpenStruct.new(
        kind: "text",
        x: x,
        y: y,
        value: value,
        fill: fill,
        font_family: font_family,
        font_size: font_size,
        align: align,
        metadata: metadata,
      )
    end

    def draw_group(children:, metadata: {})
      OpenStruct.new(kind: "group", children: children, metadata: metadata)
    end

    def create_scene(width:, height:, instructions:, background: "#ffffff", metadata: {})
      OpenStruct.new(
        width: width,
        height: height,
        instructions: instructions,
        background: background,
        metadata: metadata,
      )
    end

    def render_with(scene, renderer)
      renderer.render(scene)
    end
  end
end
