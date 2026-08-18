defmodule CodingAdventures.ChiefOfStaffChannelEpochActivation do
  @moduledoc """
  Portable D18T durable channel epoch-activation profile.

  ## The problem D18T solves

  D18P makes a channel's messages, grants, cursors, and sequence reservations
  durable. D18Q can mint a fresh channel master key (CMK) for epoch E+1 and seal
  one grant per authorized receiver. Neither can make E+1 *current*.

  That gap is not cosmetic. The obvious implementation — write a "current epoch"
  record, then publish with the new key — loses data two ways:

      crash here  ->  the new epoch is visible, but its CMK was never durably
                      stored, so nothing published afterwards can be decrypted
      crash here  ->  a concurrent publisher reserved a slot at epoch E while
                      activation committed E+1; whose key is right?

  D18T defines the missing transaction without assuming the storage backend
  offers multi-record transactions. Three authorities cooperate:

      D18P channel store   public records and the publish-reservation CAS
      injected custody     prepared CMKs and their recovery bundles
      D18Q                 grant creation, parsing, and verification

  ## The one idea worth remembering

  The active epoch lives in the *same versioned record* as the pending publish
  reservation. That is deliberate and load-bearing. A separate mutable "epoch
  head" record would not be conforming, because two independent compare-and-swap
  operations cannot exclude each other: a publisher could reserve a slot against
  the old epoch in the window between activation reading the head and writing
  it. One record means one revision means one CAS, and exactly one of
  {publish, activate} wins.
  """

  import Bitwise

  alias CodingAdventures.ChiefOfStaffChannelStore, as: Store

  @epoch_state_content_type "application/vnd.coding-adventures.chief-channel-state-v2"
  @activation_plan_content_type "application/vnd.coding-adventures.chief-channel-epoch-activation-v1"
  @max_plan_receivers 1024
  @max_u64 (1 <<< 64) - 1
  @max_epoch_cas_attempts 16
  @state_magic "D18S"
  @plan_magic "D18T"

  @error_codes ~w(not_initialized channel_destroyed invalid_plan corrupt_record
                  pending_append unactivated_epoch active_key_missing
                  conflicting_active_key preparation_missing conflicting_preparation
                  conflicting_plan conflicting_grant unexpected_epoch decreasing_epoch
                  epoch_exhausted concurrent_update storage_error custody_error
                  crypto_error)

  defmodule ActivationError do
    @moduledoc """
    Stable D18T failure. The message is exactly the code: no channel bytes, no
    epoch numbers, no key material, nothing an operator might paste into a bug
    report and regret.
    """
    defexception [:code]

    @impl true
    def message(%__MODULE__{code: code}), do: code
  end

  defmodule EpochState do
    @moduledoc """
    Decoded `D18S` version 2 record.

        offset  field           encoding
        0       magic           ascii("D18S")
        4       version         0x02
        5       active_epoch    u64be
        13      next_sequence   u64be
        21      pending_flag    u8: 0 none, 1 header follows
        22      header_length   u32be, max 16384   (only when flag = 1)
        26      reserved_header exact D18H v1 bytes (only when flag = 1)

    With no pending header the record is exactly 22 octets; with one it is
    exactly `26 + header_length`. Trailing bytes are never permitted.
    """
    @enforce_keys [:active_epoch, :next_sequence]
    defstruct [:active_epoch, :next_sequence, pending_header: nil]
  end

  defmodule ActivationPlanEntry do
    @moduledoc """
    One receiver's commitment pair. The plan carries no raw receiver ID and no
    grant body — only hashes — so the public plan record leaks neither the
    membership roster nor any key material.
    """
    @enforce_keys [:receiver_id_hash, :grant_hash]
    defstruct @enforce_keys
  end

  defmodule ActivationPlan do
    @moduledoc """
    Immutable `D18T` version 1 activation plan.

        offset  field            encoding
        0       magic            ascii("D18T")
        4       version          0x01
        5       channel_id       bytes[16], UUID v7
        21      base_epoch       u64be
        29      new_epoch        u64be
        37      receiver_count   u32be, 1 through 1024
        41      receivers        repeated: receiver_id_hash[32] grant_hash[32]

    Entries are strictly sorted by `receiver_id_hash` with no duplicate receiver
    or grant commitment. Strict sorting makes the encoding canonical: the same
    rotation always produces the same bytes, so a byte comparison is a complete
    equality test during replay.
    """
    @enforce_keys [:channel_id, :base_epoch, :new_epoch, :receivers]
    defstruct @enforce_keys
  end

  @doc "Content type tagging the D18S version 2 state record."
  def epoch_state_content_type, do: @epoch_state_content_type

  @doc "Content type tagging the immutable D18T version 1 plan record."
  def activation_plan_content_type, do: @activation_plan_content_type

  @doc "Largest rotation a single plan may carry."
  def max_plan_receivers, do: @max_plan_receivers

  @doc "Largest epoch or sequence value. Reaching it is an error, never a wrap."
  def max_u64, do: @max_u64

  @doc "Bound on every public compare-and-swap loop."
  def max_epoch_cas_attempts, do: @max_epoch_cas_attempts

  @doc "Closed, ordered stable D18T error roster."
  def error_codes, do: @error_codes

  @doc """
  Report Elixir's honest erasure capability, inherited from D18Q rather than
  claimed independently. The Rust reference reports "guaranteed"; overstating
  Elixir's position to match it would be the dishonest kind of portability.
  """
  def secret_erasure_capability do
    CodingAdventures.ChiefOfStaffChannelCrypto.KeyGrantProfile.secret_erasure_capability()
  end

  @doc false
  def fail!(code) when code in @error_codes, do: raise(ActivationError, code: code)

  @doc false
  def wire_fail!, do: fail!("corrupt_record")

  # ---------------------------------------------------------------------------
  # D18S v2 state
  # ---------------------------------------------------------------------------

  @doc """
  Validate every cross-field invariant D18T requires of a state record.

  A pending header must name this channel, sit exactly one below
  `next_sequence`, and carry the currently active epoch — that last check is
  what prevents a reservation from surviving across an activation it never
  agreed to.
  """
  def new_epoch_state!(channel_id, active_epoch, next_sequence, pending_header \\ nil) do
    require_u64!(active_epoch)
    require_u64!(next_sequence)
    channel = require_fixed!(channel_id, 16)

    unless is_nil(pending_header) do
      unless pending_header.channel_id == channel and
               pending_header.sequence != @max_u64 and
               pending_header.sequence + 1 == next_sequence and
               pending_header.key_epoch == active_epoch do
        wire_fail!()
      end
    end

    %EpochState{
      active_epoch: active_epoch,
      next_sequence: next_sequence,
      pending_header: pending_header
    }
  end

  @doc "Activation transition: change only the epoch."
  def with_active_epoch!(%EpochState{} = state, channel_id, active_epoch) do
    new_epoch_state!(channel_id, active_epoch, state.next_sequence, state.pending_header)
  end

  @doc "Reservation transition: change only the sequence and pending header."
  def with_pending!(%EpochState{} = state, channel_id, next_sequence, pending_header \\ nil) do
    new_epoch_state!(channel_id, state.active_epoch, next_sequence, pending_header)
  end

  @doc "Encode canonical D18S version 2 bytes."
  def epoch_state_serialize(%EpochState{} = state) do
    prefix =
      @state_magic <> <<2>> <> u64be(state.active_epoch) <> u64be(state.next_sequence)

    case state.pending_header do
      nil ->
        prefix <> <<0>>

      header ->
        encoded = Store.header_serialize(header)
        if byte_size(encoded) > Store.max_pending_header_bytes(), do: wire_fail!()
        prefix <> <<1>> <> u32be(byte_size(encoded)) <> encoded
    end
  end

  @doc "Decode and fully validate canonical D18S version 2 bytes."
  def epoch_state_deserialize(data, channel_id) when is_binary(data) do
    case data do
      <<@state_magic, 2, active_epoch::unsigned-big-64, next_sequence::unsigned-big-64,
        0::unsigned-8>> ->
        new_epoch_state!(channel_id, active_epoch, next_sequence)

      <<@state_magic, 2, active_epoch::unsigned-big-64, next_sequence::unsigned-big-64,
        1::unsigned-8, length::unsigned-big-32, rest::binary>> ->
        if length > Store.max_pending_header_bytes() or byte_size(rest) != length do
          wire_fail!()
        end

        header =
          try do
            Store.header_deserialize(rest)
          rescue
            _ -> wire_fail!()
          end

        new_epoch_state!(channel_id, active_epoch, next_sequence, header)

      _ ->
        wire_fail!()
    end
  end

  def epoch_state_deserialize(_, _), do: wire_fail!()

  # ---------------------------------------------------------------------------
  # D18T v1 activation plan
  # ---------------------------------------------------------------------------

  @doc "Build one commitment pair from 32-octet hashes."
  def new_plan_entry!(receiver_id_hash, grant_hash) do
    %ActivationPlanEntry{
      receiver_id_hash: require_fixed!(receiver_id_hash, 32),
      grant_hash: require_fixed!(grant_hash, 32)
    }
  end

  @doc """
  Sort, validate, and own the plan entries.

  Two distinct receiver IDs hashing to the same value would be a SHA-256
  collision — but D18T does not treat a collision as equal authorization, it
  treats it as invalid input. Rejecting rather than merging is the fail-closed
  choice.
  """
  def new_activation_plan!(channel_id, base_epoch, new_epoch, receivers) do
    channel = require_uuid_v7!(channel_id)
    require_u64!(base_epoch)
    require_u64!(new_epoch)
    if base_epoch == @max_u64 or new_epoch != base_epoch + 1, do: wire_fail!()

    ordered = Enum.sort_by(receivers, & &1.receiver_id_hash)
    count = length(ordered)
    unless count >= 1 and count <= @max_plan_receivers, do: wire_fail!()
    if length(Enum.uniq_by(ordered, & &1.receiver_id_hash)) != count, do: wire_fail!()
    if length(Enum.uniq_by(ordered, & &1.grant_hash)) != count, do: wire_fail!()

    %ActivationPlan{
      channel_id: channel,
      base_epoch: base_epoch,
      new_epoch: new_epoch,
      receivers: ordered
    }
  end

  @doc "Encode canonical D18T version 1 bytes."
  def activation_plan_serialize(%ActivationPlan{} = plan) do
    count = length(plan.receivers)
    unless count >= 1 and count <= @max_plan_receivers, do: wire_fail!()

    entries =
      Enum.map_join(plan.receivers, fn entry -> entry.receiver_id_hash <> entry.grant_hash end)

    @plan_magic <>
      <<1>> <>
      plan.channel_id <> u64be(plan.base_epoch) <> u64be(plan.new_epoch) <> u32be(count) <> entries
  end

  @doc """
  Decode canonical D18T version 1 bytes.

  Sort order is checked on the wire BEFORE the entries reach
  `new_activation_plan!/4`, which sorts its input and would otherwise silently
  canonicalize a mis-ordered record. Rejecting first is what makes the encoding
  canonical rather than merely normalized.
  """
  def activation_plan_deserialize(data) when is_binary(data) do
    case data do
      <<@plan_magic, 1, channel_id::binary-size(16), base_epoch::unsigned-big-64,
        new_epoch::unsigned-big-64, count::unsigned-big-32, rest::binary>> ->
        unless count >= 1 and count <= @max_plan_receivers, do: wire_fail!()
        if byte_size(rest) != count * 64, do: wire_fail!()

        entries =
          for <<receiver::binary-size(32), grant::binary-size(32) <- rest>>,
            do: new_plan_entry!(receiver, grant)

        entries
        |> Enum.chunk_every(2, 1, :discard)
        |> Enum.each(fn [left, right] ->
          if left.receiver_id_hash >= right.receiver_id_hash, do: wire_fail!()
        end)

        plan = new_activation_plan!(channel_id, base_epoch, new_epoch, entries)
        if plan.receivers != entries, do: wire_fail!()
        plan

      _ ->
        wire_fail!()
    end
  end

  def activation_plan_deserialize(_), do: wire_fail!()

  @doc """
  Deterministic storage key for a plan.

  The epoch is zero-padded to 20 digits so lexicographic key order and numeric
  epoch order agree, which is what lets a prefix listing walk epochs in
  sequence.
  """
  def activation_plan_record_key(channel_id, new_epoch) do
    channel = require_fixed!(channel_id, 16)
    require_u64!(new_epoch)

    Base.encode16(channel, case: :lower) <>
      "/epochs/" <> String.pad_leading(Integer.to_string(new_epoch), 20, "0") <> "/activation"
  end

  # ---------------------------------------------------------------------------
  # Shared validation helpers
  # ---------------------------------------------------------------------------

  @doc false
  def require_u64!(value) do
    unless is_integer(value) and value >= 0 and value <= @max_u64, do: wire_fail!()
    value
  end

  @doc false
  def require_fixed!(value, length) do
    unless is_binary(value) and byte_size(value) == length, do: wire_fail!()
    value
  end

  @doc """
  A channel identifier must be a real UUID v7 — version nibble 7 and variant
  bits 0b10 — not merely 16 octets. The Rust reference and the Python port both
  check this, so accepting a malformed identifier would mean two conforming
  implementations disagreed about whether the same plan record is valid.
  """
  def require_uuid_v7!(value) do
    channel = require_fixed!(value, 16)

    # Byte 6's high nibble is the version; byte 8's top two bits are the
    # variant. Byte 7 sits between them and is unconstrained.
    case channel do
      <<_::binary-size(6), 7::4, _::4, _::binary-size(1), 2::2, _::6, _::binary-size(7)>> ->
        channel

      _ ->
        wire_fail!()
    end
  end

  defp u32be(value) do
    unless is_integer(value) and value >= 0 and value < 1 <<< 32, do: wire_fail!()
    <<value::unsigned-big-32>>
  end

  defp u64be(value), do: <<require_u64!(value)::unsigned-big-64>>
end
