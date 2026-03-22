# frozen_string_literal: true

require_relative "test_helper"

class TestTCPHeader < Minitest::Test
  include CodingAdventures::NetworkStack

  def test_serialize_and_deserialize_round_trip
    header = TCPHeader.new(
      src_port: 49152,
      dst_port: 80,
      seq_num: 1000,
      ack_num: 2000,
      flags: TCP_SYN | TCP_ACK,
      window_size: 32768
    )

    bytes = header.serialize
    restored = TCPHeader.deserialize(bytes)

    assert_equal 49152, restored.src_port
    assert_equal 80, restored.dst_port
    assert_equal 1000, restored.seq_num
    assert_equal 2000, restored.ack_num
    assert_equal TCP_SYN | TCP_ACK, restored.flags
    assert_equal 32768, restored.window_size
    assert_equal 5, restored.data_offset
  end

  def test_serialize_produces_20_bytes
    header = TCPHeader.new(src_port: 80, dst_port: 443)
    assert_equal 20, header.serialize.length
  end

  def test_deserialize_returns_nil_for_short_input
    assert_nil TCPHeader.deserialize([0x00] * 19)
  end

  def test_flag_helpers
    header = TCPHeader.new(src_port: 80, dst_port: 443, flags: TCP_SYN | TCP_ACK)
    assert header.syn?
    assert header.ack?
    refute header.fin?
    refute header.rst?
    refute header.psh?
  end

  def test_fin_flag
    header = TCPHeader.new(src_port: 80, dst_port: 443, flags: TCP_FIN)
    assert header.fin?
    refute header.syn?
  end

  def test_large_sequence_number
    header = TCPHeader.new(src_port: 80, dst_port: 443, seq_num: 0xFFFFFFFF)
    bytes = header.serialize
    restored = TCPHeader.deserialize(bytes)
    assert_equal 0xFFFFFFFF, restored.seq_num
  end
end

class TestTCPConnection < Minitest::Test
  include CodingAdventures::NetworkStack

  def test_initial_state_is_closed
    conn = TCPConnection.new(local_port: 8080)
    assert_equal TCPState::CLOSED, conn.state
  end

  def test_initiate_connect_transitions_to_syn_sent
    conn = TCPConnection.new(local_port: 49152)
    syn = conn.initiate_connect([10, 0, 0, 2], 80)

    assert_equal TCPState::SYN_SENT, conn.state
    assert syn.syn?
    refute syn.ack?
    assert_equal 49152, syn.src_port
    assert_equal 80, syn.dst_port
  end

  def test_initiate_listen_transitions_to_listen
    conn = TCPConnection.new(local_port: 80)
    conn.initiate_listen

    assert_equal TCPState::LISTEN, conn.state
  end

  def test_three_way_handshake
    # Simulate the TCP three-way handshake between client and server.
    #
    # Client                    Server
    #   |--- SYN (seq=C) ------->|     CLOSED -> SYN_SENT
    #   |<-- SYN+ACK (seq=S) ---|     LISTEN -> SYN_RECEIVED
    #   |--- ACK --------------->|     SYN_SENT -> ESTABLISHED
    #                                  SYN_RECEIVED -> ESTABLISHED

    client = TCPConnection.new(local_port: 49152)
    server = TCPConnection.new(local_port: 80)

    # Server starts listening
    server.initiate_listen
    assert_equal TCPState::LISTEN, server.state

    # Client sends SYN
    syn = client.initiate_connect([10, 0, 0, 2], 80)
    assert_equal TCPState::SYN_SENT, client.state

    # Server receives SYN, sends SYN+ACK
    syn_ack = server.handle_segment(syn)
    assert_equal TCPState::SYN_RECEIVED, server.state
    assert syn_ack.syn?
    assert syn_ack.ack?

    # Client receives SYN+ACK, sends ACK
    ack = client.handle_segment(syn_ack)
    assert_equal TCPState::ESTABLISHED, client.state
    assert ack.ack?
    refute ack.syn?

    # Server receives ACK
    server.handle_segment(ack)
    assert_equal TCPState::ESTABLISHED, server.state
  end

  def test_data_transfer
    client, server = establish_connection

    # Client sends data
    data = [72, 101, 108, 108, 111]  # "Hello"
    data_header = client.send_data(data)
    refute_nil data_header
    assert data_header.psh?

    # Server receives data
    ack = server.handle_segment(data_header, data)
    refute_nil ack
    assert ack.ack?

    # Server reads the data
    received = server.receive(10)
    assert_equal data, received
  end

  def test_send_data_returns_nil_when_not_established
    conn = TCPConnection.new(local_port: 80)
    assert_nil conn.send_data([1, 2, 3])
  end

  def test_receive_empty_buffer
    client, _server = establish_connection
    assert_equal [], client.receive(10)
  end

  def test_connection_teardown
    client, server = establish_connection

    # Client initiates close (sends FIN)
    fin = client.initiate_close
    assert_equal TCPState::FIN_WAIT_1, client.state
    assert fin.fin?

    # Server receives FIN, sends ACK, transitions to CLOSE_WAIT
    ack = server.handle_segment(fin)
    assert_equal TCPState::CLOSE_WAIT, server.state
    assert ack.ack?

    # Client receives ACK, transitions to FIN_WAIT_2
    client.handle_segment(ack)
    assert_equal TCPState::FIN_WAIT_2, client.state

    # Server sends its own FIN (passive close)
    server_fin = server.initiate_close
    assert_equal TCPState::LAST_ACK, server.state
    assert server_fin.fin?

    # Client receives server FIN, sends ACK, transitions to TIME_WAIT
    final_ack = client.handle_segment(server_fin)
    assert_equal TCPState::TIME_WAIT, client.state
    assert final_ack.ack?

    # Server receives final ACK, transitions to CLOSED
    server.handle_segment(final_ack)
    assert_equal TCPState::CLOSED, server.state
  end

  def test_initiate_close_returns_nil_when_closed
    conn = TCPConnection.new(local_port: 80)
    assert_nil conn.initiate_close
  end

  def test_simultaneous_close
    client, server = establish_connection

    # Both sides send FIN at the same time
    client_fin = client.initiate_close
    assert_equal TCPState::FIN_WAIT_1, client.state

    # Client receives a FIN+ACK (server also closing)
    combined = TCPHeader.new(
      src_port: 80,
      dst_port: 49152,
      seq_num: server.seq_num,
      ack_num: client.seq_num,
      flags: TCP_FIN | TCP_ACK
    )
    result = client.handle_segment(combined)
    assert_equal TCPState::TIME_WAIT, client.state
    assert result.ack?
  end

  private

  # Helper to establish a connection between client and server.
  def establish_connection
    client = TCPConnection.new(local_port: 49152)
    server = TCPConnection.new(local_port: 80)

    server.initiate_listen
    syn = client.initiate_connect([10, 0, 0, 2], 80)
    syn_ack = server.handle_segment(syn)
    ack = client.handle_segment(syn_ack)
    server.handle_segment(ack)

    [client, server]
  end
end
