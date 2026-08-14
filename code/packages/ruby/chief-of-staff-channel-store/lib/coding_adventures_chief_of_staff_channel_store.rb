# frozen_string_literal: true

require "coding_adventures_chief_of_staff_channel_crypto"
require "coding_adventures_sha256"

module CodingAdventures
  module ChiefOfStaffChannelStore
    VERSION = "0.1.0"

    STORAGE_NAMESPACE = "chief-channels"
    DEFINITION_CONTENT_TYPE = "application/vnd.coding-adventures.chief-channel-definition-v1"
    STATE_CONTENT_TYPE = "application/vnd.coding-adventures.chief-channel-state-v1"
    MESSAGE_CONTENT_TYPE = "application/vnd.coding-adventures.chief-channel-message-v1"
    GRANT_CONTENT_TYPE = "application/vnd.coding-adventures.chief-channel-key-grant-v1"
    ACK_CONTENT_TYPE = "application/vnd.coding-adventures.chief-channel-ack-v1"
    MAX_IDENTITY_BYTES = 4 * 1024
    MAX_CONTENT_TYPE_BYTES = 1024
    MAX_RECEIVERS = 1024
    MAX_PENDING_HEADER_BYTES = 16 * 1024
    MAX_STORE_CAS_ATTEMPTS = 16
    MAX_DEFINITION_CAS_ATTEMPTS = 16
    MAX_U64 = (1 << 64) - 1
    ERROR_CODES = %w[
      invalid_definition invalid_message_id definition_not_found conflicting_definition
      corrupt_definition definition_changed channel_destroyed unauthorized_originator
      unauthorized_receiver public_key_mismatch missing_key_grant unknown_message_id
      unauthorized_message not_initialized corrupt_record pending_append no_pending_append
      pending_header_mismatch conflicting_record concurrent_update invalid_receiver_id
      invalid_page_size acknowledgement_regression acknowledgement_ahead
      acknowledgement_pending sequence_exhausted storage_error wire_error crypto_error
      metadata_error
    ].freeze

    class ChannelProfileError < StandardError
      attr_reader :code

      def initialize(code)
        raise ArgumentError, "unknown D18P error code" unless ERROR_CODES.include?(code)

        @code = code.freeze
        super(code)
      end
    end

    class OriginatorIdentity
      def initialize(agent_id:, public_key:)
        @agent_id = Support.bytes(agent_id, "invalid_definition")
        @public_key = Support.bytes(public_key, "invalid_definition")
        Support.fail!("invalid_definition") unless @public_key.bytesize == 32
        freeze
      end

      def agent_id = @agent_id.dup
      def public_key = @public_key.dup
      def __values = [@agent_id, @public_key]
    end

    class ReceiverIdentity
      def initialize(agent_id:, public_key:)
        @agent_id = Support.bytes(agent_id, "invalid_definition")
        @public_key = Support.bytes(public_key, "invalid_definition")
        Support.fail!("invalid_definition") unless @public_key.bytesize == 32
        freeze
      end

      def agent_id = @agent_id.dup
      def public_key = @public_key.dup
      def __values = [@agent_id, @public_key]
    end

    class ChannelDefinition
      def initialize(channel_id:, originator:, receivers:, created_at_ns:, key_epoch:, lifecycle: "active")
        @channel_id = Support.bytes(channel_id, "invalid_definition")
        Support.uuid_v7!(@channel_id, "invalid_definition")
        Support.fail!("invalid_definition") unless originator.is_a?(OriginatorIdentity) && receivers.respond_to?(:to_a)
        @originator = OriginatorIdentity.new(agent_id: originator.agent_id, public_key: originator.public_key)
        Support.agent_id!(@originator.agent_id, "invalid_definition")
        @receivers = receivers.to_a.map { |receiver| ReceiverIdentity.new(agent_id: receiver.agent_id, public_key: receiver.public_key) }
        @receivers.sort_by!(&:agent_id)
        Support.fail!("invalid_definition") unless @receivers.length.between?(1, MAX_RECEIVERS)
        previous = nil
        @receivers.each do |receiver|
          id = receiver.agent_id
          Support.agent_id!(id, "invalid_definition")
          Support.fail!("invalid_definition") if id == @originator.agent_id || id == previous
          previous = id
        end
        Support.u64!(created_at_ns, "invalid_definition")
        Support.u64!(key_epoch, "invalid_definition")
        Support.fail!("invalid_definition") unless %w[active destroyed].include?(lifecycle)
        @created_at_ns = created_at_ns
        @key_epoch = key_epoch
        @lifecycle = lifecycle.dup.freeze
        @receivers.freeze
        freeze
      end

      def channel_id = @channel_id.dup
      def originator = OriginatorIdentity.new(agent_id: @originator.agent_id, public_key: @originator.public_key)
      def receivers = @receivers.map { |receiver| ReceiverIdentity.new(agent_id: receiver.agent_id, public_key: receiver.public_key) }.freeze
      def created_at_ns = @created_at_ns
      def key_epoch = @key_epoch
      def lifecycle = @lifecycle.dup
      def receiver(agent_id) = @receivers.find { |receiver| receiver.agent_id == agent_id }
      def with_lifecycle(value) = ChannelDefinition.new(channel_id: @channel_id, originator: @originator, receivers: @receivers, created_at_ns: @created_at_ns, key_epoch: @key_epoch, lifecycle: value)
      def ==(other) = other.is_a?(ChannelDefinition) && ChannelProfile.definition_serialize(self) == ChannelProfile.definition_serialize(other)
      alias eql? ==
    end

    class MessageHeader
      def initialize(message_id:, timestamp_ns:, originator_id:, channel_id:, sequence:, key_epoch:, content_type:, plaintext_hash:)
        @message_id = Support.bytes(message_id, "wire_error")
        @originator_id = Support.bytes(originator_id, "wire_error")
        @channel_id = Support.bytes(channel_id, "wire_error")
        @content_type = Support.text(content_type, "wire_error")
        @plaintext_hash = Support.bytes(plaintext_hash, "wire_error")
        Support.fail!("wire_error") unless @message_id.bytesize == 16 && @channel_id.bytesize == 16 && @plaintext_hash.bytesize == 32
        Support.u64!(timestamp_ns, "wire_error")
        Support.u64!(sequence, "wire_error")
        Support.u64!(key_epoch, "wire_error")
        Support.fail!("wire_error") if @originator_id.bytesize > MAX_IDENTITY_BYTES || @content_type.bytesize > MAX_CONTENT_TYPE_BYTES
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
      def plaintext_hash = @plaintext_hash.dup
      def __values = [@message_id, @timestamp_ns, @originator_id, @channel_id, @sequence, @key_epoch, @content_type, @plaintext_hash]
      def ==(other) = other.is_a?(MessageHeader) && ChannelProfile.header_serialize(self) == ChannelProfile.header_serialize(other)
      alias eql? ==
    end

    class ChannelState
      attr_reader :next_sequence, :pending_header

      def initialize(next_sequence:, pending_header: nil)
        Support.u64!(next_sequence, "corrupt_record")
        Support.fail!("corrupt_record") unless pending_header.nil? || pending_header.is_a?(MessageHeader)
        @next_sequence = next_sequence
        @pending_header = pending_header
        freeze
      end
    end

    module Support
      module_function

      def fail!(code) = raise(ChannelProfileError, code)

      def bytes(value, code)
        fail!(code) unless value.is_a?(String)
        value.b.dup.freeze
      rescue Encoding::CompatibilityError
        fail!(code)
      end

      def text(value, code)
        fail!(code) unless value.is_a?(String)
        copy = value.dup.force_encoding(Encoding::UTF_8)
        fail!(code) unless copy.valid_encoding?
        copy.freeze
      end

      def u64!(value, code)
        fail!(code) unless value.is_a?(Integer) && value.between?(0, MAX_U64)
      end

      def agent_id!(value, code)
        fail!(code) unless value.is_a?(String) && value.bytesize.between?(1, MAX_IDENTITY_BYTES)
      end

      def uuid_v7!(value, code)
        fail!(code) unless value.bytesize == 16 && value.getbyte(6) >> 4 == 7 && value.getbyte(8) & 0xc0 == 0x80
      end

      def remap(code)
        yield
      rescue StandardError
        fail!(code)
      end

      def digest(value) = CodingAdventures::Sha256.sha256(value)
    end

    class Writer
      def initialize = @value = +"".b
      def bytes(value) = (@value << value; self)
      def u8(value) = bytes([value].pack("C"))
      def u32(value) = bytes([value].pack("N"))
      def u64(value) = bytes([value].pack("Q>"))
      def sized(value) = u32(value.bytesize).bytes(value)
      def finish = @value.dup.freeze
    end

    class Reader
      def initialize(value, code)
        @value = Support.bytes(value, code)
        @position = 0
        @code = code
      end

      def take(length)
        Support.fail!(@code) unless length.is_a?(Integer) && length >= 0 && @position + length <= @value.bytesize
        result = @value.byteslice(@position, length)
        @position += length
        result
      end

      def u8 = take(1).unpack1("C")
      def u32 = take(4).unpack1("N")
      def u64 = take(8).unpack1("Q>")
      def sized(maximum) = (length = u32; Support.fail!(@code) if length > maximum; take(length))
      def magic(value)
        Support.fail!(@code) unless take(4) == value
      end

      def version
        Support.fail!(@code) unless u8 == 1
      end

      def finish
        Support.fail!(@code) unless @position == @value.bytesize
      end
    end

    module ChannelProfile
      module_function

      def definition_serialize(definition)
        writer = Writer.new.bytes("D18C".b).u8(1).bytes(definition.channel_id)
        writer.sized(definition.originator.agent_id).bytes(definition.originator.public_key).u32(definition.receivers.length)
        definition.receivers.each { |receiver| writer.sized(receiver.agent_id).bytes(receiver.public_key) }
        writer.u64(definition.created_at_ns).u64(definition.key_epoch).u8(definition.lifecycle == "active" ? 0 : 1).finish
      end

      def definition_deserialize(value)
        Support.remap("corrupt_definition") do
          reader = Reader.new(value, "corrupt_definition")
          reader.magic("D18C".b)
          reader.version
          channel_id = reader.take(16)
          originator = OriginatorIdentity.new(agent_id: reader.sized(MAX_IDENTITY_BYTES), public_key: reader.take(32))
          count = reader.u32
          Support.fail!("corrupt_definition") unless count.between?(1, MAX_RECEIVERS)
          receivers = Array.new(count) { ReceiverIdentity.new(agent_id: reader.sized(MAX_IDENTITY_BYTES), public_key: reader.take(32)) }
          created_at_ns = reader.u64
          key_epoch = reader.u64
          lifecycle = reader.u8
          Support.fail!("corrupt_definition") unless lifecycle.between?(0, 1)
          reader.finish
          ChannelDefinition.new(channel_id: channel_id, originator: originator, receivers: receivers, created_at_ns: created_at_ns, key_epoch: key_epoch, lifecycle: lifecycle.zero? ? "active" : "destroyed")
        end
      end

      def header_serialize(header)
        values = header.__values
        Writer.new.bytes("D18H".b).u8(1).bytes(values[0]).u64(values[1]).sized(values[2]).bytes(values[3]).u64(values[4]).u64(values[5]).sized(values[6].b).bytes(values[7]).finish
      end

      def header_deserialize(value)
        reader = Reader.new(value, "wire_error")
        reader.magic("D18H".b)
        reader.version
        message_id = reader.take(16)
        timestamp_ns = reader.u64
        originator_id = reader.sized(MAX_IDENTITY_BYTES)
        channel_id = reader.take(16)
        sequence = reader.u64
        key_epoch = reader.u64
        content_type = reader.sized(MAX_CONTENT_TYPE_BYTES).force_encoding(Encoding::UTF_8)
        Support.fail!("wire_error") unless content_type.valid_encoding?
        plaintext_hash = reader.take(32)
        reader.finish
        MessageHeader.new(message_id: message_id, timestamp_ns: timestamp_ns, originator_id: originator_id, channel_id: channel_id, sequence: sequence, key_epoch: key_epoch, content_type: content_type, plaintext_hash: plaintext_hash)
      end

      def state_serialize(state)
        writer = Writer.new.bytes("D18S".b).u8(1).u64(state.next_sequence)
        return writer.u8(0).finish if state.pending_header.nil?

        header = header_serialize(state.pending_header)
        Support.fail!("corrupt_record") if header.bytesize > MAX_PENDING_HEADER_BYTES
        writer.u8(1).u32(header.bytesize).bytes(header).finish
      end

      def state_deserialize(value, channel_id)
        Support.remap("corrupt_record") do
          reader = Reader.new(value, "corrupt_record")
          reader.magic("D18S".b)
          reader.version
          next_sequence = reader.u64
          flag = reader.u8
          if flag.zero?
            reader.finish
            next ChannelState.new(next_sequence: next_sequence)
          end
          Support.fail!("corrupt_record") unless flag == 1
          length = reader.u32
          Support.fail!("corrupt_record") if length > MAX_PENDING_HEADER_BYTES
          pending = header_deserialize(reader.take(length))
          reader.finish
          Support.fail!("corrupt_record") unless pending.channel_id == channel_id && pending.sequence != MAX_U64 && pending.sequence + 1 == next_sequence
          ChannelState.new(next_sequence: next_sequence, pending_header: pending)
        end
      end

      def cursor_serialize(first_unread) = (Support.u64!(first_unread, "corrupt_record"); "D18A".b + [1, first_unread].pack("CQ>"))

      def cursor_deserialize(value)
        reader = Reader.new(value, "corrupt_record")
        reader.magic("D18A".b)
        reader.version
        result = reader.u64
        reader.finish
        result
      end

      def definition_key(channel_id) = checked_channel(channel_id, "invalid_definition") + "/definition"
      def state_key(channel_id) = checked_channel(channel_id, "invalid_definition") + "/state/next-sequence"
      def message_prefix(channel_id) = checked_channel(channel_id, "invalid_definition") + "/messages/"
      def message_key(channel_id, sequence) = message_prefix(channel_id) + format("%020d", sequence)

      def grant_key(channel_id, epoch, receiver_id)
        Support.agent_id!(receiver_id, "invalid_receiver_id")
        checked_channel(channel_id, "invalid_definition") + "/grants/#{format('%020d', epoch)}/#{Support.digest(receiver_id).unpack1('H*')}"
      end

      def ack_key(channel_id, receiver_id)
        Support.agent_id!(receiver_id, "invalid_receiver_id")
        checked_channel(channel_id, "invalid_definition") + "/receivers/#{Support.digest(receiver_id).unpack1('H*')}/ack"
      end

      def checked_channel(value, code)
        bytes = Support.bytes(value, code)
        Support.fail!(code) unless bytes.bytesize == 16
        bytes.unpack1("H*")
      end
    end
  end
end

require_relative "coding_adventures_chief_of_staff_channel_store/storage"
require_relative "coding_adventures_chief_of_staff_channel_store/endpoints"
