# frozen_string_literal: true

require_relative "test_helper"

class TestNetworkWire < Minitest::Test
  include CodingAdventures::NetworkStack

  def test_send_a_receive_b
    wire = NetworkWire.new

    wire.send_a([1, 2, 3])
    data = wire.receive_b
    assert_equal [1, 2, 3], data
  end

  def test_send_b_receive_a
    wire = NetworkWire.new

    wire.send_b([4, 5, 6])
    data = wire.receive_a
    assert_equal [4, 5, 6], data
  end

  def test_bidirectional
    wire = NetworkWire.new

    wire.send_a([1, 2])
    wire.send_b([3, 4])

    assert_equal [3, 4], wire.receive_a
    assert_equal [1, 2], wire.receive_b
  end

  def test_fifo_ordering
    wire = NetworkWire.new

    wire.send_a([1])
    wire.send_a([2])
    wire.send_a([3])

    assert_equal [1], wire.receive_b
    assert_equal [2], wire.receive_b
    assert_equal [3], wire.receive_b
  end

  def test_receive_empty_returns_nil
    wire = NetworkWire.new
    assert_nil wire.receive_a
    assert_nil wire.receive_b
  end

  def test_has_data_for_a
    wire = NetworkWire.new
    refute wire.has_data_for_a?

    wire.send_b([1])
    assert wire.has_data_for_a?

    wire.receive_a
    refute wire.has_data_for_a?
  end

  def test_has_data_for_b
    wire = NetworkWire.new
    refute wire.has_data_for_b?

    wire.send_a([1])
    assert wire.has_data_for_b?

    wire.receive_b
    refute wire.has_data_for_b?
  end

  def test_pending_counts
    wire = NetworkWire.new
    assert_equal 0, wire.pending_a_to_b
    assert_equal 0, wire.pending_b_to_a

    wire.send_a([1])
    wire.send_a([2])
    assert_equal 2, wire.pending_a_to_b
    assert_equal 0, wire.pending_b_to_a

    wire.send_b([3])
    assert_equal 1, wire.pending_b_to_a
  end

  def test_no_cross_talk
    # Data sent by A should NOT appear at A's receive;
    # it should only appear at B's receive.
    wire = NetworkWire.new

    wire.send_a([42])
    assert_nil wire.receive_a  # A should not see its own data
    assert_equal [42], wire.receive_b
  end

  def test_ethernet_frame_over_wire
    wire = NetworkWire.new

    frame = EthernetFrame.new(
      dest_mac: [0xBB] * 6,
      src_mac: [0xAA] * 6,
      ether_type: ETHER_TYPE_IPV4,
      payload: [0xDE, 0xAD]
    )

    wire.send_a(frame.serialize)
    received_bytes = wire.receive_b

    restored = EthernetFrame.deserialize(received_bytes)
    assert_equal [0xBB] * 6, restored.dest_mac
    assert_equal [0xAA] * 6, restored.src_mac
    assert_equal [0xDE, 0xAD], restored.payload
  end
end
