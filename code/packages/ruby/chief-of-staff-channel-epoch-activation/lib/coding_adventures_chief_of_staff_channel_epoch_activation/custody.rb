# frozen_string_literal: true

module CodingAdventures
  module ChiefOfStaffChannelEpochActivation
    # Three-valued result of an atomic custody claim.
    #
    # Three values, not two, and the distinction is the heart of D18T.
    # +selected+ and +idempotent+ are both successes but mean different things:
    # the first says *you* won the slot, the second says you are retrying
    # something already won with byte-identical inputs. +conflict+ means
    # somebody else owns it and you must not proceed -- notably, you may not
    # look at what they stored.
    CUSTODY_SELECTED = "selected"
    CUSTODY_IDEMPOTENT = "idempotent"
    CUSTODY_CONFLICT = "conflict"

    # Secret-free failure from the injected custody backend.
    class CustodyError < EpochActivationError
      def initialize = super("custody_error")
    end

    # Opaque, redacted reference to one retained epoch key.
    #
    # Carries no key bytes and no reversible locator -- only the channel and
    # epoch, both already public. Resolving a handle to an actual CMK is the
    # sole privilege of the originator encryption boundary, via #with_key.
    class EpochKeyHandle
      attr_reader :channel_id, :epoch

      def initialize(channel_id:, epoch:)
        @channel_id = channel_id.b.dup.freeze
        @epoch = epoch
        freeze
      end

      def ==(other)
        other.is_a?(EpochKeyHandle) && other.channel_id == @channel_id && other.epoch == @epoch
      end
      alias_method :eql? , :==

      # Redacts. Handles reach logs and debug output; one that printed its
      # channel and epoch would be harmless today and a liability the moment
      # somebody adds a field to this class.
      def inspect = "EpochKeyHandle(<redacted>)"
      alias_method :to_s, :inspect
    end

    # Exact secret-free recovery bundle retained beside a prepared CMK. After
    # any crash this is enough to replay every public write without
    # regenerating a single byte.
    class PublicPreparation
      attr_reader :channel_id, :base_epoch, :new_epoch, :plan_bytes, :grants

      def initialize(channel_id:, base_epoch:, new_epoch:, plan_bytes:, grants:)
        @channel_id = channel_id.b.dup.freeze
        @base_epoch = base_epoch
        @new_epoch = new_epoch
        @plan_bytes = plan_bytes.b.dup.freeze
        @grants = grants.map { |grant| grant.b.dup.freeze }.freeze
        freeze
      end

      def ==(other)
        other.is_a?(PublicPreparation) &&
          other.channel_id == @channel_id && other.base_epoch == @base_epoch &&
          other.new_epoch == @new_epoch && other.plan_bytes == @plan_bytes &&
          other.grants == @grants
      end
      alias_method :eql?, :==
    end

    # One indivisible candidate offered to custody: the public recovery bundle
    # *and* the secret CMK, together.
    #
    # "Indivisible" is the whole point. Custody must never store the plan
    # without the key or the key without the plan -- either half alone leaves a
    # channel that cannot recover. That is why this is a single object with a
    # single custody entry point rather than two calls a caller could
    # interleave.
    class PreparedEpoch
      attr_reader :public_preparation

      def initialize(public_preparation, cmk)
        @public_preparation = public_preparation
        @cmk = Crypto::ChannelMasterKey.from_bytes(cmk.bytes)
      end

      def clone_cmk = Crypto::ChannelMasterKey.from_bytes(@cmk.bytes)

      def destroy = @cmk.destroy

      def inspect = "PreparedEpoch(<redacted>)"
      alias_method :to_s, :inspect
    end

    # Deterministic, explicitly non-durable custody for conformance tests.
    #
    # #durable? returns false, so EpochActivationStore.open refuses it and only
    # .open_for_testing will accept it. A production deployment therefore
    # cannot wire a test double in by accident.
    class InMemoryKeyCustody
      def initialize
        @keys = {}
        @preparations = {}
      end

      def durable? = false

      # Claim an already-active epoch key. Used only at channel creation and at
      # version 1 migration -- never to invent a key.
      def import_active_if_absent(channel_id, epoch, cmk)
        slot = [channel_id.b, epoch]
        current = @keys[slot]
        if current.nil?
          @keys[slot] = Crypto::ChannelMasterKey.from_bytes(cmk.bytes)
          return CUSTODY_SELECTED
        end
        # Deliberately does not reveal *how* the stored secret differs.
        Custody.same_cmk?(current, cmk) ? CUSTODY_IDEMPOTENT : CUSTODY_CONFLICT
      end

      def resolve_handle(channel_id, epoch)
        return nil unless @keys.key?([channel_id.b, epoch])

        EpochKeyHandle.new(channel_id: channel_id, epoch: epoch)
      end

      # Atomically claim the epoch slot for one complete bundle.
      #
      # Both halves are checked before either is written, and a partially
      # present slot (key without bundle, or bundle without key) is a conflict
      # rather than something to repair -- a half-written slot means an
      # invariant already broke, and guessing at the missing half is exactly
      # the fallback D18T forbids.
      def prepare_if_absent(prepared)
        public_preparation = prepared.public_preparation
        slot = [public_preparation.channel_id, public_preparation.new_epoch]
        current_public = @preparations[slot]
        current_cmk = @keys[slot]

        if current_public.nil? && current_cmk.nil?
          @preparations[slot] = public_preparation
          @keys[slot] = prepared.clone_cmk
          return CUSTODY_SELECTED
        end
        return CUSTODY_CONFLICT if current_public.nil? || current_cmk.nil?
        return CUSTODY_CONFLICT unless current_public == public_preparation

        candidate = prepared.clone_cmk
        begin
          Custody.same_cmk?(current_cmk, candidate) ? CUSTODY_IDEMPOTENT : CUSTODY_CONFLICT
        ensure
          candidate.destroy
        end
      end

      def load_preparation(channel_id, new_epoch) = @preparations[[channel_id.b, new_epoch]]

      # Lend a transient CMK for exactly one operation and destroy it after.
      def with_key(handle)
        cmk = @keys[[handle.channel_id, handle.epoch]]
        raise CustodyError if cmk.nil?

        transient = Crypto::ChannelMasterKey.from_bytes(cmk.bytes)
        begin
          yield transient
        ensure
          transient.destroy
        end
      end

      # Erase every retained secret for one channel. Public history is
      # untouched -- that is the store's business, and D18T keeps it
      # append-only.
      def destroy_channel(channel_id)
        channel = channel_id.b
        @keys.keys.each do |slot|
          next unless slot.first == channel

          @keys.delete(slot).destroy
        end
        @preparations.keys.each { |slot| @preparations.delete(slot) if slot.first == channel }
        nil
      end

      def retained_key_count = @keys.length
    end

    # Custody helpers shared by implementations.
    module Custody
      module_function

      # Constant-time comparison. Both operands are secrets, so a
      # length-or-content early exit would leak information about the stored key
      # to a caller who controls the candidate.
      #
      # Delegates to the repository's audited primitive rather than
      # reimplementing the loop here -- which is what the other five D18T ports
      # do, and what the D18T spec asks for ("MUST use the platform's
      # constant-time primitive where one exists").
      def same_cmk?(left, right)
        CodingAdventures::CtCompare.ct_eq_fixed(left.bytes, right.bytes)
      end
    end
  end
end
