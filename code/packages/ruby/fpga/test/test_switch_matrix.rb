# frozen_string_literal: true

require_relative "test_helper"

require "set"

# ============================================================================
# Tests for SwitchMatrix.
# ============================================================================

class TestSwitchMatrix < Minitest::Test
  def setup
    @sm = CodingAdventures::FPGA::SwitchMatrix.new(
      Set["north", "south", "east", "west", "clb_out"]
    )
  end

  def test_ports
    assert_includes @sm.ports, "north"
    assert_includes @sm.ports, "south"
    assert_equal 5, @sm.ports.size
  end

  def test_connect_and_route
    @sm.connect("clb_out", "east")
    @sm.connect("north", "south")

    result = @sm.route({"clb_out" => 1, "north" => 0})
    assert_equal({"east" => 1, "south" => 0}, result)
  end

  def test_fanout
    @sm.connect("clb_out", "east")
    @sm.connect("clb_out", "north")

    result = @sm.route({"clb_out" => 1})
    assert_equal 1, result["east"]
    assert_equal 1, result["north"]
  end

  def test_disconnect
    @sm.connect("north", "south")
    assert_equal 1, @sm.connection_count

    @sm.disconnect("south")
    assert_equal 0, @sm.connection_count
  end

  def test_clear
    @sm.connect("north", "south")
    @sm.connect("east", "west")
    @sm.clear
    assert_equal 0, @sm.connection_count
  end

  def test_connections_returns_copy
    @sm.connect("north", "south")
    conns = @sm.connections
    conns["east"] = "west"
    assert_equal 1, @sm.connection_count
  end

  def test_unknown_source_raises
    assert_raises(ArgumentError) { @sm.connect("unknown", "east") }
  end

  def test_unknown_destination_raises
    assert_raises(ArgumentError) { @sm.connect("north", "unknown") }
  end

  def test_self_connection_raises
    assert_raises(ArgumentError) { @sm.connect("north", "north") }
  end

  def test_duplicate_destination_raises
    @sm.connect("north", "south")
    assert_raises(ArgumentError) { @sm.connect("east", "south") }
  end

  def test_disconnect_unknown_port_raises
    assert_raises(ArgumentError) { @sm.disconnect("unknown") }
  end

  def test_disconnect_unconnected_port_raises
    assert_raises(ArgumentError) { @sm.disconnect("south") }
  end

  def test_empty_ports_raises
    assert_raises(ArgumentError) { CodingAdventures::FPGA::SwitchMatrix.new(Set[]) }
  end

  def test_non_string_port_raises
    assert_raises(ArgumentError) { CodingAdventures::FPGA::SwitchMatrix.new(Set[123]) }
  end

  def test_empty_string_port_raises
    assert_raises(ArgumentError) { CodingAdventures::FPGA::SwitchMatrix.new(Set[""]) }
  end

  def test_route_ignores_unknown_sources
    @sm.connect("north", "south")
    result = @sm.route({"east" => 1})
    assert_equal({}, result)
  end
end
