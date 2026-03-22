"""Tests for the TCP layer — state machine, handshake, data transfer, close."""

from network_stack.tcp import (
    TCP_ACK,
    TCP_FIN,
    TCP_PSH,
    TCP_SYN,
    TCPConnection,
    TCPHeader,
    TCPState,
)


class TestTCPHeader:
    """Tests for TCP header serialization and deserialization."""

    def test_serialize_and_deserialize_roundtrip(self) -> None:
        """A TCP header should survive a serialize -> deserialize roundtrip."""
        header = TCPHeader(
            src_port=49152,
            dst_port=80,
            seq_num=1000,
            ack_num=2000,
            flags=TCP_SYN | TCP_ACK,
            window_size=32768,
        )
        raw = header.serialize()
        recovered = TCPHeader.deserialize(raw)

        assert recovered.src_port == 49152
        assert recovered.dst_port == 80
        assert recovered.seq_num == 1000
        assert recovered.ack_num == 2000
        assert recovered.flags == TCP_SYN | TCP_ACK
        assert recovered.window_size == 32768

    def test_serialize_length(self) -> None:
        """A standard TCP header should be exactly 20 bytes."""
        header = TCPHeader()
        assert len(header.serialize()) == 20

    def test_data_offset_roundtrip(self) -> None:
        """The data_offset field should survive roundtrip."""
        header = TCPHeader(data_offset=5)
        raw = header.serialize()
        recovered = TCPHeader.deserialize(raw)
        assert recovered.data_offset == 5

    def test_deserialize_too_short_raises(self) -> None:
        """Deserializing less than 20 bytes should raise ValueError."""
        try:
            TCPHeader.deserialize(b"\x00" * 19)
            assert False, "Expected ValueError"  # noqa: B011
        except ValueError as e:
            assert "too short" in str(e)

    def test_flag_combinations(self) -> None:
        """Various flag combinations should be preserved."""
        for flags in [TCP_SYN, TCP_ACK, TCP_FIN, TCP_PSH | TCP_ACK,
                      TCP_SYN | TCP_ACK, TCP_FIN | TCP_ACK]:
            header = TCPHeader(flags=flags)
            raw = header.serialize()
            recovered = TCPHeader.deserialize(raw)
            assert recovered.flags == flags

    def test_large_sequence_number(self) -> None:
        """Sequence numbers up to 2^32-1 should work."""
        header = TCPHeader(seq_num=0xFFFFFFFF)
        raw = header.serialize()
        recovered = TCPHeader.deserialize(raw)
        assert recovered.seq_num == 0xFFFFFFFF


class TestTCPThreeWayHandshake:
    """Tests for the TCP three-way handshake (connection establishment)."""

    def test_client_sends_syn(self) -> None:
        """initiate_connect should send SYN and transition to SYN_SENT."""
        client = TCPConnection(local_port=49152, remote_ip=0x0A000002,
                               remote_port=80)
        syn = client.initiate_connect()

        assert client.state == TCPState.SYN_SENT
        assert syn.flags == TCP_SYN
        assert syn.src_port == 49152
        assert syn.dst_port == 80

    def test_server_responds_synack(self) -> None:
        """Server in LISTEN state should respond to SYN with SYN+ACK."""
        server = TCPConnection(local_port=80)
        server.initiate_listen()
        assert server.state == TCPState.LISTEN

        # Simulate receiving a SYN from client
        syn = TCPHeader(src_port=49152, dst_port=80, seq_num=1000,
                        flags=TCP_SYN)
        synack = server.handle_segment(syn)

        assert server.state == TCPState.SYN_RECEIVED
        assert synack is not None
        assert synack.flags == TCP_SYN | TCP_ACK
        assert synack.ack_num == 1001  # client seq + 1

    def test_full_handshake(self) -> None:
        """Complete three-way handshake should result in ESTABLISHED."""
        # Step 1: Client sends SYN
        client = TCPConnection(local_port=49152, remote_ip=0x0A000002,
                               remote_port=80)
        syn = client.initiate_connect()

        # Step 2: Server receives SYN, sends SYN+ACK
        server = TCPConnection(local_port=80)
        server.initiate_listen()
        synack = server.handle_segment(syn)
        assert synack is not None

        # Step 3: Client receives SYN+ACK, sends ACK
        ack = client.handle_segment(synack)
        assert ack is not None
        assert client.state == TCPState.ESTABLISHED

        # Step 4: Server receives ACK
        server.handle_segment(ack)
        assert server.state == TCPState.ESTABLISHED


class TestTCPDataTransfer:
    """Tests for sending and receiving data over an established connection."""

    def _establish_connection(self) -> tuple[TCPConnection, TCPConnection]:
        """Helper: set up two ESTABLISHED connections."""
        client = TCPConnection(local_port=49152, remote_ip=0x0A000002,
                               remote_port=80)
        server = TCPConnection(local_port=80)
        server.initiate_listen()

        syn = client.initiate_connect()
        synack = server.handle_segment(syn)
        assert synack is not None
        ack = client.handle_segment(synack)
        assert ack is not None
        server.handle_segment(ack)

        return client, server

    def test_send_data(self) -> None:
        """send() should return a segment with PSH+ACK flags."""
        client, _server = self._establish_connection()

        seg = client.send(b"Hello")
        assert seg is not None
        assert seg.flags == TCP_PSH | TCP_ACK
        assert len(client.send_buffer) == 5

    def test_receive_data(self) -> None:
        """Data sent by client should be receivable by server."""
        client, server = self._establish_connection()

        seg = client.send(b"Hello, server!")
        assert seg is not None

        # Server processes the data segment
        ack = server.handle_segment(seg, payload=b"Hello, server!")
        assert ack is not None
        assert ack.flags == TCP_ACK

        # Server reads the data
        data = server.receive(100)
        assert data == b"Hello, server!"

    def test_receive_empty_buffer(self) -> None:
        """receive() on empty buffer should return empty bytes."""
        client, _server = self._establish_connection()
        assert client.receive(100) == b""

    def test_send_when_not_established(self) -> None:
        """send() should return None if not in ESTABLISHED state."""
        conn = TCPConnection(local_port=49152)
        result = conn.send(b"data")
        assert result is None

    def test_partial_receive(self) -> None:
        """receive() should return only the requested number of bytes."""
        client, server = self._establish_connection()

        seg = client.send(b"Hello, World!")
        assert seg is not None
        server.handle_segment(seg, payload=b"Hello, World!")

        # Read 5 bytes
        data = server.receive(5)
        assert data == b"Hello"

        # Read the rest
        data = server.receive(100)
        assert data == b", World!"


class TestTCPConnectionClose:
    """Tests for TCP connection teardown."""

    def _establish_connection(self) -> tuple[TCPConnection, TCPConnection]:
        """Helper: set up two ESTABLISHED connections."""
        client = TCPConnection(local_port=49152, remote_ip=0x0A000002,
                               remote_port=80)
        server = TCPConnection(local_port=80)
        server.initiate_listen()

        syn = client.initiate_connect()
        synack = server.handle_segment(syn)
        assert synack is not None
        ack = client.handle_segment(synack)
        assert ack is not None
        server.handle_segment(ack)

        return client, server

    def test_active_close(self) -> None:
        """initiate_close from ESTABLISHED should transition to FIN_WAIT_1."""
        client, _server = self._establish_connection()

        fin = client.initiate_close()
        assert fin is not None
        assert client.state == TCPState.FIN_WAIT_1
        assert fin.flags == TCP_FIN | TCP_ACK

    def test_full_close_sequence(self) -> None:
        """Full four-way close: FIN -> ACK -> FIN -> ACK."""
        client, server = self._establish_connection()

        # Client initiates close
        fin1 = client.initiate_close()
        assert fin1 is not None
        assert client.state == TCPState.FIN_WAIT_1

        # Server receives FIN, goes to CLOSE_WAIT
        ack1 = server.handle_segment(fin1)
        assert ack1 is not None
        assert server.state == TCPState.CLOSE_WAIT

        # Client receives ACK, goes to FIN_WAIT_2
        client.handle_segment(ack1)
        assert client.state == TCPState.FIN_WAIT_2

        # Server closes its side
        fin2 = server.initiate_close()
        assert fin2 is not None
        assert server.state == TCPState.LAST_ACK

        # Client receives FIN, goes to TIME_WAIT
        ack2 = client.handle_segment(fin2)
        assert ack2 is not None
        assert client.state == TCPState.TIME_WAIT

        # Server receives final ACK, goes to CLOSED
        server.handle_segment(ack2)
        assert server.state == TCPState.CLOSED

    def test_simultaneous_close(self) -> None:
        """Both sides closing at the same time (FIN+ACK crossing)."""
        client, server = self._establish_connection()

        # Both sides send FIN simultaneously
        fin_client = client.initiate_close()
        assert fin_client is not None

        # Server receives client's FIN+ACK while ESTABLISHED
        ack = server.handle_segment(fin_client)
        assert ack is not None
        assert server.state == TCPState.CLOSE_WAIT

    def test_close_from_closed_state(self) -> None:
        """initiate_close from CLOSED should return None."""
        conn = TCPConnection(local_port=49152)
        assert conn.initiate_close() is None

    def test_fin_wait_1_receives_fin_ack(self) -> None:
        """In FIN_WAIT_1, receiving FIN+ACK should go to TIME_WAIT."""
        client, server = self._establish_connection()

        client.initiate_close()
        assert client.state == TCPState.FIN_WAIT_1

        # Simulate receiving FIN+ACK
        fin_ack = TCPHeader(
            src_port=80, dst_port=49152,
            seq_num=server.seq_num,
            ack_num=client.seq_num,
            flags=TCP_FIN | TCP_ACK,
        )
        result = client.handle_segment(fin_ack)
        assert result is not None
        assert client.state == TCPState.TIME_WAIT


class TestTCPStateMachine:
    """Tests for edge cases in the TCP state machine."""

    def test_initial_state_is_closed(self) -> None:
        """A new connection should start in CLOSED state."""
        conn = TCPConnection(local_port=80)
        assert conn.state == TCPState.CLOSED

    def test_listen_transitions_to_listen(self) -> None:
        """initiate_listen should transition from CLOSED to LISTEN."""
        conn = TCPConnection(local_port=80)
        conn.initiate_listen()
        assert conn.state == TCPState.LISTEN

    def test_established_receives_pure_ack(self) -> None:
        """A pure ACK in ESTABLISHED state (no data, no FIN) returns None."""
        client = TCPConnection(local_port=49152, remote_ip=0x0A000002,
                               remote_port=80)
        server = TCPConnection(local_port=80)
        server.initiate_listen()

        syn = client.initiate_connect()
        synack = server.handle_segment(syn)
        assert synack is not None
        ack = client.handle_segment(synack)
        assert ack is not None
        server.handle_segment(ack)

        # Send a pure ACK with no data
        pure_ack = TCPHeader(flags=TCP_ACK)
        result = server.handle_segment(pure_ack)
        assert result is None

    def test_closing_state_receives_ack(self) -> None:
        """In CLOSING state, receiving ACK should transition to TIME_WAIT."""
        conn = TCPConnection(local_port=49152, remote_port=80)
        conn.state = TCPState.CLOSING

        ack = TCPHeader(flags=TCP_ACK)
        conn.handle_segment(ack)
        assert conn.state == TCPState.TIME_WAIT

    def test_last_ack_receives_ack(self) -> None:
        """In LAST_ACK state, receiving ACK should transition to CLOSED."""
        conn = TCPConnection(local_port=80)
        conn.state = TCPState.LAST_ACK

        ack = TCPHeader(flags=TCP_ACK)
        conn.handle_segment(ack)
        assert conn.state == TCPState.CLOSED

    def test_time_wait_returns_none(self) -> None:
        """In TIME_WAIT state, any segment returns None."""
        conn = TCPConnection(local_port=80)
        conn.state = TCPState.TIME_WAIT

        result = conn.handle_segment(TCPHeader(flags=TCP_ACK))
        assert result is None
