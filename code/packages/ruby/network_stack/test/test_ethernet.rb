# frozen_string_literal: true

require_relative "test_helper"

class TestEthernetFrame < Minitest::Test
  include CodingAdventures::NetworkStack

  def test_serialize_and_deserialize_round_trip
    # Build a frame, serialize it, deserialize it, and verify all fields match.
    frame = EthernetFrame.new(
      dest_mac: [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
      src_mac: [0x11, 0x22, 0x33, 0x44, 0x55, 0x66],
      ether_type: ETHER_TYPE_IPV4,
      payload: [0x01, 0x02, 0x03, 0x04]
    )

    bytes = frame.serialize
    restored = EthernetFrame.deserialize(bytes)

    assert_equal [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF], restored.dest_mac
    assert_equal [0x11, 0x22, 0x33, 0x44, 0x55, 0x66], restored.src_mac
    assert_equal ETHER_TYPE_IPV4, restored.ether_type
    assert_equal [0x01, 0x02, 0x03, 0x04], restored.payload
  end

  def test_serialize_produces_correct_byte_layout
    frame = EthernetFrame.new(
      dest_mac: [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
      src_mac: [0x00, 0x00, 0x00, 0x00, 0x00, 0x01],
      ether_type: ETHER_TYPE_ARP,
      payload: [0xDE, 0xAD]
    )

    bytes = frame.serialize
    # 6 + 6 + 2 + 2 = 16 bytes
    assert_equal 16, bytes.length
    # First 6 bytes are dest_mac
    assert_equal [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF], bytes[0..5]
    # Ether type 0x0806 in big-endian
    assert_equal 0x08, bytes[12]
    assert_equal 0x06, bytes[13]
  end

  def test_deserialize_returns_nil_for_short_input
    assert_nil EthernetFrame.deserialize([0x00] * 13)
    assert_nil EthernetFrame.deserialize([])
  end

  def test_empty_payload
    frame = EthernetFrame.new(
      dest_mac: [1, 2, 3, 4, 5, 6],
      src_mac: [7, 8, 9, 10, 11, 12],
      ether_type: ETHER_TYPE_IPV4
    )

    bytes = frame.serialize
    assert_equal 14, bytes.length

    restored = EthernetFrame.deserialize(bytes)
    assert_equal [], restored.payload
  end

  def test_broadcast_mac_constant
    assert_equal [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF], BROADCAST_MAC
  end
end

class TestARPTable < Minitest::Test
  include CodingAdventures::NetworkStack

  def test_insert_and_lookup
    table = ARPTable.new
    ip = [10, 0, 0, 1]
    mac = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]

    table.insert(ip, mac)
    result = table.lookup(ip)

    assert_equal mac, result
  end

  def test_lookup_unknown_returns_nil
    table = ARPTable.new
    assert_nil table.lookup([192, 168, 1, 1])
  end

  def test_update_existing_entry
    table = ARPTable.new
    ip = [10, 0, 0, 1]
    old_mac = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66]
    new_mac = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]

    table.insert(ip, old_mac)
    table.insert(ip, new_mac)

    assert_equal new_mac, table.lookup(ip)
    assert_equal 1, table.size
  end

  def test_multiple_entries
    table = ARPTable.new
    table.insert([10, 0, 0, 1], [0x11] * 6)
    table.insert([10, 0, 0, 2], [0x22] * 6)
    table.insert([10, 0, 0, 3], [0x33] * 6)

    assert_equal 3, table.size
    assert_equal [0x22] * 6, table.lookup([10, 0, 0, 2])
  end
end
