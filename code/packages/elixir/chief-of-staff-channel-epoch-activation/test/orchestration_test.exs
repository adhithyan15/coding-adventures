defmodule CodingAdventures.ChiefOfStaffChannelEpochActivation.OrchestrationTest do
  @moduledoc "Crash, retry, activation, publish, race, and destruction orchestration."

  use ExUnit.Case, async: true

  alias CodingAdventures.ChiefOfStaffChannelCrypto.KeyGrantProfile, as: Grants
  alias CodingAdventures.ChiefOfStaffChannelStore, as: Profile
  alias CodingAdventures.ChiefOfStaffChannelStore.{ChannelDefinitionStore, MemoryChannelStorage}
  alias CodingAdventures.ChiefOfStaffChannelEpochActivation.ActivationError
  alias CodingAdventures.ChiefOfStaffChannelEpochActivation.ActiveEpochAppendRequest
  alias CodingAdventures.ChiefOfStaffChannelEpochActivation.Custody
  alias CodingAdventures.ChiefOfStaffChannelEpochActivation.Custody.InMemory
  alias CodingAdventures.ChiefOfStaffChannelEpochActivation.EpochKeyHandle
  alias CodingAdventures.ChiefOfStaffChannelEpochActivation.Store, as: EpochStore

  @channel_id Base.decode16!("018f47a09b6c7def923456789abcdef0", case: :lower)
  @message_id Base.decode16!("018f47a09b6c7def923456789abcdef1", case: :lower)
  @current_cmk :binary.copy(<<0x22>>, 32)
  @next_cmk :binary.copy(<<0x33>>, 32)

  defmodule DurableMemory do
    @moduledoc """
    InMemory custody that claims durability, so the production constructor
    accepts it. Only tests may do this; the point of the durable?/1 split is
    that a real deployment cannot.
    """
    alias CodingAdventures.ChiefOfStaffChannelEpochActivation.Custody.InMemory

    @enforce_keys [:inner]
    defstruct @enforce_keys

    def new!, do: %__MODULE__{inner: InMemory.new!()}
    def durable?(%__MODULE__{}), do: true

    def import_active_if_absent(%__MODULE__{inner: inner}, channel_id, epoch, cmk),
      do: InMemory.import_active_if_absent(inner, channel_id, epoch, cmk)

    def resolve_handle(%__MODULE__{inner: inner}, channel_id, epoch),
      do: InMemory.resolve_handle(inner, channel_id, epoch)

    def prepare_if_absent(%__MODULE__{inner: inner}, prepared),
      do: InMemory.prepare_if_absent(inner, prepared)

    def load_preparation(%__MODULE__{inner: inner}, channel_id, new_epoch),
      do: InMemory.load_preparation(inner, channel_id, new_epoch)

    def with_key(%__MODULE__{inner: inner}, handle, operation),
      do: InMemory.with_key(inner, handle, operation)

    def destroy_channel(%__MODULE__{inner: inner}, channel_id),
      do: InMemory.destroy_channel(inner, channel_id)
  end

  setup do
    signer = Grants.originator_signing_key_from_seed(:binary.copy(<<0x11>>, 32))
    receiver_a_key = Grants.receiver_key_pair_from_private_key(:binary.copy(<<0x41>>, 32))
    receiver_b_key = Grants.receiver_key_pair_from_private_key(:binary.copy(<<0x42>>, 32))
    receiver_a = Profile.new_receiver!("receiver-a", Grants.receiver_public_key(receiver_a_key))
    receiver_b = Profile.new_receiver!("receiver-b", Grants.receiver_public_key(receiver_b_key))

    definition =
      Profile.new_definition!(
        @channel_id,
        Profile.new_originator!("originator", Grants.originator_public_key(signer)),
        [receiver_a, receiver_b],
        1_725_000_000_000_000_000,
        0
      )

    backend = MemoryChannelStorage.new!()
    custody = InMemory.new!()
    store = EpochStore.open_for_testing(backend, custody, @channel_id)

    %{
      signer: signer,
      receiver_a: receiver_a,
      receiver_b: receiver_b,
      definition: definition,
      backend: backend,
      custody: custody,
      store: store
    }
  end

  defp create(context) do
    state =
      EpochStore.create_epoch_channel(
        context.store,
        context.definition,
        Grants.channel_master_key_from_bytes(@current_cmk)
      )

    assert state.active_epoch == 0
    assert state.next_sequence == 0
    assert state.pending_header == nil
    state
  end

  defp rotation(context, cmk \\ @next_cmk, ephemeral \\ nil, nonce \\ nil) do
    Grants.plan_rotation(
      "originator",
      @channel_id,
      0,
      Grants.channel_master_key_from_bytes(cmk),
      [
        Grants.rotation_receiver_with_material(
          context.receiver_b.agent_id,
          context.receiver_b.public_key,
          ephemeral || :binary.copy(<<0x51>>, 32),
          nonce || :binary.copy(<<0x61>>, 24)
        )
      ],
      context.signer
    )
  end

  defp assert_code(code, operation) do
    error = assert_raise ActivationError, operation
    assert error.code == code
    # The message is exactly the code -- no channel bytes, no epoch numbers.
    assert Exception.message(error) == code
    error
  end

  test "production rejects non-durable custody and accepts durable", context do
    assert_code("custody_error", fn ->
      EpochStore.open(context.backend, InMemory.new!(), @channel_id)
    end)

    # The same custody, wrapped in a type that honestly claims durability, is
    # accepted -- so the gate is on the declaration, not on the type.
    durable = EpochStore.open(MemoryChannelStorage.new!(), DurableMemory.new!(), @channel_id)
    assert durable.channel_id == @channel_id
  end

  test "custody-first creation is idempotent and conflicts fail closed", context do
    create(context)

    same =
      EpochStore.create_epoch_channel(
        context.store,
        context.definition,
        Grants.channel_master_key_from_bytes(@current_cmk)
      )

    assert same.active_epoch == 0

    # A different CMK for the same epoch is a fail-closed conflict, and the
    # error does not disclose how the stored secret differed.
    assert_code("conflicting_active_key", fn ->
      EpochStore.create_epoch_channel(
        context.store,
        context.definition,
        Grants.channel_master_key_from_bytes(:binary.copy(<<0x99>>, 32))
      )
    end)
  end

  test "prepare, recover, activate, and prospective revocation", context do
    create(context)

    assert EpochStore.prepare_rotation(
             context.store,
             context.definition,
             [context.receiver_b],
             rotation(context)
           ) == "prepared"

    assert EpochStore.activation_plan(context.store, 1) != nil
    assert EpochStore.recover_preparation(context.store, context.definition, 1) == "idempotent"
    # Preparation must not change the active epoch.
    assert EpochStore.state(context.store).active_epoch == 0

    assert EpochStore.activate_prepared_epoch(context.store, context.definition, 1) == "activated"
    assert EpochStore.activate_prepared_epoch(context.store, context.definition, 1) == "idempotent"
    assert EpochStore.state(context.store).active_epoch == 1

    # Prospective revocation: the originator retains BOTH epoch keys, because
    # old messages were encrypted under epoch 0 and are never re-encrypted.
    assert InMemory.resolve_handle(context.custody, @channel_id, 0) != nil
    assert InMemory.resolve_handle(context.custody, @channel_id, 1) != nil
  end

  # The crash-safety core. Select a candidate in custody and do nothing else --
  # simulating a process that died between phase 2 and phase 4 -- then require
  # recovery to reconstruct every public record from the durable bundle alone.
  test "crash after custody selection replays public records", context do
    create(context)

    prepared =
      EpochStore.prepare_rotation_candidate(
        context.definition,
        0,
        [context.receiver_b],
        rotation(context)
      )

    assert InMemory.prepare_if_absent(context.custody, prepared) == Custody.selected()
    # A byte-identical retry is idempotent, not a conflict.
    assert InMemory.prepare_if_absent(context.custody, prepared) == Custody.idempotent()

    # Crash point: custody holds the bundle, nothing is public yet.
    assert EpochStore.activation_plan(context.store, 1) == nil
    assert EpochStore.recover_preparation(context.store, context.definition, 1) == "idempotent"

    plan = EpochStore.activation_plan(context.store, 1)
    assert plan != nil
    assert {plan.base_epoch, plan.new_epoch} == {0, 1}
  end

  # The race trace of the same name: two candidates compete for E+1, exactly one
  # is selected, and the loser must not write anything public.
  test "a different candidate loses the custody slot", context do
    create(context)

    winner =
      EpochStore.prepare_rotation_candidate(
        context.definition,
        0,
        [context.receiver_b],
        rotation(context)
      )

    assert InMemory.prepare_if_absent(context.custody, winner) == Custody.selected()

    loser =
      EpochStore.prepare_rotation_candidate(
        context.definition,
        0,
        [context.receiver_b],
        rotation(context, :binary.copy(<<0x77>>, 32), :binary.copy(<<0x52>>, 32), :binary.copy(<<0x62>>, 24))
      )

    assert InMemory.prepare_if_absent(context.custody, loser) == Custody.conflict()

    # And the store-level path reports the stable code.
    assert_code("conflicting_preparation", fn ->
      EpochStore.prepare_rotation(
        context.store,
        context.definition,
        [context.receiver_b],
        rotation(context, :binary.copy(<<0x88>>, 32), :binary.copy(<<0x53>>, 32), :binary.copy(<<0x63>>, 24))
      )
    end)
  end

  # Both directions of the shared-CAS race: a reservation in flight blocks
  # activation, and clearing it unblocks.
  test "a pending publish serializes rotation", context do
    create(context)

    request = %ActiveEpochAppendRequest{
      message_id: @message_id,
      timestamp_ns: 1_725_000_000_000_000_001,
      originator_id: "originator",
      content_type: "application/octet-stream",
      key_epoch: 0
    }

    reservation =
      EpochStore.reserve_publish_using_active_epoch(
        context.store,
        context.definition,
        request,
        "hello"
      )

    assert reservation.header.sequence == 0
    assert reservation.key_handle.epoch == 0
    # The handle redacts every field under inspection.
    assert inspect(reservation.key_handle) =~ "EpochKeyHandle"
    refute inspect(reservation.key_handle) =~ "epoch:"

    # Publication won the CAS, so activation must yield rather than race it.
    assert_code("pending_append", fn ->
      EpochStore.prepare_rotation(
        context.store,
        context.definition,
        [context.receiver_b],
        rotation(context)
      )
    end)

    assert EpochStore.abandon_pending(context.store) == reservation.header
    assert EpochStore.abandon_pending(context.store) == nil

    # With the reservation cleared, rotation proceeds.
    assert EpochStore.prepare_rotation(
             context.store,
             context.definition,
             [context.receiver_b],
             rotation(context)
           ) == "prepared"
  end

  test "an unactivated epoch is rejected before any state change", context do
    create(context)

    request = %ActiveEpochAppendRequest{
      message_id: @message_id,
      timestamp_ns: 1,
      originator_id: "originator",
      content_type: "application/octet-stream",
      key_epoch: 1
    }

    assert_code("unactivated_epoch", fn ->
      EpochStore.reserve_publish_using_active_epoch(
        context.store,
        context.definition,
        request,
        "hello"
      )
    end)

    state = EpochStore.state(context.store)
    assert state.pending_header == nil
    assert state.next_sequence == 0
  end

  test "fail-closed preconditions and stable codes", context do
    create(context)

    assert_code("preparation_missing", fn ->
      EpochStore.activate_prepared_epoch(context.store, context.definition, 1)
    end)

    assert_code("unexpected_epoch", fn ->
      EpochStore.recover_preparation(context.store, context.definition, 2)
    end)

    assert_code("invalid_plan", fn ->
      EpochStore.reserve_publish_using_active_epoch(
        context.store,
        context.definition,
        %ActiveEpochAppendRequest{
          message_id: @message_id,
          timestamp_ns: 1,
          originator_id: "not-originator",
          content_type: "application/octet-stream"
        },
        "hello"
      )
    end)

    assert_code("invalid_plan", fn ->
      EpochStore.prepare_rotation(context.store, context.definition, [], rotation(context))
    end)

    isolated = EpochStore.open_for_testing(context.backend, InMemory.new!(), @channel_id)

    assert_code("active_key_missing", fn ->
      EpochStore.migrate_epoch_state(isolated, context.definition)
    end)

    assert_code("custody_error", fn ->
      InMemory.with_key(
        context.custody,
        %EpochKeyHandle{channel_id: @channel_id, epoch: 99},
        fn _ -> nil end
      )
    end)
  end

  test "not initialized before creation", context do
    assert_code("not_initialized", fn -> EpochStore.state(context.store) end)

    assert_code("not_initialized", fn ->
      EpochStore.recover_preparation(context.store, context.definition, 1)
    end)
  end

  test "corrupt public state fails closed", context do
    create(context)
    key = Profile.state_key(@channel_id)
    record = MemoryChannelStorage.get(context.backend, Profile.storage_namespace(), key)
    assert record != nil

    MemoryChannelStorage.corrupt(context.backend, %{record | body: record.body <> <<0>>})
    assert_code("corrupt_record", fn -> EpochStore.state(context.store) end)
  end

  # Invariant 6, made observable: destruction erases secrets but leaves the
  # append-only public record exactly where it was.
  test "destruction wipes custody but retains public history", context do
    create(context)

    EpochStore.prepare_rotation(
      context.store,
      context.definition,
      [context.receiver_b],
      rotation(context)
    )

    EpochStore.activate_prepared_epoch(context.store, context.definition, 1)
    before_plan = EpochStore.activation_plan(context.store, 1)
    assert before_plan != nil

    destroyed =
      ChannelDefinitionStore.destroy!(
        ChannelDefinitionStore.new(context.backend),
        @channel_id
      )

    assert EpochStore.apply_destruction(context.store, destroyed) == :ok
    assert InMemory.retained_key_count(context.custody) == 0

    assert EpochStore.activation_plan(context.store, 1) == before_plan

    assert_code("channel_destroyed", fn ->
      EpochStore.reserve_publish_using_active_epoch(
        context.store,
        destroyed,
        %ActiveEpochAppendRequest{
          message_id: @message_id,
          timestamp_ns: 1,
          originator_id: "originator",
          content_type: "application/octet-stream"
        },
        "hello"
      )
    end)
  end

  test "apply_destruction requires a destroyed definition", context do
    create(context)

    assert_code("invalid_plan", fn ->
      EpochStore.apply_destruction(context.store, context.definition)
    end)

    assert InMemory.retained_key_count(context.custody) > 0
  end

  test "a decreasing epoch is rejected", context do
    create(context)

    EpochStore.prepare_rotation(
      context.store,
      context.definition,
      [context.receiver_b],
      rotation(context)
    )

    EpochStore.activate_prepared_epoch(context.store, context.definition, 1)

    assert_code("decreasing_epoch", fn ->
      EpochStore.recover_preparation(context.store, context.definition, 0)
    end)
  end

  test "the roster must match the grant receivers", context do
    create(context)

    # One-receiver rotation for B, but a roster naming A.
    assert_code("invalid_plan", fn ->
      EpochStore.prepare_rotation(
        context.store,
        context.definition,
        [context.receiver_a],
        rotation(context)
      )
    end)
  end
end
