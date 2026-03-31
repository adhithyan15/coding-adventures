# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_draw_instructions"

# == Test Suite for DrawInstructions
#
# These tests verify that every primitive constructor produces the correct
# struct with the right defaults, and that render_with properly delegates
# to a duck-typed renderer.
class TestDrawInstructions < Minitest::Test
  DI = CodingAdventures::DrawInstructions

  # ------------------------------------------------------------------
  # Version
  # ------------------------------------------------------------------

  def test_version_exists
    refute_nil DI::VERSION
    assert_match(/\A\d+\.\d+\.\d+\z/, DI::VERSION)
  end

  # ------------------------------------------------------------------
  # draw_rect
  # ------------------------------------------------------------------

  def test_draw_rect_creates_correct_struct_with_defaults
    rect = DI.draw_rect(x: 10, y: 20, width: 30, height: 40)
    assert_equal "rect", rect.kind
    assert_equal 10, rect.x
    assert_equal 20, rect.y
    assert_equal 30, rect.width
    assert_equal 40, rect.height
    assert_equal "#000000", rect.fill
    assert_nil rect.stroke
    assert_nil rect.stroke_width
    assert_equal({}, rect.metadata)
  end

  def test_draw_rect_with_custom_fill_and_metadata
    rect = DI.draw_rect(x: 0, y: 0, width: 5, height: 5, fill: "#ff0000", metadata: { role: "bar" })
    assert_equal "#ff0000", rect.fill
    assert_equal "bar", rect.metadata[:role]
  end

  def test_draw_rect_with_stroke_options
    rect = DI.draw_rect(x: 0, y: 0, width: 100, height: 50, fill: "#fff",
                        stroke: "#000", stroke_width: 2)
    assert_equal "#000", rect.stroke
    assert_equal 2, rect.stroke_width
  end

  def test_draw_rect_is_frozen
    rect = DI.draw_rect(x: 0, y: 0, width: 1, height: 1)
    assert rect.frozen?
  end

  # ------------------------------------------------------------------
  # draw_text
  # ------------------------------------------------------------------

  def test_draw_text_creates_correct_struct_with_defaults
    text = DI.draw_text(x: 50, y: 100, value: "Hello")
    assert_equal "text", text.kind
    assert_equal 50, text.x
    assert_equal 100, text.y
    assert_equal "Hello", text.value
    assert_equal "#000000", text.fill
    assert_equal "monospace", text.font_family
    assert_equal 16, text.font_size
    assert_equal "middle", text.align
    assert_nil text.font_weight
    assert_equal({}, text.metadata)
  end

  def test_draw_text_with_custom_options
    text = DI.draw_text(x: 0, y: 0, value: "X", fill: "#red", font_family: "Arial",
                        font_size: 24, align: "start", metadata: { col: 0 })
    assert_equal "#red", text.fill
    assert_equal "Arial", text.font_family
    assert_equal 24, text.font_size
    assert_equal "start", text.align
    assert_equal 0, text.metadata[:col]
  end

  def test_draw_text_with_font_weight
    text = DI.draw_text(x: 0, y: 0, value: "Bold!", font_weight: "bold")
    assert_equal "bold", text.font_weight
  end

  def test_draw_text_is_frozen
    text = DI.draw_text(x: 0, y: 0, value: "hi")
    assert text.frozen?
  end

  # ------------------------------------------------------------------
  # draw_group
  # ------------------------------------------------------------------

  def test_draw_group_creates_struct_with_children
    r1 = DI.draw_rect(x: 0, y: 0, width: 1, height: 1)
    r2 = DI.draw_rect(x: 1, y: 0, width: 1, height: 1)
    group = DI.draw_group(children: [r1, r2], metadata: { layer: "bars" })
    assert_equal "group", group.kind
    assert_equal 2, group.children.length
    assert_equal "bars", group.metadata[:layer]
  end

  def test_draw_group_is_frozen
    group = DI.draw_group(children: [])
    assert group.frozen?
  end

  # ------------------------------------------------------------------
  # draw_line
  # ------------------------------------------------------------------

  def test_draw_line_creates_correct_struct
    line = DI.draw_line(x1: 0, y1: 0, x2: 100, y2: 50)
    assert_equal "line", line.kind
    assert_equal 0, line.x1
    assert_equal 0, line.y1
    assert_equal 100, line.x2
    assert_equal 50, line.y2
    assert_equal "#000000", line.stroke
    assert_equal 1, line.stroke_width
    assert_equal({}, line.metadata)
  end

  def test_draw_line_with_custom_stroke
    line = DI.draw_line(x1: 0, y1: 0, x2: 10, y2: 10, stroke: "#ccc", stroke_width: 3)
    assert_equal "#ccc", line.stroke
    assert_equal 3, line.stroke_width
  end

  def test_draw_line_is_frozen
    line = DI.draw_line(x1: 0, y1: 0, x2: 1, y2: 1)
    assert line.frozen?
  end

  # ------------------------------------------------------------------
  # draw_clip
  # ------------------------------------------------------------------

  def test_draw_clip_creates_correct_struct
    text = DI.draw_text(x: 5, y: 5, value: "clipped")
    clip = DI.draw_clip(x: 0, y: 0, width: 80, height: 30, children: [text])
    assert_equal "clip", clip.kind
    assert_equal 0, clip.x
    assert_equal 0, clip.y
    assert_equal 80, clip.width
    assert_equal 30, clip.height
    assert_equal 1, clip.children.length
    assert_equal "clipped", clip.children.first.value
    assert_equal({}, clip.metadata)
  end

  def test_draw_clip_with_metadata
    clip = DI.draw_clip(x: 0, y: 0, width: 10, height: 10, children: [],
                        metadata: { cell: "A1" })
    assert_equal "A1", clip.metadata[:cell]
  end

  def test_draw_clip_is_frozen
    clip = DI.draw_clip(x: 0, y: 0, width: 1, height: 1, children: [])
    assert clip.frozen?
  end

  # ------------------------------------------------------------------
  # create_scene
  # ------------------------------------------------------------------

  def test_create_scene_with_defaults
    scene = DI.create_scene(width: 200, height: 100, instructions: [])
    assert_equal 200, scene.width
    assert_equal 100, scene.height
    assert_equal "#ffffff", scene.background
    assert_equal [], scene.instructions
    assert_equal({}, scene.metadata)
  end

  def test_create_scene_with_custom_background_and_metadata
    rect = DI.draw_rect(x: 0, y: 0, width: 10, height: 10)
    scene = DI.create_scene(width: 100, height: 50, instructions: [rect],
                            background: "#f0f0f0", metadata: { label: "test" })
    assert_equal "#f0f0f0", scene.background
    assert_equal "test", scene.metadata[:label]
    assert_equal 1, scene.instructions.length
  end

  def test_create_scene_is_frozen
    scene = DI.create_scene(width: 1, height: 1, instructions: [])
    assert scene.frozen?
  end

  # ------------------------------------------------------------------
  # render_with
  # ------------------------------------------------------------------

  def test_render_with_delegates_to_renderer
    scene = DI.create_scene(width: 10, height: 10, instructions: [])
    # A renderer is any object with a render(scene) method (duck typing).
    renderer = Object.new
    def renderer.render(scene)
      "rendered:#{scene.width}x#{scene.height}"
    end
    assert_equal "rendered:10x10", DI.render_with(scene, renderer)
  end

  def test_render_with_passes_scene_to_renderer
    rect = DI.draw_rect(x: 0, y: 0, width: 5, height: 5)
    scene = DI.create_scene(width: 100, height: 50, instructions: [rect])
    renderer = Object.new
    def renderer.render(scene)
      scene.instructions.length
    end
    assert_equal 1, DI.render_with(scene, renderer)
  end
end
