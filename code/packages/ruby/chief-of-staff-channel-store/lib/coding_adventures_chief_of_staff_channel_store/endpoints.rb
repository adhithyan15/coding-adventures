# frozen_string_literal: true

module CodingAdventures
  module ChiefOfStaffChannelStore
    MessageMetadata = Struct.new(:message_id, :timestamp_ns, keyword_init: true)
    PublishedMessage = Struct.new(:message_id, :sequence, :timestamp_ns, keyword_init: true)
    ReceivedMessage = Struct.new(:message_id, :sequence, :timestamp_ns, :content_type, :payload, keyword_init: true)

    class ChannelDefinitionStore
      def initialize(backend) = @backend = backend

      def create(definition)
        Support.fail!("invalid_definition") unless definition.lifecycle == "active"
        backend { @backend.initialize_backend }
        key = ChannelProfile.definition_key(definition.channel_id)
        body = ChannelProfile.definition_serialize(definition)
        begin
          persisted = require_record(@backend.put(StoragePut.new(namespace: STORAGE_NAMESPACE, key: key, content_type: DEFINITION_CONTENT_TYPE, body: body, if_absent: true)), definition.channel_id)
        rescue StorageConflictError
          existing = backend { @backend.get(STORAGE_NAMESPACE, key) }
          Support.fail!("definition_not_found") if existing.nil?
          Support.fail!("corrupt_definition") unless existing.content_type == DEFINITION_CONTENT_TYPE
          Support.fail!("conflicting_definition") unless existing.body == body
          persisted = require_record(existing, definition.channel_id)
        rescue StandardError => error
          raise backend_error(error)
        end
        Support.fail!("conflicting_definition") unless persisted == definition
        ChannelStore.new(@backend, definition.channel_id).initialize_store
        require_current(definition)
      end

      def load(channel_id)
        backend { @backend.initialize_backend }
        loaded = load_record(channel_id)
        loaded&.first
      end

      def destroy(channel_id)
        backend { @backend.initialize_backend }
        MAX_DEFINITION_CAS_ATTEMPTS.times do
          loaded = load_record(channel_id)
          Support.fail!("definition_not_found") if loaded.nil?
          definition, revision = loaded
          return definition if definition.lifecycle == "destroyed"

          destroyed = definition.with_lifecycle("destroyed")
          begin
            record = @backend.put(StoragePut.new(namespace: STORAGE_NAMESPACE, key: ChannelProfile.definition_key(channel_id), content_type: DEFINITION_CONTENT_TYPE, body: ChannelProfile.definition_serialize(destroyed), if_revision: revision))
            return require_record(record, channel_id)
          rescue StorageConflictError
            next
          rescue StandardError => error
            raise backend_error(error)
          end
        end
        Support.fail!("concurrent_update")
      end

      def require_current(expected)
        actual = load(expected.channel_id)
        Support.fail!("definition_not_found") if actual.nil?
        Support.fail!("channel_destroyed") if actual.lifecycle == "destroyed"
        Support.fail!("definition_changed") unless actual == expected
        actual
      end

      private

      def load_record(channel_id)
        key = ChannelProfile.definition_key(channel_id)
        record = backend { @backend.get(STORAGE_NAMESPACE, key) }
        return nil if record.nil?

        [require_record(record, channel_id), record.revision]
      end

      def require_record(record, channel_id)
        Support.fail!("corrupt_definition") unless record.content_type == DEFINITION_CONTENT_TYPE
        definition = ChannelProfile.definition_deserialize(record.body)
        Support.fail!("corrupt_definition") unless definition.channel_id == channel_id && record.key == ChannelProfile.definition_key(channel_id)
        definition
      end

      def backend
        yield
      rescue ChannelProfileError, StorageConflictError
        raise
      rescue StandardError => error
        raise backend_error(error)
      end

      def backend_error(error) = error.is_a?(ChannelProfileError) ? error : ChannelProfileError.new("storage_error")
    end

    class DurableOriginator
      def self.open(backend:, channel_id:, agent_id:, signing_secret_key:, channel_master_key:, metadata_source:)
        definition = EndpointSupport.active_definition(backend, channel_id)
        Support.fail!("unauthorized_originator") unless definition.originator.agent_id == agent_id
        Support.fail!("public_key_mismatch") unless signing_secret_key.is_a?(String) && signing_secret_key.bytesize == 64 && definition.originator.public_key == signing_secret_key.byteslice(32, 32)
        Support.fail!("crypto_error") unless channel_master_key.is_a?(String) && channel_master_key.bytesize == 32
        ChannelStore.new(backend, channel_id).initialize_store
        new(backend, definition, signing_secret_key, channel_master_key, metadata_source)
      end

      def initialize(backend, definition, signing_secret_key, channel_master_key, metadata_source)
        @backend = backend
        @definition = definition
        @signing_secret_key = signing_secret_key.b.dup.freeze
        @channel_master_key = channel_master_key.b.dup.freeze
        @metadata_source = metadata_source
      end

      def id = @definition.originator.agent_id
      def channel_id = @definition.channel_id
      def public_key = @definition.originator.public_key

      def publish(payload, content_type)
        metadata = @metadata_source.next
        publish_with_metadata(metadata, payload, content_type)
      rescue ChannelProfileError
        raise
      rescue StandardError
        Support.fail!("metadata_error")
      end

      def publish_with_metadata(metadata, payload, content_type)
        Support.uuid_v7!(metadata.message_id, "invalid_message_id")
        ChannelDefinitionStore.new(@backend).require_current(@definition)
        request = AppendRequest.new(message_id: metadata.message_id, timestamp_ns: metadata.timestamp_ns, originator_id: @definition.originator.agent_id, key_epoch: @definition.key_epoch, content_type: content_type)
        message = ChannelStore.new(@backend, @definition.channel_id).append(request, payload, @channel_master_key, @signing_secret_key)
        PublishedMessage.new(message_id: metadata.message_id.b.dup.freeze, sequence: message.sequence, timestamp_ns: metadata.timestamp_ns).freeze
      end

      def save_receiver_grant(receiver_id, grant_body)
        definition = ChannelDefinitionStore.new(@backend).require_current(@definition)
        Support.fail!("unauthorized_receiver") if definition.receiver(receiver_id).nil?
        ChannelStore.new(@backend, definition.channel_id).save_key_grant(OpaqueKeyGrant.new(channel_id: definition.channel_id, key_epoch: definition.key_epoch, receiver_id: receiver_id.b.dup.freeze, body: grant_body.b.dup.freeze))
      end
    end

    class DurableReceiver
      def self.open(backend:, channel_id:, receiver_id:, key_provider:)
        Support.agent_id!(receiver_id, "invalid_receiver_id")
        definition = EndpointSupport.active_definition(backend, channel_id)
        receiver = definition.receiver(receiver_id)
        Support.fail!("unauthorized_receiver") if receiver.nil?
        Support.fail!("public_key_mismatch") unless receiver.public_key == key_provider.public_key
        ChannelStore.new(backend, channel_id).initialize_store
        new(backend, definition, receiver_id, key_provider)
      end

      def initialize(backend, definition, receiver_id, key_provider)
        @backend = backend
        @definition = definition
        @receiver_id = receiver_id.b.dup.freeze
        @key_provider = key_provider
        @delivered = {}
      end

      def id = @receiver_id.dup
      def channel_id = @definition.channel_id
      def public_key = @key_provider.public_key.b.dup

      def receive(limit)
        ChannelDefinitionStore.new(@backend).require_current(@definition)
        store = ChannelStore.new(@backend, @definition.channel_id)
        page = store.read_for_receiver(@receiver_id, limit)
        page.messages.map do |message|
          Support.fail!("unauthorized_message") unless message.channel_id == @definition.channel_id && message.originator_id == @definition.originator.agent_id && message.key_epoch <= @definition.key_epoch
          grant = store.key_grant(message.key_epoch, @receiver_id)
          Support.fail!("missing_key_grant") if grant.nil?
          begin
            channel_key = @key_provider.open_grant(message.key_epoch, grant)
          rescue StandardError
            Support.fail!("crypto_error")
          end
          Support.fail!("missing_key_grant") if channel_key.nil?
          begin
            payload = Crypto.message_verify(message, @definition.originator.public_key, channel_key)
          rescue StandardError
            Support.fail!("crypto_error")
          end
          Support.uuid_v7!(message.message_id, "invalid_message_id")
          prior = @delivered[message.message_id]
          Support.fail!("unauthorized_message") if !prior.nil? && prior != message.sequence
          @delivered[message.message_id] = message.sequence
          ReceivedMessage.new(message_id: message.message_id, sequence: message.sequence, timestamp_ns: message.timestamp_ns, content_type: message.content_type, payload: payload.b.dup.freeze).freeze
        end.freeze
      end

      def acknowledge(message_id)
        Support.uuid_v7!(message_id, "invalid_message_id")
        ChannelDefinitionStore.new(@backend).require_current(@definition)
        sequence = @delivered[message_id]
        Support.fail!("unknown_message_id") if sequence.nil?
        ChannelStore.new(@backend, @definition.channel_id).acknowledge(@receiver_id, sequence)
      end
    end

    module EndpointSupport
      module_function

      def active_definition(backend, channel_id)
        definition = ChannelDefinitionStore.new(backend).load(channel_id)
        Support.fail!("definition_not_found") if definition.nil?
        Support.fail!("channel_destroyed") if definition.lifecycle == "destroyed"
        definition
      end
    end
  end
end
