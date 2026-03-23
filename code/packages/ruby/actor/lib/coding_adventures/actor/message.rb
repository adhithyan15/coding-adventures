# frozen_string_literal: true

require "securerandom"
require "json"

module CodingAdventures
  module Actor
    # ═══════════════════════════════════════════════════════════════
    # Message — the atom of actor communication
    # ═══════════════════════════════════════════════════════════════
    #
    # A Message is an immutable, self-describing unit of data that flows
    # between actors. Think of it as a sealed letter: once created, the
    # contents are fixed. The envelope records who sent it, when, and
    # what kind of data it contains.
    #
    # Every message has:
    #   - id:           A unique identifier (UUID v4), generated automatically
    #   - timestamp:    Monotonic nanosecond counter for ordering (not wall clock)
    #   - sender_id:    The actor that created this message
    #   - content_type: MIME type describing the payload format
    #   - payload:      Raw bytes (String with ASCII-8BIT / binary encoding)
    #   - metadata:     Optional key-value pairs for extensibility
    #
    # Messages are **frozen** after initialization — no field can be changed.
    # To "modify" a message, create a new one. The original is untouched.
    #
    # === Wire Format
    #
    # Messages serialize to a binary wire format that separates the
    # **envelope** (metadata, always JSON) from the **payload** (always
    # raw bytes). This avoids Base64-encoding binary data like images,
    # which would bloat size by 33%.
    #
    #   ┌──────────────────────────────────────────┐
    #   │ HEADER (17 bytes, fixed)                 │
    #   │  magic:           4 bytes  "ACTM"        │
    #   │  version:         1 byte   0x01          │
    #   │  envelope_length: 4 bytes  (big-endian)  │
    #   │  payload_length:  8 bytes  (big-endian)  │
    #   ├──────────────────────────────────────────┤
    #   │ ENVELOPE (UTF-8 JSON, variable length)   │
    #   ├──────────────────────────────────────────┤
    #   │ PAYLOAD  (raw bytes, variable length)    │
    #   └──────────────────────────────────────────┘
    #
    # === Why immutable?
    #
    # In the Actor model, messages travel between isolated actors. If a
    # message could be mutated after sending, the sender and receiver
    # would share mutable state — exactly what the Actor model forbids.
    # Freezing the message enforces this at the language level.
    #

    # Raised when deserializing a message with a wire format version
    # higher than what this code supports. This means the reader needs
    # to be upgraded to understand the newer format.
    class VersionError < StandardError; end

    # Raised when deserializing data that does not start with the
    # expected "ACTM" magic bytes, indicating the data is not a valid
    # actor message.
    class InvalidFormatError < StandardError; end

    class Message
      # The current wire format version. Bump this when the binary
      # layout changes. Readers must handle all versions <= their own.
      WIRE_VERSION = 1

      # The 4-byte magic number that identifies actor message data.
      # "ACTM" = Actor Message. Every serialized message starts with this.
      MAGIC = "ACTM"

      # Header size in bytes: 4 (magic) + 1 (version) + 4 (envelope len) + 8 (payload len)
      HEADER_SIZE = 17

      attr_reader :id, :timestamp, :sender_id, :content_type, :payload, :metadata

      # Create a new Message.
      #
      # @param sender_id [String] The actor that created this message.
      # @param content_type [String] MIME type describing the payload format.
      # @param payload [String] Raw bytes (will be forced to binary encoding).
      # @param metadata [Hash<String, String>] Optional key-value pairs.
      # @param id [String, nil] If nil, a UUID v4 is generated.
      # @param timestamp [Integer, nil] If nil, monotonic nanoseconds are used.
      #
      # After initialization, the message is frozen — no modifications allowed.
      def initialize(sender_id:, content_type:, payload:, metadata: nil, id: nil, timestamp: nil)
        # Generate a unique ID if none provided. UUIDs are universally unique
        # identifiers — the odds of collision are astronomically small
        # (1 in 2^122 for v4).
        @id = id || SecureRandom.uuid

        # Use monotonic clock for timestamps. Unlike wall-clock time (Time.now),
        # monotonic clocks never go backwards — they always move forward, even
        # if someone adjusts the system clock. This guarantees that messages
        # created later always have higher timestamps.
        #
        # Process.clock_gettime with CLOCK_MONOTONIC returns seconds as a float.
        # We multiply by 1e9 and truncate to get nanoseconds as an integer,
        # giving us nanosecond-level ordering precision.
        @timestamp = timestamp || (Process.clock_gettime(Process::CLOCK_MONOTONIC) * 1_000_000_000).to_i

        @sender_id = sender_id

        @content_type = content_type

        # Payload is always stored as binary (ASCII-8BIT encoding).
        # This is critical: Ruby strings can have different encodings (UTF-8,
        # ASCII, etc.), but binary data must be treated as raw bytes. Forcing
        # binary encoding prevents Ruby from mangling the bytes during string
        # operations.
        @payload = payload.dup.force_encoding(Encoding::ASCII_8BIT)

        # Metadata defaults to an empty hash if not provided.
        # We freeze it to prevent mutation after message creation.
        @metadata = (metadata || {}).freeze

        # Freeze the entire message. After this line, any attempt to modify
        # any instance variable will raise a FrozenError. This enforces the
        # Actor model's requirement that messages are immutable.
        freeze
      end

      # ─── Convenience Constructors ──────────────────────────────
      #
      # These factory methods handle the common cases (text and JSON)
      # so callers don't need to manually encode payloads.

      # Create a text message. The payload string is encoded as UTF-8 bytes.
      #
      #   msg = Message.text(sender_id: "agent", payload: "hello world")
      #   msg.content_type  # => "text/plain"
      #   msg.payload_text  # => "hello world"
      #
      # @param sender_id [String] The sending actor's ID.
      # @param payload [String] The text content.
      # @param metadata [Hash, nil] Optional metadata.
      # @return [Message] A new immutable text message.
      def self.text(sender_id:, payload:, metadata: nil)
        new(
          sender_id: sender_id,
          content_type: "text/plain",
          payload: payload.encode(Encoding::UTF_8),
          metadata: metadata
        )
      end

      # Create a JSON message. The payload (Hash or Array) is serialized
      # to a JSON string, then encoded as UTF-8 bytes.
      #
      #   msg = Message.json(sender_id: "agent", payload: {"key" => "value"})
      #   msg.content_type  # => "application/json"
      #   msg.payload_json  # => {"key" => "value"}
      #
      # @param sender_id [String] The sending actor's ID.
      # @param payload [Hash, Array] The data to serialize as JSON.
      # @param metadata [Hash, nil] Optional metadata.
      # @return [Message] A new immutable JSON message.
      def self.json(sender_id:, payload:, metadata: nil)
        new(
          sender_id: sender_id,
          content_type: "application/json",
          payload: JSON.generate(payload),
          metadata: metadata
        )
      end

      # Create a binary message with an explicit content type.
      # Use this for images, videos, or any non-text data.
      #
      #   png_bytes = File.binread("photo.png")
      #   msg = Message.binary(sender_id: "camera", content_type: "image/png", payload: png_bytes)
      #
      # @param sender_id [String] The sending actor's ID.
      # @param content_type [String] MIME type (e.g., "image/png").
      # @param payload [String] Raw binary data.
      # @param metadata [Hash, nil] Optional metadata.
      # @return [Message] A new immutable binary message.
      def self.binary(sender_id:, content_type:, payload:, metadata: nil)
        new(
          sender_id: sender_id,
          content_type: content_type,
          payload: payload,
          metadata: metadata
        )
      end

      # ─── Payload Accessors ─────────────────────────────────────

      # Decode the payload as a UTF-8 string. Use this when you know
      # the message contains text (content_type = "text/plain").
      #
      # @return [String] The payload decoded as UTF-8 text.
      def payload_text
        @payload.dup.force_encoding(Encoding::UTF_8)
      end

      # Parse the payload as JSON. Use this when the message contains
      # JSON data (content_type = "application/json").
      #
      # @return [Hash, Array] The parsed JSON data.
      def payload_json
        JSON.parse(payload_text)
      end

      # ─── Serialization ─────────────────────────────────────────

      # Serialize only the envelope (everything except the raw payload)
      # to a JSON string. Useful for logging, indexing, or debugging
      # without touching potentially large binary payloads.
      #
      # @return [String] JSON string containing id, timestamp, sender_id,
      #   content_type, and metadata.
      def envelope_to_json
        JSON.generate({
          "id" => @id,
          "timestamp" => @timestamp,
          "sender_id" => @sender_id,
          "content_type" => @content_type,
          "metadata" => @metadata
        })
      end

      # Serialize the entire message to the binary wire format.
      #
      # The format is:
      #   [4 bytes magic "ACTM"]
      #   [1 byte version]
      #   [4 bytes envelope length, big-endian unsigned 32-bit]
      #   [8 bytes payload length, big-endian unsigned 64-bit]
      #   [envelope JSON bytes]
      #   [raw payload bytes]
      #
      # Ruby's Array#pack is used for binary encoding:
      #   "N" = 32-bit unsigned big-endian (network byte order)
      #   "Q>" = 64-bit unsigned big-endian
      #   "C" = 8-bit unsigned (single byte)
      #
      # @return [String] Binary string containing the full wire format.
      def to_bytes
        envelope = envelope_to_json.encode(Encoding::UTF_8)
        envelope_bytes = envelope.dup.force_encoding(Encoding::ASCII_8BIT)

        # Build the 17-byte header:
        #   - "ACTM" magic (4 bytes) identifies this as an actor message
        #   - Version byte (1 byte) tells the reader which parser to use
        #   - Envelope length (4 bytes, big-endian u32) so the reader knows
        #     how many bytes of JSON to read
        #   - Payload length (8 bytes, big-endian u64) so the reader knows
        #     how many bytes of raw data follow the envelope
        header = [
          MAGIC,                              # 4 bytes
          WIRE_VERSION,                       # 1 byte
          envelope_bytes.bytesize,            # 4 bytes (u32 big-endian)
          @payload.bytesize                   # 8 bytes (u64 big-endian)
        ].pack("a4CNQ>")

        # Concatenate header + envelope + payload into one binary blob
        result = header + envelope_bytes + @payload
        result.force_encoding(Encoding::ASCII_8BIT)
      end

      # Deserialize a message from binary wire format data.
      #
      # This is the inverse of #to_bytes. It validates the magic bytes
      # and version number before parsing the envelope and payload.
      #
      # @param data [String] Binary data in the wire format.
      # @return [Message] The deserialized message.
      # @raise [InvalidFormatError] If magic bytes are not "ACTM".
      # @raise [VersionError] If version > WIRE_VERSION.
      def self.from_bytes(data)
        data = data.dup.force_encoding(Encoding::ASCII_8BIT)

        # Validate minimum size: we need at least the 17-byte header
        if data.bytesize < HEADER_SIZE
          raise InvalidFormatError, "Data too short: #{data.bytesize} bytes (minimum #{HEADER_SIZE})"
        end

        # Unpack the header fields using the same format as to_bytes
        magic, version, envelope_length, payload_length = data.unpack("a4CNQ>")

        # Check magic bytes — if these don't match, this isn't actor message data
        if magic != MAGIC
          raise InvalidFormatError, "Invalid magic bytes: expected 'ACTM', got '#{magic}'"
        end

        # Check version — if it's higher than what we support, we can't parse it
        if version > WIRE_VERSION
          raise VersionError, "Unsupported wire version #{version} (max supported: #{WIRE_VERSION})"
        end

        # Extract the envelope JSON and payload bytes from the data
        envelope_json = data[HEADER_SIZE, envelope_length].force_encoding(Encoding::UTF_8)
        payload_bytes = data[HEADER_SIZE + envelope_length, payload_length]

        # Parse the JSON envelope back into a hash
        envelope = JSON.parse(envelope_json)

        # Reconstruct the message with all original fields preserved
        new(
          id: envelope["id"],
          timestamp: envelope["timestamp"],
          sender_id: envelope["sender_id"],
          content_type: envelope["content_type"],
          payload: payload_bytes,
          metadata: envelope["metadata"]
        )
      end

      # Read exactly one message from an IO stream (file or socket).
      #
      # This method reads the header first to determine how many bytes
      # to read for the envelope and payload, then reads exactly that
      # many bytes. The stream is left positioned at the start of the
      # next message (or at EOF).
      #
      # @param io [IO] A readable IO object positioned at a message boundary.
      # @return [Message, nil] The next message, or nil if at EOF.
      # @raise [InvalidFormatError] If magic bytes are wrong.
      # @raise [VersionError] If version is too high.
      def self.from_io(io)
        # Read the 17-byte header. If we get nil or fewer than 17 bytes,
        # we've hit EOF or a truncated message.
        header_data = io.read(HEADER_SIZE)
        return nil if header_data.nil? || header_data.bytesize < HEADER_SIZE

        header_data.force_encoding(Encoding::ASCII_8BIT)
        magic, version, envelope_length, payload_length = header_data.unpack("a4CNQ>")

        if magic != MAGIC
          raise InvalidFormatError, "Invalid magic bytes: expected 'ACTM', got '#{magic}'"
        end

        if version > WIRE_VERSION
          raise VersionError, "Unsupported wire version #{version} (max supported: #{WIRE_VERSION})"
        end

        # Read envelope and payload based on lengths from the header
        envelope_data = io.read(envelope_length)
        return nil if envelope_data.nil? || envelope_data.bytesize < envelope_length

        payload_data = io.read(payload_length)
        # Allow nil/short payload for zero-length payloads
        payload_data = "".b if payload_data.nil?
        payload_data.force_encoding(Encoding::ASCII_8BIT)

        envelope = JSON.parse(envelope_data.force_encoding(Encoding::UTF_8))

        new(
          id: envelope["id"],
          timestamp: envelope["timestamp"],
          sender_id: envelope["sender_id"],
          content_type: envelope["content_type"],
          payload: payload_data,
          metadata: envelope["metadata"]
        )
      end
    end
  end
end
