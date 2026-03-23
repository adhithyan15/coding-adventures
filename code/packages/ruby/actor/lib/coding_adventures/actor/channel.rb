# frozen_string_literal: true

require "fileutils"

module CodingAdventures
  module Actor
    # ═══════════════════════════════════════════════════════════════
    # Channel — a one-way, append-only, ordered message log
    # ═══════════════════════════════════════════════════════════════
    #
    # A Channel is like a one-way pneumatic tube in an office building.
    # Documents (messages) go in one end and come out the other. You
    # cannot send documents backwards. The tube keeps a copy of every
    # document that has ever passed through it (the log), and each
    # office at the receiving end has a bookmark showing which
    # documents they have already read (the offset).
    #
    # === Why one-way?
    #
    # Bidirectional channels create ambiguity: "who sent this message?"
    # One-way channels eliminate that question entirely. If you need
    # bidirectional communication, use two channels — one in each
    # direction. This is not a limitation; it's a security property.
    #
    # === Why append-only?
    #
    # If messages could be deleted or modified, crash recovery would be
    # impossible. After a crash, the system asks: "what happened before
    # the crash?" If the log is mutable, the answer is "we don't know."
    # If the log is append-only, the answer is definitive: "here is
    # exactly what happened, in order, immutably recorded."
    #
    # === Persistence
    #
    # Channels persist to disk as a binary append log using the Message
    # wire format. Each message is written as its header + envelope +
    # payload, concatenated end-to-end. This format is:
    #   - Binary-native: images stored as raw bytes, not Base64
    #   - Appendable: just write bytes at the end
    #   - Replayable: parse header -> envelope -> payload -> repeat
    #   - Scannable: read headers + envelopes, skip payloads
    #
    # === Offset Tracking
    #
    # Each consumer independently tracks how far it has read. The
    # channel itself does NOT track consumer positions — it is a
    # dumb log. Consumers are smart readers.
    #
    #   Channel log:   [m0] [m1] [m2] [m3] [m4]
    #                                  ^
    #   Consumer A:    offset = 3 ─────┘
    #                  ^
    #   Consumer B:    offset = 0 (hasn't started reading yet)
    #
    class Channel
      attr_reader :id, :name, :created_at

      # Create a new Channel.
      #
      # @param channel_id [String] Unique identifier for this channel.
      # @param name [String] Human-readable name (e.g., "email-summaries").
      def initialize(channel_id:, name:)
        @id = channel_id
        @name = name
        @created_at = (Process.clock_gettime(Process::CLOCK_MONOTONIC) * 1_000_000_000).to_i

        # The log is a simple Ruby Array. Messages are appended to the
        # end, and random access by index gives O(1) reads. In production,
        # this would be backed by a memory-mapped file or a database, but
        # for our educational implementation, an array is perfect.
        @log = []
      end

      # Append a message to the end of the log.
      #
      # This is the ONLY write operation on a Channel. There is no
      # delete, no update, no insert-at-position. The sequence number
      # is the message's position in the log (0-indexed), assigned
      # monotonically: first message gets 0, second gets 1, etc.
      #
      # @param message [Message] The message to append.
      # @return [Integer] The sequence number assigned to this message.
      def append(message)
        seq = @log.length
        @log.push(message)
        seq
      end

      # Read messages from the log starting at an offset.
      #
      # This does NOT consume the messages — they remain in the log
      # forever. Multiple consumers can read the same messages
      # independently at different offsets.
      #
      # @param offset [Integer] The position to start reading from (0-indexed).
      # @param limit [Integer] Maximum number of messages to return (default 100).
      # @return [Array<Message>] A copy of the requested messages. Empty if
      #   the offset is past the end of the log.
      def read(offset: 0, limit: 100)
        # If the caller's offset is at or past the end, they're caught up
        return [] if offset >= @log.length

        # Calculate the actual end position, capping at log length
        end_pos = [offset + limit, @log.length].min

        # Return a copy (slice) of the log. The caller cannot modify
        # the channel's internal state through this returned array.
        @log[offset...end_pos]
      end

      # Return the number of messages in the log.
      #
      # @return [Integer] The count of messages.
      def length
        @log.length
      end

      # Return a slice of messages between two indices (exclusive end).
      #
      # This is equivalent to read(start, end - start) but uses the
      # more familiar start/end slice notation.
      #
      # @param start_idx [Integer] Starting index (inclusive).
      # @param end_idx [Integer] Ending index (exclusive).
      # @return [Array<Message>] Messages in the range [start_idx, end_idx).
      def slice(start_idx, end_idx)
        return [] if start_idx >= @log.length

        actual_end = [end_idx, @log.length].min
        @log[start_idx...actual_end]
      end

      # Persist the channel log to disk as a binary file.
      #
      # Each message is written in the wire format (17-byte header +
      # JSON envelope + raw payload), concatenated end-to-end. This
      # creates a file that can be replayed from the beginning to
      # reconstruct the entire channel.
      #
      # The file is named "{name}.log" inside the given directory.
      #
      # @param directory [String] The directory to write the log file to.
      def persist(directory)
        FileUtils.mkdir_p(directory)
        path = File.join(directory, "#{@name}.log")

        File.open(path, "wb") do |f|
          @log.each do |message|
            f.write(message.to_bytes)
          end
          f.flush
          f.fsync
        end
      end

      # Recover a channel from a persisted binary log file.
      #
      # This reads the file from the beginning, parsing messages one
      # by one using the wire format. If the file ends with a truncated
      # message (e.g., from a crash mid-write), the incomplete message
      # is silently discarded — all complete messages before it are
      # recovered.
      #
      # @param directory [String] The directory containing the log file.
      # @param name [String] The channel name (used to find "{name}.log").
      # @return [Channel] A new Channel with all recovered messages.
      def self.recover(directory, name)
        channel = new(channel_id: SecureRandom.uuid, name: name)
        path = File.join(directory, "#{name}.log")

        # If the file doesn't exist, return an empty channel
        return channel unless File.exist?(path)

        File.open(path, "rb") do |f|
          # Read messages one at a time until EOF or a truncated message
          loop do
            begin
              msg = Message.from_io(f)
              break if msg.nil?  # Clean EOF
              channel.append(msg)
            rescue InvalidFormatError, VersionError, JSON::ParserError
              # Truncated or corrupted message — stop here.
              # All complete messages before this point are recovered.
              break
            end
          end
        end

        channel
      end
    end
  end
end
