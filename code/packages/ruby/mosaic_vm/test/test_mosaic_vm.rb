# frozen_string_literal: true

# ================================================================
# Tests for the MosaicVM
# ================================================================
#
# The MosaicVM is the fourth stage of the Mosaic compiler pipeline.
# It traverses a MosaicIR tree and drives a backend renderer.
#
# We use a RecordingRenderer — a test double that records all
# method calls with their arguments — to verify the VM calls
# the renderer in the correct sequence with the right values.
# ================================================================

require "minitest/autorun"
require "coding_adventures_mosaic_analyzer"
require "coding_adventures_mosaic_vm"

class TestMosaicVm < Minitest::Test
  VM  = CodingAdventures::MosaicVm::MosaicVM
  MA  = CodingAdventures::MosaicAnalyzer

  # ----------------------------------------------------------------
  # RecordingRenderer: a test double that records all calls
  # ----------------------------------------------------------------

  class RecordingRenderer
    include CodingAdventures::MosaicVm::MosaicRenderer

    attr_reader :calls

    def initialize
      @calls = []
    end

    def begin_component(name, slots)
      @calls << [:begin_component, name, slots]
    end

    def end_component
      @calls << [:end_component]
    end

    def begin_node(tag, is_primitive, properties, _ctx)
      @calls << [:begin_node, tag, is_primitive, properties]
    end

    def end_node(tag)
      @calls << [:end_node, tag]
    end

    def render_slot_child(slot_name, slot_type, _ctx)
      @calls << [:render_slot_child, slot_name, slot_type]
    end

    def begin_when(slot_name, _ctx)
      @calls << [:begin_when, slot_name]
    end

    def end_when
      @calls << [:end_when]
    end

    def begin_each(slot_name, item_name, element_type, _ctx)
      @calls << [:begin_each, slot_name, item_name, element_type]
    end

    def end_each
      @calls << [:end_each]
    end

    def emit
      { files: [{ filename: "output.txt", content: @calls.map(&:first).join(",") }] }
    end
  end

  # ----------------------------------------------------------------
  # Helpers
  # ----------------------------------------------------------------

  def analyze(source)
    MA.analyze(source)
  end

  def run_vm(source)
    ir = analyze(source)
    renderer = RecordingRenderer.new
    result = VM.new(ir).run(renderer)
    [renderer.calls, result]
  end

  # ----------------------------------------------------------------
  # Version
  # ----------------------------------------------------------------

  def test_version_exists
    refute_nil CodingAdventures::MosaicVm::VERSION
  end

  # ----------------------------------------------------------------
  # Basic traversal order
  # ----------------------------------------------------------------

  def test_minimal_component_call_order
    calls, _result = run_vm("component Label { Text { } }")
    call_names = calls.map(&:first)
    assert_equal [
      :begin_component,
      :begin_node,
      :end_node,
      :end_component
    ], call_names
  end

  def test_begin_component_receives_name
    calls, _result = run_vm("component ProfileCard { Text { } }")
    bc = calls.find { |c| c[0] == :begin_component }
    assert_equal "ProfileCard", bc[1]
  end

  def test_begin_node_receives_tag
    calls, _result = run_vm("component C { Column { } }")
    bn = calls.find { |c| c[0] == :begin_node }
    assert_equal "Column", bn[1]
  end

  def test_begin_node_is_primitive_true_for_builtin
    calls, _result = run_vm("component C { Row { } }")
    bn = calls.find { |c| c[0] == :begin_node }
    assert_equal true, bn[2]
  end

  def test_begin_node_is_primitive_false_for_component
    calls, _result = run_vm("component C { Button { } }")
    bn = calls.find { |c| c[0] == :begin_node }
    assert_equal false, bn[2]
  end

  # ----------------------------------------------------------------
  # Emit result
  # ----------------------------------------------------------------

  def test_emit_returns_hash_with_files
    _calls, result = run_vm("component C { Text { } }")
    assert_kind_of Hash, result
    assert_includes result, :files
  end

  # ----------------------------------------------------------------
  # Property resolution
  # ----------------------------------------------------------------

  def test_string_property_passed_to_begin_node
    source = 'component C { Text { content: "hello"; } }'
    calls, _result = run_vm(source)
    bn = calls.find { |c| c[0] == :begin_node }
    props = bn[3]
    content = props.find { |p| p[:name] == "content" }
    refute_nil content
    assert_equal({ kind: "string", value: "hello" }, content[:value])
  end

  def test_dimension_property_passed_to_begin_node
    source = "component C { Column { padding: 16dp; } }"
    calls, _result = run_vm(source)
    bn = calls.find { |c| c[0] == :begin_node }
    props = bn[3]
    padding = props.find { |p| p[:name] == "padding" }
    refute_nil padding
    assert_equal "dimension", padding[:value][:kind]
    assert_equal 16.0, padding[:value][:value]
    assert_equal "dp", padding[:value][:unit]
  end

  def test_color_parsed_to_rgba
    source = "component C { Column { background: #2563eb; } }"
    calls, _result = run_vm(source)
    bn = calls.find { |c| c[0] == :begin_node }
    props = bn[3]
    bg = props.find { |p| p[:name] == "background" }
    refute_nil bg
    assert_equal "color", bg[:value][:kind]
    assert_equal 37,  bg[:value][:r]   # 0x25
    assert_equal 99,  bg[:value][:g]   # 0x63
    assert_equal 235, bg[:value][:b]   # 0xeb
    assert_equal 255, bg[:value][:a]   # default alpha
  end

  def test_short_color_parsed
    source = "component C { Column { background: #fff; } }"
    calls, _result = run_vm(source)
    bn = calls.find { |c| c[0] == :begin_node }
    props = bn[3]
    bg = props.find { |p| p[:name] == "background" }
    assert_equal "color", bg[:value][:kind]
    assert_equal 255, bg[:value][:r]
    assert_equal 255, bg[:value][:g]
    assert_equal 255, bg[:value][:b]
  end

  # ----------------------------------------------------------------
  # Slot child references
  # ----------------------------------------------------------------

  def test_render_slot_child_called
    source = "component C { slot header: node; Column { @header; } }"
    calls, _result = run_vm(source)
    rsc = calls.find { |c| c[0] == :render_slot_child }
    refute_nil rsc
    assert_equal "header", rsc[1]
  end

  # ----------------------------------------------------------------
  # when blocks
  # ----------------------------------------------------------------

  def test_when_block_calls
    source = <<~MOSAIC
      component C {
        slot show: bool;
        Column { when @show { Text { } } }
      }
    MOSAIC
    calls, _result = run_vm(source)
    call_names = calls.map(&:first)
    assert_includes call_names, :begin_when
    assert_includes call_names, :end_when
    bw = calls.find { |c| c[0] == :begin_when }
    assert_equal "show", bw[1]
  end

  # ----------------------------------------------------------------
  # each blocks
  # ----------------------------------------------------------------

  def test_each_block_calls
    source = <<~MOSAIC
      component List {
        slot items: list<text>;
        Column { each @items as item { Text { content: @item; } } }
      }
    MOSAIC
    calls, _result = run_vm(source)
    call_names = calls.map(&:first)
    assert_includes call_names, :begin_each
    assert_includes call_names, :end_each
    be = calls.find { |c| c[0] == :begin_each }
    assert_equal "items", be[1]
    assert_equal "item", be[2]
  end
end
