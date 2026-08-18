# frozen_string_literal: true

module CodingAdventures
  module ChiefOfStaffChannelEpochActivation
    # Asks to publish at whatever epoch is currently active. +key_epoch+ is
    # optional: leave it nil to accept the active epoch, or set it to assert an
    # expectation that is checked before any encryption.
    ActiveEpochAppendRequest = Struct.new(
      :message_id, :timestamp_ns, :originator_id, :content_type, :key_epoch,
      keyword_init: true
    )

    # Pairs a durable D18H reservation with the redacted handle for the exact
    # epoch key it was reserved against.
    EpochReservation = Struct.new(:header, :key_handle, keyword_init: true)

    # D18T coordinator over injected public storage and secret custody.
    class EpochActivationStore
      # Open a production coordinator. Refuses custody that reports itself
      # non-durable, so a test double cannot be wired into a real channel.
      def self.open(backend, custody, channel_id)
        Support.fail!("custody_error") unless custody.respond_to?(:durable?) && custody.durable?
        Support.backend { backend.initialize_backend }
        new(backend, custody, channel_id)
      end

      # Open a coordinator that accepts non-durable custody, for tests only.
      def self.open_for_testing(backend, custody, channel_id)
        Support.backend { backend.initialize_backend }
        new(backend, custody, channel_id)
      end

      def initialize(backend, custody, channel_id)
        @backend = backend
        @custody = custody
        @channel_id = channel_id.b.dup.freeze
      end

      # Create a D18T-aware channel, custody before any D18S state.
      #
      # The definition is settled *before* the custody import. Custody slots are
      # keyed by (channel_id, epoch) and the first writer wins permanently, so
      # importing first would let a caller presenting a mismatched definition
      # claim an unclaimed slot and then fail -- leaving the legitimate import
      # to hit conflicting_active_key forever. Fail closed, but permanently
      # wedged. D18T only requires custody before *state*.
      #
      # The remaining ordering is still the invariant it always was: a crash
      # between the import and the state write leaves no state record at all, so
      # a retry re-imports idempotently. The reverse order would publish a
      # channel whose active epoch has no resolvable key -- unrecoverable,
      # because D18T forbids inventing one.
      def create_epoch_channel(definition, initial_cmk)
        consumed = false
        begin
          Support.fail!("invalid_plan") unless definition.channel_id == @channel_id &&
            definition.lifecycle == "active"
          definitions = Store::ChannelDefinitionStore.new(@backend)
          begin
            existing = definitions.load(@channel_id)
            if existing.nil?
              definitions.create(definition)
            elsif existing.lifecycle == "destroyed"
              Support.fail!("channel_destroyed")
            elsif existing != definition
              Support.fail!("invalid_plan")
            end
          rescue EpochActivationError
            raise
          rescue Store::ChannelProfileError => error
            raise Support.translate_store_error(error)
          rescue StandardError
            Support.fail!("storage_error")
          end
          consumed = true
          import_initial_key(definition.key_epoch, initial_cmk)
        ensure
          # import_initial_key owns the erase once it runs; every earlier exit
          # must still erase, so an unused secret never outlives the call.
          initial_cmk.destroy unless consumed
        end
        migrate_epoch_state(definition)
      end

      # Bring a channel to D18S version 2, whether new or created under D18P
      # version 1.
      #
      # Publishing version 2 is the rolling-upgrade boundary: a version 1
      # process rejects the record rather than misreading it, so operators must
      # deploy D18T-aware readers and writers before migrating. Nothing here
      # ever clears a pending publish, resets a sequence, or generates key
      # material.
      def migrate_epoch_state(definition, current_cmk = nil)
        owned = current_cmk
        begin
          require_definition(definition, false)
          MAX_EPOCH_CAS_ATTEMPTS.times do
            record = state_record
            if !record.nil? && record.content_type == EPOCH_STATE_CONTENT_TYPE
              state = decode_v2_state_record(record)
              # An existing version 2 state is an idempotent success only once
              # custody proves its active epoch is still resolvable. Its epoch is
              # never reset from the immutable definition.
              Support.fail!("active_key_missing") if resolve_handle(state.active_epoch).nil?
              return state
            end
            ensure_initial_key(definition.key_epoch, owned)
            owned = nil

            state = build_migration_state(definition, record)
            begin
              stored = @backend.put(Support.public_put(
                Store::ChannelProfile.state_key(@channel_id), EPOCH_STATE_CONTENT_TYPE,
                Wire.epoch_state_serialize(state),
                if_absent: record.nil?, if_revision: record&.revision
              ))
              return decode_v2_state_record(stored)
            rescue Store::StorageConflictError
              next
            rescue EpochActivationError
              raise
            rescue StandardError
              Support.fail!("storage_error")
            end
          end
          Support.fail!("concurrent_update")
        ensure
          # Every exit erases the caller's key, including the common
          # steady-state path where the channel is already at version 2 and the
          # key was never needed. An unused secret is still a secret.
          owned&.destroy
        end
      end

      # Load the canonical D18S version 2 state.
      def state
        record = state_record
        Support.fail!("not_initialized") if record.nil?
        decode_v2_state_record(record)
      end

      # Run the full prepare-and-replay protocol for one candidate.
      #
      # Custody comes first, before any public write, because it is the only
      # operation that both selects a winner and makes everything needed for
      # replay durable in one atomic step. A crash before it leaves no
      # candidate; a crash after it is fully recoverable from custody plus the
      # public store.
      def prepare_rotation(definition, target_roster, rotation)
        begin
          require_definition(definition, false)
          current = state
          Support.fail!("pending_append") unless current.pending_header.nil?
          Support.fail!("epoch_exhausted") if current.active_epoch == MAX_U64
          expected = current.active_epoch + 1
          Support.fail!("unexpected_epoch") unless rotation.new_epoch == expected
        rescue StandardError
          rotation.destroy
          raise
        end

        prepared = EpochActivationStore.prepare_rotation_candidate(
          definition, current.active_epoch, target_roster, rotation
        )
        selection =
          begin
            @custody.prepare_if_absent(prepared)
          rescue EpochActivationError
            raise
          rescue StandardError
            Support.fail!("custody_error")
          ensure
            prepared.destroy
          end
        Support.fail!("conflicting_preparation") if selection == CUSTODY_CONFLICT
        replay_preparation(definition, expected)
        selection == CUSTODY_SELECTED ? "prepared" : "idempotent"
      end

      # Replay the durable bundle after a crash. Never generates a CMK, reseals
      # a grant, accepts replacement bytes, or picks a different candidate --
      # recovery finishes the selected plan or fails.
      def recover_preparation(definition, new_epoch)
        require_definition(definition, false)
        active = state.active_epoch
        Support.fail!("decreasing_epoch") if new_epoch < active
        unless new_epoch == active
          Support.fail!("epoch_exhausted") if active == MAX_U64
          Support.fail!("unexpected_epoch") unless new_epoch == active + 1
        end
        replay_preparation(definition, new_epoch)
        "idempotent"
      end

      # Commit the epoch transition with a bounded CAS.
      #
      # Every precondition is re-checked inside the retry loop, because a CAS
      # conflict means somebody else changed the state and any fact read before
      # it may now be stale.
      def activate_prepared_epoch(definition, new_epoch)
        require_definition(definition, false)
        prepared = custody_call { @custody.load_preparation(@channel_id, new_epoch) }
        Support.fail!("preparation_missing") if prepared.nil?

        MAX_EPOCH_CAS_ATTEMPTS.times do
          require_definition(definition, false)
          record = state_record
          Support.fail!("not_initialized") if record.nil?
          current = decode_v2_state_record(record)
          active = current.active_epoch

          if active == new_epoch
            validate_and_replay(definition, prepared)
            require_handle(new_epoch)
            return "idempotent"
          end
          Support.fail!("decreasing_epoch") if active > new_epoch
          Support.fail!("epoch_exhausted") if active == MAX_U64
          unless active + 1 == new_epoch && prepared.base_epoch == active && prepared.new_epoch == new_epoch
            Support.fail!("unexpected_epoch")
          end

          validate_and_replay(definition, prepared)
          require_handle(new_epoch)
          # Checked last, immediately before the CAS: a reservation that landed
          # during replay must still block this activation.
          Support.fail!("pending_append") unless current.pending_header.nil?

          updated = current.with_active_epoch(@channel_id, new_epoch)
          begin
            stored = @backend.put(Support.public_put(
              Store::ChannelProfile.state_key(@channel_id), EPOCH_STATE_CONTENT_TYPE,
              Wire.epoch_state_serialize(updated), if_revision: record.revision
            ))
            Support.fail!("corrupt_record") unless decode_v2_state_record(stored) == updated
            return "activated"
          rescue Store::StorageConflictError
            next
          rescue EpochActivationError
            raise
          rescue StandardError
            Support.fail!("storage_error")
          end
        end
        Support.fail!("concurrent_update")
      end

      # Build a D18H reservation bound to the current active epoch and its
      # resolved key handle.
      #
      # This is the publication half of the shared CAS. If activation wins the
      # race, this loop's put conflicts, reloads, and rebuilds against E+1. If
      # this wins, activation observes the pending header and reports
      # pending_append. Encryption never falls back to an old epoch and never
      # invents a missing key.
      def reserve_publish_using_active_epoch(definition, request, plaintext)
        require_definition(definition, false)
        Support.fail!("invalid_plan") unless request.originator_id == definition.originator.agent_id

        MAX_EPOCH_CAS_ATTEMPTS.times do
          record = state_record
          Support.fail!("not_initialized") if record.nil?
          current = decode_v2_state_record(record)
          if !request.key_epoch.nil? && request.key_epoch != current.active_epoch
            Support.fail!("unactivated_epoch")
          end
          handle = require_handle(current.active_epoch)
          Support.fail!("pending_append") unless current.pending_header.nil?
          Support.fail!("crypto_error") if current.next_sequence == MAX_U64

          header =
            begin
              Store::MessageHeader.new(
                message_id: request.message_id, timestamp_ns: request.timestamp_ns,
                originator_id: request.originator_id, channel_id: @channel_id,
                sequence: current.next_sequence, key_epoch: current.active_epoch,
                content_type: request.content_type,
                plaintext_hash: CodingAdventures::Sha256.sha256(plaintext.b)
              )
            rescue StandardError
              Support.fail!("crypto_error")
            end

          updated = current.with_pending(@channel_id, current.next_sequence + 1, header)
          begin
            @backend.put(Support.public_put(
              Store::ChannelProfile.state_key(@channel_id), EPOCH_STATE_CONTENT_TYPE,
              Wire.epoch_state_serialize(updated), if_revision: record.revision
            ))
            return EpochReservation.new(header: header, key_handle: handle)
          rescue Store::StorageConflictError
            next
          rescue EpochActivationError
            raise
          rescue StandardError
            Support.fail!("storage_error")
          end
        end
        Support.fail!("concurrent_update")
      end

      # Clear an in-flight reservation without publishing, releasing the CAS so
      # a blocked activation can proceed. The sequence is not rewound -- D18P
      # sequences are append-only, so the abandoned slot simply stays empty.
      def abandon_pending
        MAX_EPOCH_CAS_ATTEMPTS.times do
          record = state_record
          Support.fail!("not_initialized") if record.nil?
          current = decode_v2_state_record(record)
          pending = current.pending_header
          return nil if pending.nil?

          updated = current.with_pending(@channel_id, current.next_sequence)
          begin
            @backend.put(Support.public_put(
              Store::ChannelProfile.state_key(@channel_id), EPOCH_STATE_CONTENT_TYPE,
              Wire.epoch_state_serialize(updated), if_revision: record.revision
            ))
            return pending
          rescue Store::StorageConflictError
            next
          rescue EpochActivationError
            raise
          rescue StandardError
            Support.fail!("storage_error")
          end
        end
        Support.fail!("concurrent_update")
      end

      # Load the immutable public plan for an epoch, or nil.
      def activation_plan(new_epoch)
        key = Wire.activation_plan_record_key(@channel_id, new_epoch)
        record = Support.backend { @backend.get(Store::STORAGE_NAMESPACE, key) }
        return nil if record.nil?

        require_envelope(record, key, ACTIVATION_PLAN_CONTENT_TYPE)
        plan = Wire.activation_plan_deserialize(record.body)
        unless plan.channel_id == @channel_id && plan.new_epoch == new_epoch
          Support.fail!("corrupt_record")
        end
        plan
      end

      # Wipe custody for a destroyed channel while leaving every public plan,
      # grant, and message exactly where it is. D18T revocation is prospective:
      # it stops future access, it does not rewrite history.
      def apply_destruction(definition)
        require_definition(definition, true)
        custody_call { @custody.destroy_channel(@channel_id) }
        nil
      end

      # Build one pure custody candidate from a trusted D18Q plan, consuming
      # that rotation.
      #
      # Two orderings matter here and they are different. D18Q grants are
      # ordered by *raw* receiver ID, because that is the order D18Q produces
      # and the order the grants must be replayed in. The public D18T plan
      # entries are sorted by receiver ID *hash*, because the plan must not
      # reveal the raw roster. The two orders are unrelated, so entries are
      # derived from the D18Q order and ActivationPlan re-sorts for the wire.
      def self.prepare_rotation_candidate(definition, base_epoch, target_roster, rotation)
        grants = rotation.grants
        roster = target_roster.to_a
        unless roster.length.between?(1, MAX_PLAN_RECEIVERS) && roster.length == grants.length
          Support.fail!("invalid_plan")
        end
        ordered = roster.sort_by(&:agent_id)
        Support.fail!("invalid_plan") if ordered.map(&:agent_id).uniq.length != ordered.length

        ordered.zip(grants).each do |receiver, grant|
          # The epoch check lives here, not in verify_grant_signature: D18Q's
          # signature covers the epoch but the verifier deliberately takes no
          # expected epoch, so a validly signed grant for the wrong epoch would
          # otherwise pass. D18T step 5 owns this comparison.
          unless receiver.agent_id == grant.receiver_id && grant.key_epoch == rotation.new_epoch
            Support.fail!("invalid_plan")
          end
          Support.verify_grant_public(definition, grant, receiver.agent_id)
        end
        Support.fail!("epoch_exhausted") if base_epoch == MAX_U64
        Support.fail!("unexpected_epoch") unless rotation.new_epoch == base_epoch + 1

        grant_bytes = grants.map { |grant| Support.serialize_grant(grant) }
        entries = grants.zip(grant_bytes).map do |grant, data|
          ActivationPlanEntry.new(
            receiver_id_hash: CodingAdventures::Sha256.sha256(grant.receiver_id),
            grant_hash: CodingAdventures::Sha256.sha256(data)
          )
        end
        plan = ActivationPlan.new(channel_id: definition.channel_id, base_epoch: base_epoch,
          new_epoch: rotation.new_epoch, receivers: entries)
        public_preparation = PublicPreparation.new(
          channel_id: definition.channel_id, base_epoch: base_epoch,
          new_epoch: rotation.new_epoch, plan_bytes: Wire.activation_plan_serialize(plan),
          grants: grant_bytes
        )
        cmk = rotation.new_cmk
        begin
          PreparedEpoch.new(public_preparation, cmk)
        ensure
          cmk.destroy
        end
      ensure
        rotation.destroy
      end

      private

      def build_migration_state(definition, record)
        if record.nil?
          return EpochState.new(channel_id: @channel_id, active_epoch: definition.key_epoch,
            next_sequence: 0)
        end

        require_envelope(record, Store::ChannelProfile.state_key(@channel_id),
          Store::STATE_CONTENT_TYPE)
        prior =
          begin
            Store::ChannelProfile.state_deserialize(record.body, @channel_id)
          rescue StandardError
            Support.fail!("corrupt_record")
          end
        if !prior.pending_header.nil? && prior.pending_header.key_epoch != definition.key_epoch
          Support.fail!("corrupt_record")
        end
        EpochState.new(channel_id: @channel_id, active_epoch: definition.key_epoch,
          next_sequence: prior.next_sequence, pending_header: prior.pending_header)
      end

      def ensure_initial_key(epoch, current_cmk)
        unless resolve_handle(epoch).nil?
          current_cmk&.destroy
          return nil
        end
        Support.fail!("active_key_missing") if current_cmk.nil?
        import_initial_key(epoch, current_cmk)
      end

      def import_initial_key(epoch, current_cmk)
        Support.fail!("active_key_missing") if current_cmk.nil?
        selection =
          begin
            @custody.import_active_if_absent(@channel_id, epoch, current_cmk)
          rescue EpochActivationError
            raise
          rescue StandardError
            Support.fail!("custody_error")
          ensure
            current_cmk.destroy
          end
        Support.fail!("conflicting_active_key") if selection == CUSTODY_CONFLICT
        nil
      end

      def replay_preparation(definition, new_epoch)
        prepared = custody_call { @custody.load_preparation(@channel_id, new_epoch) }
        Support.fail!("preparation_missing") if prepared.nil?
        validate_and_replay(definition, prepared)
      end

      # Replay phases 3 through 6: re-validate the durable bundle, write the
      # plan and every grant with create-if-absent, then reload and compare.
      # Byte-identical writes are idempotent; different bytes at the same key
      # are a stable conflict and are never replaced.
      def validate_and_replay(definition, prepared)
        plan = Support.validate_public_preparation(definition, prepared)
        put_immutable(Wire.activation_plan_record_key(@channel_id, plan.new_epoch),
          ACTIVATION_PLAN_CONTENT_TYPE, prepared.plan_bytes, "conflicting_plan")
        prepared.grants.each do |data|
          grant = Support.deserialize_grant(data)
          put_immutable(Store::ChannelProfile.grant_key(@channel_id, grant.key_epoch, grant.receiver_id),
            Store::GRANT_CONTENT_TYPE, data, "conflicting_grant")
        end

        stored = activation_plan(plan.new_epoch)
        Support.fail!("corrupt_record") if stored.nil? || stored != plan

        # Phase 6 reloads every grant too. This is invariant 3, "all grants
        # before visibility" -- not paranoia about our own writes. The record a
        # put echoes back sits on the same trust boundary as the write itself,
        # so against a write-behind or eventually-consistent backend an echoed
        # success does not prove the grant is retrievable. Activation may only
        # advance the epoch once every receiver's grant can actually be read.
        prepared.grants.each do |data|
          grant = Support.deserialize_grant(data)
          key = Store::ChannelProfile.grant_key(@channel_id, grant.key_epoch, grant.receiver_id)
          record = Support.backend { @backend.get(Store::STORAGE_NAMESPACE, key) }
          Support.fail!("corrupt_record") if record.nil?
          require_envelope(record, key, Store::GRANT_CONTENT_TYPE)
          # corrupt_record, not conflicting_grant: put_immutable already reports
          # a genuine slot conflict, so reaching here means the backend returned
          # something other than what it acknowledged writing.
          Support.fail!("corrupt_record") unless record.body == data
        end
        nil
      end

      def require_handle(epoch)
        handle = resolve_handle(epoch)
        Support.fail!("active_key_missing") if handle.nil?
        handle
      end

      def resolve_handle(epoch) = custody_call { @custody.resolve_handle(@channel_id, epoch) }

      def custody_call
        yield
      rescue EpochActivationError
        raise
      rescue StandardError
        Support.fail!("custody_error")
      end

      def require_definition(expected, require_destroyed)
        Support.fail!("invalid_plan") unless expected.channel_id == @channel_id
        actual =
          begin
            Store::ChannelDefinitionStore.new(@backend).load(@channel_id)
          rescue Store::ChannelProfileError => error
            raise Support.translate_store_error(error)
          rescue StandardError
            Support.fail!("storage_error")
          end
        Support.fail!("not_initialized") if actual.nil?
        Support.fail!("invalid_plan") unless actual == expected
        if require_destroyed
          Support.fail!("invalid_plan") unless actual.lifecycle == "destroyed"
        elsif actual.lifecycle == "destroyed"
          Support.fail!("channel_destroyed")
        end
        nil
      end

      def state_record
        Support.backend do
          @backend.get(Store::STORAGE_NAMESPACE, Store::ChannelProfile.state_key(@channel_id))
        end
      end

      def decode_v2_state_record(record)
        require_envelope(record, Store::ChannelProfile.state_key(@channel_id),
          EPOCH_STATE_CONTENT_TYPE)
        Wire.epoch_state_deserialize(record.body, @channel_id)
      end

      def require_envelope(record, key, content_type)
        unless record.namespace == Store::STORAGE_NAMESPACE && record.key == key &&
            record.content_type == content_type
          Support.fail!("corrupt_record")
        end
        nil
      end

      def put_immutable(key, content_type, body, conflict_code)
        record = @backend.put(Support.public_put(key, content_type, body, if_absent: true))
        require_envelope(record, key, content_type)
        Support.fail!("corrupt_record") unless record.body == body
        nil
      rescue Store::StorageConflictError
        existing = Support.backend { @backend.get(Store::STORAGE_NAMESPACE, key) }
        Support.fail!("corrupt_record") if existing.nil?
        require_envelope(existing, key, content_type)
        Support.fail!(conflict_code) unless existing.body == body
        nil
      rescue EpochActivationError
        raise
      rescue StandardError
        Support.fail!("storage_error")
      end
    end

    # Shared helpers for the D18T coordinator.
    module Support
      module_function

      def fail!(code) = raise(EpochActivationError, code)

      def backend
        yield
      rescue EpochActivationError
        raise
      rescue StandardError
        fail!("storage_error")
      end

      def crypto
        yield
      rescue EpochActivationError
        raise
      rescue StandardError
        fail!("crypto_error")
      end

      def public_put(key, content_type, body, if_absent: false, if_revision: nil)
        Store::StoragePut.new(namespace: Store::STORAGE_NAMESPACE, key: key,
          content_type: content_type, body: body.b, if_absent: if_absent,
          if_revision: if_revision)
      end

      def serialize_grant(grant) = crypto { Crypto.grant_serialize(grant) }

      def deserialize_grant(data) = crypto { Crypto.grant_deserialize(data) }

      def verify_grant_public(definition, grant, receiver_id)
        crypto do
          Crypto.verify_grant_signature(grant, definition.originator.agent_id, receiver_id,
            definition.channel_id, definition.originator.public_key)
        end
      end

      # Re-derive the entire plan from the durable grants and require it to
      # equal the stored plan bytes.
      #
      # This runs on every replay, including recovery after a crash, and is
      # deliberately not a shortcut comparison of the plan commitment.
      # Recomputing from the grants is what makes a tampered custody bundle
      # detectable.
      def validate_public_preparation(definition, prepared)
        unless prepared.channel_id == definition.channel_id &&
            prepared.base_epoch != MAX_U64 &&
            prepared.new_epoch == prepared.base_epoch + 1 &&
            prepared.grants.length.between?(1, MAX_PLAN_RECEIVERS)
          fail!("invalid_plan")
        end
        plan =
          begin
            Wire.activation_plan_deserialize(prepared.plan_bytes)
          rescue EpochActivationError
            fail!("corrupt_record")
          end
        unless plan.channel_id == prepared.channel_id && plan.base_epoch == prepared.base_epoch &&
            plan.new_epoch == prepared.new_epoch && plan.receivers.length == prepared.grants.length
          fail!("invalid_plan")
        end

        previous = nil
        entries = prepared.grants.map do |data|
          grant = deserialize_grant(data)
          unless grant.channel_id == prepared.channel_id &&
              grant.key_epoch == prepared.new_epoch &&
              (previous.nil? || previous < grant.receiver_id)
            fail!("invalid_plan")
          end
          verify_grant_public(definition, grant, grant.receiver_id)
          previous = grant.receiver_id
          ActivationPlanEntry.new(
            receiver_id_hash: CodingAdventures::Sha256.sha256(grant.receiver_id),
            grant_hash: CodingAdventures::Sha256.sha256(data)
          )
        end
        expected = ActivationPlan.new(channel_id: prepared.channel_id,
          base_epoch: prepared.base_epoch, new_epoch: prepared.new_epoch, receivers: entries)
        fail!("invalid_plan") unless plan == expected
        plan
      end

      # Map D18P codes onto the D18T roster. Anything without a D18T meaning
      # becomes storage_error rather than leaking a foreign code.
      def translate_store_error(error)
        case error.code
        when "channel_destroyed" then EpochActivationError.new("channel_destroyed")
        when "conflicting_definition", "definition_changed" then EpochActivationError.new("invalid_plan")
        when "corrupt_definition", "corrupt_record" then EpochActivationError.new("corrupt_record")
        when "definition_not_found", "not_initialized" then EpochActivationError.new("not_initialized")
        else EpochActivationError.new("storage_error")
        end
      end
    end

    # Report Ruby's honest erasure capability, inherited from D18Q rather than
    # claimed independently. The Rust reference reports "guaranteed";
    # overstating Ruby's position to match it would be the dishonest kind of
    # portability.
    def self.secret_erasure_capability = Crypto.secret_erasure_capability
  end
end
