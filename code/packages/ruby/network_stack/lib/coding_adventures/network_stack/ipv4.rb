# frozen_string_literal: true

# ============================================================================
# Layer 3: IPv4 — Internet Protocol Version 4
# ============================================================================
#
# IP is the routing layer — it figures out how to get a packet from one
# network to another. While Ethernet handles delivery on a single local
# network (like delivering mail on one street), IP handles delivery across
# the entire Internet (like the postal routing system that moves mail
# between cities).
#
# Every device on the Internet has an IP address (e.g., 10.0.0.1). IP
# addresses are 32 bits (4 bytes) for IPv4. The IP layer adds a header
# with source and destination addresses, then hands the packet to the
# Ethernet layer for local delivery to the next hop (router or final
# destination).
#
# IPv4 Header format (20 bytes, no options):
#
#    0                   1                   2                   3
#    0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
#   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#   |Version|  IHL  |    TOS        |          Total Length         |
#   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#   |         Identification        |Flags|      Fragment Offset   |
#   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#   |  Time to Live |    Protocol   |         Header Checksum      |
#   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#   |                       Source Address                         |
#   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#   |                    Destination Address                       |
#   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#
# Protocol numbers:
#   6  = TCP
#   17 = UDP
#
# ============================================================================

module CodingAdventures
  module NetworkStack
    # Protocol numbers — these go in the IPv4 header's "protocol" field to
    # tell the receiver which Layer 4 protocol the payload contains.
    PROTOCOL_TCP = 6
    PROTOCOL_UDP = 17

    # ========================================================================
    # IPv4Header
    # ========================================================================
    #
    # The header that prefixes every IP packet. We implement the minimal
    # 20-byte header (IHL=5, no options). The checksum field uses the ones'
    # complement algorithm defined in RFC 791.
    #
    # ========================================================================
    class IPv4Header
      attr_accessor :version, :ihl, :total_length, :ttl, :protocol,
        :header_checksum, :src_ip, :dst_ip

      def initialize(
        src_ip:,
        dst_ip:,
        protocol:,
        total_length: 20,
        ttl: 64,
        version: 4,
        ihl: 5,
        header_checksum: 0
      )
        @version         = version
        @ihl             = ihl
        @total_length    = total_length
        @ttl             = ttl
        @protocol        = protocol
        @header_checksum = header_checksum
        @src_ip          = src_ip.dup
        @dst_ip          = dst_ip.dup
      end

      # Serialize the header into a 20-byte array.
      #
      # We pack the fields in network byte order (big-endian), which is the
      # standard for all Internet protocols. "Network byte order" exists because
      # different CPUs store multi-byte integers differently (big-endian vs
      # little-endian), and the Internet needed a single standard.
      #
      def serialize
        bytes = Array.new(20, 0)

        # Byte 0: version (4 bits) + IHL (4 bits)
        bytes[0] = ((@version & 0x0F) << 4) | (@ihl & 0x0F)

        # Byte 1: Type of Service (we set to 0 — no special handling)
        bytes[1] = 0

        # Bytes 2-3: Total Length (big-endian)
        bytes[2] = (@total_length >> 8) & 0xFF
        bytes[3] = @total_length & 0xFF

        # Bytes 4-5: Identification (we set to 0 — no fragmentation)
        bytes[4] = 0
        bytes[5] = 0

        # Bytes 6-7: Flags + Fragment Offset (0 — no fragmentation)
        bytes[6] = 0
        bytes[7] = 0

        # Byte 8: TTL
        bytes[8] = @ttl & 0xFF

        # Byte 9: Protocol
        bytes[9] = @protocol & 0xFF

        # Bytes 10-11: Header Checksum (big-endian)
        bytes[10] = (@header_checksum >> 8) & 0xFF
        bytes[11] = @header_checksum & 0xFF

        # Bytes 12-15: Source IP
        bytes[12..15] = @src_ip

        # Bytes 16-19: Destination IP
        bytes[16..19] = @dst_ip

        bytes
      end

      # Deserialize a 20-byte array into an IPv4Header.
      def self.deserialize(bytes)
        return nil if bytes.length < 20

        version = (bytes[0] >> 4) & 0x0F
        ihl     = bytes[0] & 0x0F

        total_length = (bytes[2] << 8) | bytes[3]
        ttl          = bytes[8]
        protocol     = bytes[9]
        header_checksum = (bytes[10] << 8) | bytes[11]
        src_ip       = bytes[12..15]
        dst_ip       = bytes[16..19]

        new(
          version: version,
          ihl: ihl,
          total_length: total_length,
          ttl: ttl,
          protocol: protocol,
          header_checksum: header_checksum,
          src_ip: src_ip,
          dst_ip: dst_ip
        )
      end

      # Compute the IP header checksum using the ones' complement algorithm.
      #
      # The algorithm:
      #   1. Set the checksum field to 0.
      #   2. Treat the 20-byte header as ten 16-bit words.
      #   3. Sum all words. If the sum exceeds 16 bits, add the carry back.
      #   4. Take the ones' complement (bitwise NOT, masked to 16 bits).
      #
      # This is the same algorithm used since the 1980s — simple enough to
      # implement in hardware, yet effective at catching single-bit errors.
      #
      def compute_checksum
        # Step 1: serialize with checksum = 0
        saved = @header_checksum
        @header_checksum = 0
        bytes = serialize
        @header_checksum = saved

        # Step 2: sum 16-bit words
        sum = 0
        (0...bytes.length).step(2) do |i|
          word = (bytes[i] << 8) | (bytes[i + 1] || 0)
          sum += word
        end

        # Step 3: fold carry bits back into the 16-bit sum
        while sum > 0xFFFF
          sum = (sum & 0xFFFF) + (sum >> 16)
        end

        # Step 4: ones' complement
        (~sum) & 0xFFFF
      end

      # Verify the checksum of a received header.
      #
      # If we sum all 16-bit words (including the checksum field) and fold
      # carries, a valid header produces 0xFFFF (all ones). This is because
      # the checksum was chosen to make the sum come out to all ones.
      #
      def verify_checksum
        bytes = serialize
        sum = 0
        (0...bytes.length).step(2) do |i|
          word = (bytes[i] << 8) | (bytes[i + 1] || 0)
          sum += word
        end
        while sum > 0xFFFF
          sum = (sum & 0xFFFF) + (sum >> 16)
        end
        sum == 0xFFFF
      end
    end

    # ========================================================================
    # RoutingTable — Longest-Prefix Match Router
    # ========================================================================
    #
    # A routing table contains rules that tell the IP layer where to send
    # packets. Each rule says: "If the destination IP matches this network
    # (after applying this mask), send it to this gateway via this interface."
    #
    # When multiple rules match, we pick the one with the longest (most
    # specific) mask — this is called "longest prefix match." For example:
    #
    #   Route 1: 10.0.0.0/8   -> gateway A   (matches any 10.x.x.x)
    #   Route 2: 10.0.1.0/24  -> gateway B   (matches only 10.0.1.x)
    #
    #   Destination 10.0.1.5 matches BOTH routes, but Route 2 has a longer
    #   prefix (/24 vs /8), so we use gateway B. This makes sense: a more
    #   specific route should take priority over a general one.
    #
    # ========================================================================
    class RoutingTable
      RouteEntry = Struct.new(:network, :mask, :gateway, :interface_name)

      def initialize
        @routes = []
      end

      # Add a route to the table.
      #
      # Parameters:
      #   network        — network address (e.g., [10, 0, 0, 0])
      #   mask           — subnet mask (e.g., [255, 255, 255, 0])
      #   gateway        — next-hop IP (e.g., [10, 0, 0, 1]) or [0,0,0,0] for direct
      #   interface_name — name of the network interface (e.g., "eth0")
      #
      def add_route(network, mask, gateway, interface_name)
        @routes.push(RouteEntry.new(network, mask, gateway, interface_name))
      end

      # Look up the best route for a destination IP using longest-prefix match.
      #
      # Returns a RouteEntry or nil if no route matches.
      #
      def lookup(dst_ip)
        best_route = nil
        best_mask_length = -1

        @routes.each do |route|
          # Apply the mask to both the destination IP and the route's network.
          # If they match, this route is a candidate.
          match = true
          4.times do |i|
            if (dst_ip[i] & route.mask[i]) != route.network[i]
              match = false
              break
            end
          end

          next unless match

          # Count the number of 1-bits in the mask — more bits = more specific.
          mask_length = route.mask.sum { |b| b.to_s(2).count("1") }

          if mask_length > best_mask_length
            best_mask_length = mask_length
            best_route = route
          end
        end

        best_route
      end

      def size
        @routes.size
      end
    end

    # ========================================================================
    # IPLayer — Send and Receive IP Packets
    # ========================================================================
    #
    # The IPLayer ties together the IPv4Header, RoutingTable, and ARPTable.
    # It creates outbound IP packets (with proper headers and checksums) and
    # parses inbound packets (verifying checksums and checking destination).
    #
    # ========================================================================
    class IPLayer
      attr_reader :local_ip, :routing_table, :arp_table

      def initialize(local_ip:, routing_table: nil, arp_table: nil)
        @local_ip      = local_ip.dup
        @routing_table = routing_table || RoutingTable.new
        @arp_table     = arp_table || ARPTable.new
      end

      # Create an IP packet (header + payload) ready for transmission.
      #
      # Returns the serialized bytes of the full IP packet, or nil if
      # routing fails.
      #
      def create_packet(dst_ip, protocol, payload)
        header = IPv4Header.new(
          src_ip: @local_ip,
          dst_ip: dst_ip,
          protocol: protocol,
          total_length: 20 + payload.length,
          ttl: 64
        )
        header.header_checksum = header.compute_checksum
        header.serialize + payload
      end

      # Parse a received IP packet.
      #
      # Returns [src_ip, protocol, payload] or nil if:
      #   - the packet is too short
      #   - the checksum is invalid
      #   - the destination is not us
      #
      def parse_packet(bytes)
        return nil if bytes.length < 20

        header = IPv4Header.deserialize(bytes)
        return nil unless header
        return nil unless header.verify_checksum

        payload = bytes[20..] || []
        [header.src_ip, header.protocol, payload]
      end
    end
  end
end
