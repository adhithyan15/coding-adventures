# frozen_string_literal: true

module CodingAdventures
  # Portable D18T durable channel epoch-activation profile.
  #
  # == The problem D18T solves
  #
  # D18P makes a channel's messages, grants, cursors, and sequence reservations
  # durable. D18Q can mint a fresh channel master key (CMK) for epoch E+1 and
  # seal one grant per authorized receiver. Neither can make E+1 *current*.
  #
  # That gap is not cosmetic. The obvious implementation -- write a "current
  # epoch" record, then publish with the new key -- loses data two ways:
  #
  #   crash here  ->  the new epoch is visible, but its CMK was never durably
  #                   stored, so nothing published afterwards can be decrypted
  #   crash here  ->  a concurrent publisher reserved a slot at epoch E while
  #                   activation committed E+1; whose key is right?
  #
  # D18T defines the missing transaction without assuming the storage backend
  # offers multi-record transactions. Three authorities cooperate:
  #
  #   D18P channel store   public records and the publish-reservation CAS
  #   injected custody     prepared CMKs and their recovery bundles
  #   D18Q                 grant creation, parsing, and verification
  #
  # == The one idea worth remembering
  #
  # The active epoch lives in the *same versioned record* as the pending publish
  # reservation. That is deliberate and load-bearing. A separate mutable "epoch
  # head" record would not be conforming, because two independent
  # compare-and-swap operations cannot exclude each other: a publisher could
  # reserve a slot against the old epoch in the window between activation
  # reading the head and writing it. One record means one revision means one
  # CAS, and exactly one of {publish, activate} wins.
  module ChiefOfStaffChannelEpochActivation
    EPOCH_STATE_CONTENT_TYPE = "application/vnd.coding-adventures.chief-channel-state-v2"
    ACTIVATION_PLAN_CONTENT_TYPE = "application/vnd.coding-adventures.chief-channel-epoch-activation-v1"
    MAX_PLAN_RECEIVERS = 1024
    MAX_U64 = (1 << 64) - 1
    MAX_EPOCH_CAS_ATTEMPTS = 16

    STATE_MAGIC = "D18S"
    PLAN_MAGIC = "D18T"

    ERROR_CODES = %w[
      not_initialized channel_destroyed invalid_plan corrupt_record pending_append
      unactivated_epoch active_key_missing conflicting_active_key preparation_missing
      conflicting_preparation conflicting_plan conflicting_grant unexpected_epoch
      decreasing_epoch epoch_exhausted concurrent_update storage_error custody_error
      crypto_error
    ].freeze

    # Stable D18T failure. The message is exactly the code: no channel bytes, no
    # epoch numbers, no key material, nothing an operator might paste into a bug
    # report and regret.
    class EpochActivationError < StandardError
      attr_reader :code

      def initialize(code)
        @code = code.dup.freeze
        super(@code)
      end
    end

    # Malformed or non-canonical public record. Always +corrupt_record+: D18T
    # deliberately does not distinguish "truncated" from "bad version" from
    # "trailing bytes", because that distinction would tell an attacker how far
    # their forgery got.
    class EpochWireError < EpochActivationError
      def initialize = super("corrupt_record")
    end

    # Decoded +D18S+ version 2 record.
    #
    #   offset  field           encoding
    #   0       magic           ascii("D18S")
    #   4       version         0x02
    #   5       active_epoch    u64be
    #   13      next_sequence   u64be
    #   21      pending_flag    u8: 0 none, 1 header follows
    #   22      header_length   u32be, max 16384   (only when flag = 1)
    #   26      reserved_header exact D18H v1 bytes (only when flag = 1)
    #
    # With no pending header the record is exactly 22 octets; with one it is
    # exactly <tt>26 + header_length</tt>. Trailing bytes are never permitted.
    class EpochState
      attr_reader :active_epoch, :next_sequence, :pending_header

      # Validates every cross-field invariant D18T requires. A pending header
      # must name this channel, sit exactly one below +next_sequence+, and carry
      # the currently active epoch -- that last check is what prevents a
      # reservation from surviving across an activation it never agreed to.
      def initialize(channel_id:, active_epoch:, next_sequence:, pending_header: nil)
        Wire.u64!(active_epoch)
        Wire.u64!(next_sequence)
        channel = Wire.fixed!(channel_id, 16)
        unless pending_header.nil?
          Wire.fail! unless pending_header.channel_id == channel &&
            pending_header.sequence != MAX_U64 &&
            pending_header.sequence + 1 == next_sequence &&
            pending_header.key_epoch == active_epoch
        end
        @active_epoch = active_epoch
        @next_sequence = next_sequence
        @pending_header = pending_header
        freeze
      end

      # Activation transition: change only the epoch.
      def with_active_epoch(channel_id, active_epoch)
        EpochState.new(channel_id: channel_id, active_epoch: active_epoch,
          next_sequence: @next_sequence, pending_header: @pending_header)
      end

      # Reservation transition: change only the sequence and pending header.
      def with_pending(channel_id, next_sequence, pending_header = nil)
        EpochState.new(channel_id: channel_id, active_epoch: @active_epoch,
          next_sequence: next_sequence, pending_header: pending_header)
      end

      # Two states are equal exactly when they would be stored identically.
      def ==(other)
        other.is_a?(EpochState) && Wire.epoch_state_serialize(self) == Wire.epoch_state_serialize(other)
      end
      alias_method :eql?, :==
    end

    # One receiver's commitment pair. The plan carries no raw receiver ID and no
    # grant body -- only hashes -- so the public plan record leaks neither the
    # membership roster nor any key material.
    class ActivationPlanEntry
      attr_reader :receiver_id_hash, :grant_hash

      def initialize(receiver_id_hash:, grant_hash:)
        @receiver_id_hash = Wire.fixed!(receiver_id_hash, 32)
        @grant_hash = Wire.fixed!(grant_hash, 32)
        freeze
      end

      def ==(other)
        other.is_a?(ActivationPlanEntry) &&
          other.receiver_id_hash == @receiver_id_hash && other.grant_hash == @grant_hash
      end
      alias_method :eql?, :==
    end

    # Immutable +D18T+ version 1 activation plan.
    #
    #   offset  field            encoding
    #   0       magic            ascii("D18T")
    #   4       version          0x01
    #   5       channel_id       bytes[16], UUID v7
    #   21      base_epoch       u64be
    #   29      new_epoch        u64be
    #   37      receiver_count   u32be, 1 through 1024
    #   41      receivers        repeated: receiver_id_hash[32] grant_hash[32]
    #
    # Entries are strictly sorted by +receiver_id_hash+ with no duplicate
    # receiver or grant commitment. Strict sorting makes the encoding canonical:
    # the same rotation always produces the same bytes, so a byte comparison is
    # a complete equality test during replay.
    class ActivationPlan
      attr_reader :channel_id, :base_epoch, :new_epoch, :receivers

      # Sorts, validates, and owns the entries.
      #
      # Two distinct receiver IDs hashing to the same value would be a SHA-256
      # collision -- but D18T does not treat a collision as equal
      # authorization, it treats it as invalid input. Rejecting rather than
      # merging is the fail-closed choice.
      def initialize(channel_id:, base_epoch:, new_epoch:, receivers:)
        @channel_id = Wire.uuid_v7!(channel_id)
        Wire.u64!(base_epoch)
        Wire.u64!(new_epoch)
        Wire.fail! if base_epoch == MAX_U64 || new_epoch != base_epoch + 1
        ordered = receivers.to_a.sort_by(&:receiver_id_hash)
        Wire.fail! unless ordered.length.between?(1, MAX_PLAN_RECEIVERS)
        Wire.fail! if ordered.map(&:receiver_id_hash).uniq.length != ordered.length
        Wire.fail! if ordered.map(&:grant_hash).uniq.length != ordered.length
        @base_epoch = base_epoch
        @new_epoch = new_epoch
        @receivers = ordered.freeze
        freeze
      end

      def ==(other)
        other.is_a?(ActivationPlan) &&
          Wire.activation_plan_serialize(self) == Wire.activation_plan_serialize(other)
      end
      alias_method :eql?, :==
    end

    # Exact D18S v2 and D18T v1 codecs.
    module Wire
      module_function

      def fail! = raise(EpochWireError)

      def u64!(value)
        fail! unless value.is_a?(Integer) && value >= 0 && value <= MAX_U64
        value
      end

      def fixed!(value, length)
        fail! unless value.is_a?(String)
        copy = value.b.dup.freeze
        fail! unless copy.bytesize == length
        copy
      end

      # A channel identifier must be a real UUID v7 -- version nibble 7 and
      # variant bits 0b10 -- not merely 16 octets. The Rust reference and the
      # Python port both check this, so accepting a malformed identifier here
      # would mean two conforming implementations disagreed about whether the
      # same plan record is valid.
      def uuid_v7!(value)
        copy = fixed!(value, 16)
        fail! unless copy.getbyte(6) >> 4 == 7 && (copy.getbyte(8) & 0xc0) == 0x80
        copy
      end

      def u32be(value)
        fail! unless value.is_a?(Integer) && value >= 0 && value < (1 << 32)
        [value].pack("N")
      end

      def u64be(value) = [u64!(value)].pack("Q>")

      def epoch_state_serialize(state)
        prefix = [STATE_MAGIC, 2.chr, u64be(state.active_epoch), u64be(state.next_sequence)].join.b
        return "#{prefix}\x00".b if state.pending_header.nil?

        header = Store::ChannelProfile.header_serialize(state.pending_header)
        fail! if header.bytesize > Store::MAX_PENDING_HEADER_BYTES
        [prefix, 1.chr, u32be(header.bytesize), header].join.b
      end

      def epoch_state_deserialize(data, channel_id)
        reader = Reader.new(data)
        fail! unless reader.take(4) == STATE_MAGIC
        fail! unless reader.take(1).getbyte(0) == 2
        active_epoch = reader.u64
        next_sequence = reader.u64
        flag = reader.take(1).getbyte(0)
        pending =
          case flag
          when 0 then nil
          when 1
            length = reader.u32
            fail! if length > Store::MAX_PENDING_HEADER_BYTES
            Store::ChannelProfile.header_deserialize(reader.take(length))
          else fail!
          end
        reader.finish
        EpochState.new(channel_id: channel_id, active_epoch: active_epoch,
          next_sequence: next_sequence, pending_header: pending)
      rescue EpochActivationError
        raise
      rescue StandardError
        fail!
      end

      def activation_plan_serialize(plan)
        fail! unless plan.receivers.length.between?(1, MAX_PLAN_RECEIVERS)
        parts = [PLAN_MAGIC, 1.chr, plan.channel_id, u64be(plan.base_epoch),
          u64be(plan.new_epoch), u32be(plan.receivers.length)]
        plan.receivers.each { |entry| parts.push(entry.receiver_id_hash, entry.grant_hash) }
        parts.join.b
      end

      # Sort order is checked on the wire BEFORE the entries reach
      # ActivationPlan, which sorts its input and would otherwise silently
      # canonicalize a mis-ordered record. Rejecting first is what makes the
      # encoding canonical rather than merely normalized.
      def activation_plan_deserialize(data)
        reader = Reader.new(data)
        fail! unless reader.take(4) == PLAN_MAGIC
        fail! unless reader.take(1).getbyte(0) == 1
        channel_id = reader.take(16)
        base_epoch = reader.u64
        new_epoch = reader.u64
        count = reader.u32
        fail! unless count.between?(1, MAX_PLAN_RECEIVERS)
        entries = Array.new(count) do
          ActivationPlanEntry.new(receiver_id_hash: reader.take(32), grant_hash: reader.take(32))
        end
        reader.finish
        entries.each_cons(2) { |left, right| fail! if left.receiver_id_hash >= right.receiver_id_hash }
        plan = ActivationPlan.new(channel_id: channel_id, base_epoch: base_epoch,
          new_epoch: new_epoch, receivers: entries)
        fail! unless plan.receivers == entries
        plan
      rescue EpochActivationError
        raise
      rescue StandardError
        fail!
      end

      # The epoch is zero-padded to 20 digits so lexicographic key order and
      # numeric epoch order agree, which is what lets a prefix listing walk
      # epochs in sequence.
      def activation_plan_record_key(channel_id, new_epoch)
        format("%s/epochs/%020d/activation", fixed!(channel_id, 16).unpack1("H*"), u64!(new_epoch))
      end

      # Byte reader that refuses to over-read or leave trailing bytes.
      class Reader
        def initialize(data)
          Wire.fail! unless data.is_a?(String)
          @data = data.b
          @offset = 0
        end

        def take(length)
          Wire.fail! if length.negative? || @offset + length > @data.bytesize
          value = @data.byteslice(@offset, length)
          @offset += length
          value
        end

        def u32 = take(4).unpack1("N")
        def u64 = take(8).unpack1("Q>")
        def finish = (Wire.fail! unless @offset == @data.bytesize)
      end
    end
  end
end
