# frozen_string_literal: true

require "thread"

module CodingAdventures
  module ChiefOfStaffChannelStore
    Crypto = CodingAdventures::ChiefOfStaffChannelCrypto

    class StorageConflictError < StandardError; end

    class StorageRecord
      attr_reader :namespace, :key, :content_type, :revision

      def initialize(namespace:, key:, content_type:, body:, revision:)
        @namespace = namespace.dup.freeze
        @key = key.dup.freeze
        @content_type = content_type.dup.freeze
        @body = Support.bytes(body, "storage_error")
        @revision = revision.dup.freeze
        freeze
      end

      def body = @body.dup
    end

    class StoragePut
      attr_reader :namespace, :key, :content_type, :if_absent, :if_revision

      def initialize(namespace:, key:, content_type:, body:, if_absent: false, if_revision: nil)
        @namespace = namespace.dup.freeze
        @key = key.dup.freeze
        @content_type = content_type.dup.freeze
        @body = Support.bytes(body, "storage_error")
        @if_absent = if_absent
        @if_revision = if_revision&.dup&.freeze
        freeze
      end

      def body = @body.dup
    end

    class StoragePage
      attr_reader :records, :next_cursor

      def initialize(records:, next_cursor: nil)
        @records = records.freeze
        @next_cursor = next_cursor&.dup&.freeze
        freeze
      end
    end

    class MemoryChannelStorage
      def initialize
        @records = {}
        @revision = 0
        @mutex = Mutex.new
      end

      def initialize_backend = nil

      def get(namespace, key)
        @mutex.synchronize { clone_record(@records[[namespace, key]]) }
      end

      def put(value)
        @mutex.synchronize do
          raise ArgumentError, "exactly one storage condition is required" if value.if_absent == !value.if_revision.nil?

          map_key = [value.namespace, value.key]
          current = @records[map_key]
          raise StorageConflictError if value.if_absent ? !current.nil? : current.nil? || current.revision != value.if_revision

          @revision += 1
          record = StorageRecord.new(namespace: value.namespace, key: value.key, content_type: value.content_type, body: value.body, revision: "r#{@revision}")
          @records[map_key] = record
          clone_record(record)
        end
      end

      def list(namespace, prefix:, recursive:, page_size:, cursor: nil)
        raise ArgumentError, "invalid backend list options" unless recursive && page_size.is_a?(Integer) && page_size.positive?

        @mutex.synchronize do
          records = @records.values.select do |record|
            record.namespace == namespace && record.key.start_with?(prefix) && (cursor.nil? || record.key > cursor)
          end.sort_by(&:key)
          selected = records.first(page_size).map { |record| clone_record(record) }
          next_cursor = records.length > selected.length ? selected.last.key : nil
          StoragePage.new(records: selected, next_cursor: next_cursor)
        end
      end

      def corrupt(record)
        @mutex.synchronize { @records[[record.namespace, record.key]] = clone_record(record) }
      end

      private

      def clone_record(record)
        return nil if record.nil?

        StorageRecord.new(namespace: record.namespace, key: record.key, content_type: record.content_type, body: record.body, revision: record.revision)
      end
    end

    AppendRequest = Struct.new(:message_id, :timestamp_ns, :originator_id, :key_epoch, :content_type, keyword_init: true)
    MessagePage = Struct.new(:messages, :next_start, keyword_init: true)
    OpaqueKeyGrant = Struct.new(:channel_id, :key_epoch, :receiver_id, :body, keyword_init: true)

    class ChannelStore
      def initialize(backend, channel_id)
        Support.uuid_v7!(channel_id, "corrupt_record")
        @backend = backend
        @channel_id = Support.bytes(channel_id, "corrupt_record")
      end

      def initialize_store
        storage { @backend.initialize_backend }
        record = state_record
        return decode_state(record) unless record.nil?

        body = ChannelProfile.state_serialize(ChannelState.new(next_sequence: 0))
        begin
          decode_state(@backend.put(put_input(ChannelProfile.state_key(@channel_id), STATE_CONTENT_TYPE, body, if_absent: true)))
        rescue StorageConflictError
          state
        rescue StandardError => error
          raise storage_error(error)
        end
      end

      def state
        record = state_record
        Support.fail!("not_initialized") if record.nil?
        decode_state(record)
      end

      def reserve_append(request, plaintext)
        Support.uuid_v7!(request.message_id, "invalid_message_id")
        begin
          fields = Crypto::MessageFields.new(message_id: request.message_id, timestamp_ns: request.timestamp_ns, originator_id: request.originator_id, channel_id: @channel_id, sequence: 0, key_epoch: request.key_epoch, content_type: request.content_type)
          Crypto.validate_message_fields(fields)
        rescue StandardError
          Support.fail!("crypto_error")
        end
        MAX_STORE_CAS_ATTEMPTS.times do
          record = state_record
          Support.fail!("not_initialized") if record.nil?
          current = decode_state(record)
          Support.fail!("pending_append") unless current.pending_header.nil?
          Support.fail!("sequence_exhausted") if current.next_sequence == MAX_U64
          header = MessageHeader.new(
            message_id: request.message_id, timestamp_ns: request.timestamp_ns, originator_id: request.originator_id,
            channel_id: @channel_id, sequence: current.next_sequence, key_epoch: request.key_epoch,
            content_type: request.content_type, plaintext_hash: Support.digest(Support.bytes(plaintext, "crypto_error"))
          )
          body = ChannelProfile.state_serialize(ChannelState.new(next_sequence: current.next_sequence + 1, pending_header: header))
          begin
            @backend.put(put_input(ChannelProfile.state_key(@channel_id), STATE_CONTENT_TYPE, body, if_revision: record.revision))
            return header
          rescue StorageConflictError
            next
          rescue StandardError => error
            raise storage_error(error)
          end
        end
        Support.fail!("concurrent_update")
      end

      def commit_reserved(header, plaintext, channel_master_key, signing_secret_key)
        Support.fail!("pending_header_mismatch") unless header.channel_id == @channel_id
        current = state
        key = ChannelProfile.message_key(@channel_id, header.sequence)
        if current.pending_header.nil?
          record = storage { @backend.get(STORAGE_NAMESPACE, key) }
          Support.fail!("no_pending_append") if record.nil?
          stored = decode_message(record)
          Support.fail!("conflicting_record") unless message_matches?(stored, header)
          expected = create_message(header, plaintext, signing_secret_key, channel_master_key)
          Support.fail!("conflicting_record") unless Crypto.message_serialize(expected) == record.body
          return stored
        end
        Support.fail!("pending_header_mismatch") unless current.pending_header == header
        message = create_message(header, plaintext, signing_secret_key, channel_master_key)
        put_idempotent(key, MESSAGE_CONTENT_TYPE, Crypto.message_serialize(message))
        clear_pending(header)
        message
      end

      def append(request, plaintext, channel_master_key, signing_secret_key)
        commit_reserved(reserve_append(request, plaintext), plaintext, channel_master_key, signing_secret_key)
      end

      def abandon_pending
        MAX_STORE_CAS_ATTEMPTS.times do
          record = state_record
          Support.fail!("not_initialized") if record.nil?
          current = decode_state(record)
          return nil if current.pending_header.nil?

          body = ChannelProfile.state_serialize(ChannelState.new(next_sequence: current.next_sequence))
          begin
            @backend.put(put_input(ChannelProfile.state_key(@channel_id), STATE_CONTENT_TYPE, body, if_revision: record.revision))
            return current.pending_header
          rescue StorageConflictError
            next
          rescue StandardError => error
            raise storage_error(error)
          end
        end
        Support.fail!("concurrent_update")
      end

      def read_messages(start, page_size)
        Support.u64!(start, "corrupt_record")
        Support.fail!("invalid_page_size") unless page_size.is_a?(Integer) && page_size.positive?
        cursor = start.positive? ? ChannelProfile.message_key(@channel_id, start - 1) : nil
        page = storage { @backend.list(STORAGE_NAMESPACE, prefix: ChannelProfile.message_prefix(@channel_id), recursive: true, page_size: page_size, cursor: cursor) }
        messages = []
        page.records.each do |record|
          message = decode_message(record)
          Support.fail!("corrupt_record") unless message.channel_id == @channel_id && message.sequence >= start && record.key == ChannelProfile.message_key(@channel_id, message.sequence) && (messages.empty? || messages.last.sequence < message.sequence)
          messages << message
        end
        next_start = nil
        unless page.next_cursor.nil?
          Support.fail!("corrupt_record") if messages.empty? || messages.last.sequence == MAX_U64
          next_start = messages.last.sequence + 1
        end
        MessagePage.new(messages: messages.freeze, next_start: next_start)
      end

      def read_for_receiver(receiver_id, page_size) = read_messages(receiver_cursor(receiver_id), page_size)

      def receiver_cursor(receiver_id)
        Support.agent_id!(receiver_id, "invalid_receiver_id")
        record = storage { @backend.get(STORAGE_NAMESPACE, ChannelProfile.ack_key(@channel_id, receiver_id)) }
        return 0 if record.nil?

        require_content_type(record, ACK_CONTENT_TYPE)
        ChannelProfile.cursor_deserialize(record.body)
      end

      def acknowledge(receiver_id, acknowledged)
        Support.agent_id!(receiver_id, "invalid_receiver_id")
        Support.u64!(acknowledged, "acknowledgement_ahead")
        current_state = state
        Support.fail!("acknowledgement_ahead") if acknowledged >= current_state.next_sequence
        Support.fail!("acknowledgement_pending") if !current_state.pending_header.nil? && acknowledged >= current_state.pending_header.sequence
        Support.fail!("sequence_exhausted") if acknowledged == MAX_U64
        desired = acknowledged + 1
        key = ChannelProfile.ack_key(@channel_id, receiver_id)
        MAX_STORE_CAS_ATTEMPTS.times do
          record = storage { @backend.get(STORAGE_NAMESPACE, key) }
          if record.nil?
            begin
              @backend.put(put_input(key, ACK_CONTENT_TYPE, ChannelProfile.cursor_serialize(desired), if_absent: true))
              return desired
            rescue StorageConflictError
              next
            rescue StandardError => error
              raise storage_error(error)
            end
          end
          require_content_type(record, ACK_CONTENT_TYPE)
          current = ChannelProfile.cursor_deserialize(record.body)
          Support.fail!("acknowledgement_regression") if desired < current
          return current if desired == current

          begin
            @backend.put(put_input(key, ACK_CONTENT_TYPE, ChannelProfile.cursor_serialize(desired), if_revision: record.revision))
            return desired
          rescue StorageConflictError
            next
          rescue StandardError => error
            raise storage_error(error)
          end
        end
        Support.fail!("concurrent_update")
      end

      def save_key_grant(grant)
        Support.fail!("corrupt_record") unless grant.channel_id == @channel_id
        Support.agent_id!(grant.receiver_id, "invalid_receiver_id")
        put_idempotent(ChannelProfile.grant_key(@channel_id, grant.key_epoch, grant.receiver_id), GRANT_CONTENT_TYPE, grant.body)
      end

      def key_grant(key_epoch, receiver_id)
        Support.agent_id!(receiver_id, "invalid_receiver_id")
        record = storage { @backend.get(STORAGE_NAMESPACE, ChannelProfile.grant_key(@channel_id, key_epoch, receiver_id)) }
        return nil if record.nil?

        require_content_type(record, GRANT_CONTENT_TYPE)
        record.body
      end

      private

      def state_record = storage { @backend.get(STORAGE_NAMESPACE, ChannelProfile.state_key(@channel_id)) }
      def decode_state(record) = (require_content_type(record, STATE_CONTENT_TYPE); ChannelProfile.state_deserialize(record.body, @channel_id))

      def decode_message(record)
        require_content_type(record, MESSAGE_CONTENT_TYPE)
        Crypto.message_deserialize(record.body)
      rescue Crypto::MessageProfileError
        Support.fail!("wire_error")
      end

      def create_message(header, plaintext, signing_secret_key, channel_master_key)
        plain = Support.bytes(plaintext, "crypto_error")
        Support.fail!("crypto_error") unless Support.digest(plain) == header.plaintext_hash
        values = header.__values
        fields = Crypto::MessageFields.new(message_id: values[0], timestamp_ns: values[1], originator_id: values[2], channel_id: values[3], sequence: values[4], key_epoch: values[5], content_type: values[6])
        Crypto.message_create(fields, plain, signing_secret_key, channel_master_key)
      rescue ChannelProfileError
        raise
      rescue StandardError
        Support.fail!("crypto_error")
      end

      def message_matches?(message, header)
        values = header.__values
        [message.message_id, message.timestamp_ns, message.originator_id, message.channel_id, message.sequence, message.key_epoch, message.content_type, message.plaintext_hash] == values
      end

      def clear_pending(expected)
        MAX_STORE_CAS_ATTEMPTS.times do
          record = state_record
          Support.fail!("not_initialized") if record.nil?
          current = decode_state(record)
          return if current.pending_header.nil?
          Support.fail!("pending_header_mismatch") unless current.pending_header == expected

          body = ChannelProfile.state_serialize(ChannelState.new(next_sequence: current.next_sequence))
          begin
            @backend.put(put_input(ChannelProfile.state_key(@channel_id), STATE_CONTENT_TYPE, body, if_revision: record.revision))
            return
          rescue StorageConflictError
            next
          rescue StandardError => error
            raise storage_error(error)
          end
        end
        Support.fail!("concurrent_update")
      end

      def put_idempotent(key, content_type, body)
        @backend.put(put_input(key, content_type, body, if_absent: true))
      rescue StorageConflictError
        current = storage { @backend.get(STORAGE_NAMESPACE, key) }
        Support.fail!("conflicting_record") if current.nil? || current.content_type != content_type || current.body != body
      rescue StandardError => error
        raise storage_error(error)
      end

      def require_content_type(record, expected)
        Support.fail!("corrupt_record") unless record.content_type == expected
      end

      def put_input(key, content_type, body, if_absent: false, if_revision: nil)
        StoragePut.new(namespace: STORAGE_NAMESPACE, key: key, content_type: content_type, body: body, if_absent: if_absent, if_revision: if_revision)
      end

      def storage
        yield
      rescue ChannelProfileError, StorageConflictError
        raise
      rescue StandardError => error
        raise storage_error(error)
      end

      def storage_error(error) = error.is_a?(ChannelProfileError) ? error : ChannelProfileError.new("storage_error")
    end
  end
end
