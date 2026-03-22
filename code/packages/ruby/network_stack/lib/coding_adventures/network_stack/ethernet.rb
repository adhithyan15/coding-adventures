# frozen_string_literal: true

# ============================================================================
# Layer 2: Ethernet — Local Network Delivery
# ============================================================================
#
# Ethernet is the foundation of local area networking. Every device on a local
# network has a unique 48-bit MAC (Media Access Control) address burned into
# its network interface card (NIC) at the factory.
#
# When you send data on a local network, the Ethernet layer wraps it in a
# "frame" with the destination MAC address, source MAC address, and a type
# field that tells the receiver how to interpret the payload.
#
# Think of Ethernet like the local postal carrier: it only delivers between
# houses on the same street. To reach another street (another network), you
# need a router (the postal routing system — that's IP's job).
#
# Frame structure on the wire:
#
#   +-----------+-----------+------------+---------+
#   | dest_mac  | src_mac   | ether_type | payload |
#   | (6 bytes) | (6 bytes) | (2 bytes)  | (var)   |
#   +-----------+-----------+------------+---------+
#
# Common ether_type values:
#   0x0800 = IPv4 — the payload is an IP packet
#   0x0806 = ARP  — the payload is an ARP message
#
# ============================================================================

module CodingAdventures
  module NetworkStack
    # EtherType constants — these tell the receiver how to interpret the payload.
    ETHER_TYPE_IPV4 = 0x0800
    ETHER_TYPE_ARP  = 0x0806

    # Broadcast MAC address — used when we need to send a frame to ALL devices
    # on the local network (e.g., ARP requests). Every NIC listens for frames
    # addressed to FF:FF:FF:FF:FF:FF.
    BROADCAST_MAC = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF].freeze

    # ========================================================================
    # EthernetFrame
    # ========================================================================
    #
    # Represents a single Ethernet frame — the fundamental unit of data on a
    # local network. Each frame carries:
    #   - dest_mac:   who should receive this frame (6 bytes)
    #   - src_mac:    who sent this frame (6 bytes)
    #   - ether_type: what the payload contains (2 bytes)
    #   - payload:    the actual data (variable length)
    #
    # ========================================================================
    class EthernetFrame
      attr_accessor :dest_mac, :src_mac, :ether_type, :payload

      def initialize(dest_mac:, src_mac:, ether_type:, payload: [])
        @dest_mac   = dest_mac.dup
        @src_mac    = src_mac.dup
        @ether_type = ether_type
        @payload    = payload.dup
      end

      # Serialize the frame into a flat array of bytes for transmission.
      #
      # The format mirrors real Ethernet frames (minus the preamble, CRC,
      # and inter-frame gap that physical hardware handles):
      #
      #   bytes[0..5]   = destination MAC (6 bytes)
      #   bytes[6..11]  = source MAC (6 bytes)
      #   bytes[12..13] = ether_type (2 bytes, big-endian)
      #   bytes[14..]   = payload (variable length)
      #
      def serialize
        bytes = []
        bytes.concat(@dest_mac)
        bytes.concat(@src_mac)
        bytes.push((@ether_type >> 8) & 0xFF)  # ether_type high byte
        bytes.push(@ether_type & 0xFF)          # ether_type low byte
        bytes.concat(@payload)
        bytes
      end

      # Deserialize a byte array back into an EthernetFrame.
      #
      # This is the reverse of serialize — we peel off the known-length header
      # fields and treat everything remaining as payload.
      #
      # Returns nil if the byte array is too short to contain a valid header
      # (minimum 14 bytes: 6 + 6 + 2).
      #
      def self.deserialize(bytes)
        return nil if bytes.length < 14

        dest_mac   = bytes[0..5]
        src_mac    = bytes[6..11]
        ether_type = (bytes[12] << 8) | bytes[13]
        payload    = bytes[14..] || []

        new(dest_mac: dest_mac, src_mac: src_mac, ether_type: ether_type, payload: payload)
      end
    end

    # ========================================================================
    # ARPTable — IP-to-MAC Address Resolution Cache
    # ========================================================================
    #
    # ARP (Address Resolution Protocol) bridges the gap between Layer 3 (IP)
    # and Layer 2 (Ethernet). When you want to send an IP packet to 10.0.0.2,
    # you need to know the MAC address of 10.0.0.2's NIC so you can address
    # the Ethernet frame correctly.
    #
    # The ARP table is a cache of recently learned IP-to-MAC mappings. In a
    # real system, entries expire after a timeout (typically 20 minutes) to
    # handle devices that change their MAC address or move to a different port.
    #
    # ARP exchange:
    #
    #   1. Host A wants to send to 10.0.0.2 but doesn't know its MAC.
    #   2. A broadcasts: "Who has 10.0.0.2? Tell 10.0.0.1"
    #      (dest_mac = FF:FF:FF:FF:FF:FF, ether_type = 0x0806)
    #   3. Host B (10.0.0.2) replies: "10.0.0.2 is at BB:BB:BB:BB:BB:BB"
    #   4. A stores the mapping in its ARP table.
    #   5. Future packets to 10.0.0.2 use the cached MAC directly.
    #
    # ========================================================================
    class ARPTable
      def initialize
        @entries = {}
      end

      # Look up the MAC address for a given IP address.
      # Returns the MAC (as a 6-byte array) or nil if not found.
      def lookup(ip)
        @entries[ip]
      end

      # Insert or update an IP-to-MAC mapping.
      # Called when we receive an ARP reply or observe traffic from a known IP.
      def insert(ip, mac)
        @entries[ip] = mac.dup
      end

      # How many entries are currently in the table?
      def size
        @entries.size
      end
    end
  end
end
