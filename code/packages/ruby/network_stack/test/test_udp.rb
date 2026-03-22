# frozen_string_literal: true

require_relative "test_helper"

class TestUDPHeader < Minitest::Test
  include CodingAdventures::NetworkStack

  def test_serialize_and_deserialize_round_trip
    header = UDPHeader.new(
      src_port: 12345,
      dst_port: 53,
      length: 20,
      checksum: 0xABCD
    )

    bytes = header.serialize
    restored = UDPHeader.deserialize(bytes)

    assert_equal 12345, restored.src_port
    assert_equal 53, restored.dst_port
    assert_equal 20, restored.length
    assert_equal 0xABCD, restored.checksum
  end

  def test_serialize_produces_8_bytes
    header = UDPHeader.new(src_port: 1000, dst_port: 2000)
    assert_equal 8, header.serialize.length
  end

  def test_deserialize_returns_nil_for_short_input
    assert_nil UDPHeader.deserialize([0x00] * 7)
    assert_nil UDPHeader.deserialize([])
  end

  def test_default_values
    header = UDPHeader.new(src_port: 100, dst_port: 200)
    assert_equal 8, header.length
    assert_equal 0, header.checksum
  end
end

class TestUDPSocket < Minitest::Test
  include CodingAdventures::NetworkStack

  def test_send_to_creates_header_and_preserves_data
    sock = UDPSocket.new(local_port: 5000)
    data = [1, 2, 3, 4, 5]

    header, sent_data = sock.send_to(data, 53)

    assert_equal 5000, header.src_port
    assert_equal 53, header.dst_port
    assert_equal 13, header.length  # 8 header + 5 data
    assert_equal data, sent_data
  end

  def test_deliver_and_receive_from
    sock = UDPSocket.new(local_port: 5000)

    sock.deliver([10, 20, 30], [192, 168, 1, 1], 12345)

    result = sock.receive_from
    refute_nil result
    assert_equal [10, 20, 30], result[:data]
    assert_equal [192, 168, 1, 1], result[:src_ip]
    assert_equal 12345, result[:src_port]
  end

  def test_receive_from_empty_returns_nil
    sock = UDPSocket.new(local_port: 5000)
    assert_nil sock.receive_from
  end

  def test_fifo_ordering
    sock = UDPSocket.new(local_port: 5000)

    sock.deliver([1], [10, 0, 0, 1], 1000)
    sock.deliver([2], [10, 0, 0, 2], 2000)
    sock.deliver([3], [10, 0, 0, 3], 3000)

    assert_equal [1], sock.receive_from[:data]
    assert_equal [2], sock.receive_from[:data]
    assert_equal [3], sock.receive_from[:data]
    assert_nil sock.receive_from
  end

  def test_local_port
    sock = UDPSocket.new(local_port: 8080)
    assert_equal 8080, sock.local_port
  end
end
