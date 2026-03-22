defmodule CodingAdventures.NetworkStack do
  @moduledoc """
  # Network Stack — From Ethernet Frames to HTTP Requests

  This module implements a complete networking stack, covering every layer of
  the TCP/IP model:

      Layer 7: HTTP         — "What are we saying?"
      Layer 7: DNS          — "What is the IP for this hostname?"
      Layer 4: TCP          — "How do we ensure reliable delivery?"
      Layer 4: UDP          — "How do we send fast, unreliable datagrams?"
      Layer 3: IP           — "How do we route across networks?"
      Layer 2: Ethernet     — "How do we talk to the next hop?"
      Layer 1: NetworkWire  — "How do we transmit bits?" (simulated)

  ## The Postal Analogy

  Sending data over a network is like sending a letter:
  - **Ethernet** is the local mail carrier — delivers between houses on the same street.
  - **IP** is the postal routing system — figures out which city and post office.
  - **TCP** is registered mail with tracking — guarantees delivery and order.
  - **UDP** is a postcard — fast, no guarantee it arrives.
  - **HTTP** is the letter itself — "Dear Server, please send me the homepage."

  ## Packet Encapsulation

  As data moves down the stack, each layer wraps the data from above in its
  own header. On the receiving side, each layer strips its header and passes
  the payload up. This is the fundamental principle of layered networking.

      Application:  "Hello, World!"
      HTTP layer:   GET / HTTP/1.1\\r\\nHost: example.com\\r\\n\\r\\nHello, World!
      TCP layer:    [TCP Header] + HTTP data
      IP layer:     [IP Header] + TCP segment
      Ethernet:     [Eth Header] + IP packet
      Wire:         raw bytes
  """

  # ===========================================================================
  # LAYER 2: ETHERNET — Local Delivery
  # ===========================================================================

  defmodule EthernetFrame do
    @moduledoc """
    ## Ethernet Frames — The Envelope for Local Delivery

    Every network interface card (NIC) has a unique 48-bit MAC address burned
    in at the factory. Ethernet frames carry data between devices on the same
    local network segment.

    A MAC address is 6 bytes, like: [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF].
    The broadcast address [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] reaches every
    device on the local network.

    The `ether_type` field tells the receiver how to interpret the payload:
      - 0x0800 = IPv4 (an IP packet is inside)
      - 0x0806 = ARP  (an address resolution request/reply is inside)
    """

    @enforce_keys [:dest_mac, :src_mac, :ether_type, :payload]
    defstruct [:dest_mac, :src_mac, :ether_type, :payload]

    @type t :: %__MODULE__{
            dest_mac: [non_neg_integer()],
            src_mac: [non_neg_integer()],
            ether_type: non_neg_integer(),
            payload: [non_neg_integer()]
          }

    @doc """
    Create a new Ethernet frame.
    """
    def new(dest_mac, src_mac, ether_type, payload) do
      %__MODULE__{
        dest_mac: dest_mac,
        src_mac: src_mac,
        ether_type: ether_type,
        payload: payload
      }
    end

    @doc """
    Serialize the frame to bytes for transmission over the wire.

    Wire format:
        [6 bytes dest_mac][6 bytes src_mac][2 bytes ether_type][N bytes payload]

    The ether_type is stored in big-endian (network byte order), which is the
    standard for all network protocols. Big-endian means the most significant
    byte comes first.
    """
    def serialize(%__MODULE__{} = frame) do
      ether_high = Bitwise.band(Bitwise.bsr(frame.ether_type, 8), 0xFF)
      ether_low = Bitwise.band(frame.ether_type, 0xFF)
      frame.dest_mac ++ frame.src_mac ++ [ether_high, ether_low] ++ frame.payload
    end

    @doc """
    Deserialize bytes received from the wire back into a structured frame.

    We read the fixed-size header fields first (6 + 6 + 2 = 14 bytes),
    then everything remaining is the payload.
    """
    def deserialize(bytes) when is_list(bytes) do
      {dest_mac, remaining} = Enum.split(bytes, 6)
      {src_mac, remaining} = Enum.split(remaining, 6)
      {[ether_high, ether_low], payload} = Enum.split(remaining, 2)
      ether_type = Bitwise.bor(Bitwise.bsl(ether_high, 8), ether_low)
      new(dest_mac, src_mac, ether_type, payload)
    end
  end

  # ===========================================================================
  # ARP Table — Bridging IP Addresses and MAC Addresses
  # ===========================================================================

  defmodule ARPTable do
    @moduledoc """
    ## ARP Table — IP-to-MAC Address Resolution Cache

    When a computer wants to send an IP packet to 192.168.1.5, it needs the MAC
    address of that device's network card. ARP (Address Resolution Protocol)
    maintains a table of IP-to-MAC mappings.

    In Elixir, we model this as an immutable map. Each "mutation" returns a new
    map — this is the functional programming way.
    """

    @type t :: %{String.t() => [non_neg_integer()]}

    @doc "Create a new empty ARP table."
    def new, do: %{}

    @doc "Look up the MAC address for a given IP. Returns nil if unknown."
    def lookup(table, ip), do: Map.get(table, ip)

    @doc "Insert or update an IP-to-MAC mapping. Returns the updated table."
    def insert(table, ip, mac), do: Map.put(table, ip, mac)

    @doc "Return the number of entries."
    def size(table), do: map_size(table)
  end

  # ===========================================================================
  # LAYER 3: IP — Routing Across Networks
  # ===========================================================================

  defmodule IPv4Header do
    @moduledoc """
    ## IPv4 Header — The Routing Label

    IP (Internet Protocol) is the backbone of the Internet. Every device has
    an IP address, and IP headers tell routers where to send each packet.

    The IPv4 header is exactly 20 bytes (when IHL=5, meaning no options):

        Byte 0:     version (4 bits) + IHL (4 bits)
        Byte 1:     Type of Service (we ignore)
        Byte 2-3:   Total Length
        Byte 4-7:   Identification, Flags, Fragment Offset (all 0)
        Byte 8:     TTL (Time To Live)
        Byte 9:     Protocol (6=TCP, 17=UDP)
        Byte 10-11: Header Checksum
        Byte 12-15: Source IP Address
        Byte 16-19: Destination IP Address
    """

    @enforce_keys [:src_ip, :dst_ip, :protocol, :total_length]
    defstruct [
      :src_ip,
      :dst_ip,
      :protocol,
      :total_length,
      version: 4,
      ihl: 5,
      ttl: 64,
      header_checksum: 0
    ]

    @type t :: %__MODULE__{
            version: non_neg_integer(),
            ihl: non_neg_integer(),
            total_length: non_neg_integer(),
            ttl: non_neg_integer(),
            protocol: non_neg_integer(),
            header_checksum: non_neg_integer(),
            src_ip: [non_neg_integer()],
            dst_ip: [non_neg_integer()]
          }

    def new(src_ip, dst_ip, protocol, total_length, ttl \\ 64) do
      %__MODULE__{
        src_ip: src_ip,
        dst_ip: dst_ip,
        protocol: protocol,
        total_length: total_length,
        ttl: ttl
      }
    end

    @doc """
    Serialize the IPv4 header to 20 bytes.

    The first byte packs both version and IHL into a single byte:
    (version <<< 4) ||| ihl. Since version=4 and ihl=5, this gives 0x45.
    """
    def serialize(%__MODULE__{} = h) do
      [
        Bitwise.bor(Bitwise.bsl(h.version, 4), h.ihl),
        0,
        Bitwise.band(Bitwise.bsr(h.total_length, 8), 0xFF),
        Bitwise.band(h.total_length, 0xFF),
        0, 0, 0, 0,
        h.ttl,
        h.protocol,
        Bitwise.band(Bitwise.bsr(h.header_checksum, 8), 0xFF),
        Bitwise.band(h.header_checksum, 0xFF),
        Enum.at(h.src_ip, 0), Enum.at(h.src_ip, 1),
        Enum.at(h.src_ip, 2), Enum.at(h.src_ip, 3),
        Enum.at(h.dst_ip, 0), Enum.at(h.dst_ip, 1),
        Enum.at(h.dst_ip, 2), Enum.at(h.dst_ip, 3)
      ]
    end

    @doc "Deserialize 20 bytes back into an IPv4 header."
    def deserialize(bytes) when is_list(bytes) do
      [b0, _b1, b2, b3, _b4, _b5, _b6, _b7, b8, b9, b10, b11,
       s0, s1, s2, s3, d0, d1, d2, d3] = Enum.take(bytes, 20)

      version = Bitwise.band(Bitwise.bsr(b0, 4), 0x0F)
      ihl = Bitwise.band(b0, 0x0F)
      total_length = Bitwise.bor(Bitwise.bsl(b2, 8), b3)
      checksum = Bitwise.bor(Bitwise.bsl(b10, 8), b11)

      %__MODULE__{
        version: version,
        ihl: ihl,
        total_length: total_length,
        ttl: b8,
        protocol: b9,
        header_checksum: checksum,
        src_ip: [s0, s1, s2, s3],
        dst_ip: [d0, d1, d2, d3]
      }
    end

    @doc """
    ## IP Checksum — Ones' Complement Sum

    The IP checksum catches accidental bit flips during transmission.

    Algorithm:
    1. Treat the header as a sequence of 16-bit words.
    2. Set the checksum field to 0 before computing.
    3. Sum all words. If the sum overflows 16 bits, fold the carry back.
    4. Take the bitwise NOT (ones' complement) of the result.
    """
    def compute_checksum(%__MODULE__{} = header) do
      bytes = serialize(%{header | header_checksum: 0})
      sum = sum_words(bytes, 0)
      sum = fold_carry(sum)
      Bitwise.band(Bitwise.bnot(sum), 0xFFFF)
    end

    @doc """
    Verify the checksum of a received header. A valid header produces
    a result of 0xFFFF when all bytes (including checksum) are summed.
    """
    def verify_checksum(%__MODULE__{} = header) do
      bytes = serialize(header)
      sum = sum_words(bytes, 0)
      sum = fold_carry(sum)
      Bitwise.band(sum, 0xFFFF) == 0xFFFF
    end

    defp sum_words([], acc), do: acc
    defp sum_words([high, low | rest], acc) do
      word = Bitwise.bor(Bitwise.bsl(high, 8), low)
      sum_words(rest, acc + word)
    end

    defp fold_carry(sum) when sum > 0xFFFF do
      fold_carry(Bitwise.band(sum, 0xFFFF) + Bitwise.bsr(sum, 16))
    end
    defp fold_carry(sum), do: sum
  end

  # ===========================================================================
  # Routing Table
  # ===========================================================================

  defmodule RouteEntry do
    @moduledoc "A single entry in the routing table."
    defstruct [:network, :mask, :gateway, :iface]

    @type t :: %__MODULE__{
            network: [non_neg_integer()],
            mask: [non_neg_integer()],
            gateway: [non_neg_integer()],
            iface: String.t()
          }
  end

  defmodule RoutingTable do
    @moduledoc """
    ## Routing Table — Finding the Path

    Each entry says: "If the destination matches this network/mask, send it
    to this gateway via this interface."

    The key algorithm is **longest prefix match**: when multiple routes match,
    pick the most specific one (the one with the longest subnet mask).
    """

    @type t :: [RouteEntry.t()]

    @doc "Create a new empty routing table."
    def new, do: []

    @doc "Add a route entry."
    def add_route(table, network, mask, gateway, iface) do
      entry = %RouteEntry{network: network, mask: mask, gateway: gateway, iface: iface}
      table ++ [entry]
    end

    @doc """
    Find the best route for a destination IP using longest prefix match.

    For each entry, we AND the destination with the mask and check if the
    result equals the network. Among matches, we pick the most specific.
    """
    def lookup(table, dst_ip) do
      table
      |> Enum.filter(fn entry -> matches?(dst_ip, entry) end)
      |> Enum.max_by(fn entry -> count_mask_bits(entry.mask) end, fn -> nil end)
    end

    defp matches?(dst_ip, %RouteEntry{network: network, mask: mask}) do
      Enum.zip([dst_ip, mask, network])
      |> Enum.all?(fn {d, m, n} -> Bitwise.band(d, m) == n end)
    end

    defp count_mask_bits(mask) do
      mask
      |> Enum.map(fn byte -> count_bits(byte, 0) end)
      |> Enum.sum()
    end

    defp count_bits(0, acc), do: acc
    defp count_bits(b, acc) do
      count_bits(Bitwise.bsr(b, 1), acc + Bitwise.band(b, 1))
    end
  end

  # ===========================================================================
  # IP Layer
  # ===========================================================================

  defmodule IPLayer do
    @moduledoc """
    ## IP Layer — The Routing Engine

    Sits between Ethernet (below) and TCP/UDP (above). Handles building IP
    packets with correct headers/checksums and parsing incoming packets.
    """

    defstruct [:local_ip, :routing_table, :arp_table]

    @type t :: %__MODULE__{
            local_ip: [non_neg_integer()],
            routing_table: RoutingTable.t(),
            arp_table: ARPTable.t()
          }

    def new(local_ip) do
      %__MODULE__{
        local_ip: local_ip,
        routing_table: RoutingTable.new(),
        arp_table: ARPTable.new()
      }
    end

    @doc "Create an IP packet ready for transmission."
    def create_packet(%__MODULE__{} = layer, dst_ip, protocol, payload) do
      total_length = 20 + length(payload)
      header = IPv4Header.new(layer.local_ip, dst_ip, protocol, total_length)
      checksum = IPv4Header.compute_checksum(header)
      header = %{header | header_checksum: checksum}
      IPv4Header.serialize(header) ++ payload
    end

    @doc """
    Parse a received IP packet. Returns {:ok, src_ip, protocol, payload}
    or :error if checksum is invalid.
    """
    def parse_packet(bytes) do
      header_bytes = Enum.take(bytes, 20)
      header = IPv4Header.deserialize(header_bytes)

      if IPv4Header.verify_checksum(header) do
        payload = Enum.drop(bytes, 20)
        {:ok, header.src_ip, header.protocol, payload}
      else
        :error
      end
    end
  end

  # ===========================================================================
  # LAYER 4: TCP — Reliable, Ordered Delivery
  # ===========================================================================

  defmodule TCPState do
    @moduledoc """
    ## TCP States — The Connection Lifecycle

    A TCP connection is a state machine with 11 states:

        CLOSED -> SYN_SENT -> ESTABLISHED -> FIN_WAIT_1 -> FIN_WAIT_2 -> TIME_WAIT -> CLOSED
        CLOSED -> LISTEN -> SYN_RECEIVED -> ESTABLISHED -> CLOSE_WAIT -> LAST_ACK -> CLOSED
    """

    @type t ::
            :closed
            | :listen
            | :syn_sent
            | :syn_received
            | :established
            | :fin_wait_1
            | :fin_wait_2
            | :close_wait
            | :last_ack
            | :time_wait
            | :closing
  end

  defmodule TCPFlags do
    @moduledoc """
    TCP control flags — each is a single bit in the flags field.

        FIN (0x01) — "I am done sending"
        SYN (0x02) — "Let's synchronize sequence numbers"
        RST (0x04) — "Abort this connection"
        PSH (0x08) — "Push data to application immediately"
        ACK (0x10) — "The ack_num field is valid"
    """
    def fin, do: 0x01
    def syn, do: 0x02
    def rst, do: 0x04
    def psh, do: 0x08
    def ack, do: 0x10

    @doc "Check if a specific flag is set."
    def has_flag?(flags, flag), do: Bitwise.band(flags, flag) != 0
  end

  defmodule TCPHeader do
    @moduledoc """
    ## TCP Header — 20 Bytes of Connection Management

    Carries everything needed for reliable delivery: port numbers,
    sequence/acknowledgment numbers, flags, and window size.
    """

    @enforce_keys [:src_port, :dst_port]
    defstruct [
      :src_port,
      :dst_port,
      seq_num: 0,
      ack_num: 0,
      data_offset: 5,
      flags: 0,
      window_size: 65535
    ]

    @type t :: %__MODULE__{
            src_port: non_neg_integer(),
            dst_port: non_neg_integer(),
            seq_num: non_neg_integer(),
            ack_num: non_neg_integer(),
            data_offset: non_neg_integer(),
            flags: non_neg_integer(),
            window_size: non_neg_integer()
          }

    def new(src_port, dst_port, opts \\ []) do
      %__MODULE__{
        src_port: src_port,
        dst_port: dst_port,
        seq_num: Keyword.get(opts, :seq_num, 0),
        ack_num: Keyword.get(opts, :ack_num, 0),
        flags: Keyword.get(opts, :flags, 0),
        window_size: Keyword.get(opts, :window_size, 65535)
      }
    end

    @doc """
    Serialize the TCP header to 20 bytes.

        Bytes 0-1:   Source port
        Bytes 2-3:   Destination port
        Bytes 4-7:   Sequence number (32-bit)
        Bytes 8-11:  Acknowledgment number (32-bit)
        Byte 12:     Data offset (upper 4 bits)
        Byte 13:     Flags
        Bytes 14-15: Window size
        Bytes 16-19: Checksum + urgent pointer (0)
    """
    def serialize(%__MODULE__{} = h) do
      [
        Bitwise.band(Bitwise.bsr(h.src_port, 8), 0xFF),
        Bitwise.band(h.src_port, 0xFF),
        Bitwise.band(Bitwise.bsr(h.dst_port, 8), 0xFF),
        Bitwise.band(h.dst_port, 0xFF),
        Bitwise.band(Bitwise.bsr(h.seq_num, 24), 0xFF),
        Bitwise.band(Bitwise.bsr(h.seq_num, 16), 0xFF),
        Bitwise.band(Bitwise.bsr(h.seq_num, 8), 0xFF),
        Bitwise.band(h.seq_num, 0xFF),
        Bitwise.band(Bitwise.bsr(h.ack_num, 24), 0xFF),
        Bitwise.band(Bitwise.bsr(h.ack_num, 16), 0xFF),
        Bitwise.band(Bitwise.bsr(h.ack_num, 8), 0xFF),
        Bitwise.band(h.ack_num, 0xFF),
        Bitwise.band(Bitwise.bsl(h.data_offset, 4), 0xF0),
        Bitwise.band(h.flags, 0xFF),
        Bitwise.band(Bitwise.bsr(h.window_size, 8), 0xFF),
        Bitwise.band(h.window_size, 0xFF),
        0, 0, 0, 0
      ]
    end

    @doc "Deserialize 20 bytes into a TCPHeader."
    def deserialize(bytes) when is_list(bytes) do
      [b0, b1, b2, b3, b4, b5, b6, b7, b8, b9, b10, b11,
       b12, b13, b14, b15, _b16, _b17, _b18, _b19] = Enum.take(bytes, 20)

      src_port = Bitwise.bor(Bitwise.bsl(b0, 8), b1)
      dst_port = Bitwise.bor(Bitwise.bsl(b2, 8), b3)
      seq_num = Bitwise.bor(
        Bitwise.bor(Bitwise.bsl(b4, 24), Bitwise.bsl(b5, 16)),
        Bitwise.bor(Bitwise.bsl(b6, 8), b7)
      )
      ack_num = Bitwise.bor(
        Bitwise.bor(Bitwise.bsl(b8, 24), Bitwise.bsl(b9, 16)),
        Bitwise.bor(Bitwise.bsl(b10, 8), b11)
      )
      data_offset = Bitwise.band(Bitwise.bsr(b12, 4), 0x0F)
      flags = b13
      window_size = Bitwise.bor(Bitwise.bsl(b14, 8), b15)

      %__MODULE__{
        src_port: src_port,
        dst_port: dst_port,
        seq_num: seq_num,
        ack_num: ack_num,
        data_offset: data_offset,
        flags: flags,
        window_size: window_size
      }
    end
  end

  # ===========================================================================
  # TCP Connection — The State Machine
  # ===========================================================================

  defmodule TCPConnection do
    @moduledoc """
    ## TCP Connection — The State Machine

    Manages the full lifecycle of a single TCP connection using an immutable
    struct. Each operation returns a new connection state and optionally a
    response header.

    In Elixir, we model state transitions by returning updated structs rather
    than mutating fields. This makes the state machine pure and testable.
    """

    defstruct [
      state: :closed,
      local_port: 0,
      remote_port: 0,
      local_ip: "0.0.0.0",
      remote_ip: "0.0.0.0",
      send_seq: 0,
      recv_next: 0,
      send_buffer: [],
      recv_buffer: []
    ]

    @type t :: %__MODULE__{
            state: TCPState.t(),
            local_port: non_neg_integer(),
            remote_port: non_neg_integer(),
            local_ip: String.t(),
            remote_ip: String.t(),
            send_seq: non_neg_integer(),
            recv_next: non_neg_integer(),
            send_buffer: [non_neg_integer()],
            recv_buffer: [non_neg_integer()]
          }

    def new(opts \\ []) do
      %__MODULE__{
        local_port: Keyword.get(opts, :local_port, 0),
        remote_port: Keyword.get(opts, :remote_port, 0),
        local_ip: Keyword.get(opts, :local_ip, "0.0.0.0"),
        remote_ip: Keyword.get(opts, :remote_ip, "0.0.0.0")
      }
    end

    @doc """
    ## Initiating a Connection — The Client Side

    Generates an initial sequence number, sends SYN, transitions to SYN_SENT.
    Returns {updated_conn, syn_header} or {conn, nil} if not in CLOSED state.
    """
    def initiate_connect(%__MODULE__{state: :closed} = conn) do
      conn = %{conn | send_seq: 1000, state: :syn_sent}
      syn = TCPHeader.new(conn.local_port, conn.remote_port,
        seq_num: conn.send_seq, flags: TCPFlags.syn())
      {conn, syn}
    end
    def initiate_connect(conn), do: {conn, nil}

    @doc "Start listening for incoming connections."
    def set_listen(conn), do: %{conn | state: :listen}

    @doc """
    ## Handling Incoming Segments — The Heart of TCP

    Implements the TCP state machine. Returns {updated_conn, response_header_or_nil}.
    """
    def handle_segment(conn, header, data \\ [])

    def handle_segment(%__MODULE__{state: :listen} = conn, header, _data) do
      if TCPFlags.has_flag?(header.flags, TCPFlags.syn()) do
        conn = %{conn |
          remote_port: header.src_port,
          recv_next: header.seq_num + 1,
          send_seq: 3000,
          state: :syn_received
        }
        response = TCPHeader.new(conn.local_port, conn.remote_port,
          seq_num: conn.send_seq,
          ack_num: conn.recv_next,
          flags: Bitwise.bor(TCPFlags.syn(), TCPFlags.ack()))
        {conn, response}
      else
        {conn, nil}
      end
    end

    def handle_segment(%__MODULE__{state: :syn_sent} = conn, header, _data) do
      if TCPFlags.has_flag?(header.flags, TCPFlags.syn()) and
         TCPFlags.has_flag?(header.flags, TCPFlags.ack()) do
        conn = %{conn |
          recv_next: header.seq_num + 1,
          send_seq: conn.send_seq + 1,
          state: :established
        }
        response = TCPHeader.new(conn.local_port, conn.remote_port,
          seq_num: conn.send_seq,
          ack_num: conn.recv_next,
          flags: TCPFlags.ack())
        {conn, response}
      else
        {conn, nil}
      end
    end

    def handle_segment(%__MODULE__{state: :syn_received} = conn, header, _data) do
      if TCPFlags.has_flag?(header.flags, TCPFlags.ack()) do
        conn = %{conn | send_seq: conn.send_seq + 1, state: :established}
        {conn, nil}
      else
        {conn, nil}
      end
    end

    def handle_segment(%__MODULE__{state: :established} = conn, header, data) do
      cond do
        TCPFlags.has_flag?(header.flags, TCPFlags.fin()) ->
          conn = %{conn | recv_next: header.seq_num + 1, state: :close_wait}
          response = TCPHeader.new(conn.local_port, conn.remote_port,
            seq_num: conn.send_seq,
            ack_num: conn.recv_next,
            flags: TCPFlags.ack())
          {conn, response}

        length(data) > 0 ->
          conn = %{conn |
            recv_buffer: conn.recv_buffer ++ data,
            recv_next: header.seq_num + length(data)
          }
          response = TCPHeader.new(conn.local_port, conn.remote_port,
            seq_num: conn.send_seq,
            ack_num: conn.recv_next,
            flags: TCPFlags.ack())
          {conn, response}

        TCPFlags.has_flag?(header.flags, TCPFlags.ack()) ->
          {conn, nil}

        true ->
          {conn, nil}
      end
    end

    def handle_segment(%__MODULE__{state: :fin_wait_1} = conn, header, _data) do
      if TCPFlags.has_flag?(header.flags, TCPFlags.ack()) do
        {%{conn | state: :fin_wait_2}, nil}
      else
        {conn, nil}
      end
    end

    def handle_segment(%__MODULE__{state: :fin_wait_2} = conn, header, _data) do
      if TCPFlags.has_flag?(header.flags, TCPFlags.fin()) do
        conn = %{conn | recv_next: header.seq_num + 1, state: :time_wait}
        response = TCPHeader.new(conn.local_port, conn.remote_port,
          seq_num: conn.send_seq,
          ack_num: conn.recv_next,
          flags: TCPFlags.ack())
        {conn, response}
      else
        {conn, nil}
      end
    end

    def handle_segment(%__MODULE__{state: :last_ack} = conn, header, _data) do
      if TCPFlags.has_flag?(header.flags, TCPFlags.ack()) do
        {%{conn | state: :closed}, nil}
      else
        {conn, nil}
      end
    end

    def handle_segment(conn, _header, _data), do: {conn, nil}

    @doc "Send data over an established connection."
    def send_data(%__MODULE__{state: :established} = conn, data) do
      header = TCPHeader.new(conn.local_port, conn.remote_port,
        seq_num: conn.send_seq,
        ack_num: conn.recv_next,
        flags: Bitwise.bor(TCPFlags.psh(), TCPFlags.ack()))
      conn = %{conn |
        send_buffer: conn.send_buffer ++ data,
        send_seq: conn.send_seq + length(data)
      }
      {conn, header, data}
    end
    def send_data(conn, _data), do: {conn, nil, []}

    @doc "Read data from the recv buffer. Returns {data, updated_conn}."
    def recv_data(conn) do
      {conn.recv_buffer, %{conn | recv_buffer: []}}
    end

    @doc "Initiate connection close by sending FIN."
    def initiate_close(%__MODULE__{state: :established} = conn) do
      header = TCPHeader.new(conn.local_port, conn.remote_port,
        seq_num: conn.send_seq,
        ack_num: conn.recv_next,
        flags: Bitwise.bor(TCPFlags.fin(), TCPFlags.ack()))
      {%{conn | state: :fin_wait_1}, header}
    end
    def initiate_close(%__MODULE__{state: :close_wait} = conn) do
      header = TCPHeader.new(conn.local_port, conn.remote_port,
        seq_num: conn.send_seq,
        ack_num: conn.recv_next,
        flags: Bitwise.bor(TCPFlags.fin(), TCPFlags.ack()))
      {%{conn | state: :last_ack}, header}
    end
    def initiate_close(conn), do: {conn, nil}
  end

  # ===========================================================================
  # LAYER 4: UDP — Fast, Unreliable Datagrams
  # ===========================================================================

  defmodule UDPHeader do
    @moduledoc """
    ## UDP Header — Just 8 Bytes

    No handshake, no acknowledgments, no ordering. Source port, destination
    port, length, and optional checksum — that's it.
    """

    @enforce_keys [:src_port, :dst_port, :length]
    defstruct [:src_port, :dst_port, :length, checksum: 0]

    @type t :: %__MODULE__{
            src_port: non_neg_integer(),
            dst_port: non_neg_integer(),
            length: non_neg_integer(),
            checksum: non_neg_integer()
          }

    def new(src_port, dst_port, data_length, checksum \\ 0) do
      %__MODULE__{
        src_port: src_port,
        dst_port: dst_port,
        length: 8 + data_length,
        checksum: checksum
      }
    end

    @doc "Serialize to 8 bytes."
    def serialize(%__MODULE__{} = h) do
      [
        Bitwise.band(Bitwise.bsr(h.src_port, 8), 0xFF),
        Bitwise.band(h.src_port, 0xFF),
        Bitwise.band(Bitwise.bsr(h.dst_port, 8), 0xFF),
        Bitwise.band(h.dst_port, 0xFF),
        Bitwise.band(Bitwise.bsr(h.length, 8), 0xFF),
        Bitwise.band(h.length, 0xFF),
        Bitwise.band(Bitwise.bsr(h.checksum, 8), 0xFF),
        Bitwise.band(h.checksum, 0xFF)
      ]
    end

    @doc "Deserialize 8 bytes into a UDPHeader."
    def deserialize(bytes) when is_list(bytes) do
      [b0, b1, b2, b3, b4, b5, b6, b7] = Enum.take(bytes, 8)
      %__MODULE__{
        src_port: Bitwise.bor(Bitwise.bsl(b0, 8), b1),
        dst_port: Bitwise.bor(Bitwise.bsl(b2, 8), b3),
        length: Bitwise.bor(Bitwise.bsl(b4, 8), b5),
        checksum: Bitwise.bor(Bitwise.bsl(b6, 8), b7)
      }
    end
  end

  defmodule UDPSocket do
    @moduledoc """
    ## UDP Socket — Connectionless Communication

    A UDP socket has a local port and a queue of received datagrams.
    No connection state — each datagram is independent.
    """

    defstruct [:local_port, recv_queue: []]

    @type t :: %__MODULE__{
            local_port: non_neg_integer(),
            recv_queue: [%{data: [non_neg_integer()], src_ip: String.t(), src_port: non_neg_integer()}]
          }

    def new(local_port), do: %__MODULE__{local_port: local_port}

    @doc "Send a datagram. Returns {header, data}."
    def send_to(%__MODULE__{} = sock, data, dst_port) do
      header = UDPHeader.new(sock.local_port, dst_port, length(data))
      {header, data}
    end

    @doc "Deliver a received datagram into the socket's queue."
    def deliver(%__MODULE__{} = sock, data, src_ip, src_port) do
      entry = %{data: data, src_ip: src_ip, src_port: src_port}
      %{sock | recv_queue: sock.recv_queue ++ [entry]}
    end

    @doc "Receive the next datagram. Returns {entry_or_nil, updated_socket}."
    def receive_from(%__MODULE__{recv_queue: []} = sock), do: {nil, sock}
    def receive_from(%__MODULE__{recv_queue: [head | tail]} = sock) do
      {head, %{sock | recv_queue: tail}}
    end
  end

  # ===========================================================================
  # SOCKET API
  # ===========================================================================

  defmodule SocketType do
    @moduledoc """
    Socket types — STREAM (TCP) or DGRAM (UDP).
    """
    def stream, do: :stream
    def dgram, do: :dgram
  end

  defmodule SocketEntry do
    @moduledoc "A socket entry in the socket manager."
    defstruct [
      :fd,
      :socket_type,
      local_ip: "0.0.0.0",
      local_port: 0,
      remote_ip: "0.0.0.0",
      remote_port: 0,
      tcp_connection: nil,
      udp_socket: nil,
      is_listening: false,
      accept_queue: []
    ]
  end

  defmodule SocketManager do
    @moduledoc """
    ## Socket Manager — The Kernel's Network Interface

    Manages all sockets and provides the familiar Berkeley sockets API.
    In Elixir, the manager is an immutable struct; each operation returns
    an updated manager.
    """

    defstruct [sockets: %{}, next_fd: 3, used_ports: MapSet.new()]

    @type t :: %__MODULE__{
            sockets: %{non_neg_integer() => SocketEntry.t()},
            next_fd: non_neg_integer(),
            used_ports: MapSet.t()
          }

    def new, do: %__MODULE__{}

    @doc "Create a new socket. Returns {fd, updated_manager}."
    def create_socket(%__MODULE__{} = mgr, socket_type) do
      fd = mgr.next_fd
      tcp_conn = if socket_type == :stream, do: TCPConnection.new(), else: nil
      udp_sock = if socket_type == :dgram, do: UDPSocket.new(0), else: nil

      entry = %SocketEntry{
        fd: fd,
        socket_type: socket_type,
        tcp_connection: tcp_conn,
        udp_socket: udp_sock
      }

      mgr = %{mgr |
        sockets: Map.put(mgr.sockets, fd, entry),
        next_fd: mgr.next_fd + 1
      }
      {fd, mgr}
    end

    @doc "Bind a socket to an IP and port. Returns {:ok, mgr} or {:error, reason}."
    def bind(%__MODULE__{} = mgr, fd, ip, port) do
      with {:ok, sock} <- get_socket(mgr, fd),
           false <- MapSet.member?(mgr.used_ports, port) do
        sock = %{sock | local_ip: ip, local_port: port}
        sock = if sock.tcp_connection do
          %{sock | tcp_connection: %{sock.tcp_connection | local_port: port, local_ip: ip}}
        else
          sock
        end
        sock = if sock.udp_socket do
          %{sock | udp_socket: %{sock.udp_socket | local_port: port}}
        else
          sock
        end
        mgr = %{mgr |
          sockets: Map.put(mgr.sockets, fd, sock),
          used_ports: MapSet.put(mgr.used_ports, port)
        }
        {:ok, mgr}
      else
        true -> {:error, :port_in_use}
        {:error, reason} -> {:error, reason}
      end
    end

    @doc "Mark a TCP socket as listening."
    def listen(%__MODULE__{} = mgr, fd) do
      with {:ok, sock} <- get_socket(mgr, fd),
           true <- sock.socket_type == :stream do
        conn = TCPConnection.set_listen(sock.tcp_connection)
        sock = %{sock | is_listening: true, tcp_connection: conn}
        {:ok, %{mgr | sockets: Map.put(mgr.sockets, fd, sock)}}
      else
        false -> {:error, :not_stream}
        {:error, reason} -> {:error, reason}
      end
    end

    @doc "Accept an incoming connection. Returns {:ok, new_fd, mgr} or {:error, reason}."
    def accept(%__MODULE__{} = mgr, fd) do
      with {:ok, sock} <- get_socket(mgr, fd),
           true <- sock.is_listening,
           [conn | rest] <- sock.accept_queue do
        new_fd = mgr.next_fd
        new_entry = %SocketEntry{
          fd: new_fd,
          socket_type: :stream,
          tcp_connection: conn,
          local_ip: sock.local_ip,
          local_port: sock.local_port,
          remote_ip: conn.remote_ip,
          remote_port: conn.remote_port
        }
        sock = %{sock | accept_queue: rest}
        mgr = %{mgr |
          sockets: mgr.sockets |> Map.put(fd, sock) |> Map.put(new_fd, new_entry),
          next_fd: mgr.next_fd + 1
        }
        {:ok, new_fd, mgr}
      else
        false -> {:error, :not_listening}
        [] -> {:error, :no_pending}
        {:error, reason} -> {:error, reason}
      end
    end

    @doc "Initiate a TCP connect. Returns {:ok, syn_header, mgr} or {:error, reason}."
    def connect(%__MODULE__{} = mgr, fd, remote_ip, remote_port) do
      with {:ok, sock} <- get_socket(mgr, fd) do
        sock = %{sock | remote_ip: remote_ip, remote_port: remote_port}
        if sock.tcp_connection do
          conn = %{sock.tcp_connection | remote_ip: remote_ip, remote_port: remote_port}
          {conn, syn} = TCPConnection.initiate_connect(conn)
          sock = %{sock | tcp_connection: conn}
          {:ok, syn, %{mgr | sockets: Map.put(mgr.sockets, fd, sock)}}
        else
          {:error, :no_tcp}
        end
      end
    end

    @doc "Close a socket. Returns {fin_header_or_nil, mgr}."
    def close(%__MODULE__{} = mgr, fd) do
      case get_socket(mgr, fd) do
        {:ok, sock} ->
          fin = if sock.tcp_connection do
            {_conn, header} = TCPConnection.initiate_close(sock.tcp_connection)
            header
          else
            nil
          end
          port = sock.local_port
          mgr = %{mgr |
            sockets: Map.delete(mgr.sockets, fd),
            used_ports: if(port > 0, do: MapSet.delete(mgr.used_ports, port), else: mgr.used_ports)
          }
          {fin, mgr}
        {:error, _} -> {nil, mgr}
      end
    end

    @doc "Get a socket by fd."
    def get_socket(%__MODULE__{} = mgr, fd) do
      case Map.get(mgr.sockets, fd) do
        nil -> {:error, :not_found}
        sock -> {:ok, sock}
      end
    end
  end

  # ===========================================================================
  # LAYER 7: DNS
  # ===========================================================================

  defmodule DNSResolver do
    @moduledoc """
    ## DNS Resolver — Turning Names into Numbers

    Humans remember names; computers use numbers. Our simplified resolver
    uses a static table. Default: "localhost" -> [127, 0, 0, 1].
    """

    @type t :: %{String.t() => [non_neg_integer()]}

    @doc "Create a new DNS resolver with localhost pre-configured."
    def new, do: %{"localhost" => [127, 0, 0, 1]}

    @doc "Resolve a hostname to an IP address."
    def resolve(table, hostname), do: Map.get(table, hostname)

    @doc "Add a static DNS entry."
    def add_static(table, hostname, ip), do: Map.put(table, hostname, ip)
  end

  # ===========================================================================
  # LAYER 7: HTTP
  # ===========================================================================

  defmodule HTTPRequest do
    @moduledoc """
    ## HTTP Request — Asking for a Resource

    Wire format:
        GET /index.html HTTP/1.1\\r\\n
        Host: example.com\\r\\n
        \\r\\n
    """

    defstruct [:method, :path, headers: %{}, body: ""]

    @type t :: %__MODULE__{
            method: String.t(),
            path: String.t(),
            headers: %{String.t() => String.t()},
            body: String.t()
          }

    def new(method, path, headers \\ %{}, body \\ ""),
      do: %__MODULE__{method: method, path: path, headers: headers, body: body}

    @doc "Serialize the request to wire format."
    def serialize(%__MODULE__{} = req) do
      request_line = "#{req.method} #{req.path} HTTP/1.1\r\n"
      header_lines = req.headers
        |> Enum.map(fn {k, v} -> "#{k}: #{v}\r\n" end)
        |> Enum.join()
      request_line <> header_lines <> "\r\n" <> req.body
    end

    @doc "Deserialize raw text into an HTTPRequest."
    def deserialize(text) do
      [head | rest_parts] = String.split(text, "\r\n\r\n", parts: 2)
      body = if rest_parts == [], do: "", else: hd(rest_parts)

      lines = String.split(head, "\r\n")
      [request_line | header_lines] = lines
      [method, path | _] = String.split(request_line, " ")

      headers = header_lines
        |> Enum.filter(&(&1 != ""))
        |> Enum.map(fn line ->
          [key | val_parts] = String.split(line, ": ", parts: 2)
          {key, Enum.join(val_parts, ": ")}
        end)
        |> Map.new()

      new(method, path, headers, body)
    end
  end

  defmodule HTTPResponse do
    @moduledoc """
    ## HTTP Response — The Server's Answer

    Wire format:
        HTTP/1.1 200 OK\\r\\n
        Content-Type: text/html\\r\\n
        \\r\\n
        Hello, World!
    """

    defstruct [:status_code, :status_text, headers: %{}, body: ""]

    @type t :: %__MODULE__{
            status_code: non_neg_integer(),
            status_text: String.t(),
            headers: %{String.t() => String.t()},
            body: String.t()
          }

    def new(status_code, status_text, headers \\ %{}, body \\ ""),
      do: %__MODULE__{status_code: status_code, status_text: status_text, headers: headers, body: body}

    @doc "Serialize to wire format."
    def serialize(%__MODULE__{} = resp) do
      status_line = "HTTP/1.1 #{resp.status_code} #{resp.status_text}\r\n"
      header_lines = resp.headers
        |> Enum.map(fn {k, v} -> "#{k}: #{v}\r\n" end)
        |> Enum.join()
      status_line <> header_lines <> "\r\n" <> resp.body
    end

    @doc "Deserialize raw text into an HTTPResponse."
    def deserialize(text) do
      [head | rest_parts] = String.split(text, "\r\n\r\n", parts: 2)
      body = if rest_parts == [], do: "", else: hd(rest_parts)

      lines = String.split(head, "\r\n")
      [status_line | header_lines] = lines
      parts = String.split(status_line, " ", parts: 3)
      status_code = parts |> Enum.at(1) |> String.to_integer()
      status_text = Enum.at(parts, 2, "")

      headers = header_lines
        |> Enum.filter(&(&1 != ""))
        |> Enum.map(fn line ->
          [key | val_parts] = String.split(line, ": ", parts: 2)
          {key, Enum.join(val_parts, ": ")}
        end)
        |> Map.new()

      new(status_code, status_text, headers, body)
    end
  end

  defmodule HTTPClient do
    @moduledoc """
    ## HTTP Client — Building and Parsing HTTP Over TCP

    Constructs HTTP requests from URLs and parses HTTP responses.
    """

    @doc """
    Build an HTTP GET request from a URL.

    URL format: http://hostname:port/path
    Port defaults to 80; path defaults to "/".
    """
    def build_request(url, dns \\ nil) do
      rest = String.replace_prefix(url, "http://", "")

      {host_part, path} = case String.split(rest, "/", parts: 2) do
        [hp, p] -> {hp, "/" <> p}
        [hp] -> {hp, "/"}
      end

      {host, port} = case String.split(host_part, ":", parts: 2) do
        [h, p] -> {h, String.to_integer(p)}
        [h] -> {h, 80}
      end

      request = HTTPRequest.new("GET", path, %{"Host" => host})
      _ = dns  # DNS resolver available for future use
      {request, host, port}
    end

    @doc "Parse an HTTP response from raw text."
    def parse_response(text), do: HTTPResponse.deserialize(text)
  end

  # ===========================================================================
  # NETWORK WIRE — Simulated Physical Medium
  # ===========================================================================

  defmodule NetworkWire do
    @moduledoc """
    ## Network Wire — A Virtual Ethernet Cable

    A bidirectional channel connecting two endpoints. Uses an Agent process
    to hold mutable state (two queues), which is the idiomatic Elixir way
    to model mutable shared state.

        ┌─────┐     ┌──────────────┐     ┌─────┐
        │  A  │────>│ queue_a_to_b │────>│  B  │
        │     │<────│ queue_b_to_a │<────│     │
        └─────┘     └──────────────┘     └─────┘
    """

    @doc "Start a new network wire (an Agent process)."
    def new do
      {:ok, pid} = Agent.start_link(fn -> %{a_to_b: :queue.new(), b_to_a: :queue.new()} end)
      pid
    end

    @doc "Side A sends data (will be received by side B)."
    def send_a(wire, data) do
      Agent.update(wire, fn state ->
        %{state | a_to_b: :queue.in(data, state.a_to_b)}
      end)
    end

    @doc "Side B sends data (will be received by side A)."
    def send_b(wire, data) do
      Agent.update(wire, fn state ->
        %{state | b_to_a: :queue.in(data, state.b_to_a)}
      end)
    end

    @doc "Side A receives data (sent by side B)."
    def receive_a(wire) do
      Agent.get_and_update(wire, fn state ->
        case :queue.out(state.b_to_a) do
          {{:value, data}, new_queue} -> {data, %{state | b_to_a: new_queue}}
          {:empty, _} -> {nil, state}
        end
      end)
    end

    @doc "Side B receives data (sent by side A)."
    def receive_b(wire) do
      Agent.get_and_update(wire, fn state ->
        case :queue.out(state.a_to_b) do
          {{:value, data}, new_queue} -> {data, %{state | a_to_b: new_queue}}
          {:empty, _} -> {nil, state}
        end
      end)
    end

    @doc "Check if side A has data waiting."
    def has_data_for_a?(wire) do
      Agent.get(wire, fn state -> not :queue.is_empty(state.b_to_a) end)
    end

    @doc "Check if side B has data waiting."
    def has_data_for_b?(wire) do
      Agent.get(wire, fn state -> not :queue.is_empty(state.a_to_b) end)
    end

    @doc "Stop the wire Agent."
    def stop(wire), do: Agent.stop(wire)
  end
end
