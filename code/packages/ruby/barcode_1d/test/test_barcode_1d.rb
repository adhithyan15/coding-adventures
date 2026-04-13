# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_barcode_1d"

class Barcode1DTest < Minitest::Test
  def test_build_scene
    scene = CodingAdventures::Barcode1D.build_scene("HELLO-123", symbology: :code39)
    assert_equal "code39", scene.metadata[:symbology]
    assert_operator scene.width, :>, 0
    assert_equal 120, scene.height
  end

  def test_build_scene_for_additional_symbologies
    cases = {
      codabar: "40156",
      code128: "Code 128",
      ean13: "400638133393",
      itf: "123456",
      upca: "03600029145",
    }

    cases.each do |symbology, data|
      scene = CodingAdventures::Barcode1D.build_scene(data, symbology: symbology)
      assert scene.metadata[:symbology]
      assert_operator scene.width, :>, 0
    end
  end

  def test_backend_probe
    assert_includes [nil, :metal], CodingAdventures::Barcode1D.current_backend
  end

  def test_unsupported_symbology
    assert_raises(CodingAdventures::Barcode1D::UnsupportedSymbologyError) do
      CodingAdventures::Barcode1D.build_scene("HELLO-123", symbology: :qr)
    end
  end

  def test_render_png_when_backend_is_available
    skip "Metal unavailable" unless CodingAdventures::Barcode1D.current_backend == :metal

    png = CodingAdventures::Barcode1D.render_png("HELLO-123", symbology: :code39)
    assert_equal "\x89PNG\r\n\x1a\n".b, png.byteslice(0, 8)
    assert_operator png.bytesize, :>, 100
  end
end
