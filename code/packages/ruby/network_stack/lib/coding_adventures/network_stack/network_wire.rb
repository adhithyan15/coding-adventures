# frozen_string_literal: true

# ============================================================================
# NetworkWire — Simulated Physical Network Medium
# ============================================================================
#
# In a real network, an Ethernet cable carries electrical signals between
# two devices. We simulate this with an in-memory bidirectional channel.
#
# The NetworkWire connects two endpoints (A and B). When A sends a frame,
# it appears in B's receive queue, and vice versa. Think of it as a
# virtual Ethernet cable:
#
#   +---------+                              +---------+
#   | Host A  | -------- NetworkWire ------> | Host B  |
#   |         | <------- NetworkWire ------- |         |
#   +---------+                              +---------+
#
# Internally, the wire maintains two FIFO queues:
#   - a_to_b: frames sent by A, waiting for B to receive
#   - b_to_a: frames sent by B, waiting for A to receive
#
# This is the simplest possible network simulation — no latency, no packet
# loss, no bandwidth limits, no collision detection. A real Ethernet cable
# has all of these properties, but they're not needed to understand how the
# protocol layers work.
#
# ============================================================================

module CodingAdventures
  module NetworkStack
    class NetworkWire
      def initialize
        @a_to_b = []
        @b_to_a = []
      end

      # Host A sends a frame — it will be received by Host B.
      def send_a(frame)
        @a_to_b.push(frame)
      end

      # Host B sends a frame — it will be received by Host A.
      def send_b(frame)
        @b_to_a.push(frame)
      end

      # Host A receives a frame sent by Host B.
      # Returns the frame or nil if no data is waiting.
      def receive_a
        @b_to_a.shift
      end

      # Host B receives a frame sent by Host A.
      # Returns the frame or nil if no data is waiting.
      def receive_b
        @a_to_b.shift
      end

      # Is there data waiting for Host A?
      def has_data_for_a?
        !@b_to_a.empty?
      end

      # Is there data waiting for Host B?
      def has_data_for_b?
        !@a_to_b.empty?
      end

      # How many frames are queued in each direction?
      def pending_a_to_b
        @a_to_b.length
      end

      def pending_b_to_a
        @b_to_a.length
      end
    end
  end
end
