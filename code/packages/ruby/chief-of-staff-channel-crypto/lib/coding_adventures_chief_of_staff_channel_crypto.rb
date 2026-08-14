# frozen_string_literal: true

# Portable D18F encrypted messages for Chief of Staff channels.
require "base64"
require "json"
require "coding_adventures_chacha20_poly1305"
require "coding_adventures_ed25519"
require "coding_adventures_sha256"

module CodingAdventures
  module ChiefOfStaffChannelCrypto
    VERSION = "0.1.0"
    MAX_MESSAGE_JSON_BYTES = 90 * 1024 * 1024

    MESSAGE_CONTEXT = "chief-channel-message-v1".b.freeze
    MESSAGE_MAGIC = "D18M".b.freeze
    WIRE_VERSION = 1
    MAX_IDENTITY_BYTES = 4 * 1024
    MAX_CONTENT_TYPE_BYTES = 1024
    MAX_CIPHERTEXT_BYTES = 64 * 1024 * 1024
    MAX_U64 = (1 << 64) - 1
    MAX_UUID_TIMESTAMP = (1 << 48) - 1
    RANDOM_MASK = (1 << 74) - 1
    JSON_FIELDS = %w[
      record_type wire_version message_id timestamp_ns originator_id_b64
      channel_id sequence key_epoch content_type plaintext_hash_hex
      ciphertext_b64 authentication_tag_b64 originator_signature_b64
    ].freeze
    ERROR_CODES = %w[
      invalid_magic unsupported_version truncated_record trailing_bytes
      length_limit_exceeded invalid_utf8 invalid_field invalid_json
      missing_epoch_key invalid_signature authentication_failed
      plaintext_hash_mismatch
    ].freeze

    class MessageProfileError < StandardError
      attr_reader :code

      def initialize(code)
        raise ArgumentError, "unknown D18F error code" unless ERROR_CODES.include?(code)

        @code = code.freeze
        super(code)
      end
    end

    class MessageFields
      def initialize(message_id:, timestamp_ns:, originator_id:, channel_id:, sequence:, key_epoch:, content_type:)
        @message_id = ChannelCryptoSupport.copy_bytes(message_id)
        @originator_id = ChannelCryptoSupport.copy_bytes(originator_id)
        @channel_id = ChannelCryptoSupport.copy_bytes(channel_id)
        @content_type = ChannelCryptoSupport.copy_text(content_type)
        ChannelCryptoSupport.require_length(@message_id, 16)
        ChannelCryptoSupport.require_u64(timestamp_ns)
        ChannelCryptoSupport.fail!("length_limit_exceeded") if @originator_id.bytesize > MAX_IDENTITY_BYTES
        ChannelCryptoSupport.require_length(@channel_id, 16)
        ChannelCryptoSupport.require_u64(sequence)
        ChannelCryptoSupport.require_u64(key_epoch)
        ChannelCryptoSupport.fail!("length_limit_exceeded") if @content_type.bytesize > MAX_CONTENT_TYPE_BYTES
        @timestamp_ns = timestamp_ns
        @sequence = sequence
        @key_epoch = key_epoch
        freeze
      end

      def message_id = @message_id.dup
      def timestamp_ns = @timestamp_ns
      def originator_id = @originator_id.dup
      def channel_id = @channel_id.dup
      def sequence = @sequence
      def key_epoch = @key_epoch
      def content_type = @content_type.dup

      def __values
        [@message_id, @timestamp_ns, @originator_id, @channel_id, @sequence, @key_epoch, @content_type]
      end
    end

    class SourcedMessageFields
      def initialize(originator_id:, channel_id:, sequence:, key_epoch:, content_type:)
        fields = MessageFields.new(
          message_id: "\0" * 16,
          timestamp_ns: 0,
          originator_id: originator_id,
          channel_id: channel_id,
          sequence: sequence,
          key_epoch: key_epoch,
          content_type: content_type
        )
        _, _, @originator_id, @channel_id, @sequence, @key_epoch, @content_type = fields.__values
        freeze
      end

      def __values = [@originator_id, @channel_id, @sequence, @key_epoch, @content_type]
    end

    class D18Message
      def initialize(message_id:, timestamp_ns:, originator_id:, channel_id:, sequence:, key_epoch:, content_type:,
                     plaintext_hash:, ciphertext:, authentication_tag:, originator_signature:)
        @fields = MessageFields.new(
          message_id: message_id,
          timestamp_ns: timestamp_ns,
          originator_id: originator_id,
          channel_id: channel_id,
          sequence: sequence,
          key_epoch: key_epoch,
          content_type: content_type
        )
        @plaintext_hash = ChannelCryptoSupport.copy_bytes(plaintext_hash)
        @ciphertext = ChannelCryptoSupport.copy_bytes(ciphertext)
        @authentication_tag = ChannelCryptoSupport.copy_bytes(authentication_tag)
        @originator_signature = ChannelCryptoSupport.copy_bytes(originator_signature)
        ChannelCryptoSupport.require_length(@plaintext_hash, 32)
        ChannelCryptoSupport.fail!("length_limit_exceeded") if @ciphertext.bytesize > MAX_CIPHERTEXT_BYTES
        ChannelCryptoSupport.require_length(@authentication_tag, 16)
        ChannelCryptoSupport.require_length(@originator_signature, 64)
        freeze
      end

      def fields
        values = @fields.__values
        MessageFields.new(
          message_id: values[0], timestamp_ns: values[1], originator_id: values[2], channel_id: values[3],
          sequence: values[4], key_epoch: values[5], content_type: values[6]
        )
      end

      def message_id = @fields.message_id
      def timestamp_ns = @fields.timestamp_ns
      def originator_id = @fields.originator_id
      def channel_id = @fields.channel_id
      def sequence = @fields.sequence
      def key_epoch = @fields.key_epoch
      def content_type = @fields.content_type
      def plaintext_hash = @plaintext_hash.dup
      def ciphertext = @ciphertext.dup
      def authentication_tag = @authentication_tag.dup
      def originator_signature = @originator_signature.dup

      def __values = [@fields, @plaintext_hash, @ciphertext, @authentication_tag, @originator_signature]
    end

    class MonotonicUuidV7Generator
      def initialize
        @last_timestamp_ms = nil
        @last_random = 0
      end

      def next(timestamp_ms, entropy)
        ChannelCryptoSupport.require_u64(timestamp_ms)
        ChannelCryptoSupport.fail!("invalid_field") if timestamp_ms > MAX_UUID_TIMESTAMP
        entropy_copy = ChannelCryptoSupport.copy_bytes(entropy)
        ChannelCryptoSupport.require_length(entropy_copy, 10)
        random = entropy_copy.unpack1("H*").to_i(16) & RANDOM_MASK
        effective_timestamp = timestamp_ms
        if !@last_timestamp_ms.nil? && timestamp_ms <= @last_timestamp_ms
          effective_timestamp = @last_timestamp_ms
          if @last_random < RANDOM_MASK
            random = @last_random + 1
          elsif effective_timestamp < MAX_UUID_TIMESTAMP
            effective_timestamp += 1
            random = 0
          else
            ChannelCryptoSupport.fail!("invalid_field")
          end
        end
        @last_timestamp_ms = effective_timestamp
        @last_random = random
        random_a = (random >> 62) & 0xfff
        random_b = random & ((1 << 62) - 1)
        value = (effective_timestamp << 80) | (7 << 76) | (random_a << 64) | (2 << 62) | random_b
        [value.to_s(16).rjust(32, "0")].pack("H*")
      end
    end

    module ChannelCryptoSupport
      module_function

      def fail!(code) = raise(MessageProfileError, code)

      def copy_bytes(value)
        fail!("invalid_field") unless value.is_a?(String)

        value.b.dup.freeze
      rescue Encoding::CompatibilityError
        fail!("invalid_field")
      end

      def copy_text(value)
        fail!("invalid_field") unless value.is_a?(String)

        copy = value.dup.force_encoding(Encoding::UTF_8)
        fail!("invalid_field") unless copy.valid_encoding?

        copy.freeze
      end

      def require_length(value, length)
        fail!("invalid_field") unless value.bytesize == length
      end

      def require_u64(value)
        fail!("invalid_field") unless value.is_a?(Integer) && value.between?(0, MAX_U64)
      end

      def validate_fields(fields)
        fail!("invalid_field") unless fields.is_a?(MessageFields)

        message_id, timestamp_ns, originator_id, channel_id, sequence, key_epoch, content_type = fields.__values
        validate_uuid_v7(message_id)
        validate_uuid_v7(channel_id)
        require_u64(timestamp_ns)
        require_u64(sequence)
        require_u64(key_epoch)
        fail!("invalid_field") if originator_id.empty?
        fail!("length_limit_exceeded") if originator_id.bytesize > MAX_IDENTITY_BYTES
        fail!("length_limit_exceeded") if content_type.bytesize > MAX_CONTENT_TYPE_BYTES
        validate_mime(content_type)
      end

      def validate_uuid_v7(value)
        require_length(value, 16)
        fail!("invalid_field") unless value.getbyte(6) >> 4 == 7 && value.getbyte(8) & 0xc0 == 0x80
      end

      def uuid_string(value)
        require_length(value, 16)
        hex = value.unpack1("H*")
        "#{hex[0, 8]}-#{hex[8, 4]}-#{hex[12, 4]}-#{hex[16, 4]}-#{hex[20, 12]}"
      end

      def decode_uuid_v7(value)
        fail!("invalid_field") unless value.is_a?(String) && value.match?(/\A[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\z/)

        decoded = [value.delete("-")].pack("H*")
        validate_uuid_v7(decoded)
        decoded
      end

      def u64be(value)
        require_u64(value)
        [value].pack("Q>")
      end

      def frame(parts) = parts.map { |part| u64be(part.bytesize) + part }.join.b

      def authenticated_header(fields, plaintext_hash)
        message_id, timestamp_ns, originator_id, channel_id, sequence, key_epoch, content_type = fields.__values
        frame([
          MESSAGE_CONTEXT, message_id, u64be(timestamp_ns), originator_id, channel_id,
          u64be(sequence), u64be(key_epoch), content_type.b, plaintext_hash
        ])
      end

      def nonce(channel_id, sequence) = channel_id + u64be(sequence)

      def validate_mime(value)
        encoded = value.b
        fail!("invalid_field") if encoded.empty? || encoded.bytes.any? { |byte| byte < 0x20 || byte > 0x7e }
        index = consume_token(encoded, 0)
        fail!("invalid_field") unless index < encoded.bytesize && encoded.getbyte(index) == 47
        index = consume_token(encoded, index + 1)
        while index < encoded.bytesize
          index = consume_spaces(encoded, index)
          fail!("invalid_field") unless index < encoded.bytesize && encoded.getbyte(index) == 59
          index = consume_spaces(encoded, index + 1)
          index = consume_token(encoded, index)
          index = consume_spaces(encoded, index)
          fail!("invalid_field") unless index < encoded.bytesize && encoded.getbyte(index) == 61
          index = consume_spaces(encoded, index + 1)
          if index < encoded.bytesize && encoded.getbyte(index) == 34
            index += 1
            loop do
              fail!("invalid_field") if index >= encoded.bytesize
              if encoded.getbyte(index) == 34
                index += 1
                break
              end
              if encoded.getbyte(index) == 92
                index += 1
                fail!("invalid_field") if index >= encoded.bytesize
              end
              index += 1
            end
          else
            index = consume_token(encoded, index)
          end
        end
      end

      def consume_token(value, index)
        start = index
        index += 1 while index < value.bytesize && mime_token?(value.getbyte(index))
        fail!("invalid_field") if index == start
        index
      end

      def consume_spaces(value, index)
        index += 1 while index < value.bytesize && value.getbyte(index) == 32
        index
      end

      def mime_token?(byte)
        byte.between?(48, 57) || byte.between?(65, 90) || byte.between?(97, 122) || "!#$%&'*+-.^_`|~".bytes.include?(byte)
      end

      def decode_decimal(value)
        fail!("invalid_field") unless value.is_a?(String) && value.match?(/\A(?:0|[1-9][0-9]*)\z/)

        decoded = Integer(value, 10)
        require_u64(decoded)
        decoded
      rescue ArgumentError
        fail!("invalid_field")
      end

      def decode_base64(value, maximum, exact = nil)
        fail!("invalid_field") unless value.is_a?(String) && value.bytesize % 4 == 0
        fail!("length_limit_exceeded") if value.bytesize / 4 * 3 > maximum + 2
        decoded = Base64.strict_decode64(value)
        fail!("length_limit_exceeded") if decoded.bytesize > maximum
        fail!("invalid_field") if !exact.nil? && decoded.bytesize != exact
        fail!("invalid_field") unless Base64.strict_encode64(decoded) == value
        decoded
      rescue ArgumentError
        fail!("invalid_field")
      end

      def decode_hex(value, length)
        fail!("invalid_field") unless value.is_a?(String) && value.match?(/\A[0-9a-f]+\z/) && value.bytesize == length * 2

        [value].pack("H*")
      end

      def validate_json_surrogates!(data)
        in_string = false
        index = 0
        while index < data.bytesize
          byte = data.getbyte(index)
          if !in_string
            in_string = true if byte == 34
            index += 1
            next
          end
          if byte == 34
            in_string = false
            index += 1
            next
          end
          unless byte == 92
            index += 1
            next
          end
          index += 1
          break if index >= data.bytesize
          unless data.getbyte(index) == 117
            index += 1
            next
          end
          hex = data.byteslice(index + 1, 4)
          index += 5
          next unless hex&.match?(/\A[0-9a-fA-F]{4}\z/)

          code = hex.to_i(16)
          if code.between?(0xd800, 0xdbff)
            low_escape = data.byteslice(index, 6)
            fail!("invalid_field") unless low_escape&.match?(/\A\\u[0-9a-fA-F]{4}\z/)
            low = low_escape[2, 4].to_i(16)
            fail!("invalid_field") unless low.between?(0xdc00, 0xdfff)
            index += 6
          elsif code.between?(0xdc00, 0xdfff)
            fail!("invalid_field")
          end
        end
      end
    end

    module_function

    def validate_message_fields(fields) = ChannelCryptoSupport.validate_fields(fields)

    def message_create(fields, plaintext, signing_secret_key, channel_master_key)
      ChannelCryptoSupport.validate_fields(fields)
      plaintext_copy = ChannelCryptoSupport.copy_bytes(plaintext)
      signing_key = ChannelCryptoSupport.copy_bytes(signing_secret_key)
      channel_key = ChannelCryptoSupport.copy_bytes(channel_master_key)
      ChannelCryptoSupport.fail!("length_limit_exceeded") if plaintext_copy.bytesize > MAX_CIPHERTEXT_BYTES
      ChannelCryptoSupport.require_length(signing_key, 64)
      ChannelCryptoSupport.require_length(channel_key, 32)
      plaintext_hash = Sha256.sha256(plaintext_copy)
      header = ChannelCryptoSupport.authenticated_header(fields, plaintext_hash)
      values = fields.__values
      ciphertext, tag = Chacha20Poly1305.xchacha20_poly1305_encrypt(
        plaintext_copy, channel_key, ChannelCryptoSupport.nonce(values[3], values[4]), header
      )
      D18Message.new(
        message_id: values[0], timestamp_ns: values[1], originator_id: values[2], channel_id: values[3],
        sequence: values[4], key_epoch: values[5], content_type: values[6], plaintext_hash: plaintext_hash,
        ciphertext: ciphertext, authentication_tag: tag, originator_signature: Ed25519.sign(header, signing_key)
      )
    rescue MessageProfileError
      raise
    rescue StandardError
      ChannelCryptoSupport.fail!("invalid_field")
    end

    def message_create_with_sources(fields, plaintext, signing_secret_key, channel_master_key, uuid_source, clock)
      ChannelCryptoSupport.fail!("invalid_field") unless fields.is_a?(SourcedMessageFields)
      originator_id, channel_id, sequence, key_epoch, content_type = fields.__values
      complete = MessageFields.new(
        message_id: uuid_source.next_uuid_v7,
        timestamp_ns: clock.now_nanoseconds,
        originator_id: originator_id,
        channel_id: channel_id,
        sequence: sequence,
        key_epoch: key_epoch,
        content_type: content_type
      )
      message_create(complete, plaintext, signing_secret_key, channel_master_key)
    rescue MessageProfileError
      raise
    rescue StandardError
      ChannelCryptoSupport.fail!("invalid_field")
    end

    def message_verify(message, originator_public_key, channel_master_key)
      ChannelCryptoSupport.fail!("invalid_field") unless message.is_a?(D18Message)
      ChannelCryptoSupport.validate_fields(message.__values[0])
      key = ChannelCryptoSupport.copy_bytes(channel_master_key)
      ChannelCryptoSupport.require_length(key, 32)
      verify_cryptography(message, originator_public_key, key)
    end

    def message_verify_with_key_resolver(message, originator_public_key, key_for_epoch = nil, &block)
      ChannelCryptoSupport.fail!("invalid_field") unless message.is_a?(D18Message)
      ChannelCryptoSupport.validate_fields(message.__values[0])
      resolver = key_for_epoch || block
      ChannelCryptoSupport.fail!("invalid_field") unless resolver.respond_to?(:call)
      key = resolver.call(message.key_epoch)
      ChannelCryptoSupport.fail!("missing_epoch_key") if key.nil?
      key_copy = ChannelCryptoSupport.copy_bytes(key)
      ChannelCryptoSupport.require_length(key_copy, 32)
      verify_cryptography(message, originator_public_key, key_copy)
    end

    def verify_cryptography(message, originator_public_key, channel_master_key)
      fields, plaintext_hash, ciphertext, tag, signature = message.__values
      public_key = ChannelCryptoSupport.copy_bytes(originator_public_key)
      ChannelCryptoSupport.require_length(public_key, 32)
      header = ChannelCryptoSupport.authenticated_header(fields, plaintext_hash)
      signature_valid = Ed25519.verify(header, signature, public_key)
      ChannelCryptoSupport.fail!("invalid_signature") unless signature_valid
      values = fields.__values
      begin
        plaintext = Chacha20Poly1305.xchacha20_poly1305_decrypt(
          ciphertext, channel_master_key, ChannelCryptoSupport.nonce(values[3], values[4]), header, tag
        )
      rescue StandardError
        ChannelCryptoSupport.fail!("authentication_failed")
      end
      actual_hash = Sha256.sha256(plaintext)
      difference = actual_hash.bytes.zip(plaintext_hash.bytes).reduce(0) { |result, pair| result | (pair[0] ^ pair[1]) }
      ChannelCryptoSupport.fail!("plaintext_hash_mismatch") unless difference.zero?
      plaintext
    rescue MessageProfileError
      raise
    rescue StandardError
      ChannelCryptoSupport.fail!("invalid_signature")
    end
    private_class_method :verify_cryptography

    def message_authenticated_header(message)
      ChannelCryptoSupport.fail!("invalid_field") unless message.is_a?(D18Message)
      fields, plaintext_hash = message.__values
      ChannelCryptoSupport.authenticated_header(fields, plaintext_hash)
    end

    def message_serialize(message)
      ChannelCryptoSupport.fail!("invalid_field") unless message.is_a?(D18Message)
      fields, plaintext_hash, ciphertext, tag, signature = message.__values
      message_id, timestamp_ns, originator_id, channel_id, sequence, key_epoch, content_type = fields.__values
      [
        MESSAGE_MAGIC, WIRE_VERSION.chr.b, message_id, ChannelCryptoSupport.u64be(timestamp_ns),
        [originator_id.bytesize].pack("N"), originator_id, channel_id, ChannelCryptoSupport.u64be(sequence),
        ChannelCryptoSupport.u64be(key_epoch), [content_type.bytesize].pack("N"), content_type.b,
        plaintext_hash, ChannelCryptoSupport.u64be(ciphertext.bytesize), ciphertext, tag, signature
      ].join.b
    end

    class BinaryDecoder
      def initialize(data)
        @data = ChannelCryptoSupport.copy_bytes(data)
        @position = 0
      end

      def take(length)
        ChannelCryptoSupport.fail!("truncated_record") if length.negative? || length > @data.bytesize - @position
        result = @data.byteslice(@position, length)
        @position += length
        result
      end

      def read_u64 = take(8).unpack1("Q>")

      def bounded_u32(maximum) = bounded(take(4).unpack1("N"), maximum)
      def bounded_u64(maximum) = bounded(read_u64, maximum)

      def bounded(length, maximum)
        ChannelCryptoSupport.fail!("length_limit_exceeded") if length > maximum
        take(length)
      end

      def finish
        ChannelCryptoSupport.fail!("trailing_bytes") unless @position == @data.bytesize
      end
    end

    def message_deserialize(data)
      decoder = BinaryDecoder.new(data)
      ChannelCryptoSupport.fail!("invalid_magic") unless decoder.take(4) == MESSAGE_MAGIC
      ChannelCryptoSupport.fail!("unsupported_version") unless decoder.take(1).getbyte(0) == WIRE_VERSION
      message_id = decoder.take(16)
      timestamp_ns = decoder.read_u64
      originator_id = decoder.bounded_u32(MAX_IDENTITY_BYTES)
      channel_id = decoder.take(16)
      sequence = decoder.read_u64
      key_epoch = decoder.read_u64
      content_type = decoder.bounded_u32(MAX_CONTENT_TYPE_BYTES).force_encoding(Encoding::UTF_8)
      ChannelCryptoSupport.fail!("invalid_utf8") unless content_type.valid_encoding?
      plaintext_hash = decoder.take(32)
      ciphertext = decoder.bounded_u64(MAX_CIPHERTEXT_BYTES)
      tag = decoder.take(16)
      signature = decoder.take(64)
      decoder.finish
      D18Message.new(
        message_id: message_id, timestamp_ns: timestamp_ns, originator_id: originator_id, channel_id: channel_id,
        sequence: sequence, key_epoch: key_epoch, content_type: content_type, plaintext_hash: plaintext_hash,
        ciphertext: ciphertext, authentication_tag: tag, originator_signature: signature
      )
    end

    def message_to_json(message)
      ChannelCryptoSupport.fail!("invalid_field") unless message.is_a?(D18Message)
      fields, plaintext_hash, ciphertext, tag, signature = message.__values
      message_id, timestamp_ns, originator_id, channel_id, sequence, key_epoch, content_type = fields.__values
      value = {
        "record_type" => "D18M",
        "wire_version" => 1,
        "message_id" => ChannelCryptoSupport.uuid_string(message_id),
        "timestamp_ns" => timestamp_ns.to_s,
        "originator_id_b64" => Base64.strict_encode64(originator_id),
        "channel_id" => ChannelCryptoSupport.uuid_string(channel_id),
        "sequence" => sequence.to_s,
        "key_epoch" => key_epoch.to_s,
        "content_type" => content_type,
        "plaintext_hash_hex" => plaintext_hash.unpack1("H*"),
        "ciphertext_b64" => Base64.strict_encode64(ciphertext),
        "authentication_tag_b64" => Base64.strict_encode64(tag),
        "originator_signature_b64" => Base64.strict_encode64(signature)
      }
      encoded = JSON.generate(value).encode(Encoding::UTF_8)
      ChannelCryptoSupport.fail!("length_limit_exceeded") if encoded.bytesize > MAX_MESSAGE_JSON_BYTES
      encoded
    rescue MessageProfileError
      raise
    rescue StandardError
      ChannelCryptoSupport.fail!("invalid_field")
    end

    def message_from_json(data)
      raw = ChannelCryptoSupport.copy_bytes(data)
      ChannelCryptoSupport.fail!("length_limit_exceeded") if raw.bytesize > MAX_MESSAGE_JSON_BYTES
      text = raw.dup.force_encoding(Encoding::UTF_8)
      ChannelCryptoSupport.fail!("invalid_json") unless text.valid_encoding?
      ChannelCryptoSupport.validate_json_surrogates!(text)
      value = JSON.parse(text, allow_duplicate_key: false)
      ChannelCryptoSupport.fail!("invalid_json") unless value.is_a?(Hash) && value.keys.sort == JSON_FIELDS.sort
      ChannelCryptoSupport.fail!("invalid_json") unless value["wire_version"].is_a?(Integer)
      JSON_FIELDS.each do |name|
        next if name == "wire_version"
        ChannelCryptoSupport.fail!("invalid_json") unless value[name].is_a?(String)
      end
      ChannelCryptoSupport.fail!("invalid_magic") unless value["record_type"] == "D18M"
      ChannelCryptoSupport.fail!("unsupported_version") unless value["wire_version"] == 1
      message_id = ChannelCryptoSupport.decode_uuid_v7(value["message_id"])
      timestamp_ns = ChannelCryptoSupport.decode_decimal(value["timestamp_ns"])
      originator_id = ChannelCryptoSupport.decode_base64(value["originator_id_b64"], MAX_IDENTITY_BYTES)
      channel_id = ChannelCryptoSupport.decode_uuid_v7(value["channel_id"])
      sequence = ChannelCryptoSupport.decode_decimal(value["sequence"])
      key_epoch = ChannelCryptoSupport.decode_decimal(value["key_epoch"])
      content_type = ChannelCryptoSupport.copy_text(value["content_type"])
      ChannelCryptoSupport.fail!("length_limit_exceeded") if content_type.bytesize > MAX_CONTENT_TYPE_BYTES
      plaintext_hash = ChannelCryptoSupport.decode_hex(value["plaintext_hash_hex"], 32)
      ciphertext = ChannelCryptoSupport.decode_base64(value["ciphertext_b64"], MAX_CIPHERTEXT_BYTES)
      tag = ChannelCryptoSupport.decode_base64(value["authentication_tag_b64"], 16, 16)
      signature = ChannelCryptoSupport.decode_base64(value["originator_signature_b64"], 64, 64)
      D18Message.new(
        message_id: message_id, timestamp_ns: timestamp_ns, originator_id: originator_id, channel_id: channel_id,
        sequence: sequence, key_epoch: key_epoch, content_type: content_type, plaintext_hash: plaintext_hash,
        ciphertext: ciphertext, authentication_tag: tag, originator_signature: signature
      )
    rescue MessageProfileError
      raise
    rescue JSON::ParserError, EncodingError, TypeError
      ChannelCryptoSupport.fail!("invalid_json")
    end
  end
end
