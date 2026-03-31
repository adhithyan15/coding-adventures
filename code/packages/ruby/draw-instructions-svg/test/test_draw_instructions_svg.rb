# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_draw_instructions_svg"

# == Test Suite for DrawInstructionsSvg
#
# These tests verify that the SVG renderer correctly serializes every
# instruction type, handles optional attributes like stroke and font_weight,
# and properly escapes XML entities.
class TestDrawInstructionsSvg < Minitest::Test
  DI = CodingAdventures::DrawInstructions
  SVG = CodingAdventures::DrawInstructionsSvg

  # ------------------------------------------------------------------
  # Version
  # ------------------------------------------------------------------

  def test_version_exists
    refute_nil SVG::VERSION
    assert_match(/\A\d+\.\d+\.\d+\z/, SVG::VERSION)
  end

  # ------------------------------------------------------------------
  # Complete SVG document
  # ------------------------------------------------------------------

  def test_renders_complete_svg_document
    scene = DI.create_scene(width: 200, height: 100, instructions: [],
                            metadata: { label: "test scene" })
    svg = SVG.render_svg(scene)
    assert_includes svg, '<svg xmlns="http://www.w3.org/2000/svg"'
    assert_includes svg, 'width="200"'
    assert_includes svg, 'height="100"'
    assert_includes svg, 'viewBox="0 0 200 100"'
    assert_includes svg, 'role="img"'
    assert_includes svg, 'aria-label="test scene"'
    assert_includes svg, "</svg>"
  end

  def test_renders_default_aria_label_when_no_label_metadata
    scene = DI.create_scene(width: 10, height: 10, instructions: [])
    svg = SVG.render_svg(scene)
    assert_includes svg, 'aria-label="draw instructions scene"'
  end

  def test_renders_background_rect
    scene = DI.create_scene(width: 50, height: 30, instructions: [],
                            background: "#f0f0f0")
    svg = SVG.render_svg(scene)
    assert_includes svg, 'fill="#f0f0f0"'
  end

  # ------------------------------------------------------------------
  # Rect rendering
  # ------------------------------------------------------------------

  def test_renders_rect
    rect = DI.draw_rect(x: 10, y: 20, width: 30, height: 40, fill: "#3366cc")
    scene = DI.create_scene(width: 100, height: 100, instructions: [rect])
    svg = SVG.render_svg(scene)
    assert_includes svg, '<rect x="10" y="20" width="30" height="40" fill="#3366cc"'
    assert_includes svg, "/>"
  end

  def test_renders_rect_with_stroke
    rect = DI.draw_rect(x: 0, y: 0, width: 50, height: 50, fill: "#fff",
                        stroke: "#000", stroke_width: 2)
    scene = DI.create_scene(width: 100, height: 100, instructions: [rect])
    svg = SVG.render_svg(scene)
    assert_includes svg, 'stroke="#000"'
    assert_includes svg, 'stroke-width="2"'
  end

  def test_renders_rect_with_metadata
    rect = DI.draw_rect(x: 0, y: 0, width: 5, height: 5,
                        metadata: { char: "A", index: 0 })
    scene = DI.create_scene(width: 10, height: 10, instructions: [rect])
    svg = SVG.render_svg(scene)
    assert_includes svg, 'data-char="A"'
    assert_includes svg, 'data-index="0"'
  end

  # ------------------------------------------------------------------
  # Text rendering
  # ------------------------------------------------------------------

  def test_renders_text
    text = DI.draw_text(x: 50, y: 100, value: "Hello")
    scene = DI.create_scene(width: 200, height: 150, instructions: [text])
    svg = SVG.render_svg(scene)
    assert_includes svg, '<text x="50" y="100"'
    assert_includes svg, 'text-anchor="middle"'
    assert_includes svg, 'font-family="monospace"'
    assert_includes svg, 'font-size="16"'
    assert_includes svg, ">Hello</text>"
  end

  def test_renders_text_with_font_weight
    text = DI.draw_text(x: 0, y: 0, value: "Bold", font_weight: "bold")
    scene = DI.create_scene(width: 100, height: 50, instructions: [text])
    svg = SVG.render_svg(scene)
    assert_includes svg, 'font-weight="bold"'
  end

  def test_does_not_render_font_weight_when_normal
    text = DI.draw_text(x: 0, y: 0, value: "Normal", font_weight: "normal")
    scene = DI.create_scene(width: 100, height: 50, instructions: [text])
    svg = SVG.render_svg(scene)
    refute_includes svg, "font-weight"
  end

  def test_does_not_render_font_weight_when_nil
    text = DI.draw_text(x: 0, y: 0, value: "Default")
    scene = DI.create_scene(width: 100, height: 50, instructions: [text])
    svg = SVG.render_svg(scene)
    refute_includes svg, "font-weight"
  end

  # ------------------------------------------------------------------
  # Group rendering
  # ------------------------------------------------------------------

  def test_renders_group
    r1 = DI.draw_rect(x: 0, y: 0, width: 10, height: 10)
    r2 = DI.draw_rect(x: 10, y: 0, width: 10, height: 10)
    group = DI.draw_group(children: [r1, r2], metadata: { layer: "bars" })
    scene = DI.create_scene(width: 50, height: 50, instructions: [group])
    svg = SVG.render_svg(scene)
    assert_includes svg, '<g data-layer="bars">'
    assert_includes svg, "</g>"
    # Both children should appear
    assert_equal 2, svg.scan(/<rect /).count - 1 # minus background rect
  end

  # ------------------------------------------------------------------
  # Line rendering
  # ------------------------------------------------------------------

  def test_renders_line
    line = DI.draw_line(x1: 0, y1: 10, x2: 100, y2: 10, stroke: "#ccc", stroke_width: 2)
    scene = DI.create_scene(width: 100, height: 20, instructions: [line])
    svg = SVG.render_svg(scene)
    assert_includes svg, '<line x1="0" y1="10" x2="100" y2="10"'
    assert_includes svg, 'stroke="#ccc"'
    assert_includes svg, 'stroke-width="2"'
  end

  # ------------------------------------------------------------------
  # Clip rendering
  # ------------------------------------------------------------------

  def test_renders_clip
    text = DI.draw_text(x: 5, y: 15, value: "clipped")
    clip = DI.draw_clip(x: 0, y: 0, width: 80, height: 30, children: [text])
    scene = DI.create_scene(width: 100, height: 50, instructions: [clip])
    svg = SVG.render_svg(scene)
    assert_includes svg, "<clipPath"
    assert_includes svg, 'clip-path="url(#clip-1)"'
    assert_includes svg, ">clipped</text>"
  end

  def test_clip_ids_are_unique
    clip1 = DI.draw_clip(x: 0, y: 0, width: 10, height: 10, children: [])
    clip2 = DI.draw_clip(x: 20, y: 0, width: 10, height: 10, children: [])
    scene = DI.create_scene(width: 50, height: 20, instructions: [clip1, clip2])
    svg = SVG.render_svg(scene)
    assert_includes svg, 'id="clip-1"'
    assert_includes svg, 'id="clip-2"'
  end

  def test_clip_ids_reset_between_renders
    clip = DI.draw_clip(x: 0, y: 0, width: 10, height: 10, children: [])
    scene = DI.create_scene(width: 20, height: 20, instructions: [clip])
    svg1 = SVG.render_svg(scene)
    svg2 = SVG.render_svg(scene)
    # Both renders should produce clip-1, not clip-1 then clip-2
    assert_includes svg1, 'id="clip-1"'
    assert_includes svg2, 'id="clip-1"'
  end

  # ------------------------------------------------------------------
  # XML escaping
  # ------------------------------------------------------------------

  def test_escapes_xml_entities_in_text
    text = DI.draw_text(x: 0, y: 0, value: '<script>alert("xss")</script>')
    scene = DI.create_scene(width: 100, height: 50, instructions: [text])
    svg = SVG.render_svg(scene)
    refute_includes svg, "<script>"
    assert_includes svg, "&lt;script&gt;"
  end

  def test_escapes_xml_entities_in_fill
    rect = DI.draw_rect(x: 0, y: 0, width: 10, height: 10, fill: '"><bad')
    scene = DI.create_scene(width: 20, height: 20, instructions: [rect])
    svg = SVG.render_svg(scene)
    assert_includes svg, "&quot;&gt;&lt;bad"
  end

  def test_escapes_xml_entities_in_metadata
    rect = DI.draw_rect(x: 0, y: 0, width: 10, height: 10,
                        metadata: { note: 'a&b<c>"d' })
    scene = DI.create_scene(width: 20, height: 20, instructions: [rect])
    svg = SVG.render_svg(scene)
    assert_includes svg, "a&amp;b&lt;c&gt;&quot;d"
  end

  # ------------------------------------------------------------------
  # SvgRenderer class (duck-typed renderer)
  # ------------------------------------------------------------------

  def test_svg_renderer_class_works_with_render_with
    rect = DI.draw_rect(x: 0, y: 0, width: 10, height: 10)
    scene = DI.create_scene(width: 20, height: 20, instructions: [rect])
    renderer = SVG::SvgRenderer.new
    svg = DI.render_with(scene, renderer)
    assert_includes svg, "<svg"
    assert_includes svg, "</svg>"
  end
end
