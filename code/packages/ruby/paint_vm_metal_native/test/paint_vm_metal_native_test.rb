# frozen_string_literal: true

require_relative "test_helper"
require "coding_adventures_paint_instructions"

class PaintVmMetalNativeTest < Minitest::Test
  def test_runtime_probe_matches_availability
    if CodingAdventures::PaintVmMetalNative.supported_runtime?
      assert CodingAdventures::PaintVmMetalNative.available?
    else
      refute CodingAdventures::PaintVmMetalNative.available?
    end
  end

  def test_renders_a_rect_scene
    skip "Metal unavailable" unless CodingAdventures::PaintVmMetalNative.available?

    scene = CodingAdventures::PaintInstructions.paint_scene(
      width: 40,
      height: 20,
      instructions: [
        CodingAdventures::PaintInstructions.paint_rect(
          x: 10,
          y: 0,
          width: 20,
          height: 20,
          fill: "#000000",
        ),
      ],
      background: "#ffffff",
    )

    pixels = CodingAdventures::PaintVmMetalNative.render(scene)
    assert_equal 40, pixels.width
    assert_equal 20, pixels.height
    assert_equal [255, 255, 255, 255], CodingAdventures::PixelContainer.pixel_at(pixels, 5, 10)
    assert_equal [0, 0, 0, 255], CodingAdventures::PixelContainer.pixel_at(pixels, 20, 10)
  end
end
