# frozen_string_literal: true

require_relative "test_helper"
require "coding_adventures/pixel_container"

class PaintCodecPngNativeTest < Minitest::Test
  def test_available
    assert CodingAdventures::PaintCodecPngNative.available?
  end

  def test_encode_png
    pixels = CodingAdventures::PixelContainer::Container.new(1, 1, [0, 0, 0, 255].pack("C*").b)
    png = CodingAdventures::PaintCodecPngNative.encode(pixels)
    assert_equal "\x89PNG\r\n\x1a\n".b, png.byteslice(0, 8)
  end
end
