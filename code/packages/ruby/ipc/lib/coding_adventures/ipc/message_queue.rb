# frozen_string_literal: true

# Message Queue -- structured, typed message passing between processes.
#
# While pipes transmit raw bytes (the reader must know how to parse them),
# message queues transmit discrete, typed **messages**. Each message carries:
#
#   +----------+-----------------------------+
#   | msg_type | body (up to 4096 bytes)     |
#   +----------+-----------------------------+
#
# The type tag allows selective receiving: "give me only messages of type 3"
# while leaving other message types in the queue for someone else to pick up.
#
# === Analogy ===
#
# Think of a shared mailbox in an apartment building's lobby. Anyone can drop
# off an envelope (send), and anyone can pick one up (receive). Each envelope
# has a label ("type") -- you might only care about envelopes labeled "rent"
# and ignore the ones labeled "newsletter."
#
# === FIFO Ordering ===
#
# Messages within the queue follow FIFO (First-In, First-Out) order. When
# receiving with a type filter, the queue returns the OLDEST message matching
# that type, preserving arrival order among messages of the same type:
#
#   Queue contents (front → back):
#     [type=1, "apple"] → [type=2, "banana"] → [type=1, "cherry"]
#
#   receive(type=0)  → [type=1, "apple"]   (any type, oldest overall)
#   receive(type=2)  → [type=2, "banana"]  (oldest type-2 message)
#   receive(type=1)  → [type=1, "apple"]   (oldest type-1, skips type-2)
#
# === Capacity Limits ===
#
#   +-------------------+-------+------------------------------------------+
#   | Limit             | Value | What happens when exceeded               |
#   +-------------------+-------+------------------------------------------+
#   | max_messages      |  256  | send() returns false (queue full)        |
#   | max_message_size  | 4096  | send() returns false (message too large) |
#   +-------------------+-------+------------------------------------------+
#
# In a real OS, exceeding these limits would block the sending process until
# space is available. In our simulation, we return false to indicate failure,
# letting the caller decide how to handle it (retry, drop, etc.).

module CodingAdventures
  module Ipc
    # Default limits matching System V IPC conventions.
    DEFAULT_MAX_MESSAGES = 256
    DEFAULT_MAX_MESSAGE_SIZE = 4096

    # A single message in the queue: a (type, body) pair.
    #
    # The type is a positive integer that senders and receivers agree on.
    # For example, a client-server protocol might use:
    #   type=1 → request
    #   type=2 → response
    #   type=3 → heartbeat
    Message = Struct.new(:msg_type, :body)

    class MessageQueue
      attr_reader :max_messages, :max_message_size, :messages

      def initialize(max_messages: DEFAULT_MAX_MESSAGES, max_message_size: DEFAULT_MAX_MESSAGE_SIZE)
        @max_messages = max_messages
        @max_message_size = max_message_size

        # The internal storage is a simple Ruby Array used as a FIFO.
        # We push to the back (<<) and shift from the front (delete_at).
        @messages = []
      end

      # Send a typed message to the queue.
      #
      # Parameters:
      #   msg_type - positive integer identifying the message kind
      #   body     - array of byte values (0-255), up to max_message_size bytes
      #
      # Returns true if the message was enqueued, false if:
      #   - The queue is full (messages.length >= max_messages)
      #   - The message body exceeds max_message_size
      #   - The message type is not a positive integer
      def send(msg_type, body)
        # Validate: type must be a positive integer.
        return false unless msg_type.is_a?(Integer) && msg_type > 0

        # Validate: body must not exceed the size limit.
        return false if body.length > @max_message_size

        # Validate: queue must not be full.
        return false if @messages.length >= @max_messages

        @messages << Message.new(msg_type, body.dup)
        true
      end

      # Receive a message from the queue.
      #
      # Parameters:
      #   msg_type - 0 means "any type" (dequeue the oldest message regardless
      #              of type). A positive integer means "give me the oldest
      #              message of this specific type."
      #
      # Returns a Message (with msg_type and body), or nil if no matching
      # message is found.
      #
      # When filtering by type, non-matching messages are left in the queue
      # in their original positions. Only the first matching message is removed.
      def receive(msg_type = 0)
        if msg_type == 0
          # Any type: dequeue the oldest message (front of the array).
          @messages.shift
        else
          # Specific type: find the first message with a matching type.
          # We scan from the front (oldest) to preserve FIFO order within
          # a given type.
          index = @messages.index { |m| m.msg_type == msg_type }
          return nil if index.nil?

          @messages.delete_at(index)
        end
      end

      # How many messages are currently in the queue?
      def count
        @messages.length
      end

      # Is the queue full?
      def full?
        @messages.length >= @max_messages
      end

      # Is the queue empty?
      def empty?
        @messages.empty?
      end
    end
  end
end
