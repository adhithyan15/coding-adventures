defmodule CodingAdventures.NetworkStackTest do
  @moduledoc """
  # Network Stack Tests

  These tests verify every layer of the network stack, from Ethernet frames
  at the bottom to HTTP at the top. Each test group corresponds to one layer
  of the TCP/IP model.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.NetworkStack.{
    EthernetFrame,
    ARPTable,
    IPv4Header,
    RoutingTable,
    IPLayer,
    TCPHeader,
    TCPFlags,
    TCPConnection,
    UDPHeader,
    UDPSocket,
    SocketManager,
    DNSResolver,
    HTTPRequest,
    HTTPResponse,
    HTTPClient,
    NetworkWire
  }

  # ===========================================================================
  # Layer 2: Ethernet Tests
  # ===========================================================================

  describe "EthernetFrame" do
    test "serialize and deserialize round-trip" do
      dest = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]
      src = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66]
      payload = [0x01, 0x02, 0x03, 0x04]
      frame = EthernetFrame.new(dest, src, 0x0800, payload)

      bytes = EthernetFrame.serialize(frame)
      restored = EthernetFrame.deserialize(bytes)

      assert restored.dest_mac == dest
      assert restored.src_mac == src
      assert restored.ether_type == 0x0800
      assert restored.payload == payload
    end

    test "ARP ether_type (0x0806)" do
      frame = EthernetFrame.new(
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
        [0x11, 0x22, 0x33, 0x44, 0x55, 0x66],
        0x0806,
        [0x00, 0x01]
      )
      bytes = EthernetFrame.serialize(frame)
      restored = EthernetFrame.deserialize(bytes)
      assert restored.ether_type == 0x0806
    end

    test "correct byte count" do
      frame = EthernetFrame.new([0,0,0,0,0,0], [0,0,0,0,0,0], 0x0800, [1,2,3])
      assert length(EthernetFrame.serialize(frame)) == 17
    end

    test "empty payload" do
      frame = EthernetFrame.new(
        [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
        [0x11, 0x22, 0x33, 0x44, 0x55, 0x66],
        0x0800,
        []
      )
      bytes = EthernetFrame.serialize(frame)
      assert length(bytes) == 14
      restored = EthernetFrame.deserialize(bytes)
      assert restored.payload == []
    end
  end

  describe "ARPTable" do
    test "insert and lookup" do
      mac = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]
      table = ARPTable.new() |> ARPTable.insert("10.0.0.1", mac)
      assert ARPTable.lookup(table, "10.0.0.1") == mac
      assert ARPTable.size(table) == 1
    end

    test "unknown IP returns nil" do
      table = ARPTable.new()
      assert ARPTable.lookup(table, "10.0.0.99") == nil
    end

    test "update existing entry" do
      table = ARPTable.new()
        |> ARPTable.insert("10.0.0.1", [0x11, 0x11, 0x11, 0x11, 0x11, 0x11])
        |> ARPTable.insert("10.0.0.1", [0x22, 0x22, 0x22, 0x22, 0x22, 0x22])
      assert ARPTable.lookup(table, "10.0.0.1") == [0x22, 0x22, 0x22, 0x22, 0x22, 0x22]
      assert ARPTable.size(table) == 1
    end

    test "multiple entries" do
      table = ARPTable.new()
        |> ARPTable.insert("10.0.0.1", [0x11, 0x11, 0x11, 0x11, 0x11, 0x11])
        |> ARPTable.insert("10.0.0.2", [0x22, 0x22, 0x22, 0x22, 0x22, 0x22])
      assert ARPTable.size(table) == 2
    end
  end

  # ===========================================================================
  # Layer 3: IP Tests
  # ===========================================================================

  describe "IPv4Header" do
    test "serialize and deserialize round-trip" do
      header = IPv4Header.new([10, 0, 0, 1], [10, 0, 0, 2], 6, 40, 64)
      bytes = IPv4Header.serialize(header)
      assert length(bytes) == 20

      restored = IPv4Header.deserialize(bytes)
      assert restored.version == 4
      assert restored.ihl == 5
      assert restored.total_length == 40
      assert restored.ttl == 64
      assert restored.protocol == 6
      assert restored.src_ip == [10, 0, 0, 1]
      assert restored.dst_ip == [10, 0, 0, 2]
    end

    test "compute and verify checksum" do
      header = IPv4Header.new([10, 0, 0, 1], [10, 0, 0, 2], 6, 40)
      checksum = IPv4Header.compute_checksum(header)
      assert checksum != 0
      header = %{header | header_checksum: checksum}
      assert IPv4Header.verify_checksum(header) == true
    end

    test "detect corrupted header via checksum" do
      header = IPv4Header.new([10, 0, 0, 1], [10, 0, 0, 2], 6, 40)
      checksum = IPv4Header.compute_checksum(header)
      header = %{header | header_checksum: checksum, ttl: 32}
      assert IPv4Header.verify_checksum(header) == false
    end

    test "UDP protocol number" do
      header = IPv4Header.new([192, 168, 1, 1], [192, 168, 1, 2], 17, 28)
      bytes = IPv4Header.serialize(header)
      restored = IPv4Header.deserialize(bytes)
      assert restored.protocol == 17
    end
  end

  describe "RoutingTable" do
    test "match a route" do
      table = RoutingTable.new()
        |> RoutingTable.add_route([10, 0, 0, 0], [255, 255, 255, 0], [0, 0, 0, 0], "eth0")
      route = RoutingTable.lookup(table, [10, 0, 0, 5])
      assert route != nil
      assert route.iface == "eth0"
    end

    test "no match returns nil" do
      table = RoutingTable.new()
        |> RoutingTable.add_route([10, 0, 0, 0], [255, 255, 255, 0], [0, 0, 0, 0], "eth0")
      assert RoutingTable.lookup(table, [192, 168, 1, 1]) == nil
    end

    test "longest prefix match" do
      table = RoutingTable.new()
        |> RoutingTable.add_route([10, 0, 0, 0], [255, 0, 0, 0], [10, 0, 0, 1], "eth0")
        |> RoutingTable.add_route([10, 0, 1, 0], [255, 255, 255, 0], [10, 0, 1, 1], "eth1")
      route = RoutingTable.lookup(table, [10, 0, 1, 5])
      assert route.iface == "eth1"
      assert route.gateway == [10, 0, 1, 1]
    end

    test "default route (0.0.0.0/0)" do
      table = RoutingTable.new()
        |> RoutingTable.add_route([0, 0, 0, 0], [0, 0, 0, 0], [10, 0, 0, 1], "eth0")
      route = RoutingTable.lookup(table, [8, 8, 8, 8])
      assert route != nil
      assert route.gateway == [10, 0, 0, 1]
    end
  end

  describe "IPLayer" do
    test "create and parse a packet" do
      layer = IPLayer.new([10, 0, 0, 1])
      payload = [0x01, 0x02, 0x03]
      packet = IPLayer.create_packet(layer, [10, 0, 0, 2], 6, payload)

      assert length(packet) == 23

      {:ok, src_ip, protocol, parsed_payload} = IPLayer.parse_packet(packet)
      assert src_ip == [10, 0, 0, 1]
      assert protocol == 6
      assert parsed_payload == payload
    end

    test "reject invalid checksum" do
      layer = IPLayer.new([10, 0, 0, 1])
      packet = IPLayer.create_packet(layer, [10, 0, 0, 2], 6, [0x01])
      # Corrupt a byte
      corrupted = List.update_at(packet, 8, fn b -> Bitwise.bxor(b, 0xFF) end)
      assert IPLayer.parse_packet(corrupted) == :error
    end
  end

  # ===========================================================================
  # Layer 4: TCP Tests
  # ===========================================================================

  describe "TCPHeader" do
    test "serialize and deserialize round-trip" do
      header = TCPHeader.new(49152, 80, seq_num: 1000, flags: TCPFlags.syn())
      bytes = TCPHeader.serialize(header)
      assert length(bytes) == 20

      restored = TCPHeader.deserialize(bytes)
      assert restored.src_port == 49152
      assert restored.dst_port == 80
      assert restored.seq_num == 1000
      assert restored.ack_num == 0
      assert restored.flags == TCPFlags.syn()
      assert restored.window_size == 65535
      assert restored.data_offset == 5
    end

    test "SYN+ACK flags" do
      flags = Bitwise.bor(TCPFlags.syn(), TCPFlags.ack())
      header = TCPHeader.new(80, 49152, seq_num: 3000, ack_num: 1001, flags: flags)
      bytes = TCPHeader.serialize(header)
      restored = TCPHeader.deserialize(bytes)
      assert restored.flags == flags
      assert restored.seq_num == 3000
      assert restored.ack_num == 1001
    end

    test "all flag combinations" do
      all_flags = Bitwise.bor(TCPFlags.fin(),
        Bitwise.bor(TCPFlags.syn(),
          Bitwise.bor(TCPFlags.rst(),
            Bitwise.bor(TCPFlags.psh(), TCPFlags.ack()))))
      header = TCPHeader.new(1000, 2000, flags: all_flags)
      bytes = TCPHeader.serialize(header)
      restored = TCPHeader.deserialize(bytes)
      assert restored.flags == all_flags
    end
  end

  describe "TCPConnection — Three-Way Handshake" do
    test "complete handshake" do
      client = TCPConnection.new(local_port: 49152, remote_port: 80,
        local_ip: "10.0.0.1", remote_ip: "10.0.0.2")
      server = TCPConnection.new(local_port: 80,
        local_ip: "10.0.0.2", remote_ip: "10.0.0.1")

      # Server listens
      server = TCPConnection.set_listen(server)
      assert server.state == :listen

      # Client sends SYN
      {client, syn} = TCPConnection.initiate_connect(client)
      assert syn != nil
      assert TCPFlags.has_flag?(syn.flags, TCPFlags.syn())
      assert client.state == :syn_sent

      # Server receives SYN, sends SYN+ACK
      {server, syn_ack} = TCPConnection.handle_segment(server, syn)
      assert syn_ack != nil
      assert TCPFlags.has_flag?(syn_ack.flags, TCPFlags.syn())
      assert TCPFlags.has_flag?(syn_ack.flags, TCPFlags.ack())
      assert server.state == :syn_received

      # Client receives SYN+ACK, sends ACK
      {client, final_ack} = TCPConnection.handle_segment(client, syn_ack)
      assert final_ack != nil
      assert client.state == :established

      # Server receives ACK
      {server, nil_resp} = TCPConnection.handle_segment(server, final_ack)
      assert nil_resp == nil
      assert server.state == :established
    end

    test "cannot connect from non-CLOSED state" do
      conn = %{TCPConnection.new() | state: :established}
      {_conn, result} = TCPConnection.initiate_connect(conn)
      assert result == nil
    end
  end

  describe "TCPConnection — Data Transfer" do
    defp setup_connected do
      client = TCPConnection.new(local_port: 49152, remote_port: 80,
        local_ip: "10.0.0.1", remote_ip: "10.0.0.2")
      server = TCPConnection.new(local_port: 80,
        local_ip: "10.0.0.2", remote_ip: "10.0.0.1")
      server = TCPConnection.set_listen(server)
      {client, syn} = TCPConnection.initiate_connect(client)
      {server, syn_ack} = TCPConnection.handle_segment(server, syn)
      {client, final_ack} = TCPConnection.handle_segment(client, syn_ack)
      {server, _} = TCPConnection.handle_segment(server, final_ack)
      {client, server}
    end

    test "send and recv data" do
      {client, server} = setup_connected()

      data = [72, 101, 108, 108, 111]
      {client, header, sent_data} = TCPConnection.send_data(client, data)
      assert header != nil
      assert sent_data == data

      {server, _ack} = TCPConnection.handle_segment(server, header, sent_data)
      {received, _server} = TCPConnection.recv_data(server)
      assert received == data

      # Verify sequence number advanced
      assert client.send_seq > 1001
    end

    test "cannot send in non-ESTABLISHED state" do
      conn = TCPConnection.new()
      {_conn, header, _data} = TCPConnection.send_data(conn, [1, 2, 3])
      assert header == nil
    end

    test "empty recv buffer" do
      {_client, server} = setup_connected()
      {data, _server} = TCPConnection.recv_data(server)
      assert data == []
    end
  end

  describe "TCPConnection — Connection Close" do
    defp setup_connected_for_close do
      client = TCPConnection.new(local_port: 49152, remote_port: 80,
        local_ip: "10.0.0.1", remote_ip: "10.0.0.2")
      server = TCPConnection.new(local_port: 80,
        local_ip: "10.0.0.2", remote_ip: "10.0.0.1")
      server = TCPConnection.set_listen(server)
      {client, syn} = TCPConnection.initiate_connect(client)
      {server, syn_ack} = TCPConnection.handle_segment(server, syn)
      {client, final_ack} = TCPConnection.handle_segment(client, syn_ack)
      {server, _} = TCPConnection.handle_segment(server, final_ack)
      {client, server}
    end

    test "four-way close" do
      {client, server} = setup_connected_for_close()

      # Client initiates close
      {client, fin} = TCPConnection.initiate_close(client)
      assert fin != nil
      assert TCPFlags.has_flag?(fin.flags, TCPFlags.fin())
      assert client.state == :fin_wait_1

      # Server receives FIN, sends ACK
      {server, fin_ack} = TCPConnection.handle_segment(server, fin)
      assert fin_ack != nil
      assert server.state == :close_wait

      # Client receives ACK
      {client, _} = TCPConnection.handle_segment(client, fin_ack)
      assert client.state == :fin_wait_2

      # Server sends FIN
      {server, server_fin} = TCPConnection.initiate_close(server)
      assert server_fin != nil
      assert server.state == :last_ack

      # Client receives server FIN
      {client, final_ack} = TCPConnection.handle_segment(client, server_fin)
      assert final_ack != nil
      assert client.state == :time_wait

      # Server receives final ACK
      {server, _} = TCPConnection.handle_segment(server, final_ack)
      assert server.state == :closed
    end

    test "cannot close from CLOSED state" do
      conn = TCPConnection.new()
      {_conn, result} = TCPConnection.initiate_close(conn)
      assert result == nil
    end
  end

  # ===========================================================================
  # Layer 4: UDP Tests
  # ===========================================================================

  describe "UDPHeader" do
    test "serialize and deserialize round-trip" do
      header = UDPHeader.new(12345, 53, 12)
      bytes = UDPHeader.serialize(header)
      assert length(bytes) == 8

      restored = UDPHeader.deserialize(bytes)
      assert restored.src_port == 12345
      assert restored.dst_port == 53
      assert restored.length == 20  # 8 header + 12 data
      assert restored.checksum == 0
    end

    test "non-zero checksum" do
      header = %{UDPHeader.new(1000, 2000, 8) | checksum: 0xABCD}
      bytes = UDPHeader.serialize(header)
      restored = UDPHeader.deserialize(bytes)
      assert restored.checksum == 0xABCD
    end
  end

  describe "UDPSocket" do
    test "send_to produces correct header" do
      sock = UDPSocket.new(12345)
      {header, data} = UDPSocket.send_to(sock, [1, 2, 3], 53)
      assert header.src_port == 12345
      assert header.dst_port == 53
      assert header.length == 11
      assert data == [1, 2, 3]
    end

    test "deliver and receive_from" do
      sock = UDPSocket.new(53)
      sock = UDPSocket.deliver(sock, [10, 20, 30], "10.0.0.1", 12345)

      {result, _sock} = UDPSocket.receive_from(sock)
      assert result != nil
      assert result.data == [10, 20, 30]
      assert result.src_ip == "10.0.0.1"
      assert result.src_port == 12345
    end

    test "empty queue returns nil" do
      sock = UDPSocket.new(53)
      {result, _sock} = UDPSocket.receive_from(sock)
      assert result == nil
    end

    test "multiple datagrams in order" do
      sock = UDPSocket.new(53)
        |> UDPSocket.deliver([1], "10.0.0.1", 1000)
        |> UDPSocket.deliver([2], "10.0.0.2", 2000)

      {first, sock} = UDPSocket.receive_from(sock)
      assert first.data == [1]
      assert first.src_port == 1000

      {second, _sock} = UDPSocket.receive_from(sock)
      assert second.data == [2]
      assert second.src_port == 2000
    end
  end

  # ===========================================================================
  # Socket API Tests
  # ===========================================================================

  describe "SocketManager" do
    test "create sockets with unique fds" do
      mgr = SocketManager.new()
      {fd1, mgr} = SocketManager.create_socket(mgr, :stream)
      {fd2, _mgr} = SocketManager.create_socket(mgr, :dgram)
      assert fd1 != fd2
      assert fd1 >= 3
    end

    test "bind to a port" do
      mgr = SocketManager.new()
      {fd, mgr} = SocketManager.create_socket(mgr, :stream)
      {:ok, mgr} = SocketManager.bind(mgr, fd, "10.0.0.1", 80)

      {:ok, sock} = SocketManager.get_socket(mgr, fd)
      assert sock.local_ip == "10.0.0.1"
      assert sock.local_port == 80
    end

    test "reject duplicate port binding" do
      mgr = SocketManager.new()
      {fd1, mgr} = SocketManager.create_socket(mgr, :stream)
      {fd2, mgr} = SocketManager.create_socket(mgr, :stream)
      {:ok, mgr} = SocketManager.bind(mgr, fd1, "10.0.0.1", 80)
      {:error, :port_in_use} = SocketManager.bind(mgr, fd2, "10.0.0.2", 80)
    end

    test "listen on TCP socket" do
      mgr = SocketManager.new()
      {fd, mgr} = SocketManager.create_socket(mgr, :stream)
      {:ok, mgr} = SocketManager.bind(mgr, fd, "10.0.0.1", 80)
      {:ok, mgr} = SocketManager.listen(mgr, fd)
      {:ok, sock} = SocketManager.get_socket(mgr, fd)
      assert sock.is_listening == true
    end

    test "cannot listen on UDP socket" do
      mgr = SocketManager.new()
      {fd, mgr} = SocketManager.create_socket(mgr, :dgram)
      {:error, :not_stream} = SocketManager.listen(mgr, fd)
    end

    test "connect returns SYN" do
      mgr = SocketManager.new()
      {fd, mgr} = SocketManager.create_socket(mgr, :stream)
      {:ok, mgr} = SocketManager.bind(mgr, fd, "10.0.0.1", 49152)
      {:ok, syn, _mgr} = SocketManager.connect(mgr, fd, "10.0.0.2", 80)
      assert syn != nil
      assert TCPFlags.has_flag?(syn.flags, TCPFlags.syn())
    end

    test "close and free port" do
      mgr = SocketManager.new()
      {fd, mgr} = SocketManager.create_socket(mgr, :stream)
      {:ok, mgr} = SocketManager.bind(mgr, fd, "10.0.0.1", 80)
      {_fin, mgr} = SocketManager.close(mgr, fd)

      # Port should be free again
      {fd2, mgr} = SocketManager.create_socket(mgr, :stream)
      {:ok, _mgr} = SocketManager.bind(mgr, fd2, "10.0.0.1", 80)
    end

    test "invalid fd returns error" do
      mgr = SocketManager.new()
      {:error, :not_found} = SocketManager.get_socket(mgr, 999)
    end

    test "accept incoming connection" do
      mgr = SocketManager.new()
      {fd, mgr} = SocketManager.create_socket(mgr, :stream)
      {:ok, mgr} = SocketManager.bind(mgr, fd, "10.0.0.2", 80)
      {:ok, mgr} = SocketManager.listen(mgr, fd)

      # Simulate incoming connection
      conn = %{TCPConnection.new(local_port: 80, remote_port: 49152,
        local_ip: "10.0.0.2", remote_ip: "10.0.0.1") | state: :established}
      {:ok, sock} = SocketManager.get_socket(mgr, fd)
      sock = %{sock | accept_queue: [conn]}
      mgr = %{mgr | sockets: Map.put(mgr.sockets, fd, sock)}

      {:ok, new_fd, mgr} = SocketManager.accept(mgr, fd)
      {:ok, new_sock} = SocketManager.get_socket(mgr, new_fd)
      assert new_sock.remote_ip == "10.0.0.1"
      assert new_sock.remote_port == 49152
    end

    test "accept on empty queue returns error" do
      mgr = SocketManager.new()
      {fd, mgr} = SocketManager.create_socket(mgr, :stream)
      {:ok, mgr} = SocketManager.bind(mgr, fd, "10.0.0.2", 80)
      {:ok, mgr} = SocketManager.listen(mgr, fd)
      {:error, :no_pending} = SocketManager.accept(mgr, fd)
    end
  end

  # ===========================================================================
  # Layer 7: DNS Tests
  # ===========================================================================

  describe "DNSResolver" do
    test "resolve localhost by default" do
      dns = DNSResolver.new()
      assert DNSResolver.resolve(dns, "localhost") == [127, 0, 0, 1]
    end

    test "unknown hostname returns nil" do
      dns = DNSResolver.new()
      assert DNSResolver.resolve(dns, "unknown.com") == nil
    end

    test "add and resolve static entries" do
      dns = DNSResolver.new()
        |> DNSResolver.add_static("example.com", [93, 184, 216, 34])
      assert DNSResolver.resolve(dns, "example.com") == [93, 184, 216, 34]
    end

    test "overwrite existing entries" do
      dns = DNSResolver.new()
        |> DNSResolver.add_static("example.com", [1, 1, 1, 1])
        |> DNSResolver.add_static("example.com", [2, 2, 2, 2])
      assert DNSResolver.resolve(dns, "example.com") == [2, 2, 2, 2]
    end
  end

  # ===========================================================================
  # Layer 7: HTTP Tests
  # ===========================================================================

  describe "HTTPRequest" do
    test "serialize a GET request" do
      req = HTTPRequest.new("GET", "/index.html", %{"Host" => "example.com"})
      text = HTTPRequest.serialize(req)
      assert String.contains?(text, "GET /index.html HTTP/1.1\r\n")
      assert String.contains?(text, "Host: example.com\r\n")
    end

    test "serialize a POST with body" do
      req = HTTPRequest.new("POST", "/api",
        %{"Content-Type" => "text/plain", "Content-Length" => "5"}, "hello")
      text = HTTPRequest.serialize(req)
      assert String.contains?(text, "POST /api HTTP/1.1\r\n")
      assert String.contains?(text, "hello")
    end

    test "deserialize a GET request" do
      text = "GET /path HTTP/1.1\r\nHost: example.com\r\n\r\n"
      req = HTTPRequest.deserialize(text)
      assert req.method == "GET"
      assert req.path == "/path"
      assert req.headers["Host"] == "example.com"
    end

    test "round-trip" do
      original = HTTPRequest.new("GET", "/", %{"Host" => "example.com"})
      restored = HTTPRequest.deserialize(HTTPRequest.serialize(original))
      assert restored.method == "GET"
      assert restored.path == "/"
      assert restored.headers["Host"] == "example.com"
    end

    test "request with body" do
      text = "POST /data HTTP/1.1\r\nContent-Length: 3\r\n\r\nabc"
      req = HTTPRequest.deserialize(text)
      assert req.method == "POST"
      assert req.body == "abc"
    end
  end

  describe "HTTPResponse" do
    test "serialize 200 OK" do
      resp = HTTPResponse.new(200, "OK", %{"Content-Type" => "text/html"}, "Hello, World!")
      text = HTTPResponse.serialize(resp)
      assert String.contains?(text, "HTTP/1.1 200 OK\r\n")
      assert String.contains?(text, "Hello, World!")
    end

    test "deserialize a response" do
      text = "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\n\r\nNot Found"
      resp = HTTPResponse.deserialize(text)
      assert resp.status_code == 404
      assert resp.status_text == "Not Found"
      assert resp.body == "Not Found"
      assert resp.headers["Content-Type"] == "text/plain"
    end

    test "round-trip" do
      original = HTTPResponse.new(200, "OK", %{"Content-Length" => "5"}, "hello")
      restored = HTTPResponse.deserialize(HTTPResponse.serialize(original))
      assert restored.status_code == 200
      assert restored.status_text == "OK"
      assert restored.body == "hello"
    end

    test "empty body" do
      resp = HTTPResponse.new(204, "No Content")
      text = HTTPResponse.serialize(resp)
      restored = HTTPResponse.deserialize(text)
      assert restored.status_code == 204
      assert restored.body == ""
    end
  end

  describe "HTTPClient" do
    test "build request from URL" do
      {request, host, port} = HTTPClient.build_request("http://example.com/path")
      assert request.method == "GET"
      assert request.path == "/path"
      assert request.headers["Host"] == "example.com"
      assert host == "example.com"
      assert port == 80
    end

    test "URL with port" do
      {_request, host, port} = HTTPClient.build_request("http://example.com:8080/api")
      assert host == "example.com"
      assert port == 8080
    end

    test "default to / path" do
      {request, _host, _port} = HTTPClient.build_request("http://example.com")
      assert request.path == "/"
    end

    test "parse response" do
      text = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<html></html>"
      resp = HTTPClient.parse_response(text)
      assert resp.status_code == 200
      assert resp.body == "<html></html>"
    end
  end

  # ===========================================================================
  # NetworkWire Tests
  # ===========================================================================

  describe "NetworkWire" do
    test "deliver data from A to B" do
      wire = NetworkWire.new()

      NetworkWire.send_a(wire, [1, 2, 3])
      assert NetworkWire.has_data_for_b?(wire) == true
      assert NetworkWire.has_data_for_a?(wire) == false

      data = NetworkWire.receive_b(wire)
      assert data == [1, 2, 3]

      NetworkWire.stop(wire)
    end

    test "deliver data from B to A" do
      wire = NetworkWire.new()

      NetworkWire.send_b(wire, [4, 5, 6])
      assert NetworkWire.has_data_for_a?(wire) == true
      assert NetworkWire.has_data_for_b?(wire) == false

      data = NetworkWire.receive_a(wire)
      assert data == [4, 5, 6]

      NetworkWire.stop(wire)
    end

    test "bidirectional communication" do
      wire = NetworkWire.new()

      NetworkWire.send_a(wire, [1, 2, 3])
      NetworkWire.send_b(wire, [4, 5, 6])

      assert NetworkWire.receive_b(wire) == [1, 2, 3]
      assert NetworkWire.receive_a(wire) == [4, 5, 6]

      NetworkWire.stop(wire)
    end

    test "empty wire returns nil" do
      wire = NetworkWire.new()

      assert NetworkWire.receive_a(wire) == nil
      assert NetworkWire.receive_b(wire) == nil

      NetworkWire.stop(wire)
    end

    test "FIFO order" do
      wire = NetworkWire.new()

      NetworkWire.send_a(wire, [1])
      NetworkWire.send_a(wire, [2])
      NetworkWire.send_a(wire, [3])

      assert NetworkWire.receive_b(wire) == [1]
      assert NetworkWire.receive_b(wire) == [2]
      assert NetworkWire.receive_b(wire) == [3]

      NetworkWire.stop(wire)
    end
  end
end
