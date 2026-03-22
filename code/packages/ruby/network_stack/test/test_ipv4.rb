# frozen_string_literal: true

require_relative "test_helper"

class TestIPv4Header < Minitest::Test
  include CodingAdventures::NetworkStack

  def test_serialize_and_deserialize_round_trip
    header = IPv4Header.new(
      src_ip: [10, 0, 0, 1],
      dst_ip: [10, 0, 0, 2],
      protocol: PROTOCOL_TCP,
      total_length: 60,
      ttl: 64
    )
    header.header_checksum = header.compute_checksum

    bytes = header.serialize
    restored = IPv4Header.deserialize(bytes)

    assert_equal 4, restored.version
    assert_equal 5, restored.ihl
    assert_equal 60, restored.total_length
    assert_equal 64, restored.ttl
    assert_equal PROTOCOL_TCP, restored.protocol
    assert_equal [10, 0, 0, 1], restored.src_ip
    assert_equal [10, 0, 0, 2], restored.dst_ip
  end

  def test_serialize_produces_20_bytes
    header = IPv4Header.new(
      src_ip: [192, 168, 1, 1],
      dst_ip: [192, 168, 1, 2],
      protocol: PROTOCOL_UDP
    )
    assert_equal 20, header.serialize.length
  end

  def test_deserialize_returns_nil_for_short_input
    assert_nil IPv4Header.deserialize([0x00] * 19)
    assert_nil IPv4Header.deserialize([])
  end

  def test_compute_checksum_nonzero
    header = IPv4Header.new(
      src_ip: [10, 0, 0, 1],
      dst_ip: [10, 0, 0, 2],
      protocol: PROTOCOL_TCP,
      total_length: 40
    )
    checksum = header.compute_checksum
    refute_equal 0, checksum
  end

  def test_verify_checksum_succeeds
    header = IPv4Header.new(
      src_ip: [10, 0, 0, 1],
      dst_ip: [10, 0, 0, 2],
      protocol: PROTOCOL_TCP,
      total_length: 40
    )
    header.header_checksum = header.compute_checksum
    assert header.verify_checksum
  end

  def test_verify_checksum_fails_on_corruption
    header = IPv4Header.new(
      src_ip: [10, 0, 0, 1],
      dst_ip: [10, 0, 0, 2],
      protocol: PROTOCOL_TCP,
      total_length: 40
    )
    header.header_checksum = header.compute_checksum
    # Corrupt the TTL field
    header.ttl = 32
    refute header.verify_checksum
  end

  def test_protocol_constants
    assert_equal 6, PROTOCOL_TCP
    assert_equal 17, PROTOCOL_UDP
  end
end

class TestRoutingTable < Minitest::Test
  include CodingAdventures::NetworkStack

  def test_add_and_lookup_single_route
    table = RoutingTable.new
    table.add_route([10, 0, 0, 0], [255, 255, 255, 0], [0, 0, 0, 0], "eth0")

    route = table.lookup([10, 0, 0, 5])
    refute_nil route
    assert_equal "eth0", route.interface_name
  end

  def test_longest_prefix_match
    table = RoutingTable.new
    # Broad route: 10.0.0.0/8
    table.add_route([10, 0, 0, 0], [255, 0, 0, 0], [10, 0, 0, 1], "eth0")
    # Specific route: 10.0.1.0/24
    table.add_route([10, 0, 1, 0], [255, 255, 255, 0], [10, 0, 1, 1], "eth1")

    # 10.0.1.5 matches both, but /24 is more specific
    route = table.lookup([10, 0, 1, 5])
    assert_equal "eth1", route.interface_name
    assert_equal [10, 0, 1, 1], route.gateway

    # 10.0.2.5 only matches the /8 route
    route = table.lookup([10, 0, 2, 5])
    assert_equal "eth0", route.interface_name
  end

  def test_no_matching_route_returns_nil
    table = RoutingTable.new
    table.add_route([10, 0, 0, 0], [255, 255, 255, 0], [0, 0, 0, 0], "eth0")

    assert_nil table.lookup([192, 168, 1, 1])
  end

  def test_default_route
    table = RoutingTable.new
    # Default route: 0.0.0.0/0 matches everything
    table.add_route([0, 0, 0, 0], [0, 0, 0, 0], [10, 0, 0, 1], "eth0")

    route = table.lookup([8, 8, 8, 8])
    refute_nil route
    assert_equal [10, 0, 0, 1], route.gateway
  end

  def test_size
    table = RoutingTable.new
    assert_equal 0, table.size
    table.add_route([10, 0, 0, 0], [255, 255, 255, 0], [0, 0, 0, 0], "eth0")
    assert_equal 1, table.size
  end
end

class TestIPLayer < Minitest::Test
  include CodingAdventures::NetworkStack

  def test_create_packet
    layer = IPLayer.new(local_ip: [10, 0, 0, 1])
    payload = [0x01, 0x02, 0x03, 0x04]

    packet = layer.create_packet([10, 0, 0, 2], PROTOCOL_TCP, payload)
    refute_nil packet
    assert_equal 24, packet.length  # 20 header + 4 payload
  end

  def test_parse_packet_round_trip
    layer = IPLayer.new(local_ip: [10, 0, 0, 1])
    payload = [0xDE, 0xAD, 0xBE, 0xEF]

    packet = layer.create_packet([10, 0, 0, 2], PROTOCOL_UDP, payload)
    src_ip, protocol, parsed_payload = layer.parse_packet(packet)

    assert_equal [10, 0, 0, 1], src_ip
    assert_equal PROTOCOL_UDP, protocol
    assert_equal payload, parsed_payload
  end

  def test_parse_packet_returns_nil_for_short_input
    layer = IPLayer.new(local_ip: [10, 0, 0, 1])
    assert_nil layer.parse_packet([0x00] * 10)
  end

  def test_parse_packet_returns_nil_for_bad_checksum
    layer = IPLayer.new(local_ip: [10, 0, 0, 1])
    packet = layer.create_packet([10, 0, 0, 2], PROTOCOL_TCP, [1, 2, 3])

    # Corrupt the packet
    packet[8] = (packet[8] + 1) & 0xFF  # flip TTL

    assert_nil layer.parse_packet(packet)
  end
end
